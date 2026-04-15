package com.sezip.sezip.data

import com.sezip.sezip.model.PasswordEntry
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * 密码仓库
 *
 * 管理密码的 CRUD 操作，持久化到 SharedPreferences。
 */
class PasswordRepository(private val prefs: PreferencesManager) {
    private val json = Json { ignoreUnknownKeys = true }

    fun getAll(): List<PasswordEntry> {
        return try {
            json.decodeFromString<List<PasswordEntry>>(prefs.passwordsJson)
        } catch (e: Exception) {
            emptyList()
        }
    }

    fun save(entries: List<PasswordEntry>) {
        prefs.passwordsJson = json.encodeToString(entries)
    }

    fun add(entry: PasswordEntry) {
        val list = getAll().toMutableList()
        list.add(entry)
        save(list)
    }

    fun update(entry: PasswordEntry) {
        val list = getAll().toMutableList()
        val index = list.indexOfFirst { it.id == entry.id }
        if (index >= 0) {
            list[index] = entry
            save(list)
        }
    }

    fun delete(id: String) {
        val list = getAll().toMutableList()
        list.removeAll { it.id == id }
        save(list)
    }

    fun findById(id: String): PasswordEntry? = getAll().find { it.id == id }
}
