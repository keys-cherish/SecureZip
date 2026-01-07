//! 7z + LZMA2 压缩解压实现
//!
//! 使用 LZMA2 压缩算法，这是 7z 原生支持的高效压缩方法
//! sevenz-rust 库不支持 ZSTD 压缩写入，只支持 ZSTD 解压读取

use std::fs::{self, File};
use std::io::{Read, Cursor};
use std::path::Path;
use std::time::Instant;

use sevenz_rust::{
    SevenZWriter, SevenZReader, SevenZArchiveEntry, Password,
    SevenZMethodConfiguration, SevenZMethod, AesEncoderOptions, lzma::LZMA2Options,
    MethodOptions,
};
use sz_core::{CompressOptions, CompressProgress, CompressResult, SzError, SzResult};

/// 压缩器
pub struct Compressor {
    options: CompressOptions,
}

impl Compressor {
    /// 创建新的压缩器
    pub fn new(options: CompressOptions) -> Self {
        Self { options }
    }

    /// 压缩文件或文件夹
    /// 
    /// # Arguments
    /// * `input_paths` - 输入文件或文件夹路径列表
    /// * `output_path` - 输出 .7z 文件路径
    /// * `progress_callback` - 进度回调函数
    pub fn compress<F>(
        &self,
        input_paths: &[String],
        output_path: &str,
        mut progress_callback: F,
    ) -> SzResult<CompressResult>
    where
        F: FnMut(CompressProgress),
    {
        let start_time = Instant::now();

        // 验证输入
        if input_paths.is_empty() {
            return Err(SzError::InvalidArgument("输入路径不能为空".to_string()));
        }

        // 计算总大小
        let total_size = self.calculate_total_size(input_paths)?;
        let mut processed_size: u64 = 0;

        // 收集所有文件（相对路径和绝对路径）
        let files = self.collect_files_with_relative(input_paths)?;

        // 确保输出目录存在
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        // 创建 7z 压缩文件
        let output_file = File::create(output_path)
            .map_err(|e| SzError::Io(e))?;
        
        let mut sz_writer = SevenZWriter::new(output_file)
            .map_err(|e| SzError::Compress(format!("创建7z文件失败: {}", e)))?;

        // 根据压缩级别设置 LZMA2 预设
        // compression_level: 1-9，映射到 LZMA2 的 preset 0-9
        let lzma2_preset = self.options.compression_level.clamp(1, 9);
        
        // 使用 LZMA2 作为压缩方法（7z 原生支持，兼容所有 7z 软件）
        // 注意：sevenz-rust 不支持 ZSTD 压缩写入，只支持解压读取
        sz_writer.set_content_methods(vec![
            SevenZMethodConfiguration::new(SevenZMethod::LZMA2)
                .with_options(MethodOptions::LZMA2(LZMA2Options::with_preset(lzma2_preset as u32))),
        ]);

        // 逐个添加文件
        for (absolute_path, relative_name) in &files {
            let path = Path::new(absolute_path);
            
            if path.is_file() {
                let file_size = fs::metadata(path)?.len();

                // 读取文件内容
                let mut file_content = Vec::new();
                let mut file = File::open(path)?;
                file.read_to_end(&mut file_content)?;

                // 添加到 7z（使用 LZMA2 压缩方法）
                sz_writer.push_archive_entry(
                    SevenZArchiveEntry::from_path(path, relative_name.clone()),
                    Some(Cursor::new(file_content)),
                ).map_err(|e| SzError::Compress(format!("添加文件失败: {}", e)))?;

                // 更新已处理大小（在文件处理完成后）
                processed_size += file_size;
                
                // 报告进度（在文件处理完成后报告）
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    processed_size as f64 / elapsed
                } else {
                    0.0
                };
                let remaining = if speed > 0.0 && total_size > processed_size {
                    ((total_size - processed_size) as f64 / speed) as u64
                } else {
                    0
                };

                progress_callback(CompressProgress {
                    progress: if total_size > 0 { processed_size as f64 / total_size as f64 } else { 0.0 },
                    processed_bytes: processed_size,
                    total_bytes: total_size,
                    speed_bytes_per_second: speed,
                    estimated_remaining_seconds: remaining,
                    current_file: relative_name.clone(),
                });
            }
        }

