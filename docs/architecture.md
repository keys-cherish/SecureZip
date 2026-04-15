# SecureZip 架构文档

## 整体架构

SecureZip 采用 Kotlin + Jetpack Compose 构建 UI 层，核心压缩/加密逻辑由 Rust 实现，
两层之间通过 JNI (Java Native Interface) 桥接。

```
+--------------------------------------------------------------+
|                     Android Application                       |
+--------------------------------------------------------------+
|                                                              |
|  +--------------------------------------------------------+  |
|  |              Kotlin / Jetpack Compose                   |  |
|  |                                                        |  |
|  |  +----------+   +---------+   +---------------------+  |  |
|  |  | Screens  |-->|ViewModel|-->| StateFlow / State   |  |  |
|  |  | (11个)   |<--| (7个)   |<--| collectAsState()    |  |  |
|  |  +----------+   +---------+   +---------------------+  |  |
|  |       |              |                                  |  |
|  |       |         Dispatchers.IO                          |  |
|  |       |              |                                  |  |
|  |       |      +---------------+    +------------------+  |  |
|  |       |      | RustBridge.kt |    | PreferencesManager| |  |
|  |       |      | (31 external) |    | (SharedPrefs)    |  |  |
|  |       |      +-------+-------+    +------------------+  |  |
|  +------------------|---+----------------------------------+  |
|                     | JNI                                     |
|  +------------------|--------------------------------------+  |
|  |                  v         Rust (libsz_ffi.so)          |  |
|  |                                                        |  |
|  |  +----------+  +-----------+  +----------+  +--------+ |  |
|  |  |sz-compress|  |sz-crypto  |  |sz-webdav |  |sz-photo| |  |
|  |  | zbak     |  | AES-GCM   |  | HTTP PUT |  | sync   | |  |
|  |  | encrypted|  | Argon2id  |  | MKCOL    |  | diff   | |  |
|  |  | sevenz   |  | HKDF      |  | GET/HEAD |  | scan   | |  |
|  |  | smart    |  | password  |  | DELETE   |  |        | |  |
|  |  +----------+  +-----------+  +----------+  +--------+ |  |
|  |  +----------+  +-----------+                            |  |
|  |  |sz-filename|  |sz-core    |                           |  |
|  |  | 5种方案  |  | 共享类型   |                            |  |
|  |  +----------+  +-----------+                            |  |
|  +--------------------------------------------------------+  |
+--------------------------------------------------------------+
```

## 技术栈

| 层级 | 技术 |
|------|------|
| UI 框架 | Kotlin + Jetpack Compose |
| 设计系统 | Material Design 3 (Material You) |
| 状态管理 | ViewModel + StateFlow + collectAsState |
| 导航 | Navigation Compose |
| 序列化 | kotlinx.serialization (JSON) |
| 持久化 | SharedPreferences (兼容 Flutter 数据) |
| 核心引擎 | Rust (压缩/加密/WebDAV/文件名混淆/照片同步) |
| FFI 桥接 | JNI (Java Native Interface) |
| 编译产物 | libsz_ffi.so (cdylib) |
| 构建工具 | Gradle (Android) + cargo-ndk (Rust) |


## 模块结构

### Kotlin 源码 (`android/app/src/main/kotlin/com/sezip/sezip/`)

