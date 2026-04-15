//! .zbak 压缩写入器
//!
//! 管线: 收集文件 → 写占位头(96字节) → 逐文件 流式读取+CRC32+Zstd压缩+可选加密+写入
//!       → 序列化索引 → (可选加密索引) → 写索引 → (可选RS恢复块) → 回写文件头
//!
//! 性能优化:
//!   - 256KB 分块读取，不加载整个文件到内存
//!   - Zstd 多线程压缩 + WindowLog + LDM（大文件）
//!   - 逐文件实时进度回调
//!   - 内存占用 ≈ 压缩后大小（而非原始大小）

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read as IoRead, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher as Crc32Hasher;
use zstd::zstd_safe::CParameter;

use sz_core::{SzError, SzResult};
use super::format::*;
use super::crypto;
use super::recovery::RecoveryGenerator;

/// 读取缓冲区大小
const READ_BUF_SIZE: usize = 256 * 1024; // 256KB

/// 进度更新间隔（每 1MB 更新一次）
const PROGRESS_INTERVAL: u64 = 1024 * 1024;

/// Zstd 最大窗口日志（128MB）
const ZSTD_WINDOW_LOG_MAX: u32 = 27;

/// .zbak 写入器
pub struct ZbakWriter {
    compression_level: i32,
    cancel_flag: Option<Arc<AtomicBool>>,
    encrypt_filenames: bool,
    enable_recovery: bool,
    recovery_ratio: f32,
}

impl ZbakWriter {
    pub fn new(compression_level: i32) -> Self {
        Self {
            compression_level: compression_level.clamp(1, 22),
            cancel_flag: None,
            encrypt_filenames: false,
            enable_recovery: false,
            recovery_ratio: 0.10,
        }
    }

