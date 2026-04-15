//! .zbak 加密层
//!
//! 密钥派生链:
//!   用户密码 + Salt → Argon2id(m=65536,t=3,p=4) → 256-bit 主密钥(master_key)
//!   master_key + "verify"     → HKDF → 验证子密钥
//!   master_key + file_index   → HKDF → 逐文件子密钥
//!   master_key + "index"      → HKDF → 索引加密子密钥

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use rand::Rng;

use sz_core::{SzError, SzResult};
use super::format::{NONCE_SIZE, GCM_TAG_SIZE};

// ============================================================================
// Argon2id 主密钥派生
// ============================================================================

/// Argon2id 参数: m=65536(64MB), t=3, p=4
/// 与 sz-crypto/aes.rs 统一参数，满足 OWASP 2024 推荐
/// 旧参数 (m=16384,t=2,p=1) 在 2026 年已不够安全
pub fn derive_master_key(password: &str, salt: &[u8; 16]) -> SzResult<[u8; 32]> {
    use argon2::{Argon2, Params, Algorithm, Version};

    let params = Params::new(65536, 3, 4, Some(32))
        .map_err(|e| SzError::Encryption(format!("Argon2 参数错误: {}", e)))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut master_key = [0u8; 32];

    argon2
        .hash_password_into(password.as_bytes(), salt, &mut master_key)
        .map_err(|e| SzError::Encryption(format!("Argon2id 密钥派生失败: {}", e)))?;

    Ok(master_key)
}

// ============================================================================
// HKDF 子密钥派生
// ============================================================================

/// 派生逐文件加密子密钥
pub fn derive_file_key(master_key: &[u8; 32], file_index: u32) -> [u8; 32] {
    let info = format!("zbak-file-{}", file_index);
    hkdf_derive(master_key, info.as_bytes())
}

/// 派生索引加密子密钥
pub fn derive_index_key(master_key: &[u8; 32]) -> [u8; 32] {
    hkdf_derive(master_key, b"zbak-index")
}

/// 派生验证子密钥
pub fn derive_verify_key(master_key: &[u8; 32]) -> [u8; 32] {
    hkdf_derive(master_key, b"zbak-verify")
}

/// HKDF-SHA256 派生 32 字节子密钥
/// salt 使用固定前缀 "zbak-hkdf-v1"，满足 NIST SP 800-56C 要求
fn hkdf_derive(master_key: &[u8; 32], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"zbak-hkdf-v1"), master_key);
    let mut output = [0u8; 32];
    hk.expand(info, &mut output)
        .expect("HKDF expand 不应失败 (输出长度 <= 255*HashLen)");
    output
}

// ============================================================================
// 密码验证块
// ============================================================================

/// 已知明文，用于生成密码验证块
const VERIFY_PLAINTEXT: &[u8; 16] = b"ZBAK_VERIFY_OK!!";