```
com/sezip/sezip/
  |-- MainActivity.kt              # 入口 Activity
  |-- SeZipApp.kt                  # Application 类
  |-- RustBridge.kt                # JNI 桥接 (31 个 external fun)
  |
  |-- navigation/
  |     |-- NavGraph.kt            # 导航图定义 (11 个路由)
  |
  |-- screens/                     # 11 个 Compose Screen
  |     |-- HomeScreen.kt          # 首页 — 功能入口卡片
  |     |-- CompressScreen.kt      # 压缩配置 — 模式/密码/恢复记录/分卷
  |     |-- CompressProgressScreen.kt  # 压缩进度 — 实时速度/百分比/取消
  |     |-- CompressResultScreen.kt    # 压缩结果 — 压缩比/文件大小
  |     |-- DecompressScreen.kt    # 解压 — 格式检测/文件预览/单文件提取
  |     |-- PasswordsScreen.kt     # 密码本 — 管理/生成/强度评估
  |     |-- WebDavScreen.kt        # WebDAV — 服务器配置/连接测试
  |     |-- WebDavFilesScreen.kt   # WebDAV 文件浏览 — 备份列表/恢复
  |     |-- MappingsScreen.kt      # 文件名映射表 — 原始名<->混淆名
  |     |-- SettingsScreen.kt      # 设置 — 主题/压缩级别/输出目录
  |     |-- PhotoBackupScreen.kt   # 照片备份 — 增量扫描/EXIF/本地+WebDAV
  |
  |-- viewmodel/                   # 7 个 ViewModel
  |     |-- CompressViewModel.kt           # 压缩配置状态
  |     |-- CompressProgressViewModel.kt   # 压缩执行 + CancelToken
  |     |-- DecompressViewModel.kt         # 解压状态
  |     |-- PasswordsViewModel.kt          # 密码本 CRUD
  |     |-- WebDavViewModel.kt             # WebDAV 连接/备份/恢复
  |     |-- SettingsViewModel.kt           # 设置读写
  |     |-- PhotoBackupViewModel.kt        # 照片增量备份
  |
  |-- model/                       # 数据模型
  |     |-- CompressModels.kt      # CompressMode/Options/Progress/Result + 照片模型
  |     |-- PasswordEntry.kt       # 密码条目
  |     |-- WebDavModels.kt        # WebDAV 配置/文件信息
  |     |-- MappingEntry.kt        # 文件名映射条目
  |
  |-- data/                        # 持久化层
  |     |-- PreferencesManager.kt  # SharedPreferences (兼容 Flutter key)
  |     |-- PasswordRepository.kt  # 密码本 JSON 存储
  |
  |-- ui/
  |     |-- theme/
  |     |     |-- Color.kt         # 颜色定义 (Blue/Teal/Amber/Red)
  |     |     |-- Type.kt          # 字体定义
  |     |     |-- Theme.kt         # SeZipTheme (浅色/深色/跟随系统/Material You)
  |     |
  |     |-- components/
  |           |-- PasswordField.kt # 密码输入框 (显示/隐藏/强度指示)
  |           |-- ProgressCard.kt  # 进度卡片 (百分比/速度/当前文件)
  |           |-- FeatureCard.kt   # 功能入口卡片
  |
  |-- util/
        |-- FormatUtils.kt         # 文件大小/时间格式化
        |-- FileUtils.kt           # 文件计数/路径处理/目录创建
```

### Rust Workspace (`rust/crates/`)

| Crate | 用途 | 关键模块 |
|-------|------|----------|
| `sz-core` | 共享类型、错误定义 | `WebDavConfig`, `SzError`, `SzResult`, `ObfuscationScheme` |
| `sz-compress` | 所有压缩格式实现 | `zbak/` (writer, reader, crypto, recovery, chunker, uploader, split), `encrypted.rs` (旧版 .sz7z), `sevenz.rs` (标准 7z), `smart_decompress.rs` |
| `sz-crypto` | 加密/密码工具 | `aes.rs` (AES-256-GCM + Argon2id), `password.rs` (随机密码生成, 强度计算, SHA256) |
| `sz-webdav` | WebDAV HTTP 客户端 | `client.rs` (PUT, MKCOL, HEAD, GET, DELETE, test_connection, list_directory) |
| `sz-filename` | 文件名混淆 | `schemes.rs` (顺序/日期/随机/哈希/加密 5 种方案) |
| `sz-photo-sync` | 照片增量同步 | 扫描、去重索引 (dedup_key)、增量 diff、SyncIndex 持久化 |
| `sz-ffi` | JNI 导出层 | `api.rs` (纯 Rust API), `lib.rs` (JNI 胶水代码, cdylib → libsz_ffi.so) |


## 数据流

### 压缩操作的完整数据流

```
[用户操作 CompressScreen]
        |
        v
CompressViewModel.buildOptions()          # 构建 CompressOptions
        |
        v
CompressProgressViewModel.startCompress() # 切换到 Dispatchers.IO
        |
        v
RustBridge.cancelTokenNew()               # 分配 CancelToken (返回 jlong)
        |
        v
RustBridge.compressZbak(...)              # JNI external fun
        |
        v  (JNI 边界)
Java_com_sezip_sezip_RustBridge_compressZbak()  # Rust JNI 导出函数
        |
        v
api::compress_zbak()                      # 纯 Rust API 层
        |
        v
sz_compress::ZbakWriter::compress()       # Zstd 压缩 + AES-GCM 加密
        |
        v
ProgressCallback.onProgress()             # JNI 回调 Kotlin
        |
        v
CompressProgress (StateFlow)              # UI 自动更新
        |
        v
CompressResultScreen                      # 显示结果
```

### 解压操作的数据流

```
[用户选择文件 DecompressScreen]
        |
        v
RustBridge.detectFormat()         # 魔数检测: ZBAK / SZ7Z / 7Z / SZP
        |
        v
RustBridge.smartRequiresPassword()  # 是否需要密码
        |                             (是) -> 弹出密码输入框
        v
RustBridge.smartDecompress()      # 自动分发到对应格式的解压器
        |
        v
sz_compress::SmartDecompressor    # 根据格式调用 ZbakReader / EncryptedCompressor / Decompressor
```


