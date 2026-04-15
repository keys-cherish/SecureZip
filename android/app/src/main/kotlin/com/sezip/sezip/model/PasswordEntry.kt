package com.sezip.sezip.model

import kotlinx.serialization.Serializable

/** 密码本条目 — 用于管理用户保存的加密密码 */
@Serializable
data class PasswordEntry(
    val id: String,
    val name: String,
    val password: String,
    val createdAt: Long = System.currentTimeMillis(),
    val note: String = "",
)
