package com.sezip.sezip

/**
 * Rust JNI 桥接层
 *
 * 通过 JNI 调用 Rust 编写的 sz-ffi 库，提供压缩/解密/WebDAV 等功能。
 * 所有 external fun 对应 Rust 侧的 Java_com_sezip_sezip_RustBridge_* 函数。
 *
 * 当 Rust .so 未就绪时（JNI 符号不匹配），isAvailable = false，
 * 调用方应检查此标志并展示相应提示。
 */
object RustBridge {
    /** Rust 库是否可用（加载成功且 JNI 符号匹配） */
    var isAvailable: Boolean = false
        private set

    init {
        try {
            System.loadLibrary("sz_ffi")
            // 用一个简单的同步函数探测 JNI 符号是否匹配
            getVersion()
            isAvailable = true
        } catch (_: UnsatisfiedLinkError) {
            // .so 不存在或 JNI 符号不匹配
            isAvailable = false
        } catch (_: Exception) {
            isAvailable = false
        }
    }

    /** 进度回调接口，由 Rust 侧通过 JNI 调用 */
    interface ProgressCallback {
        fun onProgress(current: Long, total: Long, currentFile: String?)
    }

    // ── CancelToken 管理 ──────────────────────────────────────────────

    external fun cancelTokenNew(): Long
    external fun cancelTokenCancel(handle: Long)
    external fun cancelTokenFree(handle: Long)

    // ── .zbak 备份 API ────────────────────────────────────────────────

    external fun compressZbak(
        inputPaths: Array<String>,
        outputPath: String,
        password: String?,
        level: Int,
        encryptFilenames: Boolean,
        enableRecovery: Boolean,
        recoveryRatio: Float,
        splitSize: Long,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun decompressZbak(
        archivePath: String,
        outputDir: String,
        password: String?,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun listZbakContents(archivePath: String, password: String?): String

    external fun extractZbakFile(
        archivePath: String,
        filePath: String,
        outputPath: String,
        password: String?,
    )

    external fun zbakRequiresPassword(archivePath: String): Boolean
    external fun zbakVerifyPassword(archivePath: String, password: String): Boolean

    // ── 智能解压 API ──────────────────────────────────────────────────

    external fun smartDecompress(
        archivePath: String,
        outputDir: String,
        password: String?,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun detectFormat(archivePath: String): String
    external fun smartRequiresPassword(archivePath: String): Boolean
    external fun smartVerifyPassword(archivePath: String, password: String): Boolean

    // ── 标准 7z API ───────────────────────────────────────────────────

    external fun compress7z(
        inputPaths: Array<String>,
        outputPath: String,
        password: String?,
        level: Int,
        callback: ProgressCallback,
    ): String

    external fun decompress7z(
        archivePath: String,
        outputDir: String,
        password: String?,
        callback: ProgressCallback,
    ): String

    external fun list7zContents(archivePath: String): String

    // ── 旧版 .sz7z API ───────────────────────────────────────────────

    external fun compressLegacy(
        inputPaths: Array<String>,
        outputPath: String,
        level: Int,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun compressLegacyEncrypted(
        inputPaths: Array<String>,
        outputPath: String,
        password: String,
        level: Int,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun verifyLegacyPassword(archivePath: String, password: String): Boolean

    // ── WebDAV API ────────────────────────────────────────────────────

    external fun webdavTestConnection(url: String, username: String, password: String): Boolean

    external fun webdavBackup(
        inputPaths: Array<String>,
        url: String,
        username: String,
        webdavPassword: String,
        encryptPassword: String?,
        level: Int,
        recoveryRatio: Float,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun webdavRestore(
        backupId: String,
        outputDir: String,
        url: String,
        username: String,
        webdavPassword: String,
        encryptPassword: String?,
        callback: ProgressCallback,
    ): String

    external fun webdavListBackups(url: String, username: String, password: String): String

    // ── 加密工具 API ──────────────────────────────────────────────────

    external fun encryptString(data: String, password: String): String
    external fun decryptString(encryptedData: String, password: String): String
    external fun generateRandomPassword(length: Int, includeSymbols: Boolean): String
    external fun calculatePasswordStrength(password: String): Int

    // ── 文件名混淆 API ────────────────────────────────────────────────

    external fun obfuscateFilenames(
        originalNames: Array<String>,
        scheme: Int,
        archivePath: String,
    ): String

    // ── 工具 API ──────────────────────────────────────────────────────

    external fun initLogger()
    external fun getVersion(): String

    // ── 照片增量备份 API ──────────────────────────────────────────────

    external fun photoScanIncremental(
        directories: Array<String>,
        indexPath: String,
        includeVideos: Boolean,
    ): String

    external fun photoBackupIncremental(
        directories: Array<String>,
        outputPath: String,
        indexPath: String,
        password: String?,
        exifStripLevel: Int,
        includeVideos: Boolean,
        compressionLevel: Int,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun photoBackupToWebdav(
        directories: Array<String>,
        indexPath: String,
        url: String,
        username: String,
        webdavPassword: String,
        encryptPassword: String?,
        exifStripLevel: Int,
        includeVideos: Boolean,
        compressionLevel: Int,
        cancelHandle: Long,
        callback: ProgressCallback,
    ): String

    external fun photoGetSyncStats(indexPath: String): String
}
