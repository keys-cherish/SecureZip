//! 智能解压模块
//!
//! 自动检测压缩包格式并选择合适的解压方法
//! 
//! 支持的格式：
//! - .szp (SZPK): Tar + Zstd + 可选 AES-256-GCM
//! - .sz7z (SZ7Z): 7z + Zstd + AES-256-GCM（专属格式）
//! - .tar.zst: 标准 Tar + Zstd（无加密）
//! - .7z: 标准 7z 格式

use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;

use sz_core::{SzError, SzResult};
use crate::{TarZstdCompressor, EncryptedCompressor, Decompressor};

/// 压缩包格式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArchiveFormat {
    /// SecureZip Package: Tar + Zstd + 可选 AES-256-GCM
    Szp,
    /// SecureZip 7z: 7z + Zstd + AES-256-GCM（专属加密格式）
    Sz7z,
    /// 标准 Tar + Zstd
    TarZstd,
    /// 标准 7z 格式
    SevenZ,
    /// 未知格式
    Unknown,
}

impl ArchiveFormat {
    /// 获取格式描述
    pub fn description(&self) -> &'static str {
        match self {
            ArchiveFormat::Szp => "SecureZip Package (Tar + Zstd)",
            ArchiveFormat::Sz7z => "SecureZip 专属加密格式 (7z + Zstd + AES)",
            ArchiveFormat::TarZstd => "标准 Tar + Zstd 格式",
            ArchiveFormat::SevenZ => "标准 7z 格式",
            ArchiveFormat::Unknown => "未知格式",
        }
    }
}

/// 魔数定义
const MAGIC_SZPK: &[u8; 4] = b"SZPK";  // .szp 文件
const MAGIC_SZ7Z: &[u8; 4] = b"SZ7Z";  // .sz7z 文件
const MAGIC_7Z: &[u8; 6] = b"7z\xBC\xAF\x27\x1C";  // 标准 7z
const MAGIC_ZSTD: &[u8; 4] = &[0x28, 0xB5, 0x2F, 0xFD];  // Zstd 帧魔数

/// 智能解压器
pub struct SmartDecompressor;

impl SmartDecompressor {
    pub fn new() -> Self {
        Self
    }

    /// 检测压缩包格式
    pub fn detect_format(archive_path: &str) -> SzResult<ArchiveFormat> {
        let path = Path::new(archive_path);
        if !path.exists() {
            return Err(SzError::FileNotFound(archive_path.to_string()));
        }

        let mut file = BufReader::new(File::open(path)?);
        let mut header = [0u8; 6];
        let bytes_read = file.read(&mut header)?;

        if bytes_read < 4 {
            return Ok(ArchiveFormat::Unknown);
        }

        // 检查各种魔数
        if &header[0..4] == MAGIC_SZPK {
            return Ok(ArchiveFormat::Szp);
        }

        if &header[0..4] == MAGIC_SZ7Z {
            return Ok(ArchiveFormat::Sz7z);
        }

        if bytes_read >= 6 && &header[0..6] == MAGIC_7Z {
            return Ok(ArchiveFormat::SevenZ);
        }

        if &header[0..4] == MAGIC_ZSTD {
            return Ok(ArchiveFormat::TarZstd);
        }

        // 通过扩展名判断
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match ext.as_str() {
            "szp" => Ok(ArchiveFormat::Szp),
            "sz7z" => Ok(ArchiveFormat::Sz7z),
            "7z" => Ok(ArchiveFormat::SevenZ),
            "zst" | "zstd" => {
                // 检查是否是 .tar.zst
                if stem.ends_with(".tar") {
                    Ok(ArchiveFormat::TarZstd)
                } else {
                    Ok(ArchiveFormat::TarZstd)  // 也当作 tar.zst 处理
                }
            }
            _ => Ok(ArchiveFormat::Unknown),
        }
    }

