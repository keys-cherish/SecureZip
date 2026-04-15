//! WebDAV 流式上传器
//!
//! 真正的流式备份：边压缩边分块边上传，内存占用 ≈ chunk_size (50MB)
//! 流程:
//!   1. 收集文件 → 准备加密上下文
//!   2. 写 zbak 头占位 → 逐文件压缩+加密 → 追加到缓冲
//!   3. 缓冲满 chunk_size → 立即上传 → 释放
//!   4. 所有文件完成 → 写索引 → flush 剩余缓冲
//!   5. 回写 chunk_0 头部 → 上传 chunk_0
//!   6. (可选) RS 恢复块 → 上传
//!   7. 上传 manifest.json (完成标记)

use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher as Crc32Hasher;
use sha2::{Sha256, Digest};

use sz_core::{SzError, SzResult};
use sz_webdav::WebDavClient;

use super::format::*;
use super::crypto;
use super::recovery::RecoveryGenerator;
use super::chunker::{BackupManifest, ChunkInfo, DEFAULT_CHUNK_SIZE};
use super::writer::collect_files;

/// 流式上传器
pub struct StreamingUploader {
    webdav: WebDavClient,
    chunk_size: usize,
    recovery_ratio: f32,
    enable_recovery: bool,
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl StreamingUploader {
    pub fn new(webdav: WebDavClient) -> Self {
        Self {
            webdav,
            chunk_size: DEFAULT_CHUNK_SIZE,
            recovery_ratio: 0.10,
            enable_recovery: false,
            cancel_flag: None,
        }
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = if size == 0 { DEFAULT_CHUNK_SIZE } else { size };
        self
    }

    pub fn with_recovery(mut self, enable: bool, ratio: f32) -> Self {
        self.enable_recovery = enable;
        self.recovery_ratio = ratio.clamp(0.01, 0.50);
        self
    }

    pub fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel_flag = Some(flag);
        self
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag.as_ref().map_or(false, |f| f.load(Ordering::Relaxed))
    }

