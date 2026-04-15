package com.sezip.sezip.util

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import java.io.File

object FileUtils {

    /** 从 content:// URI 获取真实文件路径（SAF） */
    fun uriToRealPath(uri: Uri): String? {
        val path = uri.path ?: return null
        // SAF URI 格式: /document/primary:path/to/file → /storage/emulated/0/path/to/file
        // tree URI 格式: /tree/primary:path/to/dir → /storage/emulated/0/path/to/dir
        val segments = path.split(":")
        if (segments.size >= 2) {
            val relativePath = segments.last()
            return "/storage/emulated/0/$relativePath"
        }
        // 已经是真实路径
        if (File(path).exists()) return path
        return null
    }

    /** 从 content:// URI 列表获取真实路径 */
    fun urisToRealPaths(uris: List<Uri>): List<String> {
        return uris.mapNotNull { uriToRealPath(it) }
    }

    /** 通过 ContentResolver 查询 URI 的显示名和大小 */
    data class UriFileInfo(val displayName: String, val size: Long)

    fun queryUriInfo(context: Context, uri: Uri): UriFileInfo {
        var name = "unknown"
        var size = 0L
        try {
            context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
                if (cursor.moveToFirst()) {
                    val nameIdx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    val sizeIdx = cursor.getColumnIndex(OpenableColumns.SIZE)
                    if (nameIdx >= 0) name = cursor.getString(nameIdx) ?: "unknown"
                    if (sizeIdx >= 0) size = cursor.getLong(sizeIdx)
                }
            }
        } catch (_: Exception) { }
        return UriFileInfo(name, size)
    }

    /** 获取文件/目录的总大小 */
    fun getTotalSize(paths: List<String>): Long {
        return paths.sumOf { path ->
            val file = File(path)
            if (file.isDirectory) {
                file.walkTopDown().filter { it.isFile }.sumOf { it.length() }
            } else {
                file.length()
            }
        }
    }

    /** 获取文件/目录中的文件数量 */
    fun getFileCount(paths: List<String>): Int {
        return paths.sumOf { path ->
            val file = File(path)
            if (file.isDirectory) {
                file.walkTopDown().filter { it.isFile }.count()
            } else {
                1
            }
        }
    }

    /** 从路径生成默认输出文件名 */
    fun generateOutputName(inputPaths: List<String>, extension: String): String {
        if (inputPaths.isEmpty()) return "archive$extension"
        val firstPath = File(inputPaths.first())
        val baseName = firstPath.nameWithoutExtension
        return "$baseName$extension"
    }

    /** 确保目录存在 */
    fun ensureDirectory(path: String): File {
        val dir = File(path)
        if (!dir.exists()) dir.mkdirs()
        return dir
    }
}