    pub fn with_cancel_flag(compression_level: i32, cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            compression_level: compression_level.clamp(1, 22),
            cancel_flag: Some(cancel_flag),
            encrypt_filenames: false,
            enable_recovery: false,
            recovery_ratio: 0.10,
        }
    }

    pub fn set_encrypt_filenames(&mut self, val: bool) {
        self.encrypt_filenames = val;
    }

    pub fn set_recovery(&mut self, enable: bool, ratio: f32) {
        self.enable_recovery = enable;
        self.recovery_ratio = ratio.clamp(0.01, 0.50);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag
            .as_ref()
            .map_or(false, |f| f.load(Ordering::Relaxed))
    }

    /// 压缩多个路径到 .zbak 文件
    pub fn compress<F>(
        &self,
        input_paths: &[String],
        output_path: &str,
        password: Option<&str>,
        mut progress_callback: F,
    ) -> SzResult<ZbakResult>
    where
        F: FnMut(u64, u64, &str),
    {
        let file_list = collect_files(input_paths)?;
        if file_list.is_empty() {
            return Err(SzError::InvalidArgument("没有找到要压缩的文件".into()));
        }

        let total_size: u64 = file_list.iter().map(|f| f.size).sum();
        let mut processed: u64 = 0;

        progress_callback(0, total_size, "准备中...");

        // 准备密钥
        let encryption = if let Some(pwd) = password {
            if !pwd.is_empty() {
                progress_callback(0, total_size, "密钥派生中...");
                let salt = crypto::generate_salt();
                let master_key = crypto::derive_master_key(pwd, &salt)?;
                Some(EncryptionContext { salt, master_key })
            } else {
                None
            }
        } else {
            None
        };

        // 确保输出目录存在
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        // 打开输出文件
        let output = File::create(output_path)
            .map_err(|e| SzError::Compress(format!("创建输出文件失败: {}", e)))?;
        let mut writer = BufWriter::with_capacity(1024 * 1024, output); // 1MB 写缓冲

        let mut header = ZbakHeader::new(self.compression_level as u8);
        header.entry_count = file_list.len() as u32;

        if let Some(ref enc) = encryption {
            header.set_encrypted(true);
            header.salt = enc.salt;
            // [安全] 有密码时始终加密索引区，防止文件名/大小/时间戳泄露
            header.set_filename_encrypted(true);
            let verify_key = crypto::derive_verify_key(&enc.master_key);
            let (nonce, tag) = crypto::create_verify_block(&verify_key)?;
            header.verify_nonce = nonce;
            header.verify_tag = tag;
        }

        if self.enable_recovery {
            header.set_has_recovery(true);
        }

        header.write_to(&mut writer)?;

        // 逐文件压缩
        let mut index_entries = Vec::with_capacity(file_list.len());

        for (file_idx, file_info) in file_list.iter().enumerate() {
            if self.is_cancelled() {
                return Err(SzError::Cancelled);
            }

            let block_offset = writer.stream_position()
                .map_err(|e| SzError::Compress(format!("获取文件位置失败: {}", e)))?;

            // 目录条目
            if file_info.is_directory {
                index_entries.push(ZbakIndexEntry {
                    path: file_info.rel_path.clone(),
                    original_size: 0,
                    compressed_size: 0,
                    block_offset,
                    crc32: 0,
                    mtime: file_info.mtime,
                    permissions: file_info.permissions,
                    is_directory: true,
                });
                continue;
            }

            progress_callback(processed, total_size, &file_info.rel_path);

            // 流式压缩: 分块读取 → CRC32 → Zstd → 输出到 Vec（带实时进度）
            let (compressed, crc32_val, original_size) = stream_compress_file(
                &file_info.abs_path,
                file_info.size,
                self.compression_level,
                processed,
                total_size,
                &file_info.rel_path,
                &mut progress_callback,
            )?;

            // 可选加密
            let (block_data, nonce_bytes) = if let Some(ref enc) = encryption {
                let file_key = crypto::derive_file_key(&enc.master_key, file_idx as u32);
                let (ciphertext, nonce) = crypto::encrypt_block(&file_key, &compressed)?;
                drop(compressed); // 立即释放压缩数据
                (ciphertext, nonce)
            } else {
                (compressed, [0u8; NONCE_SIZE])
            };

            let compressed_size = block_data.len() as u64;

            // 写数据块
            writer.write_u64::<LittleEndian>(compressed_size)?;
            writer.write_u64::<LittleEndian>(original_size)?;
            writer.write_all(&nonce_bytes)?;
            writer.write_all(&block_data)?;
            drop(block_data); // 立即释放

            index_entries.push(ZbakIndexEntry {
                path: file_info.rel_path.clone(),
                original_size,
                compressed_size,
                block_offset,
                crc32: crc32_val,
                mtime: file_info.mtime,
                permissions: file_info.permissions,
                is_directory: false,
            });

            processed += file_info.size;
            progress_callback(processed, total_size, &file_info.rel_path);
        }

        // 写索引区
        let index_offset = writer.stream_position()
            .map_err(|e| SzError::Compress(format!("获取索引偏移失败: {}", e)))?;

        let mut index_buf = Vec::new();
        write_index(&index_entries, &mut index_buf)?;

        // [安全] 有密码时默认加密索引（文件名），防止元数据泄露
        let index_data = if let Some(ref enc) = encryption {
            let index_key = crypto::derive_index_key(&enc.master_key);
            crypto::encrypt_index(&index_key, &index_buf)?
        } else {
            index_buf
        };

        let index_size = index_data.len() as u32;
        writer.write_all(&index_data)?;

        // 可选恢复记录
        let mut recovery_offset: u64 = 0;
        let mut recovery_size: u32 = 0;

        if self.enable_recovery {
            progress_callback(processed, total_size, "生成恢复记录...");

            let rec_offset = writer.stream_position()
                .map_err(|e| SzError::Compress(format!("获取恢复区偏移失败: {}", e)))?;

            writer.flush()?;

            // 流式分块生成恢复记录（64MB/块），避免大文件 OOM
            let rec_written = write_recovery_streaming(
                output_path,
                rec_offset as usize,
                self.recovery_ratio,
                &mut writer,
            )?;

            recovery_offset = rec_offset;
            recovery_size = rec_written;
        }

        // 回写文件头 — 先 flush 确保缓冲区写入磁盘
        header.index_offset = index_offset;
        header.index_size = index_size;
        header.recovery_offset = recovery_offset;
        header.recovery_size = recovery_size;

        writer.flush()?;
        writer.seek(SeekFrom::Start(0))
            .map_err(|e| SzError::Compress(format!("回写头失败: {}", e)))?;
        header.write_to(&mut writer)?;
        writer.flush()?;

        let compressed_file_size = fs::metadata(output_path)
            .map_err(|e| SzError::Compress(format!("获取输出文件大小失败: {}", e)))?.len();

        progress_callback(total_size, total_size, "完成");

        Ok(ZbakResult {
            original_size: total_size,
            compressed_size: compressed_file_size,
            file_count: file_list.len() as u32,
        })
    }
}

// ============================================================================
// 流式压缩（带 Zstd 优化 + 实时进度）
// ============================================================================

