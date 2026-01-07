// ignore_for_file: non_constant_identifier_names, camel_case_types

import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

/// 压缩结果结构体
final class CCompressResult extends Struct {
  @Int32()
  external int success;

  @Uint64()
  external int originalSize;

  @Uint64()
  external int compressedSize;

  external Pointer<Utf8> errorMessage;
}

/// 解压结果结构体
final class CDecompressResult extends Struct {
  @Int32()
  external int success;

  @Int32()
  external int fileCount;

  external Pointer<Utf8> errorMessage;
}

/// 进度结构体
final class CProgress extends Struct {
  @Uint64()
  external int current;

  @Uint64()
  external int total;
}

// ============================================================================
// Native 函数类型定义
// ============================================================================

/// sz_compress
typedef SzCompressNative = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Int32 compressionLevel,
);
typedef SzCompressDart = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  int compressionLevel,
);

/// sz_compress_encrypted
typedef SzCompressEncryptedNative = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Pointer<Utf8> password,
  Int32 compressionLevel,
);
typedef SzCompressEncryptedDart = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Pointer<Utf8> password,
  int compressionLevel,
);

/// sz_decompress
typedef SzDecompressNative = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
);
typedef SzDecompressDart = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
);

/// sz_decompress_encrypted
typedef SzDecompressEncryptedNative = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);
typedef SzDecompressEncryptedDart = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);

/// sz_compress_7z - 标准7z压缩
typedef SzCompress7zNative = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Int32 compressionLevel,
);
typedef SzCompress7zDart = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  int compressionLevel,
);

/// sz_compress_7z_encrypted - 标准7z加密压缩
typedef SzCompress7zEncryptedNative = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Pointer<Utf8> password,
  Int32 compressionLevel,
);
typedef SzCompress7zEncryptedDart = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Pointer<Utf8> password,
  int compressionLevel,
);

/// sz_decompress_7z - 标准7z解压
typedef SzDecompress7zNative = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);
typedef SzDecompress7zDart = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);

/// sz_list_7z_contents - 列出7z压缩包内容
typedef SzList7zContentsNative = Pointer<Utf8> Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);
typedef SzList7zContentsDart = Pointer<Utf8> Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);

/// sz_verify_password
typedef SzVerifyPasswordNative = Int32 Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);
typedef SzVerifyPasswordDart = int Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);

/// sz_get_progress
typedef SzGetProgressNative = CProgress Function();
typedef SzGetProgressDart = CProgress Function();

/// sz_reset_progress
typedef SzResetProgressNative = Void Function();
typedef SzResetProgressDart = void Function();

/// sz_request_cancel - 请求取消当前操作
typedef SzRequestCancelNative = Void Function();
typedef SzRequestCancelDart = void Function();

/// sz_is_cancelled - 检查是否已请求取消
typedef SzIsCancelledNative = Int32 Function();
typedef SzIsCancelledDart = int Function();

/// sz_free_string
typedef SzFreeStringNative = Void Function(Pointer<Utf8> s);
typedef SzFreeStringDart = void Function(Pointer<Utf8> s);

/// sz_compress_tar_zstd - Tar + Zstd 压缩（可选密码）
typedef SzCompressTarZstdNative = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Pointer<Utf8> password,
  Int32 compressionLevel,
);
typedef SzCompressTarZstdDart = CCompressResult Function(
  Pointer<Utf8> inputPaths,
  Pointer<Utf8> outputPath,
  Pointer<Utf8> password,
  int compressionLevel,
);

/// sz_decompress_tar_zstd - Tar + Zstd 解压
typedef SzDecompressTarZstdNative = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);
typedef SzDecompressTarZstdDart = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);

/// sz_verify_tar_zstd_password
typedef SzVerifyTarZstdPasswordNative = Int32 Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);
typedef SzVerifyTarZstdPasswordDart = int Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);

/// sz_requires_tar_zstd_password
typedef SzRequiresTarZstdPasswordNative = Int32 Function(
  Pointer<Utf8> archivePath,
);
typedef SzRequiresTarZstdPasswordDart = int Function(
  Pointer<Utf8> archivePath,
);

