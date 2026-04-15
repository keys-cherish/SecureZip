//! 照片目录扫描器
//!
//! 递归扫描指定目录，找出所有照片文件，
//! 提取 (path, size, mtime, exif) 四元组

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::exif::ExifInfo;

/// 扫描到的照片文件信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScannedPhoto {
    /// 文件绝对路径
    pub path: PathBuf,
    /// 文件大小（字节）
    pub size: u64,
    /// 文件修改时间
    pub mtime: DateTime<Utc>,
    /// EXIF 信息（照片才有，视频没有）
    pub exif: Option<ExifInfo>,
    /// 去重指纹（exif.date_taken + size，用于检测重命名/移动）
    pub dedup_key: String,
}

/// 支持的照片/视频扩展名
const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "webp", "avif",
    "raw", "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "3gp",
];

/// 照片扫描器
pub struct PhotoScanner {
    /// 是否包含视频
    include_videos: bool,
    /// 最小文件大小（过滤缩略图，默认 10KB）
    min_size: u64,
}

impl PhotoScanner {
    pub fn new() -> Self {
        Self {
            include_videos: true,
            min_size: 10 * 1024, // 10KB
        }
    }

    pub fn include_videos(mut self, val: bool) -> Self {
        self.include_videos = val;
        self
    }

    pub fn min_size(mut self, bytes: u64) -> Self {
        self.min_size = bytes;
        self
    }

    /// 扫描目录，返回所有照片/视频文件列表
    ///
    /// 使用 rayon 并行提取 EXIF，大量照片时显著加速
    pub fn scan(&self, directories: &[String]) -> Vec<ScannedPhoto> {
        let mut all_paths: Vec<PathBuf> = Vec::new();

        for dir in directories {
            let dir_path = Path::new(dir);
            if !dir_path.exists() {
                log::warn!("目录不存在，跳过: {}", dir);
                continue;
            }

            for entry in WalkDir::new(dir_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                if self.is_supported_file(path) {
                    all_paths.push(path.to_path_buf());
                }
            }
        }

        log::info!("找到 {} 个候选文件，开始提取元数据...", all_paths.len());

        // 并行提取 EXIF + 文件元数据
        all_paths
            .par_iter()
            .filter_map(|path| self.scan_single_file(path))
            .collect()
    }

    /// 扫描单个文件
    fn scan_single_file(&self, path: &Path) -> Option<ScannedPhoto> {
        let metadata = fs::metadata(path).ok()?;
        let size = metadata.len();

        if size < self.min_size {
            return None; // 过滤缩略图和垃圾文件
        }

        let mtime = metadata
            .modified()
            .ok()
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mtime: DateTime<Utc> = mtime.into();

        // 照片才提取 EXIF，视频跳过
        let exif = if is_photo_extension(path) {
            ExifInfo::from_file(path)
        } else {
            None
        };

        let dedup_key = if let Some(ref exif_info) = exif {
            exif_info.dedup_key(size)
        } else {
            // 没有 EXIF 的文件用 size + mtime 做粗略指纹
            format!("{}_{}", mtime.format("%Y%m%d%H%M%S"), size)
        };

        Some(ScannedPhoto {
            path: path.to_path_buf(),
            size,
            mtime,
            exif,
            dedup_key,
        })
    }

    /// 检查是否是支持的文件类型
    fn is_supported_file(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match ext {
            Some(ref e) if PHOTO_EXTENSIONS.contains(&e.as_str()) => true,
            Some(ref e) if self.include_videos && VIDEO_EXTENSIONS.contains(&e.as_str()) => true,
            _ => false,
        }
    }
}

impl Default for PhotoScanner {
    fn default() -> Self {
        Self::new()
    }
}

fn is_photo_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| PHOTO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