    /// 流式备份：边压缩边上传
    ///
    /// 内存占用: 无 RS ≈ chunk_size, 有 RS ≈ 总压缩大小
    pub fn backup<F>(
        &self,
        input_paths: &[String],
        password: Option<&str>,
        compression_level: i32,
        encrypt_filenames: bool,
        mut progress_callback: F,
    ) -> SzResult<BackupManifest>
    where
        F: FnMut(u64, u64, &str),
    {
        let level = compression_level.clamp(1, 22);

        // 1. 收集文件
        let file_list = collect_files(input_paths)?;
        if file_list.is_empty() {
            return Err(SzError::InvalidArgument("没有找到要备份的文件".into()));
        }

        let total_size: u64 = file_list.iter().map(|f| f.size).sum();
        let mut processed: u64 = 0;
        progress_callback(0, total_size, "准备备份...");

        // 2. 准备加密
        let encryption = if let Some(pwd) = password {
            let salt = crypto::generate_salt();
            let master_key = crypto::derive_master_key(pwd, &salt)?;
            Some(EncryptionCtx { salt, master_key })
        } else {
            None
        };

        // 3. 创建远程目录
        let backup_id = BackupManifest::generate_backup_id();
        let remote_dir = format!("backups/{}", backup_id);
        self.webdav.create_directory("backups")?;
        self.webdav.create_directory(&remote_dir)?;

        // 4. 构建 zbak 字节流 → 分块上传
        let mut buffer = Vec::with_capacity(self.chunk_size + 1024 * 1024);
        let mut chunk_index: u32 = 0;
        let mut uploaded_chunks: Vec<ChunkInfo> = Vec::new();
        // chunk_0 延迟上传（需要回写头部）
        let mut chunk_0_data: Option<Vec<u8>> = None;
        // RS 需要所有 chunk 数据
        let mut all_chunk_data: Vec<Vec<u8>> = Vec::new();

        // 写 zbak 头占位 (96 字节)
        let mut header = ZbakHeader::new(level as u8);
        header.entry_count = file_list.len() as u32;

        if let Some(ref enc) = encryption {
            header.set_encrypted(true);
            header.salt = enc.salt;
            if encrypt_filenames {
                header.set_filename_encrypted(true);
            }
            let verify_key = crypto::derive_verify_key(&enc.master_key);
            let (nonce, tag) = crypto::create_verify_block(&verify_key)?;
            header.verify_nonce = nonce;
            header.verify_tag = tag;
        }
        if self.enable_recovery {
            header.set_has_recovery(true);
        }
        header.set_chunked(true);
        header.chunk_size = self.chunk_size as u32;

        // 写占位头到 buffer
        header.write_to(&mut buffer)?;

        // 5. 逐文件压缩 → 追加到 buffer → 满则上传
        let mut index_entries = Vec::with_capacity(file_list.len());

        for (file_idx, file_info) in file_list.iter().enumerate() {
            if self.is_cancelled() {
                return Err(SzError::Cancelled);
            }

            progress_callback(processed, total_size, &file_info.rel_path);

            let block_offset = buffer.len() as u64
                + chunk_index as u64 * self.chunk_size as u64;

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

            // 读取 + CRC32 + Zstd 压缩
            let file_data = fs::read(&file_info.abs_path)
                .map_err(|e| SzError::Compress(format!("读取失败 {}: {}", file_info.rel_path, e)))?;

            let mut crc_hasher = Crc32Hasher::new();
            crc_hasher.update(&file_data);
            let crc32_val = crc_hasher.finalize();

            let compressed = zstd::encode_all(&file_data[..], level)
                .map_err(|e| SzError::Compress(format!("Zstd 压缩失败: {}", e)))?;

            // 可选加密
            let (block_data, nonce_bytes) = if let Some(ref enc) = encryption {
                let file_key = crypto::derive_file_key(&enc.master_key, file_idx as u32);
                let (ct, nonce) = crypto::encrypt_block(&file_key, &compressed)?;
                (ct, nonce)
            } else {
                (compressed, [0u8; NONCE_SIZE])
            };

            let original_size = file_data.len() as u64;
            let compressed_size = block_data.len() as u64;

            // 写数据块到 buffer
            buffer.write_u64::<LittleEndian>(compressed_size)?;
            buffer.write_u64::<LittleEndian>(original_size)?;
            buffer.write_all(&nonce_bytes)?;
            buffer.write_all(&block_data)?;

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

            processed += original_size;
            progress_callback(processed, total_size, &file_info.rel_path);

            // buffer 满了 → 切块上传
            while buffer.len() >= self.chunk_size {
                let chunk_data: Vec<u8> = buffer.drain(..self.chunk_size).collect();
                self.upload_chunk(
                    &remote_dir,
                    chunk_index,
                    &chunk_data,
                    &mut chunk_0_data,
                    &mut uploaded_chunks,
                    &mut all_chunk_data,
                )?;
                chunk_index += 1;
            }
        }

        // 6. 写索引区到 buffer
        let index_global_offset = buffer.len() as u64
            + chunk_index as u64 * self.chunk_size as u64;

        let mut index_buf = Vec::new();
        write_index(&index_entries, &mut index_buf)?;

        let index_data = if let Some(ref enc) = encryption {
            if encrypt_filenames {
                let index_key = crypto::derive_index_key(&enc.master_key);
                crypto::encrypt_index(&index_key, &index_buf)?
            } else {
                index_buf
            }
        } else {
            index_buf
        };

        let index_size = index_data.len() as u32;
        buffer.write_all(&index_data)?;

        // flush 剩余 buffer
        while buffer.len() >= self.chunk_size {
            let chunk_data: Vec<u8> = buffer.drain(..self.chunk_size).collect();
            self.upload_chunk(
                &remote_dir,
                chunk_index,
                &chunk_data,
                &mut chunk_0_data,
                &mut uploaded_chunks,
                &mut all_chunk_data,
            )?;
            chunk_index += 1;
        }
        // 最后不足 chunk_size 的尾部
        if !buffer.is_empty() {
            let chunk_data = std::mem::take(&mut buffer);
            self.upload_chunk(
                &remote_dir,
                chunk_index,
                &chunk_data,
                &mut chunk_0_data,
                &mut uploaded_chunks,
                &mut all_chunk_data,
            )?;
            chunk_index += 1;
        }

        // 7. 回写 chunk_0 头部并上传
        if let Some(ref mut c0) = chunk_0_data {
            header.index_offset = index_global_offset;
            header.index_size = index_size;
            // recovery 偏移在 RS 生成后填写

            let mut header_bytes = Vec::new();
            header.write_to(&mut header_bytes)?;
            c0[..HEADER_SIZE].copy_from_slice(&header_bytes);
        }

        progress_callback(processed, total_size, "上传恢复记录...");

        // 8. 可选 RS 恢复块
        let mut recovery_chunk_count: u32 = 0;
        if self.enable_recovery && !all_chunk_data.is_empty() {
            // 拼接所有 chunk 数据用于 RS
            let total_data: Vec<u8> = all_chunk_data.iter().flatten().copied().collect();
            drop(all_chunk_data); // 释放

            let recovery_data = RecoveryGenerator::generate(&total_data, self.recovery_ratio)?;
            let rec_bytes = recovery_data.serialize()?;

            // 将恢复数据分块上传
            let mut rec_offset = 0;
            let mut rec_idx: u32 = 0;
            while rec_offset < rec_bytes.len() {
                let end = (rec_offset + self.chunk_size).min(rec_bytes.len());
                let rec_chunk = &rec_bytes[rec_offset..end];
                let filename = format!("recovery_{:03}.chunk", rec_idx);
                let remote_path = format!("{}/{}", remote_dir, filename);

                self.webdav.upload_data(&remote_path, rec_chunk)?;

                uploaded_chunks.push(ChunkInfo {
                    filename,
                    size: rec_chunk.len() as u64,
                    sha256: sha256_hex(rec_chunk),
                    is_recovery: true,
                });

                rec_offset = end;
                rec_idx += 1;
            }
            recovery_chunk_count = rec_idx;
        } else {
            drop(all_chunk_data);
        }

        // 上传 chunk_0 (最后上传，确保头部正确)
        if let Some(c0) = chunk_0_data {
            let remote_path = format!("{}/data_{:06}.chunk", remote_dir, 0);
            self.webdav.upload_data(&remote_path, &c0)?;
            // chunk_0 info 已在 uploaded_chunks[0]，更新 sha256
            if let Some(first) = uploaded_chunks.iter_mut().find(|c| c.filename == "data_000000.chunk") {
                first.sha256 = sha256_hex(&c0);
                first.size = c0.len() as u64;
            }
        }

        progress_callback(processed, total_size, "上传清单...");

        // 9. 生成并上传 manifest.json
        let compressed_size: u64 = uploaded_chunks.iter()
            .filter(|c| !c.is_recovery)
            .map(|c| c.size)
            .sum();

        let mut manifest = BackupManifest::new(
            backup_id.clone(),
            input_paths.iter().map(|s| s.to_string()).collect(),
        );
        manifest.total_files = file_list.len() as u32;
        manifest.original_size = total_size;
        manifest.compressed_size = compressed_size;
        manifest.chunk_count = chunk_index;
        manifest.chunk_size = self.chunk_size as u32;
        manifest.recovery_count = recovery_chunk_count;
        manifest.encrypted = encryption.is_some();
        if let Some(ref enc) = encryption {
            manifest.kdf_salt_hex = hex_encode(&enc.salt);
        }
        manifest.chunks = uploaded_chunks;

        let manifest_json = manifest.to_json()?;
        let manifest_path = format!("{}/manifest.json", remote_dir);
        self.webdav.upload_data(&manifest_path, manifest_json.as_bytes())?;

        progress_callback(total_size, total_size, "备份完成");
        Ok(manifest)
    }

