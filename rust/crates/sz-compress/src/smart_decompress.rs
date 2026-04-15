//! 智能解压模块
//!
//! 自动检测压缩包格式并选择合适的解压方法
//!
//! 支持的格式：
//! - .zbak (ZBAK): SecureZip 备份格式（主推）
//! - .sz7z (SZ7Z): 旧版 7z + Zstd + 可选 AES-256-GCM（向后兼容解压）
//! - .7z: 标准 7z 格式（兼容模式）
//! - .szp (SZPK): 旧版 Tar + Zstd（仅向后兼容解压）

use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;

use sz_core::{SzError, SzResult};
use crate::{EncryptedCompressor, zbak::ZbakReader};
use crate::zbak::split;
#[cfg(feature = "legacy-7z")]
use crate::Decompressor;

/// 压缩包格式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArchiveFormat {
    /// SecureZip 备份格式（主推）
    Zbak,
    /// 旧版 SecureZip 7z.zstd 专属格式
    Sz7z,
    /// 标准 7z 格式（兼容模式）
    SevenZ,
    /// 旧版 SecureZip Package（仅向后兼容解压）
    SzpLegacy,
    /// 未知格式
    Unknown,
}

impl ArchiveFormat {
    /// 获取格式描述
    pub fn description(&self) -> &'static str {
        match self {
            ArchiveFormat::Zbak => "SecureZip 备份格式 (Zstd + AES-256-GCM + 恢复记录)",
            ArchiveFormat::Sz7z => "旧版 SecureZip 专属格式 (7z + Zstd + 可选AES)",
            ArchiveFormat::SevenZ => "标准 7z 格式",
            ArchiveFormat::SzpLegacy => "旧版 SecureZip Package (Tar + Zstd)",
            ArchiveFormat::Unknown => "未知格式",
        }
    }

    /// 格式代码（用于 FFI）
    /// 0=未知, 1=sz7z, 2=7z, 3=旧版szp, 4=zbak
    pub fn to_code(&self) -> i32 {
        match self {
            ArchiveFormat::Unknown => 0,
            ArchiveFormat::Sz7z => 1,
            ArchiveFormat::SevenZ => 2,
            ArchiveFormat::SzpLegacy => 3,
            ArchiveFormat::Zbak => 4,
        }
    }

    pub fn from_code(code: i32) -> Self {
        match code {
            1 => ArchiveFormat::Sz7z,
            2 => ArchiveFormat::SevenZ,
            3 => ArchiveFormat::SzpLegacy,
            4 => ArchiveFormat::Zbak,
            _ => ArchiveFormat::Unknown,
        }
    }
}

/// 魔数定义
const MAGIC_SZPK: &[u8; 4] = b"SZPK";  // 旧版 .szp 文件
const MAGIC_SZ7Z: &[u8; 4] = b"SZ7Z";  // .sz7z 文件
const MAGIC_ZBAK: &[u8; 4] = b"ZBAK";  // .zbak 文件
const MAGIC_7Z: &[u8; 6] = b"7z\xBC\xAF\x27\x1C";  // 标准 7z

/// 智能解压器
pub struct SmartDecompressor;

impl SmartDecompressor {
    pub fn new() -> Self {
        Self
    }

