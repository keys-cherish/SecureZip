import 'dart:async';
import 'dart:io';
import 'dart:isolate';
import 'dart:math';
import 'package:crypto/crypto.dart';
import 'dart:convert';
import 'package:path_provider/path_provider.dart';
import 'package:path/path.dart' as p;
import '../ffi/rust_compress_ffi.dart';
import '../models/compress_options.dart';
import '../models/mapping_entry.dart';
import 'compress_service.dart' as fallback;

/// 默认输出目录
const String _defaultAndroidOutputDir =
    '/storage/emulated/0/SecureZip/compressed';

// ============================================================================
// Isolate 通信使用 Map 而不是自定义类，因为 Isolate 只能传递基本类�?
// ============================================================================

/// 创建压缩参数 Map
Map<String, dynamic> _makeCompressParams({
  required List<String> inputPaths,
  required String outputPath,
  String? password,
  required int compressionLevel,
  required SendPort resultPort,
}) =>
    {
      'inputPaths': inputPaths,
      'outputPath': outputPath,
      'password': password,
      'compressionLevel': compressionLevel,
      'resultPort': resultPort,
    };

/// 创建 7z 压缩参数 Map
Map<String, dynamic> _make7zParams({
  required List<String> inputPaths,
  required String outputPath,
  required int compressionLevel,
  required SendPort resultPort,
}) =>
    {
      'inputPaths': inputPaths,
      'outputPath': outputPath,
      'compressionLevel': compressionLevel,
      'resultPort': resultPort,
    };

/// 创建 7z 加密压缩参数 Map
Map<String, dynamic> _make7zEncryptedParams({
  required List<String> inputPaths,
  required String outputPath,
  required String password,
  required int compressionLevel,
  required SendPort resultPort,
}) =>
    {
      'inputPaths': inputPaths,
      'outputPath': outputPath,
      'password': password,
      'compressionLevel': compressionLevel,
      'resultPort': resultPort,
    };

/// 创建解压参数 Map
Map<String, dynamic> _makeDecompressParams({
  required String archivePath,
  required String outputDir,
  String? password,
  required SendPort resultPort,
}) =>
    {
      'archivePath': archivePath,
      'outputDir': outputDir,
      'password': password,
      'resultPort': resultPort,
    };

/// 创建压缩结果 Map
Map<String, dynamic> _makeCompressResult({
  required bool success,
  required int originalSize,
  required int compressedSize,
  String? errorMessage,
}) =>
    {
      'success': success,
      'originalSize': originalSize,
      'compressedSize': compressedSize,
      'errorMessage': errorMessage,
    };

/// 创建解压结果 Map
Map<String, dynamic> _makeDecompressResult({
  required bool success,
  required int fileCount,
  String? errorMessage,
}) =>
    {
      'success': success,
      'fileCount': fileCount,
      'errorMessage': errorMessage,
    };

/// Rust 压缩服务
/// 使用 Isolate 实现真正的异步进度更�?
/// 底层使用 Rust FFI 调用 7z + ZSTD 压缩
class RustCompressService {
  final RustCompressLib _rustLib = RustCompressLib.instance;

  /// Rust 库是否可�?
  bool get isRustAvailable => _rustLib.isAvailable;

  /// 最后一次压缩生成的映射（用于混淆模式）
  List<MappingEntry> _lastMappings = [];

  /// 获取最后生成的映射列表
  List<MappingEntry> get lastMappings => _lastMappings;

  /// 清空映射
  void clearMappings() {
    _lastMappings = [];
  }

  /// 设置外部文件名映射（压缩包名混淆时使用）
  /// [obfuscatedArchiveName] 混淆后的压缩包文件名（不含路径）
  /// [originalInputNames] 原始输入文件名列�?
  /// [archivePath] 压缩包完整路�?
  void setExternalNameMapping({
    required String obfuscatedArchiveName,
    required List<String> originalInputNames,
    required String archivePath,
  }) {
    _lastMappings = [
      MappingEntry(
        // 这里 originalName 存储的是原始输入文件名（用逗号分隔多个�?
        originalName: originalInputNames.join(', '),
        // obfuscatedName 存储的是混淆后的压缩包名
        obfuscatedName: obfuscatedArchiveName,
        archivePath: archivePath,
      ),
    ];
  }

  /// 获取默认输出目录
  static Future<String> getDefaultOutputDir() async {
    if (Platform.isAndroid) {
      final dir = Directory(_defaultAndroidOutputDir);
      if (!await dir.exists()) {
        await dir.create(recursive: true);
      }
      return _defaultAndroidOutputDir;
    } else {
      final docDir = await getApplicationDocumentsDirectory();
      final outputDir = Directory('${docDir.path}/SecureZip/compressed');
      if (!await outputDir.exists()) {
        await outputDir.create(recursive: true);
      }
      return outputDir.path;
    }
  }