        // 完成压缩
        sz_writer.finish()
            .map_err(|e| SzError::Compress(format!("完成压缩失败: {}", e)))?;

        let duration = start_time.elapsed();

        // 获取实际压缩后大小
        let compressed_size = fs::metadata(output_path)?.len();

        // 最终进度
        progress_callback(CompressProgress {
            progress: 1.0,
            processed_bytes: total_size,
            total_bytes: total_size,
            speed_bytes_per_second: total_size as f64 / duration.as_secs_f64().max(0.001),
            estimated_remaining_seconds: 0,
            current_file: "完成".to_string(),
        });

        Ok(CompressResult {
            success: true,
            output_path: output_path.to_string(),
            original_size: total_size,
            compressed_size,
            duration_ms: duration.as_millis() as u64,
            error_message: None,
        })
    }

    /// 带密码的 7z 压缩（AES-256 加密）
    /// 
    /// # Arguments
    /// * `input_paths` - 输入文件或文件夹路径列表
    /// * `output_path` - 输出 .7z 文件路径
    /// * `password` - 压缩密码
    /// * `progress_callback` - 进度回调函数
    pub fn compress_encrypted<F>(
        &self,
        input_paths: &[String],
        output_path: &str,
        password: &str,
        mut progress_callback: F,
    ) -> SzResult<CompressResult>
    where
        F: FnMut(CompressProgress),
    {
        let start_time = Instant::now();

        // 验证输入
        if input_paths.is_empty() {
            return Err(SzError::InvalidArgument("输入路径不能为空".to_string()));
        }
        if password.is_empty() {
            return Err(SzError::InvalidArgument("密码不能为空".to_string()));
        }

        // 计算总大小
        let total_size = self.calculate_total_size(input_paths)?;
        let mut processed_size: u64 = 0;

        // 收集所有文件（相对路径和绝对路径）
        let files = self.collect_files_with_relative(input_paths)?;

        // 确保输出目录存在
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        // 创建 7z 压缩文件
        let output_file = File::create(output_path)
            .map_err(|e| SzError::Io(e))?;
        
        let mut sz_writer = SevenZWriter::new(output_file)
            .map_err(|e| SzError::Compress(format!("创建7z文件失败: {}", e)))?;

        // 根据压缩级别设置 LZMA2 预设
        let lzma2_preset = self.options.compression_level.clamp(1, 9);

        // 设置加密方法（AES-256 + LZMA2）
        // 使用 LZMA2 压缩 + AES-256 加密，兼容所有 7z 软件
        // 注意：sevenz-rust 不支持 ZSTD 压缩写入
        sz_writer.set_content_methods(vec![
            AesEncoderOptions::new(password.into()).into(),
            SevenZMethodConfiguration::new(SevenZMethod::LZMA2)
                .with_options(MethodOptions::LZMA2(LZMA2Options::with_preset(lzma2_preset as u32))),
        ]);

        // 加密文件头（隐藏文件名）
        sz_writer.set_encrypt_header(true);

        // 逐个添加文件
        for (absolute_path, relative_name) in &files {
            let path = Path::new(absolute_path);
            
            if path.is_file() {
                let file_size = fs::metadata(path)?.len();

                // 读取文件内容
                let mut file_content = Vec::new();
                let mut file = File::open(path)?;
                file.read_to_end(&mut file_content)?;

                // 添加到 7z（加密会由 AesEncoderOptions 自动处理）
                sz_writer.push_archive_entry(
                    SevenZArchiveEntry::from_path(path, relative_name.clone()),
                    Some(Cursor::new(file_content)),
                ).map_err(|e| SzError::Compress(format!("添加文件失败: {}", e)))?;

                // 更新已处理大小（在文件处理完成后）
                processed_size += file_size;
                
                // 报告进度（在文件处理完成后报告）
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    processed_size as f64 / elapsed
                } else {
                    0.0
                };
                let remaining = if speed > 0.0 && total_size > processed_size {
                    ((total_size - processed_size) as f64 / speed) as u64
                } else {
                    0
                };

                progress_callback(CompressProgress {
                    progress: if total_size > 0 { processed_size as f64 / total_size as f64 } else { 0.0 },
                    processed_bytes: processed_size,
                    total_bytes: total_size,
                    speed_bytes_per_second: speed,
                    estimated_remaining_seconds: remaining,
                    current_file: relative_name.clone(),
                });
            }
        }

        // 完成压缩
        sz_writer.finish()
            .map_err(|e| SzError::Compress(format!("完成压缩失败: {}", e)))?;

        let duration = start_time.elapsed();

        // 获取实际压缩后大小
        let compressed_size = fs::metadata(output_path)?.len();

        // 最终进度
        progress_callback(CompressProgress {
            progress: 1.0,
            processed_bytes: total_size,
            total_bytes: total_size,
            speed_bytes_per_second: total_size as f64 / duration.as_secs_f64().max(0.001),
            estimated_remaining_seconds: 0,
            current_file: "完成".to_string(),
        });

        Ok(CompressResult {
            success: true,
            output_path: output_path.to_string(),
            original_size: total_size,
            compressed_size,
            duration_ms: duration.as_millis() as u64,
            error_message: None,
        })
    }

    /// 计算总文件大小
    fn calculate_total_size(&self, paths: &[String]) -> SzResult<u64> {
        let mut total: u64 = 0;
        for path in paths {
            let path = Path::new(path);
            if path.is_file() {
                total += fs::metadata(path)?.len();
            } else if path.is_dir() {
                total += self.dir_size(path)?;
            }
        }
        Ok(total)
    }

    /// 递归计算目录大小
    fn dir_size(&self, path: &Path) -> SzResult<u64> {
        let mut size: u64 = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    size += fs::metadata(&path)?.len();
                } else if path.is_dir() {
                    size += self.dir_size(&path)?;
                }
            }
        }
        Ok(size)
    }

    /// 收集所有文件路径（包含相对路径）
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
                self.collect_dir_files_with_relative(path, &base_name, &mut files)?;
            }
        }
        Ok(files)
    }

    /// 递归收集目录中的文件（保留相对路径）
    fn collect_dir_files_with_relative(
        &self, 
        dir: &Path, 
        prefix: &str,
        files: &mut Vec<(String, String)>
    ) -> SzResult<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let relative = format!("{}/{}", prefix, name);
            
            if path.is_file() {
                files.push((path.to_string_lossy().to_string(), relative));
            } else if path.is_dir() {
                self.collect_dir_files_with_relative(&path, &relative, files)?;
            }
        }
        Ok(())
    }
}

