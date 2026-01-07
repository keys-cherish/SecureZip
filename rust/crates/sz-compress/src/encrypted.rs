//! 专属加密压缩模块
//! 
//! 正确的分层架构：
//! 压缩：源文件 → 7z归档(LZMA2) → Zstd压缩(整个7z) → AES-256-GCM加密 → .sz文件
//! 解压：.sz文件 → AES-256-GCM解密 → Zstd解压 → 7z解压 → 源文件
//! 
//! 文件格式（.sz7z）：
//! [0-4]     魔数 "SZ7Z"
//! [4]       版本号 (1)
//! [5-16]    Nonce (12字节)
//! [17-32]   Salt (16字节)
//! [33-N]    加密数据 (AES-256-GCM 加密的 Zstd 压缩的 7z 数据 + 16字节 GCM Tag)

use std::fs::{self, File};
use std::io::{Read, Write, Cursor, BufReader, BufWriter};
use std::path::Path;
use std::time::Instant;

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
use sz_core::{SzError, SzResult};

/// 专属模式魔数
const MAGIC: &[u8; 4] = b"SZ7Z";
/// 版本号
const VERSION: u8 = 1;
/// Salt 长度
const SALT_LEN: usize = 16;
/// Nonce 长度
const NONCE_LEN: usize = 12;

/// 加密压缩结果
pub struct EncryptedResult {
    pub success: bool,
    pub original_size: u64,
    pub compressed_size: u64,
    pub encrypted_size: u64,
    pub output_path: String,
    pub duration_ms: u64,
}

/// 使用 Argon2id 从密码派生 32 字节密钥
/// 使用优化的参数以提高速度，同时保持足够的安全性
fn derive_key(password: &str, salt: &[u8]) -> SzResult<[u8; 32]> {
    let mut key = [0u8; 32];
    // 使用较快的 Argon2id 参数：
    // - m_cost = 16384 (16 MB 内存，而不是默认的 64 MB)
    // - t_cost = 2 (2 次迭代，而不是默认的 3 次)
    // - p_cost = 1 (单线程)
    let argon2 = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(16384, 2, 1, Some(32)).unwrap(),
    );
    argon2.hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| SzError::Encryption(format!("密钥派生失败: {}", e)))?;
    Ok(key)
}

/// 专属加密压缩器
/// 
/// 生成的 .sz7z 文件无法被任何其他软件打开
pub struct EncryptedCompressor {
    compression_level: i32,
}

impl Default for EncryptedCompressor {
    fn default() -> Self {
        Self::new(3)
    }
}

impl EncryptedCompressor {
    /// 创建专属加密压缩器
    pub fn new(compression_level: i32) -> Self {
        Self {
            compression_level: compression_level.clamp(1, 22),
        }
    }
    