  /// 压缩文件或文件夹（带实时进度�?
  ///
  /// 标准模式�?z格式）：
  /// - 输出 .7z 格式，可被所�?z软件打开
  /// - 使用 ZSTD 高效压缩算法
  ///
  /// 专属模式�?
  /// - 输出 .sz7z 格式（自定义加密格式�?
  Stream<CompressProgress> compress({
    required List<String> inputPaths,
    required String outputPath,
    CompressOptions options = const CompressOptions(),
  }) async* {
    // 清空上次的映�?
    _lastMappings = [];

    // 如果 Rust 不可用或 7z 不支持，回退�?Dart 实现
    if (!isRustAvailable || !_rustLib.has7zSupport) {
      print(
          'Rust 库不可用�?z不支持，使用 Dart 实现 (isAvailable: $isRustAvailable, has7z: ${_rustLib.has7zSupport})');
      final dartService = fallback.CompressService();
      yield* dartService.compress(
        inputPaths: inputPaths,
        outputPath: outputPath,
        options: options,
      );
      return;
    }

    print('使用 Rust 压缩');

    // 专属模式：使�?tar + ZSTD + AES256 加密�?szp 后缀表示专属格式�?
    // 专属模式必须有密�?
    if (options.compressMode == CompressMode.exclusive) {
      if (options.password == null || options.password!.isEmpty) {
        throw Exception('专属模式必须设置密码');
      }

      // 检�?tar+zstd 支持
      if (!_rustLib.hasTarZstdSupport) {
        throw Exception('专属模式压缩不可用（Tar+Zstd 支持未加载）');
      }

      // 计算总大�?
      int totalBytes = 0;
      int fileCount = 0;
      for (final path in inputPaths) {
        final type = FileSystemEntity.typeSync(path);
        if (type == FileSystemEntityType.file) {
          totalBytes += await File(path).length();
          fileCount++;
        } else if (type == FileSystemEntityType.directory) {
          await for (final entity in Directory(path).list(recursive: true)) {
            if (entity is File) {
              totalBytes += await entity.length();
              fileCount++;
            }
          }
        }
      }

      // 确保输出路径使用 .szp 后缀
      String szpOutputPath = outputPath;
      if (!szpOutputPath.toLowerCase().endsWith('.szp')) {
        // 移除现有的扩展名，添�?.szp
        String basePath = szpOutputPath;
        for (final ext in ['.sz7z', '.7z', '.zip', '.tar.zst']) {
          if (basePath.toLowerCase().endsWith(ext)) {
            basePath = basePath.substring(0, basePath.length - ext.length);
            break;
          }
        }
        szpOutputPath = '$basePath.szp';
      }

      // 使用 tar + ZSTD + AES256 加密
      yield* _compressTarZstd(
        inputPaths: inputPaths,
        outputPath: szpOutputPath,
        password: options.password!,
        totalBytes: totalBytes,
        fileCount: fileCount,
        compressionLevel: options.compressionLevel,
      );
      return;
    }

    // 验证输入
    if (inputPaths.isEmpty) {
      throw Exception('输入路径不能为空');
    }

    // 处理文件名混�?
    List<String> actualInputPaths = inputPaths;
    _ObfuscationPrepResult? obfuscationResult;

    // 文件名混淆：只混淆输出压缩包的文件名，不混淆内部文件
    // 映射关系在压缩完成后�?compress_page 传入
    if (options.enableObfuscation) {
      yield CompressProgress(
        progress: 0.0,
        processedBytes: 0,
        totalBytes: 1,
        speedBytesPerSecond: 0,
        estimatedRemaining: Duration.zero,
        currentFile: '准备压缩...',
      );
      // 不再对内部文件进行混淆，直接使用原始路径
      // actualInputPaths 保持不变
    }

    try {
      // 计算总大小（用于进度估算�?
      int totalBytes = 0;
      int fileCount = 0;
      for (final path in actualInputPaths) {
        final type = FileSystemEntity.typeSync(path);
        if (type == FileSystemEntityType.file) {
          totalBytes += await File(path).length();
          fileCount++;
        } else if (type == FileSystemEntityType.directory) {
          await for (final entity in Directory(path).list(recursive: true)) {
            if (entity is File) {
              totalBytes += await entity.length();
              fileCount++;
            }
          }
        }
      }

      // 确定是否有密�?
      final hasPassword =
          options.password != null && options.password!.isNotEmpty;

      // 根据用户选择的后缀确定压缩格式
      final ext = options.fileExtension?.toLowerCase() ?? '.7z';
      final bool use7z = ext == '.7z' || ext == '.zip';

      String finalOutputPath = outputPath;

      // 获取基础名称（不含扩展名�?
      String basePath = outputPath;
      for (final knownExt in ['.sz7z', '.7z', '.zip']) {
        if (outputPath.toLowerCase().endsWith(knownExt)) {
          basePath =
              outputPath.substring(0, outputPath.length - knownExt.length);
          break;
        }
      }

      // 确定实际输出格式
      if (use7z && !hasPassword) {
        // 使用标准7z格式（Rust FFI�?
        finalOutputPath = '$basePath.7z';
        yield* _compress7z(
          inputPaths: actualInputPaths,
          outputPath: finalOutputPath,
          totalBytes: totalBytes,
          fileCount: fileCount,
          compressionLevel: options.compressionLevel,
        );
        return;
      } else if (use7z && hasPassword) {
        // 使用标准7z加密格式（AES-256，可被所�?z软件打开�?
        finalOutputPath = '$basePath.7z';
        yield* _compress7zEncrypted(
          inputPaths: actualInputPaths,
          outputPath: finalOutputPath,
          password: options.password!,
          totalBytes: totalBytes,
          fileCount: fileCount,
          compressionLevel: options.compressionLevel,
        );
        return;
      } else if (hasPassword) {
        // 默认使用 7z 加密格式
        finalOutputPath = '$basePath.7z';
        yield* _compress7zEncrypted(
          inputPaths: actualInputPaths,
          outputPath: finalOutputPath,
          password: options.password!,
          totalBytes: totalBytes,
          fileCount: fileCount,
          compressionLevel: options.compressionLevel,
        );
        return;
      } else {
        // 默认使用7z格式
        finalOutputPath = '$basePath.7z';
        yield* _compress7z(
          inputPaths: actualInputPaths,
          outputPath: finalOutputPath,
          totalBytes: totalBytes,
          fileCount: fileCount,
          compressionLevel: options.compressionLevel,
        );
        return;
      }

      // 确保输出目录存在
      final outputFile = File(finalOutputPath);
      final outputDir = outputFile.parent;
      if (!await outputDir.exists()) {
        await outputDir.create(recursive: true);
      }

      // 重置 Rust 进度计数�?
      _rustLib.resetProgress();

      final startTime = DateTime.now();

      // 发送初始进�?
      yield CompressProgress(
        progress: 0.0,
        processedBytes: 0,
        totalBytes: totalBytes,
        speedBytesPerSecond: 0,
        estimatedRemaining: Duration.zero,
        currentFile: hasPassword
            ? '准备压缩加密 ($fileCount 个文�?...'
            : '准备压缩 ($fileCount 个文�?...',
      );

      // 创建结果接收端口
      final resultPort = ReceivePort();
      Isolate? isolate;
      bool completed = false;
      String? error;
      Map<String, dynamic>? result;

      // �?Isolate 中执行压缩（不阻塞主线程�?
      try {
        isolate = await Isolate.spawn(
          _compressInIsolate,
          <String, dynamic>{
            'inputPaths': inputPaths,
            'outputPath': finalOutputPath,
            'password': hasPassword ? options.password : null,
            'compressionLevel': options.compressionLevel,
            'resultPort': resultPort.sendPort,
          },
        );
      } catch (e) {
        resultPort.close();
        throw Exception('启动压缩任务失败: $e');
      }

      // 监听 Isolate 结果
      final subscription = resultPort.listen((message) {
        if (message is Map<String, dynamic>) {
          result = message;
          completed = true;
          final success = message['success'] as bool? ?? false;
          if (!success) {
            error = (message['errorMessage'] as String?) ?? '压缩失败';
          }
        } else if (message is String && message.startsWith('ERROR:')) {
          error = message.substring(6);
          completed = true;
        }
      });

      // 轮询进度（每50ms更新一次，实现更平滑的动画）
      int lastProgressCurrent = 0;
      int stuckCount = 0;

      // 平滑进度插值参数
      double displayProgress = 0.0; // 当前显示的进度
      double targetProgress = 0.0; // 目标进度（来自 Rust）
      const double smoothingFactor = 0.12; // 插值因子（越小越平滑）
      const int pollIntervalMs = 50; // 轮询间隔

      while (!completed) {
        await Future.delayed(const Duration(milliseconds: pollIntervalMs));

        final progress = _rustLib.getProgress();
        final elapsed = DateTime.now().difference(startTime);

        // 检查进度是否卡住
        if (progress.current == lastProgressCurrent && progress.current > 0) {
          stuckCount++;
        } else {
          stuckCount = 0;
          lastProgressCurrent = progress.current;
        }

        // 使用 Rust 报告的 total，如果为 0 则使用预计算的 totalBytes
        final progressTotal = progress.total > 0 ? progress.total : totalBytes;

        // 计算目标进度（来自 Rust）
        if (progressTotal > 0) {
          targetProgress = progress.current / progressTotal;
        }

        // 平滑进度插值：让显示进度平滑过渡到目标进度
        // 这样即使 Rust 进度跳跃，显示也会平滑
        if (displayProgress < targetProgress) {
          // 正常向前进度：使用指数平滑
          displayProgress +=
              (targetProgress - displayProgress) * smoothingFactor;
          // 确保不超过目标
          if (displayProgress > targetProgress) {
            displayProgress = targetProgress;
          }
        } else if (targetProgress >= 0.99) {
          // 接近完成时直接跳到 100%
          displayProgress = targetProgress;
        }

        // 如果进度卡住但已经过了一段时间，模拟缓慢增长（让用户知道还在处理）
        if (stuckCount > 10 &&
            displayProgress < 0.95 &&
            displayProgress > 0.05) {
          // 卡住时缓慢增加进度（每次约0.1%），让用户知道还在处理
          final simulatedIncrement =
              0.001 * (1.0 - displayProgress); // 越接近100%增长越慢
          displayProgress += simulatedIncrement;
          if (displayProgress > 0.95) {
            displayProgress = 0.95; // 模拟进度最高到95%
          }
        }

        // 计算速度
        final speedBps = elapsed.inMilliseconds > 0
            ? progress.current / (elapsed.inMilliseconds / 1000)
            : 0.0;

        // 计算剩余时间
        final remainingBytes = progressTotal - progress.current;
        final remainingSeconds =
            speedBps > 0 ? (remainingBytes / speedBps).round() : 0;

        // 确定当前阶段的显示文本
        String currentFile;
        if (displayProgress < 0.3) {
          currentFile = '读取文件...';
        } else if (displayProgress < 0.7) {
          currentFile = 'Zstd 压缩中...';
        } else if (hasPassword && displayProgress < 0.95) {
          currentFile = 'AES-256 加密中...';
        } else if (displayProgress < 0.99) {
          currentFile = '写入文件...';
        } else {
          currentFile = '即将完成...';
        }

        yield CompressProgress(
          progress: displayProgress.clamp(0.0, 1.0),
          processedBytes: progress.current,
          totalBytes: progressTotal,
          speedBytesPerSecond: speedBps,
          estimatedRemaining: Duration(seconds: remainingSeconds),
          currentFile: currentFile,
        );

        // 如果进度长时间卡住（超过120秒），认为可能完成了
        if (stuckCount > 2400) {
          break;
        }
      }

      // 清理资源
      await subscription.cancel();
      resultPort.close();
      isolate.kill(priority: Isolate.immediate);

      // 检查错�?
      if (error != null) {
        throw Exception(error);
      }

      // 计算最终统�?
      final duration = DateTime.now().difference(startTime);
      final compressedSize = (result?['compressedSize'] as int?) ?? 0;
      final originalSize = (result?['originalSize'] as int?) ?? totalBytes;

      // 计算压缩�?
      final ratio = originalSize > 0
          ? (compressedSize / originalSize * 100).toStringAsFixed(1)
          : '0';

      // 发送最终进�?
      yield CompressProgress(
        progress: 1.0,
        processedBytes: originalSize,
        totalBytes: originalSize,
        speedBytesPerSecond: duration.inMilliseconds > 0
            ? originalSize / (duration.inMilliseconds / 1000)
            : originalSize.toDouble(),
        estimatedRemaining: Duration.zero,
        currentFile: '完成！压缩率: $ratio%, 共$fileCount 个文件',
      );
    } finally {
      // 清理临时目录
      if (obfuscationResult != null) {
        await _cleanupTempDir(obfuscationResult.tempDir);
      }
    }
  }