/// 配置 Zstd 编码器（参考 7-Zip-zstd 最佳实践）
fn configure_encoder<W: Write>(
    encoder: &mut zstd::stream::Encoder<W>,
    file_size: u64,
) -> SzResult<()> {
    let num_threads = num_cpus::get() as u32;

    // 多线程
    if num_threads > 1 {
        encoder.multithread(num_threads)
            .map_err(|e| SzError::Compress(format!("设置多线程失败: {}", e)))?;
    }

    // 窗口大小 — 根据文件大小自适应
    let window_log: u32 = if file_size == 0 {
        20
    } else {
        ((file_size as f64).log2().ceil() as u32).max(10).min(ZSTD_WINDOW_LOG_MAX)
    };
    encoder.set_parameter(CParameter::WindowLog(window_log))
        .map_err(|e| SzError::Compress(format!("设置WindowLog失败: {}", e)))?;

    // 大文件启用 LDM（长距离匹配）
    if file_size > 16 * 1024 * 1024 {
        encoder.set_parameter(CParameter::EnableLongDistanceMatching(true))
            .map_err(|e| SzError::Compress(format!("设置LDM失败: {}", e)))?;
        let ldm_hash_log = if file_size > 1024 * 1024 * 1024 { 20 } else { 16 };
        encoder.set_parameter(CParameter::LdmHashLog(ldm_hash_log))
            .map_err(|e| SzError::Compress(format!("设置LdmHashLog失败: {}", e)))?;
        encoder.set_parameter(CParameter::LdmMinMatch(64))
            .map_err(|e| SzError::Compress(format!("设置LdmMinMatch失败: {}", e)))?;
    }

    // 禁用校验和（由 CRC32 + AES-GCM 保证）
    encoder.set_parameter(CParameter::ChecksumFlag(false))
        .map_err(|e| SzError::Compress(format!("设置ChecksumFlag失败: {}", e)))?;
    encoder.set_parameter(CParameter::ContentSizeFlag(true))
        .map_err(|e| SzError::Compress(format!("设置ContentSizeFlag失败: {}", e)))?;

    if file_size > 0 {
        encoder.set_pledged_src_size(Some(file_size))
            .map_err(|e| SzError::Compress(format!("设置预期大小失败: {}", e)))?;
    }

    // 多线程任务配置
    if num_threads > 1 {
        let window_size = 1u64 << window_log;
        let job_size = (window_size * 4).max(4 * 1024 * 1024).min(128 * 1024 * 1024) as u32;
        encoder.set_parameter(CParameter::JobSize(job_size))
            .map_err(|e| SzError::Compress(format!("设置JobSize失败: {}", e)))?;
    }

    Ok(())
}

/// 流式压缩单个文件: 分块读取 → CRC32 → Zstd 压缩（带实时进度）
/// 返回 (compressed_data, crc32, original_size)
fn stream_compress_file<F>(
    path: &Path,
    file_size: u64,
    level: i32,
    base_processed: u64,
    total_size: u64,
    file_name: &str,
    progress_callback: &mut F,
) -> SzResult<(Vec<u8>, u32, u64)>
where
    F: FnMut(u64, u64, &str),
{
    let file = File::open(path)
        .map_err(|e| SzError::Compress(format!("打开文件失败 {:?}: {}", path, e)))?;
    let mut reader = BufReader::with_capacity(READ_BUF_SIZE, file);

    // 已压缩格式用最低级别快速通过，避免浪费 CPU
    let effective_level = if is_compressed_format(path) { 1 } else { level };

    // 预分配压缩输出（估算压缩率 50%，最小 4KB，最大 512MB）
    let estimated = ((file_size / 2) as usize).max(4096);
    let output_buf = Vec::with_capacity(estimated.min(512 * 1024 * 1024));

    let mut encoder = zstd::stream::Encoder::new(output_buf, effective_level)
        .map_err(|e| SzError::Compress(format!("Zstd 编码器创建失败: {}", e)))?;

    // 应用 Zstd 优化配置
    configure_encoder(&mut encoder, file_size)?;

    let mut crc_hasher = Crc32Hasher::new();
    let mut buf = [0u8; READ_BUF_SIZE];
    let mut original_size: u64 = 0;
    let mut last_progress: u64 = 0;

    loop {
        let n = reader.read(&mut buf)
            .map_err(|e| SzError::Compress(format!("读取文件失败: {}", e)))?;
        if n == 0 { break; }

        crc_hasher.update(&buf[..n]);
        encoder.write_all(&buf[..n])
            .map_err(|e| SzError::Compress(format!("Zstd 压缩写入失败: {}", e)))?;
        original_size += n as u64;

        // 每 1MB 更新一次进度
        if original_size - last_progress >= PROGRESS_INTERVAL {
            last_progress = original_size;
            progress_callback(base_processed + original_size, total_size, file_name);
        }
    }

    let compressed = encoder.finish()
        .map_err(|e| SzError::Compress(format!("Zstd 压缩完成失败: {}", e)))?;
    let crc32_val = crc_hasher.finalize();

    Ok((compressed, crc32_val, original_size))
}

// ============================================================================
// 流式恢复记录生成（64MB 分块，避免 OOM）
// ============================================================================

/// 恢复记录分块大小: 64MB
const RECOVERY_BLOCK_SIZE: usize = 64 * 1024 * 1024;

