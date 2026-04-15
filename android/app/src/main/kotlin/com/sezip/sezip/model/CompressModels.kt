package com.sezip.sezip.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/** 压缩模式 */
enum class CompressMode(val displayName: String) {
    ZBAK("本地备份"),
    ZBAK_WEBDAV("WebDAV 备份"),
    LEGACY_7Z("7z 导出"),
}

/** 恢复记录冗余比例 */
enum class RecoveryRatio(val value: Float, val displayName: String) {
    PERCENT_5(0.05f, "5%"),
    PERCENT_10(0.10f, "10%"),
    PERCENT_20(0.20f, "20%"),
}

/** 分卷大小预设 */
enum class SplitSizePreset(val bytes: Long, val displayName: String) {
    NONE(0, "不分卷"),
    MB_100(100L * 1024 * 1024, "100 MB"),
    MB_200(200L * 1024 * 1024, "200 MB"),
    MB_500(500L * 1024 * 1024, "500 MB"),
    GB_1(1024L * 1024 * 1024, "1 GB"),
    GB_2(2L * 1024 * 1024 * 1024, "2 GB"),
    GB_4(4L * 1024 * 1024 * 1024, "4 GB"),
}

/** 文件名混淆方案 */
enum class ObfuscationType(val code: Int, val displayName: String, val description: String) {
    SEQUENTIAL(0, "顺序编号", "file_001, file_002, ..."),
    DATE_SEQUENTIAL(1, "日期编号", "20260415_001, 20260415_002, ..."),
    RANDOM(2, "随机字符", "a3f8b2c1, x7e9d4f0, ..."),
    HASH(3, "哈希", "SHA256(原始名).前8位"),
    ENCRYPTED(4, "加密", "AES 加密后的文件名"),
}

/** 压缩选项 — 聚合 UI 层所有可配置参数 */
data class CompressOptions(
    val password: String? = null,
    val compressionLevel: Int = 6,
    val compressMode: CompressMode = CompressMode.ZBAK,
    val encryptFilenames: Boolean = false,
    val enableRecovery: Boolean = false,
    val recoveryRatio: RecoveryRatio = RecoveryRatio.PERCENT_5,
    val splitSize: SplitSizePreset = SplitSizePreset.NONE,
    val enableObfuscation: Boolean = false,
    val obfuscationType: ObfuscationType = ObfuscationType.SEQUENTIAL,
    val webdavUrl: String = "",
    val webdavUsername: String = "",
    val webdavPassword: String = "",
)

/** 压缩进度 — 由 Rust ProgressCallback 更新 */
data class CompressProgress(
    val current: Long = 0,
    val total: Long = 0,
    val currentFile: String? = null,
) {
    /** 完成百分比 (0.0 ~ 1.0) */
    val fraction: Float get() = if (total > 0) current.toFloat() / total else 0f

    /** 完成百分比 (0 ~ 100) */
    val percentage: Int get() = (fraction * 100).toInt()
}

/** Rust 压缩结果 (JSON 反序列化) */
@Serializable
data class CompressResultFfi(
    @SerialName("original_size") val originalSize: Long = 0,
    @SerialName("compressed_size") val compressedSize: Long = 0,
)

/** Rust 解压结果 (JSON 反序列化) */
@Serializable
data class DecompressResultFfi(
    @SerialName("file_count") val fileCount: Int = 0,
)

/** 压缩完成结果 — UI 层展示用 */
data class CompressResult(
    val success: Boolean,
    val outputPath: String = "",
    val originalSize: Long = 0,
    val compressedSize: Long = 0,
    val errorMessage: String? = null,
) {
    /** 压缩比 (compressedSize / originalSize)，越小越好 */
    val compressionRatio: Float
        get() = if (originalSize > 0) compressedSize.toFloat() / originalSize else 0f
}

/** 文件名映射条目 — 从 Rust obfuscateFilenames 返回的 JSON 反序列化 */
@Serializable
data class FfiMappingEntry(
    @SerialName("original_name") val originalName: String,
    @SerialName("obfuscated_name") val obfuscatedName: String,
)

/** 照片扫描结果 (JSON 反序列化) */
@Serializable
data class PhotoScanResult(
    @SerialName("total_files") val totalFiles: Int = 0,
    @SerialName("new_files") val newFiles: Int = 0,
    @SerialName("transfer_bytes") val transferBytes: Long = 0,
    @SerialName("skipped_files") val skippedFiles: Int = 0,
    @SerialName("deleted_files") val deletedFiles: Int = 0,
)

/** 照片同步统计 (JSON 反序列化) */
@Serializable
data class PhotoSyncStats(
    @SerialName("total_backed_up") val totalBackedUp: Int = 0,
    @SerialName("total_bytes") val totalBytes: Long = 0,
    @SerialName("saved_bytes") val savedBytes: Long = 0,
    @SerialName("last_sync") val lastSync: String? = null,
)