  /// 7z标准压缩（可被所�?z软件打开�?
  Stream<CompressProgress> _compress7z({
    required List<String> inputPaths,
    required String outputPath,
    required int totalBytes,
    required int fileCount,
    int compressionLevel = 5,
  }) async* {
    // 检�?7z 支持
    if (!_rustLib.isAvailable || !_rustLib.has7zSupport) {
      throw Exception(
          '7z 压缩不可�?(isAvailable: ${_rustLib.isAvailable}, has7z: ${_rustLib.has7zSupport})');
    }

    // 确保输出目录存在
    final outputFile = File(outputPath);
    final outputDir = outputFile.parent;
    if (!await outputDir.exists()) {
      await outputDir.create(recursive: true);
    }

    _rustLib.resetProgress();
    final startTime = DateTime.now();

    yield CompressProgress(
      progress: 0.0,
      processedBytes: 0,
      totalBytes: totalBytes,
      speedBytesPerSecond: 0,
      estimatedRemaining: Duration.zero,
      currentFile: '准备7z压缩 ($fileCount 个文�?...',
    );

    // 创建结果接收端口
    final resultPort = ReceivePort();
    Isolate? isolate;
    bool completed = false;
    String? error;
    Map<String, dynamic>? result;

    // �?Isolate 中执�?z压缩
    try {
      isolate = await Isolate.spawn(
        _compress7zInIsolate,
        _make7zParams(
          inputPaths: inputPaths,
          outputPath: outputPath,
          compressionLevel: compressionLevel,
          resultPort: resultPort.sendPort,
        ),
      );
    } catch (e) {
      resultPort.close();
      throw Exception('启动7z压缩任务失败: $e');
    }

    final subscription = resultPort.listen((message) {
      if (message is Map<String, dynamic>) {
        result = message;
        completed = true;
        if (message['success'] != true) {
          error = message['errorMessage'] as String? ?? '7z压缩失败';
        }
      } else if (message is String && message.startsWith('ERROR:')) {
        error = message.substring(6);
        completed = true;
      }
    });

    // 轮询进度（平滑动画）
    double displayProgress = 0.0;
    double targetProgress = 0.0;
    const double smoothingFactor = 0.12;
    int stuckCount = 0;
    int lastProgressCurrent = 0;

    while (!completed) {
      await Future.delayed(const Duration(milliseconds: 50));

      final progress = _rustLib.getProgress();
      final elapsed = DateTime.now().difference(startTime);
      final progressTotal = progress.total > 0 ? progress.total : totalBytes;

      // 检查卡住
      if (progress.current == lastProgressCurrent && progress.current > 0) {
        stuckCount++;
      } else {
        stuckCount = 0;
        lastProgressCurrent = progress.current;
      }

      // 计算目标进度
      if (progressTotal > 0) {
        targetProgress = progress.current / progressTotal;
      }

      // 平滑插值
      if (displayProgress < targetProgress) {
        displayProgress += (targetProgress - displayProgress) * smoothingFactor;
        if (displayProgress > targetProgress) displayProgress = targetProgress;
      } else if (targetProgress >= 0.99) {
        displayProgress = targetProgress;
      }

      // 卡住时模拟进度
      if (stuckCount > 10 && displayProgress < 0.95 && displayProgress > 0.05) {
        displayProgress += 0.001 * (1.0 - displayProgress);
        if (displayProgress > 0.95) displayProgress = 0.95;
      }

      final speedBps = elapsed.inMilliseconds > 0
          ? progress.current / (elapsed.inMilliseconds / 1000)
          : 0.0;
      final remainingBytes = progressTotal - progress.current;
      final remainingSeconds =
          speedBps > 0 ? (remainingBytes / speedBps).round() : 0;

      yield CompressProgress(
        progress: displayProgress.clamp(0.0, 1.0),
        processedBytes: progress.current,
        totalBytes: progressTotal,
        speedBytesPerSecond: speedBps,
        estimatedRemaining: Duration(seconds: remainingSeconds),
        currentFile: '7z LZMA2 压缩中...',
      );
    }

    await subscription.cancel();
    resultPort.close();
    isolate.kill(priority: Isolate.immediate);

    if (error != null) {
      throw Exception(error);
    }

    final duration = DateTime.now().difference(startTime);
    final compressedSize = (result?['compressedSize'] as int?) ?? 0;
    final originalSize = (result?['originalSize'] as int?) ?? totalBytes;
    final ratio = originalSize > 0
        ? (compressedSize / originalSize * 100).toStringAsFixed(1)
        : '0';

    yield CompressProgress(
      progress: 1.0,
      processedBytes: originalSize,
      totalBytes: originalSize,
      speedBytesPerSecond: duration.inMilliseconds > 0
          ? originalSize / (duration.inMilliseconds / 1000)
          : originalSize.toDouble(),
      estimatedRemaining: Duration.zero,
      currentFile: '完成！压缩率: $ratio%, 共$fileCount 个文件',
    );
  }

