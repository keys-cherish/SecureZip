//! FFI API 导出
//! 
//! 这些函数将通过 flutter_rust_bridge 暴露给 Flutter

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sz_core::{
    CompressOptions, CompressResult, 
    ObfuscationScheme, MappingEntry,
    // WebDavConfig, WebDavFileInfo,  // WebDAV 暂时禁用
};
use sz_compress::{Compressor, Decompressor};
use sz_crypto::{AesEncryptor, generate_salt, derive_key_from_password};
use sz_filename::FilenameObfuscator;
// use sz_webdav::WebDavClient;  // WebDAV 暂时禁用

// ============ 压缩 API ============

/// 压缩文件
/// 
/// # Arguments
/// * `input_paths` - 输入文件或文件夹路径
/// * `output_path` - 输出 .7z 文件路径
/// * `password` - 可选密码
/// * `enable_obfuscation` - 是否启用文件名混淆
/// * `obfuscation_scheme` - 混淆方案 (0-4)
/// * `compression_level` - 压缩级别 (1-9)
pub fn compress_files(
    input_paths: Vec<String>,
    output_path: String,
    password: Option<String>,
    enable_obfuscation: bool,
    obfuscation_scheme: u8,
    compression_level: u8,
) -> Result<CompressResult, String> {
    let scheme = match obfuscation_scheme {
        0 => ObfuscationScheme::Sequential,
        1 => ObfuscationScheme::DateSequential,
        2 => ObfuscationScheme::Random,
        3 => ObfuscationScheme::Hash,
        4 => ObfuscationScheme::Encrypted,
        _ => ObfuscationScheme::Sequential,
    };

    let options = CompressOptions {
        password,
        enable_obfuscation,
        obfuscation_scheme: scheme,
        compression_level,
    };

    let compressor = Compressor::new(options);
    
    // 使用空回调进行压缩（实际应用中应使用流式回调）
    compressor
        .compress(&input_paths, &output_path, |_progress| {})
        .map_err(|e| e.to_string())
}

/// 解压文件
/// 
/// # Arguments
/// * `archive_path` - 压缩包路径
/// * `output_dir` - 输出目录
/// * `password` - 可选密码
pub fn decompress_file(
    archive_path: String,
    output_dir: String,
    password: Option<String>,
) -> Result<Vec<String>, String> {
    let decompressor = Decompressor::new();
    
    decompressor
        .decompress(
            &archive_path,
            &output_dir,
            password.as_deref(),
            |_progress| {},
        )
        .map_err(|e| e.to_string())
}

/// 检查压缩包是否需要密码
pub fn check_archive_requires_password(archive_path: String) -> Result<bool, String> {
    let decompressor = Decompressor::new();
    decompressor
        .requires_password(&archive_path)
        .map_err(|e| e.to_string())
}

/// 列出压缩包内容
pub fn list_archive_contents(archive_path: String) -> Result<Vec<String>, String> {
    let decompressor = Decompressor::new();
    decompressor
        .list_contents(&archive_path)
        .map_err(|e| e.to_string())
}

/// 验证压缩包密码
pub fn verify_archive_password(archive_path: String, password: String) -> Result<bool, String> {
    let decompressor = Decompressor::new();
    decompressor
        .verify_password(&archive_path, &password)
        .map_err(|e| e.to_string())
}

// ============ 加密 API ============

/// 加密字符串
pub fn encrypt_string(data: String, password: String) -> Result<String, String> {
    let salt = generate_salt();
    let key = derive_key_from_password(&password, &salt).map_err(|e| e.to_string())?;
    let encryptor = AesEncryptor::new(&key);
    
    // 将 salt 和加密数据一起返回
    let encrypted = encryptor.encrypt_string(&data).map_err(|e| e.to_string())?;
    
    // 格式: base64(salt):encrypted
    let salt_b64 = STANDARD.encode(&salt);
    Ok(format!("{}:{}", salt_b64, encrypted))
}