    /// 检测压缩包格式（支持分卷自动检测）
    pub fn detect_format(archive_path: &str) -> SzResult<ArchiveFormat> {
        // 如果是分卷文件，需要读取第一个分卷的头部
        let actual_path = if split::is_split_volume(archive_path) {
            if let Some(volumes) = split::detect_volumes(archive_path) {
                volumes[0].clone()
            } else {
                archive_path.to_string()
            }
        } else {
            archive_path.to_string()
        };

        let path = Path::new(&actual_path);
        if !path.exists() {
            return Err(SzError::FileNotFound(archive_path.to_string()));
        }

        let mut file = BufReader::new(File::open(path)?);
        let mut header = [0u8; 6];
        let bytes_read = file.read(&mut header)?;

        if bytes_read < 4 {
            return Ok(ArchiveFormat::Unknown);
        }

        // 优先按魔数判断
        if &header[0..4] == MAGIC_ZBAK {
            return Ok(ArchiveFormat::Zbak);
        }
        if &header[0..4] == MAGIC_SZ7Z {
            return Ok(ArchiveFormat::Sz7z);
        }
        if &header[0..4] == MAGIC_SZPK {
            return Ok(ArchiveFormat::SzpLegacy);
        }
        if bytes_read >= 6 && &header[0..6] == MAGIC_7Z {
            return Ok(ArchiveFormat::SevenZ);
        }

        // 按扩展名回退
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "zbak" => Ok(ArchiveFormat::Zbak),
            "sz7z" => Ok(ArchiveFormat::Sz7z),
            "szp" => Ok(ArchiveFormat::SzpLegacy),
            "7z" => Ok(ArchiveFormat::SevenZ),
            _ => Ok(ArchiveFormat::Unknown),
        }
    }

    /// 检查压缩包是否需要密码
    pub fn requires_password(archive_path: &str) -> SzResult<bool> {
        let format = Self::detect_format(archive_path)?;

        match format {
            ArchiveFormat::Zbak => {
                ZbakReader::requires_password(archive_path)
            }
            ArchiveFormat::Sz7z => {
                EncryptedCompressor::requires_password(archive_path)
            }
            ArchiveFormat::SzpLegacy => {
                Ok(true)
            }
            ArchiveFormat::SevenZ => {
                Ok(false)
            }
            ArchiveFormat::Unknown => {
                Err(SzError::InvalidArgument("无法识别的压缩包格式".to_string()))
            }
        }
    }

    /// 验证密码
    pub fn verify_password(archive_path: &str, password: &str) -> SzResult<bool> {
        let format = Self::detect_format(archive_path)?;

        match format {
            ArchiveFormat::Zbak => {
                ZbakReader::verify_password(archive_path, password)
            }
            ArchiveFormat::Sz7z => {
                let compressor = EncryptedCompressor::default();
                compressor.verify_password(archive_path, password)
            }
            ArchiveFormat::SzpLegacy | ArchiveFormat::SevenZ => {
                Ok(true)
            }
            ArchiveFormat::Unknown => {
                Err(SzError::InvalidArgument("无法识别的压缩包格式".to_string()))
            }
        }
    }

    /// 智能解压（自动处理分卷）
    pub fn decompress<F>(
        archive_path: &str,
        output_dir: &str,
        password: Option<&str>,
        mut progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(u64, u64, &str),
    {
        // 自动检测分卷，拼接为临时文件后解压
        let (actual_path, temp_joined) = Self::resolve_split_volumes(archive_path)?;
        let actual_path_str = actual_path.to_string_lossy().to_string();

        let format = Self::detect_format(&actual_path_str)?;

        let result = match format {
            ArchiveFormat::Zbak => {
                let reader = ZbakReader::new();
                reader.decompress(&actual_path_str, output_dir, password, progress_callback)
            }
            ArchiveFormat::Sz7z => {
                let compressor = EncryptedCompressor::default();
                compressor.decompress(&actual_path_str, output_dir, password, progress_callback)
            }
            ArchiveFormat::SzpLegacy => {
                Err(SzError::InvalidArgument(
                    "旧版 .szp 格式已废弃，请使用旧版本软件解压".to_string()
                ))
            }
            ArchiveFormat::SevenZ => {
                #[cfg(feature = "legacy-7z")]
                {
                    let decompressor = Decompressor::new();
                    decompressor.decompress(&actual_path_str, output_dir, password, |progress| {
                        progress_callback(progress.processed_bytes, progress.total_bytes, &progress.current_file);
                    })
                }
                #[cfg(not(feature = "legacy-7z"))]
                {
                    Err(SzError::UnsupportedFormat("7z 格式支持未编译 (需要 legacy-7z feature)".into()))
                }
            }
            ArchiveFormat::Unknown => {
                #[cfg(feature = "legacy-7z")]
                {
                    let decompressor = Decompressor::new();
                    decompressor.decompress(&actual_path_str, output_dir, password, |progress| {
                        progress_callback(progress.processed_bytes, progress.total_bytes, &progress.current_file);
                    })
                }
                #[cfg(not(feature = "legacy-7z"))]
                {
                    Err(SzError::UnsupportedFormat("未知格式且 7z 支持未编译".into()))
                }
            }
        };

        // 清理临时拼接文件
        if let Some(temp) = temp_joined {
            let _ = std::fs::remove_file(temp);
        }

        result
    }

    /// 处理分卷: 如果是分卷文件，拼接为临时文件并返回路径
    /// 返回 (实际文件路径, 临时文件路径Option)
    fn resolve_split_volumes(archive_path: &str) -> SzResult<(std::path::PathBuf, Option<std::path::PathBuf>)> {
        if let Some(volumes) = split::detect_volumes(archive_path) {
            // 是分卷文件，需要拼接
            let _base = split::base_path_from_volume(archive_path)
                .unwrap_or_else(|| archive_path.to_string());
            let temp_path = std::env::temp_dir().join(format!(
                "zbak_join_{}.zbak",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ));
            let temp_str = temp_path.to_string_lossy().to_string();
            split::join_volumes(&volumes, &temp_str)?;
            Ok((temp_path.clone(), Some(temp_path)))
        } else {
            Ok((std::path::PathBuf::from(archive_path), None))
        }
    }

    /// 列出压缩包内容
    pub fn list_contents(archive_path: &str, password: Option<&str>) -> SzResult<Vec<String>> {
        let format = Self::detect_format(archive_path)?;

        match format {
            ArchiveFormat::Zbak => {
                let entries = ZbakReader::list_contents(archive_path, password)?;
                Ok(entries.into_iter().map(|e| e.path).collect())
            }
            #[cfg(feature = "legacy-7z")]
            ArchiveFormat::SevenZ => {
                let decompressor = Decompressor::new();
                decompressor.list_contents(archive_path)
            }
            _ => {
                Err(SzError::InvalidArgument("此格式暂不支持列出内容".to_string()))
            }
        }
    }
}

impl Default for SmartDecompressor {
    fn default() -> Self {
        Self::new()
    }
}