  /// 7z 加密压缩（内部方法）
  Stream<CompressProgress> _compress7zEncrypted({
    required List<String> inputPaths,
    required String outputPath,
    required String password,
    required int totalBytes,
    required int fileCount,
    int compressionLevel = 5,
  }) async* {
    // 检�?7z 支持
    if (!_rustLib.isAvailable || !_rustLib.has7zSupport) {
      throw Exception(
          '7z 加密压缩不可�?(isAvailable: ${_rustLib.isAvailable}, has7z: ${_rustLib.has7zSupport})');
    }

    // 确保输出目录存在
    final outputFile = File(outputPath);
    final outputDir = outputFile.parent;
    if (!await outputDir.exists()) {
      await outputDir.create(recursive: true);
    }

    _rustLib.resetProgress();
    final startTime = DateTime.now();

    yield CompressProgress(
      progress: 0.0,
      processedBytes: 0,
      totalBytes: totalBytes,
      speedBytesPerSecond: 0,
      estimatedRemaining: Duration.zero,
      currentFile: '准备7z加密压缩 ($fileCount 个文�?...',
    );

    // 创建结果接收端口
    final resultPort = ReceivePort();
    Isolate? isolate;
    bool completed = false;
    String? error;
    Map<String, dynamic>? result;

    // �?Isolate 中执�?z加密压缩
    try {
      isolate = await Isolate.spawn(
        _compress7zEncryptedInIsolate,
        _make7zEncryptedParams(
          inputPaths: inputPaths,
          outputPath: outputPath,
          password: password,
          compressionLevel: compressionLevel,
          resultPort: resultPort.sendPort,
        ),
      );
    } catch (e) {
      resultPort.close();
      throw Exception('启动7z加密压缩任务失败: $e');
    }

    final subscription = resultPort.listen((message) {
      if (message is Map<String, dynamic>) {
        result = message;
        completed = true;
        if (message['success'] != true) {
          error = message['errorMessage'] as String? ?? '7z加密压缩失败';
        }
      } else if (message is String && message.startsWith('ERROR:')) {
        error = message.substring(6);
        completed = true;
      }
    });

    // 轮询进度（平滑动画）
    double displayProgress = 0.0;
    double targetProgress = 0.0;
    const double smoothingFactor = 0.12;
    int stuckCount = 0;
    int lastProgressCurrent = 0;

    while (!completed) {
      await Future.delayed(const Duration(milliseconds: 50));

      final progress = _rustLib.getProgress();
      final elapsed = DateTime.now().difference(startTime);
      final progressTotal = progress.total > 0 ? progress.total : totalBytes;

      // 检查卡住
      if (progress.current == lastProgressCurrent && progress.current > 0) {
        stuckCount++;
      } else {
        stuckCount = 0;
        lastProgressCurrent = progress.current;
      }

      // 计算目标进度
      if (progressTotal > 0) {
        targetProgress = progress.current / progressTotal;
      }

      // 平滑插值
      if (displayProgress < targetProgress) {
        displayProgress += (targetProgress - displayProgress) * smoothingFactor;
        if (displayProgress > targetProgress) displayProgress = targetProgress;
      } else if (targetProgress >= 0.99) {
        displayProgress = targetProgress;
      }

      // 卡住时模拟进度
      if (stuckCount > 10 && displayProgress < 0.95 && displayProgress > 0.05) {
        displayProgress += 0.001 * (1.0 - displayProgress);
        if (displayProgress > 0.95) displayProgress = 0.95;
      }

      final speedBps = elapsed.inMilliseconds > 0
          ? progress.current / (elapsed.inMilliseconds / 1000)
          : 0.0;
      final remainingBytes = progressTotal - progress.current;
      final remainingSeconds =
          speedBps > 0 ? (remainingBytes / speedBps).round() : 0;

      yield CompressProgress(
        progress: displayProgress.clamp(0.0, 1.0),
        processedBytes: progress.current,
        totalBytes: progressTotal,
        speedBytesPerSecond: speedBps,
        estimatedRemaining: Duration(seconds: remainingSeconds),
        currentFile: '7z AES-256 加密压缩中...',
      );
    }

    await subscription.cancel();
    resultPort.close();
    isolate.kill(priority: Isolate.immediate);

    if (error != null) {
      throw Exception(error);
    }

    final duration = DateTime.now().difference(startTime);
    final compressedSize = (result?['compressedSize'] as int?) ?? 0;
    final originalSize = (result?['originalSize'] as int?) ?? totalBytes;
    final ratio = originalSize > 0
        ? (compressedSize / originalSize * 100).toStringAsFixed(1)
        : '0';

    yield CompressProgress(
      progress: 1.0,
      processedBytes: originalSize,
      totalBytes: originalSize,
      speedBytesPerSecond: duration.inMilliseconds > 0
          ? originalSize / (duration.inMilliseconds / 1000)
          : originalSize.toDouble(),
      estimatedRemaining: Duration.zero,
      currentFile: '完成！压缩率: $ratio%, 共$fileCount 个文件（AES-256加密）',
    );
  }

