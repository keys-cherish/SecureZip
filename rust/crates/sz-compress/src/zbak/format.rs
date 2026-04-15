//! .zbak 二进制格式定义
//!
//! 96 字节固定文件头 + 逐文件数据块 + 文件索引区 + 可选恢复记录区
//!
//! 格式版本: 1
//! 算法标识: 0x01 = Zstd + AES-256-GCM + Argon2id + Reed-Solomon

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use sz_core::{SzError, SzResult};

// ============================================================================
// 常量
// ============================================================================

/// 魔数 "ZBAK"
pub const MAGIC: &[u8; 4] = b"ZBAK";

/// 当前格式版本
pub const VERSION: u8 = 1;

/// 文件头固定大小
pub const HEADER_SIZE: usize = 96;

/// 算法标识: Zstd + AES-256-GCM + Argon2id + Reed-Solomon
pub const ALGORITHM_ZSTD_AES_ARGON2_RS: u32 = 0x01;

/// 每文件数据块头部大小: 8(compressed_size) + 8(original_size) + 12(nonce) = 28
pub const FILE_BLOCK_HEADER_SIZE: usize = 28;

/// AES-256-GCM tag 大小
pub const GCM_TAG_SIZE: usize = 16;

/// Nonce 大小
pub const NONCE_SIZE: usize = 12;

/// Salt 大小
pub const SALT_SIZE: usize = 16;

// ============================================================================
// 标志位
// ============================================================================

/// 内容加密
pub const FLAG_ENCRYPTED: u16       = 0b0000_0001;
/// 文件名加密
pub const FLAG_FILENAME_ENCRYPTED: u16 = 0b0000_0010;
/// 有恢复记录
pub const FLAG_HAS_RECOVERY: u16    = 0b0000_0100;
/// WebDAV 分块模式
pub const FLAG_CHUNKED: u16         = 0b0000_1000;

// ============================================================================
// 文件头
// ============================================================================

/// .zbak 文件头 (96 字节固定)
#[derive(Debug, Clone)]
pub struct ZbakHeader {
    /// 格式版本
    pub version: u8,
    /// 标志位
    pub flags: u16,
    /// 压缩级别 (1-22)
    pub compression_level: u8,
    /// Salt (Argon2id 用, 16 字节)
    pub salt: [u8; SALT_SIZE],
    /// 文件条目数
    pub entry_count: u32,
    /// 索引区偏移
    pub index_offset: u64,
    /// 索引区大小
    pub index_size: u32,
    /// 恢复区偏移 (0=无)
    pub recovery_offset: u64,
    /// 恢复区大小 (0=无)
    pub recovery_size: u32,
    /// 分块大小 (0=不分块, 单位字节)
    pub chunk_size: u32,
    /// 密码验证块 (16 字节 GCM tag)
    pub verify_tag: [u8; GCM_TAG_SIZE],
    /// 验证块 Nonce (12 字节)
    pub verify_nonce: [u8; NONCE_SIZE],
    /// 算法标识
    pub algorithm: u32,
}

impl ZbakHeader {
    /// 创建新的空文件头
    pub fn new(compression_level: u8) -> Self {
        Self {
            version: VERSION,
            flags: 0,
            compression_level,
            salt: [0u8; SALT_SIZE],
            entry_count: 0,
            index_offset: 0,
            index_size: 0,
            recovery_offset: 0,
            recovery_size: 0,
            chunk_size: 0,
            verify_tag: [0u8; GCM_TAG_SIZE],
            verify_nonce: [0u8; NONCE_SIZE],
            algorithm: ALGORITHM_ZSTD_AES_ARGON2_RS,
        }
    }

    /// 是否加密
    pub fn is_encrypted(&self) -> bool {
        self.flags & FLAG_ENCRYPTED != 0
    }

    /// 是否文件名加密
    pub fn is_filename_encrypted(&self) -> bool {
        self.flags & FLAG_FILENAME_ENCRYPTED != 0
    }

    /// 是否有恢复记录
    pub fn has_recovery(&self) -> bool {
        self.flags & FLAG_HAS_RECOVERY != 0
    }

    /// 设置加密标志
    pub fn set_encrypted(&mut self, val: bool) {
        if val { self.flags |= FLAG_ENCRYPTED; } else { self.flags &= !FLAG_ENCRYPTED; }
    }

    /// 设置文件名加密标志
    pub fn set_filename_encrypted(&mut self, val: bool) {
        if val { self.flags |= FLAG_FILENAME_ENCRYPTED; } else { self.flags &= !FLAG_FILENAME_ENCRYPTED; }
    }

    /// 设置恢复记录标志
    pub fn set_has_recovery(&mut self, val: bool) {
        if val { self.flags |= FLAG_HAS_RECOVERY; } else { self.flags &= !FLAG_HAS_RECOVERY; }
    }

    /// 设置分块模式标志
    pub fn set_chunked(&mut self, val: bool) {
        if val { self.flags |= FLAG_CHUNKED; } else { self.flags &= !FLAG_CHUNKED; }
    }