/// sz_smart_decompress - 智能解压（自动检测格式）
typedef SzSmartDecompressNative = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);
typedef SzSmartDecompressDart = CDecompressResult Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> outputDir,
  Pointer<Utf8> password,
);

/// sz_detect_format - 检测压缩包格式
typedef SzDetectFormatNative = Int32 Function(
  Pointer<Utf8> archivePath,
);
typedef SzDetectFormatDart = int Function(
  Pointer<Utf8> archivePath,
);

/// sz_smart_requires_password - 智能检测是否需要密码
typedef SzSmartRequiresPasswordNative = Int32 Function(
  Pointer<Utf8> archivePath,
);
typedef SzSmartRequiresPasswordDart = int Function(
  Pointer<Utf8> archivePath,
);

/// sz_smart_verify_password - 智能验证密码
typedef SzSmartVerifyPasswordNative = Int32 Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);
typedef SzSmartVerifyPasswordDart = int Function(
  Pointer<Utf8> archivePath,
  Pointer<Utf8> password,
);

// ============================================================================
// Dart API 封装
// ============================================================================

/// Rust 压缩库 FFI 封装
class RustCompressLib {
  static RustCompressLib? _instance;
  late final DynamicLibrary _lib;

  // 函数绑定
  late final SzCompressDart _compress;
  late final SzCompressEncryptedDart _compressEncrypted;
  late final SzDecompressDart _decompress;
  late final SzDecompressEncryptedDart _decompressEncrypted;
  late final SzCompress7zDart _compress7z;
  late final SzCompress7zEncryptedDart _compress7zEncrypted;
  late final SzDecompress7zDart _decompress7z;
  late final SzList7zContentsDart _list7zContents;
  late final SzVerifyPasswordDart _verifyPassword;
  late final SzGetProgressDart _getProgress;
  late final SzResetProgressDart _resetProgress;
  late final SzRequestCancelDart _requestCancel;
  late final SzIsCancelledDart _isCancelled;
  late final SzFreeStringDart _freeString;

  // Tar + Zstd 函数绑定
  late final SzCompressTarZstdDart _compressTarZstd;
  late final SzDecompressTarZstdDart _decompressTarZstd;
  late final SzVerifyTarZstdPasswordDart _verifyTarZstdPassword;
  late final SzRequiresTarZstdPasswordDart _requiresTarZstdPassword;
  bool _hasTarZstdSupport = false;

  // 智能解压函数绑定
  late final SzSmartDecompressDart _smartDecompress;
  late final SzDetectFormatDart _detectFormat;
  late final SzSmartRequiresPasswordDart _smartRequiresPassword;
  late final SzSmartVerifyPasswordDart _smartVerifyPassword;
  bool _hasSmartDecompressSupport = false;

  /// 智能解压支持是否可用
  bool get hasSmartDecompressSupport => _hasSmartDecompressSupport;

  /// Tar+Zstd 支持是否可用
  bool get hasTarZstdSupport => _hasTarZstdSupport;

  /// 是否可用
  bool get isAvailable => _isLoaded;
  bool _isLoaded = false;
  bool _has7zSupport = false;

  /// 7z支持是否可用
  bool get has7zSupport => _has7zSupport;

  /// 获取单例
  static RustCompressLib get instance {
    _instance ??= RustCompressLib._();
    return _instance!;
  }

  RustCompressLib._() {
    _loadLibrary();
  }

  void _loadLibrary() {
    try {
      if (Platform.isAndroid) {
        _lib = DynamicLibrary.open('libsz_ffi.so');
      } else if (Platform.isWindows) {
        _lib = DynamicLibrary.open('sz_ffi.dll');
      } else if (Platform.isLinux) {
        _lib = DynamicLibrary.open('libsz_ffi.so');
      } else if (Platform.isMacOS) {
        _lib = DynamicLibrary.open('libsz_ffi.dylib');
      } else {
        throw UnsupportedError('不支持的平台');
      }

      _bindFunctions();
      _isLoaded = true;
    } catch (e) {
      print('警告: 无法加载 Rust 库: $e');
      _isLoaded = false;
    }
  }

