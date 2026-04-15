# SeZip 任务列表

## 严重问题

- [x] **FFI 内存分配器不匹配**：`toNativeUtf8()` 使用 malloc 分配，但用 `calloc.free()` 释放 → 已全部改为 `malloc.free()`
- [x] **Rust `get_cancel_flag()` 无效**：每次返回独立的 `AtomicBool(false)` → 已修复为读取全局 CANCELLED 状态
- [x] **`compress()` 方法死代码**：所有分支都以 `return` 结束，后续约200行代码永远不执行 → 已删除死代码

## 中等问题

- [x] **Argon2 使用默认参数** → 已固定为 Argon2id, m=65536, t=3, p=4
- [x] **`sha2` 未使用导入** → 已删除
- [ ] **AES 最小密文长度检查过宽**：`aes.rs:71` 检查 `< 13`，但 AES-GCM 密文含 16 字节 tag，最小有效长度应为 28 → ✅ 已修复为 29

## 低优先级

- [x] **冗余依赖清理**：移除了 `cryptography` 和 `flutter_rust_bridge`（`pointycastle` 在 Dart 回退实现中使用，保留）
- [ ] **密码生成不保证字符类别覆盖**：`password.rs` 的 `generate_random_password` 纯随机采样 → ✅ 已修复，保证每类字符至少出现一次

## 架构优化（已完成）

目标：统一为 7z.zstd 专属格式（类似 7-Zip-zstd），删除 tar.zstd 方案

- [x] 修复 AES 最小密文长度检查（aes.rs:71，改为 29）
- [x] 修复密码生成字符类别覆盖（password.rs）
- [x] 优化 encrypted.rs：v2 格式、多线程 Zstd、取消支持、进度回调改进、可选加密
- [x] 删除 tar_zstd.rs 及所有相关引用
- [x] 更新 smart_decompress.rs：移除 Szp/TarZstd 格式，保留 SzpLegacy 向后兼容
- [x] 更新 c_api.rs：移除 tar_zstd FFI 接口，统一为 7z.zstd
- [x] 更新 Dart 侧 FFI 绑定（rust_compress_ffi.dart）和 compress_service（rust_compress_service.dart）
- [x] 清理 Cargo.toml 中 tar 依赖
- [x] 压缩/解压取消功能全面支持（cancel_flag 贯穿 compress/decompress）

## 功能增强

- [ ] 添加 .zip 格式支持
- [ ] WebDAV 同步功能完善
- [ ] 文件名混淆模式的 UI 优化