  /// Tar + Zstd + AES256 加密压缩（专属模式）
  /// 输出 .szp 格式（SecureZip Package）
  /// 使用 Isolate 在后台执行，通过轮询进度更新 UI
  Stream<CompressProgress> _compressTarZstd({
    required List<String> inputPaths,
    required String outputPath,
    required String password,
    required int totalBytes,
    required int fileCount,
    int compressionLevel = 3,
  }) async* {
    // 检查支�?
    if (!_rustLib.isAvailable || !_rustLib.hasTarZstdSupport) {
      throw Exception(
          'Tar+Zstd 加密压缩不可�?(isAvailable: ${_rustLib.isAvailable}, hasTarZstd: ${_rustLib.hasTarZstdSupport})');
    }

    // 确保输出目录存在
    final outputFile = File(outputPath);
    final outputDir = outputFile.parent;
    if (!await outputDir.exists()) {
      await outputDir.create(recursive: true);
    }

    _rustLib.resetProgress();
    final startTime = DateTime.now();

    yield CompressProgress(
      progress: 0.0,
      processedBytes: 0,
      totalBytes: totalBytes,
      speedBytesPerSecond: 0,
      estimatedRemaining: Duration.zero,
      currentFile: '准备专属格式压缩 ($fileCount 个文�?...',
    );

    // 创建结果接收端口
    final resultPort = ReceivePort();
    Isolate? isolate;
    bool completed = false;
    String? error;
    Map<String, dynamic>? result;

    // �?Isolate 中执行压�?
    try {
      isolate = await Isolate.spawn(
        _compressTarZstdInIsolate,
        <String, dynamic>{
          'inputPaths': inputPaths,
          'outputPath': outputPath,
          'password': password,
          'compressionLevel': compressionLevel,
          'resultPort': resultPort.sendPort,
        },
      );
    } catch (e) {
      resultPort.close();
      throw Exception('启动Tar+Zstd压缩任务失败: $e');
    }

    // 监听 Isolate 结果
    final subscription = resultPort.listen((message) {
      if (message is Map<String, dynamic>) {
        result = message;
        completed = true;
        final success = message['success'] as bool? ?? false;
        if (!success) {
          error = (message['errorMessage'] as String?) ?? 'Tar+Zstd压缩失败';
        }
      } else if (message is String && message.startsWith('ERROR:')) {
        error = message.substring(6);
        completed = true;
      }
    });

    // 轮询进度 - 使用平滑动画
    // 注意：在 Flutter 中，Isolate 加载同一个动态库时共享全局变量
    // 因此可以从主线程读取 Rust 设置的进度
    int lastCurrentBytes = 0;
    DateTime lastSpeedUpdate = DateTime.now();
    double smoothedSpeed = 0.0;

    // 平滑进度动画参数
    double displayProgress = 0.0;
    double targetProgress = 0.0;
    const smoothingFactor = 0.12;
    int stuckCount = 0;
    double lastTargetProgress = 0.0;

    while (!completed) {
      await Future.delayed(const Duration(milliseconds: 50)); // 更频繁的更新

      final now = DateTime.now();
      final elapsed = now.difference(startTime);
      final progress = _rustLib.getProgress();

      // 计算进度百分比
      int currentBytes = progress.current;

      // 使用 Rust 报告的进度
      final progressTotal = progress.total > 0 ? progress.total : totalBytes;
      if (progressTotal > 0) {
        targetProgress = currentBytes / progressTotal;
      }

      // 检测进度是否卡住
      if ((targetProgress - lastTargetProgress).abs() < 0.001) {
        stuckCount++;
      } else {
        stuckCount = 0;
        lastTargetProgress = targetProgress;
      }

      // 平滑插值 - 使用指数插值让进度更平滑
      displayProgress += (targetProgress - displayProgress) * smoothingFactor;

      // 如果进度卡住，添加微小的模拟进度
      if (stuckCount > 10 && displayProgress < 0.95) {
        displayProgress += 0.001 * (1.0 - displayProgress);
      }

      // 确保进度不超过目标太多
      if (displayProgress > targetProgress + 0.05) {
        displayProgress = targetProgress + 0.05;
      }

      // 平滑速度计算（使用增量）
      final timeSinceLastUpdate =
          now.difference(lastSpeedUpdate).inMilliseconds;
      if (timeSinceLastUpdate >= 200 && currentBytes > lastCurrentBytes) {
        final bytesPerSec =
            (currentBytes - lastCurrentBytes) * 1000.0 / timeSinceLastUpdate;
        // 指数移动平均
        smoothedSpeed = smoothedSpeed == 0
            ? bytesPerSec
            : smoothedSpeed * 0.7 + bytesPerSec * 0.3;
        lastCurrentBytes = currentBytes;
        lastSpeedUpdate = now;
      }

      // 如果还没有速度数据，使用总体平均速度
      final speedBps = smoothedSpeed > 0
          ? smoothedSpeed
          : (elapsed.inMilliseconds > 0
              ? currentBytes / (elapsed.inMilliseconds / 1000)
              : 0.0);

      // 计算剩余时间
      final remainingBytes = progressTotal - currentBytes;
      final remainingSeconds =
          speedBps > 0 ? (remainingBytes / speedBps).round() : 0;

      // 确定当前阶段的显示文字
      String currentFile;
      if (displayProgress < 0.95) {
        currentFile = 'Tar + Zstd 压缩中...';
      } else if (displayProgress < 1.0) {
        currentFile = '正在完成压缩...';
      } else {
        currentFile = '完成';
      }

      yield CompressProgress(
        progress: displayProgress.clamp(0.0, 1.0), // 使用平滑后的进度
        processedBytes: currentBytes,
        totalBytes: progressTotal,
        speedBytesPerSecond: speedBps,
        estimatedRemaining: Duration(seconds: remainingSeconds),
        currentFile: currentFile,
      );

      // 防止无限循环（超�?0分钟�?
      if (elapsed.inMinutes > 30) {
        error = '压缩超时';
        break;
      }
    }

    // 清理资源
    await subscription.cancel();
    resultPort.close();
    if (isolate != null) {
      isolate.kill(priority: Isolate.immediate);
    }

    // 检查错�?
    if (error != null) {
      throw Exception(error);
    }

    final duration = DateTime.now().difference(startTime);
    final compressedSize = (result?['compressedSize'] as int?) ?? 0;
    final originalSize = (result?['originalSize'] as int?) ?? totalBytes;
    final ratio = originalSize > 0
        ? (compressedSize / originalSize * 100).toStringAsFixed(1)
        : '0';

    yield CompressProgress(
      progress: 1.0,
      processedBytes: originalSize,
      totalBytes: originalSize,
      speedBytesPerSecond: duration.inMilliseconds > 0
          ? originalSize / (duration.inMilliseconds / 1000)
          : originalSize.toDouble(),
      estimatedRemaining: Duration.zero,
      currentFile: '完成！压缩率: $ratio%, 共$fileCount 个文件（AES-256加密）',
    );
  }

  /// 在 Isolate 中执行压缩
  static void _compressInIsolate(Map<String, dynamic> params) {
    try {
      // 在 Isolate 中获取 Rust 库实例
      final rustLib = RustCompressLib.instance;
      final resultPort = params['resultPort'] as SendPort;

      if (!rustLib.isAvailable) {
        resultPort.send('ERROR:Rust 库不可用');
        return;
      }

      final inputPaths = (params['inputPaths'] as List).cast<String>();
      final outputPath = params['outputPath'] as String;
      final password = params['password'] as String?;
      final compressionLevel = params['compressionLevel'] as int;

      RustCompressResult result;

      if (password != null && password.isNotEmpty) {
        // 有密码：使用加密压缩�?z + ZSTD + AES-256-GCM�?
        result = rustLib.compressEncrypted(
          inputPaths: inputPaths,
          outputPath: outputPath,
          password: password!,
          compressionLevel: compressionLevel,
        );
      } else {
        // 无密码：普通压缩（7z + ZSTD�?
        result = rustLib.compress(
          inputPaths: inputPaths,
          outputPath: outputPath,
          compressionLevel: compressionLevel,
        );
      }

      resultPort.send(_makeCompressResult(
        success: result.success,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: result.errorMessage,
      ));
    } catch (e) {
      final resultPort = params['resultPort'] as SendPort;
      resultPort.send('ERROR:$e');
    }
  }

  /// �?Isolate 中执�?z压缩
  static void _compress7zInIsolate(Map<String, dynamic> params) {
    final resultPort = params['resultPort'] as SendPort;
    try {
      final rustLib = RustCompressLib.instance;

      if (!rustLib.isAvailable || !rustLib.has7zSupport) {
        resultPort.send('ERROR:7z 压缩不可用');
        return;
      }

      final inputPaths = (params['inputPaths'] as List).cast<String>();
      final outputPath = params['outputPath'] as String;
      final compressionLevel = params['compressionLevel'] as int;

      final result = rustLib.compress7z(
        inputPaths: inputPaths,
        outputPath: outputPath,
        compressionLevel: compressionLevel,
      );

      resultPort.send(_makeCompressResult(
        success: result.success,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: result.errorMessage,
      ));
    } catch (e) {
      resultPort.send('ERROR:$e');
    }
  }

  /// �?Isolate 中执�?z加密压缩
  static void _compress7zEncryptedInIsolate(Map<String, dynamic> params) {
    final resultPort = params['resultPort'] as SendPort;
    try {
      final rustLib = RustCompressLib.instance;

      if (!rustLib.isAvailable || !rustLib.has7zSupport) {
        resultPort.send('ERROR:7z 加密压缩不可用');
        return;
      }

      final inputPaths = (params['inputPaths'] as List).cast<String>();
      final outputPath = params['outputPath'] as String;
      final password = params['password'] as String;
      final compressionLevel = params['compressionLevel'] as int;

      final result = rustLib.compress7zEncrypted(
        inputPaths: inputPaths,
        outputPath: outputPath,
        password: password,
        compressionLevel: compressionLevel,
      );

      resultPort.send(_makeCompressResult(
        success: result.success,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: result.errorMessage,
      ));
    } catch (e) {
      resultPort.send('ERROR:$e');
    }
  }

  /// �?Isolate 中执�?Tar+Zstd+AES256 加密压缩（专属模式）
  static void _compressTarZstdInIsolate(Map<String, dynamic> params) {
    final resultPort = params['resultPort'] as SendPort;
    try {
      final rustLib = RustCompressLib.instance;

      if (!rustLib.isAvailable || !rustLib.hasTarZstdSupport) {
        resultPort.send('ERROR:Tar+Zstd 加密压缩不可用');
        return;
      }

      final inputPaths = (params['inputPaths'] as List).cast<String>();
      final outputPath = params['outputPath'] as String;
      final password = params['password'] as String;
      final compressionLevel = params['compressionLevel'] as int;

      final result = rustLib.compressTarZstd(
        inputPaths: inputPaths,
        outputPath: outputPath,
        password: password,
        compressionLevel: compressionLevel,
      );

      resultPort.send(_makeCompressResult(
        success: result.success,
        originalSize: result.originalSize,
        compressedSize: result.compressedSize,
        errorMessage: result.errorMessage,
      ));
    } catch (e) {
      resultPort.send('ERROR:$e');
    }
  }

