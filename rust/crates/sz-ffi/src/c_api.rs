//! C ABI FFI 导出
//! 
//! 提供纯 C 接口，可被 dart:ffi 直接调用
//!
//! 主要压缩方案：Tar + ZSTD + AES-256-GCM
//! 解压：智能检测格式，自动选择合适的解压方法

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use sz_core::CompressOptions;

/// 全局进度状态
static PROGRESS_CURRENT: AtomicU64 = AtomicU64::new(0);
static PROGRESS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// 全局取消标志
static CANCELLED: AtomicBool = AtomicBool::new(false);

/// 获取取消标志的 Arc 包装（用于传递给压缩器）
fn get_cancel_flag() -> Arc<AtomicBool> {
    // 创建一个新的 Arc，但它会检查全局 CANCELLED 标志
    // 这里使用一个技巧：创建一个代理 AtomicBool
    Arc::new(AtomicBool::new(false))
}

/// 检查是否已取消（内部使用）
fn check_cancelled() -> bool {
    CANCELLED.load(Ordering::Relaxed)
}

/// 压缩结果
#[repr(C)]
pub struct CCompressResult {
    pub success: i32,
    pub original_size: u64,
    pub compressed_size: u64,
    pub error_message: *mut c_char,
}

impl Default for CCompressResult {
    fn default() -> Self {
        Self {
            success: 0,
            original_size: 0,
            compressed_size: 0,
            error_message: ptr::null_mut(),
        }
    }
}

/// 解压结果
#[repr(C)]
pub struct CDecompressResult {
    pub success: i32,
    pub file_count: i32,
    pub error_message: *mut c_char,
}

/// 进度信息
#[repr(C)]
pub struct CProgress {
    pub current: u64,
    pub total: u64,
}

// ============================================================================
// 核心压缩函数
// ============================================================================

/// 压缩文件（无密码）- 使用 Tar + ZSTD（主推方案）
/// 
/// # Safety
/// 所有指针必须有效
#[no_mangle]
pub unsafe extern "C" fn sz_compress(
    input_paths: *const c_char,  // 用 | 分隔的路径
    output_path: *const c_char,
    compression_level: i32,
) -> CCompressResult {
    let mut result = CCompressResult::default();
    
    // 解析输入路径
    let input_str = match CStr::from_ptr(input_paths).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输入路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出路径编码");
            return result;
        }
    };
    
    let paths: Vec<String> = input_str.split('|').map(String::from).collect();
    
    // 使用 Tar + ZSTD 压缩（主推方案，高效可靠）
    let compressor = sz_compress::TarZstdCompressor::new(compression_level);
    
    match compressor.compress(&paths, output_str, None, |current, total, _| {
        PROGRESS_CURRENT.store(current, Ordering::SeqCst);
        PROGRESS_TOTAL.store(total, Ordering::SeqCst);
    }) {
        Ok(r) => {
            result.success = 1;
            result.original_size = r.original_size;
            result.compressed_size = r.compressed_size;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("压缩错误：{}", e));
        }
    }
    
    result
}

/// 压缩文件（带密码）- 使用 Tar + ZSTD + AES-256-GCM
#[no_mangle]
pub unsafe extern "C" fn sz_compress_encrypted(
    input_paths: *const c_char,
    output_path: *const c_char,
    password: *const c_char,
    compression_level: i32,
) -> CCompressResult {
    let mut result = CCompressResult::default();
    
    let input_str = match CStr::from_ptr(input_paths).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输入路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出路径编码");
            return result;
        }
    };
    
    let password_str = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的密码编码");
            return result;
        }
    };
    
    let paths: Vec<String> = input_str.split('|').map(String::from).collect();
    
    // 使用 Tar + ZSTD + AES-256-GCM 加密压缩
    let compressor = sz_compress::TarZstdCompressor::new(compression_level);
    
    match compressor.compress(&paths, output_str, Some(password_str), |current, total, _| {
        PROGRESS_CURRENT.store(current, Ordering::SeqCst);
        PROGRESS_TOTAL.store(total, Ordering::SeqCst);
    }) {
        Ok(r) => {
            result.success = 1;
            result.original_size = r.original_size;
            result.compressed_size = r.compressed_size;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("加密压缩错误：{}", e));
        }
    }
    
    result
}

/// 解压文件（无密码）- 使用智能解压（自动检测格式）
#[no_mangle]
pub unsafe extern "C" fn sz_decompress(
    archive_path: *const c_char,
    output_dir: *const c_char,
) -> CDecompressResult {
    let mut result = CDecompressResult {
        success: 0,
        file_count: 0,
        error_message: ptr::null_mut(),
    };
    
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的压缩包路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_dir).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出目录编码");
            return result;
        }
    };
    
    // 使用智能解压（自动检测格式）
    match sz_compress::SmartDecompressor::decompress(archive_str, output_str, None, |current, total, _| {
        PROGRESS_CURRENT.store(current, Ordering::SeqCst);
        PROGRESS_TOTAL.store(total, Ordering::SeqCst);
    }) {
        Ok(files) => {
            result.success = 1;
            result.file_count = files.len() as i32;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("解压错误：{}", e));
        }
    }
    
    result
}

