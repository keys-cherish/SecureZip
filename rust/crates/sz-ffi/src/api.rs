//! Rust API 层
//!
//! 所有暴露给 Kotlin (JNI) 的函数都在这里定义。
//! 函数使用纯 Rust 类型 + FnMut 回调，不依赖任何 FFI 框架。
//!
//! 设计原则：
//! - 参数使用 Rust 原生类型
//! - 进度回调使用 FnMut(u64, u64, Option<&str>)
//! - 取消使用 CancelToken（内部 Arc<AtomicBool>）
//! - 错误统一使用 anyhow::Result

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;

// ============================================================================
// 公共类型
// ============================================================================

/// 压缩结果
#[derive(Debug, Clone, Serialize)]
pub struct CompressResultFfi {
    pub original_size: u64,
    pub compressed_size: u64,
}

/// 解压结果
#[derive(Debug, Clone, Serialize)]
pub struct DecompressResultFfi {
    pub file_count: i32,
}

/// 压缩包格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ArchiveFormat {
    Unknown,
    Sz7z,
    SevenZ,
    LegacySzp,
    Zbak,
}

/// 取消令牌
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> CancelToken {
        CancelToken {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 从已有的 Arc<AtomicBool> 构造（JNI 桥接层使用）
    pub fn from_flag(flag: Arc<AtomicBool>) -> CancelToken {
        CancelToken { flag }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    pub fn inner(&self) -> Arc<AtomicBool> {
        self.flag.clone()
    }
}

/// 文件名映射条目
#[derive(Debug, Clone, Serialize)]
pub struct FfiMappingEntry {
    pub original_name: String,
    pub obfuscated_name: String,
}

/// 照片扫描结果
#[derive(Debug, Clone, Serialize)]
pub struct PhotoScanResult {
    pub total_files: u32,
    pub new_files: u32,
    pub transfer_bytes: u64,
    pub skipped_files: u32,
    pub deleted_files: u32,
}

/// 照片同步统计
#[derive(Debug, Clone, Serialize)]
pub struct PhotoSyncStats {
    pub total_backed_up: u32,
    pub total_bytes: u64,
    pub saved_bytes: u64,
    pub last_sync: Option<String>,
}

// ============================================================================
// .zbak 备份格式 API（主推）
// ============================================================================

/// .zbak 压缩
pub fn compress_zbak(
    input_paths: Vec<String>,
    output_path: String,
    password: Option<String>,
    compression_level: i32,
    encrypt_filenames: bool,
    enable_recovery: bool,
    recovery_ratio: f32,
    split_size: u64,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<CompressResultFfi> {
    let cancel_flag = cancel_token.inner();

    let mut writer = sz_compress::ZbakWriter::with_cancel_flag(compression_level, cancel_flag);
    writer.set_encrypt_filenames(encrypt_filenames);
    writer.set_recovery(enable_recovery, recovery_ratio);

    let result = writer.compress(
        &input_paths,
        &output_path,
        password.as_deref(),
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    if split_size > 0 {
        sz_compress::zbak::split::split_file(&output_path, split_size)?;
    }

    Ok(CompressResultFfi {
        original_size: result.original_size,
        compressed_size: result.compressed_size,
    })
}

/// .zbak 解压
pub fn decompress_zbak(
    archive_path: String,
    output_dir: String,
    password: Option<String>,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<DecompressResultFfi> {
    let cancel_flag = cancel_token.inner();
    let reader = sz_compress::ZbakReader::with_cancel_flag(cancel_flag);

    let files = reader.decompress(
        &archive_path,
        &output_dir,
        password.as_deref(),
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    Ok(DecompressResultFfi {
        file_count: files.len() as i32,
    })
}

/// .zbak 列出内容
pub fn list_zbak_contents(
    archive_path: String,
    password: Option<String>,
) -> anyhow::Result<Vec<String>> {
    let entries =
        sz_compress::ZbakReader::list_contents(&archive_path, password.as_deref())?;
    Ok(entries.into_iter().map(|e| e.path).collect())
}

/// .zbak 提取单个文件
pub fn extract_zbak_file(
    archive_path: String,
    file_path: String,
    output_path: String,
    password: Option<String>,
) -> anyhow::Result<()> {
    sz_compress::ZbakReader::extract_file(
        &archive_path,
        &file_path,
        &output_path,
        password.as_deref(),
    )?;
    Ok(())
}

/// .zbak 是否需要密码
pub fn zbak_requires_password(archive_path: String) -> anyhow::Result<bool> {
    Ok(sz_compress::ZbakReader::requires_password(&archive_path)?)
}

/// .zbak 验证密码
pub fn zbak_verify_password(
    archive_path: String,
    password: String,
) -> anyhow::Result<bool> {
    Ok(sz_compress::ZbakReader::verify_password(
        &archive_path,
        &password,
    )?)
}

// ============================================================================
// 智能解压 API（自动检测格式）
// ============================================================================

/// 智能解压（自动检测 .zbak / .sz7z / .7z 格式）
pub fn smart_decompress(
    archive_path: String,
    output_dir: String,
    password: Option<String>,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<DecompressResultFfi> {
    let _cancel_flag = cancel_token.inner();

    let files = sz_compress::SmartDecompressor::decompress(
        &archive_path,
        &output_dir,
        password.as_deref(),
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    Ok(DecompressResultFfi {
        file_count: files.len() as i32,
    })
}

/// 检测压缩包格式
pub fn detect_format(archive_path: String) -> anyhow::Result<ArchiveFormat> {
    let fmt = sz_compress::SmartDecompressor::detect_format(&archive_path)?;
    Ok(match fmt.to_code() {
        1 => ArchiveFormat::Sz7z,
        2 => ArchiveFormat::SevenZ,
        3 => ArchiveFormat::LegacySzp,
        4 => ArchiveFormat::Zbak,
        _ => ArchiveFormat::Unknown,
    })
}

/// 智能检测是否需要密码
pub fn smart_requires_password(archive_path: String) -> anyhow::Result<bool> {
    Ok(sz_compress::SmartDecompressor::requires_password(
        &archive_path,
    )?)
}

/// 智能验证密码
pub fn smart_verify_password(
    archive_path: String,
    password: String,
) -> anyhow::Result<bool> {
    Ok(sz_compress::SmartDecompressor::verify_password(
        &archive_path,
        &password,
    )?)
}

// ============================================================================
// 标准 7z 格式 API（兼容模式）
// ============================================================================

/// 7z 标准压缩
pub fn compress_7z(
    input_paths: Vec<String>,
    output_path: String,
    password: Option<String>,
    compression_level: u8,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<CompressResultFfi> {
    let options = sz_core::CompressOptions {
        compression_level,
        password: None,
        ..Default::default()
    };
    let compressor = sz_compress::Compressor::new(options);

    let result = if let Some(pwd) = password {
        compressor.compress_encrypted(&input_paths, &output_path, &pwd, |p| {
            progress(p.processed_bytes, p.total_bytes, None);
        })?
    } else {
        compressor.compress(&input_paths, &output_path, |p| {
            progress(p.processed_bytes, p.total_bytes, None);
        })?
    };

    Ok(CompressResultFfi {
        original_size: result.original_size,
        compressed_size: result.compressed_size,
    })
}

/// 7z 解压
pub fn decompress_7z(
    archive_path: String,
    output_dir: String,
    password: Option<String>,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<DecompressResultFfi> {
    let decompressor = sz_compress::Decompressor::new();

    let files = decompressor.decompress(
        &archive_path,
        &output_dir,
        password.as_deref(),
        |p| {
            progress(p.processed_bytes, p.total_bytes, None);
        },
    )?;

    Ok(DecompressResultFfi {
        file_count: files.len() as i32,
    })
}

/// 7z 列出内容
pub fn list_7z_contents(archive_path: String) -> anyhow::Result<Vec<String>> {
    let decompressor = sz_compress::Decompressor::new();
    Ok(decompressor.list_contents(&archive_path)?)
}

// ============================================================================
// 旧版 .sz7z 格式 API（向后兼容）
// ============================================================================

/// 旧版 .sz7z 压缩（不加密）
pub fn compress_legacy(
    input_paths: Vec<String>,
    output_path: String,
    compression_level: i32,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<CompressResultFfi> {
    let cancel_flag = cancel_token.inner();
    let compressor =
        sz_compress::EncryptedCompressor::with_cancel_flag(compression_level, cancel_flag);

    let result = compressor.compress(
        &input_paths,
        &output_path,
        None,
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    Ok(CompressResultFfi {
        original_size: result.original_size,
        compressed_size: result.compressed_size,
    })
}

/// 旧版 .sz7z 加密压缩
pub fn compress_legacy_encrypted(
    input_paths: Vec<String>,
    output_path: String,
    password: String,
    compression_level: i32,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<CompressResultFfi> {
    let cancel_flag = cancel_token.inner();
    let compressor =
        sz_compress::EncryptedCompressor::with_cancel_flag(compression_level, cancel_flag);

    let result = compressor.compress(
        &input_paths,
        &output_path,
        Some(&password),
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    Ok(CompressResultFfi {
        original_size: result.original_size,
        compressed_size: result.compressed_size,
    })
}

/// 旧版密码验证
pub fn verify_legacy_password(
    archive_path: String,
    password: String,
) -> anyhow::Result<bool> {
    let compressor = sz_compress::EncryptedCompressor::default();
    Ok(compressor.verify_password(&archive_path, &password)?)
}

// ============================================================================
// WebDAV 备份 API
// ============================================================================

/// WebDAV 连接测试
pub fn webdav_test_connection(
    url: String,
    username: String,
    password: String,
) -> anyhow::Result<bool> {
    let config = sz_core::WebDavConfig {
        server_url: url,
        username,
        password,
        remote_path: "/".to_string(),
    };
    let client = sz_webdav::WebDavClient::new(config)?;
    Ok(client.test_connection()?)
}

/// WebDAV 流式备份
pub fn webdav_backup(
    input_paths: Vec<String>,
    url: String,
    username: String,
    webdav_password: String,
    encrypt_password: Option<String>,
    compression_level: i32,
    recovery_ratio: f32,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<String> {
    let config = sz_core::WebDavConfig {
        server_url: url,
        username,
        password: webdav_password,
        remote_path: "/backups".to_string(),
    };

    let webdav = sz_webdav::WebDavClient::new(config)?;
    let cancel_flag = cancel_token.inner();

    let uploader = sz_compress::zbak::uploader::StreamingUploader::new(webdav)
        .with_recovery(recovery_ratio > 0.0, recovery_ratio)
        .with_cancel_flag(cancel_flag);

    let manifest = uploader.backup(
        &input_paths,
        encrypt_password.as_deref(),
        compression_level,
        false,
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    Ok(manifest.to_json()?)
}

/// WebDAV 恢复备份
pub fn webdav_restore(
    backup_id: String,
    output_dir: String,
    url: String,
    username: String,
    webdav_password: String,
    encrypt_password: Option<String>,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<DecompressResultFfi> {
    let config = sz_core::WebDavConfig {
        server_url: url,
        username,
        password: webdav_password,
        remote_path: "/backups".to_string(),
    };

    let webdav = sz_webdav::WebDavClient::new(config)?;
    let uploader = sz_compress::zbak::uploader::StreamingUploader::new(webdav);

    let files = uploader.restore(
        &backup_id,
        &output_dir,
        encrypt_password.as_deref(),
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    Ok(DecompressResultFfi {
        file_count: files.len() as i32,
    })
}

/// WebDAV 列出备份
pub fn webdav_list_backups(
    url: String,
    username: String,
    password: String,
) -> anyhow::Result<String> {
    let config = sz_core::WebDavConfig {
        server_url: url,
        username,
        password,
        remote_path: "/backups".to_string(),
    };

    let webdav = sz_webdav::WebDavClient::new(config)?;
    let uploader = sz_compress::zbak::uploader::StreamingUploader::new(webdav);
    let manifests = uploader.list_backups()?;
    Ok(serde_json::to_string(&manifests)?)
}

// ============================================================================
// 加密工具 API
// ============================================================================

/// 加密字符串
pub fn encrypt_string(data: String, password: String) -> anyhow::Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let salt = sz_crypto::generate_salt();
    let key = sz_crypto::derive_key_from_password(&password, &salt)?;
    let encryptor = sz_crypto::AesEncryptor::new(&key);
    let encrypted = encryptor.encrypt_string(&data)?;
    let salt_b64 = STANDARD.encode(&salt);
    Ok(format!("{}:{}", salt_b64, encrypted))
}

/// 解密字符串
pub fn decrypt_string(encrypted_data: String, password: String) -> anyhow::Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let parts: Vec<&str> = encrypted_data.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("无效的加密数据格式");
    }

    let salt = STANDARD.decode(parts[0])?;
    let key = sz_crypto::derive_key_from_password(&password, &salt)?;
    let encryptor = sz_crypto::AesEncryptor::new(&key);
    Ok(encryptor.decrypt_string(parts[1])?)
}

/// 生成随机密码
pub fn generate_random_password(length: u32, include_symbols: bool) -> String {
    sz_crypto::generate_random_password(length as usize, include_symbols)
}

/// 计算密码强度 (0-4)
pub fn calculate_password_strength(password: String) -> u8 {
    sz_crypto::calculate_password_strength(&password)
}

// ============================================================================
// 文件名混淆 API
// ============================================================================

/// 混淆文件名列表
pub fn obfuscate_filenames(
    original_names: Vec<String>,
    scheme: u8,
    archive_path: String,
) -> Vec<FfiMappingEntry> {
    let obfuscation_scheme = match scheme {
        0 => sz_core::ObfuscationScheme::Sequential,
        1 => sz_core::ObfuscationScheme::DateSequential,
        2 => sz_core::ObfuscationScheme::Random,
        3 => sz_core::ObfuscationScheme::Hash,
        4 => sz_core::ObfuscationScheme::Encrypted,
        _ => sz_core::ObfuscationScheme::Sequential,
    };

    let mut obfuscator = sz_filename::FilenameObfuscator::new(obfuscation_scheme);
    obfuscator
        .obfuscate_batch(&original_names, &archive_path)
        .into_iter()
        .map(|e| FfiMappingEntry {
            original_name: e.original_name,
            obfuscated_name: e.obfuscated_name,
        })
        .collect()
}

// ============================================================================
// 工具 API
// ============================================================================

/// 初始化日志
pub fn init_logger() {
    let _ = env_logger::try_init();
}

/// 获取 Rust 库版本
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============================================================================
// 照片增量备份 API
// ============================================================================

/// 扫描照片目录并计算增量
pub fn photo_scan_incremental(
    directories: Vec<String>,
    index_path: String,
    include_videos: bool,
) -> anyhow::Result<PhotoScanResult> {
    use sz_photo_sync::{PhotoScanner, SyncIndex, diff};

    let scanner = PhotoScanner::new().include_videos(include_videos);
    let scanned = scanner.scan(&directories);

    let index = SyncIndex::load(std::path::Path::new(&index_path))?;
    let diff_result = diff::compute_diff(&scanned, &index);

    Ok(PhotoScanResult {
        total_files: diff_result.total_scanned as u32,
        new_files: diff_result.to_backup.len() as u32,
        transfer_bytes: diff_result.transfer_bytes(),
        skipped_files: diff_result.unchanged_count as u32,
        deleted_files: diff_result.deleted.len() as u32,
    })
}

/// 执行照片增量备份（本地 .zbak）
pub fn photo_backup_incremental(
    directories: Vec<String>,
    output_path: String,
    index_path: String,
    password: Option<String>,
    exif_strip_level: u8,
    include_videos: bool,
    compression_level: i32,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<CompressResultFfi> {
    use sz_photo_sync::{PhotoScanner, SyncIndex, diff};
    use sz_photo_sync::index::PhotoRecord;

    let cancel_flag = cancel_token.inner();

    progress(0, 0, Some("扫描照片目录..."));

    let scanner = PhotoScanner::new().include_videos(include_videos);
    let scanned = scanner.scan(&directories);

    let index_file = std::path::Path::new(&index_path);
    let mut index = SyncIndex::load(index_file)?;
    let diff_result = diff::compute_diff(&scanned, &index);

    if diff_result.to_backup.is_empty() {
        return Ok(CompressResultFfi {
            original_size: 0,
            compressed_size: 0,
        });
    }

    progress(
        0,
        diff_result.transfer_bytes(),
        Some(&format!("发现 {} 张新照片", diff_result.to_backup.len())),
    );

    let paths_to_backup: Vec<String> = diff_result
        .to_backup
        .iter()
        .map(|p| p.path.to_string_lossy().to_string())
        .collect();

    let mut writer = sz_compress::ZbakWriter::with_cancel_flag(
        compression_level,
        cancel_flag,
    );
    writer.set_encrypt_filenames(true);
    writer.set_recovery(true, 0.05);

    let result = writer.compress(
        &paths_to_backup,
        &output_path,
        password.as_deref(),
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    let backup_id = output_path.clone();
    let now = chrono::Utc::now();
    for photo in &diff_result.to_backup {
        index.add_record(PhotoRecord {
            dedup_key: photo.dedup_key.clone(),
            original_path: photo.path.to_string_lossy().to_string(),
            size: photo.size,
            mtime: photo.mtime,
            backup_time: now,
            backup_id: backup_id.clone(),
            encrypted_name: None,
        });
    }

    for change in &diff_result.to_update {
        if let diff::FileChange::Moved { photo, .. } = change {
            if let Some(record) = index.records.get_mut(&photo.dedup_key) {
                record.original_path = photo.path.to_string_lossy().to_string();
            }
        }
    }

    index.mark_synced();
    index.save(index_file)?;

    Ok(CompressResultFfi {
        original_size: result.original_size,
        compressed_size: result.compressed_size,
    })
}

/// 照片增量备份到 WebDAV
pub fn photo_backup_to_webdav(
    directories: Vec<String>,
    index_path: String,
    url: String,
    username: String,
    webdav_password: String,
    encrypt_password: Option<String>,
    exif_strip_level: u8,
    include_videos: bool,
    compression_level: i32,
    cancel_token: &CancelToken,
    mut progress: impl FnMut(u64, u64, Option<&str>),
) -> anyhow::Result<String> {
    use sz_photo_sync::{PhotoScanner, SyncIndex, diff};
    use sz_photo_sync::index::PhotoRecord;

    let cancel_flag = cancel_token.inner();

    let scanner = PhotoScanner::new().include_videos(include_videos);
    let scanned = scanner.scan(&directories);

    let index_file = std::path::Path::new(&index_path);
    let mut index = SyncIndex::load(index_file)?;
    let diff_result = diff::compute_diff(&scanned, &index);

    if diff_result.to_backup.is_empty() {
        return Ok("{}".to_string());
    }

    progress(
        0,
        diff_result.transfer_bytes(),
        Some(&format!("发现 {} 张新照片，准备上传", diff_result.to_backup.len())),
    );

    let paths_to_backup: Vec<String> = diff_result
        .to_backup
        .iter()
        .map(|p| p.path.to_string_lossy().to_string())
        .collect();

    let config = sz_core::WebDavConfig {
        server_url: url,
        username,
        password: webdav_password,
        remote_path: "/photo_backups".to_string(),
    };

    let webdav = sz_webdav::WebDavClient::new(config)?;
    let uploader = sz_compress::zbak::uploader::StreamingUploader::new(webdav)
        .with_recovery(true, 0.05)
        .with_cancel_flag(cancel_flag);

    let manifest = uploader.backup(
        &paths_to_backup,
        encrypt_password.as_deref(),
        compression_level,
        true,
        |current, total, file| {
            progress(current, total, Some(file));
        },
    )?;

    let backup_id = manifest.backup_id.clone();
    let now = chrono::Utc::now();
    for photo in &diff_result.to_backup {
        index.add_record(PhotoRecord {
            dedup_key: photo.dedup_key.clone(),
            original_path: photo.path.to_string_lossy().to_string(),
            size: photo.size,
            mtime: photo.mtime,
            backup_time: now,
            backup_id: backup_id.clone(),
            encrypted_name: None,
        });
    }

    index.mark_synced();
    index.save(index_file)?;

    Ok(manifest.to_json()?)
}

/// 获取照片备份统计信息
pub fn photo_get_sync_stats(index_path: String) -> anyhow::Result<PhotoSyncStats> {
    let index = sz_photo_sync::SyncIndex::load(std::path::Path::new(&index_path))?;
    Ok(PhotoSyncStats {
        total_backed_up: index.backed_up_count() as u32,
        total_bytes: index.stats.total_bytes,
        saved_bytes: index.stats.saved_bytes,
        last_sync: index.last_sync.map(|t| t.to_rfc3339()),
    })
}