  void _bindFunctions() {
    _compress = _lib
        .lookup<NativeFunction<SzCompressNative>>('sz_compress')
        .asFunction();

    _compressEncrypted = _lib
        .lookup<NativeFunction<SzCompressEncryptedNative>>(
            'sz_compress_encrypted')
        .asFunction();

    _decompress = _lib
        .lookup<NativeFunction<SzDecompressNative>>('sz_decompress')
        .asFunction();

    _decompressEncrypted = _lib
        .lookup<NativeFunction<SzDecompressEncryptedNative>>(
            'sz_decompress_encrypted')
        .asFunction();

    // 绑定7z函数（可选，如果不存在则使用回退）
    try {
      _compress7z = _lib
          .lookup<NativeFunction<SzCompress7zNative>>('sz_compress_7z')
          .asFunction();
      _compress7zEncrypted = _lib
          .lookup<NativeFunction<SzCompress7zEncryptedNative>>(
              'sz_compress_7z_encrypted')
          .asFunction();
      _decompress7z = _lib
          .lookup<NativeFunction<SzDecompress7zNative>>('sz_decompress_7z')
          .asFunction();
      _list7zContents = _lib
          .lookup<NativeFunction<SzList7zContentsNative>>('sz_list_7z_contents')
          .asFunction();
      _has7zSupport = true;
    } catch (e) {
      print('警告: 7z 函数不可用: $e');
      _has7zSupport = false;
    }

    _verifyPassword = _lib
        .lookup<NativeFunction<SzVerifyPasswordNative>>('sz_verify_password')
        .asFunction();

    _getProgress = _lib
        .lookup<NativeFunction<SzGetProgressNative>>('sz_get_progress')
        .asFunction();

    _resetProgress = _lib
        .lookup<NativeFunction<SzResetProgressNative>>('sz_reset_progress')
        .asFunction();

    _requestCancel = _lib
        .lookup<NativeFunction<SzRequestCancelNative>>('sz_request_cancel')
        .asFunction();

    _isCancelled = _lib
        .lookup<NativeFunction<SzIsCancelledNative>>('sz_is_cancelled')
        .asFunction();

    _freeString = _lib
        .lookup<NativeFunction<SzFreeStringNative>>('sz_free_string')
        .asFunction();

    // 绑定 Tar + Zstd 函数（可选）
    try {
      _compressTarZstd = _lib
          .lookup<NativeFunction<SzCompressTarZstdNative>>(
              'sz_compress_tar_zstd')
          .asFunction();
      _decompressTarZstd = _lib
          .lookup<NativeFunction<SzDecompressTarZstdNative>>(
              'sz_decompress_tar_zstd')
          .asFunction();
      _verifyTarZstdPassword = _lib
          .lookup<NativeFunction<SzVerifyTarZstdPasswordNative>>(
              'sz_verify_tar_zstd_password')
          .asFunction();
      _requiresTarZstdPassword = _lib
          .lookup<NativeFunction<SzRequiresTarZstdPasswordNative>>(
              'sz_requires_tar_zstd_password')
          .asFunction();
      _hasTarZstdSupport = true;
      print('Tar+Zstd 压缩支持已加载');
    } catch (e) {
      print('警告: Tar+Zstd 函数不可用: $e');
      _hasTarZstdSupport = false;
    }

    // 绑定智能解压函数（可选）
    try {
      _smartDecompress = _lib
          .lookup<NativeFunction<SzSmartDecompressNative>>(
              'sz_smart_decompress')
          .asFunction();
      _detectFormat = _lib
          .lookup<NativeFunction<SzDetectFormatNative>>('sz_detect_format')
          .asFunction();
      _smartRequiresPassword = _lib
          .lookup<NativeFunction<SzSmartRequiresPasswordNative>>(
              'sz_smart_requires_password')
          .asFunction();
      _smartVerifyPassword = _lib
          .lookup<NativeFunction<SzSmartVerifyPasswordNative>>(
              'sz_smart_verify_password')
          .asFunction();
      _hasSmartDecompressSupport = true;
      print('智能解压支持已加载');
    } catch (e) {
      print('警告: 智能解压函数不可用: $e');
      _hasSmartDecompressSupport = false;
    }
  }

