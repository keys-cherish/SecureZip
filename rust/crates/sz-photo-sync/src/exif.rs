//! EXIF 元数据提取与擦除
//!
//! 隐私威胁分析：
//! - GPS 坐标：暴露你家/公司/常去地点
//! - 设备型号：暴露手机品牌（社工攻击）
//! - 拍摄时间：暴露作息规律
//! - 缩略图：可能保留编辑前的原始内容（裁剪掉的人/信息）
//!
//! 策略：备份时提取需要的元数据保存到加密索引，
//! 然后从上传的文件中完全剥离 EXIF

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use chrono::{DateTime, NaiveDateTime, Utc};

/// 从照片中提取的 EXIF 信息（用于索引和去重）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExifInfo {
    /// 拍摄时间（EXIF DateTimeOriginal）
    pub date_taken: Option<DateTime<Utc>>,
    /// GPS 纬度
    pub latitude: Option<f64>,
    /// GPS 经度
    pub longitude: Option<f64>,
    /// 设备制造商
    pub camera_make: Option<String>,
    /// 设备型号
    pub camera_model: Option<String>,
    /// 图片宽度
    pub width: Option<u32>,
    /// 图片高度
    pub height: Option<u32>,
    /// 图片方向（1-8，EXIF Orientation tag）
    pub orientation: Option<u16>,
}

impl ExifInfo {
    /// 从文件读取 EXIF 信息
    pub fn from_file(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mut reader = BufReader::new(file);
        let exif_reader = exif::Reader::new();
        let exif_data = exif_reader.read_from_container(&mut reader).ok()?;

        let date_taken = exif_data
            .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
            .and_then(|f| parse_exif_datetime(&f.display_value().to_string()));

        let latitude = get_gps_coord(&exif_data, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef);
        let longitude = get_gps_coord(&exif_data, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef);

        let camera_make = exif_data
            .get_field(exif::Tag::Make, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string().trim().to_string());
        let camera_model = exif_data
            .get_field(exif::Tag::Model, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string().trim().to_string());

        let width = get_u32_field(&exif_data, exif::Tag::PixelXDimension)
            .or_else(|| get_u32_field(&exif_data, exif::Tag::ImageWidth));
        let height = get_u32_field(&exif_data, exif::Tag::PixelYDimension)
            .or_else(|| get_u32_field(&exif_data, exif::Tag::ImageLength));

        let orientation = exif_data
            .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0).map(|v| v as u16));

        Some(ExifInfo {
            date_taken,
            latitude,
            longitude,
            camera_make,
            camera_model,
            width,
            height,
            orientation,
        })
    }

    /// 用于去重的指纹：拍摄时间 + 文件大小
    /// 同一张照片即使重命名/移动，指纹不变
    pub fn dedup_key(&self, file_size: u64) -> String {
        let time_str = self
            .date_taken
            .map(|t| t.format("%Y%m%d%H%M%S").to_string())
            .unwrap_or_default();
        format!("{}_{}", time_str, file_size)
    }
}

/// 解析 EXIF 日期字符串 "2024:01:15 14:30:22"
fn parse_exif_datetime(s: &str) -> Option<DateTime<Utc>> {
    // EXIF 格式通常是 "2024:01:15 14:30:22" 或带引号
    let cleaned = s.trim_matches('"').trim();
    let naive = NaiveDateTime::parse_from_str(cleaned, "%Y:%m:%d %H:%M:%S").ok()?;
    Some(DateTime::from_naive_utc_and_offset(naive, Utc))
}

/// 提取 GPS 坐标
fn get_gps_coord(
    exif_data: &exif::Exif,
    coord_tag: exif::Tag,
    ref_tag: exif::Tag,
) -> Option<f64> {
    let coord_field = exif_data.get_field(coord_tag, exif::In::PRIMARY)?;
    let ref_field = exif_data.get_field(ref_tag, exif::In::PRIMARY)?;

    // GPS 坐标格式：[degrees, minutes, seconds] 三个 Rational 值
    if let exif::Value::Rational(ref rationals) = coord_field.value {
        if rationals.len() >= 3 {
            let degrees = rationals[0].to_f64();
            let minutes = rationals[1].to_f64();
            let seconds = rationals[2].to_f64();
            let mut coord = degrees + minutes / 60.0 + seconds / 3600.0;

            // S/W 为负值
            let ref_str = ref_field.display_value().to_string();
            if ref_str.contains('S') || ref_str.contains('W') {
                coord = -coord;
            }
            return Some(coord);
        }
    }
    None
}

/// 读取 u32 类型的 EXIF 字段
fn get_u32_field(exif_data: &exif::Exif, tag: exif::Tag) -> Option<u32> {
    exif_data
        .get_field(tag, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
}

/// EXIF 擦除等级
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ExifStripLevel {
    /// 不擦除（保留所有 EXIF）
    None,
    /// 只擦除 GPS（最常用：保留拍摄时间和设备信息）
    GpsOnly,
    /// 擦除 GPS + 设备信息（保留拍摄时间和尺寸）
    GpsAndDevice,
    /// 完全擦除（上传到不信任的服务器时用）
    All,
}