    /// 专属加密压缩
    /// 
    /// 流程：
    /// 1. 源文件 → 7z归档(LZMA2) → 临时 7z 数据
    /// 2. 7z数据 → Zstd压缩 → 压缩后数据
    /// 3. 压缩数据 → AES-256-GCM加密 → 加密数据
    /// 4. 写入：MAGIC + VERSION + NONCE + SALT + 加密数据
    pub fn compress_encrypted<F>(
        &self,
        input_paths: &[String],
        output_path: &str,
        password: &str,
        mut progress_callback: F,
    ) -> SzResult<EncryptedResult>
    where
        F: FnMut(u64, u64, &str),
    {
        let start_time = Instant::now();
        
        // 计算总大小
        let total_size = self.calculate_total_size(input_paths)?;
        let mut processed_size: u64 = 0;
        
        progress_callback(0, total_size, "创建 7z 归档...");
        
        // ==================== 第一步：创建 7z 归档（使用 LZMA2，不使用 Zstd！） ====================
        let mut seven_z_data = Vec::new();
        {
            let cursor = Cursor::new(&mut seven_z_data);
            let mut sz_writer = SevenZWriter::new(cursor)
                .map_err(|e| SzError::Compress(format!("创建7z失败: {}", e)))?;
            
            // 使用 LZMA2（7z 原生支持的压缩方法）
            // 这里只做归档，主要压缩由外层 Zstd 完成
            sz_writer.set_content_methods(vec![
                SevenZMethodConfiguration::new(SevenZMethod::LZMA2)
                    .with_options(MethodOptions::LZMA2(LZMA2Options::with_preset(1))), // 低级别，快速归档
            ]);
            
            let files = self.collect_files_with_relative(input_paths)?;
            
            for (absolute_path, relative_name) in &files {
                let path = Path::new(absolute_path);
                
                if path.is_file() {
                    let file_size = fs::metadata(path)?.len();
                    
                    let mut file_content = Vec::new();
                    let mut file = File::open(path)?;
                    file.read_to_end(&mut file_content)?;
                    
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
        progress_callback((total_size as f64 * 0.4) as u64, total_size, "Zstd 压缩中...");
        
        // ==================== 第二步：用 Zstd 压缩整个 7z 数据 ====================
        let zstd_level = self.compression_level.clamp(1, 22);
        let zstd_data = zstd::encode_all(Cursor::new(&seven_z_data), zstd_level)
            .map_err(|e| SzError::Compress(format!("Zstd压缩失败: {}", e)))?;
        
        let compressed_size = zstd_data.len() as u64;
        progress_callback((total_size as f64 * 0.7) as u64, total_size, "AES-256-GCM 加密中...");
        
        // ==================== 第三步：AES-256-GCM 加密 ====================
        // 生成随机 Salt 和 Nonce
        let mut salt = [0u8; SALT_LEN];
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce_bytes);
        
        // 使用 Argon2 派生密钥
        let key = derive_key(password, &salt)?;
        
        // AES-256-GCM 加密
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| SzError::Encryption(format!("创建加密器失败: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let encrypted_data = cipher.encrypt(nonce, zstd_data.as_ref())
            .map_err(|e| SzError::Encryption(format!("加密失败: {}", e)))?;
        
        progress_callback((total_size as f64 * 0.9) as u64, total_size, "写入文件...");
        
        // ==================== 第四步：写入专属格式文件 ====================
        // 文件格式：[MAGIC 4字节][VERSION 1字节][NONCE 12字节][SALT 16字节][加密数据...]
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut output_file = BufWriter::new(File::create(output_path)?);
        
        output_file.write_all(MAGIC)?;           // 4 bytes: "SZ7Z"
        output_file.write_all(&[VERSION])?;      // 1 byte: version
        output_file.write_all(&nonce_bytes)?;    // 12 bytes: nonce
        output_file.write_all(&salt)?;           // 16 bytes: salt
        output_file.write_all(&encrypted_data)?; // 加密数据 (包含 GCM tag)
        output_file.flush()?;
        
        let final_size = fs::metadata(output_path)?.len();
        let duration = start_time.elapsed();
        
        progress_callback(total_size, total_size, "完成");
        
        Ok(EncryptedResult {
            success: true,
            original_size: total_size,
            compressed_size: seven_z_size, // 7z 归档大小
            encrypted_size: final_size,
            output_path: output_path.to_string(),
            duration_ms: duration.as_millis() as u64,
        })
    }
    
    /// 专属解密解压
    /// 
    /// 流程：
    /// 1. 读取文件头，验证 MAGIC
    /// 2. 读取 VERSION, NONCE, SALT
    /// 3. AES-256-GCM 解密 → Zstd 压缩数据
    /// 4. Zstd 解压 → 7z 数据
    /// 5. 7z 解压 → 源文件
    pub fn decompress_encrypted<F>(
        &self,
        archive_path: &str,
        output_dir: &str,
        password: &str,
        mut progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(u64, u64, &str),
    {
        let file_size = fs::metadata(archive_path)?.len();
        
        progress_callback(0, file_size, "读取文件头...");
        
        // ==================== 第一步：读取并验证文件头 ====================
        let mut file = BufReader::new(File::open(archive_path)?);
        
        // 验证魔数
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(SzError::Decryption(format!(
                "无效的专属加密文件格式（魔数不匹配：期望 {:?}，实际 {:?}）", 
                MAGIC, magic
            )));
        }
        
        // 读取版本
        let mut version = [0u8; 1];
        file.read_exact(&mut version)?;
        if version[0] > VERSION {
            return Err(SzError::Decryption(format!("不支持的版本: {}", version[0])));
        }
        
        // 读取 Nonce 和 Salt
        let mut nonce_bytes = [0u8; NONCE_LEN];
        let mut salt = [0u8; SALT_LEN];
        file.read_exact(&mut nonce_bytes)?;
        file.read_exact(&mut salt)?;
        
        progress_callback(file_size / 10, file_size, "派生密钥...");
        
        // ==================== 第二步：读取加密数据 ====================
        let mut encrypted_data = Vec::new();
        file.read_to_end(&mut encrypted_data)?;
        
        progress_callback(file_size / 5, file_size, "AES-256-GCM 解密中...");
        
        // ==================== 第三步：AES-256-GCM 解密 ====================
        let key = derive_key(password, &salt)?;
        
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| SzError::Decryption(format!("创建解密器失败: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let zstd_data = cipher.decrypt(nonce, encrypted_data.as_ref())
            .map_err(|_| SzError::Decryption("解密失败：密码错误或文件损坏".to_string()))?;
        
        progress_callback(file_size / 3, file_size, "Zstd 解压中...");
        
        // ==================== 第四步：Zstd 解压 ====================
        let seven_z_data = zstd::decode_all(Cursor::new(&zstd_data))
            .map_err(|e| SzError::Decompress(format!("Zstd解压失败: {}", e)))?;
        
        progress_callback(file_size / 2, file_size, "7z 解压中...");
        
        // ==================== 第五步：7z 解压 ====================
        let output_path = Path::new(output_dir);
        fs::create_dir_all(output_path)?;
        
        // 使用 sevenz-rust 解压内存中的 7z 数据
        let cursor = Cursor::new(&seven_z_data);
        
        sevenz_rust::decompress(cursor, output_path)
            .map_err(|e| SzError::Decompress(format!("7z解压失败: {}", e)))?;
        
        // 收集解压后的文件
        let mut extracted_files = Vec::new();
        self.collect_extracted_files(output_path, &mut extracted_files)?;
        
        progress_callback(file_size, file_size, "完成");
        
        Ok(extracted_files)
    }
    
    /// 验证密码是否正确
    pub fn verify_password(&self, archive_path: &str, password: &str) -> SzResult<bool> {
        let mut file = BufReader::new(File::open(archive_path)?);
        
        // 验证魔数
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Ok(false);
        }
        
        // 跳过版本
        let mut version = [0u8; 1];
        file.read_exact(&mut version)?;
        
        // 读取 Nonce 和 Salt
        let mut nonce_bytes = [0u8; NONCE_LEN];
        let mut salt = [0u8; SALT_LEN];
        file.read_exact(&mut nonce_bytes)?;
        file.read_exact(&mut salt)?;
        
        // 读取加密数据
        let mut encrypted_data = Vec::new();
        file.read_to_end(&mut encrypted_data)?;
        
        // 尝试解密
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
    
    /// 检查文件是否为专属加密格式
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
