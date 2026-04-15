# SecureZip 构建文档

## 环境要求

### Android 开发环境

| 组件 | 版本要求 |
|------|----------|
| Android SDK | API 35 (compileSdk) |
| Android NDK | 最新稳定版 (cargo-ndk 自动定位) |
| Java (JDK) | 17 |
| Gradle | 项目内 Wrapper 管理 |

### Rust 工具链

| 组件 | 安装方式 |
|------|----------|
| Rust toolchain | `rustup` (stable channel) |
| cargo-ndk | `cargo install cargo-ndk` |
| Android 交叉编译目标 | 见下方说明 |

安装 Android 交叉编译目标：

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
```

### 环境变量

确保以下环境变量正确设置：
- `ANDROID_HOME` / `ANDROID_SDK_ROOT` — 指向 Android SDK 安装目录
- `ANDROID_NDK_HOME` — 指向 NDK 安装目录 (可选，cargo-ndk 可自动从 SDK 定位)


## 构建步骤

### 方式一：完整构建 (推荐)

使用 PowerShell 脚本一键完成 Rust 库编译 + APK 打包：

```powershell
# Debug APK
.\scripts\build_apk.ps1

# Release APK
.\scripts\build_apk.ps1 -Release

# Release + 通用 APK (不按 ABI 分包)
.\scripts\build_apk.ps1 -Release -Universal

# 跳过 Rust 编译 (仅修改 Kotlin 代码时)
.\scripts\build_apk.ps1 -SkipRust
```

### 方式二：分步构建

**第一步：编译 Rust 库**

```powershell
# Debug 构建
.\scripts\build_android_rust.ps1

# Release 构建
.\scripts\build_android_rust.ps1 -Release

# 跳过 cargo-ndk 安装检查和 target 添加
.\scripts\build_android_rust.ps1 -Release -SkipInstall
```

脚本自动完成：
1. 检查并安装 cargo-ndk
2. 添加 Rust 交叉编译目标
3. 创建 jniLibs 目录
4. 对 3 个 ABI 分别执行 `cargo ndk -t <triple> build -p sz-ffi`
5. 复制 `libsz_ffi.so` 到 `android/app/src/main/jniLibs/<abi>/`

**第二步：构建 APK**

```powershell
# 使用 Gradle (推荐)
cd android
./gradlew assembleDebug       # Debug
./gradlew assembleRelease     # Release
```

### 方式三：仅 Rust 开发

不涉及 Android 交叉编译，直接在宿主机上构建和测试：

```bash
# 构建 sz-ffi crate
cd rust && cargo build -p sz-ffi

# 构建整个 workspace
cd rust && cargo build

# 运行所有测试
cd rust && cargo test

# 运行特定 crate 的测试
cd rust && cargo test -p sz-compress
cd rust && cargo test -p sz-crypto
```


## ABI 分包说明

Rust 交叉编译为 3 个 Android ABI，对应关系：

| Rust Triple | Android ABI | 典型设备 |
|-------------|-------------|----------|
| `aarch64-linux-android` | `arm64-v8a` | 绝大多数现代手机 (2017+) |
| `armv7-linux-androideabi` | `armeabi-v7a` | 旧款 32 位 ARM 设备 |
| `x86_64-linux-android` | `x86_64` | 模拟器、Chromebook |

编译产物存放位置：

```
android/app/src/main/jniLibs/
  |-- arm64-v8a/
  |     |-- libsz_ffi.so
  |-- armeabi-v7a/
  |     |-- libsz_ffi.so
  |-- x86_64/
        |-- libsz_ffi.so
```

### APK 分包策略

- **Release 默认**：`--split-per-abi` 按 ABI 分成 3 个 APK，每个只包含对应架构的 .so
- **Universal**：`-Universal` 参数生成单个 APK，包含所有 ABI 的 .so (体积较大)
- **Debug**：始终生成单个 universal APK


## APK 输出位置

```
build/app/outputs/flutter-apk/
  |-- app-arm64-v8a-release.apk       # arm64 Release
  |-- app-armeabi-v7a-release.apk     # armv7 Release
  |-- app-x86_64-release.apk          # x86_64 Release
  |-- app-release.apk                 # Universal Release (使用 -Universal)
  |-- app-debug.apk                   # Debug
```

使用 Gradle 直接构建时，输出位置为：

```
android/app/build/outputs/apk/
  |-- debug/app-debug.apk
  |-- release/app-release.apk
```


## Release 构建优化

### Rust 优化 (workspace `Cargo.toml`)

```toml
[profile.release]
lto = true              # 链接时优化 — 跨 crate 内联，大幅减小体积
codegen-units = 1       # 单编译单元 — 最大化优化效果
opt-level = 3           # 最高优化级别
strip = "symbols"       # 剥离符号表
```

这些设置使 Release .so 体积比 Debug 缩小约 60-70%，运行速度提升 2-5 倍 (尤其 Zstd 压缩和 Argon2id 密钥派生)。

### Android 优化

- **ProGuard / R8**: Release 构建默认启用代码混淆和 tree-shaking
- **minSdk**: 尽可能设置较高的 minSdk 以利用新 API 和减少兼容代码
- **资源压缩**: `shrinkResources true` 移除未使用的资源文件


## 常见问题

### cargo-ndk 找不到 NDK

确保 Android SDK 中已安装 NDK，或设置 `ANDROID_NDK_HOME` 环境变量：

```powershell
$env:ANDROID_NDK_HOME = "C:\Users\<user>\AppData\Local\Android\Sdk\ndk\<version>"
```

### 链接错误：undefined symbol

通常是 Rust crate 的 feature flag 未启用。检查 `rust/crates/sz-ffi/Cargo.toml` 的 dependencies 配置。

### .so 文件未更新

确保不是使用了缓存的旧 .so：

```powershell
# 清理 Rust 构建缓存
cd rust && cargo clean

# 重新完整构建
.\scripts\build_apk.ps1 -Release
```

### Gradle 构建失败

```powershell
# 清理 Gradle 缓存
cd android && ./gradlew clean

# 重新同步依赖
cd android && ./gradlew --refresh-dependencies
```
