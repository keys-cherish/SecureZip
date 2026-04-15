//! 分卷压缩模块
//!
//! 将 .zbak 文件拆分为多个固定大小的分卷: .zbak.001, .zbak.002, ...
//! 解压时自动检测分卷并拼接还原。

use std::fs::{self, File};
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::path::Path;

use sz_core::{SzError, SzResult};

/// 分卷文件扩展名格式: .001, .002, ...
const VOLUME_EXT_LEN: usize = 3;

/// 将文件拆分为指定大小的分卷
///
/// - `input_path`: 原始 .zbak 文件路径
/// - `split_size`: 每个分卷的最大字节数
///
/// 返回所有分卷路径列表。如果文件小于 split_size，不拆分，返回空 Vec。
pub fn split_file(input_path: &str, split_size: u64) -> SzResult<Vec<String>> {
    if split_size == 0 {
        return Ok(Vec::new());
    }

    let path = Path::new(input_path);
    if !path.exists() {
        return Err(SzError::FileNotFound(input_path.to_string()));
    }

    let file_size = fs::metadata(path)?.len();

    // 文件小于等于 split_size，不需要拆分
    if file_size <= split_size {
        return Ok(Vec::new());
    }

    let base_name = input_path.to_string();
    let mut volumes = Vec::new();
    let mut reader = BufReader::with_capacity(1024 * 1024, File::open(path)?);
    let mut volume_index: u32 = 1;
    let mut remaining = file_size;

    while remaining > 0 {
        let volume_path = format!("{}.{:03}", &base_name, volume_index);
        let chunk_size = remaining.min(split_size);

        let mut writer = BufWriter::new(
            File::create(&volume_path)
                .map_err(|e| SzError::Compress(format!("创建分卷失败 {}: {}", &volume_path, e)))?
        );

        let mut written: u64 = 0;
        let mut buf = [0u8; 65536]; // 64KB 缓冲
        while written < chunk_size {
            let to_read = ((chunk_size - written) as usize).min(buf.len());
            let n = reader.read(&mut buf[..to_read])
                .map_err(|e| SzError::Compress(format!("读取源文件失败: {}", e)))?;
            if n == 0 {
                break;
            }
            writer.write_all(&buf[..n])
                .map_err(|e| SzError::Compress(format!("写入分卷失败: {}", e)))?;
            written += n as u64;
        }
        writer.flush()?;

        volumes.push(volume_path);
        remaining -= written;
        volume_index += 1;
    }

    // 删除原始文件
    let _ = fs::remove_file(path);

    Ok(volumes)
}

/// 检测路径是否为分卷文件，返回所有分卷路径（按序）
///
/// 支持两种输入:
/// - 第一个分卷 `xxx.zbak.001`
/// - 基础文件名 `xxx.zbak`（自动查找 .001, .002, ...）
pub fn detect_volumes(path: &str) -> Option<Vec<String>> {
    let p = Path::new(path);

    // 情况1: 输入是 .001 分卷
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        if ext.len() == VOLUME_EXT_LEN && ext.chars().all(|c| c.is_ascii_digit()) {
            // 提取基础路径 (去掉 .NNN)
            let base = &path[..path.len() - VOLUME_EXT_LEN - 1]; // 去掉 ".NNN"
            return collect_volumes(base);
        }
    }

    // 情况2: 输入是基础文件名，检查 .001 是否存在
    let first_volume = format!("{}.001", path);
    if Path::new(&first_volume).exists() {
        return collect_volumes(path);
    }

    None
}

/// 收集所有连续分卷路径
fn collect_volumes(base_path: &str) -> Option<Vec<String>> {
    let mut volumes = Vec::new();
    let mut idx: u32 = 1;

    loop {
        let volume_path = format!("{}.{:03}", base_path, idx);
        if Path::new(&volume_path).exists() {
            volumes.push(volume_path);
            idx += 1;
        } else {
            break;
        }
    }

    if volumes.is_empty() {
        None
    } else {
        Some(volumes)
    }
}