    /// 序列化到 writer (恰好 96 字节)
    pub fn write_to<W: Write>(&self, w: &mut W) -> SzResult<()> {
        w.write_all(MAGIC)?;                          // [0-3]   魔数
        w.write_u8(self.version)?;                     // [4]     版本
        w.write_u16::<LittleEndian>(self.flags)?;      // [5-6]   标志位
        w.write_u8(self.compression_level)?;           // [7]     压缩级别
        w.write_all(&self.salt)?;                      // [8-23]  Salt
        w.write_u32::<LittleEndian>(self.entry_count)?;// [24-27] 文件条目数
        w.write_u64::<LittleEndian>(self.index_offset)?;// [28-35] 索引区偏移
        w.write_u32::<LittleEndian>(self.index_size)?; // [36-39] 索引区大小
        w.write_u64::<LittleEndian>(self.recovery_offset)?;// [40-47] 恢复区偏移
        w.write_u32::<LittleEndian>(self.recovery_size)?;  // [48-51] 恢复区大小
        w.write_u32::<LittleEndian>(self.chunk_size)?; // [52-55] 分块大小
        w.write_all(&self.verify_tag)?;                // [56-71] 密码验证块
        w.write_all(&self.verify_nonce)?;              // [72-83] 验证块 Nonce
        w.write_u32::<LittleEndian>(self.algorithm)?;  // [84-87] 算法标识
        w.write_all(&[0u8; 8])?;                       // [88-95] 保留
        Ok(())
    }

    /// 从 reader 反序列化 (读取恰好 96 字节)
    pub fn read_from<R: Read>(r: &mut R) -> SzResult<Self> {
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(SzError::Decompress("不是有效的 .zbak 文件（魔数不匹配）".into()));
        }

        let version = r.read_u8()?;
        if version > VERSION {
            return Err(SzError::Decompress(format!(
                "不支持的 .zbak 版本: {}，当前最高支持: {}", version, VERSION
            )));
        }

        let flags = r.read_u16::<LittleEndian>()?;
        let compression_level = r.read_u8()?;

        let mut salt = [0u8; SALT_SIZE];
        r.read_exact(&mut salt)?;

        let entry_count = r.read_u32::<LittleEndian>()?;
        let index_offset = r.read_u64::<LittleEndian>()?;
        let index_size = r.read_u32::<LittleEndian>()?;
        let recovery_offset = r.read_u64::<LittleEndian>()?;
        let recovery_size = r.read_u32::<LittleEndian>()?;
        let chunk_size = r.read_u32::<LittleEndian>()?;

        let mut verify_tag = [0u8; GCM_TAG_SIZE];
        r.read_exact(&mut verify_tag)?;

        let mut verify_nonce = [0u8; NONCE_SIZE];
        r.read_exact(&mut verify_nonce)?;

        let algorithm = r.read_u32::<LittleEndian>()?;

        // 跳过保留字段
        let mut _reserved = [0u8; 8];
        r.read_exact(&mut _reserved)?;

        Ok(Self {
            version,
            flags,
            compression_level,
            salt,
            entry_count,
            index_offset,
            index_size,
            recovery_offset,
            recovery_size,
            chunk_size,
            verify_tag,
            verify_nonce,
            algorithm,
        })
    }
}

// ============================================================================
// 文件索引条目
// ============================================================================

/// 文件索引条目
#[derive(Debug, Clone)]
pub struct ZbakIndexEntry {
    /// 文件路径 (相对路径, UTF-8)
    pub path: String,
    /// 原始大小
    pub original_size: u64,
    /// 压缩+加密后大小 (含 GCM tag)
    pub compressed_size: u64,
    /// 数据块在文件中的偏移
    pub block_offset: u64,
    /// CRC32 校验值 (原始数据)
    pub crc32: u32,
    /// 修改时间 (Unix timestamp)
    pub mtime: i64,
    /// 权限位 (Unix permissions)
    pub permissions: u32,
    /// 是否为目录
    pub is_directory: bool,
}

impl ZbakIndexEntry {
    /// 序列化到 writer
    pub fn write_to<W: Write>(&self, w: &mut W) -> SzResult<()> {
        let path_bytes = self.path.as_bytes();
        w.write_u16::<LittleEndian>(path_bytes.len() as u16)?;
        w.write_all(path_bytes)?;
        w.write_u64::<LittleEndian>(self.original_size)?;
        w.write_u64::<LittleEndian>(self.compressed_size)?;
        w.write_u64::<LittleEndian>(self.block_offset)?;
        w.write_u32::<LittleEndian>(self.crc32)?;
        w.write_i64::<LittleEndian>(self.mtime)?;
        w.write_u32::<LittleEndian>(self.permissions)?;
        w.write_u8(if self.is_directory { 1 } else { 0 })?;
        Ok(())
    }