/// 解密字符串
pub fn decrypt_string(encrypted_data: String, password: String) -> Result<String, String> {
    let parts: Vec<&str> = encrypted_data.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("无效的加密数据格式".to_string());
    }
    
    let salt = STANDARD.decode(parts[0])
        .map_err(|e| e.to_string())?;
    let encrypted = parts[1];
    
    let key = derive_key_from_password(&password, &salt).map_err(|e| e.to_string())?;
    let encryptor = AesEncryptor::new(&key);
    
    encryptor.decrypt_string(encrypted).map_err(|e| e.to_string())
}

/// 生成随机密码
pub fn generate_random_password(length: u32, include_symbols: bool) -> String {
    sz_crypto::generate_random_password(length as usize, include_symbols)
}

/// 计算密码强度 (0-4)
pub fn calculate_password_strength(password: String) -> u8 {
    sz_crypto::calculate_password_strength(&password)
}

// ============ 文件名混淆 API ============

/// 混淆文件名列表
pub fn obfuscate_filenames(
    original_names: Vec<String>,
    scheme: u8,
    archive_path: String,
) -> Vec<MappingEntry> {
    let obfuscation_scheme = match scheme {
        0 => ObfuscationScheme::Sequential,
        1 => ObfuscationScheme::DateSequential,
        2 => ObfuscationScheme::Random,
        3 => ObfuscationScheme::Hash,
        4 => ObfuscationScheme::Encrypted,
        _ => ObfuscationScheme::Sequential,
    };

    let mut obfuscator = FilenameObfuscator::new(obfuscation_scheme);
    obfuscator.obfuscate_batch(&original_names, &archive_path)
}

// ============ WebDAV API ============
// WebDAV 功能暂时禁用（需要 OpenSSL 交叉编译）
/*
/// 测试 WebDAV 连接
pub fn test_webdav_connection(
    server_url: String,
    username: String,
    password: String,
    remote_path: String,
) -> Result<bool, String> {
    let config = WebDavConfig {
        server_url,
        username,
        password,
        remote_path,
    };

    let client = WebDavClient::new(config).map_err(|e| e.to_string())?;
    client.test_connection().map_err(|e| e.to_string())
}

/// 列出 WebDAV 目录内容
pub fn list_webdav_directory(
    server_url: String,
    username: String,
    password: String,
    remote_path: String,
) -> Result<Vec<WebDavFileInfo>, String> {
    let config = WebDavConfig {
        server_url,
        username,
        password,
        remote_path: "/".to_string(),
    };

    let client = WebDavClient::new(config).map_err(|e| e.to_string())?;
    client.list_directory(&remote_path).map_err(|e| e.to_string())
}

/// 上传文件到 WebDAV
pub fn upload_to_webdav(
    server_url: String,
    username: String,
    password: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let config = WebDavConfig {
        server_url,
        username,
        password,
        remote_path: "/".to_string(),
    };

    let client = WebDavClient::new(config).map_err(|e| e.to_string())?;
    client
        .upload_file(&local_path, &remote_path, |_| {})
        .map_err(|e| e.to_string())
}

/// 从 WebDAV 下载文件
pub fn download_from_webdav(
    server_url: String,
    username: String,
    password: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    let config = WebDavConfig {
        server_url,
        username,
        password,
        remote_path: "/".to_string(),
    };

    let client = WebDavClient::new(config).map_err(|e| e.to_string())?;
    client
        .download_file(&remote_path, &local_path, |_| {})
        .map_err(|e| e.to_string())
}

/// 删除 WebDAV 文件
pub fn delete_webdav_file(
    server_url: String,
    username: String,
    password: String,
    remote_path: String,
) -> Result<(), String> {
    let config = WebDavConfig {
        server_url,
        username,
        password,
        remote_path: "/".to_string(),
    };

    let client = WebDavClient::new(config).map_err(|e| e.to_string())?;
    client.delete(&remote_path).map_err(|e| e.to_string())
}
*/

// ============ 工具 API ============

/// 初始化日志
pub fn init_logger() {
    let _ = env_logger::try_init();
}

/// 获取版本信息
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
