//! 密码处理工具

use sha2::{Sha256, Digest};
use rand::Rng;

/// 生成随机密码
/// 
/// # Arguments
/// * `length` - 密码长度
/// * `include_symbols` - 是否包含特殊字符
pub fn generate_random_password(length: usize, include_symbols: bool) -> String {
    let chars: Vec<char> = if include_symbols {
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+[]{}|;:,.<>?"
            .chars()
            .collect()
    } else {
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars()
            .collect()
    };

    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

/// 计算密码强度 (0-4)
/// 
/// 0 - 非常弱
/// 1 - 弱
/// 2 - 中等
/// 3 - 强
/// 4 - 非常强
pub fn calculate_password_strength(password: &str) -> u8 {
    let mut score: u8 = 0;

    // 长度检查
    if password.len() >= 8 {
        score += 1;
    }
    if password.len() >= 12 {
        score += 1;
    }

    // 包含小写字母
    if password.chars().any(|c| c.is_ascii_lowercase()) {
        score += 1;
    }

    // 包含大写字母
    if password.chars().any(|c| c.is_ascii_uppercase()) {
        score += 1;
    }

    // 包含数字
    if password.chars().any(|c| c.is_ascii_digit()) {
        score += 1;
    }

    // 包含特殊字符
    if password.chars().any(|c| !c.is_alphanumeric()) {
        score += 1;
    }

    // 转换为 0-4 范围
    match score {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4,
    }
}

/// 计算字符串的 SHA256 哈希
pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// 计算字符串的 SHA256 哈希（前 N 个字符）
pub fn sha256_hash_short(input: &str, length: usize) -> String {
    let hash = sha256_hash(input);
    hash.chars().take(length).collect()
}

/// 简单的 hex 编码模块
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_password() {
        let password = generate_random_password(16, true);
        assert_eq!(password.len(), 16);
    }

    #[test]
    fn test_password_strength() {
        assert_eq!(calculate_password_strength("123"), 0);
        assert_eq!(calculate_password_strength("password"), 1);
        assert_eq!(calculate_password_strength("Password1"), 2);
        assert_eq!(calculate_password_strength("Password1!"), 3);
        assert_eq!(calculate_password_strength("MyP@ssw0rd!Long"), 4);
    }

    #[test]
    fn test_sha256_hash() {
        let hash = sha256_hash("test");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_sha256_hash_short() {
        let hash = sha256_hash_short("test", 8);
        assert_eq!(hash.len(), 8);
    }
}
