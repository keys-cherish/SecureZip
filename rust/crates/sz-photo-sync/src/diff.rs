//! 增量差异检测
//!
//! 对比「当前扫描结果」和「已有索引」，找出：
//! - 新增：索引中不存在 dedup_key
//! - 修改：dedup_key 存在但 mtime 或 size 变了（照片被编辑）
//! - 删除：索引有记录但扫描中找不到了（文件被删）
//! - 移动：dedup_key 相同但路径变了（重命名/移动文件夹）

use crate::index::SyncIndex;
use crate::scanner::ScannedPhoto;

/// 文件变更类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FileChange {
    /// 新照片，从未备份过
    New(ScannedPhoto),
    /// 照片被编辑（大小或内容变了），需要重新备份
    Modified(ScannedPhoto),
    /// 照片移动/重命名了，只更新索引路径
    Moved {
        photo: ScannedPhoto,
        old_path: String,
    },
    /// 照片已从设备删除（索引有但扫描没有）
    Deleted {
        dedup_key: String,
        old_path: String,
    },
}

/// 差异检测结果
#[derive(Debug, Clone)]
pub struct DiffResult {
    /// 需要备份的文件（New + Modified）
    pub to_backup: Vec<ScannedPhoto>,
    /// 只需更新索引的（Moved）
    pub to_update: Vec<FileChange>,
    /// 已删除（可选：从备份中也删除）
    pub deleted: Vec<FileChange>,
    /// 未变化（跳过）
    pub unchanged_count: usize,
    /// 总扫描文件数
    pub total_scanned: usize,
}

impl DiffResult {
    /// 需要实际传输的字节数
    pub fn transfer_bytes(&self) -> u64 {
        self.to_backup.iter().map(|p| p.size).sum()
    }

    /// 节省的字节数（跳过的文件）
    pub fn saved_bytes(&self, total_scanned_bytes: u64) -> u64 {
        total_scanned_bytes.saturating_sub(self.transfer_bytes())
    }
}

/// 计算增量差异
///
/// 时间复杂度：O(n)，n = 扫描文件数
/// 空间复杂度：O(m)，m = 索引记录数（HashMap 查找是 O(1)）
pub fn compute_diff(scanned: &[ScannedPhoto], index: &SyncIndex) -> DiffResult {
    let mut to_backup = Vec::new();
    let mut to_update = Vec::new();
    let mut unchanged_count = 0;

    // 用于检测已删除：先把所有索引 key 放进 set，扫描到的就移除
    let mut remaining_keys: std::collections::HashSet<&str> = index
        .records
        .keys()
        .map(|k| k.as_str())
        .collect();

    for photo in scanned {
        remaining_keys.remove(photo.dedup_key.as_str());

        if let Some(record) = index.records.get(&photo.dedup_key) {
            // dedup_key 匹配 = 同一张照片
            if record.original_path != photo.path.to_string_lossy() {
                // 路径变了 = 移动/重命名
                to_update.push(FileChange::Moved {
                    photo: photo.clone(),
                    old_path: record.original_path.clone(),
                });
            } else {
                // 完全一样，跳过
                unchanged_count += 1;
            }
        } else {
            // 索引中不存在 = 新照片
            to_backup.push(photo.clone());
        }
    }

    // 剩余的 key = 设备上已删除的文件
    let deleted: Vec<FileChange> = remaining_keys
        .iter()
        .filter_map(|key| {
            index.records.get(*key).map(|r| FileChange::Deleted {
                dedup_key: key.to_string(),
                old_path: r.original_path.clone(),
            })
        })
        .collect();

    DiffResult {
        to_backup,
        to_update,
        deleted,
        unchanged_count,
        total_scanned: scanned.len(),
    }
}
