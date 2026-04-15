//! 密码处理工具

use sha2::{Sha256, Digest};
use rand::Rng;

/// 生成随机密码
/// 
/// # Arguments
/// * `length` - 密码长度
/// * `include_symbols` - 是否包含特殊字符
pub fn generate_random_password(length: usize, include_symbols: bool) -> String {
    const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
    const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const DIGITS: &str = "0123456789";
    const SYMBOLS: &str = "!@#$%^&*()-_=+[]{}|;:,.<>?";

    let mut rng = rand::thread_rng();

    // 确定必选字符类别
    let categories: Vec<&str> = if include_symbols {
        vec![LOWER, UPPER, DIGITS, SYMBOLS]
    } else {
        vec![LOWER, UPPER, DIGITS]
    };

    // 长度不足以覆盖所有类别时，回退到纯随机
    let min_required = categories.len();
    if length < min_required {
        let all: Vec<char> = categories.iter().flat_map(|s| s.chars()).collect();
        return (0..length)
            .map(|_| all[rng.gen_range(0..all.len())])
            .collect();
    }

    // 每个类别至少取一个字符
    let mut password: Vec<char> = categories.iter()
        .map(|cat| {
            let chars: Vec<char> = cat.chars().collect();
            chars[rng.gen_range(0..chars.len())]
        })
        .collect();

    // 剩余位置从全字符集随机填充
    let all: Vec<char> = categories.iter().flat_map(|s| s.chars()).collect();
    for _ in 0..(length - min_required) {
        password.push(all[rng.gen_range(0..all.len())]);
    }

    // Fisher-Yates 洗牌，避免必选字符固定在前几位
    for i in (1..password.len()).rev() {
        let j = rng.gen_range(0..=i);
        password.swap(i, j);
    }

    password.into_iter().collect()
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
        assert_eq!(calculate_password_strength("Password1"), 3);
        assert_eq!(calculate_password_strength("Password1!"), 4);
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