/// 解压文件（带密码）- 使用智能解压（自动检测格式）
#[no_mangle]
pub unsafe extern "C" fn sz_decompress_encrypted(
    archive_path: *const c_char,
    output_dir: *const c_char,
    password: *const c_char,
) -> CDecompressResult {
    let mut result = CDecompressResult {
        success: 0,
        file_count: 0,
        error_message: ptr::null_mut(),
    };
    
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的压缩包路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_dir).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出目录编码");
            return result;
        }
    };
    
    let password_str = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的密码编码");
            return result;
        }
    };
    
    // 使用智能解压（自动检测格式）
    match sz_compress::SmartDecompressor::decompress(archive_str, output_str, Some(password_str), |current, total, _| {
        PROGRESS_CURRENT.store(current, Ordering::SeqCst);
        PROGRESS_TOTAL.store(total, Ordering::SeqCst);
    }) {
        Ok(files) => {
            result.success = 1;
            result.file_count = files.len() as i32;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("解压错误：{}", e));
        }
    }
    
    result
}

/// 验证密码
#[no_mangle]
pub unsafe extern "C" fn sz_verify_password(
    archive_path: *const c_char,
    password: *const c_char,
) -> i32 {
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let password_str = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let compressor = sz_compress::EncryptedCompressor::default();
    
    match compressor.verify_password(archive_str, password_str) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

/// 获取当前进度
#[no_mangle]
pub extern "C" fn sz_get_progress() -> CProgress {
    CProgress {
        current: PROGRESS_CURRENT.load(Ordering::SeqCst),
        total: PROGRESS_TOTAL.load(Ordering::SeqCst),
    }
}

/// 重置进度
#[no_mangle]
pub extern "C" fn sz_reset_progress() {
    PROGRESS_CURRENT.store(0, Ordering::SeqCst);
    PROGRESS_TOTAL.store(0, Ordering::SeqCst);
    CANCELLED.store(false, Ordering::SeqCst);
}

/// 请求取消当前操作
#[no_mangle]
pub extern "C" fn sz_request_cancel() {
    CANCELLED.store(true, Ordering::SeqCst);
}

/// 检查是否已请求取消
#[no_mangle]
pub extern "C" fn sz_is_cancelled() -> i32 {
    if CANCELLED.load(Ordering::SeqCst) { 1 } else { 0 }
}

/// 释放错误消息
#[no_mangle]
pub unsafe extern "C" fn sz_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ============================================================================
// 7z 标准压缩函数 (sevenz-rust)
// ============================================================================

/// 7z 标准压缩（无密码）- 可被所有7z软件打开
#[no_mangle]
pub unsafe extern "C" fn sz_compress_7z(
    input_paths: *const c_char,
    output_path: *const c_char,
    compression_level: i32,
) -> CCompressResult {
    let mut result = CCompressResult::default();
    
    let input_str = match CStr::from_ptr(input_paths).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输入路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出路径编码");
            return result;
        }
    };
    
    let paths: Vec<String> = input_str.split('|').map(String::from).collect();
    
    // 使用 sevenz-rust 的 Compressor (LZMA2 压缩)
    let options = CompressOptions {
        compression_level: compression_level.clamp(1, 9) as u8,
        ..Default::default()
    };
    let compressor = sz_compress::Compressor::new(options);
    
    match compressor.compress(&paths, output_str, |progress| {
        PROGRESS_CURRENT.store(progress.processed_bytes, Ordering::SeqCst);
        PROGRESS_TOTAL.store(progress.total_bytes, Ordering::SeqCst);
    }) {
        Ok(r) => {
            result.success = 1;
            result.original_size = r.original_size;
            result.compressed_size = r.compressed_size;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("{}", e));
        }
    }
    
    result
}