  /// 压缩文件（无密码）
  /// 输出 .szp 格式（使用 Tar + ZSTD 压缩）
  RustCompressResult compress({
    required List<String> inputPaths,
    required String outputPath,
    int compressionLevel = 3,
  }) {
    if (!_isLoaded) {
      return RustCompressResult(
        success: false,
        originalSize: 0,
        compressedSize: 0,
        errorMessage: 'Rust 库未加载',
      );
    }

    _resetProgress();

    final inputStr = inputPaths.join('|');
    final inputPtr = inputStr.toNativeUtf8();
    final outputPtr = outputPath.toNativeUtf8();

    try {
      final result = _compress(inputPtr, outputPtr, compressionLevel);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustCompressResult(
        success: result.success == 1,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: error,
      );
    } finally {
      calloc.free(inputPtr);
      calloc.free(outputPtr);
    }
  }

  /// 压缩文件（带密码）
  /// 输出 .sz7z 格式（专属加密格式）
  RustCompressResult compressEncrypted({
    required List<String> inputPaths,
    required String outputPath,
    required String password,
    int compressionLevel = 3,
  }) {
    if (!_isLoaded) {
      return RustCompressResult(
        success: false,
        originalSize: 0,
        compressedSize: 0,
        errorMessage: 'Rust 库未加载',
      );
    }

    _resetProgress();

    final inputStr = inputPaths.join('|');
    final inputPtr = inputStr.toNativeUtf8();
    final outputPtr = outputPath.toNativeUtf8();
    final passwordPtr = password.toNativeUtf8();

    try {
      final result = _compressEncrypted(
          inputPtr, outputPtr, passwordPtr, compressionLevel);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustCompressResult(
        success: result.success == 1,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: error,
      );
    } finally {
      calloc.free(inputPtr);
      calloc.free(outputPtr);
      calloc.free(passwordPtr);
    }
  }

  /// 解压文件（无密码）
  RustDecompressResult decompress({
    required String archivePath,
    required String outputDir,
  }) {
    if (!_isLoaded) {
      return RustDecompressResult(
        success: false,
        fileCount: 0,
        errorMessage: 'Rust 库未加载',
      );
    }

    _resetProgress();

    final archivePtr = archivePath.toNativeUtf8();
    final outputPtr = outputDir.toNativeUtf8();

    try {
      final result = _decompress(archivePtr, outputPtr);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustDecompressResult(
        success: result.success == 1,
        fileCount: result.fileCount,
        errorMessage: error,
      );
    } finally {
      calloc.free(archivePtr);
      calloc.free(outputPtr);
    }
  }

  /// 解压文件（带密码）
  RustDecompressResult decompressEncrypted({
    required String archivePath,
    required String outputDir,
    required String password,
  }) {
    if (!_isLoaded) {
      return RustDecompressResult(
        success: false,
        fileCount: 0,
        errorMessage: 'Rust 库未加载',
      );
    }

    _resetProgress();

    final archivePtr = archivePath.toNativeUtf8();
    final outputPtr = outputDir.toNativeUtf8();
    final passwordPtr = password.toNativeUtf8();

    try {
      final result = _decompressEncrypted(archivePtr, outputPtr, passwordPtr);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustDecompressResult(
        success: result.success == 1,
        fileCount: result.fileCount,
        errorMessage: error,
      );
    } finally {
      calloc.free(archivePtr);
      calloc.free(outputPtr);
      calloc.free(passwordPtr);
    }
  }

