//! JNI 桥接层
//!
//! 将 Rust API 函数暴露为 JNI 函数，供 Kotlin 直接调用。
//!
//! 命名规则: Java_com_sezip_sezip_RustBridge_<functionName>
//!
//! 模式:
//! - 进度回调: Kotlin 传入 ProgressCallback 对象，Rust 通过 env.call_method 回调
//! - CancelToken: cancelTokenNew() 返回 jlong (Box<Arc<AtomicBool>> 指针)
//! - 错误处理: catch anyhow::Result，出错时 env.throw_new
//! - 复杂返回值: JSON 字符串

use jni::JNIEnv;
use jni::objects::{JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::{jboolean, jfloat, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::api;

// ============================================================================
// 辅助函数
// ============================================================================

/// 从 JString 提取 Rust String
fn get_string(env: &mut JNIEnv, s: &JString) -> String {
    env.get_string(s)
        .map(|s| s.into())
        .unwrap_or_default()
}

/// 从 JString 提取 Option<String>（null → None, 空串 → None）
fn get_optional_string(env: &mut JNIEnv, s: &JString) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let s = get_string(env, s);
    if s.is_empty() { None } else { Some(s) }
}

/// 从 JObjectArray 提取 Vec<String>
fn get_string_array(env: &mut JNIEnv, arr: &JObjectArray) -> Vec<String> {
    let len = env.get_array_length(arr).unwrap_or(0);
    (0..len)
        .filter_map(|i| {
            let obj = env.get_object_array_element(arr, i).ok()?;
            let jstr: JString = obj.into();
            Some(get_string(env, &jstr))
        })
        .collect()
}

/// 创建 Java String 返回值
fn make_jstring(env: &mut JNIEnv, s: &str) -> jstring {
    env.new_string(s)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// 回调进度到 Kotlin ProgressCallback.onProgress(current, total, currentFile)
fn call_progress(env: &mut JNIEnv, callback: &JObject, current: u64, total: u64, file: Option<&str>) {
    let file_str = match file {
        Some(f) => match env.new_string(f) {
            Ok(s) => JObject::from(s),
            Err(_) => JObject::null(),
        },
        None => JObject::null(),
    };

    let _ = env.call_method(
        callback,
        "onProgress",
        "(JJLjava/lang/String;)V",
        &[
            JValue::Long(current as i64),
            JValue::Long(total as i64),
            JValue::Object(&file_str),
        ],
    );
}

/// 处理 Result，出错时 throw RuntimeException 并返回 default
fn handle_error<T>(env: &mut JNIEnv, result: anyhow::Result<T>, default: T) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("{:#}", e);
            let _ = env.throw_new("java/lang/RuntimeException", &msg);
            default
        }
    }
}

/// 将 CancelToken 指针还原为 Arc<AtomicBool> 引用（不释放）
fn get_cancel_flag(handle: jlong) -> Arc<AtomicBool> {
    if handle == 0 {
        return Arc::new(AtomicBool::new(false));
    }
    let boxed = unsafe { &*(handle as *const Arc<AtomicBool>) };
    boxed.clone()
}

// ============================================================================
// CancelToken 管理
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_cancelTokenNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let token = api::CancelToken::new();
    let flag = token.inner();
    let boxed = Box::new(flag);
    Box::into_raw(boxed) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_cancelTokenCancel(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        let flag = unsafe { &*(handle as *const Arc<AtomicBool>) };
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_cancelTokenFree(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut Arc<AtomicBool>);
        }
    }
}

