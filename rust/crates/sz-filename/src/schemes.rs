//! 文件名混淆方案实现

use chrono::Utc;
use rand::Rng;
use sha2::{Sha256, Digest};

use sz_core::{MappingEntry, ObfuscationScheme, SzResult};
use sz_crypto::AesEncryptor;

/// 文件名混淆器
pub struct FilenameObfuscator {
    scheme: ObfuscationScheme,
    encryption_key: Option<[u8; 32]>,
    counter: u32,
}

impl FilenameObfuscator {
    /// 创建新的混淆器
    pub fn new(scheme: ObfuscationScheme) -> Self {
        Self {
            scheme,
            encryption_key: None,
            counter: 0,
        }
    }

    /// 创建带加密密钥的混淆器（用于加密模式）
    pub fn with_encryption_key(scheme: ObfuscationScheme, key: [u8; 32]) -> Self {
        Self {
            scheme,
            encryption_key: Some(key),
            counter: 0,
        }
    }

    /// 混淆文件名
    pub fn obfuscate(&mut self, original_name: &str) -> String {
        self.counter += 1;
        
        match self.scheme {
            ObfuscationScheme::Sequential => self.sequential(original_name),
            ObfuscationScheme::DateSequential => self.date_sequential(original_name),
            ObfuscationScheme::Random => self.random(original_name),
            ObfuscationScheme::Hash => self.hash(original_name),
            ObfuscationScheme::Encrypted => self.encrypted(original_name),
        }
    }

    /// 批量混淆文件名
    pub fn obfuscate_batch(
        &mut self,
        original_names: &[String],
        archive_path: &str,
    ) -> Vec<MappingEntry> {
        original_names
            .iter()
            .map(|name| {
                let obfuscated = self.obfuscate(name);
                MappingEntry::new(
                    name.clone(),
                    obfuscated,
                    archive_path.to_string(),
                )
            })
            .collect()
    }

    /// 还原文件名（仅加密模式支持）
    pub fn restore(&self, obfuscated_name: &str) -> SzResult<String> {
        match self.scheme {
            ObfuscationScheme::Encrypted => self.decrypt_name(obfuscated_name),
            _ => Err(sz_core::SzError::InvalidArgument(
                "只有加密模式支持还原文件名".to_string(),
            )),
        }
    }

    // ========== 各种混淆方案实现 ==========

    /// 序号模式: 001.dat, 002.dat
    fn sequential(&self, _original: &str) -> String {
        format!("{:03}.dat", self.counter)
    }

    /// 日期序号模式: 20240115_001.dat
    fn date_sequential(&self, _original: &str) -> String {
        let date = Utc::now().format("%Y%m%d");
        format!("{}_{:03}.dat", date, self.counter)
    }

    /// 随机字符模式: a7x2k9m3.dat
    fn random(&self, _original: &str) -> String {
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789"
            .chars()
            .collect();
        let mut rng = rand::thread_rng();
        let random_str: String = (0..8)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect();
        format!("{}.dat", random_str)
    }

    /// 哈希模式: 8a3c2b1f.dat (SHA256前8位)
    fn hash(&self, original: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(original.as_bytes());
        let result = hasher.finalize();
        let hex: String = result.iter().take(4).map(|b| format!("{:02x}", b)).collect();
        format!("{}.dat", hex)
    }

    /// 加密模式: Base64(AES(原名)).enc
    fn encrypted(&self, original: &str) -> String {
        if let Some(key) = &self.encryption_key {
            let encryptor = AesEncryptor::new(key);
            match encryptor.encrypt_string(original) {
                Ok(encrypted) => {
                    // URL-safe Base64 以避免文件名问题
                    let safe_name = encrypted
                        .replace('+', "-")
                        .replace('/', "_")
                        .replace('=', "");
                    format!("{}.enc", safe_name)
                }
                Err(_) => self.random(original), // 回退到随机模式
            }
        } else {
            self.random(original) // 没有密钥时回退到随机模式
        }
    }

