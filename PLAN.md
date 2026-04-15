# 修复计划：5个关键Bug

## Bug 1: 压缩完成后闪过"压缩失败"页面
**根因**: `compress_progress_page.dart` 的 `stream.listen` 没有设 `cancelOnError`。当压缩流抛异常时，`onError` 先触发（显示错误），然后 `onDone` 也触发（导航到结果页）。两个回调都执行导致"闪一下"。
**修复**: 在 `onDone` 中添加 `_error == null` 守卫。

## Bug 2: 目标目录重复名字
**根因**: `compress_page.dart` 的 `_startCompress()` 直接使用输出路径，不检查文件是否已存在。
**修复**: 在 `_startCompress()` 中，检查输出文件是否存在，若存在则自动追加 `_1`, `_2` 等后缀。

## Bug 3: 速度不正常 + 压缩率是0.0
**根因**: `compress_progress_page.dart` 的 `onDone` 调用 `getResult()`，该方法重新遍历磁盘文件来计算 originalSize 和 compressedSize。在 Android 上可能路径访问有问题，且 `duration` 固定为 `Duration.zero`。Rust Isolate 已经返回了正确的 originalSize/compressedSize，但没有被使用。
**修复**:
- 在 `CompressProgress` 中增加可选 `originalSize` 和 `compressedSize` 字段
- `_compressZbak` 最终 yield 时填入 Isolate 返回的值
- `compress_progress_page.dart` 直接使用最后一条 progress 的数据构建结果，不再调用 `getResult()`

## Bug 4: 解压时密码正确但仍报错
**根因**: `rust_compress_service.dart` 的 `decompress()` 方法中：
```dart
final isEncrypted = lowerPath.endsWith('.enc') || lowerPath.endsWith('.sz7z');
'password': isEncrypted ? password : null,
```
`.zbak` 文件不匹配 `.enc` 或 `.sz7z`，所以 `isEncrypted = false`，密码被强制设为 null！Smart decompress 收到 null 密码，解密失败。
**修复**: 直接传递 password，不做格式过滤。Rust 层的 smart_decompress 会自行判断是否需要密码。

## Bug 5: 解压大文件页面卡顿
**根因**: `decompress_page.dart` 使用 `await for` 消费进度流，每次 yield 都同步阻塞在 `setState()` 上形成背压。与 compress_progress_page.dart（使用 `stream.listen`）不同。
**修复**: 将 `await for` 改为 `stream.listen()` 模式，与压缩进度页保持一致。

## 修改文件列表
1. `lib/pages/compress_progress_page.dart` — Bug 1 + Bug 3
2. `lib/pages/compress_page.dart` — Bug 2
3. `lib/models/compress_options.dart` — Bug 3 (CompressProgress 增加字段)
4. `lib/services/rust_compress_service.dart` — Bug 3 + Bug 4
5. `lib/pages/decompress_page.dart` — Bug 5
