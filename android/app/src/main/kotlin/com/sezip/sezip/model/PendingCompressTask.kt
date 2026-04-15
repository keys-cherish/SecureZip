package com.sezip.sezip.model

/**
 * 待执行的压缩任务（CompressScreen → CompressProgressScreen 传参用）
 *
 * Navigation Compose 不方便传复杂对象，用单例暂存。
 * CompressScreen 设置参数 → 导航 → CompressProgressScreen 取出并清空。
 */
object PendingCompressTask {
    var inputPaths: List<String> = emptyList()
    var outputDir: String = ""
    var outputName: String = ""
    var options: CompressOptions = CompressOptions()

    fun set(paths: List<String>, dir: String, name: String, opts: CompressOptions) {
        inputPaths = paths
        outputDir = dir
        outputName = name
        options = opts
    }

    fun consume(): Boolean {
        return inputPaths.isNotEmpty()
    }

    fun clear() {
        inputPaths = emptyList()
        outputDir = ""
        outputName = ""
        options = CompressOptions()
    }
}