// ============================================================================
// .zbak 备份 API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_compressZbak<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_paths: JObjectArray<'local>,
    output_path: JString<'local>,
    password: JString<'local>,
    level: jint,
    encrypt_filenames: jboolean,
    enable_recovery: jboolean,
    recovery_ratio: jfloat,
    split_size: jlong,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let paths = get_string_array(&mut env, &input_paths);
    let output = get_string(&mut env, &output_path);
    let pwd = get_optional_string(&mut env, &password);
    let cancel_flag = get_cancel_flag(cancel_handle);

    let cancel = api::CancelToken::from_flag(cancel_flag);

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::compress_zbak(
        paths,
        output,
        pwd,
        level,
        encrypt_filenames != JNI_FALSE,
        enable_recovery != JNI_FALSE,
        recovery_ratio,
        split_size as u64,
        &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::CompressResultFfi {
        original_size: 0,
        compressed_size: 0,
    });

    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_decompressZbak<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    output_dir: JString<'local>,
    password: JString<'local>,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let archive = get_string(&mut env, &archive_path);
    let output = get_string(&mut env, &output_dir);
    let pwd = get_optional_string(&mut env, &password);
    let cancel = api::CancelToken::from_flag(get_cancel_flag(cancel_handle));

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::decompress_zbak(
        archive,
        output,
        pwd,
        &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::DecompressResultFfi { file_count: 0 });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_listZbakContents<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    password: JString<'local>,
) -> jstring {
    let archive = get_string(&mut env, &archive_path);
    let pwd = get_optional_string(&mut env, &password);

    let result = api::list_zbak_contents(archive, pwd);
    let files = handle_error(&mut env, result, vec![]);
    let json = serde_json::to_string(&files).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_extractZbakFile<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    file_path: JString<'local>,
    output_path: JString<'local>,
    password: JString<'local>,
) {
    let archive = get_string(&mut env, &archive_path);
    let file = get_string(&mut env, &file_path);
    let output = get_string(&mut env, &output_path);
    let pwd = get_optional_string(&mut env, &password);

    let result = api::extract_zbak_file(archive, file, output, pwd);
    handle_error(&mut env, result, ());
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_zbakRequiresPassword<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
) -> jboolean {
    let archive = get_string(&mut env, &archive_path);
    let result = api::zbak_requires_password(archive);
    if handle_error(&mut env, result, false) { JNI_TRUE } else { JNI_FALSE }
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_zbakVerifyPassword<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    password: JString<'local>,
) -> jboolean {
    let archive = get_string(&mut env, &archive_path);
    let pwd = get_string(&mut env, &password);
    let result = api::zbak_verify_password(archive, pwd);
    if handle_error(&mut env, result, false) { JNI_TRUE } else { JNI_FALSE }
}

// ============================================================================
// 智能解压 API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_smartDecompress<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    output_dir: JString<'local>,
    password: JString<'local>,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let archive = get_string(&mut env, &archive_path);
    let output = get_string(&mut env, &output_dir);
    let pwd = get_optional_string(&mut env, &password);
    let cancel = api::CancelToken::from_flag(get_cancel_flag(cancel_handle));

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::smart_decompress(
        archive,
        output,
        pwd,
        &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::DecompressResultFfi { file_count: 0 });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_detectFormat<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
) -> jstring {
    let archive = get_string(&mut env, &archive_path);
    let result = api::detect_format(archive);
    let fmt = handle_error(&mut env, result, api::ArchiveFormat::Unknown);
    let name = match fmt {
        api::ArchiveFormat::Zbak => "zbak",
        api::ArchiveFormat::Sz7z => "sz7z",
        api::ArchiveFormat::SevenZ => "7z",
        api::ArchiveFormat::LegacySzp => "szp",
        api::ArchiveFormat::Unknown => "unknown",
    };
    make_jstring(&mut env, name)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_smartRequiresPassword<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
) -> jboolean {
    let archive = get_string(&mut env, &archive_path);
    let result = api::smart_requires_password(archive);
    if handle_error(&mut env, result, false) { JNI_TRUE } else { JNI_FALSE }
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_smartVerifyPassword<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    password: JString<'local>,
) -> jboolean {
    let archive = get_string(&mut env, &archive_path);
    let pwd = get_string(&mut env, &password);
    let result = api::smart_verify_password(archive, pwd);
    if handle_error(&mut env, result, false) { JNI_TRUE } else { JNI_FALSE }
}

// ============================================================================
// 标准 7z API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_compress7z<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_paths: JObjectArray<'local>,
    output_path: JString<'local>,
    password: JString<'local>,
    level: jint,
    callback: JObject<'local>,
) -> jstring {
    let paths = get_string_array(&mut env, &input_paths);
    let output = get_string(&mut env, &output_path);
    let pwd = get_optional_string(&mut env, &password);

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::compress_7z(
        paths,
        output,
        pwd,
        level as u8,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::CompressResultFfi {
        original_size: 0,
        compressed_size: 0,
    });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_decompress7z<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    output_dir: JString<'local>,
    password: JString<'local>,
    callback: JObject<'local>,
) -> jstring {
    let archive = get_string(&mut env, &archive_path);
    let output = get_string(&mut env, &output_dir);
    let pwd = get_optional_string(&mut env, &password);

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::decompress_7z(
        archive,
        output,
        pwd,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::DecompressResultFfi { file_count: 0 });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_list7zContents<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
) -> jstring {
    let archive = get_string(&mut env, &archive_path);
    let result = api::list_7z_contents(archive);
    let files = handle_error(&mut env, result, vec![]);
    let json = serde_json::to_string(&files).unwrap_or_default();
    make_jstring(&mut env, &json)
}

