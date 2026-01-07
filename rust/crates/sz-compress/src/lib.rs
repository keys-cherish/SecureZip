//! SecureZip 压缩模块
//! 
//! 提供多种压缩解压功能:
//! - Tar + Zstd + AES-256-GCM: 主推方案，高效可靠
//! - 7z + LZMA2: 标准 7z 格式（兼容模式）
//! - 专属加密格式: 7z + Zstd + AES-256-GCM
//! - 智能解压: 自动检测格式

pub mod sevenz;
pub mod encrypted;
pub mod tar_zstd;
pub mod smart_decompress;

pub use sevenz::*;
pub use encrypted::*;
pub use tar_zstd::*;
pub use smart_decompress::*;
