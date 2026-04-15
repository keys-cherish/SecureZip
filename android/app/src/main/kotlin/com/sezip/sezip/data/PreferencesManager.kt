package com.sezip.sezip.data

import android.content.Context
import android.content.SharedPreferences

/**
 * SharedPreferences 管理器
 *
 * 使用与 Flutter 相同的 SharedPreferences 文件和 key，
 * 确保用户从 Flutter 版本升级后数据零迁移。
 *
 * Flutter SharedPreferences 在 Android 上的文件名: FlutterSharedPreferences
 * Flutter 的 key 前缀: "flutter."
 */
class PreferencesManager(context: Context) {
    private val prefs: SharedPreferences =
        context.getSharedPreferences("FlutterSharedPreferences", Context.MODE_PRIVATE)

    // 与 Flutter SettingsService 使用相同的 key
    companion object {
        private const val PREFIX = "flutter."
        private const val KEY_THEME_MODE = "${PREFIX}secure_zip_theme_mode"
        private const val KEY_DEFAULT_SCHEME = "${PREFIX}secure_zip_default_scheme"
        private const val KEY_COMPRESSION_LEVEL = "${PREFIX}secure_zip_compression_level"
        private const val KEY_OUTPUT_DIR = "${PREFIX}secure_zip_output_dir"
        private const val KEY_DECOMPRESS_DIR = "${PREFIX}secure_zip_decompress_output_dir"
        private const val KEY_PASSWORDS = "${PREFIX}secure_zip_passwords"
        private const val KEY_MAPPINGS = "${PREFIX}secure_zip_mappings"
        private const val KEY_EXT_MAPPINGS = "${PREFIX}secure_zip_ext_mappings"
        private const val KEY_WEBDAV_CONFIG = "${PREFIX}secure_zip_webdav_config"

        const val DEFAULT_COMPRESS_DIR = "/storage/emulated/0/SecureZip/compressed"
        const val DEFAULT_DECOMPRESS_DIR = "/storage/emulated/0/SecureZip/extracted"
    }

    // 主题
    var themeModeIndex: Int
        get() = prefs.getLong(KEY_THEME_MODE, 0).toInt()
        set(value) = prefs.edit().putLong(KEY_THEME_MODE, value.toLong()).apply()

    // 压缩级别
    var compressionLevel: Int
        get() = prefs.getLong(KEY_COMPRESSION_LEVEL, 6).toInt()
        set(value) = prefs.edit().putLong(KEY_COMPRESSION_LEVEL, value.toLong()).apply()

    // 默认混淆方案
    var defaultObfuscationScheme: String
        get() = prefs.getString(KEY_DEFAULT_SCHEME, "sequential") ?: "sequential"
        set(value) = prefs.edit().putString(KEY_DEFAULT_SCHEME, value).apply()

    // 输出目录
    var outputDir: String
        get() = prefs.getString(KEY_OUTPUT_DIR, "") ?: ""
        set(value) = prefs.edit().putString(KEY_OUTPUT_DIR, value).apply()

    var decompressOutputDir: String
        get() = prefs.getString(KEY_DECOMPRESS_DIR, "") ?: ""
        set(value) = prefs.edit().putString(KEY_DECOMPRESS_DIR, value).apply()

    fun getEffectiveOutputDir(): String = outputDir.ifBlank { DEFAULT_COMPRESS_DIR }
    fun getEffectiveDecompressDir(): String = decompressOutputDir.ifBlank { DEFAULT_DECOMPRESS_DIR }

    fun resetOutputDirs() {
        prefs.edit()
            .remove(KEY_OUTPUT_DIR)
            .remove(KEY_DECOMPRESS_DIR)
            .apply()
    }

    // 密码本 (JSON 字符串)
    var passwordsJson: String
        get() = prefs.getString(KEY_PASSWORDS, "[]") ?: "[]"
        set(value) = prefs.edit().putString(KEY_PASSWORDS, value).apply()

    // 映射表 (JSON 字符串)
    var mappingsJson: String
        get() = prefs.getString(KEY_MAPPINGS, "[]") ?: "[]"
        set(value) = prefs.edit().putString(KEY_MAPPINGS, value).apply()

    // 扩展名-密码映射 (JSON 字符串)
    var extensionMappingsJson: String
        get() = prefs.getString(KEY_EXT_MAPPINGS, "[]") ?: "[]"
        set(value) = prefs.edit().putString(KEY_EXT_MAPPINGS, value).apply()

    // WebDAV 配置 (JSON 字符串)
    var webdavConfigJson: String
        get() = prefs.getString(KEY_WEBDAV_CONFIG, "{}") ?: "{}"
        set(value) = prefs.edit().putString(KEY_WEBDAV_CONFIG, value).apply()
}
