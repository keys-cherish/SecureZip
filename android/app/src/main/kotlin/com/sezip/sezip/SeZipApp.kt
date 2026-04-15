package com.sezip.sezip

import android.app.Application
import android.util.Log

class SeZipApp : Application() {
    override fun onCreate() {
        super.onCreate()
        if (RustBridge.isAvailable) {
            try {
                RustBridge.initLogger()
            } catch (_: Exception) { }
        } else {
            Log.w("SeZipApp", "Rust 库不可用，压缩/解压功能将不可用")
        }
    }
}
