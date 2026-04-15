//! 错误类型定义

use thiserror::Error;

/// SecureZip 统一错误类型
#[derive(Error, Debug)]
pub enum SzError {
    #[error("压缩错误: {0}")]
    Compress(String),

    #[error("解压错误: {0}")]
    Decompress(String),

    #[error("加密错误: {0}")]
    Encryption(String),

    #[error("解密错误: {0}")]
    Decryption(String),

    #[error("密码错误")]
    WrongPassword,

    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(String),

    #[error("WebDAV错误: {0}")]
    WebDav(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("无效参数: {0}")]
    InvalidArgument(String),

    #[error("操作取消")]
    Cancelled,

    #[error("数据损坏: {0}")]
    DataCorrupted(String),

    #[error("恢复失败: {0}")]
    RecoveryFailed(String),

    #[error("格式不支持: {0}")]
    UnsupportedFormat(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

/// 统一结果类型
pub type SzResult<T> = Result<T, SzError>;

impl From<serde_json::Error> for SzError {
    fn from(e: serde_json::Error) -> Self {
        SzError::Serialization(e.to_string())
    }
}