## 状态管理

采用 ViewModel + StateFlow 模式，Compose UI 通过 `collectAsState()` 订阅状态变更。

```kotlin
// ViewModel 侧 — 暴露不可变 StateFlow
class CompressViewModel : AndroidViewModel(app) {
    private val _password = MutableStateFlow("")
    val password: StateFlow<String> = _password.asStateFlow()
}

// Compose 侧 — 订阅状态
@Composable
fun CompressScreen(viewModel: CompressViewModel) {
    val password by viewModel.password.collectAsState()
    TextField(value = password, onValueChange = { viewModel.setPassword(it) })
}
```

所有 ViewModel 均为 `AndroidViewModel`，可通过 `getApplication()` 访问 Context 和 PreferencesManager。


## 进度回调机制

Kotlin 定义回调接口，Rust 侧通过 JNI `env.call_method` 回调：

```
Kotlin                          JNI                         Rust
RustBridge.ProgressCallback  ->  jobject callback  ->  env.call_method(
  fun onProgress(                                       callback,
    current: Long,                                      "onProgress",
    total: Long,                                        "(JJLjava/lang/String;)V",
    currentFile: String?                                &[current, total, file]
  )                                                   )
```

回调频率限制：ViewModel 侧每 200ms 更新一次 StateFlow，避免 UI 过度刷新。


## 取消机制

基于 Rust `Arc<AtomicBool>` 实现协作式取消：

```
1. cancelTokenNew()     # Kotlin 调用，Rust 创建 Arc<AtomicBool>(false)
                        # 返回 jlong (Box::into_raw 的指针值)

2. 传入 compressZbak()  # Rust 克隆 Arc，在循环中检查 is_cancelled()

3. cancelTokenCancel()  # 用户点击取消，Rust store(true, SeqCst)

4. cancelTokenFree()    # ViewModel.onCleared() 中释放
                        # Rust Box::from_raw 回收内存
```

**生命周期管理要点**：
- `cancelTokenNew()` 和 `cancelTokenFree()` 必须配对调用
- `ViewModel.onCleared()` 中自动取消并释放
- 一个 CancelToken 只能用于一次操作


## 错误处理

Rust 到 Kotlin 的错误传播链：

```
Rust 内部               JNI 边界                 Kotlin
anyhow::Result<T>  ->  match result {         ->  try {
  Ok(val)               Ok(val) => 返回 val,        RustBridge.xxx()
  Err(e)                Err(e) => {               } catch (e: RuntimeException) {
                          env.throw_new(            // 处理错误
                            "java/lang/RuntimeException",
                            e.to_string()
                          );
                        }
                      }
```

所有 JNI 函数遵循统一模式：Rust 侧的 `anyhow::Error` 转为 Java `RuntimeException`，
Kotlin 侧在 `viewModelScope.launch(Dispatchers.IO)` 中用 `try/catch` 捕获。


## 导航路由

| 路由 | Screen | 功能 |
|------|--------|------|
| `home` | HomeScreen | 首页功能入口 |
| `compress` | CompressScreen | 压缩配置 |
| `compress_progress` | CompressProgressScreen | 压缩执行进度 |
| `compress_result` | CompressResultScreen | 压缩结果展示 |
| `decompress` | DecompressScreen | 解压操作 |
| `passwords` | PasswordsScreen | 密码本管理 |
| `webdav` | WebDavScreen | WebDAV 服务器配置 |
| `webdav_files` | WebDavFilesScreen | WebDAV 文件浏览 |
| `mappings` | MappingsScreen | 文件名映射表 |
| `settings` | SettingsScreen | 应用设置 |
| `photo_backup` | PhotoBackupScreen | 照片增量备份 |


## 主题系统

- Android 12+ (API 31+): 使用 Material You 动态取色 (`dynamicLightColorScheme` / `dynamicDarkColorScheme`)
- Android 12 以下: 使用自定义配色 (主色 Blue, 辅色 Teal, 强调 Amber)
- 三种模式: 跟随系统 / 浅色 / 深色
- 主题选择持久化到 SharedPreferences，跨会话保持


## 压缩模式

| 模式 | 枚举值 | 说明 | 输出格式 |
|------|--------|------|----------|
| 本地备份 | `ZBAK` | Zstd + AES-GCM + 恢复记录 | `.zbak` |
| WebDAV 备份 | `ZBAK_WEBDAV` | 流式压缩 + 分块上传 | WebDAV 远程 |
| 7z 导出 | `LEGACY_7Z` | 标准 7z 兼容格式 | `.7z` |