  /// 7z 标准压缩（可被所有7z软件打开）
  RustCompressResult compress7z({
    required List<String> inputPaths,
    required String outputPath,
    int compressionLevel = 5,
  }) {
    if (!_isLoaded || !_has7zSupport) {
      return RustCompressResult(
        success: false,
        originalSize: 0,
        compressedSize: 0,
        errorMessage: '7z 压缩不可用',
      );
    }

    _resetProgress();

    final inputStr = inputPaths.join('|');
    final inputPtr = inputStr.toNativeUtf8();
    final outputPtr = outputPath.toNativeUtf8();

    try {
      final result = _compress7z(inputPtr, outputPtr, compressionLevel);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustCompressResult(
        success: result.success == 1,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: error,
      );
    } finally {
      calloc.free(inputPtr);
      calloc.free(outputPtr);
    }
  }

  /// 7z 标准加密压缩（AES-256，可被所有7z软件打开）
  RustCompressResult compress7zEncrypted({
    required List<String> inputPaths,
    required String outputPath,
    required String password,
    int compressionLevel = 5,
  }) {
    if (!_isLoaded || !_has7zSupport) {
      return RustCompressResult(
        success: false,
        originalSize: 0,
        compressedSize: 0,
        errorMessage: '7z 加密压缩不可用',
      );
    }

    _resetProgress();

    final inputStr = inputPaths.join('|');
    final inputPtr = inputStr.toNativeUtf8();
    final outputPtr = outputPath.toNativeUtf8();
    final passwordPtr = password.toNativeUtf8();

    try {
      final result = _compress7zEncrypted(
          inputPtr, outputPtr, passwordPtr, compressionLevel);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustCompressResult(
        success: result.success == 1,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: error,
      );
    } finally {
      calloc.free(inputPtr);
      calloc.free(outputPtr);
      calloc.free(passwordPtr);
    }
  }

  /// 7z 标准解压
  RustDecompressResult decompress7z({
    required String archivePath,
    required String outputDir,
    String? password,
  }) {
    if (!_isLoaded || !_has7zSupport) {
      return RustDecompressResult(
        success: false,
        fileCount: 0,
        errorMessage: '7z 解压不可用',
      );
    }

    _resetProgress();

    final archivePtr = archivePath.toNativeUtf8();
    final outputPtr = outputDir.toNativeUtf8();
    final passwordPtr = (password ?? '').toNativeUtf8();

    try {
      final result = _decompress7z(archivePtr, outputPtr, passwordPtr);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustDecompressResult(
        success: result.success == 1,
        fileCount: result.fileCount,
        errorMessage: error,
      );
    } finally {
      calloc.free(archivePtr);
      calloc.free(outputPtr);
      calloc.free(passwordPtr);
    }
  }

  /// 7z 列出压缩包内容
  List<String> list7zContents({
    required String archivePath,
    String? password,
  }) {
    if (!_isLoaded || !_has7zSupport) {
      return [];
    }

    final archivePtr = archivePath.toNativeUtf8();
    final passwordPtr = (password ?? '').toNativeUtf8();

    try {
      final resultPtr = _list7zContents(archivePtr, passwordPtr);
      if (resultPtr == nullptr) {
        return [];
      }

      final resultStr = resultPtr.toDartString();
      _freeString(resultPtr);

      if (resultStr.isEmpty) {
        return [];
      }

      return resultStr.split('|').where((s) => s.isNotEmpty).toList();
    } finally {
      calloc.free(archivePtr);
      calloc.free(passwordPtr);
    }
  }

  /// 验证密码
  bool verifyPassword({
    required String archivePath,
    required String password,
  }) {
    if (!_isLoaded) return false;

    final archivePtr = archivePath.toNativeUtf8();
    final passwordPtr = password.toNativeUtf8();

    try {
      final result = _verifyPassword(archivePtr, passwordPtr);
      return result == 1;
    } finally {
      calloc.free(archivePtr);
      calloc.free(passwordPtr);
    }
  }

