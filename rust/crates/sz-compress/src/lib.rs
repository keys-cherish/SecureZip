//! SecureZip 压缩模块
//!
//! 提供多种压缩解压功能:
//! - .zbak 备份格式（主推）: Zstd 逐文件压缩 + AES-256-GCM + HKDF + Reed-Solomon
//! - .sz7z 旧版专属格式: 7z + Zstd + 可选 AES-256-GCM（仅保留解压兼容）
//! - 7z + LZMA2: 标准 7z 格式（兼容模式）
//! - 智能解压: 自动检测格式

pub mod zbak;
#[cfg(feature = "legacy-7z")]
pub mod sevenz;
pub mod encrypted;
pub mod smart_decompress;

pub use zbak::{ZbakWriter, ZbakReader, ZbakHeader, ZbakResult, ZbakIndexEntry};
#[cfg(feature = "legacy-7z")]
pub use sevenz::*;
pub use encrypted::*;
pub use smart_decompress::*;
