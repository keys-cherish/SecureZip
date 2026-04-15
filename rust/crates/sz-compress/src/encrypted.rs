//! 7z.zstd 专属压缩模块（主推方案）
//!
//! 分层架构（类似 7-Zip-zstd）：
//! 压缩：源文件 → 7z归档(LZMA2低级别) → Zstd多线程压缩 → [可选]AES-256-GCM加密 → .sz7z
//! 解压：.sz7z → [可选]AES-256-GCM解密 → Zstd解压 → 7z解压 → 源文件
//!
//! 文件格式 v2（.sz7z）：
//! [0-4]     魔数 "SZ7Z"
//! [4]       版本号 (2)
//! [5]       标志位 (bit0: 是否加密)
//! [6-17]    Nonce (12字节, 仅加密时有效)
//! [18-33]   Salt (16字节, 仅加密时有效)
//! [34-N]    数据 (Zstd压缩的7z数据, 加密时为AES-GCM密文)
//!
//! v1 格式（向后兼容读取）：
//! [0-4] "SZ7Z" [4] 1 [5-16] Nonce [17-32] Salt [33-N] 加密数据

use std::fs::{self, File};
use std::io::{Read, Write, Cursor, BufReader, BufWriter};
use std::path::Path;
use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use sevenz_rust::{
    SevenZWriter, SevenZArchiveEntry,
    SevenZMethodConfiguration, SevenZMethod, lzma::LZMA2Options,
    MethodOptions,
};
use zstd::stream::Encoder;
use zstd::zstd_safe::CParameter;
use sz_core::{SzError, SzResult};

/// 专属模式魔数
const MAGIC: &[u8; 4] = b"SZ7Z";
/// 版本号（v2 新增 flags 字段）
const VERSION: u8 = 2;
/// Salt 长度
const SALT_LEN: usize = 16;
/// Nonce 长度
const NONCE_LEN: usize = 12;
/// 标志位：是否加密
const FLAG_ENCRYPTED: u8 = 0x01;
/// Zstd 分块大小（128KB）
const ZSTD_CHUNK_SIZE: usize = 128 * 1024;
/// 进度更新间隔（256KB）
const PROGRESS_INTERVAL: u64 = 256 * 1024;
/// Zstd 最大窗口日志（128MB）
const ZSTD_WINDOW_LOG_MAX: u32 = 27;

/// 压缩结果
pub struct EncryptedResult {
    pub success: bool,
    pub original_size: u64,
    pub compressed_size: u64,
    pub encrypted_size: u64,
    pub output_path: String,
    pub duration_ms: u64,
}

/// Argon2id 密钥派生（m=16MB, t=2, p=1）
fn derive_key(password: &str, salt: &[u8]) -> SzResult<[u8; 32]> {
    let mut key = [0u8; 32];
    let argon2 = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(16384, 2, 1, Some(32)).unwrap(),
    );
    argon2.hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| SzError::Encryption(format!("密钥派生失败: {}", e)))?;
    Ok(key)
}

/// 7z.zstd 专属压缩器（主推方案）
///
/// 支持加密和非加密两种模式，生成 .sz7z 专属格式
pub struct EncryptedCompressor {
    compression_level: i32,
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl Default for EncryptedCompressor {
    fn default() -> Self {
        Self::new(3)
    }
}

impl EncryptedCompressor {
    pub fn new(compression_level: i32) -> Self {
        Self {
            compression_level: compression_level.clamp(1, 22),
            cancel_flag: None,
        }
    }

    /// 创建带取消标志的压缩器
    pub fn with_cancel_flag(compression_level: i32, cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            compression_level: compression_level.clamp(1, 22),
            cancel_flag: Some(cancel_flag),
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag.as_ref().map(|f| f.load(Ordering::Relaxed)).unwrap_or(false)
    }