  // ==========================================================================
  // Tar + Zstd 压缩方法（推荐使用）
  // ==========================================================================

  /// Tar + Zstd 压缩（可选密码加密）
  /// 使用 AES-256-GCM 加密，Argon2 密钥派生
  /// 输出 .szp 格式（SecureZip Package）
  RustCompressResult compressTarZstd({
    required List<String> inputPaths,
    required String outputPath,
    String? password,
    int compressionLevel = 3,
  }) {
    if (!_isLoaded || !_hasTarZstdSupport) {
      return RustCompressResult(
        success: false,
        originalSize: 0,
        compressedSize: 0,
        errorMessage: 'Tar+Zstd 压缩不可用',
      );
    }

    _resetProgress();

    final inputStr = inputPaths.join('|');
    final inputPtr = inputStr.toNativeUtf8();
    final outputPtr = outputPath.toNativeUtf8();
    final passwordPtr = (password ?? '').toNativeUtf8();

    try {
      final result = _compressTarZstd(
        inputPtr,
        outputPtr,
        passwordPtr,
        compressionLevel,
      );

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustCompressResult(
        success: result.success == 1,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: error,
      );
    } finally {
      calloc.free(inputPtr);
      calloc.free(outputPtr);
      calloc.free(passwordPtr);
    }
  }

  /// Tar + Zstd 解压
  RustDecompressResult decompressTarZstd({
    required String archivePath,
    required String outputDir,
    String? password,
  }) {
    if (!_isLoaded || !_hasTarZstdSupport) {
      return RustDecompressResult(
        success: false,
        fileCount: 0,
        errorMessage: 'Tar+Zstd 解压不可用',
      );
    }

    _resetProgress();

    final archivePtr = archivePath.toNativeUtf8();
    final outputPtr = outputDir.toNativeUtf8();
    final passwordPtr = (password ?? '').toNativeUtf8();

    try {
      final result = _decompressTarZstd(archivePtr, outputPtr, passwordPtr);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustDecompressResult(
        success: result.success == 1,
        fileCount: result.fileCount,
        errorMessage: error,
      );
    } finally {
      calloc.free(archivePtr);
      calloc.free(outputPtr);
      calloc.free(passwordPtr);
    }
  }

  /// 验证 Tar+Zstd 压缩包密码
  bool verifyTarZstdPassword({
    required String archivePath,
    required String password,
  }) {
    if (!_isLoaded || !_hasTarZstdSupport) return false;

    final archivePtr = archivePath.toNativeUtf8();
    final passwordPtr = password.toNativeUtf8();

    try {
      final result = _verifyTarZstdPassword(archivePtr, passwordPtr);
      return result == 1;
    } finally {
      calloc.free(archivePtr);
      calloc.free(passwordPtr);
    }
  }

  /// 检查 Tar+Zstd 压缩包是否需要密码
  bool requiresTarZstdPassword({
    required String archivePath,
  }) {
    if (!_isLoaded || !_hasTarZstdSupport) return false;

    final archivePtr = archivePath.toNativeUtf8();

    try {
      final result = _requiresTarZstdPassword(archivePtr);
      return result == 1;
    } finally {
      calloc.free(archivePtr);
    }
  }

  // ============================================================================
  // 智能解压 API
  // ============================================================================

