# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SecureZip (sezip) is a Kotlin + Jetpack Compose + Rust encrypted compression backup tool for Android. The primary format is `.zbak` (Zstd per-file compression + AES-256-GCM + Reed-Solomon recovery). A legacy `.sz7z` format and standard `.7z` are supported for backward compatibility. The Rust-Kotlin bridge uses JNI (Java Native Interface) via the `jni` crate. Documentation and code comments are primarily in Chinese.

## Build Commands

```powershell
# Build Rust .so for Android (requires cargo-ndk)
.\scripts\build_android_rust.ps1           # Debug
.\scripts\build_android_rust.ps1 -Release  # Release

# Full APK build (Rust + Kotlin)
.\scripts\build_apk.ps1                   # Debug
.\scripts\build_apk.ps1 -Release          # Release
.\scripts\build_apk.ps1 -SkipRust         # Skip Rust rebuild

# Kotlin/Android-only build (from project root)
cd android && ./gradlew assembleDebug      # Debug
cd android && ./gradlew assembleRelease    # Release

# Rust-only build (from repo root)
cd rust && cargo build -p sz-ffi
```

## Test and Lint Commands

```bash
cd android && ./gradlew test       # Kotlin unit tests
cd android && ./gradlew lint       # Android Lint static analysis
cd rust && cargo test              # Rust tests (47 tests across all crates)
```

## Architecture

### JNI Bridge (Rust `jni` crate)

**MIGRATED from Flutter/Dart/FRB to Kotlin + JNI** (2026-04).

**Data flow:** Rust API (`rust/crates/sz-ffi/src/api.rs`) --> JNI bridge (`rust/crates/sz-ffi/src/jni_bridge.rs`) --> Kotlin `RustBridge.kt` --> ViewModel / Service layer

- Rust side: `api.rs` contains pure Rust logic, `jni_bridge.rs` wraps each function with `#[no_mangle] pub extern "system" fn Java_...` JNI exports
- Kotlin side: `RustBridge.kt` declares `external fun` methods and calls `System.loadLibrary("sz_ffi")` to load the native library
- Progress reporting: Rust calls back into Kotlin via JNI `CallVoidMethod` on a progress callback object
- Cancellation: Rust checks a `AtomicBool` flag set from Kotlin via JNI
- Error handling: Rust errors are converted to Java exceptions via `JNIEnv::throw_new`
- The `sz-ffi` crate uses the `jni` crate (`jni = "0.21"`) and compiles as `cdylib` -> `libsz_ffi.so`

### Compression Formats

**Primary format (.zbak):** Non-solid per-file Zstd compression + AES-256-GCM encryption with HKDF per-file subkeys + optional Reed-Solomon recovery records.
- 96-byte fixed header: `[0-3] "ZBAK" | [4] VERSION(1) | [5-6] FLAGS | [7] LEVEL | [8-23] SALT | [24-27] FILE_COUNT | [28-95] offsets/sizes/verify`
- Key derivation: Argon2id (m=65536, t=3, p=4) -> HKDF per-file subkeys
- Password verification block: GCM tag of known plaintext for instant wrong-password detection
- Per-file data blocks: Zstd -> AES-256-GCM (each file has independent key + random nonce)
- Index section: file paths, sizes, offsets, CRC32, timestamps (optionally encrypted for filename privacy)
- Recovery section: Reed-Solomon erasure coding (5%/10%/20% redundancy)
- Implemented in `rust/crates/sz-compress/src/zbak/` (format.rs, writer.rs, reader.rs, crypto.rs, recovery.rs)

**Legacy format (.sz7z):** 7z (LZMA2) -> Zstd -> optional AES-256-GCM (deprecated for new archives, read-only support preserved)
- Implemented in `rust/crates/sz-compress/src/encrypted.rs`

**Standard format (.7z):** Direct `sevenz-rust` crate usage with optional native 7z AES-256 (behind `legacy-7z` feature flag)
- Implemented in `rust/crates/sz-compress/src/sevenz.rs`

**Smart decompression** (`rust/crates/sz-compress/src/smart_decompress.rs`): Auto-detects format by magic bytes (ZBAK, SZ7Z, SZPK legacy, 7z standard), falls back to file extension.

### Compress Modes

Three modes in `CompressMode` enum:
- `zbak` -- Primary: .zbak local backup (Zstd + AES-GCM + recovery)
- `zbakWebdav` -- WebDAV streaming: compress -> chunk(50MB) -> upload -> release memory
- `legacy7z` -- Standard .7z for compatibility

All three modes are selectable via segmented button in the Compose UI.

### Rust Workspace (`rust/crates/`)

| Crate | Purpose |
|---|---|
| `sz-core` | Shared types (`WebDavConfig`, `WebDavFileInfo`), error definitions (`SzError`, `SzResult`) |
| `sz-compress` | All compression: zbak module, encrypted (legacy .sz7z), sevenz (behind feature), smart_decompress |
| `sz-crypto` | AES-256-GCM encryption, Argon2id key derivation, password strength |
| `sz-filename` | Filename obfuscation (5 schemes: sequential, dateSequential, random, hash, encrypted) |
| `sz-webdav` | WebDAV client (PUT, MKCOL, HEAD, GET, DELETE, test_connection, list_directory) |
| `sz-ffi` | JNI exports (cdylib -> `libsz_ffi.so`) -- api.rs (logic), jni_bridge.rs (JNI glue), zbak/7z/smart/webdav functions |