  /// 解压文件（带实时进度�?
  Stream<CompressProgress> decompress({
    required String archivePath,
    required String outputDir,
    String? password,
  }) async* {
    if (!isRustAvailable) {
      final dartService = fallback.CompressService();
      yield* dartService.decompress(
        archivePath: archivePath,
        outputDir: outputDir,
        password: password,
      );
      return;
    }

    final archiveFile = File(archivePath);
    if (!await archiveFile.exists()) {
      throw Exception('压缩包不存在: $archivePath');
    }

    final totalBytes = await archiveFile.length();

    yield CompressProgress(
      progress: 0.0,
      processedBytes: 0,
      totalBytes: totalBytes,
      speedBytesPerSecond: 0,
      estimatedRemaining: Duration.zero,
      currentFile: '准备解压...',
    );

    _rustLib.resetProgress();

    final startTime = DateTime.now();

    // 判断文件类型
    final lowerPath = archivePath.toLowerCase();
    final isSzpFile = lowerPath.endsWith('.szp');
    final isTarZst =
        lowerPath.endsWith('.tar.zst') || lowerPath.endsWith('.zst');
    final isEncrypted =
        lowerPath.endsWith('.enc') || lowerPath.endsWith('.sz7z') || isSzpFile;

    // .szp �?.tar.zst 文件使用 Tar+Zstd 解压
    if (isSzpFile || isTarZst) {
      if (!_rustLib.hasTarZstdSupport) {
        throw Exception('专属格式解压不可用（Tar+Zstd 支持未加载）');
      }

      yield* _decompressTarZstd(
        archivePath: archivePath,
        outputDir: outputDir,
        password: password,
        totalBytes: totalBytes,
      );
      return;
    }

    // 创建结果端口
    final resultPort = ReceivePort();
    bool completed = false;
    String? error;
    int fileCount = 0;
    Isolate? isolate;

    try {
      isolate = await Isolate.spawn(
        _decompressInIsolate,
        <String, dynamic>{
          'archivePath': archivePath,
          'outputDir': outputDir,
          'password': isEncrypted ? password : null,
          'resultPort': resultPort.sendPort,
        },
      );
    } catch (e) {
      resultPort.close();
      throw Exception('启动解压任务失败: $e');
    }

    // 监听结果
    final subscription = resultPort.listen((message) {
      if (message is Map<String, dynamic>) {
        completed = true;
        fileCount = (message['fileCount'] as int?) ?? 0;
        final success = message['success'] as bool? ?? false;
        if (!success) {
          error = (message['errorMessage'] as String?) ?? '解压失败';
        }
      } else if (message is String && message.startsWith('ERROR:')) {
        error = message.substring(6);
        completed = true;
      }
    });

    // 轮询进度 - 使用平滑动画
    double displayProgress = 0.0;
    double targetProgress = 0.0;
    const smoothingFactor = 0.15;
    int stuckCount = 0;
    double lastTargetProgress = 0.0;

    while (!completed) {
      await Future.delayed(const Duration(milliseconds: 50));

      final progress = _rustLib.getProgress();
      final elapsed = DateTime.now().difference(startTime);

      final progressTotal = progress.total > 0 ? progress.total : totalBytes;

      if (progressTotal > 0) {
        targetProgress = progress.current / progressTotal;
      }

      // 检测进度是否卡住
      if ((targetProgress - lastTargetProgress).abs() < 0.001) {
        stuckCount++;
      } else {
        stuckCount = 0;
        lastTargetProgress = targetProgress;
      }

      // 平滑插值
      displayProgress += (targetProgress - displayProgress) * smoothingFactor;

      // 如果进度卡住，添加微小的模拟进度
      if (stuckCount > 10 && displayProgress < 0.95) {
        displayProgress += 0.001 * (1.0 - displayProgress);
      }

      // 确保进度不超过目标太多
      if (displayProgress > targetProgress + 0.05) {
        displayProgress = targetProgress + 0.05;
      }

      final speedBps = elapsed.inMilliseconds > 0
          ? progress.current / (elapsed.inMilliseconds / 1000)
          : 0.0;

      String currentFile;
      if (isEncrypted && displayProgress < 0.3) {
        currentFile = '解密中...';
      } else if (displayProgress < 0.6) {
        currentFile = 'Zstd 解压中...';
      } else {
        currentFile = '写入文件...';
      }

      yield CompressProgress(
        progress: displayProgress.clamp(0.0, 1.0),
        processedBytes: progress.current,
        totalBytes: progressTotal,
        speedBytesPerSecond: speedBps,
        estimatedRemaining: Duration.zero,
        currentFile: currentFile,
      );
    }

    await subscription.cancel();
    resultPort.close();
    isolate.kill(priority: Isolate.immediate);

    if (error != null) {
      throw Exception(error);
    }

    final duration = DateTime.now().difference(startTime);

    yield CompressProgress(
      progress: 1.0,
      processedBytes: totalBytes,
      totalBytes: totalBytes,
      speedBytesPerSecond: duration.inMilliseconds > 0
          ? totalBytes / (duration.inMilliseconds / 1000)
          : totalBytes.toDouble(),
      estimatedRemaining: Duration.zero,
      currentFile: '完成！已解压 $fileCount 个文件',
    );
  }