// ============================================================================
// 旧版 .sz7z API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_compressLegacy<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_paths: JObjectArray<'local>,
    output_path: JString<'local>,
    level: jint,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let paths = get_string_array(&mut env, &input_paths);
    let output = get_string(&mut env, &output_path);
    let cancel = api::CancelToken::from_flag(get_cancel_flag(cancel_handle));

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::compress_legacy(
        paths,
        output,
        level,
        &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::CompressResultFfi {
        original_size: 0,
        compressed_size: 0,
    });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_compressLegacyEncrypted<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_paths: JObjectArray<'local>,
    output_path: JString<'local>,
    password: JString<'local>,
    level: jint,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let paths = get_string_array(&mut env, &input_paths);
    let output = get_string(&mut env, &output_path);
    let pwd = get_string(&mut env, &password);
    let cancel = api::CancelToken::from_flag(get_cancel_flag(cancel_handle));

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::compress_legacy_encrypted(
        paths,
        output,
        pwd,
        level,
        &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::CompressResultFfi {
        original_size: 0,
        compressed_size: 0,
    });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_verifyLegacyPassword<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    archive_path: JString<'local>,
    password: JString<'local>,
) -> jboolean {
    let archive = get_string(&mut env, &archive_path);
    let pwd = get_string(&mut env, &password);
    let result = api::verify_legacy_password(archive, pwd);
    if handle_error(&mut env, result, false) { JNI_TRUE } else { JNI_FALSE }
}

// ============================================================================
// WebDAV API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_webdavTestConnection<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    url: JString<'local>,
    username: JString<'local>,
    password: JString<'local>,
) -> jboolean {
    let url = get_string(&mut env, &url);
    let user = get_string(&mut env, &username);
    let pwd = get_string(&mut env, &password);
    let result = api::webdav_test_connection(url, user, pwd);
    if handle_error(&mut env, result, false) { JNI_TRUE } else { JNI_FALSE }
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_webdavBackup<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input_paths: JObjectArray<'local>,
    url: JString<'local>,
    username: JString<'local>,
    webdav_password: JString<'local>,
    encrypt_password: JString<'local>,
    level: jint,
    recovery_ratio: jfloat,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let paths = get_string_array(&mut env, &input_paths);
    let url = get_string(&mut env, &url);
    let user = get_string(&mut env, &username);
    let wdav_pwd = get_string(&mut env, &webdav_password);
    let enc_pwd = get_optional_string(&mut env, &encrypt_password);
    let cancel = api::CancelToken::from_flag(get_cancel_flag(cancel_handle));

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::webdav_backup(
        paths, url, user, wdav_pwd, enc_pwd, level, recovery_ratio, &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let manifest = handle_error(&mut env, result, "{}".to_string());
    make_jstring(&mut env, &manifest)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_webdavRestore<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    backup_id: JString<'local>,
    output_dir: JString<'local>,
    url: JString<'local>,
    username: JString<'local>,
    webdav_password: JString<'local>,
    encrypt_password: JString<'local>,
    callback: JObject<'local>,
) -> jstring {
    let bid = get_string(&mut env, &backup_id);
    let output = get_string(&mut env, &output_dir);
    let url = get_string(&mut env, &url);
    let user = get_string(&mut env, &username);
    let wdav_pwd = get_string(&mut env, &webdav_password);
    let enc_pwd = get_optional_string(&mut env, &encrypt_password);

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::webdav_restore(
        bid, output, url, user, wdav_pwd, enc_pwd,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::DecompressResultFfi { file_count: 0 });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_webdavListBackups<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    url: JString<'local>,
    username: JString<'local>,
    password: JString<'local>,
) -> jstring {
    let url = get_string(&mut env, &url);
    let user = get_string(&mut env, &username);
    let pwd = get_string(&mut env, &password);
    let result = api::webdav_list_backups(url, user, pwd);
    let json = handle_error(&mut env, result, "[]".to_string());
    make_jstring(&mut env, &json)
}

// ============================================================================
// 加密工具 API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_encryptString<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    data: JString<'local>,
    password: JString<'local>,
) -> jstring {
    let data = get_string(&mut env, &data);
    let pwd = get_string(&mut env, &password);
    let result = api::encrypt_string(data, pwd);
    let encrypted = handle_error(&mut env, result, String::new());
    make_jstring(&mut env, &encrypted)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_decryptString<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    encrypted_data: JString<'local>,
    password: JString<'local>,
) -> jstring {
    let data = get_string(&mut env, &encrypted_data);
    let pwd = get_string(&mut env, &password);
    let result = api::decrypt_string(data, pwd);
    let decrypted = handle_error(&mut env, result, String::new());
    make_jstring(&mut env, &decrypted)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_generateRandomPassword<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    length: jint,
    include_symbols: jboolean,
) -> jstring {
    let pwd = api::generate_random_password(length as u32, include_symbols != JNI_FALSE);
    make_jstring(&mut env, &pwd)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_calculatePasswordStrength<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    password: JString<'local>,
) -> jint {
    let pwd = get_string(&mut env, &password);
    api::calculate_password_strength(pwd) as jint
}

