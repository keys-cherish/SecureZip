//! Reed-Solomon 恢复记录
//!
//! 使用 reed-solomon-erasure 库为 .zbak 数据生成纠删码恢复分片

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use reed_solomon_erasure::galois_8::ReedSolomon;
use std::io::{Read, Write, Cursor};

use sz_core::{SzError, SzResult};

/// 恢复记录生成器
pub struct RecoveryGenerator;

/// 恢复记录数据
#[derive(Debug)]
pub struct RecoveryData {
    /// 数据分片数
    pub data_shards: u32,
    /// 恢复分片数
    pub recovery_shards: u32,
    /// 每个分片大小
    pub shard_size: u32,
    /// 恢复分片数据 (仅恢复分片, 不含原始数据)
    pub recovery_shards_data: Vec<Vec<u8>>,
}

impl RecoveryGenerator {
    /// 从数据生成恢复记录
    ///
    /// ratio: 恢复冗余比例 (0.05 = 5%, 0.10 = 10%, 0.20 = 20%)
    pub fn generate(data: &[u8], ratio: f32) -> SzResult<RecoveryData> {
        if data.is_empty() {
            return Err(SzError::InvalidArgument("空数据无法生成恢复记录".into()));
        }

        // 计算分片参数
        // 目标: 将数据分为 N 个数据分片 + M 个恢复分片
        // M = ceil(N * ratio)
        let shard_size = Self::calc_shard_size(data.len());
        let data_shards = (data.len() + shard_size - 1) / shard_size;
        let recovery_shards = ((data_shards as f32 * ratio).ceil() as usize).max(1);

        // 确保合法 (reed-solomon-erasure 要求至少 1 个数据分片和 1 个恢复分片)
        if data_shards == 0 {
            return Err(SzError::InvalidArgument("数据太小".into()));
        }

        let rs = ReedSolomon::new(data_shards, recovery_shards)
            .map_err(|e| SzError::Compress(format!("Reed-Solomon 初始化失败: {:?}", e)))?;

        // 将数据分片 (补零对齐)
        let mut shards: Vec<Vec<u8>> = Vec::with_capacity(data_shards + recovery_shards);
        for i in 0..data_shards {
            let start = i * shard_size;
            let end = ((i + 1) * shard_size).min(data.len());
            let mut shard = vec![0u8; shard_size];
            shard[..end - start].copy_from_slice(&data[start..end]);
            shards.push(shard);
        }

        // 添加空恢复分片
        for _ in 0..recovery_shards {
            shards.push(vec![0u8; shard_size]);
        }

        // 生成恢复分片
        rs.encode(&mut shards)
            .map_err(|e| SzError::Compress(format!("Reed-Solomon 编码失败: {:?}", e)))?;

        // 只保存恢复分片（数据分片已经在文件里了）
        let recovery_shards_data: Vec<Vec<u8>> = shards[data_shards..].to_vec();

        Ok(RecoveryData {
            data_shards: data_shards as u32,
            recovery_shards: recovery_shards as u32,
            shard_size: shard_size as u32,
            recovery_shards_data,
        })
    }

    /// 尝试恢复损坏的数据
    ///
    /// shards: 数据分片(可能有 None 表示损坏) + 恢复分片
    pub fn recover(
        data_shards_count: usize,
        recovery_shards_count: usize,
        shard_size: usize,
        shards: &mut Vec<Option<Vec<u8>>>,
    ) -> SzResult<Vec<u8>> {
        let rs = ReedSolomon::new(data_shards_count, recovery_shards_count)
            .map_err(|e| SzError::Decompress(format!("Reed-Solomon 初始化失败: {:?}", e)))?;

        // 确保所有分片大小一致
        for shard in shards.iter_mut() {
            if let Some(ref mut s) = shard {
                s.resize(shard_size, 0);
            }
        }

        // 尝试重建
        rs.reconstruct(shards)
            .map_err(|e| SzError::Decompress(format!("数据恢复失败: {:?}", e)))?;

        // 拼接数据分片
        let mut result = Vec::with_capacity(data_shards_count * shard_size);
        for shard in shards[..data_shards_count].iter() {
            match shard {
                Some(data) => result.extend_from_slice(data),
                None => return Err(SzError::Decompress("恢复后仍有缺失分片".into())),
            }
        }

        Ok(result)
    }