### zbak Module Structure (`rust/crates/sz-compress/src/zbak/`)

| File | Purpose |
|---|---|
| `mod.rs` | Module re-exports |
| `format.rs` | ZbakHeader (96 bytes), ZbakIndexEntry, serialization, constants, `write_index`/`read_index` |
| `crypto.rs` | Argon2id master key, HKDF file/index/verify subkeys, AES-GCM encrypt/decrypt block/index |
| `writer.rs` | ZbakWriter: rayon parallel Zstd compress -> optional encrypt -> write index -> optional RS recovery. Exports `collect_files()` and `FileInfo` for reuse by uploader |
| `reader.rs` | ZbakReader: requires_password, verify_password, list_contents, decompress, extract_file |
| `recovery.rs` | RecoveryGenerator: Reed-Solomon generate/recover with configurable ratio (5%/10%/20%) |
| `chunker.rs` | Chunker: split data into fixed-size chunks (default 50MB), BackupManifest (JSON), SHA-256 per chunk |
| `uploader.rs` | StreamingUploader: true streaming WebDAV backup (compress->buffer->upload chunk when full->release), restore, list_backups. Chunk 0 delayed for header fixup. RS optional (requires keeping all chunk data in memory) |
| `split.rs` | Split/join local archive volumes (.zbak.001, .zbak.002, ...) |

### WebDAV Streaming Upload Flow

```
1. Collect files -> prepare encryption context
2. Write zbak header placeholder -> compress files one by one -> append to buffer
3. Buffer >= 50MB -> cut chunk -> upload immediately -> release (chunk 0 held for header fixup)
4. All files done -> write index -> flush remaining buffer as final chunks
5. Fix header in chunk 0 -> upload chunk 0
6. (Optional) Generate RS recovery from all chunk data -> upload recovery chunks
7. Upload manifest.json (last = completion marker)
```

Memory: ~50MB without RS, ~total compressed size with RS enabled.
Resume: HEAD request checks existing chunks -> skip -> continue remaining.

### Kotlin App (`android/app/src/main/java/`)

- **UI framework:** Jetpack Compose with Material Design 3
- **Architecture:** MVVM (ViewModel + Repository pattern)
- **DI:** Hilt for dependency injection
- **Navigation:** Compose Navigation with routes: `/`, `/compress`, `/decompress`, `/passwords`, `/webdav`, `/webdav/files`, `/mappings`, `/settings`
- **Theme:** Material Design 3, seed color `#1565C0`, light + dark modes
- **Coroutines:** Kotlin Coroutines + Flow for async operations and progress streams

### Key Kotlin Files

| File | Purpose |
|---|---|
| `RustBridge.kt` | JNI bindings: `external fun` declarations, `System.loadLibrary("sz_ffi")`, type conversions |
| `CompressViewModel.kt` | Mode dispatch (zbak/zbakWebdav/legacy7z), coroutine-based compression, progress Flow |
| `DecompressViewModel.kt` | Format auto-detection, decompression orchestration, single-file extraction |
| `CompressScreen.kt` | 3-mode selector, recovery/filename encryption toggles, split volume config |
| `DecompressScreen.kt` | Format auto-detection badge, file list preview, single-file extraction for .zbak |
| `CompressProgressScreen.kt` | Real-time progress (speed, percentage, current file), cancel support |
| `WebDavScreen.kt` | WebDAV server config, connection test, app data backup/restore |
| `SettingsScreen.kt` | App settings: theme, language, default options |
| `CompressOptions.kt` | CompressMode enum, CompressOptions (incl. WebDAV config), CompressProgress, RecoveryRatio |

### Android Build Targets

Rust cross-compiles via `cargo ndk` to three ABIs. The `.so` files are placed in `android/app/src/main/jniLibs/<abi>/`:
- `aarch64-linux-android` -> `arm64-v8a`
- `armv7-linux-androideabi` -> `armeabi-v7a`
- `x86_64-linux-android` -> `x86_64`

### Key Rust Build Settings (Release)

LTO=true, codegen-units=1, opt-level=3 (configured in workspace `Cargo.toml`).

### Security Design

- **Argon2id** (64MB x 3 iterations x 4 parallelism): Anti-GPU/ASIC brute force
- **HKDF per-file subkeys**: Each file gets independent key derived from master key + file index. Single file key leak doesn't compromise others
- **Password verification block**: GCM tag of known plaintext in header -> wrong password detected instantly without wasting time decompressing
- **Three-tier protection**: (1) GCM tags detect corruption per-file, (2) Reed-Solomon repairs damaged blocks, (3) Non-solid structure limits damage to affected blocks only
- **Algorithm ID in header**: Any developer can read the header and know exactly which standard algorithms to use for decryption
