package com.sezip.sezip.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import com.sezip.sezip.data.PreferencesManager
import com.sezip.sezip.ui.theme.ThemeMode
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

class SettingsViewModel(application: Application) : AndroidViewModel(application) {
    private val prefs = PreferencesManager(application)

    private val _themeMode = MutableStateFlow(ThemeMode.entries[prefs.themeModeIndex.coerceIn(0, 2)])
    val themeMode: StateFlow<ThemeMode> = _themeMode.asStateFlow()

    private val _compressionLevel = MutableStateFlow(prefs.compressionLevel.coerceIn(1, 22))
    val compressionLevel: StateFlow<Int> = _compressionLevel.asStateFlow()

    private val _defaultScheme = MutableStateFlow(prefs.defaultObfuscationScheme)
    val defaultScheme: StateFlow<String> = _defaultScheme.asStateFlow()

    private val _outputDir = MutableStateFlow(prefs.getEffectiveOutputDir())
    val outputDir: StateFlow<String> = _outputDir.asStateFlow()

    private val _decompressDir = MutableStateFlow(prefs.getEffectiveDecompressDir())
    val decompressDir: StateFlow<String> = _decompressDir.asStateFlow()

    fun setThemeMode(mode: ThemeMode) {
        _themeMode.value = mode
        prefs.themeModeIndex = mode.ordinal
    }

    fun setCompressionLevel(level: Int) {
        val clamped = level.coerceIn(1, 22)
        _compressionLevel.value = clamped
        prefs.compressionLevel = clamped
    }

    fun setDefaultScheme(scheme: String) {
        _defaultScheme.value = scheme
        prefs.defaultObfuscationScheme = scheme
    }

    fun setOutputDir(dir: String) {
        _outputDir.value = dir
        prefs.outputDir = dir
    }

    fun setDecompressDir(dir: String) {
        _decompressDir.value = dir
        prefs.decompressOutputDir = dir
    }

    fun resetOutputDirs() {
        prefs.resetOutputDirs()
        _outputDir.value = PreferencesManager.DEFAULT_COMPRESS_DIR
        _decompressDir.value = PreferencesManager.DEFAULT_DECOMPRESS_DIR
    }

    /** Rust 库版本（同步调用，缓存结果） */
    val rustVersion: String by lazy {
        try { com.sezip.sezip.RustBridge.getVersion() } catch (_: Exception) { "N/A" }
    }
}
