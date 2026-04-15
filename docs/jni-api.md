# SecureZip JNI API 参考

## 概述

所有 JNI 函数定义在 `RustBridge.kt` (Kotlin 侧) 和 `api.rs` + `lib.rs` (Rust 侧)。
Kotlin 通过 `System.loadLibrary("sz_ffi")` 加载 Rust 编译的 `libsz_ffi.so`。

共 **31 个 external fun**，按功能分为 9 组。


## ProgressCallback 接口

```kotlin
interface ProgressCallback {
    fun onProgress(current: Long, total: Long, currentFile: String?)
}
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `current` | `Long` | 已处理字节数 |
| `total` | `Long` | 总字节数 (0 表示未知) |
| `currentFile` | `String?` | 当前正在处理的文件名 (可为 null) |

Rust 侧通过 JNI `env.call_method(callback, "onProgress", "(JJLjava/lang/String;)V", ...)` 回调。
建议在 ViewModel 侧限制 UI 更新频率 (200ms 间隔)，避免过度刷新。


## CancelToken 生命周期管理

CancelToken 是 Rust 侧 `Arc<AtomicBool>` 的句柄，通过 `jlong` (指针值) 传递给 Kotlin。

### 正确使用方式

```kotlin
var handle = 0L
try {
    handle = RustBridge.cancelTokenNew()     // 1. 创建
    RustBridge.compressZbak(..., handle, ...) // 2. 传入操作
} catch (e: Exception) {
    // 处理错误
} finally {
    if (handle != 0L) {
        RustBridge.cancelTokenFree(handle)   // 3. 必须释放
        handle = 0
    }
}
```

### 取消操作

```kotlin
// 用户点击取消按钮
RustBridge.cancelTokenCancel(handle)  // 设置 AtomicBool = true

