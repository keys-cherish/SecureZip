package com.sezip.sezip.model

import kotlinx.serialization.Serializable

/** WebDAV 服务器配置 */
@Serializable
data class WebDavConfig(
    val serverUrl: String = "",
    val username: String = "",
    val password: String = "",
    val remotePath: String = "/",
) {
    /** 是否已完成最基本的配置（URL + 用户名） */
    val isConfigured: Boolean get() = serverUrl.isNotBlank() && username.isNotBlank()
}

/** WebDAV 远程文件信息 */
data class WebDavFileInfo(
    val name: String,
    val path: String,
    val isDirectory: Boolean,
    val size: Long = 0,
    val lastModified: Long = 0,
)

/** WebDAV 备份记录 */
data class WebDavBackupInfo(
    val fileName: String,
    val size: Long,
    val backupDate: Long,
)