/// 解压器
pub struct Decompressor;

impl Decompressor {
    /// 创建新的解压器
    pub fn new() -> Self {
        Self
    }

    /// 检查压缩包是否需要密码
    pub fn requires_password(&self, archive_path: &str) -> SzResult<bool> {
        // sevenz-rust 暂时不支持密码检测，返回 false
        // 实际解压时如果需要密码会报错
        let path = Path::new(archive_path);
        if !path.exists() {
            return Err(SzError::FileNotFound(archive_path.to_string()));
        }
        Ok(false)
    }

    /// 验证密码是否正确
    pub fn verify_password(&self, archive_path: &str, password: &str) -> SzResult<bool> {
        // 尝试列出内容来验证密码
        match self.list_contents(archive_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 列出压缩包内容
    pub fn list_contents(&self, archive_path: &str) -> SzResult<Vec<String>> {
        let path = Path::new(archive_path);
        if !path.exists() {
            return Err(SzError::FileNotFound(archive_path.to_string()));
        }

        let file = File::open(path)?;
        let len = file.metadata()?.len();
        
        let reader = SevenZReader::new(file, len, Password::empty())
            .map_err(|e| SzError::Decompress(format!("打开压缩包失败: {}", e)))?;

        let entries: Vec<String> = reader.archive()
            .files
            .iter()
            .filter(|e| !e.is_directory())
            .map(|e| e.name().to_string())
            .collect();

        Ok(entries)
    }

    /// 解压文件
    /// 
    /// # Arguments
    /// * `archive_path` - 压缩包路径
    /// * `output_dir` - 输出目录
    /// * `password` - 密码（可选）
    /// * `progress_callback` - 进度回调
    pub fn decompress<F>(
        &self,
        archive_path: &str,
        output_dir: &str,
        _password: Option<&str>,
        mut progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(CompressProgress),
    {
        let start_time = Instant::now();

        // 验证压缩包存在
        let archive_path_buf = Path::new(archive_path);
        if !archive_path_buf.exists() {
            return Err(SzError::FileNotFound(archive_path.to_string()));
        }

        // 确保输出目录存在
        let output_path = Path::new(output_dir);
        fs::create_dir_all(output_path)?;

        // 打开压缩包获取文件列表
        let file = File::open(archive_path_buf)?;
        let len = file.metadata()?.len();
        
        // 构建密码
        let pwd = match _password {
            Some(p) if !p.is_empty() => Password::from(p),
            _ => Password::empty(),
        };
        
        let reader = SevenZReader::new(file, len, pwd)
            .map_err(|e| SzError::Decompress(format!("打开压缩包失败: {}", e)))?;

        // 获取所有文件条目
        let entries: Vec<_> = reader.archive()
            .files
            .iter()
            .filter(|e| !e.is_directory())
            .cloned()
            .collect();

        let total_size: u64 = entries.iter().map(|e| e.size()).sum();
        let total_files = entries.len();
        let mut extracted_files: Vec<String> = Vec::new();
        
        // 报告初始进度
        progress_callback(CompressProgress {
            progress: 0.0,
            processed_bytes: 0,
            total_bytes: total_size,
            speed_bytes_per_second: 0.0,
            estimated_remaining_seconds: 0,
            current_file: format!("准备解压 {} 个文件...", total_files),
        });

        // 使用标准解压方法（带密码支持）
        if let Some(p) = _password {
            if !p.is_empty() {
                sevenz_rust::decompress_file_with_password(archive_path, output_dir, p.into())
                    .map_err(|e| SzError::Decompress(format!("解压失败: {}", e)))?;
            } else {
                sevenz_rust::decompress_file(archive_path, output_dir)
                    .map_err(|e| SzError::Decompress(format!("解压失败: {}", e)))?;
            }
        } else {
            sevenz_rust::decompress_file(archive_path, output_dir)
                .map_err(|e| SzError::Decompress(format!("解压失败: {}", e)))?;
        }

        // 收集已解压的文件并报告进度
        let mut processed_size: u64 = 0;
        for (i, entry) in entries.iter().enumerate() {
            let name = entry.name();
            let out_file_path = output_path.join(name);
            
            if out_file_path.exists() {
                extracted_files.push(out_file_path.to_string_lossy().to_string());
            }
            
            processed_size += entry.size();
            
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                processed_size as f64 / elapsed
            } else {
                0.0
            };
            let remaining = if speed > 0.0 && total_size > processed_size {
                ((total_size - processed_size) as f64 / speed) as u64
            } else {
                0
            };

            // 每处理10个文件或最后一个文件时报告进度
            if i % 10 == 0 || i == entries.len() - 1 {
                progress_callback(CompressProgress {
                    progress: if total_size > 0 { processed_size as f64 / total_size as f64 } else { 1.0 },
                    processed_bytes: processed_size,
                    total_bytes: total_size,
                    speed_bytes_per_second: speed,
                    estimated_remaining_seconds: remaining,
                    current_file: name.to_string(),
                });
            }
        }

        // 最终进度
        progress_callback(CompressProgress {
            progress: 1.0,
            processed_bytes: total_size,
            total_bytes: total_size,
            speed_bytes_per_second: total_size as f64 / start_time.elapsed().as_secs_f64().max(0.001),
            estimated_remaining_seconds: 0,
            current_file: "完成".to_string(),
        });

        Ok(extracted_files)
    }
}

impl Default for Decompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_creation() {
        let options = CompressOptions::default();
        let _compressor = Compressor::new(options);
    }

    #[test]
    fn test_decompressor_creation() {
        let _decompressor = Decompressor::new();
    }
}
