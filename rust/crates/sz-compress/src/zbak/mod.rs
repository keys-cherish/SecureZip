//! .zbak 备份格式模块
//!
//! SecureZip 新一代备份格式，特性:
//! - 非固实结构：逐文件 Zstd 压缩 + AES-256-GCM 加密
//! - HKDF 逐文件子密钥：单文件泄露不影响其他文件
//! - 密码验证块：错误密码立即报错
//! - Reed-Solomon 恢复卷：可修复损坏数据
//! - WebDAV 流式分块上传：内存占用低
//! - 随机访问：可提取单个文件而无需全部解压
//! - 分卷压缩：大文件拆分为多个固定大小分卷

pub mod format;
pub mod crypto;
pub mod writer;
pub mod reader;
pub mod recovery;
pub mod chunker;
pub mod uploader;
pub mod split;

pub use format::{ZbakHeader, ZbakIndexEntry, ZbakResult, MAGIC as ZBAK_MAGIC};
pub use writer::ZbakWriter;
pub use reader::ZbakReader;
pub use recovery::RecoveryGenerator;
pub use chunker::{ChunkInfo, BackupManifest};
pub use split::{split_file, detect_volumes, join_volumes, is_split_volume, base_path_from_volume};