/// 7z 标准压缩（带密码）- AES-256 加密，可被所有7z软件打开
#[no_mangle]
pub unsafe extern "C" fn sz_compress_7z_encrypted(
    input_paths: *const c_char,
    output_path: *const c_char,
    password: *const c_char,
    compression_level: i32,
) -> CCompressResult {
    let mut result = CCompressResult::default();
    
    let input_str = match CStr::from_ptr(input_paths).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输入路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出路径编码");
            return result;
        }
    };
    
    let password_str = if password.is_null() {
        result.error_message = string_to_c("密码不能为空");
        return result;
    } else {
        match CStr::from_ptr(password).to_str() {
            Ok(s) if !s.is_empty() => s,
            _ => {
                result.error_message = string_to_c("无效的密码编码");
                return result;
            }
        }
    };
    
    let paths: Vec<String> = input_str.split('|').map(String::from).collect();
    
    // 使用 sevenz-rust 的 Compressor 带加密 (LZMA2 + AES-256)
    let options = CompressOptions {
        compression_level: compression_level.clamp(1, 9) as u8,
        ..Default::default()
    };
    let compressor = sz_compress::Compressor::new(options);
    
    match compressor.compress_encrypted(&paths, output_str, password_str, |progress| {
        PROGRESS_CURRENT.store(progress.processed_bytes, Ordering::SeqCst);
        PROGRESS_TOTAL.store(progress.total_bytes, Ordering::SeqCst);
    }) {
        Ok(r) => {
            result.success = 1;
            result.original_size = r.original_size;
            result.compressed_size = r.compressed_size;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("{}", e));
        }
    }
    
    result
}

/// 7z 解压（标准格式）
#[no_mangle]
pub unsafe extern "C" fn sz_decompress_7z(
    archive_path: *const c_char,
    output_dir: *const c_char,
    password: *const c_char,
) -> CDecompressResult {
    let mut result = CDecompressResult {
        success: 0,
        file_count: 0,
        error_message: ptr::null_mut(),
    };
    
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的压缩包路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_dir).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出目录编码");
            return result;
        }
    };
    
    // 解析密码
    let password_opt = if password.is_null() {
        None
    } else {
        match CStr::from_ptr(password).to_str() {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    };
    
    let decompressor = sz_compress::Decompressor::new();
    
    match decompressor.decompress(archive_str, output_str, password_opt, |progress| {
        PROGRESS_CURRENT.store(progress.processed_bytes, Ordering::SeqCst);
        PROGRESS_TOTAL.store(progress.total_bytes, Ordering::SeqCst);
    }) {
        Ok(files) => {
            result.success = 1;
            result.file_count = files.len() as i32;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("{}", e));
        }
    }
    
    result
}

/// 7z 列出压缩包内容
#[no_mangle]
pub unsafe extern "C" fn sz_list_7z_contents(
    archive_path: *const c_char,
    password: *const c_char,
) -> *mut c_char {
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => return string_to_c(""),
    };
    
    // 解析密码（可选）
    let _password_opt = if password.is_null() {
        None
    } else {
        match CStr::from_ptr(password).to_str() {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    };
    
    let decompressor = sz_compress::Decompressor::new();
    
    match decompressor.list_contents(archive_str) {
        Ok(files) => {
            // 用 | 分隔文件名列表
            let result = files.join("|");
            string_to_c(&result)
        }
        Err(_) => string_to_c(""),
    }
}

// ============================================================================
// 工具函数
// ============================================================================

fn string_to_c(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// ============================================================================
// Tar + Zstd + AES256 压缩函数（主推方案）
// ============================================================================

/// Tar + Zstd 压缩（可选密码）
/// 
/// 输出 .szp 格式
/// 如果提供密码，将使用 AES-256-GCM 加密
#[no_mangle]
pub unsafe extern "C" fn sz_compress_tar_zstd(
    input_paths: *const c_char,  // 用 | 分隔的路径
    output_path: *const c_char,
    password: *const c_char,     // 可为空
    compression_level: i32,
) -> CCompressResult {
    let mut result = CCompressResult::default();
    
    // 解析输入路径
    let input_str = match CStr::from_ptr(input_paths).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输入路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出路径编码");
            return result;
        }
    };
    
    // 解析密码（可选）
    let password_opt = if password.is_null() {
        None
    } else {
        match CStr::from_ptr(password).to_str() {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    };
    
    let paths: Vec<String> = input_str.split('|').map(String::from).collect();
    
    // 重置取消标志
    CANCELLED.store(false, Ordering::SeqCst);
    
    // 创建取消标志的 Arc
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_clone = cancel_flag.clone();
    
    // 使用 Tar + Zstd 压缩，带取消支持
    let compressor = sz_compress::TarZstdCompressor::with_cancel_flag(compression_level, cancel_flag);
    
    match compressor.compress(&paths, output_str, password_opt, |current, total, _| {
        PROGRESS_CURRENT.store(current, Ordering::SeqCst);
        PROGRESS_TOTAL.store(total, Ordering::SeqCst);
        // 同步全局取消标志到压缩器
        if CANCELLED.load(Ordering::Relaxed) {
            cancel_flag_clone.store(true, Ordering::Relaxed);
        }
    }) {
        Ok(r) => {
            result.success = 1;
            result.original_size = r.original_size;
            result.compressed_size = r.compressed_size;
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            // 如果是取消错误，不设置错误消息
            if err_msg.contains("取消") {
                result.error_message = string_to_c("操作已取消");
            } else {
                result.error_message = string_to_c(&err_msg);
            }
        }
    }
    
    result
}