// ============================================================================
// 文件名混淆 API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_obfuscateFilenames<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    original_names: JObjectArray<'local>,
    scheme: jint,
    archive_path: JString<'local>,
) -> jstring {
    let names = get_string_array(&mut env, &original_names);
    let archive = get_string(&mut env, &archive_path);
    let mappings = api::obfuscate_filenames(names, scheme as u8, archive);
    let json = serde_json::to_string(&mappings).unwrap_or_default();
    make_jstring(&mut env, &json)
}

// ============================================================================
// 工具 API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_initLogger(
    _env: JNIEnv,
    _class: JClass,
) {
    api::init_logger();
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_getVersion<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    let version = api::get_version();
    make_jstring(&mut env, &version)
}

// ============================================================================
// 照片增量备份 API
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_photoScanIncremental<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    directories: JObjectArray<'local>,
    index_path: JString<'local>,
    include_videos: jboolean,
) -> jstring {
    let dirs = get_string_array(&mut env, &directories);
    let idx_path = get_string(&mut env, &index_path);
    let result = api::photo_scan_incremental(dirs, idx_path, include_videos != JNI_FALSE);
    let scan = handle_error(&mut env, result, api::PhotoScanResult {
        total_files: 0,
        new_files: 0,
        transfer_bytes: 0,
        skipped_files: 0,
        deleted_files: 0,
    });
    let json = serde_json::to_string(&scan).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_photoBackupIncremental<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    directories: JObjectArray<'local>,
    output_path: JString<'local>,
    index_path: JString<'local>,
    password: JString<'local>,
    exif_strip_level: jint,
    include_videos: jboolean,
    compression_level: jint,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let dirs = get_string_array(&mut env, &directories);
    let output = get_string(&mut env, &output_path);
    let idx_path = get_string(&mut env, &index_path);
    let pwd = get_optional_string(&mut env, &password);
    let cancel = api::CancelToken::from_flag(get_cancel_flag(cancel_handle));

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::photo_backup_incremental(
        dirs,
        output,
        idx_path,
        pwd,
        exif_strip_level as u8,
        include_videos != JNI_FALSE,
        compression_level,
        &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let result = handle_error(&mut env, result, api::CompressResultFfi {
        original_size: 0,
        compressed_size: 0,
    });
    let json = serde_json::to_string(&result).unwrap_or_default();
    make_jstring(&mut env, &json)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_photoBackupToWebdav<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    directories: JObjectArray<'local>,
    index_path: JString<'local>,
    url: JString<'local>,
    username: JString<'local>,
    webdav_password: JString<'local>,
    encrypt_password: JString<'local>,
    exif_strip_level: jint,
    include_videos: jboolean,
    compression_level: jint,
    cancel_handle: jlong,
    callback: JObject<'local>,
) -> jstring {
    let dirs = get_string_array(&mut env, &directories);
    let idx_path = get_string(&mut env, &index_path);
    let url = get_string(&mut env, &url);
    let user = get_string(&mut env, &username);
    let wdav_pwd = get_string(&mut env, &webdav_password);
    let enc_pwd = get_optional_string(&mut env, &encrypt_password);
    let cancel = api::CancelToken::from_flag(get_cancel_flag(cancel_handle));

    let callback_ref = env.new_global_ref(callback).unwrap();
    let jvm = env.get_java_vm().unwrap();

    let result = api::photo_backup_to_webdav(
        dirs, idx_path, url, user, wdav_pwd, enc_pwd,
        exif_strip_level as u8, include_videos != JNI_FALSE, compression_level,
        &cancel,
        |current, total, file| {
            let mut env = jvm.attach_current_thread().unwrap();
            call_progress(&mut env, callback_ref.as_obj(), current, total, file);
        },
    );

    let manifest = handle_error(&mut env, result, "{}".to_string());
    make_jstring(&mut env, &manifest)
}

#[no_mangle]
pub extern "system" fn Java_com_sezip_sezip_RustBridge_photoGetSyncStats<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    index_path: JString<'local>,
) -> jstring {
    let idx_path = get_string(&mut env, &index_path);
    let result = api::photo_get_sync_stats(idx_path);
    let stats = handle_error(&mut env, result, api::PhotoSyncStats {
        total_backed_up: 0,
        total_bytes: 0,
        saved_bytes: 0,
        last_sync: None,
    });
    let json = serde_json::to_string(&stats).unwrap_or_default();
    make_jstring(&mut env, &json)
}