// Rust 侧在循环中检查:
// if cancel_token.is_cancelled() { return Err("用户取消") }
```

### 注意事项

- `cancelTokenNew()` 和 `cancelTokenFree()` 必须配对调用，否则内存泄漏
- 一个 CancelToken 只能绑定一次操作，不可复用
- `cancelTokenFree()` 后不可再使用该 handle (use-after-free)
- 在 ViewModel 的 `onCleared()` 中自动取消并释放是最安全的做法


---


## 1. CancelToken 管理 (3 个函数)

### cancelTokenNew

```kotlin
external fun cancelTokenNew(): Long
```

创建新的取消令牌。

| 返回值 | 类型 | 说明 |
|--------|------|------|
| handle | `Long` | CancelToken 句柄 (非零值) |

### cancelTokenCancel

```kotlin
external fun cancelTokenCancel(handle: Long)
```

请求取消操作。设置内部 `AtomicBool` 为 `true`。

| 参数 | 类型 | 说明 |
|------|------|------|
| `handle` | `Long` | cancelTokenNew 返回的句柄 |

### cancelTokenFree

```kotlin
external fun cancelTokenFree(handle: Long)
```

释放取消令牌的内存。调用后 handle 失效。

| 参数 | 类型 | 说明 |
|------|------|------|
| `handle` | `Long` | cancelTokenNew 返回的句柄 |


---


## 2. .zbak 备份 API (6 个函数)

### compressZbak

```kotlin
external fun compressZbak(
    inputPaths: Array<String>,
    outputPath: String,
    password: String?,
    level: Int,
    encryptFilenames: Boolean,
    enableRecovery: Boolean,
    recoveryRatio: Float,
    splitSize: Long,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

使用 zbak 格式压缩文件。支持加密、文件名加密、恢复记录、分卷。

| 参数 | 类型 | 说明 |
|------|------|------|
| `inputPaths` | `Array<String>` | 待压缩的文件/目录路径列表 |
| `outputPath` | `String` | 输出 .zbak 文件路径 |
| `password` | `String?` | 加密密码 (null = 不加密) |
| `level` | `Int` | Zstd 压缩级别 (1-22, 推荐 6) |
| `encryptFilenames` | `Boolean` | 是否加密索引区的文件名 |
| `enableRecovery` | `Boolean` | 是否生成 Reed-Solomon 恢复记录 |
| `recoveryRatio` | `Float` | 恢复记录冗余比例 (0.05 / 0.10 / 0.20) |
| `splitSize` | `Long` | 分卷大小 (字节, 0 = 不分卷) |
| `cancelHandle` | `Long` | CancelToken 句柄 |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON 字符串

```json
{
  "original_size": 104857600,
  "compressed_size": 52428800
}
```

**异常**: `RuntimeException` (密码为空但源文件需要加密、路径无效、磁盘空间不足等)

### decompressZbak

```kotlin
external fun decompressZbak(
    archivePath: String,
    outputDir: String,
    password: String?,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

解压 .zbak 文件到指定目录。

| 参数 | 类型 | 说明 |
|------|------|------|
| `archivePath` | `String` | .zbak 文件路径 |
| `outputDir` | `String` | 输出目录 |
| `password` | `String?` | 解密密码 (null = 无加密) |
| `cancelHandle` | `Long` | CancelToken 句柄 |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON 字符串

```json
{
  "file_count": 42
}
```

### listZbakContents

```kotlin
external fun listZbakContents(archivePath: String, password: String?): String
```

列出 .zbak 文件内容 (不解压)。如果文件名已加密，需要提供密码。

| 参数 | 类型 | 说明 |
|------|------|------|
| `archivePath` | `String` | .zbak 文件路径 |
| `password` | `String?` | 密码 (仅文件名加密时需要) |

**返回值**: JSON 字符串数组

```json
["docs/readme.txt", "images/photo.jpg", "data/config.json"]
```

### extractZbakFile

```kotlin
external fun extractZbakFile(
    archivePath: String,
    filePath: String,
    outputPath: String,
    password: String?,
)
```

从 .zbak 中提取单个文件 (无需解压整个压缩包)。

| 参数 | 类型 | 说明 |
|------|------|------|
| `archivePath` | `String` | .zbak 文件路径 |
| `filePath` | `String` | 压缩包内的文件路径 (从 listZbakContents 获取) |
| `outputPath` | `String` | 输出文件路径 |
| `password` | `String?` | 解密密码 |

**返回值**: 无 (成功时无返回值，失败时抛出异常)

### zbakRequiresPassword

```kotlin
external fun zbakRequiresPassword(archivePath: String): Boolean
```

检查 .zbak 文件是否需要密码。通过读取文件头的 FLAG_ENCRYPTED 标志位判断。

| 参数 | 类型 | 说明 |
|------|------|------|
| `archivePath` | `String` | .zbak 文件路径 |

**返回值**: `true` = 需要密码, `false` = 无加密

### zbakVerifyPassword

```kotlin
external fun zbakVerifyPassword(archivePath: String, password: String): Boolean
```

验证 .zbak 文件的密码是否正确。使用文件头中的验证块 (GCM tag) 进行快速验证，
无需解压任何数据，耗时仅取决于 Argon2id 密钥派生 (约 200-500ms)。

| 参数 | 类型 | 说明 |
|------|------|------|
| `archivePath` | `String` | .zbak 文件路径 |
| `password` | `String` | 待验证的密码 |

**返回值**: `true` = 密码正确, `false` = 密码错误


---


## 3. 智能解压 API (4 个函数)

### smartDecompress

```kotlin
external fun smartDecompress(
    archivePath: String,
    outputDir: String,
    password: String?,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

自动检测压缩包格式并解压。支持 .zbak / .sz7z / .7z / .szp (旧版)。

| 参数 | 类型 | 说明 |
|------|------|------|
| `archivePath` | `String` | 压缩包路径 |
| `outputDir` | `String` | 输出目录 |
| `password` | `String?` | 密码 |
| `cancelHandle` | `Long` | CancelToken 句柄 |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON `{"file_count": N}`

### detectFormat

```kotlin
external fun detectFormat(archivePath: String): String
```

检测压缩包格式，通过读取文件头魔数判断。

| 参数 | 类型 | 说明 |
|------|------|------|
| `archivePath` | `String` | 压缩包路径 |

**返回值**: 格式名称字符串

| 返回值 | 格式 | 魔数 |
|--------|------|------|
| `"zbak"` | .zbak 格式 | `ZBAK` (0x5A42414B) |
| `"sz7z"` | 旧版 .sz7z | `SZ7Z` |
| `"7z"` | 标准 7z | `7z\xBC\xAF\x27\x1C` |
| `"szp"` | 旧版 .szp | `SZPK` |
| `"unknown"` | 未知格式 | - |

### smartRequiresPassword

```kotlin
external fun smartRequiresPassword(archivePath: String): Boolean
```

自动检测格式后判断是否需要密码。

### smartVerifyPassword

```kotlin
external fun smartVerifyPassword(archivePath: String, password: String): Boolean
```

自动检测格式后验证密码。


---


## 4. 标准 7z API (3 个函数)

### compress7z

```kotlin
external fun compress7z(
    inputPaths: Array<String>,
    outputPath: String,
    password: String?,
    level: Int,
    callback: ProgressCallback,
): String
```

使用标准 7z 格式 (LZMA2) 压缩。不支持取消操作。

| 参数 | 类型 | 说明 |
|------|------|------|
| `inputPaths` | `Array<String>` | 待压缩路径列表 |
| `outputPath` | `String` | 输出 .7z 文件路径 |
| `password` | `String?` | 加密密码 (使用 7z 原生 AES-256) |
| `level` | `Int` | 压缩级别 (1-9) |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON `{"original_size": N, "compressed_size": N}`

### decompress7z

```kotlin
external fun decompress7z(
    archivePath: String,
    outputDir: String,
    password: String?,
    callback: ProgressCallback,
): String
```

解压标准 7z 文件。

**返回值**: JSON `{"file_count": N}`

### list7zContents

```kotlin
external fun list7zContents(archivePath: String): String
```

列出 7z 文件内容。

**返回值**: JSON 字符串数组 `["file1.txt", "dir/file2.txt"]`


---


## 5. 旧版 .sz7z API (3 个函数)

这些 API 用于向后兼容旧版 .sz7z 格式 (7z + Zstd + 可选 AES-GCM)。

### compressLegacy

```kotlin
external fun compressLegacy(
    inputPaths: Array<String>,
    outputPath: String,
    level: Int,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

旧版 .sz7z 无加密压缩。

**返回值**: JSON `{"original_size": N, "compressed_size": N}`

### compressLegacyEncrypted

```kotlin
external fun compressLegacyEncrypted(
    inputPaths: Array<String>,
    outputPath: String,
    password: String,
    level: Int,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

旧版 .sz7z 加密压缩。注意 `password` 为非空 `String` (非 `String?`)。

**返回值**: JSON `{"original_size": N, "compressed_size": N}`

### verifyLegacyPassword

```kotlin
external fun verifyLegacyPassword(archivePath: String, password: String): Boolean
```

验证旧版 .sz7z 文件的密码。


---


## 6. WebDAV API (4 个函数)

### webdavTestConnection

```kotlin
external fun webdavTestConnection(url: String, username: String, password: String): Boolean
```

测试 WebDAV 服务器连接。

| 参数 | 类型 | 说明 |
|------|------|------|
| `url` | `String` | WebDAV 服务器 URL (含路径) |
| `username` | `String` | 用户名 |
| `password` | `String` | WebDAV 密码 |

**返回值**: `true` = 连接成功, `false` = 失败

### webdavBackup

```kotlin
external fun webdavBackup(
    inputPaths: Array<String>,
    url: String,
    username: String,
    webdavPassword: String,
    encryptPassword: String?,
    level: Int,
    recoveryRatio: Float,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

流式压缩并上传到 WebDAV。数据按 50MB 分块，压缩完一块立即上传并释放内存。

| 参数 | 类型 | 说明 |
|------|------|------|
| `inputPaths` | `Array<String>` | 待备份路径列表 |
| `url` | `String` | WebDAV 服务器 URL |
| `username` | `String` | WebDAV 用户名 |
| `webdavPassword` | `String` | WebDAV 密码 |
| `encryptPassword` | `String?` | 加密密码 (null = 不加密) |
| `level` | `Int` | Zstd 压缩级别 |
| `recoveryRatio` | `Float` | 恢复记录比例 (0 = 不启用) |
| `cancelHandle` | `Long` | CancelToken 句柄 |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON — BackupManifest (包含 backup_id, chunk 列表, 校验和等)

### webdavRestore

```kotlin
external fun webdavRestore(
    backupId: String,
    outputDir: String,
    url: String,
    username: String,
    webdavPassword: String,
    encryptPassword: String?,
    callback: ProgressCallback,
): String
```

从 WebDAV 恢复备份。

| 参数 | 类型 | 说明 |
|------|------|------|
| `backupId` | `String` | 备份 ID (从 webdavListBackups 获取) |
| `outputDir` | `String` | 本地输出目录 |
| `url` | `String` | WebDAV 服务器 URL |
| `username` | `String` | WebDAV 用户名 |
| `webdavPassword` | `String` | WebDAV 密码 |
| `encryptPassword` | `String?` | 解密密码 |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON `{"file_count": N}`

### webdavListBackups

```kotlin
external fun webdavListBackups(url: String, username: String, password: String): String
```

列出 WebDAV 上所有备份。

**返回值**: JSON 数组 — BackupManifest 列表


---


## 7. 加密工具 API (4 个函数)

### encryptString

```kotlin
external fun encryptString(data: String, password: String): String
```

加密字符串。用于密码本等敏感数据的本地存储加密。

内部流程: 生成随机 Salt → Argon2id 派生密钥 → AES-256-GCM 加密 → Base64 编码。

| 参数 | 类型 | 说明 |
|------|------|------|
| `data` | `String` | 待加密的明文 |
| `password` | `String` | 加密密码 |

**返回值**: `String` — 格式为 `<salt_base64>:<encrypted_base64>`

### decryptString

```kotlin
external fun decryptString(encryptedData: String, password: String): String
```

解密字符串。

| 参数 | 类型 | 说明 |
|------|------|------|
| `encryptedData` | `String` | encryptString 的返回值 |
| `password` | `String` | 加密时使用的密码 |

**返回值**: 原始明文字符串

**异常**: 密码错误或数据损坏时抛出 `RuntimeException`

### generateRandomPassword

```kotlin
external fun generateRandomPassword(length: Int, includeSymbols: Boolean): String
```

生成密码学安全的随机密码。保证包含每个字符类别至少一个字符。

| 参数 | 类型 | 说明 |
|------|------|------|
| `length` | `Int` | 密码长度 |
| `includeSymbols` | `Boolean` | 是否包含特殊字符 (`!@#$%^&*...`) |

**返回值**: 随机密码字符串

字符集:
- 小写字母: `a-z`
- 大写字母: `A-Z`
- 数字: `0-9`
- 特殊字符 (可选): `!@#$%^&*()-_=+[]{}|;:,.<>?`

### calculatePasswordStrength

```kotlin
external fun calculatePasswordStrength(password: String): Int
```

计算密码强度评分。

| 参数 | 类型 | 说明 |
|------|------|------|
| `password` | `String` | 待评估的密码 |

**返回值**: 0-4 的整数

| 分值 | 强度 | 典型示例 |
|------|------|----------|
| 0 | 非常弱 | `123`, `abc` |
| 1 | 弱 | `password` |
| 2 | 中等 | `Pass1234` |
| 3 | 强 | `Password1` |
| 4 | 非常强 | `MyP@ssw0rd!Long` |

评分规则:
- 长度 >= 8: +1 分
- 长度 >= 12: +1 分
- 包含小写字母: +1 分
- 包含大写字母: +1 分
- 包含数字: +1 分
- 包含特殊字符: +1 分
- 总分 (0-6) 映射到 0-4 范围


---


## 8. 文件名混淆 API (1 个函数)

### obfuscateFilenames

```kotlin
external fun obfuscateFilenames(
    originalNames: Array<String>,
    scheme: Int,
    archivePath: String,
): String
```

批量混淆文件名。

| 参数 | 类型 | 说明 |
|------|------|------|
| `originalNames` | `Array<String>` | 原始文件名列表 |
| `scheme` | `Int` | 混淆方案编号 (见下表) |
| `archivePath` | `String` | 压缩包路径 (某些方案用于生成上下文信息) |

混淆方案:

| 编号 | 方案 | 示例 |
|------|------|------|
| 0 | 顺序编号 | `file_001`, `file_002`, ... |
| 1 | 日期编号 | `20260415_001`, `20260415_002`, ... |
| 2 | 随机字符 | `a3f8b2c1`, `x7e9d4f0`, ... |
| 3 | 哈希 | SHA256(原始名) 前 8 位 |
| 4 | 加密 | AES 加密后的文件名 |

**返回值**: JSON 数组

```json
[
  {"original_name": "photo.jpg", "obfuscated_name": "file_001.jpg"},
  {"original_name": "document.pdf", "obfuscated_name": "file_002.pdf"}
]
```


---


## 9. 照片增量备份 API (4 个函数)

### photoScanIncremental

```kotlin
external fun photoScanIncremental(
    directories: Array<String>,
    indexPath: String,
    includeVideos: Boolean,
): String
```

扫描照片目录，与已有索引比较，计算增量。

| 参数 | 类型 | 说明 |
|------|------|------|
| `directories` | `Array<String>` | 待扫描的目录列表 |
| `indexPath` | `String` | 同步索引文件路径 (首次使用时自动创建) |
| `includeVideos` | `Boolean` | 是否包含视频文件 |

**返回值**: JSON

```json
{
  "total_files": 1200,
  "new_files": 45,
  "transfer_bytes": 157286400,
  "skipped_files": 1150,
  "deleted_files": 5
}
```

### photoBackupIncremental

```kotlin
external fun photoBackupIncremental(
    directories: Array<String>,
    outputPath: String,
    indexPath: String,
    password: String?,
    exifStripLevel: Int,
    includeVideos: Boolean,
    compressionLevel: Int,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

执行照片增量备份到本地 .zbak 文件。

| 参数 | 类型 | 说明 |
|------|------|------|
| `directories` | `Array<String>` | 照片目录列表 |
| `outputPath` | `String` | 输出 .zbak 路径 |
| `indexPath` | `String` | 同步索引路径 |
| `password` | `String?` | 加密密码 |
| `exifStripLevel` | `Int` | EXIF 处理级别 (0-3, 见安全文档) |
| `includeVideos` | `Boolean` | 是否包含视频 |
| `compressionLevel` | `Int` | Zstd 压缩级别 |
| `cancelHandle` | `Long` | CancelToken 句柄 |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON `{"original_size": N, "compressed_size": N}`

自动行为:
- 文件名加密: 始终启用
- 恢复记录: 始终启用 (5%)
- 备份完成后自动更新同步索引
- 无新照片时返回 `original_size: 0, compressed_size: 0`

### photoBackupToWebdav

```kotlin
external fun photoBackupToWebdav(
    directories: Array<String>,
    indexPath: String,
    url: String,
    username: String,
    webdavPassword: String,
    encryptPassword: String?,
    exifStripLevel: Int,
    includeVideos: Boolean,
    compressionLevel: Int,
    cancelHandle: Long,
    callback: ProgressCallback,
): String
```

照片增量备份到 WebDAV 服务器。

| 参数 | 类型 | 说明 |
|------|------|------|
| `directories` | `Array<String>` | 照片目录列表 |
| `indexPath` | `String` | 同步索引路径 |
| `url` | `String` | WebDAV URL |
| `username` | `String` | WebDAV 用户名 |
| `webdavPassword` | `String` | WebDAV 密码 |
| `encryptPassword` | `String?` | 加密密码 |
| `exifStripLevel` | `Int` | EXIF 处理级别 |
| `includeVideos` | `Boolean` | 是否包含视频 |
| `compressionLevel` | `Int` | Zstd 压缩级别 |
| `cancelHandle` | `Long` | CancelToken 句柄 |
| `callback` | `ProgressCallback` | 进度回调 |

**返回值**: JSON — BackupManifest (无新照片时返回 `"{}"`)

远程路径固定为 `/photo_backups/`。

### photoGetSyncStats

```kotlin
external fun photoGetSyncStats(indexPath: String): String
```

获取照片备份统计信息。

| 参数 | 类型 | 说明 |
|------|------|------|
| `indexPath` | `String` | 同步索引文件路径 |

**返回值**: JSON

```json
{
  "total_backed_up": 3500,
  "total_bytes": 15728640000,
  "saved_bytes": 4718592000,
  "last_sync": "2026-04-15T10:30:00+08:00"
}
```


---


## 10. 工具 API (2 个函数)

### initLogger

```kotlin
external fun initLogger()
```

初始化 Rust 日志系统 (env_logger)。应在 Application 启动时调用一次。
多次调用是安全的 (内部使用 `try_init`)。

### getVersion

```kotlin
external fun getVersion(): String
```

返回 Rust 库的版本号 (来自 `Cargo.toml` 的 `version` 字段)。

**返回值**: 版本字符串，如 `"0.1.0"`


---


## JSON 返回值类型汇总

| 类型名 | 字段 | 使用位置 |
|--------|------|----------|
| `CompressResultFfi` | `original_size: Long`, `compressed_size: Long` | compressZbak, compress7z, compressLegacy, photoBackupIncremental |
| `DecompressResultFfi` | `file_count: Int` | decompressZbak, smartDecompress, decompress7z, webdavRestore |
| `FfiMappingEntry[]` | `original_name: String`, `obfuscated_name: String` | obfuscateFilenames |
| `PhotoScanResult` | `total_files`, `new_files`, `transfer_bytes`, `skipped_files`, `deleted_files` | photoScanIncremental |
| `PhotoSyncStats` | `total_backed_up`, `total_bytes`, `saved_bytes`, `last_sync` | photoGetSyncStats |
| `String[]` | 文件路径列表 | listZbakContents, list7zContents |
| `BackupManifest` | backup_id, chunks, checksums | webdavBackup, webdavRestore, webdavListBackups, photoBackupToWebdav |

所有 JSON 均使用 snake_case 字段名。Kotlin 侧通过 `kotlinx.serialization` 的 `@SerialName` 注解映射到 camelCase。
