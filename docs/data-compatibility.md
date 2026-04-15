# Flutter -> Kotlin 数据兼容方案

## 概述

SecureZip 从 Flutter 迁移到 Kotlin + Jetpack Compose 后，
用户已有的所有数据（密码本、设置、文件名映射、WebDAV 配置）自动保留，无需任何手动迁移。

实现原理：Kotlin 侧直接读写 Flutter 创建的 SharedPreferences 文件，使用相同的文件名和 key。


## SharedPreferences 文件

Flutter 的 `shared_preferences` 包在 Android 上将数据存储为标准 SharedPreferences XML 文件：

```
/data/data/com.sezip.sezip/shared_prefs/FlutterSharedPreferences.xml
```

Kotlin 侧通过以下方式打开同一文件：

```kotlin
context.getSharedPreferences("FlutterSharedPreferences", Context.MODE_PRIVATE)
```


## Key 前缀

Flutter `shared_preferences` 包为所有 key 添加 `flutter.` 前缀。
Kotlin 侧必须沿用此前缀才能读取 Flutter 写入的数据。

```kotlin
companion object {
    private const val PREFIX = "flutter."
}
```


## 完整 Key 列表

| Key (含前缀) | 数据类型 | 默认值 | 说明 |
|--------------|----------|--------|------|
| `flutter.secure_zip_theme_mode` | Long | `0` | 主题模式 (0=跟随系统, 1=浅色, 2=深色) |
| `flutter.secure_zip_default_scheme` | String | `"sequential"` | 默认文件名混淆方案 |
| `flutter.secure_zip_compression_level` | Long | `6` | 默认压缩级别 (1-22) |
| `flutter.secure_zip_output_dir` | String | `""` | 自定义压缩输出目录 |
| `flutter.secure_zip_decompress_output_dir` | String | `""` | 自定义解压输出目录 |
| `flutter.secure_zip_passwords` | String | `"[]"` | 密码本 (JSON 数组) |
| `flutter.secure_zip_mappings` | String | `"[]"` | 文件名映射表 (JSON 数组) |
| `flutter.secure_zip_ext_mappings` | String | `"[]"` | 扩展名-密码映射 (JSON 数组) |
| `flutter.secure_zip_webdav_config` | String | `"{}"` | WebDAV 服务器配置 (JSON 对象) |


## Flutter Long 类型问题

Flutter `shared_preferences` 在 Android 上将 Dart 的 `int` 存储为 `Long` (64 位整数)。
这导致一个兼容性细节：

```kotlin
// Flutter 存储 int 时使用 putLong
// Kotlin 读取时必须用 getLong, 而不是 getInt

var themeModeIndex: Int
    get() = prefs.getLong(KEY_THEME_MODE, 0).toInt()  // getLong + toInt
    set(value) = prefs.edit().putLong(KEY_THEME_MODE, value.toLong()).apply()  // putLong

var compressionLevel: Int
    get() = prefs.getLong(KEY_COMPRESSION_LEVEL, 6).toInt()  // getLong + toInt
    set(value) = prefs.edit().putLong(KEY_COMPRESSION_LEVEL, value.toLong()).apply()  // putLong
```

如果用 `getInt()` 读取 Flutter 写入的 Long 值，会抛出 `ClassCastException`。
Kotlin 侧写入时也必须用 `putLong()` 保持一致，这样即使用户降级回 Flutter 版本也不会出错。

String 类型的数据无此问题，`getString()` / `putString()` 两端完全兼容。


## 升级后零迁移

用户从 Flutter 版升级到 Kotlin 版后：

1. **密码本** — `secure_zip_passwords` key 中的 JSON 数组保持不变，Kotlin 侧使用 `kotlinx.serialization` 反序列化
2. **压缩设置** — 主题模式、压缩级别、输出目录等设置自动生效
3. **文件名映射表** — 已有的原始名<->混淆名映射完全保留
4. **WebDAV 配置** — 服务器 URL、用户名、密码等配置保留
5. **已创建的压缩包** — .zbak / .sz7z / .7z 文件格式不变，Rust 引擎完全兼容


## 默认输出目录

当用户未自定义输出目录时，使用默认路径：

```kotlin
const val DEFAULT_COMPRESS_DIR = "/storage/emulated/0/SecureZip/compressed"
const val DEFAULT_DECOMPRESS_DIR = "/storage/emulated/0/SecureZip/extracted"
```

这与 Flutter 版本使用的默认路径一致。


## 注意事项

1. **不可更改文件名**：SharedPreferences 文件名必须保持 `FlutterSharedPreferences`，即使项目已完全迁离 Flutter。更改文件名会导致已有用户数据丢失。

2. **不可移除 flutter. 前缀**：所有 key 必须保留 `flutter.` 前缀。这看起来不够优雅，但是保证数据兼容的必要代价。

3. **新增 key 的约定**：如果 Kotlin 版需要新增配置项，建议继续使用 `flutter.secure_zip_` 前缀，保持命名一致性。虽然新的 key 与 Flutter 无关，但统一前缀有助于避免混乱。

4. **JSON 格式一致性**：密码本、映射表等 JSON 数据的字段名和格式必须与 Flutter 版本保持一致。任何结构变更都应该做兼容处理 (能解析旧格式)。

5. **避免使用 getInt()**：由于 Flutter 存储 int 为 Long，所有整数类型的读取都必须使用 `getLong().toInt()`。如果新代码误用 `getInt()`，在升级用户设备上会立即崩溃。

6. **测试建议**：升级测试时，先安装 Flutter 版本，写入各种配置数据，然后覆盖安装 Kotlin 版本，验证所有数据正确读取。
