package com.sezip.sezip.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.sezip.sezip.RustBridge
import com.sezip.sezip.data.PreferencesManager
import com.sezip.sezip.model.CompressProgress
import com.sezip.sezip.model.DecompressResultFfi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json

/** 解压页面状态 */
sealed class DecompressState {
    data object SelectFile : DecompressState()
    data object Ready : DecompressState()
    data object Running : DecompressState()
    data class Completed(val fileCount: Int, val outputDir: String) : DecompressState()
    data class Error(val message: String) : DecompressState()
}

class DecompressViewModel(application: Application) : AndroidViewModel(application) {
    private val prefs = PreferencesManager(application)
    private val json = Json { ignoreUnknownKeys = true }

    private val _state = MutableStateFlow<DecompressState>(DecompressState.SelectFile)
    val state: StateFlow<DecompressState> = _state.asStateFlow()

    private val _archivePath = MutableStateFlow("")
    val archivePath: StateFlow<String> = _archivePath.asStateFlow()

    private val _detectedFormat = MutableStateFlow("")
    val detectedFormat: StateFlow<String> = _detectedFormat.asStateFlow()

    private val _requiresPassword = MutableStateFlow(false)
    val requiresPassword: StateFlow<Boolean> = _requiresPassword.asStateFlow()

    private val _password = MutableStateFlow("")
    val password: StateFlow<String> = _password.asStateFlow()

    private val _outputDir = MutableStateFlow(prefs.getEffectiveDecompressDir())
    val outputDir: StateFlow<String> = _outputDir.asStateFlow()

    private val _contents = MutableStateFlow<List<String>>(emptyList())
    val contents: StateFlow<List<String>> = _contents.asStateFlow()

    private val _progress = MutableStateFlow(CompressProgress())
    val progress: StateFlow<CompressProgress> = _progress.asStateFlow()

    private var cancelHandle: Long = 0

    /** 选择归档文件后分析 */
    fun setArchivePath(path: String) {
        _archivePath.value = path
        _state.value = DecompressState.Ready

        viewModelScope.launch(Dispatchers.IO) {
            try {
                _detectedFormat.value = RustBridge.detectFormat(path)
                _requiresPassword.value = RustBridge.smartRequiresPassword(path)
            } catch (_: Throwable) { }

            // 尝试列出内容（zbak 专用，7z 等格式会失败但不影响）
            try {
                val contentsJson = RustBridge.listZbakContents(path, null)
                _contents.value = json.decodeFromString<List<String>>(contentsJson)
            } catch (_: Throwable) { }

            // zbak 失败后尝试 7z 列表
            if (_contents.value.isEmpty()) {
                try {
                    val contentsJson = RustBridge.list7zContents(path)
                    _contents.value = json.decodeFromString<List<String>>(contentsJson)
                } catch (_: Throwable) { }
            }
        }
    }

    fun setPassword(pwd: String) { _password.value = pwd }
    fun setOutputDir(dir: String) { _outputDir.value = dir }

    /** 验证密码 */
    fun verifyPassword(onResult: (Boolean) -> Unit) {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val ok = RustBridge.smartVerifyPassword(_archivePath.value, _password.value)
                onResult(ok)
            } catch (_: Exception) {
                onResult(false)
            }
        }
    }

    /** 开始解压 */
    fun startDecompress() {
        if (_state.value is DecompressState.Running) return

        if (!RustBridge.isAvailable) {
            _state.value = DecompressState.Error("Rust 压缩引擎不可用，请重新编译 .so 库")
            return
        }

        _state.value = DecompressState.Running
        _progress.value = CompressProgress()

        val outputDir = _outputDir.value
        com.sezip.sezip.util.FileUtils.ensureDirectory(outputDir)

        viewModelScope.launch(Dispatchers.IO) {
            cancelHandle = RustBridge.cancelTokenNew()
            try {
                val resultJson = RustBridge.smartDecompress(
                    _archivePath.value, outputDir,
                    _password.value.ifBlank { null },
                    cancelHandle,
                    object : RustBridge.ProgressCallback {
                        override fun onProgress(current: Long, total: Long, currentFile: String?) {
                            _progress.value = CompressProgress(current, total, currentFile)
                        }
                    }
                )
                val result = json.decodeFromString<DecompressResultFfi>(resultJson)
                _state.value = DecompressState.Completed(result.fileCount, outputDir)
            } catch (e: Exception) {
                _state.value = DecompressState.Error(e.message ?: "解压失败")
            } finally {
                if (cancelHandle != 0L) {
                    RustBridge.cancelTokenFree(cancelHandle)
                    cancelHandle = 0
                }
            }
        }
    }

    fun requestCancel() {
        if (cancelHandle != 0L) RustBridge.cancelTokenCancel(cancelHandle)
    }

    fun reset() {
        _state.value = DecompressState.SelectFile
        _archivePath.value = ""
        _detectedFormat.value = ""
        _requiresPassword.value = false
        _password.value = ""
        _contents.value = emptyList()
        _progress.value = CompressProgress()
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
