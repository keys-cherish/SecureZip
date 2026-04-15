package com.sezip.sezip.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.sezip.sezip.RustBridge
import com.sezip.sezip.data.PreferencesManager
import com.sezip.sezip.model.WebDavConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json

class WebDavViewModel(application: Application) : AndroidViewModel(application) {
    private val prefs = PreferencesManager(application)
    private val json = Json { ignoreUnknownKeys = true }

    private val _config = MutableStateFlow(loadConfig())
    val config: StateFlow<WebDavConfig> = _config.asStateFlow()

    private val _connectionState = MutableStateFlow<ConnectionState>(ConnectionState.Idle)
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private val _backups = MutableStateFlow<List<String>>(emptyList())
    val backups: StateFlow<List<String>> = _backups.asStateFlow()

    sealed class ConnectionState {
        data object Idle : ConnectionState()
        data object Testing : ConnectionState()
        data object Connected : ConnectionState()
        data class Failed(val message: String) : ConnectionState()
    }

    private fun loadConfig(): WebDavConfig {
        return try {
            json.decodeFromString<WebDavConfig>(prefs.webdavConfigJson)
        } catch (_: Exception) {
            WebDavConfig()
        }
    }

    fun updateConfig(config: WebDavConfig) {
        _config.value = config
        prefs.webdavConfigJson = json.encodeToString(WebDavConfig.serializer(), config)
    }

    fun testConnection() {
        val cfg = _config.value
        if (!cfg.isConfigured) return

        _connectionState.value = ConnectionState.Testing
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val ok = RustBridge.webdavTestConnection(cfg.serverUrl, cfg.username, cfg.password)
                _connectionState.value = if (ok) ConnectionState.Connected else ConnectionState.Failed("连接被拒绝")
            } catch (e: Exception) {
                _connectionState.value = ConnectionState.Failed(e.message ?: "连接失败")
            }
        }
    }

    fun loadBackups() {
        val cfg = _config.value
        if (!cfg.isConfigured) return

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val result = RustBridge.webdavListBackups(cfg.serverUrl, cfg.username, cfg.password)
                _backups.value = json.decodeFromString<List<String>>(result)
            } catch (_: Exception) {
                _backups.value = emptyList()
            }
        }
    }
}
