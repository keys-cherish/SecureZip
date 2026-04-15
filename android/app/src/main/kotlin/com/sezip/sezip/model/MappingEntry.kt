package com.sezip.sezip.model

import kotlinx.serialization.Serializable

/** 文件名混淆映射条目 — 记录原始文件名与混淆后文件名的对应关系 */
@Serializable
data class MappingEntry(
    val id: String,
    val originalName: String,
    val obfuscatedName: String,
    val archivePath: String,
    val createdAt: Long = System.currentTimeMillis(),
)

/** 扩展名-密码映射 — 将特定文件扩展名关联到密码本中的某个密码 */
@Serializable
data class ExtensionPasswordMapping(
    val id: String,
    val extension: String,
    val passwordId: String,
    val description: String = "",
    val createdAt: Long = System.currentTimeMillis(),
)