    /// 配置 Zstd 编码器（参考 7-Zip-zstd）
    fn configure_encoder<W: Write>(
        encoder: &mut Encoder<W>,
        total_size: u64,
        _compression_level: i32,
    ) -> SzResult<()> {
        let num_threads = num_cpus::get() as u32;

        // 多线程
        if num_threads > 1 {
            encoder.multithread(num_threads)
                .map_err(|e| SzError::Compress(format!("设置多线程失败: {}", e)))?;
        }

        // 窗口大小
        let window_log: u32 = if total_size == 0 { 20 } else {
            ((total_size as f64).log2().ceil() as u32).max(10).min(ZSTD_WINDOW_LOG_MAX)
        };
        encoder.set_parameter(CParameter::WindowLog(window_log))
            .map_err(|e| SzError::Compress(format!("设置WindowLog失败: {}", e)))?;

        // 大文件启用 LDM
        if total_size > 16 * 1024 * 1024 {
            encoder.set_parameter(CParameter::EnableLongDistanceMatching(true))
                .map_err(|e| SzError::Compress(format!("设置LDM失败: {}", e)))?;
            let ldm_hash_log = if total_size > 1024 * 1024 * 1024 { 20 } else { 16 };
            encoder.set_parameter(CParameter::LdmHashLog(ldm_hash_log))
                .map_err(|e| SzError::Compress(format!("设置LdmHashLog失败: {}", e)))?;
            encoder.set_parameter(CParameter::LdmMinMatch(64))
                .map_err(|e| SzError::Compress(format!("设置LdmMinMatch失败: {}", e)))?;
        }

        // 禁用校验和（由 AES-GCM 或 7z 内部保证）
        encoder.set_parameter(CParameter::ChecksumFlag(false))
            .map_err(|e| SzError::Compress(format!("设置ChecksumFlag失败: {}", e)))?;
        encoder.set_parameter(CParameter::ContentSizeFlag(true))
            .map_err(|e| SzError::Compress(format!("设置ContentSizeFlag失败: {}", e)))?;

        if total_size > 0 {
            encoder.set_pledged_src_size(Some(total_size))
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
    
    /// 7z.zstd 压缩（统一入口，可选密码）
    ///
    /// 流程：源文件 → 7z归档(LZMA2低级别) → Zstd多线程压缩 → [可选]AES-256-GCM加密 → .sz7z
    pub fn compress<F>(
        &self,
        input_paths: &[String],
        output_path: &str,
        password: Option<&str>,
        mut progress_callback: F,
    ) -> SzResult<EncryptedResult>
    where
        F: FnMut(u64, u64, &str),
    {
        let start_time = Instant::now();
        let needs_encryption = password.map(|p| !p.is_empty()).unwrap_or(false);

        if input_paths.is_empty() {
            return Err(SzError::InvalidArgument("输入路径不能为空".to_string()));
        }
        if self.is_cancelled() { return Err(SzError::Cancelled); }

        // 计算总大小
        let total_size = self.calculate_total_size(input_paths)?;
        let mut processed_size: u64 = 0;

        progress_callback(0, total_size, "创建 7z 归档...");

        // ===== 第一步：7z 归档（LZMA2 preset 1，快速归档，主压缩交给 Zstd） =====
        let mut seven_z_data = Vec::new();
        {
            let cursor = Cursor::new(&mut seven_z_data);
            let mut sz_writer = SevenZWriter::new(cursor)
                .map_err(|e| SzError::Compress(format!("创建7z失败: {}", e)))?;

            sz_writer.set_content_methods(vec![
                SevenZMethodConfiguration::new(SevenZMethod::LZMA2)
                    .with_options(MethodOptions::LZMA2(LZMA2Options::with_preset(1))),
            ]);

            let files = self.collect_files_with_relative(input_paths)?;

            for (absolute_path, relative_name) in &files {
                if self.is_cancelled() { return Err(SzError::Cancelled); }
                let path = Path::new(absolute_path);

                if path.is_file() {
                    let file_size = fs::metadata(path)?.len();
                    let mut file_content = Vec::new();
                    File::open(path)?.read_to_end(&mut file_content)?;

                    sz_writer.push_archive_entry(
                        SevenZArchiveEntry::from_path(path, relative_name.clone()),
                        Some(Cursor::new(file_content)),
                    ).map_err(|e| SzError::Compress(format!("添加文件失败: {}", e)))?;

                    processed_size += file_size;
                    // 7z 归档占 40% 进度
                    let progress = (processed_size as f64 / total_size as f64 * 0.4 * total_size as f64) as u64;
                    progress_callback(progress, total_size, relative_name);
                }
            }

            sz_writer.finish()
                .map_err(|e| SzError::Compress(format!("完成7z归档失败: {}", e)))?;
        }
        
        let seven_z_size = seven_z_data.len() as u64;
        if self.is_cancelled() { return Err(SzError::Cancelled); }

        // ===== 第二步：Zstd 多线程分块压缩 =====
        progress_callback((total_size as f64 * 0.4) as u64, total_size, "Zstd 压缩中...");

        let mut zstd_data = Vec::with_capacity((seven_z_size / 2) as usize);
        {
            let mut encoder = Encoder::new(&mut zstd_data, self.compression_level)?;
            Self::configure_encoder(&mut encoder, seven_z_size, self.compression_level)?;
            encoder.include_checksum(false)?;

            let mut processed: u64 = 0;
            let mut last_reported: u64 = 0;
            for chunk in seven_z_data.chunks(ZSTD_CHUNK_SIZE) {
                if self.is_cancelled() { return Err(SzError::Cancelled); }
                encoder.write_all(chunk)?;
                processed += chunk.len() as u64;
                if processed - last_reported >= PROGRESS_INTERVAL {
                    last_reported = processed;
                    let pct = 0.4 + 0.3 * (processed as f64 / seven_z_size as f64);
                    progress_callback((pct * total_size as f64) as u64, total_size, "Zstd 压缩中...");
                }
            }
            encoder.finish()?;
        }

        let compressed_size = zstd_data.len() as u64;
        if self.is_cancelled() { return Err(SzError::Cancelled); }

        // ===== 第三步：确保输出目录存在 =====
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        // ===== 第四步：写入文件（v2 格式） =====
        if needs_encryption {
            let pwd = password.unwrap();
            progress_callback((total_size as f64 * 0.7) as u64, total_size, "AES-256-GCM 加密中...");

            let mut salt = [0u8; SALT_LEN];
            let mut nonce_bytes = [0u8; NONCE_LEN];
            OsRng.fill_bytes(&mut salt);
            OsRng.fill_bytes(&mut nonce_bytes);

            let key = derive_key(pwd, &salt)?;
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|e| SzError::Encryption(format!("创建加密器失败: {}", e)))?;
            let nonce = Nonce::from_slice(&nonce_bytes);
            let encrypted_data = cipher.encrypt(nonce, zstd_data.as_ref())
                .map_err(|e| SzError::Encryption(format!("加密失败: {}", e)))?;

            progress_callback((total_size as f64 * 0.9) as u64, total_size, "写入文件...");

            let mut out = BufWriter::new(File::create(output_path)?);
            out.write_all(MAGIC)?;
            out.write_all(&[VERSION])?;
            out.write_all(&[FLAG_ENCRYPTED])?;
            out.write_all(&nonce_bytes)?;
            out.write_all(&salt)?;
            out.write_all(&encrypted_data)?;
            out.flush()?;
        } else {
            progress_callback((total_size as f64 * 0.9) as u64, total_size, "写入文件...");

            let mut out = BufWriter::new(File::create(output_path)?);
            out.write_all(MAGIC)?;
            out.write_all(&[VERSION])?;
            out.write_all(&[0u8])?; // flags = 0, 无加密
            out.write_all(&[0u8; NONCE_LEN])?; // 占位 nonce
            out.write_all(&[0u8; SALT_LEN])?;  // 占位 salt
            out.write_all(&zstd_data)?;
            out.flush()?;
        }

        let final_size = fs::metadata(output_path)?.len();
        let duration = start_time.elapsed();
        progress_callback(total_size, total_size, "完成");

        Ok(EncryptedResult {
            success: true,
            original_size: total_size,
            compressed_size,
            encrypted_size: final_size,
            output_path: output_path.to_string(),
            duration_ms: duration.as_millis() as u64,
        })
    }

    /// 向后兼容：加密压缩（调用统一 compress）
    pub fn compress_encrypted<F>(
        &self,
        input_paths: &[String],
        output_path: &str,
        password: &str,
        progress_callback: F,
    ) -> SzResult<EncryptedResult>
    where
        F: FnMut(u64, u64, &str),
    {
        self.compress(input_paths, output_path, Some(password), progress_callback)
    }
    
    /// 统一解压入口（支持 v1/v2 格式，可选密码）
    ///
    /// v1: MAGIC + VERSION(1) + NONCE + SALT + 加密数据（始终加密）
    /// v2: MAGIC + VERSION(2) + FLAGS + NONCE + SALT + 数据（根据 flags 决定是否加密）
    pub fn decompress<F>(
        &self,
        archive_path: &str,
        output_dir: &str,
        password: Option<&str>,
        mut progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(u64, u64, &str),
    {
        let file_size = fs::metadata(archive_path)?.len();
        progress_callback(0, file_size, "读取文件头...");
        if self.is_cancelled() { return Err(SzError::Cancelled); }

        // ===== 第一步：读取并验证文件头 =====
        let mut file = BufReader::new(File::open(archive_path)?);

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(SzError::Decryption(format!(
                "无效的 SZ7Z 文件（魔数不匹配：期望 {:?}，实际 {:?}）",
                MAGIC, magic
            )));
        }

        let mut ver_buf = [0u8; 1];
        file.read_exact(&mut ver_buf)?;
        let version = ver_buf[0];
        if version > VERSION {
            return Err(SzError::Decryption(format!("不支持的版本: {}", version)));
        }

        // v2 有 flags 字段，v1 没有（v1 始终加密）
        let is_encrypted = if version >= 2 {
            let mut flags = [0u8; 1];
            file.read_exact(&mut flags)?;
            flags[0] & FLAG_ENCRYPTED != 0
        } else {
            true // v1 始终加密
        };

        let mut nonce_bytes = [0u8; NONCE_LEN];
        let mut salt = [0u8; SALT_LEN];
        file.read_exact(&mut nonce_bytes)?;
        file.read_exact(&mut salt)?;

        // ===== 第二步：读取数据 =====
        let mut raw_data = Vec::new();
        file.read_to_end(&mut raw_data)?;
        if self.is_cancelled() { return Err(SzError::Cancelled); }

        // ===== 第三步：解密（如需要） =====
        let zstd_data = if is_encrypted {
            let pwd = password.ok_or_else(|| {
                SzError::Decryption("此文件已加密，需要密码".to_string())
            })?;
            progress_callback(file_size / 10, file_size, "派生密钥...");
            let key = derive_key(pwd, &salt)?;
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|e| SzError::Decryption(format!("创建解密器失败: {}", e)))?;
            let nonce = Nonce::from_slice(&nonce_bytes);
            progress_callback(file_size / 5, file_size, "AES-256-GCM 解密中...");
            cipher.decrypt(nonce, raw_data.as_ref())
                .map_err(|_| SzError::Decryption("解密失败：密码错误或文件损坏".to_string()))?
        } else {
            raw_data
        };
        if self.is_cancelled() { return Err(SzError::Cancelled); }

        // ===== 第四步：Zstd 解压 =====
        progress_callback(file_size / 3, file_size, "Zstd 解压中...");
        let seven_z_data = zstd::decode_all(Cursor::new(&zstd_data))
            .map_err(|e| SzError::Decompress(format!("Zstd 解压失败: {}", e)))?;
        if self.is_cancelled() { return Err(SzError::Cancelled); }

        // ===== 第五步：7z 解压 =====
        progress_callback(file_size / 2, file_size, "7z 解压中...");
        let output_path = Path::new(output_dir);
        fs::create_dir_all(output_path)?;
        sevenz_rust::decompress(Cursor::new(&seven_z_data), output_path)
            .map_err(|e| SzError::Decompress(format!("7z 解压失败: {}", e)))?;

        let mut extracted_files = Vec::new();
        self.collect_extracted_files(output_path, &mut extracted_files)?;
        progress_callback(file_size, file_size, "完成");
        Ok(extracted_files)
    }

