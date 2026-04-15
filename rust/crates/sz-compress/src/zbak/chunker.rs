//! .zbak 分块逻辑 + BackupManifest
//!
//! 用于 WebDAV 流式上传场景：将 .zbak 数据流切分为固定大小的 chunk

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

use sz_core::{SzError, SzResult};

/// 默认分块大小: 50MB
pub const DEFAULT_CHUNK_SIZE: usize = 50 * 1024 * 1024;

/// 单个 chunk 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// chunk 文件名 (e.g. "data_000000.chunk")
    pub filename: String,
    /// chunk 大小 (字节)
    pub size: u64,
    /// SHA-256 哈希 (hex)
    pub sha256: String,
    /// 是否为恢复块
    pub is_recovery: bool,
}

/// 备份清单 (manifest.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// 备份 ID (时间戳 + 随机后缀)
    pub backup_id: String,
    /// 创建时间 (ISO 8601)
    pub created_at: String,
    /// 源路径列表
    pub source_paths: Vec<String>,
    /// 总文件数
    pub total_files: u32,
    /// 原始大小 (字节)
    pub original_size: u64,
    /// 压缩后大小 (字节)
    pub compressed_size: u64,
    /// chunk 数量
    pub chunk_count: u32,
    /// 每个 chunk 大小 (字节)
    pub chunk_size: u32,
    /// 恢复块数量
    pub recovery_count: u32,
    /// KDF 算法
    pub kdf_algorithm: String,
    /// KDF salt (hex)
    pub kdf_salt_hex: String,
    /// 是否加密
    pub encrypted: bool,
    /// 所有 chunk 信息
    pub chunks: Vec<ChunkInfo>,
}

impl BackupManifest {
    /// 创建新的空清单
    pub fn new(backup_id: String, source_paths: Vec<String>) -> Self {
        Self {
            backup_id,
            created_at: chrono::Utc::now().to_rfc3339(),
            source_paths,
            total_files: 0,
            original_size: 0,
            compressed_size: 0,
            chunk_count: 0,
            chunk_size: DEFAULT_CHUNK_SIZE as u32,
            recovery_count: 0,
            kdf_algorithm: "argon2id".to_string(),
            kdf_salt_hex: String::new(),
            encrypted: false,
            chunks: Vec::new(),
        }
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> SzResult<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SzError::Serialization(e.to_string()))
    }

    /// 从 JSON 反序列化
    pub fn from_json(json: &str) -> SzResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| SzError::Serialization(e.to_string()))
    }

    /// 生成备份 ID
    pub fn generate_backup_id() -> String {
        let now = chrono::Utc::now();
        let random: u32 = rand::random();
        format!("{}_{:08x}", now.format("%Y-%m-%d_%H%M%S"), random)
    }
}

/// 将数据切分为固定大小的 chunks
pub struct Chunker {
    chunk_size: usize,
}

impl Chunker {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size: if chunk_size == 0 { DEFAULT_CHUNK_SIZE } else { chunk_size },
        }
    }

    /// 将数据分块, 返回每个 chunk 的 (数据, ChunkInfo)
    pub fn split_data(&self, data: &[u8]) -> Vec<(Vec<u8>, ChunkInfo)> {
        let mut chunks = Vec::new();
        let mut offset = 0;
        let mut index = 0u32;

        while offset < data.len() {
            let end = (offset + self.chunk_size).min(data.len());
            let chunk_data = data[offset..end].to_vec();

            let sha256 = Self::sha256_hex(&chunk_data);
            let filename = format!("data_{:06}.chunk", index);

            chunks.push((
                chunk_data.clone(),
                ChunkInfo {
                    filename,
                    size: chunk_data.len() as u64,
                    sha256,
                    is_recovery: false,
                },
            ));

            offset = end;
            index += 1;
        }

        chunks
    }

    /// 为恢复数据创建 chunk info
    pub fn create_recovery_chunk(index: u32, data: &[u8]) -> ChunkInfo {
        ChunkInfo {
            filename: format!("recovery_{:03}.chunk", index),
            size: data.len() as u64,
            sha256: Self::sha256_hex(data),
            is_recovery: true,
        }
    }

    /// 计算 SHA-256 hex
    pub fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex_encode(&result)
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new(DEFAULT_CHUNK_SIZE)
    }
}

/// 简单 hex 编码
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_split() {
        let data = vec![42u8; 150]; // 150 bytes
        let chunker = Chunker::new(64);
        let chunks = chunker.split_data(&data);

        assert_eq!(chunks.len(), 3); // 64 + 64 + 22
        assert_eq!(chunks[0].0.len(), 64);
        assert_eq!(chunks[1].0.len(), 64);
        assert_eq!(chunks[2].0.len(), 22);
        assert_eq!(chunks[0].1.filename, "data_000000.chunk");
        assert_eq!(chunks[2].1.filename, "data_000002.chunk");
    }

    #[test]
    fn test_manifest_json_roundtrip() {
        let mut manifest = BackupManifest::new(
            "2024-01-15_143022_a3f8b2".to_string(),
            vec!["/data/photos".to_string()],
        );
        manifest.total_files = 100;
        manifest.original_size = 1024 * 1024 * 500;
        manifest.chunks.push(ChunkInfo {
            filename: "data_000000.chunk".to_string(),
            size: DEFAULT_CHUNK_SIZE as u64,
            sha256: "abc123".to_string(),
            is_recovery: false,
        });

        let json = manifest.to_json().unwrap();
        let decoded = BackupManifest::from_json(&json).unwrap();

        assert_eq!(decoded.backup_id, manifest.backup_id);
        assert_eq!(decoded.total_files, 100);
        assert_eq!(decoded.chunks.len(), 1);
    }

    #[test]
    fn test_sha256_hex() {
        let hash = Chunker::sha256_hex(b"hello");
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_backup_id_format() {
        let id = BackupManifest::generate_backup_id();
        assert!(id.contains('_'));
        assert!(id.len() > 20);
    }
}