/// Tar + Zstd 解压（可选密码）
#[no_mangle]
pub unsafe extern "C" fn sz_decompress_tar_zstd(
    archive_path: *const c_char,
    output_dir: *const c_char,
    password: *const c_char,  // 可为空
) -> CDecompressResult {
    let mut result = CDecompressResult {
        success: 0,
        file_count: 0,
        error_message: ptr::null_mut(),
    };
    
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的压缩包路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_dir).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出目录编码");
            return result;
        }
    };
    
    // 解析密码（可选）
    let password_opt = if password.is_null() {
        None
    } else {
        match CStr::from_ptr(password).to_str() {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    };
    
    let decompressor = sz_compress::TarZstdDecompressor::new();
    
    match decompressor.decompress(archive_str, output_str, password_opt, |current, total, _| {
        PROGRESS_CURRENT.store(current, Ordering::SeqCst);
        PROGRESS_TOTAL.store(total, Ordering::SeqCst);
    }) {
        Ok(files) => {
            result.success = 1;
            result.file_count = files.len() as i32;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("{}", e));
        }
    }
    
    result
}

/// 验证 Tar+Zstd 压缩包密码
#[no_mangle]
pub unsafe extern "C" fn sz_verify_tar_zstd_password(
    archive_path: *const c_char,
    password: *const c_char,
) -> i32 {
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let password_str = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let decompressor = sz_compress::TarZstdDecompressor::new();
    
    match decompressor.verify_password(archive_str, password_str) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

/// 检查 Tar+Zstd 压缩包是否需要密码
#[no_mangle]
pub unsafe extern "C" fn sz_requires_tar_zstd_password(
    archive_path: *const c_char,
) -> i32 {
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    match sz_compress::TarZstdDecompressor::requires_password(archive_str) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

// ============================================================================
// 智能解压函数（自动检测格式）
// ============================================================================

/// 智能解压（自动检测格式）
/// 
/// 支持：.szp, .sz7z, .tar.zst, .7z
#[no_mangle]
pub unsafe extern "C" fn sz_smart_decompress(
    archive_path: *const c_char,
    output_dir: *const c_char,
    password: *const c_char,  // 可为空
) -> CDecompressResult {
    let mut result = CDecompressResult {
        success: 0,
        file_count: 0,
        error_message: ptr::null_mut(),
    };
    
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的压缩包路径编码");
            return result;
        }
    };
    
    let output_str = match CStr::from_ptr(output_dir).to_str() {
        Ok(s) => s,
        Err(_) => {
            result.error_message = string_to_c("无效的输出目录编码");
            return result;
        }
    };
    
    let password_opt = if password.is_null() {
        None
    } else {
        match CStr::from_ptr(password).to_str() {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    };
    
    match sz_compress::SmartDecompressor::decompress(archive_str, output_str, password_opt, |current, total, _| {
        PROGRESS_CURRENT.store(current, Ordering::SeqCst);
        PROGRESS_TOTAL.store(total, Ordering::SeqCst);
    }) {
        Ok(files) => {
            result.success = 1;
            result.file_count = files.len() as i32;
        }
        Err(e) => {
            result.error_message = string_to_c(&format!("{}", e));
        }
    }
    
    result
}

/// 检测压缩包格式
/// 
/// 返回: 0=未知, 1=szp, 2=sz7z, 3=tar.zst, 4=7z
#[no_mangle]
pub unsafe extern "C" fn sz_detect_format(
    archive_path: *const c_char,
) -> i32 {
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    match sz_compress::SmartDecompressor::detect_format(archive_str) {
        Ok(format) => match format {
            sz_compress::ArchiveFormat::Szp => 1,
            sz_compress::ArchiveFormat::Sz7z => 2,
            sz_compress::ArchiveFormat::TarZstd => 3,
            sz_compress::ArchiveFormat::SevenZ => 4,
            sz_compress::ArchiveFormat::Unknown => 0,
        },
        Err(_) => 0,
    }
}

/// 智能检测是否需要密码
#[no_mangle]
pub unsafe extern "C" fn sz_smart_requires_password(
    archive_path: *const c_char,
) -> i32 {
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    match sz_compress::SmartDecompressor::requires_password(archive_str) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

/// 智能验证密码
#[no_mangle]
pub unsafe extern "C" fn sz_smart_verify_password(
    archive_path: *const c_char,
    password: *const c_char,
) -> i32 {
    let archive_str = match CStr::from_ptr(archive_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    let password_str = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    match sz_compress::SmartDecompressor::verify_password(archive_str, password_str) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

