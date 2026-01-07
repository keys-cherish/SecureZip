# SecureZip

一款基于 Flutter + Rust 的加密压缩备份工具。

## 功能特性

- 📦 **7z + Zstd 压缩解压** - 高效压缩算法
- 🔐 **AES-256-GCM 加密** - 安全加密保护
- 📚 **密码本管理** - 安全存储多个密码
- ☁️ **WebDAV 云备份** - 支持云端同步
- 🎭 **文件名混淆** - 5种混淆方案保护隐私
- 🎯 **智能后缀匹配** - 自动根据文件后缀选择密码

## 系统要求

- Flutter 3.19+
- Rust 1.75+
- flutter_rust_bridge_codegen 2.x

## 快速开始

### 1. 安装依赖

```bash
# 安装 Flutter 依赖
flutter pub get

# 安装 flutter_rust_bridge 代码生成器
cargo install flutter_rust_bridge_codegen

# 安装 Rust 依赖
cd rust
cargo build
```

### 2. 生成 FFI 绑定

```bash
# 在项目根目录执行
flutter_rust_bridge_codegen generate
```

### 3. 运行应用

```bash
flutter run
```

## 项目结构

```
sezip/
├── lib/                      # Flutter 代码
│   ├── main.dart            # 入口文件
│   ├── app.dart             # 根组件
│   ├── theme.dart           # 主题配置
│   ├── router.dart          # 路由配置
│   ├── models/              # 数据模型
│   ├── services/            # 业务服务
│   ├── pages/               # 页面
│   ├── widgets/             # 可复用组件
│   └── src/rust/            # Rust 生成代码
├── rust/                     # Rust 代码
│   ├── Cargo.toml           # 工作区配置
│   └── crates/              # Rust crates
│       ├── sz-core/         # 核心类型和错误
│       ├── sz-compress/     # 压缩功能
│       ├── sz-crypto/       # 加密功能
│       ├── sz-webdav/       # WebDAV 客户端
│       ├── sz-filename/     # 文件名混淆
│       └── sz-ffi/          # FFI 接口
└── flutter_rust_bridge.yaml # FRB 配置
```

## 技术栈

### Flutter
- Material Design 3
- go_router 路由
- Provider 状态管理
- flutter_rust_bridge FFI

### Rust
- sevenz-rust: 7z 压缩
- zstd: Zstd 算法
- aes-gcm: AES 加密
- argon2: 密钥派生
- reqwest: HTTP 客户端

## 许可证

MIT License