/// 将多个分卷拼接为单个文件
///
/// 返回拼接后的临时文件路径
pub fn join_volumes(volumes: &[String], output_path: &str) -> SzResult<()> {
    let mut writer = BufWriter::new(
        File::create(output_path)
            .map_err(|e| SzError::Decompress(format!("创建拼接文件失败: {}", e)))?
    );

    for volume in volumes {
        let mut reader = BufReader::with_capacity(
            1024 * 1024,
            File::open(volume)
                .map_err(|e| SzError::Decompress(format!("打开分卷失败 {}: {}", volume, e)))?
        );

        io::copy(&mut reader, &mut writer)
            .map_err(|e| SzError::Decompress(format!("拼接分卷失败: {}", e)))?;
    }

    writer.flush()?;
    Ok(())
}

/// 判断路径是否为分卷文件
pub fn is_split_volume(path: &str) -> bool {
    let p = Path::new(path);
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        if ext.len() == VOLUME_EXT_LEN && ext.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    // 也检查是否存在 .001 分卷
    Path::new(&format!("{}.001", path)).exists()
}

/// 从分卷路径中提取基础 .zbak 路径
///
/// `foo.zbak.001` → `foo.zbak`
pub fn base_path_from_volume(path: &str) -> Option<String> {
    let p = Path::new(path);
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        if ext.len() == VOLUME_EXT_LEN && ext.chars().all(|c| c.is_ascii_digit()) {
            return Some(path[..path.len() - VOLUME_EXT_LEN - 1].to_string());
        }
    }
    None
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_split_and_join() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("test.zbak");

        // 创建 1MB 的测试数据
        let data: Vec<u8> = (0..1_000_000u32).map(|i| (i % 256) as u8).collect();
        fs::write(&src, &data).unwrap();

        // 拆分为 300KB 分卷
        let volumes = split_file(src.to_str().unwrap(), 300_000).unwrap();
        assert_eq!(volumes.len(), 4); // 1MB / 300KB = 3.33 → 4 卷
        assert!(!src.exists()); // 原文件已删除
        assert!(volumes[0].ends_with(".001"));
        assert!(volumes[3].ends_with(".004"));

        // 检测分卷
        let detected = detect_volumes(&volumes[0]).unwrap();
        assert_eq!(detected.len(), 4);

        // 从基础路径检测
        let base = base_path_from_volume(&volumes[0]).unwrap();
        let detected2 = detect_volumes(&base).unwrap();
        assert_eq!(detected2.len(), 4);

        // 拼接
        let joined = tmp.path().join("joined.zbak");
        join_volumes(&volumes, joined.to_str().unwrap()).unwrap();

        let joined_data = fs::read(&joined).unwrap();
        assert_eq!(joined_data, data);
    }

    #[test]
    fn test_split_file_smaller_than_chunk() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("small.zbak");

        let data = vec![1u8; 100];
        fs::write(&src, &data).unwrap();

        // 分卷大小大于文件 → 不拆分
        let volumes = split_file(src.to_str().unwrap(), 1000).unwrap();
        assert!(volumes.is_empty());
        assert!(src.exists()); // 原文件保留
    }

    #[test]
    fn test_is_split_volume() {
        assert!(!is_split_volume("test.zbak"));
        // 无法在不创建文件的情况下测试 .001 检测
        // 但可以测试纯扩展名判断
        let tmp = TempDir::new().unwrap();
        let vol = tmp.path().join("test.zbak.001");
        fs::write(&vol, b"test").unwrap();
        assert!(is_split_volume(vol.to_str().unwrap()));
    }

    #[test]
    fn test_base_path_from_volume() {
        assert_eq!(
            base_path_from_volume("foo/bar.zbak.001"),
            Some("foo/bar.zbak".to_string())
        );
        assert_eq!(
            base_path_from_volume("test.zbak.023"),
            Some("test.zbak".to_string())
        );
        assert_eq!(base_path_from_volume("test.zbak"), None);
    }
}
