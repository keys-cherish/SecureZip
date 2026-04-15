//! 同步索引：跟踪哪些照片已备份
//!
//! 存储为 JSON 文件（人类可读，方便调试）
//! 位置：应用数据目录/photo_sync_index.json
//!
//! 每条记录：(dedup_key, original_path, size, mtime, backup_time, backup_id)
//! dedup_key 是主键，用于检测"同一张照片换了路径/文件名"的场景

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 单张照片的备份记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoRecord {
    /// 去重指纹（exif_date + file_size）
    pub dedup_key: String,
    /// 最后已知的文件路径
    pub original_path: String,
    /// 文件大小
    pub size: u64,
    /// 文件修改时间
    pub mtime: DateTime<Utc>,
    /// 备份时间
    pub backup_time: DateTime<Utc>,
    /// 所属备份 ID（WebDAV 上的 backup_id 或本地 .zbak 路径）
    pub backup_id: String,
    /// 文件在备份包中的加密路径（用于恢复时定位）
    pub encrypted_name: Option<String>,
}

/// 同步索引
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncIndex {
    /// 格式版本
    pub version: u32,
    /// 最后同步时间
    pub last_sync: Option<DateTime<Utc>>,
    /// 已备份照片：dedup_key → PhotoRecord
    pub records: HashMap<String, PhotoRecord>,
    /// 统计信息
    pub stats: SyncStats,
}

/// 同步统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncStats {
    /// 总备份照片数
    pub total_photos: u64,
    /// 总备份字节数
    pub total_bytes: u64,
    /// 累计节省的字节数（跳过的重复文件）
    pub saved_bytes: u64,
}

impl SyncIndex {
    /// 创建空索引
    pub fn new() -> Self {
        Self {
            version: 1,
            last_sync: None,
            records: HashMap::new(),
            stats: SyncStats::default(),
        }
    }

    /// 从文件加载索引
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    /// 保存索引到文件
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    /// 检查某张照片是否已备份（通过 dedup_key）
    pub fn is_backed_up(&self, dedup_key: &str) -> bool {
        self.records.contains_key(dedup_key)
    }

    /// 添加备份记录
    pub fn add_record(&mut self, record: PhotoRecord) {
        self.stats.total_photos += 1;
        self.stats.total_bytes += record.size;
        self.records.insert(record.dedup_key.clone(), record);
    }

    /// 标记同步完成
    pub fn mark_synced(&mut self) {
        self.last_sync = Some(Utc::now());
    }

    /// 获取已备份照片总数
    pub fn backed_up_count(&self) -> usize {
        self.records.len()
    }
}

impl Default for SyncIndex {
    fn default() -> Self {
        Self::new()
    }
}
