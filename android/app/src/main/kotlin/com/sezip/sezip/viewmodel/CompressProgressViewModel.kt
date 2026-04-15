package com.sezip.sezip.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.sezip.sezip.RustBridge
import com.sezip.sezip.model.*
import com.sezip.sezip.util.FileUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json

/** 压缩/解压执行状态 */
sealed class OperationState {
    data object Idle : OperationState()
    data object Running : OperationState()
    data class Completed(val result: CompressResult) : OperationState()
    data class Error(val message: String) : OperationState()
}

class CompressProgressViewModel(application: Application) : AndroidViewModel(application) {
    private val json = Json { ignoreUnknownKeys = true }

    private val _state = MutableStateFlow<OperationState>(OperationState.Idle)
    val state: StateFlow<OperationState> = _state.asStateFlow()

    private val _progress = MutableStateFlow(CompressProgress())
    val progress: StateFlow<CompressProgress> = _progress.asStateFlow()

    private var cancelHandle: Long = 0
    private var lastProgressTime = 0L
    private var lastProgressBytes = 0L

    /** 启动压缩任务 */
    fun startCompress(inputPaths: List<String>, outputDir: String, outputName: String, options: CompressOptions) {
        if (_state.value is OperationState.Running) return

        if (!RustBridge.isAvailable) {
            _state.value = OperationState.Error("Rust 压缩引擎不可用，请重新编译 .so 库")
            return
        }

        _state.value = OperationState.Running
        _progress.value = CompressProgress()

        val outputPath = "$outputDir/$outputName"
        FileUtils.ensureDirectory(outputDir)

        viewModelScope.launch(Dispatchers.IO) {
            cancelHandle = RustBridge.cancelTokenNew()
            try {
                val callback = createProgressCallback()
                val resultJson = when (options.compressMode) {
                    CompressMode.ZBAK -> RustBridge.compressZbak(
                        inputPaths.toTypedArray(), outputPath, options.password,
                        options.compressionLevel, options.encryptFilenames,
                        options.enableRecovery, options.recoveryRatio.value,
                        options.splitSize.bytes, cancelHandle, callback
                    )
                    CompressMode.ZBAK_WEBDAV -> RustBridge.webdavBackup(
                        inputPaths.toTypedArray(), options.webdavUrl,
                        options.webdavUsername, options.webdavPassword,
                        options.password, options.compressionLevel,
                        if (options.enableRecovery) options.recoveryRatio.value else 0f,
                        cancelHandle, callback
                    )
                    CompressMode.LEGACY_7Z -> RustBridge.compress7z(
                        inputPaths.toTypedArray(), outputPath, options.password,
                        options.compressionLevel, callback
                    )
                }

                val ffiResult = json.decodeFromString<CompressResultFfi>(resultJson)
                _state.value = OperationState.Completed(CompressResult(
                    success = true,
                    outputPath = outputPath,
                    originalSize = ffiResult.originalSize,
                    compressedSize = ffiResult.compressedSize,
                ))
            } catch (e: Exception) {
                _state.value = OperationState.Error(e.message ?: "压缩失败")
            } finally {
                if (cancelHandle != 0L) {
                    RustBridge.cancelTokenFree(cancelHandle)
                    cancelHandle = 0
                }
            }
        }
    }

    /** 请求取消 */
    fun requestCancel() {
        if (cancelHandle != 0L) {
            RustBridge.cancelTokenCancel(cancelHandle)
        }
    }

    private fun createProgressCallback(): RustBridge.ProgressCallback {
        lastProgressTime = System.currentTimeMillis()
        lastProgressBytes = 0
        return object : RustBridge.ProgressCallback {
            override fun onProgress(current: Long, total: Long, currentFile: String?) {
                val now = System.currentTimeMillis()
                val elapsed = now - lastProgressTime
                val speed = if (elapsed > 0) {
                    (current - lastProgressBytes) * 1000 / elapsed
                } else 0L

                if (elapsed > 200) { // 限制更新频率
                    lastProgressTime = now
                    lastProgressBytes = current
                }

                _progress.value = CompressProgress(
                    current = current,
                    total = total,
                    currentFile = currentFile,
                )
            }
        }
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