/// 创建密码验证块: 加密已知数据, 返回 (nonce, GCM tag)
/// tag 存入文件头的 verify_tag 字段，用于快速密码验证
pub fn create_verify_block(verify_key: &[u8; 32]) -> SzResult<([u8; NONCE_SIZE], [u8; GCM_TAG_SIZE])> {
    let cipher = Aes256Gcm::new_from_slice(verify_key)
        .map_err(|e| SzError::Encryption(format!("AES 初始化失败: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, VERIFY_PLAINTEXT.as_ref())
        .map_err(|e| SzError::Encryption(format!("验证块加密失败: {}", e)))?;

    // GCM tag 是密文末尾 16 字节
    let tag_start = ciphertext.len() - GCM_TAG_SIZE;
    let mut tag = [0u8; GCM_TAG_SIZE];
    tag.copy_from_slice(&ciphertext[tag_start..]);

    Ok((nonce_bytes, tag))
}

/// 验证密码验证块: 尝试解密已知数据
pub fn check_verify_block(
    verify_key: &[u8; 32],
    nonce: &[u8; NONCE_SIZE],
    tag: &[u8; GCM_TAG_SIZE],
) -> bool {
    let cipher = match Aes256Gcm::new_from_slice(verify_key) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let nonce = Nonce::from_slice(nonce);

    match cipher.encrypt(nonce, VERIFY_PLAINTEXT.as_ref()) {
        Ok(ciphertext) => {
            let tag_start = ciphertext.len() - GCM_TAG_SIZE;
            // [安全] 常量时间比较：防止时序侧信道攻击
            constant_time_eq(&ciphertext[tag_start..], tag)
        }
        Err(_) => false,
    }
}

/// 常量时间字节比较：无论哪个位置不同，耗时一致
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ============================================================================
// AES-256-GCM 逐文件加密/解密
// ============================================================================

/// 加密数据块 (Zstd 压缩后的数据)
/// 返回: nonce(12) + ciphertext(含GCM tag)
pub fn encrypt_block(key: &[u8; 32], plaintext: &[u8]) -> SzResult<(Vec<u8>, [u8; NONCE_SIZE])> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SzError::Encryption(format!("AES 初始化失败: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| SzError::Encryption(format!("加密失败: {}", e)))?;

    Ok((ciphertext, nonce_bytes))
}

/// 解密数据块
/// 输入: nonce + ciphertext(含GCM tag)
pub fn decrypt_block(
    key: &[u8; 32],
    nonce: &[u8; NONCE_SIZE],
    ciphertext: &[u8],
) -> SzResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SzError::Decryption(format!("AES 初始化失败: {}", e)))?;

    let nonce = Nonce::from_slice(nonce);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| SzError::Decryption("数据块解密失败（数据损坏或密码错误）".into()))
}

/// 加密索引区
pub fn encrypt_index(key: &[u8; 32], plaintext: &[u8]) -> SzResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SzError::Encryption(format!("AES 初始化失败: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| SzError::Encryption(format!("索引加密失败: {}", e)))?;

    // 输出格式: nonce(12) + ciphertext(含tag)
    let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// 解密索引区
pub fn decrypt_index(key: &[u8; 32], data: &[u8]) -> SzResult<Vec<u8>> {
    if data.len() < NONCE_SIZE + GCM_TAG_SIZE {
        return Err(SzError::Decryption("索引数据太短".into()));
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SzError::Decryption(format!("AES 初始化失败: {}", e)))?;

    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| SzError::Decryption("索引解密失败（密码错误或数据损坏）".into()))
}

/// 生成随机 Salt
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill(&mut salt);
    salt
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_derivation() {
        let salt = [42u8; 16];
        let key1 = derive_master_key("test_password", &salt).unwrap();
        let key2 = derive_master_key("test_password", &salt).unwrap();
        assert_eq!(key1, key2); // 相同输入 → 相同输出

        let key3 = derive_master_key("different_password", &salt).unwrap();
        assert_ne!(key1, key3); // 不同密码 → 不同密钥
    }

    #[test]
    fn test_hkdf_file_keys_unique() {
        let master = [1u8; 32];
        let k0 = derive_file_key(&master, 0);
        let k1 = derive_file_key(&master, 1);
        let k2 = derive_file_key(&master, 2);
        assert_ne!(k0, k1);
        assert_ne!(k1, k2);
        assert_ne!(k0, k2);
    }

    #[test]
    fn test_verify_block_correct_password() {
        let master = [99u8; 32];
        let verify_key = derive_verify_key(&master);
        let (nonce, tag) = create_verify_block(&verify_key).unwrap();
        assert!(check_verify_block(&verify_key, &nonce, &tag));
    }

    #[test]
    fn test_verify_block_wrong_password() {
        let master1 = [99u8; 32];
        let master2 = [88u8; 32];
        let verify_key1 = derive_verify_key(&master1);
        let verify_key2 = derive_verify_key(&master2);
        let (nonce, tag) = create_verify_block(&verify_key1).unwrap();
        assert!(!check_verify_block(&verify_key2, &nonce, &tag));
    }

    #[test]
    fn test_encrypt_decrypt_block() {
        let key = [42u8; 32];
        let data = b"Hello, ZBAK encryption!";
        let (ciphertext, nonce) = encrypt_block(&key, data).unwrap();
        let decrypted = decrypt_block(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(&decrypted, data);
    }

    #[test]
    fn test_encrypt_decrypt_block_wrong_key() {
        let key1 = [42u8; 32];
        let key2 = [43u8; 32];
        let data = b"secret data";
        let (ciphertext, nonce) = encrypt_block(&key1, data).unwrap();
        assert!(decrypt_block(&key2, &nonce, &ciphertext).is_err());
    }

    #[test]
    fn test_encrypt_decrypt_index() {
        let key = [55u8; 32];
        let data = b"index data with file entries";
        let encrypted = encrypt_index(&key, data).unwrap();
        let decrypted = decrypt_index(&key, &encrypted).unwrap();
        assert_eq!(&decrypted, data);
    }

    #[test]
    fn test_encrypt_decrypt_index_wrong_key() {
        let key1 = [55u8; 32];
        let key2 = [66u8; 32];
        let data = b"index data";
        let encrypted = encrypt_index(&key1, data).unwrap();
        assert!(decrypt_index(&key2, &encrypted).is_err());
    }
}
