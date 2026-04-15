package com.sezip.sezip.viewmodel

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import com.sezip.sezip.data.PreferencesManager
import com.sezip.sezip.model.*
import com.sezip.sezip.util.FileUtils
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

class CompressViewModel(application: Application) : AndroidViewModel(application) {
    private val prefs = PreferencesManager(application)

    // 选中的文件真实路径
    private val _selectedPaths = MutableStateFlow<List<String>>(emptyList())
    val selectedPaths: StateFlow<List<String>> = _selectedPaths.asStateFlow()

    // 文件统计
    private val _fileCount = MutableStateFlow(0)
    val fileCount: StateFlow<Int> = _fileCount.asStateFlow()

    private val _totalSize = MutableStateFlow(0L)
    val totalSize: StateFlow<Long> = _totalSize.asStateFlow()

    // 输出文件名
    private val _outputName = MutableStateFlow("")
    val outputName: StateFlow<String> = _outputName.asStateFlow()

    // 压缩选项
    private val _compressMode = MutableStateFlow(CompressMode.ZBAK)
    val compressMode: StateFlow<CompressMode> = _compressMode.asStateFlow()

    private val _password = MutableStateFlow("")
    val password: StateFlow<String> = _password.asStateFlow()

    private val _compressionLevel = MutableStateFlow(prefs.compressionLevel.coerceIn(1, 22))
    val compressionLevel: StateFlow<Int> = _compressionLevel.asStateFlow()

    private val _encryptFilenames = MutableStateFlow(false)
    val encryptFilenames: StateFlow<Boolean> = _encryptFilenames.asStateFlow()

    private val _enableRecovery = MutableStateFlow(false)
    val enableRecovery: StateFlow<Boolean> = _enableRecovery.asStateFlow()

    private val _recoveryRatio = MutableStateFlow(RecoveryRatio.PERCENT_5)
    val recoveryRatio: StateFlow<RecoveryRatio> = _recoveryRatio.asStateFlow()

    private val _splitSize = MutableStateFlow(SplitSizePreset.NONE)
    val splitSize: StateFlow<SplitSizePreset> = _splitSize.asStateFlow()

    private val _enableObfuscation = MutableStateFlow(false)
    val enableObfuscation: StateFlow<Boolean> = _enableObfuscation.asStateFlow()

    private val _obfuscationType = MutableStateFlow(ObfuscationType.SEQUENTIAL)
    val obfuscationType: StateFlow<ObfuscationType> = _obfuscationType.asStateFlow()

    /** 从 SAF URI 列表设置选中文件（自动转真实路径 + 查大小） */
    fun setSelectedUris(uris: List<Uri>) {
        val context = getApplication<Application>()
        val paths = FileUtils.urisToRealPaths(uris)
        _selectedPaths.value = paths

        // 通过 ContentResolver 获取准确大小
        var totalSize = 0L
        var count = 0
        for (uri in uris) {
            val info = FileUtils.queryUriInfo(context, uri)
            totalSize += info.size
            count++
        }
        // 如果 ContentResolver 返回 0，回退到文件系统查询
        if (totalSize == 0L && paths.isNotEmpty()) {
            totalSize = FileUtils.getTotalSize(paths)
            count = FileUtils.getFileCount(paths)
        }
        _fileCount.value = count
        _totalSize.value = totalSize

        // 根据第一个文件名自动生成输出名
        if (paths.isNotEmpty() && _outputName.value.isBlank()) {
            val ext = if (_compressMode.value == CompressMode.LEGACY_7Z) ".7z" else ".zbak"
            _outputName.value = FileUtils.generateOutputName(paths, ext)
        }
    }

    /** 从文件夹 URI 设置（SAF OpenDocumentTree） */
    fun setSelectedFolderUri(uri: Uri) {
        val path = FileUtils.uriToRealPath(uri)
        if (path != null) {
            _selectedPaths.value = listOf(path)
            _fileCount.value = FileUtils.getFileCount(listOf(path))
            _totalSize.value = FileUtils.getTotalSize(listOf(path))
            if (_outputName.value.isBlank()) {
                val ext = if (_compressMode.value == CompressMode.LEGACY_7Z) ".7z" else ".zbak"
                _outputName.value = FileUtils.generateOutputName(listOf(path), ext)
            }
        }
    }

    fun setOutputName(name: String) { _outputName.value = name }
    fun setCompressMode(mode: CompressMode) {
        _compressMode.value = mode
        val ext = if (mode == CompressMode.LEGACY_7Z) ".7z" else ".zbak"
        val current = _outputName.value
        if (current.isNotBlank()) {
            val baseName = current.substringBeforeLast(".")
            _outputName.value = "$baseName$ext"
        }
    }
    fun setPassword(pwd: String) { _password.value = pwd }
    fun setCompressionLevel(level: Int) { _compressionLevel.value = level.coerceIn(1, 22) }
    fun setEncryptFilenames(v: Boolean) { _encryptFilenames.value = v }
    fun setEnableRecovery(v: Boolean) { _enableRecovery.value = v }
    fun setRecoveryRatio(r: RecoveryRatio) { _recoveryRatio.value = r }
    fun setSplitSize(s: SplitSizePreset) { _splitSize.value = s }
    fun setEnableObfuscation(v: Boolean) { _enableObfuscation.value = v }
    fun setObfuscationType(t: ObfuscationType) { _obfuscationType.value = t }

    fun buildOptions(): CompressOptions = CompressOptions(
        password = _password.value.ifBlank { null },
        compressionLevel = _compressionLevel.value,
        compressMode = _compressMode.value,
        encryptFilenames = _encryptFilenames.value,
        enableRecovery = _enableRecovery.value,
        recoveryRatio = _recoveryRatio.value,
        splitSize = _splitSize.value,
        enableObfuscation = _enableObfuscation.value,
        obfuscationType = _obfuscationType.value,
    )

    fun getOutputDir(): String = prefs.getEffectiveOutputDir()
}