    /// 上传一个 chunk，chunk_0 延迟上传
    fn upload_chunk(
        &self,
        remote_dir: &str,
        chunk_index: u32,
        chunk_data: &[u8],
        chunk_0_data: &mut Option<Vec<u8>>,
        uploaded_chunks: &mut Vec<ChunkInfo>,
        all_chunk_data: &mut Vec<Vec<u8>>,
    ) -> SzResult<()> {
        let filename = format!("data_{:06}.chunk", chunk_index);
        let sha256 = sha256_hex(chunk_data);

        let info = ChunkInfo {
            filename: filename.clone(),
            size: chunk_data.len() as u64,
            sha256,
            is_recovery: false,
        };

        if chunk_index == 0 {
            // chunk_0 延迟上传（需要回写头部）
            *chunk_0_data = Some(chunk_data.to_vec());
        } else {
            // 断点续传: HEAD 检查是否已上传
            let remote_path = format!("{}/{}", remote_dir, filename);
            match self.webdav.head(&remote_path) {
                Ok(Some(size)) if size == chunk_data.len() as u64 => {
                    // 已上传，跳过
                }
                _ => {
                    self.webdav.upload_data(&remote_path, chunk_data)?;
                }
            }
        }

        uploaded_chunks.push(info);

        // RS 需要保留数据
        if self.enable_recovery {
            all_chunk_data.push(chunk_data.to_vec());
        }

        Ok(())
    }

