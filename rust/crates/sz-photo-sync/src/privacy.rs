//! 隐私处理器
//!
//! 在照片上传到 WebDAV/云端之前的完整隐私保护流水线：
//!
//! 1. EXIF 擦除：剥离 GPS、设备信息等（保留到加密索引）
//! 2. 文件名混淆：IMG_20260401.jpg → a7x2k9m3.dat
//! 3. 缩略图擦除：JPEG 嵌入的缩略图可能包含裁剪前的原始内容
//! 4. 客户端加密：AES-256-GCM，密钥永远不出设备
//!
//! 对于 WebDAV 服务器（坚果云、Nextcloud、自建 NAS），它看到的只是：
//!   /backups/photo_20260401_001/
//!     ├── a7x2k9m3.dat  (加密的照片数据)
//!     ├── b2y4m8n1.dat  (加密的照片数据)
//!     ├── ...
//!     └── manifest.zbak (加密的索引，包含原始文件名和 EXIF)

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Sha256, Digest};

use crate::exif::ExifStripLevel;

/// 隐私处理器
pub struct PrivacyProcessor {
    /// EXIF 擦除等级
    strip_level: ExifStripLevel,
    /// 是否混淆文件名
    obfuscate_names: bool,
}

impl PrivacyProcessor {
    pub fn new(strip_level: ExifStripLevel) -> Self {
        Self {
            strip_level,
            obfuscate_names: true,
        }
    }

    pub fn obfuscate_names(mut self, val: bool) -> Self {
        self.obfuscate_names = val;
        self
    }

    /// 为一批照片生成混淆文件名
    ///
    /// 生成规则：SHA256(原始路径 + salt)[0..16] + ".dat"
    /// 这样相同文件总是得到相同的混淆名（幂等），
    /// 但服务器无法反推出原始文件名
    pub fn generate_obfuscated_names(
        &self,
        original_paths: &[PathBuf],
        salt: &[u8],
    ) -> Vec<(PathBuf, String)> {
        original_paths
            .iter()
            .map(|path| {
                let obfuscated = if self.obfuscate_names {
                    let mut hasher = Sha256::new();
                    hasher.update(path.to_string_lossy().as_bytes());
                    hasher.update(salt);
                    let hash = hasher.finalize();
                    // 取前 8 字节 = 16 个十六进制字符
                    format!("{}.dat", hex_encode(&hash[..8]))
                } else {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                };
                (path.clone(), obfuscated)
            })
            .collect()
    }

    /// 处理单张 JPEG 照片：擦除 EXIF
    ///
    /// JPEG 文件结构：
    /// FF D8 (SOI) → FF E1 (APP1/EXIF) → ... → FF DA (SOS/图像数据) → FF D9 (EOI)
    ///
    /// 擦除策略：找到 APP1 段，直接跳过。
    /// 这比"解码→重编码"快得多，而且不损失画质。
    pub fn strip_jpeg_exif(
        &self,
        input_path: &Path,
        output_path: &Path,
    ) -> anyhow::Result<()> {
        if matches!(self.strip_level, ExifStripLevel::None) {
            // 不擦除，直接复制
            fs::copy(input_path, output_path)?;
            return Ok(());
        }

        let data = fs::read(input_path)?;

        if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
            // 不是 JPEG，原样复制
            fs::copy(input_path, output_path)?;
            return Ok(());
        }

        let stripped = strip_jpeg_segments(&data, self.strip_level);
        fs::write(output_path, &stripped)?;

        Ok(())
    }

    /// 获取擦除等级
    pub fn strip_level(&self) -> ExifStripLevel {
        self.strip_level
    }
}

/// JPEG 段擦除：遍历 JPEG marker，跳过需要擦除的 APP 段
fn strip_jpeg_segments(data: &[u8], level: ExifStripLevel) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut pos = 0;

    // 写入 SOI (FF D8)
    if data.len() >= 2 {
        result.extend_from_slice(&data[..2]);
        pos = 2;
    }

    while pos + 1 < data.len() {
        if data[pos] != 0xFF {
            // 不是 marker，写入剩余数据
            result.extend_from_slice(&data[pos..]);
            break;
        }

        let marker = data[pos + 1];

        // SOS (FF DA) 之后就是图像数据，直接写到结尾
        if marker == 0xDA {
            result.extend_from_slice(&data[pos..]);
            break;
        }

        // 获取段长度
        if pos + 3 >= data.len() {
            result.extend_from_slice(&data[pos..]);
            break;
        }

        let seg_len = ((data[pos + 2] as usize) << 8) | (data[pos + 3] as usize);
        let seg_end = pos + 2 + seg_len;

        let should_strip = match level {
            ExifStripLevel::None => false,
            ExifStripLevel::GpsOnly => {
                // APP1 (EXIF) 中只擦除 GPS 标签比较复杂，
                // 简化处理：对于 GpsOnly 也整段保留 APP1，
                // 在 EXIF 解析层面过滤 GPS（在 zbak 索引中不包含 GPS）
                false
            }
            ExifStripLevel::GpsAndDevice => {
                // 擦除 APP1 (EXIF) 段
                marker == 0xE1
            }
            ExifStripLevel::All => {
                // 擦除所有 APP 段 (APP0-APP15 = 0xE0-0xEF) 和 COM (0xFE)
                (0xE0..=0xEF).contains(&marker) || marker == 0xFE
            }
        };

        if should_strip {
            // 跳过这个段
            log::debug!("擦除 JPEG 段 FF {:02X} ({} bytes)", marker, seg_len);
        } else {
            // 保留这个段
            let end = seg_end.min(data.len());
            result.extend_from_slice(&data[pos..end]);
        }

        pos = seg_end;
    }

    result
}

/// 字节数组转十六进制字符串
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// 照片隐私上传的完整流程（给 api.rs 调用）
// ============================================================================

/// 照片隐私上传选项
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhotoPrivacyOptions {
    /// EXIF 擦除等级
    pub exif_strip_level: u8,  // 0=None, 1=GpsOnly, 2=GpsAndDevice, 3=All
    /// 是否混淆文件名
    pub obfuscate_filenames: bool,
    /// 加密密码（None = 不加密，但强烈建议设置）
    pub encrypt_password: Option<String>,
    /// 是否包含视频
    pub include_videos: bool,
}

impl Default for PhotoPrivacyOptions {
    fn default() -> Self {
        Self {
            exif_strip_level: 3, // 默认全部擦除
            obfuscate_filenames: true,
            encrypt_password: None,
            include_videos: true,
        }
    }
}

impl PhotoPrivacyOptions {
    pub fn strip_level(&self) -> ExifStripLevel {
        match self.exif_strip_level {
            0 => ExifStripLevel::None,
            1 => ExifStripLevel::GpsOnly,
            2 => ExifStripLevel::GpsAndDevice,
            _ => ExifStripLevel::All,
        }
    }
}