  /// Tar + Zstd 解压（专属模式）
  Stream<CompressProgress> _decompressTarZstd({
    required String archivePath,
    required String outputDir,
    String? password,
    required int totalBytes,
  }) async* {
    if (!_rustLib.isAvailable || !_rustLib.hasTarZstdSupport) {
      throw Exception('Tar+Zstd 解压不可用');
    }

    // 检查是否需要密码
    final requiresPassword = _rustLib.requiresTarZstdPassword(
      archivePath: archivePath,
    );

    if (requiresPassword && (password == null || password.isEmpty)) {
      throw Exception('此压缩包需要密码');
    }

    // 确保输出目录存在
    final outputDirFile = Directory(outputDir);
    if (!await outputDirFile.exists()) {
      await outputDirFile.create(recursive: true);
    }

    _rustLib.resetProgress();
    final startTime = DateTime.now();

    yield CompressProgress(
      progress: 0.0,
      processedBytes: 0,
      totalBytes: totalBytes,
      speedBytesPerSecond: 0,
      estimatedRemaining: Duration.zero,
      currentFile: '准备解压专属格式...',
    );

    // 创建结果端口
    final resultPort = ReceivePort();
    Isolate? isolate;
    bool completed = false;
    String? error;
    int fileCount = 0;

    try {
      isolate = await Isolate.spawn(
        _decompressTarZstdInIsolate,
        <String, dynamic>{
          'archivePath': archivePath,
          'outputDir': outputDir,
          'password': password,
          'resultPort': resultPort.sendPort,
        },
      );
    } catch (e) {
      resultPort.close();
      throw Exception('启动解压任务失败: $e');
    }

    final subscription = resultPort.listen((message) {
      if (message is Map<String, dynamic>) {
        completed = true;
        fileCount = (message['fileCount'] as int?) ?? 0;
        final success = message['success'] as bool? ?? false;
        if (!success) {
          error = (message['errorMessage'] as String?) ?? '解压失败';
        }
      } else if (message is String && message.startsWith('ERROR:')) {
        error = message.substring(6);
        completed = true;
      }
    });

    // 轮询进度 - 使用平滑动画
    double displayProgress = 0.0;
    double targetProgress = 0.0;
    const smoothingFactor = 0.15;
    int stuckCount = 0;
    double lastTargetProgress = 0.0;

    while (!completed) {
      await Future.delayed(const Duration(milliseconds: 50));

      final progress = _rustLib.getProgress();
      final elapsed = DateTime.now().difference(startTime);
      final progressTotal = progress.total > 0 ? progress.total : totalBytes;

      if (progressTotal > 0) {
        targetProgress = progress.current / progressTotal;
      }

      // 检测进度是否卡住
      if ((targetProgress - lastTargetProgress).abs() < 0.001) {
        stuckCount++;
      } else {
        stuckCount = 0;
        lastTargetProgress = targetProgress;
      }

      // 平滑插值
      displayProgress += (targetProgress - displayProgress) * smoothingFactor;

      // 如果进度卡住，添加微小的模拟进度
      if (stuckCount > 10 && displayProgress < 0.95) {
        displayProgress += 0.001 * (1.0 - displayProgress);
      }

      // 确保进度不超过目标太多
      if (displayProgress > targetProgress + 0.05) {
        displayProgress = targetProgress + 0.05;
      }

      final speedBps = elapsed.inMilliseconds > 0
          ? progress.current / (elapsed.inMilliseconds / 1000)
          : 0.0;
      final remainingBytes = progressTotal - progress.current;
      final remainingSeconds =
          speedBps > 0 ? (remainingBytes / speedBps).round() : 0;

      yield CompressProgress(
        progress: displayProgress.clamp(0.0, 1.0),
        processedBytes: progress.current,
        totalBytes: progressTotal,
        speedBytesPerSecond: speedBps,
        estimatedRemaining: Duration(seconds: remainingSeconds),
        currentFile: '解压中...',
      );
    }

    await subscription.cancel();
    resultPort.close();
    isolate.kill(priority: Isolate.immediate);

    if (error != null) {
      throw Exception(error);
    }

    final duration = DateTime.now().difference(startTime);

    yield CompressProgress(
      progress: 1.0,
      processedBytes: totalBytes,
      totalBytes: totalBytes,
      speedBytesPerSecond: duration.inMilliseconds > 0
          ? totalBytes / (duration.inMilliseconds / 1000)
          : totalBytes.toDouble(),
      estimatedRemaining: Duration.zero,
      currentFile: '完成！已解压 $fileCount 个文件',
    );
  }

  /// 在 Isolate 中执行 Tar+Zstd 解压
  static void _decompressTarZstdInIsolate(Map<String, dynamic> params) {
    final resultPort = params['resultPort'] as SendPort;
    try {
      final rustLib = RustCompressLib.instance;

      if (!rustLib.isAvailable || !rustLib.hasTarZstdSupport) {
        resultPort.send('ERROR:Tar+Zstd 解压不可用');
        return;
      }

      final archivePath = params['archivePath'] as String;
      final outputDir = params['outputDir'] as String;
      final password = params['password'] as String?;

      final result = rustLib.decompressTarZstd(
        archivePath: archivePath,
        outputDir: outputDir,
        password: password,
      );

      resultPort.send(_makeDecompressResult(
        success: result.success,
        fileCount: result.fileCount,
        errorMessage: result.errorMessage,
      ));
    } catch (e) {
      resultPort.send('ERROR:$e');
    }
  }

  /// �?Isolate 中执行解�?
  static void _decompressInIsolate(Map<String, dynamic> params) {
    final resultPort = params['resultPort'] as SendPort;
    try {
      final rustLib = RustCompressLib.instance;

      if (!rustLib.isAvailable) {
        resultPort.send('ERROR:Rust 库不可用');
        return;
      }

      final archivePath = params['archivePath'] as String;
      final outputDir = params['outputDir'] as String;
      final password = params['password'] as String?;

      final lowerPath = archivePath.toLowerCase();
      RustDecompressResult result;

      // 优先使用智能解压（支持自动格式检测）
      if (rustLib.hasSmartDecompressSupport) {
        result = rustLib.smartDecompress(
          archivePath: archivePath,
          outputDir: outputDir,
          password: password,
        );
      } else if (lowerPath.endsWith('.7z')) {
        // 回退：使�?7z 解压
        if (rustLib.has7zSupport) {
          result = rustLib.decompress7z(
            archivePath: archivePath,
            outputDir: outputDir,
            password: password,
          );
        } else {
          resultPort.send('ERROR:7z 解压不可用');
          return;
        }
      } else if (password != null && password.isNotEmpty) {
        // 回退：加密的 sz7z 格式
        result = rustLib.decompressEncrypted(
          archivePath: archivePath,
          outputDir: outputDir,
          password: password,
        );
      } else {
        // 回退：普�?7z 格式
        result = rustLib.decompress(
          archivePath: archivePath,
          outputDir: outputDir,
        );
      }

      resultPort.send(_makeDecompressResult(
        success: result.success,
        fileCount: result.fileCount,
        errorMessage: result.errorMessage,
      ));
    } catch (e) {
      resultPort.send('ERROR:$e');
    }
  }

  /// 验证密码
  Future<bool> verifyPassword({
    required String archivePath,
    required String password,
  }) async {
    if (!isRustAvailable) {
      return false;
    }

    // .szp 文件使用专用验证方法
    final lowerPath = archivePath.toLowerCase();
    if (lowerPath.endsWith('.szp')) {
      if (!_rustLib.hasTarZstdSupport) {
        return false;
      }
      return _rustLib.verifyTarZstdPassword(
        archivePath: archivePath,
        password: password,
      );
    }

    return _rustLib.verifyPassword(
      archivePath: archivePath,
      password: password,
    );
  }

  /// 检查压缩包是否需要密�?
  Future<bool> requiresPassword(String archivePath) async {
    // 优先使用智能检�?
    if (isRustAvailable && _rustLib.hasSmartDecompressSupport) {
      return _rustLib.smartRequiresPassword(archivePath: archivePath);
    }

    // 根据文件扩展名判断是否需要密�?
    final lowerPath = archivePath.toLowerCase();

    // .szp 文件使用专用检查方�?
    if (lowerPath.endsWith('.szp')) {
      if (!isRustAvailable || !_rustLib.hasTarZstdSupport) {
        return true; // 保守起见，假设需要密�?
      }
      return _rustLib.requiresTarZstdPassword(archivePath: archivePath);
    }

    // .tar.zst �?.zst 文件不需要密码（�?Zstd 压缩�?
    if (lowerPath.endsWith('.tar.zst') || lowerPath.endsWith('.zst')) {
      return false;
    }

    // .enc �?.sz7z 文件需要密�?
    if (lowerPath.endsWith('.enc') || lowerPath.endsWith('.sz7z')) {
      return true;
    }

    // 标准�?.7z 文件不需要密码（除非加密�?
    if (lowerPath.endsWith('.7z') || lowerPath.endsWith('.zip')) {
      return false;
    }

    // 其他情况尝试读取文件头判�?
    try {
      final file = File(archivePath);
      if (await file.exists()) {
        final bytes = await file.openRead(0, 4).first;
        // 检查加密文件的魔数 "SZEN"
        if (bytes.length >= 4 &&
            bytes[0] == 0x53 && // S
            bytes[1] == 0x5A && // Z
            bytes[2] == 0x45 && // E
            bytes[3] == 0x4E) {
          // N
          return true;
        }
        // 检�?SZPK 魔数
        if (bytes.length >= 4 &&
            bytes[0] == 0x53 && // S
            bytes[1] == 0x5A && // Z
            bytes[2] == 0x50 && // P
            bytes[3] == 0x4B) {
          // K
          // 需要读取更多字节检查是否加�?
          return true; // 保守起见，假设需要密�?
        }
      }
    } catch (e) {
      // 忽略读取错误
    }

    return false;
  }

