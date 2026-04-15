//! .zbak 解压读取器
//!
//! 支持:
//! - 全部解压
//! - 仅读索引（列出内容）
//! - 随机访问提取单个文件

use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, Component};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use byteorder::{LittleEndian, ReadBytesExt};
use crc32fast::Hasher as Crc32Hasher;

use sz_core::{SzError, SzResult};
use super::format::*;
use super::crypto;

/// 单次分配上限 (512MB)：超过此值的 index_size/compressed_size 视为恶意
const MAX_SANE_ALLOC: u64 = 512 * 1024 * 1024;

/// 路径穿越防护：确保解压路径不会逃逸出目标目录
///
/// 拒绝：
/// - 绝对路径 (/etc/passwd, C:\Windows\...)
/// - 包含 ".." 的组件 (../../secret)
/// - 当 join 后的规范路径不以 base_dir 开头时
fn safe_join(base_dir: &str, entry_path: &str) -> SzResult<std::path::PathBuf> {
    let entry = Path::new(entry_path);

    // 检查每个路径组件
    for component in entry.components() {
        match component {
            Component::ParentDir => {
                return Err(SzError::DataCorrupted(format!(
                    "路径穿越攻击：路径包含 '..' — '{}'", entry_path
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(SzError::DataCorrupted(format!(
                    "路径穿越攻击：绝对路径 — '{}'", entry_path
                )));
            }
            _ => {}
        }
    }

    let joined = Path::new(base_dir).join(entry);

    // 双重检查：规范化后的路径必须仍在 base_dir 内
    // 用 starts_with 检查，即使有符号链接也能防护
    let base_canonical = fs::canonicalize(base_dir).unwrap_or_else(|_| Path::new(base_dir).to_path_buf());
    // joined 可能还不存在，取其父目录检查
    if let Some(parent) = joined.parent() {
        if parent.exists() {
            let parent_canonical = fs::canonicalize(parent)
                .unwrap_or_else(|_| parent.to_path_buf());
            if !parent_canonical.starts_with(&base_canonical) {
                return Err(SzError::DataCorrupted(format!(
                    "路径穿越攻击：解析后路径逃逸出目标目录 — '{}'", entry_path
                )));
            }
        }
    }

    Ok(joined)
}

/// .zbak 读取器
pub struct ZbakReader {
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl ZbakReader {
    pub fn new() -> Self {
        Self { cancel_flag: None }
    }

    pub fn with_cancel_flag(cancel_flag: Arc<AtomicBool>) -> Self {
        Self { cancel_flag: Some(cancel_flag) }
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag
            .as_ref()
            .map_or(false, |f| f.load(Ordering::Relaxed))
    }

    /// 检查是否需要密码
    pub fn requires_password(archive_path: &str) -> SzResult<bool> {
        let mut file = BufReader::new(File::open(archive_path)?);
        let header = ZbakHeader::read_from(&mut file)?;
        Ok(header.is_encrypted())
    }

    /// 快速验证密码（不解压任何数据）
    pub fn verify_password(archive_path: &str, password: &str) -> SzResult<bool> {
        let mut file = BufReader::new(File::open(archive_path)?);
        let header = ZbakHeader::read_from(&mut file)?;

        if !header.is_encrypted() {
            return Ok(true); // 无加密, 任何密码都"正确"
        }

        let master_key = crypto::derive_master_key(password, &header.salt)?;
        let verify_key = crypto::derive_verify_key(&master_key);
        Ok(crypto::check_verify_block(&verify_key, &header.verify_nonce, &header.verify_tag))
    }

    /// 读取索引（列出内容）
    pub fn list_contents(
        archive_path: &str,
        password: Option<&str>,
    ) -> SzResult<Vec<ZbakIndexEntry>> {
        let mut file = BufReader::new(File::open(archive_path)?);
        let header = ZbakHeader::read_from(&mut file)?;

        // 密码校验
        let master_key = if header.is_encrypted() {
            let pwd = password.ok_or(SzError::WrongPassword)?;
            let mk = crypto::derive_master_key(pwd, &header.salt)?;
            let verify_key = crypto::derive_verify_key(&mk);
            if !crypto::check_verify_block(&verify_key, &header.verify_nonce, &header.verify_tag) {
                return Err(SzError::WrongPassword);
            }
            Some(mk)
        } else {
            None
        };

        // [安全] 索引大小合理性检查：防止恶意文件触发 OOM
        if header.index_size as u64 > MAX_SANE_ALLOC {
            return Err(SzError::DataCorrupted(format!(
                "索引区大小异常: {} 字节（上限 {}）", header.index_size, MAX_SANE_ALLOC
            )));
        }

        // 跳到索引区
        file.seek(SeekFrom::Start(header.index_offset))?;
        let mut index_data = vec![0u8; header.index_size as usize];
        file.read_exact(&mut index_data)?;

        // 解密索引（如果文件名加密）
        let index_bytes = if header.is_filename_encrypted() {
            let mk = master_key.as_ref().ok_or(SzError::WrongPassword)?;
            let index_key = crypto::derive_index_key(mk);
            crypto::decrypt_index(&index_key, &index_data)?
        } else {
            index_data
        };

        let mut cursor = std::io::Cursor::new(&index_bytes);
        read_index(&mut cursor, header.entry_count)
    }

    /// 全部解压
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
        let mut file = BufReader::new(File::open(archive_path)?);
        let header = ZbakHeader::read_from(&mut file)?;

        // 密码校验
        let master_key = if header.is_encrypted() {
            let pwd = password.ok_or(SzError::WrongPassword)?;
            let mk = crypto::derive_master_key(pwd, &header.salt)?;
            let verify_key = crypto::derive_verify_key(&mk);
            if !crypto::check_verify_block(&verify_key, &header.verify_nonce, &header.verify_tag) {
                return Err(SzError::WrongPassword);
            }
            Some(mk)
        } else {
            None
        };

        // 读索引
        file.seek(SeekFrom::Start(header.index_offset))?;
        let mut index_data = vec![0u8; header.index_size as usize];
        file.read_exact(&mut index_data)?;

        let index_bytes = if header.is_filename_encrypted() {
            let mk = master_key.as_ref().ok_or(SzError::WrongPassword)?;
            let index_key = crypto::derive_index_key(mk);
            crypto::decrypt_index(&index_key, &index_data)?
        } else {
            index_data
        };

        let mut cursor = std::io::Cursor::new(&index_bytes);
        let entries = read_index(&mut cursor, header.entry_count)?;

        // 计算总大小
        let total_size: u64 = entries.iter().map(|e| e.original_size).sum();
        let mut processed: u64 = 0;

        // 确保输出目录存在
        fs::create_dir_all(output_dir)?;

        let mut extracted_files = Vec::new();

        for (file_idx, entry) in entries.iter().enumerate() {
            if self.is_cancelled() {
                return Err(SzError::Cancelled);
            }

            progress_callback(processed, total_size, &entry.path);

            // [安全] 路径穿越防护：拒绝包含 ".." 或绝对路径的条目
            let output_path = safe_join(output_dir, &entry.path)?;

            if entry.is_directory {
                fs::create_dir_all(&output_path)?;
                extracted_files.push(entry.path.clone());
                continue;
            }

            // 确保父目录存在
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // 读取数据块
            file.seek(SeekFrom::Start(entry.block_offset))?;
            let compressed_size = file.read_u64::<LittleEndian>()?;
            let _original_size = file.read_u64::<LittleEndian>()?;
            let mut nonce = [0u8; NONCE_SIZE];
            file.read_exact(&mut nonce)?;

            // [安全] 数据块大小合理性检查：防止恶意文件触发 OOM
            if compressed_size > MAX_SANE_ALLOC {
                return Err(SzError::DataCorrupted(format!(
                    "文件 '{}' 数据块大小异常: {} 字节", entry.path, compressed_size
                )));
            }

            let mut block_data = vec![0u8; compressed_size as usize];
            file.read_exact(&mut block_data)?;

            // 解密
            let compressed_data = if header.is_encrypted() {
                let mk = master_key.as_ref().ok_or(SzError::WrongPassword)?;
                let file_key = crypto::derive_file_key(mk, file_idx as u32);
                crypto::decrypt_block(&file_key, &nonce, &block_data)
                    .map_err(|_| SzError::Decryption(format!(
                        "文件 '{}' 解密失败（数据损坏）", entry.path
                    )))?
            } else {
                block_data
            };

            // Zstd 解压
            let decompressed = zstd::decode_all(&compressed_data[..])
                .map_err(|e| SzError::Decompress(format!(
                    "文件 '{}' Zstd 解压失败: {}", entry.path, e
                )))?;

            // CRC32 校验
            let mut crc_hasher = Crc32Hasher::new();
            crc_hasher.update(&decompressed);
            let crc32_val = crc_hasher.finalize();
            if crc32_val != entry.crc32 {
                return Err(SzError::Decompress(format!(
                    "文件 '{}' CRC32 校验失败 (期望: {:08x}, 实际: {:08x})",
                    entry.path, entry.crc32, crc32_val
                )));
            }

            // 写文件
            let mut out_file = File::create(&output_path)?;
            out_file.write_all(&decompressed)?;

            extracted_files.push(entry.path.clone());
            processed += entry.original_size;
            progress_callback(processed, total_size, &entry.path);
        }

        progress_callback(total_size, total_size, "完成");
        Ok(extracted_files)
    }

    /// 随机访问提取单个文件
    pub fn extract_file(
        archive_path: &str,
        file_path: &str,
        output_path: &str,
        password: Option<&str>,
    ) -> SzResult<()> {
        // 读索引找到目标文件
        let entries = Self::list_contents(archive_path, password)?;
        let (file_idx, entry) = entries.iter().enumerate()
            .find(|(_, e)| e.path == file_path)
            .ok_or_else(|| SzError::FileNotFound(format!(
                "压缩包中未找到文件: {}", file_path
            )))?;

        if entry.is_directory {
            fs::create_dir_all(output_path)?;
            return Ok(());
        }

        let mut file = BufReader::new(File::open(archive_path)?);
        let header = ZbakHeader::read_from(&mut file)?;

        let master_key = if header.is_encrypted() {
            let pwd = password.ok_or(SzError::WrongPassword)?;
            let mk = crypto::derive_master_key(pwd, &header.salt)?;
            Some(mk)
        } else {
            None
        };

        // 跳到数据块
        file.seek(SeekFrom::Start(entry.block_offset))?;
        let compressed_size = file.read_u64::<LittleEndian>()?;
        let _original_size = file.read_u64::<LittleEndian>()?;
        let mut nonce = [0u8; NONCE_SIZE];
        file.read_exact(&mut nonce)?;

        let mut block_data = vec![0u8; compressed_size as usize];
        file.read_exact(&mut block_data)?;

        // 解密
        let compressed_data = if header.is_encrypted() {
            let mk = master_key.as_ref().ok_or(SzError::WrongPassword)?;
            let file_key = crypto::derive_file_key(mk, file_idx as u32);
            crypto::decrypt_block(&file_key, &nonce, &block_data)?
        } else {
            block_data
        };

        // 解压
        let decompressed = zstd::decode_all(&compressed_data[..])
            .map_err(|e| SzError::Decompress(format!("Zstd 解压失败: {}", e)))?;

        // CRC32 校验
        let mut crc_hasher = Crc32Hasher::new();
        crc_hasher.update(&decompressed);
        let crc32_val = crc_hasher.finalize();
        if crc32_val != entry.crc32 {
            return Err(SzError::Decompress(format!(
                "CRC32 校验失败 (期望: {:08x}, 实际: {:08x})",
                entry.crc32, crc32_val
            )));
        }

        // 确保输出目录存在
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out_file = File::create(output_path)?;
        out_file.write_all(&decompressed)?;

        Ok(())
    }
}

impl Default for ZbakReader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::writer::ZbakWriter;
    use tempfile::TempDir;

    fn create_test_files(dir: &std::path::Path) {
        let mut f1 = File::create(dir.join("hello.txt")).unwrap();
        f1.write_all(b"Hello, World!").unwrap();

        let mut f2 = File::create(dir.join("data.bin")).unwrap();
        f2.write_all(&vec![42u8; 2048]).unwrap();

        fs::create_dir_all(dir.join("subdir")).unwrap();
        let mut f3 = File::create(dir.join("subdir/nested.txt")).unwrap();
        f3.write_all(b"Nested file content here").unwrap();
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let archive = tmp.path().join("test.zbak");
        fs::create_dir_all(&src).unwrap();
        create_test_files(&src);

        // 压缩
        let writer = ZbakWriter::new(3);
        writer.compress(
            &[src.to_string_lossy().to_string()],
            archive.to_str().unwrap(),
            None,
            |_, _, _| {},
        ).unwrap();

        // 解压
        let reader = ZbakReader::new();
        let files = reader.decompress(
            archive.to_str().unwrap(),
            dst.to_str().unwrap(),
            None,
            |_, _, _| {},
        ).unwrap();

        assert!(!files.is_empty());

        // 验证文件内容
        let hello = fs::read_to_string(dst.join("src/hello.txt")).unwrap();
        assert_eq!(hello, "Hello, World!");

        let data = fs::read(dst.join("src/data.bin")).unwrap();
        assert_eq!(data.len(), 2048);
        assert!(data.iter().all(|&b| b == 42));
    }

    #[test]
    fn test_encrypted_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let archive = tmp.path().join("enc.zbak");
        fs::create_dir_all(&src).unwrap();
        create_test_files(&src);

        let password = "my_secure_password_123";

        // 压缩
        let writer = ZbakWriter::new(3);
        writer.compress(
            &[src.to_string_lossy().to_string()],
            archive.to_str().unwrap(),
            Some(password),
            |_, _, _| {},
        ).unwrap();

        // 验证需要密码
        assert!(ZbakReader::requires_password(archive.to_str().unwrap()).unwrap());

        // 验证密码正确
        assert!(ZbakReader::verify_password(archive.to_str().unwrap(), password).unwrap());

        // 验证密码错误
        assert!(!ZbakReader::verify_password(archive.to_str().unwrap(), "wrong_password").unwrap());

        // 解压（正确密码）
        let reader = ZbakReader::new();
        let files = reader.decompress(
            archive.to_str().unwrap(),
            dst.to_str().unwrap(),
            Some(password),
            |_, _, _| {},
        ).unwrap();
        assert!(!files.is_empty());

        // 验证内容
        let hello = fs::read_to_string(dst.join("src/hello.txt")).unwrap();
        assert_eq!(hello, "Hello, World!");
    }

    #[test]
    fn test_wrong_password_fails() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let archive = tmp.path().join("enc2.zbak");
        fs::create_dir_all(&src).unwrap();

        let mut f = File::create(src.join("secret.txt")).unwrap();
        f.write_all(b"secret data").unwrap();

        let writer = ZbakWriter::new(3);
        writer.compress(
            &[src.to_string_lossy().to_string()],
            archive.to_str().unwrap(),
            Some("correct_password"),
            |_, _, _| {},
        ).unwrap();

        let reader = ZbakReader::new();
        let result = reader.decompress(
            archive.to_str().unwrap(),
            dst.to_str().unwrap(),
            Some("wrong_password"),
            |_, _, _| {},
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_list_contents() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let archive = tmp.path().join("list.zbak");
        fs::create_dir_all(&src).unwrap();
        create_test_files(&src);

        let writer = ZbakWriter::new(3);
        writer.compress(
            &[src.to_string_lossy().to_string()],
            archive.to_str().unwrap(),
            None,
            |_, _, _| {},
        ).unwrap();

        let entries = ZbakReader::list_contents(archive.to_str().unwrap(), None).unwrap();
        assert!(!entries.is_empty());

        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("hello.txt")));
    }

    #[test]
    fn test_extract_single_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let archive = tmp.path().join("extract.zbak");
        fs::create_dir_all(&src).unwrap();
        create_test_files(&src);

        let writer = ZbakWriter::new(3);
        writer.compress(
            &[src.to_string_lossy().to_string()],
            archive.to_str().unwrap(),
            None,
            |_, _, _| {},
        ).unwrap();

        // 列出内容找到目标文件
        let entries = ZbakReader::list_contents(archive.to_str().unwrap(), None).unwrap();
        let hello_entry = entries.iter().find(|e| e.path.contains("hello.txt")).unwrap();

        let output = tmp.path().join("extracted_hello.txt");
        ZbakReader::extract_file(
            archive.to_str().unwrap(),
            &hello_entry.path,
            output.to_str().unwrap(),
            None,
        ).unwrap();

        let content = fs::read_to_string(&output).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_encrypted_filename_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let archive = tmp.path().join("enc_fn.zbak");
        fs::create_dir_all(&src).unwrap();
        create_test_files(&src);

        let password = "filename_encrypt_test";

        // 压缩（启用文件名加密）
        let mut writer = ZbakWriter::new(3);
        writer.set_encrypt_filenames(true);
        writer.compress(
            &[src.to_string_lossy().to_string()],
            archive.to_str().unwrap(),
            Some(password),
            |_, _, _| {},
        ).unwrap();

        // 无密码时无法列出内容
        assert!(ZbakReader::list_contents(archive.to_str().unwrap(), None).is_err());

        // 正确密码可以列出内容
        let entries = ZbakReader::list_contents(archive.to_str().unwrap(), Some(password)).unwrap();
        assert!(!entries.is_empty());
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("hello.txt")));

        // 正确密码可以解压
        let reader = ZbakReader::new();
        let files = reader.decompress(
            archive.to_str().unwrap(),
            dst.to_str().unwrap(),
            Some(password),
            |_, _, _| {},
        ).unwrap();
        assert!(!files.is_empty());

        let hello = fs::read_to_string(dst.join("src/hello.txt")).unwrap();
        assert_eq!(hello, "Hello, World!");
    }
}