    /// 解密文件名
    fn decrypt_name(&self, obfuscated: &str) -> SzResult<String> {
        let key = self.encryption_key.ok_or_else(|| {
            sz_core::SzError::InvalidArgument("需要加密密钥".to_string())
        })?;

        // 移除 .enc 后缀
        let encrypted = obfuscated
            .strip_suffix(".enc")
            .ok_or_else(|| sz_core::SzError::InvalidArgument("无效的加密文件名".to_string()))?;

        // 还原 Base64 字符
        let mut encrypted = encrypted
            .replace('-', "+")
            .replace('_', "/");

        // 还原 Base64 padding
        let pad_len = (4 - encrypted.len() % 4) % 4;
        for _ in 0..pad_len {
            encrypted.push('=');
        }

        let encryptor = AesEncryptor::new(&key);
        encryptor.decrypt_string(&encrypted)
    }
}

/// 从映射表查找原始文件名
pub fn lookup_original_name<'a>(
    mappings: &'a [MappingEntry],
    obfuscated_name: &str,
) -> Option<&'a str> {
    mappings
        .iter()
        .find(|m| m.obfuscated_name == obfuscated_name)
        .map(|m| m.original_name.as_str())
}

/// 从映射表查找混淆后的文件名
pub fn lookup_obfuscated_name<'a>(
    mappings: &'a [MappingEntry],
    original_name: &str,
) -> Option<&'a str> {
    mappings
        .iter()
        .find(|m| m.original_name == original_name)
        .map(|m| m.obfuscated_name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_scheme() {
        let mut obfuscator = FilenameObfuscator::new(ObfuscationScheme::Sequential);
        assert_eq!(obfuscator.obfuscate("test.txt"), "001.dat");
        assert_eq!(obfuscator.obfuscate("image.jpg"), "002.dat");
        assert_eq!(obfuscator.obfuscate("document.pdf"), "003.dat");
    }

    #[test]
    fn test_date_sequential_scheme() {
        let mut obfuscator = FilenameObfuscator::new(ObfuscationScheme::DateSequential);
        let name = obfuscator.obfuscate("test.txt");
        assert!(name.ends_with("_001.dat"));
        assert!(name.len() == 16); // YYYYMMDD_001.dat
    }

    #[test]
    fn test_random_scheme() {
        let mut obfuscator = FilenameObfuscator::new(ObfuscationScheme::Random);
        let name = obfuscator.obfuscate("test.txt");
        assert!(name.ends_with(".dat"));
        assert_eq!(name.len(), 12); // 8 chars + .dat
    }

    #[test]
    fn test_hash_scheme() {
        let mut obfuscator = FilenameObfuscator::new(ObfuscationScheme::Hash);
        let name1 = obfuscator.obfuscate("test.txt");
        let name2 = obfuscator.obfuscate("test.txt");
        // 相同输入产生相同输出
        assert_eq!(name1, name2);
        assert!(name1.ends_with(".dat"));
    }

    #[test]
    fn test_encrypted_scheme() {
        let key = sz_crypto::generate_key();
        let mut obfuscator = FilenameObfuscator::with_encryption_key(
            ObfuscationScheme::Encrypted,
            key,
        );
        
        let original = "secret_file.txt";
        let obfuscated = obfuscator.obfuscate(original);
        assert!(obfuscated.ends_with(".enc"));
        
        // 验证可以还原
        let restored = obfuscator.restore(&obfuscated).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_batch_obfuscation() {
        let mut obfuscator = FilenameObfuscator::new(ObfuscationScheme::Sequential);
        let files = vec![
            "file1.txt".to_string(),
            "file2.jpg".to_string(),
            "file3.pdf".to_string(),
        ];
        
        let mappings = obfuscator.obfuscate_batch(&files, "/archive.7z");
        
        assert_eq!(mappings.len(), 3);
        assert_eq!(mappings[0].original_name, "file1.txt");
        assert_eq!(mappings[0].obfuscated_name, "001.dat");
        assert_eq!(mappings[1].obfuscated_name, "002.dat");
        assert_eq!(mappings[2].obfuscated_name, "003.dat");
    }
}
