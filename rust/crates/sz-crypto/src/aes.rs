//! AES-256-GCM 加密解密实现

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::Rng;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use sz_core::{SzError, SzResult};

/// AES-256-GCM 加密器
pub struct AesEncryptor {
    cipher: Aes256Gcm,
}

impl AesEncryptor {
    /// 从密钥创建加密器
    /// 
    /// # Arguments
    /// * `key` - 32 字节密钥
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(key).expect("密钥长度必须为32字节");
        Self { cipher }
    }

    /// 从密码派生密钥并创建加密器
    pub fn from_password(password: &str, salt: &[u8]) -> SzResult<Self> {
        let key = derive_key_from_password(password, salt)?;
        Ok(Self::new(&key))
    }

    /// 加密数据
    /// 
    /// 返回 Base64 编码的密文（包含 nonce）
    pub fn encrypt(&self, plaintext: &[u8]) -> SzResult<String> {
        // 生成随机 nonce (12 bytes)
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| SzError::Encryption(e.to_string()))?;

        // 组合 nonce + ciphertext
        let mut combined = nonce_bytes.to_vec();
        combined.extend(ciphertext);

        // Base64 编码
        Ok(BASE64.encode(combined))
    }

    /// 加密字符串
    pub fn encrypt_string(&self, plaintext: &str) -> SzResult<String> {
        self.encrypt(plaintext.as_bytes())
    }

    /// 解密数据
    /// 
    /// # Arguments
    /// * `encrypted` - Base64 编码的密文（包含 nonce）
    pub fn decrypt(&self, encrypted: &str) -> SzResult<Vec<u8>> {
        // Base64 解码
        let combined = BASE64
            .decode(encrypted)
            .map_err(|e| SzError::Decryption(format!("Base64解码失败: {}", e)))?;

        if combined.len() < 13 {
            return Err(SzError::Decryption("密文太短".to_string()));
        }

        // 分离 nonce 和 ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // 解密
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| SzError::WrongPassword)
    }

    /// 解密为字符串
    pub fn decrypt_string(&self, encrypted: &str) -> SzResult<String> {
        let bytes = self.decrypt(encrypted)?;
        String::from_utf8(bytes).map_err(|e| SzError::Decryption(e.to_string()))
    }
}

/// 从密码派生 32 字节密钥
pub fn derive_key_from_password(password: &str, salt: &[u8]) -> SzResult<[u8; 32]> {
    use sha2::{Sha256, Digest};
    use argon2::Argon2;

    // 使用 Argon2id 派生密钥
    let argon2 = Argon2::default();
    let mut output_key = [0u8; 32];
    
    argon2.hash_password_into(
        password.as_bytes(),
        salt,
        &mut output_key
    ).map_err(|e| SzError::Encryption(format!("Argon2 密钥派生失败: {}", e)))?;

    Ok(output_key)
}

/// 生成随机 salt
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill(&mut salt);
    salt
}

/// 生成随机密钥
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = generate_key();
        let encryptor = AesEncryptor::new(&key);

        let plaintext = "Hello, SecureZip!";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();
        let decrypted = encryptor.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = generate_key();
        let key2 = generate_key();

        let encryptor1 = AesEncryptor::new(&key1);
        let encryptor2 = AesEncryptor::new(&key2);

        let encrypted = encryptor1.encrypt_string("secret").unwrap();
        let result = encryptor2.decrypt_string(&encrypted);

        assert!(result.is_err());
    }

    #[test]
    fn test_password_derived_key() {
        let salt = generate_salt();
        let encryptor = AesEncryptor::from_password("my_password", &salt).unwrap();

        let plaintext = "Secret data";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();
        let decrypted = encryptor.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }
}