    /// 从 reader 反序列化
    pub fn read_from<R: Read>(r: &mut R) -> SzResult<Self> {
        let path_len = r.read_u16::<LittleEndian>()? as usize;
        let mut path_bytes = vec![0u8; path_len];
        r.read_exact(&mut path_bytes)?;
        let path = String::from_utf8(path_bytes)
            .map_err(|e| SzError::Decompress(format!("无效的文件路径编码: {}", e)))?;

        let original_size = r.read_u64::<LittleEndian>()?;
        let compressed_size = r.read_u64::<LittleEndian>()?;
        let block_offset = r.read_u64::<LittleEndian>()?;
        let crc32 = r.read_u32::<LittleEndian>()?;
        let mtime = r.read_i64::<LittleEndian>()?;
        let permissions = r.read_u32::<LittleEndian>()?;
        let is_directory = r.read_u8()? != 0;

        Ok(Self {
            path,
            original_size,
            compressed_size,
            block_offset,
            crc32,
            mtime,
            permissions,
            is_directory,
        })
    }
}

/// 序列化索引区（多条目）
pub fn write_index<W: Write>(entries: &[ZbakIndexEntry], w: &mut W) -> SzResult<()> {
    for entry in entries {
        entry.write_to(w)?;
    }
    Ok(())
}

/// 反序列化索引区
pub fn read_index<R: Read>(r: &mut R, count: u32) -> SzResult<Vec<ZbakIndexEntry>> {
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        entries.push(ZbakIndexEntry::read_from(r)?);
    }
    Ok(entries)
}

/// 压缩结果
#[derive(Debug, Clone)]
pub struct ZbakResult {
    pub original_size: u64,
    pub compressed_size: u64,
    pub file_count: u32,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_header_roundtrip() {
        let mut header = ZbakHeader::new(6);
        header.entry_count = 42;
        header.index_offset = 1024;
        header.index_size = 512;
        header.salt = [1u8; 16];
        header.set_encrypted(true);
        header.set_filename_encrypted(true);
        header.set_has_recovery(true);

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), HEADER_SIZE);

        let mut cursor = Cursor::new(&buf);
        let decoded = ZbakHeader::read_from(&mut cursor).unwrap();

        assert_eq!(decoded.version, VERSION);
        assert_eq!(decoded.compression_level, 6);
        assert_eq!(decoded.entry_count, 42);
        assert_eq!(decoded.index_offset, 1024);
        assert_eq!(decoded.index_size, 512);
        assert_eq!(decoded.salt, [1u8; 16]);
        assert!(decoded.is_encrypted());
        assert!(decoded.is_filename_encrypted());
        assert!(decoded.has_recovery());
    }

    #[test]
    fn test_index_entry_roundtrip() {
        let entry = ZbakIndexEntry {
            path: "测试文件/hello.txt".to_string(),
            original_size: 12345,
            compressed_size: 6789,
            block_offset: 96,
            crc32: 0xDEADBEEF,
            mtime: 1700000000,
            permissions: 0o644,
            is_directory: false,
        };

        let mut buf = Vec::new();
        entry.write_to(&mut buf).unwrap();

        let mut cursor = Cursor::new(&buf);
        let decoded = ZbakIndexEntry::read_from(&mut cursor).unwrap();

        assert_eq!(decoded.path, entry.path);
        assert_eq!(decoded.original_size, entry.original_size);
        assert_eq!(decoded.compressed_size, entry.compressed_size);
        assert_eq!(decoded.block_offset, entry.block_offset);
        assert_eq!(decoded.crc32, entry.crc32);
        assert_eq!(decoded.mtime, entry.mtime);
        assert_eq!(decoded.permissions, entry.permissions);
        assert_eq!(decoded.is_directory, entry.is_directory);
    }

    #[test]
    fn test_invalid_magic() {
        let data = b"NOPE_not_zbak____padding_to_96_bytes____________________________aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut cursor = Cursor::new(&data[..]);
        assert!(ZbakHeader::read_from(&mut cursor).is_err());
    }

    #[test]
    fn test_index_multi_roundtrip() {
        let entries = vec![
            ZbakIndexEntry {
                path: "dir/".to_string(),
                original_size: 0,
                compressed_size: 0,
                block_offset: 96,
                crc32: 0,
                mtime: 1700000000,
                permissions: 0o755,
                is_directory: true,
            },
            ZbakIndexEntry {
                path: "dir/file.bin".to_string(),
                original_size: 999999,
                compressed_size: 500000,
                block_offset: 124,
                crc32: 0xCAFEBABE,
                mtime: 1700000001,
                permissions: 0o644,
                is_directory: false,
            },
        ];

        let mut buf = Vec::new();
        write_index(&entries, &mut buf).unwrap();

        let mut cursor = Cursor::new(&buf);
        let decoded = read_index(&mut cursor, 2).unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].path, "dir/");
        assert!(decoded[0].is_directory);
        assert_eq!(decoded[1].path, "dir/file.bin");
        assert_eq!(decoded[1].crc32, 0xCAFEBABE);
    }
}