  /// 列出压缩包内�?
  Future<List<String>> listArchiveContents(String archivePath) async {
    final lowerPath = archivePath.toLowerCase();

    // 7z 格式 - 使用 Rust FFI
    if (lowerPath.endsWith('.7z')) {
      if (_rustLib.isAvailable && _rustLib.has7zSupport) {
        return _rustLib.list7zContents(archivePath: archivePath);
      }
    }

    // 其他格式暂时返回空列�?
    return [];
  }

  /// 获取压缩结果
  Future<CompressResult> getResult({
    required List<String> inputPaths,
    required String outputPath,
  }) async {
    int originalSize = 0;
    for (final path in inputPaths) {
      final type = FileSystemEntity.typeSync(path);
      if (type == FileSystemEntityType.file) {
        originalSize += await File(path).length();
      } else if (type == FileSystemEntityType.directory) {
        await for (final entity in Directory(path).list(recursive: true)) {
          if (entity is File) {
            originalSize += await entity.length();
          }
        }
      }
    }

    // 尝试查找实际输出文件
    // 获取不含扩展名的基础路径
    String basePath = outputPath;
    for (final ext in ['.sz7z', '.7z', '.zip']) {
      if (outputPath.toLowerCase().endsWith(ext)) {
        basePath = outputPath.substring(0, outputPath.length - ext.length);
        break;
      }
    }

    // 可能的输出路径（按优先级�?
    final possiblePaths = [
      outputPath, // 用户指定的路�?
      '$basePath.7z', // 7z压缩
      '$basePath.sz7z', // 专属模式
    ];

    String actualOutputPath = outputPath;
    for (final path in possiblePaths) {
      if (await File(path).exists()) {
        actualOutputPath = path;
        break;
      }
    }

    final outputFile = File(actualOutputPath);
    final compressedSize =
        await outputFile.exists() ? await outputFile.length() : 0;

    return CompressResult(
      success: await outputFile.exists() && compressedSize > 0,
      outputPath: actualOutputPath,
      originalSize: originalSize,
      compressedSize: compressedSize,
      duration: Duration.zero,
    );
  }

  // ========== 文件名混淆辅助方�?==========

  /// 生成混淆文件名（公开方法，供外部调用�?
  /// usedNames: 已使用的名称集合，用于避免重�?
  String generateObfuscatedName(
      String originalName, ObfuscationType type, int counter,
      {Set<String>? usedNames}) {
    usedNames ??= <String>{};

    String generateName(int cnt) {
      switch (type) {
        case ObfuscationType.sequential:
          return '${cnt.toString().padLeft(3, '0')}.dat';
        case ObfuscationType.dateSequential:
          final date = DateTime.now()
              .toIso8601String()
              .split('T')[0]
              .replaceAll('-', '');
          return '${date}_${cnt.toString().padLeft(3, '0')}.dat';
        case ObfuscationType.random:
          final random = Random();
          const chars = 'abcdefghijklmnopqrstuvwxyz0123456789';
          final randomStr =
              List.generate(8, (_) => chars[random.nextInt(chars.length)])
                  .join();
          return '$randomStr.dat';
        case ObfuscationType.hash:
          final bytes =
              utf8.encode(originalName + DateTime.now().toString() + '$cnt');
          final hash = sha256.convert(bytes);
          return '${hash.toString().substring(0, 12)}.dat';
        case ObfuscationType.encrypted:
          // 加密模式使用 base64 编码
          final encoded = base64Url.encode(utf8.encode(originalName));
          return '${encoded.substring(0, encoded.length.clamp(0, 20))}.enc';
      }
    }

    // 生成名称并检查是否重�?
    String name = generateName(counter);
    int attempts = 0;
    while (usedNames.contains(name) && attempts < 10000) {
      counter++;
      attempts++;
      name = generateName(counter);
    }

    return name;
  }

  /// 准备混淆文件（创建临时目录并复制文件�?
  Future<_ObfuscationPrepResult> _prepareObfuscatedFiles(
    List<String> inputPaths,
    String archivePath,
    ObfuscationType type,
  ) async {
    final tempDir = await Directory.systemTemp.createTemp('sezip_obfuscate_');
    final mappings = <MappingEntry>[];
    final obfuscatedPaths = <String>[];
    final usedNames = <String>{}; // 跟踪已使用的名称，避免重�?
    int counter = 1;

    for (final inputPath in inputPaths) {
      final entity = FileSystemEntity.typeSync(inputPath);
      if (entity == FileSystemEntityType.file) {
        final originalName = p.basename(inputPath);
        final obfuscatedName = generateObfuscatedName(
            originalName, type, counter++,
            usedNames: usedNames);
        usedNames.add(obfuscatedName); // 记录已使�?
        final destPath = p.join(tempDir.path, obfuscatedName);

        await File(inputPath).copy(destPath);
        obfuscatedPaths.add(destPath);

        mappings.add(MappingEntry(
          originalName: originalName,
          obfuscatedName: obfuscatedName,
          archivePath: archivePath,
        ));
      } else if (entity == FileSystemEntityType.directory) {
        // 对于目录，递归处理其中的文�?
        counter = await _processDirectoryForObfuscation(
          Directory(inputPath),
          tempDir,
          archivePath,
          type,
          mappings,
          obfuscatedPaths,
          counter,
          usedNames,
        );
      }
    }

    return _ObfuscationPrepResult(
      tempDir: tempDir,
      obfuscatedPaths: obfuscatedPaths,
      mappings: mappings,
    );
  }

  /// 递归处理目录中的文件进行混淆
  Future<int> _processDirectoryForObfuscation(
    Directory dir,
    Directory tempDir,
    String archivePath,
    ObfuscationType type,
    List<MappingEntry> mappings,
    List<String> obfuscatedPaths,
    int counter,
    Set<String> usedNames,
  ) async {
    final dirName = p.basename(dir.path);
    final obfuscatedDirName =
        generateObfuscatedName(dirName, type, counter++, usedNames: usedNames);
    usedNames.add(obfuscatedDirName);
    final destDir = Directory(p.join(tempDir.path, obfuscatedDirName));
    await destDir.create(recursive: true);

    mappings.add(MappingEntry(
      originalName: '$dirName/',
      obfuscatedName: '$obfuscatedDirName/',
      archivePath: archivePath,
    ));

    await for (final entity in dir.list(recursive: false)) {
      if (entity is File) {
        final originalName = p.basename(entity.path);
        final obfuscatedName = generateObfuscatedName(
            originalName, type, counter++,
            usedNames: usedNames);
        usedNames.add(obfuscatedName);
        final destPath = p.join(destDir.path, obfuscatedName);

        await entity.copy(destPath);

        mappings.add(MappingEntry(
          originalName: '$dirName/$originalName',
          obfuscatedName: '$obfuscatedDirName/$obfuscatedName',
          archivePath: archivePath,
        ));
      } else if (entity is Directory) {
        counter = await _processDirectoryForObfuscation(
          entity,
          destDir,
          archivePath,
          type,
          mappings,
          obfuscatedPaths,
          counter,
          usedNames,
        );
      }
    }

    obfuscatedPaths.add(destDir.path);
    return counter;
  }

  /// 清理临时目录
  Future<void> _cleanupTempDir(Directory tempDir) async {
    try {
      if (await tempDir.exists()) {
        await tempDir.delete(recursive: true);
      }
    } catch (e) {
      print('清理临时目录失败: $e');
    }
  }
}

/// 混淆准备结果
class _ObfuscationPrepResult {
  final Directory tempDir;
  final List<String> obfuscatedPaths;
  final List<MappingEntry> mappings;

  _ObfuscationPrepResult({
    required this.tempDir,
    required this.obfuscatedPaths,
    required this.mappings,
  });
}