    /// 从 WebDAV 恢复备份
    pub fn restore<F>(
        &self,
        backup_id: &str,
        output_dir: &str,
        password: Option<&str>,
        mut progress_callback: F,
    ) -> SzResult<Vec<String>>
    where
        F: FnMut(u64, u64, &str),
    {
        progress_callback(0, 1, "下载清单...");

        // 1. 下载 manifest.json
        let manifest_path = format!("backups/{}/manifest.json", backup_id);
        let manifest_bytes = self.webdav.download_data(&manifest_path)?;
        let manifest_str = String::from_utf8(manifest_bytes)
            .map_err(|e| SzError::Decompress(format!("manifest 解析失败: {}", e)))?;
        let manifest = BackupManifest::from_json(&manifest_str)?;

        let data_chunks: Vec<&ChunkInfo> = manifest.chunks.iter()
            .filter(|c| !c.is_recovery)
            .collect();
        let total_download: u64 = data_chunks.iter().map(|c| c.size).sum();
        let mut downloaded: u64 = 0;

        progress_callback(0, total_download, "下载数据块...");

        // 2. 下载所有数据 chunk → 拼接为完整 .zbak
        let mut zbak_data = Vec::with_capacity(total_download as usize);
        for chunk_info in &data_chunks {
            if self.is_cancelled() {
                return Err(SzError::Cancelled);
            }

            let remote_path = format!("backups/{}/{}", backup_id, chunk_info.filename);
            let chunk_bytes = self.webdav.download_data(&remote_path)?;

            // SHA-256 校验
            let actual_hash = sha256_hex(&chunk_bytes);
            if actual_hash != chunk_info.sha256 {
                return Err(SzError::Decompress(format!(
                    "chunk {} SHA-256 校验失败", chunk_info.filename
                )));
            }

            zbak_data.extend_from_slice(&chunk_bytes);
            downloaded += chunk_info.size;
            progress_callback(downloaded, total_download, &chunk_info.filename);
        }

        progress_callback(downloaded, total_download, "解压中...");

        // 3. 写入临时文件 → 用 ZbakReader 解压
        let temp_path = format!("{}/._restore_temp.zbak", output_dir);
        fs::create_dir_all(output_dir)
            .map_err(|e| SzError::Decompress(format!("创建输出目录失败: {}", e)))?;
        fs::write(&temp_path, &zbak_data)
            .map_err(|e| SzError::Decompress(format!("写入临时文件失败: {}", e)))?;
        drop(zbak_data);

        let reader = super::reader::ZbakReader::new();
        let result = reader.decompress(
            &temp_path,
            output_dir,
            password,
            |cur, tot, name| progress_callback(cur, tot, name),
        );

        // 清理临时文件
        let _ = fs::remove_file(&temp_path);

        result
    }

    /// 列出 WebDAV 上的所有备份
    pub fn list_backups(&self) -> SzResult<Vec<BackupManifest>> {
        let entries = self.webdav.list_directory("backups")?;
        let mut manifests = Vec::new();

        for entry in entries {
            if entry.is_directory {
                let manifest_path = format!("backups/{}/manifest.json", entry.name);
                match self.webdav.download_data(&manifest_path) {
                    Ok(data) => {
                        if let Ok(json) = String::from_utf8(data) {
                            if let Ok(manifest) = BackupManifest::from_json(&json) {
                                manifests.push(manifest);
                            }
                        }
                    }
                    Err(_) => continue, // 不完整的备份（无 manifest）
                }
            }
        }

        // 按创建时间倒序
        manifests.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(manifests)
    }
}

/// 加密上下文
struct EncryptionCtx {
    salt: [u8; 16],
    master_key: [u8; 32],
}

/// SHA-256 hex
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

/// hex 编码
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