    /// 计算合适的分片大小 (目标: 64KB ~ 1MB 之间)
    fn calc_shard_size(data_len: usize) -> usize {
        const MIN_SHARD: usize = 64 * 1024;   // 64KB
        const MAX_SHARD: usize = 1024 * 1024;  // 1MB
        const TARGET_SHARDS: usize = 256;       // 目标分片数

        let ideal = data_len / TARGET_SHARDS;
        ideal.clamp(MIN_SHARD, MAX_SHARD)
    }
}

impl RecoveryData {
    /// 序列化恢复数据
    pub fn serialize(&self) -> SzResult<Vec<u8>> {
        let mut buf = Vec::new();
        buf.write_u32::<LittleEndian>(self.data_shards)?;
        buf.write_u32::<LittleEndian>(self.recovery_shards)?;
        buf.write_u32::<LittleEndian>(self.shard_size)?;

        for shard in &self.recovery_shards_data {
            buf.write_all(shard)?;
        }

        Ok(buf)
    }

    /// 反序列化恢复数据
    pub fn deserialize(data: &[u8]) -> SzResult<Self> {
        if data.len() < 12 {
            return Err(SzError::Decompress("恢复数据太短".into()));
        }

        let mut cursor = Cursor::new(data);
        let data_shards = cursor.read_u32::<LittleEndian>()?;
        let recovery_shards = cursor.read_u32::<LittleEndian>()?;
        let shard_size = cursor.read_u32::<LittleEndian>()?;

        let mut recovery_shards_data = Vec::with_capacity(recovery_shards as usize);
        for _ in 0..recovery_shards {
            let mut shard = vec![0u8; shard_size as usize];
            cursor.read_exact(&mut shard)
                .map_err(|_| SzError::Decompress("恢复数据不完整".into()))?;
            recovery_shards_data.push(shard);
        }

        Ok(Self {
            data_shards,
            recovery_shards,
            shard_size,
            recovery_shards_data,
        })
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_serialize_roundtrip() {
        let data = vec![42u8; 256 * 1024]; // 256KB
        let recovery = RecoveryGenerator::generate(&data, 0.10).unwrap();

        assert!(recovery.data_shards > 0);
        assert!(recovery.recovery_shards > 0);

        let serialized = recovery.serialize().unwrap();
        let deserialized = RecoveryData::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.data_shards, recovery.data_shards);
        assert_eq!(deserialized.recovery_shards, recovery.recovery_shards);
        assert_eq!(deserialized.shard_size, recovery.shard_size);
        assert_eq!(deserialized.recovery_shards_data.len(), recovery.recovery_shards_data.len());
    }

    #[test]
    fn test_recover_with_missing_shards() {
        let data = vec![99u8; 512 * 1024]; // 512KB
        let shard_size = 64 * 1024; // 64KB
        let data_shard_count = (data.len() + shard_size - 1) / shard_size;
        let recovery_shard_count = ((data_shard_count as f32 * 0.20).ceil() as usize).max(1);

        let rs = ReedSolomon::new(data_shard_count, recovery_shard_count).unwrap();

        // 创建分片
        let mut shards: Vec<Vec<u8>> = Vec::new();
        for i in 0..data_shard_count {
            let start = i * shard_size;
            let end = ((i + 1) * shard_size).min(data.len());
            let mut shard = vec![0u8; shard_size];
            shard[..end - start].copy_from_slice(&data[start..end]);
            shards.push(shard);
        }
        for _ in 0..recovery_shard_count {
            shards.push(vec![0u8; shard_size]);
        }
        rs.encode(&mut shards).unwrap();

        // 模拟损坏: 丢弃一个数据分片
        let mut option_shards: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        option_shards[0] = None; // 丢弃第一个数据分片

        // 恢复
        let recovered = RecoveryGenerator::recover(
            data_shard_count,
            recovery_shard_count,
            shard_size,
            &mut option_shards,
        ).unwrap();

        // 验证前 512KB 数据正确
        assert_eq!(&recovered[..data.len()], &data[..]);
    }

    #[test]
    fn test_recovery_ratios() {
        let data = vec![1u8; 1024 * 1024]; // 1MB
        for &ratio in &[0.05f32, 0.10, 0.20] {
            let recovery = RecoveryGenerator::generate(&data, ratio).unwrap();
            assert!(recovery.recovery_shards > 0);
            // 恢复分片数应大约等于 data_shards * ratio
            let expected = (recovery.data_shards as f32 * ratio).ceil() as u32;
            assert_eq!(recovery.recovery_shards, expected.max(1));
        }
    }
}
