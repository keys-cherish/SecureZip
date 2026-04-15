//! 照片增量备份模块
//!
//! 核心功能：
//! - 扫描照片目录，提取 EXIF 元数据
//! - 维护本地索引，跟踪已备份文件
//! - 增量检测：只备份新增/修改的照片
//! - 隐私保护：EXIF 擦除 + 文件名混淆 + 客户端加密

pub mod scanner;
pub mod index;
pub mod diff;
pub mod exif;
pub mod privacy;

pub use scanner::PhotoScanner;
pub use index::{SyncIndex, PhotoRecord};
pub use diff::{DiffResult, FileChange};
pub use exif::ExifInfo;
pub use privacy::PrivacyProcessor;
