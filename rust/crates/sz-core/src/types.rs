//! 公共类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 压缩选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressOptions {
    /// 压缩密码（可选）
    pub password: Option<String>,
    /// 是否启用文件名混淆
    pub enable_obfuscation: bool,
    /// 混淆方案
    pub obfuscation_scheme: ObfuscationScheme,
    /// 压缩级别 (1-9)
    pub compression_level: u8,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            password: None,
            enable_obfuscation: false,
            obfuscation_scheme: ObfuscationScheme::Sequential,
            compression_level: 6,
        }
    }
}

/// 文件名混淆方案
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObfuscationScheme {
    /// 序号模式: 001.dat, 002.dat
    Sequential,
    /// 日期序号模式: 20240115_001.dat
    DateSequential,
    /// 随机字符模式: a7x2k9m3.dat
    Random,
    /// 哈希模式: 8a3c2b1f.dat (SHA256前8位)
    Hash,
    /// 加密模式: Base64(AES(原名)).enc
    Encrypted,
}

/// 压缩进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressProgress {
    /// 进度百分比 (0.0 - 1.0)
    pub progress: f64,
    /// 已处理字节数
    pub processed_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 处理速度 (字节/秒)
    pub speed_bytes_per_second: f64,
    /// 预计剩余时间 (秒)
    pub estimated_remaining_seconds: u64,
    /// 当前处理的文件名
    pub current_file: String,
}

/// 压缩结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressResult {
    /// 是否成功
    pub success: bool,
    /// 输出文件路径
    pub output_path: String,
    /// 原始大小
    pub original_size: u64,
    /// 压缩后大小
    pub compressed_size: u64,
    /// 耗时 (毫秒)
    pub duration_ms: u64,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 密码条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub name: String,
    pub password: String,
    pub created_at: DateTime<Utc>,
    pub remark: Option<String>,
}

impl PasswordEntry {
    pub fn new(name: String, password: String, remark: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            password,
            created_at: Utc::now(),
            remark,
        }
    }
}

/// 文件名映射条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingEntry {
    pub id: String,
    pub original_name: String,
    pub obfuscated_name: String,
    pub archive_path: String,
    pub created_at: DateTime<Utc>,
}

impl MappingEntry {
    pub fn new(original_name: String, obfuscated_name: String, archive_path: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            original_name,
            obfuscated_name,
            archive_path,
            created_at: Utc::now(),
        }
    }
}

/// 后缀密码映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionPasswordMapping {
    pub id: String,
    pub extension: String,
    pub password_id: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

impl ExtensionPasswordMapping {
    pub fn new(extension: String, password_id: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            extension,
            password_id,
            description,
            created_at: Utc::now(),
        }
    }
}

/// WebDAV 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavConfig {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub remote_path: String,
}

impl WebDavConfig {
    pub fn is_configured(&self) -> bool {
        !self.server_url.is_empty() && !self.username.is_empty() && !self.password.is_empty()
    }
}

/// WebDAV 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavFileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub last_modified: Option<DateTime<Utc>>,
}

/// 应用备份数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppBackupData {
    pub version: u32,
    pub timestamp: DateTime<Utc>,
    pub passwords: Vec<PasswordEntry>,
    pub filename_mappings: Vec<MappingEntry>,
    pub extension_mappings: Vec<ExtensionPasswordMapping>,
}