/// 流式生成恢复记录并直接写入 writer
/// 将受保护数据分成 64MB 块，逐块生成 RS 恢复码
/// 内存峰值 ≈ 64MB（块数据）+ ~7MB（恢复分片）≈ 71MB
fn write_recovery_streaming<W: Write>(
    file_path: &str,
    protected_size: usize,
    ratio: f32,
    writer: &mut W,
) -> SzResult<u32> {
    let file = File::open(file_path)
        .map_err(|e| SzError::Compress(format!("打开文件生成恢复记录失败: {}", e)))?;
    let mut reader = BufReader::with_capacity(READ_BUF_SIZE, file);

    let block_count = if protected_size == 0 { 0 } else {
        (protected_size + RECOVERY_BLOCK_SIZE - 1) / RECOVERY_BLOCK_SIZE
    };

    let mut total_written: u32 = 0;

    // 写分块恢复头: version(1) + block_count(4) + protected_size(8) = 13 bytes
    writer.write_all(&[2u8])?; // version = 2 (block-based)
    writer.write_u32::<LittleEndian>(block_count as u32)?;
    writer.write_u64::<LittleEndian>(protected_size as u64)?;
    total_written += 13;

    let mut remaining = protected_size;

    for _ in 0..block_count {
        let block_size = remaining.min(RECOVERY_BLOCK_SIZE);
        let mut block = vec![0u8; block_size];
        reader.read_exact(&mut block)
            .map_err(|e| SzError::Compress(format!("读取数据块失败: {}", e)))?;

        let recovery = RecoveryGenerator::generate(&block, ratio)?;
        drop(block); // 立即释放块数据

        let rec_bytes = recovery.serialize()?;
        writer.write_all(&rec_bytes)?;
        total_written += rec_bytes.len() as u32;

        remaining -= block_size;
    }

    Ok(total_written)
}

// ============================================================================
// 已压缩格式检测
// ============================================================================

/// 检查文件是否为已知的压缩/媒体格式（再压缩无收益，用最低级别快速通过）
fn is_compressed_format(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    matches!(ext.as_str(),
        // 压缩包
        "zbak" | "sz7z" | "7z" | "zip" | "rar" | "gz" | "bz2" | "xz" | "zst" | "lz4" | "lzma" | "tar.gz" | "tgz" |
        // 图片（已压缩）
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "heic" | "heif" |
        // 视频
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" |
        // 音频
        "mp3" | "aac" | "ogg" | "flac" | "opus" | "wma" | "m4a" |
        // 安装包
        "apk" | "jar" | "deb" | "rpm" | "msi" | "dmg"
    )
}

// ============================================================================
// 内部辅助
// ============================================================================

struct EncryptionContext {
    salt: [u8; 16],
    master_key: [u8; 32],
}

/// 收集到的文件信息
pub struct FileInfo {
    pub abs_path: PathBuf,
    pub rel_path: String,
    pub size: u64,
    pub mtime: i64,
    pub permissions: u32,
    pub is_directory: bool,
}

/// 收集所有要压缩的文件（递归展开目录）
pub fn collect_files(input_paths: &[String]) -> SzResult<Vec<FileInfo>> {
    let mut files = Vec::new();

    for path_str in input_paths {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(SzError::FileNotFound(path_str.clone()));
        }

        let base_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if path.is_dir() {
            collect_dir_recursive(path, base_name, &mut files)?;
        } else {
            let meta = path.metadata()?;
            let mtime = meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            files.push(FileInfo {
                abs_path: path.to_path_buf(),
                rel_path: base_name.to_string(),
                size: meta.len(),
                mtime,
                permissions: 0o644,
                is_directory: false,
            });
        }
    }

    Ok(files)
}

/// 递归收集目录中的文件
fn collect_dir_recursive(
    dir: &Path,
    prefix: &str,
    files: &mut Vec<FileInfo>,
) -> SzResult<()> {
    // 先添加目录本身
    let dir_meta = dir.metadata()?;
    let dir_mtime = dir_meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    files.push(FileInfo {
        abs_path: dir.to_path_buf(),
        rel_path: prefix.to_string(),
        size: 0,
        mtime: dir_mtime,
        permissions: 0o755,
        is_directory: true,
    });

    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| SzError::Compress(format!("读取目录失败 {:?}: {}", dir, e)))?
        .filter_map(|e| e.ok())
        .collect();

    // 排序保证确定性
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_str().unwrap_or("unknown");
        let rel = format!("{}/{}", prefix, name_str);

        if path.is_dir() {
            collect_dir_recursive(&path, &rel, files)?;
        } else {
            let meta = path.metadata()?;
            let mtime = meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            files.push(FileInfo {
                abs_path: path.clone(),
                rel_path: rel,
                size: meta.len(),
                mtime,
                permissions: 0o644,
                is_directory: false,
            });
        }
    }

    Ok(())
}