    /// 检查压缩包是否需要密码
    pub fn requires_password(archive_path: &str) -> SzResult<bool> {
        let format = Self::detect_format(archive_path)?;

        match format {
            ArchiveFormat::Szp => {
                TarZstdCompressor::requires_password(archive_path)
            }
            ArchiveFormat::Sz7z => {
                // .sz7z 始终需要密码
                Ok(true)
            }
            ArchiveFormat::TarZstd => {
                // 标准 tar.zst 不支持密码
                Ok(false)
            }
            ArchiveFormat::SevenZ => {
                // 7z 格式检查（sevenz-rust 暂不支持密码检测）
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
            ArchiveFormat::Szp => {
                let compressor = TarZstdCompressor::default();
                compressor.verify_password(archive_path, password)
            }
            ArchiveFormat::Sz7z => {
                let compressor = EncryptedCompressor::default();
                compressor.verify_password(archive_path, password)
            }
            ArchiveFormat::TarZstd | ArchiveFormat::SevenZ => {
                // 这些格式不使用我们的加密，返回 true
                Ok(true)
            }
            ArchiveFormat::Unknown => {
                Err(SzError::InvalidArgument("无法识别的压缩包格式".to_string()))
            }
        }
    }

    /// 智能解压
    pub fn decompress<F>(
        archive_path: &str,
        output_dir: &str,
        password: Option<&str>,
        mut progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(u64, u64, &str),
    {
        let format = Self::detect_format(archive_path)?;

        match format {
            ArchiveFormat::Szp => {
                let compressor = TarZstdCompressor::default();
                compressor.decompress(archive_path, output_dir, password, progress_callback)
            }
            ArchiveFormat::Sz7z => {
                let pwd = password.ok_or_else(|| {
                    SzError::Decryption("此文件需要密码".to_string())
                })?;
                let compressor = EncryptedCompressor::default();
                compressor.decompress_encrypted(archive_path, output_dir, pwd, progress_callback)
            }
            ArchiveFormat::TarZstd => {
                Self::decompress_tar_zstd(archive_path, output_dir, progress_callback)
            }
            ArchiveFormat::SevenZ => {
                // 7z 解压器使用不同的回调签名，需要适配
                let decompressor = Decompressor::new();
                decompressor.decompress(archive_path, output_dir, password, |progress| {
                    progress_callback(progress.processed_bytes, progress.total_bytes, &progress.current_file);
                })
            }
            ArchiveFormat::Unknown => {
                // 尝试按 7z 格式解压
                let decompressor = Decompressor::new();
                decompressor.decompress(archive_path, output_dir, password, |progress| {
                    progress_callback(progress.processed_bytes, progress.total_bytes, &progress.current_file);
                })
            }
        }
    }

    /// 解压标准 tar.zst 格式（无自定义魔数）
    fn decompress_tar_zstd<F>(
        archive_path: &str,
        output_dir: &str,
        mut progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(u64, u64, &str),
    {
        use std::fs;
        use std::io::Cursor;
        use tar::Archive;

        let file_size = fs::metadata(archive_path)?.len();
        progress_callback(0, file_size, "读取 Zstd 文件...");

        // 读取整个文件
        let mut file = BufReader::new(File::open(archive_path)?);
        let mut zstd_data = Vec::new();
        file.read_to_end(&mut zstd_data)?;

        progress_callback(file_size / 3, file_size, "Zstd 解压中...");

        // Zstd 解压
        let tar_data = zstd::decode_all(Cursor::new(&zstd_data))
            .map_err(|e| SzError::Decompress(format!("Zstd 解压失败: {}", e)))?;

        progress_callback(file_size * 2 / 3, file_size, "Tar 解档中...");

        // Tar 解档
        let output_path = Path::new(output_dir);
        fs::create_dir_all(output_path)?;

        let mut archive = Archive::new(Cursor::new(&tar_data));
        archive.unpack(output_path)
            .map_err(|e| SzError::Decompress(format!("Tar 解档失败: {}", e)))?;

        // 收集文件
        let mut extracted_files = Vec::new();
        Self::collect_files(output_path, &mut extracted_files)?;

        progress_callback(file_size, file_size, "完成");

        Ok(extracted_files)
    }

    fn collect_files(dir: &Path, files: &mut Vec<String>) -> SzResult<()> {
        use std::fs;
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    files.push(path.to_string_lossy().to_string());
                } else if path.is_dir() {
                    Self::collect_files(&path, files)?;
                }
            }
        }
        Ok(())
    }

    /// 列出压缩包内容
    pub fn list_contents(archive_path: &str, password: Option<&str>) -> SzResult<Vec<String>> {
        let format = Self::detect_format(archive_path)?;

        match format {
            ArchiveFormat::SevenZ => {
                let decompressor = Decompressor::new();
                decompressor.list_contents(archive_path)
            }
            _ => {
                // 对于其他格式，暂不支持仅列出内容
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
