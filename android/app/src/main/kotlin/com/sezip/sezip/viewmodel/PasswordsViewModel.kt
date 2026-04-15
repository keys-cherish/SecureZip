package com.sezip.sezip.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import com.sezip.sezip.RustBridge
import com.sezip.sezip.data.PasswordRepository
import com.sezip.sezip.data.PreferencesManager
import com.sezip.sezip.model.PasswordEntry
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.UUID

class PasswordsViewModel(application: Application) : AndroidViewModel(application) {
    private val repo = PasswordRepository(PreferencesManager(application))

    private val _passwords = MutableStateFlow<List<PasswordEntry>>(emptyList())
    val passwords: StateFlow<List<PasswordEntry>> = _passwords.asStateFlow()

    init { reload() }

    private fun reload() { _passwords.value = repo.getAll() }

    fun add(name: String, password: String, note: String = "") {
        repo.add(PasswordEntry(
            id = UUID.randomUUID().toString(),
            name = name,
            password = password,
            note = note,
        ))
        reload()
    }

    fun update(entry: PasswordEntry) {
        repo.update(entry)
        reload()
    }

    fun delete(id: String) {
        repo.delete(id)
        reload()
    }

    /** 生成随机密码 */
    fun generatePassword(length: Int = 16, includeSymbols: Boolean = true): String {
        return try {
            RustBridge.generateRandomPassword(length, includeSymbols)
        } catch (_: Exception) {
            // Rust 不可用时使用 Kotlin 生成
            val chars = buildString {
                append("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")
                if (includeSymbols) append("!@#\$%^&*()_+-=[]{}|;:,.<>?")
            }
            (1..length).map { chars.random() }.joinToString("")
        }
    }

    /** 计算密码强度 (0-4) */
    fun passwordStrength(password: String): Int {
        return try {
            RustBridge.calculatePasswordStrength(password)
        } catch (_: Exception) {
            0
        }
    }

    /** 导出为 JSON */
    fun exportJson(): String {
        return jsonFormat.encodeToString(kotlinx.serialization.builtins.ListSerializer(PasswordEntry.serializer()), _passwords.value)
    }

    /** 从 JSON 导入 */
    fun importJson(jsonStr: String): Int {
        return try {
            val entries = jsonFormat.decodeFromString<List<PasswordEntry>>(jsonStr)
            val existing = repo.getAll().map { it.id }.toSet()
            var count = 0
            for (entry in entries) {
                if (entry.id !in existing) {
                    repo.add(entry)
                    count++
                }
            }
            reload()
            count
        } catch (_: Exception) {
            0
        }
    }

    companion object {
        /** 共享 Json 实例，避免重复创建 */
        private val jsonFormat = kotlinx.serialization.json.Json {
            prettyPrint = true
            ignoreUnknownKeys = true
        }
    }
}