    /// 向后兼容：加密解压（调用统一 decompress）
    pub fn decompress_encrypted<F>(
        &self,
        archive_path: &str,
        output_dir: &str,
        password: &str,
        progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(u64, u64, &str),
    {
        self.decompress(archive_path, output_dir, Some(password), progress_callback)
    }
    
    /// 验证密码是否正确（支持 v1/v2）
    pub fn verify_password(&self, archive_path: &str, password: &str) -> SzResult<bool> {
        let mut file = BufReader::new(File::open(archive_path)?);

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC { return Ok(false); }

        let mut ver_buf = [0u8; 1];
        file.read_exact(&mut ver_buf)?;
        let version = ver_buf[0];

        // v2: 读取 flags；v1: 始终加密
        let is_encrypted = if version >= 2 {
            let mut flags = [0u8; 1];
            file.read_exact(&mut flags)?;
            flags[0] & FLAG_ENCRYPTED != 0
        } else {
            true
        };

        if !is_encrypted { return Ok(true); } // 未加密文件无需验证

        let mut nonce_bytes = [0u8; NONCE_LEN];
        let mut salt = [0u8; SALT_LEN];
        file.read_exact(&mut nonce_bytes)?;
        file.read_exact(&mut salt)?;

        let mut encrypted_data = Vec::new();
        file.read_to_end(&mut encrypted_data)?;

        let key = derive_key(password, &salt)?;
        let cipher = match Aes256Gcm::new_from_slice(&key) {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };
        let nonce = Nonce::from_slice(&nonce_bytes);

        match cipher.decrypt(nonce, encrypted_data.as_ref()) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 检查 .sz7z 文件是否需要密码（支持 v1/v2）
    pub fn requires_password(archive_path: &str) -> SzResult<bool> {
        let mut file = BufReader::new(File::open(archive_path)?);

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC { return Ok(false); }

        let mut ver_buf = [0u8; 1];
        file.read_exact(&mut ver_buf)?;

        if ver_buf[0] >= 2 {
            let mut flags = [0u8; 1];
            file.read_exact(&mut flags)?;
            Ok(flags[0] & FLAG_ENCRYPTED != 0)
        } else {
            Ok(true) // v1 始终加密
        }
    }

    /// 检查文件是否为 SZ7Z 专属格式
    pub fn is_exclusive_format(archive_path: &str) -> SzResult<bool> {
        let mut file = File::open(archive_path)?;
        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_ok() {
            Ok(&magic == MAGIC)
        } else {
            Ok(false)
        }
    }
    
    fn calculate_total_size(&self, paths: &[String]) -> SzResult<u64> {
        let mut total: u64 = 0;
        for path_str in paths {
            let path = Path::new(path_str);
            if path.is_file() {
                total += fs::metadata(path)?.len();
            } else if path.is_dir() {
                total += self.dir_size(path)?;
            }
        }
        Ok(total)
    }
    
    fn dir_size(&self, path: &Path) -> SzResult<u64> {
        let mut size: u64 = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let p = entry.path();
                if p.is_file() {
                    size += fs::metadata(&p)?.len();
                } else if p.is_dir() {
                    size += self.dir_size(&p)?;
                }
            }
        }
        Ok(size)
    }
    
    fn collect_files_with_relative(&self, paths: &[String]) -> SzResult<Vec<(String, String)>> {
        let mut files = Vec::new();
        for path_str in paths {
            let path = Path::new(path_str);
            if path.is_file() {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".to_string());
                files.push((path_str.clone(), name));
            } else if path.is_dir() {
                let base_name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "folder".to_string());
                self.collect_dir_files(&path, &base_name, &mut files)?;
            }
        }
        Ok(files)
    }
    
    fn collect_dir_files(&self, dir: &Path, prefix: &str, files: &mut Vec<(String, String)>) -> SzResult<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let relative = format!("{}/{}", prefix, name);
            
            if path.is_file() {
                files.push((path.to_string_lossy().to_string(), relative));
            } else if path.is_dir() {
                self.collect_dir_files(&path, &relative, files)?;
            }
        }
        Ok(())
    }
    
    fn collect_extracted_files(&self, dir: &Path, files: &mut Vec<String>) -> SzResult<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    files.push(path.to_string_lossy().to_string());
                } else if path.is_dir() {
                    self.collect_extracted_files(&path, files)?;
                }
            }
        }
        Ok(())
    }
}