  /// 智能解压（自动检测格式）
  /// 支持：.szp, .sz7z, .tar.zst, .7z
  RustDecompressResult smartDecompress({
    required String archivePath,
    required String outputDir,
    String? password,
  }) {
    if (!_isLoaded) {
      return RustDecompressResult(
        success: false,
        fileCount: 0,
        errorMessage: 'Rust 库未加载',
      );
    }

    // 如果智能解压不可用，尝试使用其他方法
    if (!_hasSmartDecompressSupport) {
      // 尝试 Tar+Zstd 解压
      if (_hasTarZstdSupport) {
        return decompressTarZstd(
          archivePath: archivePath,
          outputDir: outputDir,
          password: password,
        );
      }
      // 尝试标准解压
      if (password != null && password.isNotEmpty) {
        return decompressEncrypted(
          archivePath: archivePath,
          outputDir: outputDir,
          password: password,
        );
      }
      return decompress(
        archivePath: archivePath,
        outputDir: outputDir,
      );
    }

    _resetProgress();

    final archivePtr = archivePath.toNativeUtf8();
    final outputPtr = outputDir.toNativeUtf8();
    final passwordPtr = (password ?? '').toNativeUtf8();

    try {
      final result = _smartDecompress(archivePtr, outputPtr, passwordPtr);

      String? error;
      if (result.errorMessage != nullptr) {
        error = result.errorMessage.toDartString();
        _freeString(result.errorMessage);
      }

      return RustDecompressResult(
        success: result.success == 1,
        fileCount: result.fileCount,
        errorMessage: error,
      );
    } finally {
      calloc.free(archivePtr);
      calloc.free(outputPtr);
      calloc.free(passwordPtr);
    }
  }

  /// 检测压缩包格式
  /// 返回: 0=未知, 1=szp, 2=sz7z, 3=tar.zst, 4=7z
  int detectFormat({required String archivePath}) {
    if (!_isLoaded || !_hasSmartDecompressSupport) return 0;

    final archivePtr = archivePath.toNativeUtf8();

    try {
      return _detectFormat(archivePtr);
    } finally {
      calloc.free(archivePtr);
    }
  }

  /// 智能检测压缩包是否需要密码
  bool smartRequiresPassword({required String archivePath}) {
    if (!_isLoaded || !_hasSmartDecompressSupport) return false;

    final archivePtr = archivePath.toNativeUtf8();

    try {
      return _smartRequiresPassword(archivePtr) == 1;
    } finally {
      calloc.free(archivePtr);
    }
  }

  /// 智能验证密码
  bool smartVerifyPassword({
    required String archivePath,
    required String password,
  }) {
    if (!_isLoaded || !_hasSmartDecompressSupport) return false;

    final archivePtr = archivePath.toNativeUtf8();
    final passwordPtr = password.toNativeUtf8();

    try {
      return _smartVerifyPassword(archivePtr, passwordPtr) == 1;
    } finally {
      calloc.free(archivePtr);
      calloc.free(passwordPtr);
    }
  }

  /// 获取当前进度
  RustProgress getProgress() {
    if (!_isLoaded) {
      return RustProgress(current: 0, total: 0);
    }

    final result = _getProgress();
    return RustProgress(
      current: result.current,
      total: result.total,
    );
  }

  /// 重置进度
  void resetProgress() {
    if (_isLoaded) {
      _resetProgress();
    }
  }

  /// 请求取消当前操作
  void requestCancel() {
    if (_isLoaded) {
      _requestCancel();
    }
  }

  /// 检查是否已请求取消
  bool isCancelled() {
    if (!_isLoaded) return false;
    return _isCancelled() == 1;
  }
}

// ============================================================================
// 结果类
// ============================================================================

/// Rust 压缩结果
class RustCompressResult {
  final bool success;
  final int originalSize;
  final int compressedSize;
  final String? errorMessage;

  RustCompressResult({
    required this.success,
    required this.originalSize,
    required this.compressedSize,
    this.errorMessage,
  });

  /// 压缩率
  double get compressionRatio {
    if (originalSize == 0) return 0;
    return compressedSize / originalSize;
  }
}

/// Rust 解压结果
class RustDecompressResult {
  final bool success;
  final int fileCount;
  final String? errorMessage;

  RustDecompressResult({
    required this.success,
    required this.fileCount,
    this.errorMessage,
  });
}

/// Rust 进度
class RustProgress {
  final int current;
  final int total;

  RustProgress({
    required this.current,
    required this.total,
  });

  /// 进度百分比 (0.0 - 1.0)
  double get percentage {
    if (total == 0) return 0;
    return current / total;
  }
}
