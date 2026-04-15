package com.sezip.sezip.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.sezip.sezip.RustBridge
import com.sezip.sezip.data.PreferencesManager
import com.sezip.sezip.model.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import java.io.File

enum class BackupTarget(val displayName: String) {
    LOCAL("本地备份"),
    WEBDAV("WebDAV 上传");
}

class PhotoBackupViewModel(application: Application) : AndroidViewModel(application) {
    private val prefs = PreferencesManager(application)
    private val json = Json { ignoreUnknownKeys = true }

    private val _directories = MutableStateFlow<List<String>>(emptyList())
    val directories: StateFlow<List<String>> = _directories.asStateFlow()

    private val _includeVideos = MutableStateFlow(false)
    val includeVideos: StateFlow<Boolean> = _includeVideos.asStateFlow()

    private val _exifStripLevel = MutableStateFlow(1)
    val exifStripLevel: StateFlow<Int> = _exifStripLevel.asStateFlow()

    private val _password = MutableStateFlow("")
    val password: StateFlow<String> = _password.asStateFlow()

    private val _backupTarget = MutableStateFlow(BackupTarget.LOCAL)
    val backupTarget: StateFlow<BackupTarget> = _backupTarget.asStateFlow()

    private val _scanResult = MutableStateFlow<PhotoScanResult?>(null)
    val scanResult: StateFlow<PhotoScanResult?> = _scanResult.asStateFlow()

    private val _syncStats = MutableStateFlow<PhotoSyncStats?>(null)
    val syncStats: StateFlow<PhotoSyncStats?> = _syncStats.asStateFlow()

    private val _isScanning = MutableStateFlow(false)
    val isScanning: StateFlow<Boolean> = _isScanning.asStateFlow()

    private val _isBackingUp = MutableStateFlow(false)
    val isBackingUp: StateFlow<Boolean> = _isBackingUp.asStateFlow()

    private val _progress = MutableStateFlow(CompressProgress())
    val progress: StateFlow<CompressProgress> = _progress.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private var cancelHandle: Long = 0

    private val indexPath: String get() {
        val dir = getApplication<Application>().filesDir
        return File(dir, "photo_sync_index.json").absolutePath
    }

    fun setDirectories(dirs: List<String>) { _directories.value = dirs }
    fun setIncludeVideos(v: Boolean) { _includeVideos.value = v }
    fun setExifStripLevel(level: Int) { _exifStripLevel.value = level.coerceIn(0, 3) }
    fun setPassword(pwd: String) { _password.value = pwd }
    fun setBackupTarget(target: BackupTarget) { _backupTarget.value = target }

    fun scan() {
        val dirs = _directories.value
        if (dirs.isEmpty()) return

        _isScanning.value = true
        _error.value = null

        viewModelScope.launch(Dispatchers.IO) {
            try {
                val resultJson = RustBridge.photoScanIncremental(
                    dirs.toTypedArray(), indexPath, _includeVideos.value
                )
                _scanResult.value = json.decodeFromString<PhotoScanResult>(resultJson)
                loadStats()
            } catch (e: Exception) {
                _error.value = e.message
            } finally {
                _isScanning.value = false
            }
        }
    }

    fun startBackup() {
        if (_isBackingUp.value) return
        _isBackingUp.value = true
        _error.value = null

        viewModelScope.launch(Dispatchers.IO) {
            cancelHandle = RustBridge.cancelTokenNew()
            try {
                val callback = object : RustBridge.ProgressCallback {
                    override fun onProgress(current: Long, total: Long, currentFile: String?) {
                        _progress.value = CompressProgress(current, total, currentFile)
                    }
                }

                when (_backupTarget.value) {
                    BackupTarget.LOCAL -> {
                        val outputDir = prefs.getEffectiveOutputDir()
                        com.sezip.sezip.util.FileUtils.ensureDirectory(outputDir)
                        val outputPath = "$outputDir/photo_backup_${System.currentTimeMillis()}.zbak"

                        RustBridge.photoBackupIncremental(
                            _directories.value.toTypedArray(), outputPath, indexPath,
                            _password.value.ifBlank { null }, _exifStripLevel.value,
                            _includeVideos.value, prefs.compressionLevel,
                            cancelHandle, callback
                        )
                    }
                    BackupTarget.WEBDAV -> {
                        val cfg = json.decodeFromString<com.sezip.sezip.model.WebDavConfig>(prefs.webdavConfigJson)
                        RustBridge.photoBackupToWebdav(
                            _directories.value.toTypedArray(), indexPath,
                            cfg.serverUrl, cfg.username, cfg.password,
                            _password.value.ifBlank { null }, _exifStripLevel.value,
                            _includeVideos.value, prefs.compressionLevel,
                            cancelHandle, callback
                        )
                    }
                }
                loadStats()
            } catch (e: Exception) {
                _error.value = e.message
            } finally {
                if (cancelHandle != 0L) {
                    RustBridge.cancelTokenFree(cancelHandle)
                    cancelHandle = 0
                }
                _isBackingUp.value = false
            }
        }
    }

    fun requestCancel() {
        if (cancelHandle != 0L) RustBridge.cancelTokenCancel(cancelHandle)
    }

    private fun loadStats() {
        try {
            val statsJson = RustBridge.photoGetSyncStats(indexPath)
            _syncStats.value = json.decodeFromString<PhotoSyncStats>(statsJson)
        } catch (_: Exception) {}
    }

    override fun onCleared() {
        super.onCleared()
        if (cancelHandle != 0L) {
            RustBridge.cancelTokenCancel(cancelHandle)
            RustBridge.cancelTokenFree(cancelHandle)
            cancelHandle = 0
        }
    }
}
