import 'dart:async';
import 'dart:io';
import 'dart:convert';
import 'dart:isolate';
import 'dart:math';
import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:archive/archive.dart';
import 'package:pointycastle/export.dart';
import '../models/compress_options.dart';

/// 默认输出目录
const String _defaultAndroidOutputDir =
    '/storage/emulated/0/SecureZip/compressed';

/// 压缩任务参数
class _CompressTaskParams {
  final List<String> inputPaths;
  final String outputPath;
  final CompressOptions options;
  final SendPort sendPort;

  _CompressTaskParams({
    required this.inputPaths,
    required this.outputPath,
    required this.options,
    required this.sendPort,
  });
}

/// 压缩服务（Dart 回退实现）
/// 封装压缩/解压功能，使用 Isolate 在后台线程执行
/// 注意：此服务仅在 Rust FFI 不可用时使用，使用 ZLib 作为压缩算法
class CompressService {
  /// 获取算法显示名称
  static String getAlgorithmName(CompressionAlgorithm algorithm) {
    switch (algorithm) {
      case CompressionAlgorithm.zstd:
        return 'ZSTD (7z格式)';
    }
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

  /// 压缩文件或文件夹
  /// 返回进度流，在后台 Isolate 中执行
  Stream<CompressProgress> compress({
    required List<String> inputPaths,
    required String outputPath,
    CompressOptions options = const CompressOptions(),
  }) async* {
    // 验证输入
    if (inputPaths.isEmpty) {
      throw Exception('输入路径不能为空');
    }

    // 使用 Isolate 在后台执行压缩
    final receivePort = ReceivePort();

    final params = _CompressTaskParams(
      inputPaths: inputPaths,
      outputPath: outputPath,
      options: options,
      sendPort: receivePort.sendPort,
    );

    // 启动后台 Isolate
    await Isolate.spawn(_compressInIsolate, params);

    // 监听进度
    await for (final message in receivePort) {
      if (message is CompressProgress) {
        yield message;
        if (message.progress >= 1.0) {
          receivePort.close();
          break;
        }
      } else if (message is String && message.startsWith('ERROR:')) {
        receivePort.close();
        throw Exception(message.substring(6));
      }
    }
  }

  /// 在 Isolate 中执行压缩
  static Future<void> _compressInIsolate(_CompressTaskParams params) async {
    try {
      final inputPaths = params.inputPaths;
      final outputPath = params.outputPath;
      final options = params.options;
      final sendPort = params.sendPort;

      // 确保输出目录存在
      final outputFile = File(outputPath);
      final outputDir = outputFile.parent;
      if (!await outputDir.exists()) {
        await outputDir.create(recursive: true);
      }

      // 收集所有文件
      int totalBytes = 0;
      final files = <MapEntry<File, String>>[]; // File -> relative path

      for (final path in inputPaths) {
        final type = FileSystemEntity.typeSync(path);
        if (type == FileSystemEntityType.file) {
          final file = File(path);
          totalBytes += await file.length();
          final name = file.path.split('/').last.split('\\').last;
          files.add(MapEntry(file, name));
        } else if (type == FileSystemEntityType.directory) {
          final dir = Directory(path);
          final baseName = dir.path.split('/').last.split('\\').last;
          await for (final entity in dir.list(recursive: true)) {
            if (entity is File) {
              totalBytes += await entity.length();
              // 计算相对路径
              final relativePath = entity.path
                  .replaceFirst(dir.path, baseName)
                  .replaceAll('\\', '/');
              files.add(MapEntry(entity, relativePath));
            }
          }
        }
      }

      if (files.isEmpty) {
        sendPort.send('ERROR:没有找到要压缩的文件');
        return;
      }

      final startTime = DateTime.now();

      // 根据模式选择压缩方式
      final isStandardMode = options.compressMode == CompressMode.standard;

      if (isStandardMode) {
        // 标准模式：使用 Rust FFI 的 7z + ZSTD 格式
        // Dart 回退实现使用 ZIP 格式
        await _compressStandardMode(
          files: files,
          outputFile: outputFile,
          options: options,
          totalBytes: totalBytes,
          startTime: startTime,
          sendPort: sendPort,
        );
      } else {
        // 专属模式：SecureZip 自定义格式（.sz7z）
        await _compressExclusiveMode(
          files: files,
          outputFile: outputFile,
          options: options,
          totalBytes: totalBytes,
          startTime: startTime,
          sendPort: sendPort,
        );
      }

      // 最终进度
      final duration = DateTime.now().difference(startTime);
      sendPort.send(CompressProgress(
        progress: 1.0,
        processedBytes: totalBytes,
        totalBytes: totalBytes,
        speedBytesPerSecond: duration.inMilliseconds > 0
            ? totalBytes / (duration.inMilliseconds / 1000)
            : totalBytes.toDouble(),
        estimatedRemaining: Duration.zero,
        currentFile: '完成',
      ));
    } catch (e) {
      params.sendPort.send('ERROR:$e');
    }
  }

  /// 标准模式压缩（Dart 回退实现）
  /// 正式版本使用 Rust FFI 的 7z + ZSTD
  static Future<void> _compressStandardMode({
    required List<MapEntry<File, String>> files,
    required File outputFile,
    required CompressOptions options,
    required int totalBytes,
    required DateTime startTime,
    required SendPort sendPort,
  }) async {
    final archive = Archive();
    int processedBytes = 0;

    for (int i = 0; i < files.length; i++) {
      final entry = files[i];
      final file = entry.key;
      final relativePath = entry.value;
      final fileBytes = await file.readAsBytes();

      // 添加文件到归档
      archive.addFile(ArchiveFile(relativePath, fileBytes.length, fileBytes));

      processedBytes += fileBytes.length;
      _sendProgress(
          sendPort, processedBytes, totalBytes, startTime, relativePath, 0.9);
    }

    // 编码为 ZIP 格式（Dart 回退实现）
    // 正式版本使用 Rust FFI 的 7z + ZSTD 格式
    final zipEncoder = ZipEncoder();
    final compressedData = zipEncoder.encode(archive);

    if (compressedData != null) {
      // 如果有密码，进行 AES 加密
      if (options.password != null && options.password!.isNotEmpty) {
        final encryptedData = _encryptData(compressedData, options.password!);
        // 添加加密标识头
        final finalBuffer = BytesBuilder();
        finalBuffer.add(utf8.encode('SZPW')); // 加密标识
        finalBuffer.add(_intToBytes(compressedData.length, 8)); // 原始大小
        finalBuffer.add(encryptedData);
        await outputFile.writeAsBytes(finalBuffer.toBytes());
      } else {
        await outputFile.writeAsBytes(compressedData);
      }
    }
  }

  /// 专属模式压缩
  static Future<void> _compressExclusiveMode({
    required List<MapEntry<File, String>> files,
    required File outputFile,
    required CompressOptions options,
    required int totalBytes,
    required DateTime startTime,
    required SendPort sendPort,
  }) async {
    final archiveBuffer = BytesBuilder();
    int processedBytes = 0;

    // 写入文件数量
    archiveBuffer.add(_intToBytes(files.length, 4));

    for (int i = 0; i < files.length; i++) {
      final entry = files[i];
      final file = entry.key;
      final relativePath = entry.value;
      final fileBytes = await file.readAsBytes();

      // 写入文件名
      final nameBytes = utf8.encode(relativePath);
      archiveBuffer.add(_intToBytes(nameBytes.length, 4));
      archiveBuffer.add(nameBytes);

      // 写入文件大小
      archiveBuffer.add(_intToBytes(fileBytes.length, 8));

      // 写入文件内容
      archiveBuffer.add(fileBytes);

      processedBytes += fileBytes.length;
      _sendProgress(
          sendPort, processedBytes, totalBytes, startTime, relativePath, 1.0);
    }

    // 压缩数据
    final rawData = archiveBuffer.toBytes();
    List<int> compressedData;

    // Dart 回退实现使用 ZLib 压缩
    // 正式功能使用 Rust FFI 的 7z + ZSTD
    compressedData = ZLibCodec(level: options.compressionLevel).encode(rawData);

    // 构建最终文件
    final finalBuffer = BytesBuilder();

    // 写入魔数和算法标识
    if (options.password != null && options.password!.isNotEmpty) {
      // 加密模式
      final encryptedData = _encryptData(compressedData, options.password!);
      finalBuffer.add(utf8.encode('SZ7E')); // 加密专属格式
      finalBuffer.add([options.algorithm.index]); // 算法标识
      finalBuffer.add(_intToBytes(rawData.length, 8)); // 原始大小
      finalBuffer.add(encryptedData);
    } else {
      // 非加密模式
      finalBuffer.add(utf8.encode('SZ7Z')); // 专属格式魔数
      finalBuffer.add([options.algorithm.index]); // 算法标识
      finalBuffer.add(_intToBytes(rawData.length, 8)); // 原始大小
      finalBuffer.add(compressedData);
    }

    await outputFile.writeAsBytes(finalBuffer.toBytes());
  }

  /// 发送进度
  static void _sendProgress(
    SendPort sendPort,
    int processedBytes,
    int totalBytes,
    DateTime startTime,
    String currentFile,
    double maxProgress,
  ) {
    final elapsed = DateTime.now().difference(startTime);
    final speedBps = elapsed.inMilliseconds > 0
        ? processedBytes / (elapsed.inMilliseconds / 1000)
        : 0.0;
    final remainingBytes = totalBytes - processedBytes;
    final remainingSeconds =
        speedBps > 0 ? (remainingBytes / speedBps).round() : 0;

    sendPort.send(CompressProgress(
      progress: (processedBytes / totalBytes) * maxProgress,
      processedBytes: processedBytes,
      totalBytes: totalBytes,
      speedBytesPerSecond: speedBps,
      estimatedRemaining: Duration(seconds: remainingSeconds),
      currentFile: currentFile,
    ));
  }

  /// AES-256-GCM 加密
  /// 使用 PBKDF2 派生密钥，GCM 模式提供认证加密
  static List<int> _encryptData(List<int> data, String password) {
    const int nonceLength = 12;
    const int saltLength = 16;
    const int tagLength = 16;
    const int keyLength = 32;
    const int pbkdf2Iterations = 100000;

    // 生成随机 salt 和 nonce
    final random = Random.secure();
    final salt = Uint8List.fromList(
      List.generate(saltLength, (_) => random.nextInt(256)),
    );
    final nonce = Uint8List.fromList(
      List.generate(nonceLength, (_) => random.nextInt(256)),
    );

    // 从密码派生密钥（PBKDF2-HMAC-SHA256）
    final pbkdf2 = PBKDF2KeyDerivator(HMac(SHA256Digest(), 64))
      ..init(Pbkdf2Parameters(salt, pbkdf2Iterations, keyLength));
    final key = pbkdf2.process(Uint8List.fromList(utf8.encode(password)));

    // 初始化 AES-256-GCM
    final cipher = GCMBlockCipher(AESEngine())
      ..init(
        true, // 加密模式
        AEADParameters(
          KeyParameter(key),
          tagLength * 8,
          nonce,
          Uint8List(0),
        ),
      );

    // 加密
    final plaintext = Uint8List.fromList(data);
    final ciphertext = Uint8List(cipher.getOutputSize(plaintext.length));
    final len =
        cipher.processBytes(plaintext, 0, plaintext.length, ciphertext, 0);
    cipher.doFinal(ciphertext, len);

    // 组合结果：salt + nonce + ciphertext
    final result = BytesBuilder();
    result.add(salt);
    result.add(nonce);
    result.add(ciphertext);

    return result.toBytes();
  }

  /// AES-256-GCM 解密
  static List<int> _decryptData(List<int> data, String password) {
    const int nonceLength = 12;
    const int saltLength = 16;
    const int tagLength = 16;
    const int keyLength = 32;
    const int pbkdf2Iterations = 100000;

    if (data.length < saltLength + nonceLength + tagLength) {
      throw Exception('密文数据太短');
    }

    // 提取 salt 和 nonce
    final salt = Uint8List.fromList(data.sublist(0, saltLength));
    final nonce =
        Uint8List.fromList(data.sublist(saltLength, saltLength + nonceLength));
    final encryptedData =
        Uint8List.fromList(data.sublist(saltLength + nonceLength));

    // 从密码派生密钥
    final pbkdf2 = PBKDF2KeyDerivator(HMac(SHA256Digest(), 64))
      ..init(Pbkdf2Parameters(salt, pbkdf2Iterations, keyLength));
    final key = pbkdf2.process(Uint8List.fromList(utf8.encode(password)));

    // 初始化 AES-256-GCM
    final cipher = GCMBlockCipher(AESEngine())
      ..init(
        false, // 解密模式
        AEADParameters(
          KeyParameter(key),
          tagLength * 8,
          nonce,
          Uint8List(0),
        ),
      );

    // 解密
    final plaintext = Uint8List(cipher.getOutputSize(encryptedData.length));
    final len = cipher.processBytes(
        encryptedData, 0, encryptedData.length, plaintext, 0);
    cipher.doFinal(plaintext, len);

    return plaintext.toList();
  }

  /// 获取压缩结果
  Future<CompressResult> getResult({
    required List<String> inputPaths,
    required String outputPath,
  }) async {
    // 计算原始大小
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

    // 获取压缩后大小
    final outputFile = File(outputPath);
    final compressedSize =
        await outputFile.exists() ? await outputFile.length() : 0;

    return CompressResult(
      success: await outputFile.exists() && compressedSize > 0,
      outputPath: outputPath,
      originalSize: originalSize,
      compressedSize: compressedSize,
      duration: const Duration(seconds: 0),
    );
  }

  /// 解压文件
  /// 返回进度流
  Stream<CompressProgress> decompress({
    required String archivePath,
    required String outputDir,
    String? password,
  }) async* {
    // 验证压缩包存在
    final archiveFile = File(archivePath);
    if (!await archiveFile.exists()) {
      throw Exception('压缩包不存在: $archivePath');
    }

    // 确保输出目录存在
    final outDir = Directory(outputDir);
    if (!await outDir.exists()) {
      await outDir.create(recursive: true);
    }

    final startTime = DateTime.now();
    final archiveBytes = await archiveFile.readAsBytes();
    final totalBytes = archiveBytes.length;

    yield CompressProgress(
      progress: 0.1,
      processedBytes: 0,
      totalBytes: totalBytes,
      speedBytesPerSecond: 0,
      estimatedRemaining: const Duration(seconds: 5),
      currentFile: '正在读取压缩包...',
    );

    try {
      // 检查文件头
      if (archiveBytes.length < 4) {
        throw Exception('无效的压缩包格式');
      }

      final magic = utf8.decode(archiveBytes.sublist(0, 4));

      if (magic == 'SZ7Z' || magic == 'SZ7E') {
        // SecureZip 专属格式
        yield* _decompressExclusiveFormat(
          archiveBytes: archiveBytes,
          magic: magic,
          outputDir: outputDir,
          password: password,
          startTime: startTime,
        );
      } else if (magic == 'SZPW') {
        // 加密的标准格式
        yield* _decompressEncryptedStandard(
          archiveBytes: archiveBytes,
          outputDir: outputDir,
          password: password,
          startTime: startTime,
        );
      } else {
        // 尝试作为标准 ZIP 格式解压
        yield* _decompressStandardFormat(
          archiveBytes: archiveBytes,
          outputDir: outputDir,
          startTime: startTime,
        );
      }
    } catch (e) {
      throw Exception('解压失败: $e');
    }
  }

  /// 解压专属格式
  Stream<CompressProgress> _decompressExclusiveFormat({
    required List<int> archiveBytes,
    required String magic,
    required String outputDir,
    String? password,
    required DateTime startTime,
  }) async* {
    final isEncrypted = magic == 'SZ7E';
    int offset = 4;

    // 读取算法标识
    final algorithmIndex = archiveBytes[offset];
    offset += 1;

    // 读取原始大小
    final originalSize = _bytesToInt(archiveBytes.sublist(offset, offset + 8));
    offset += 8;

    // 获取压缩数据
    var compressedData = archiveBytes.sublist(offset);

    // 如果是加密的，先解密
    if (isEncrypted) {
      if (password == null || password.isEmpty) {
        throw Exception('此压缩包需要密码');
      }
      compressedData = _decryptData(compressedData, password);
    }

    yield CompressProgress(
      progress: 0.3,
      processedBytes: archiveBytes.length ~/ 3,
      totalBytes: archiveBytes.length,
      speedBytesPerSecond: archiveBytes.length / 3,
      estimatedRemaining: const Duration(seconds: 3),
      currentFile: '正在解压...',
    );

    // 解压数据
    final decompressedData = ZLibCodec().decode(compressedData);

    yield CompressProgress(
      progress: 0.5,
      processedBytes: archiveBytes.length ~/ 2,
      totalBytes: archiveBytes.length,
      speedBytesPerSecond: archiveBytes.length / 2,
      estimatedRemaining: const Duration(seconds: 2),
      currentFile: '正在写入文件...',
    );

    // 解析归档结构
    int dataOffset = 0;
    final fileCount =
        _bytesToInt(decompressedData.sublist(dataOffset, dataOffset + 4));
    dataOffset += 4;

    for (int i = 0; i < fileCount; i++) {
      // 读取文件名
      final nameLen =
          _bytesToInt(decompressedData.sublist(dataOffset, dataOffset + 4));
      dataOffset += 4;
      final fileName = utf8
          .decode(decompressedData.sublist(dataOffset, dataOffset + nameLen));
      dataOffset += nameLen;

      // 读取文件大小
      final fileSize =
          _bytesToInt(decompressedData.sublist(dataOffset, dataOffset + 8));
      dataOffset += 8;

      // 读取文件内容
      final fileContent =
          decompressedData.sublist(dataOffset, dataOffset + fileSize);
      dataOffset += fileSize;

      // 写入文件
      final outputFile = File('$outputDir/$fileName');
      await outputFile.parent.create(recursive: true);
      await outputFile.writeAsBytes(fileContent);

      yield CompressProgress(
        progress: 0.5 + (0.5 * (i + 1) / fileCount),
        processedBytes: dataOffset,
        totalBytes: decompressedData.length,
        speedBytesPerSecond: dataOffset /
            DateTime.now().difference(startTime).inMilliseconds *
            1000,
        estimatedRemaining: Duration(
            seconds: ((decompressedData.length - dataOffset) /
                    (dataOffset /
                        DateTime.now().difference(startTime).inMilliseconds *
                        1000))
                .round()
                .abs()),
        currentFile: fileName,
      );
    }

    yield CompressProgress(
      progress: 1.0,
      processedBytes: decompressedData.length,
      totalBytes: decompressedData.length,
      speedBytesPerSecond: decompressedData.length /
          DateTime.now().difference(startTime).inMilliseconds *
          1000,
      estimatedRemaining: Duration.zero,
      currentFile: '完成',
    );
  }

  /// 解压加密的标准格式
  Stream<CompressProgress> _decompressEncryptedStandard({
    required List<int> archiveBytes,
    required String outputDir,
    String? password,
    required DateTime startTime,
  }) async* {
    if (password == null || password.isEmpty) {
      throw Exception('此压缩包需要密码');
    }

    // 读取原始大小
    final originalSize = _bytesToInt(archiveBytes.sublist(4, 12));

    // 获取加密数据并解密
    final encryptedData = archiveBytes.sublist(12);
    final decryptedData = _decryptData(encryptedData, password);

    yield CompressProgress(
      progress: 0.3,
      processedBytes: archiveBytes.length ~/ 3,
      totalBytes: archiveBytes.length,
      speedBytesPerSecond: archiveBytes.length / 3,
      estimatedRemaining: const Duration(seconds: 3),
      currentFile: '正在解密...',
    );

    // 解压 ZIP 数据
    yield* _decompressStandardFormat(
      archiveBytes: decryptedData,
      outputDir: outputDir,
      startTime: startTime,
      progressOffset: 0.3,
    );
  }

  /// 解压标准 ZIP 格式
  Stream<CompressProgress> _decompressStandardFormat({
    required List<int> archiveBytes,
    required String outputDir,
    required DateTime startTime,
    double progressOffset = 0.0,
  }) async* {
    try {
      final archive = ZipDecoder().decodeBytes(archiveBytes);
      final totalFiles = archive.files.length;
      int processedFiles = 0;

      for (final file in archive.files) {
        if (file.isFile) {
          final outputFile = File('$outputDir/${file.name}');
          await outputFile.parent.create(recursive: true);
          await outputFile.writeAsBytes(file.content as List<int>);
        }

        processedFiles++;
        yield CompressProgress(
          progress: progressOffset +
              (1.0 - progressOffset) * processedFiles / totalFiles,
          processedBytes: processedFiles,
          totalBytes: totalFiles,
          speedBytesPerSecond: processedFiles /
              DateTime.now().difference(startTime).inMilliseconds *
              1000,
          estimatedRemaining: Duration(
              seconds: ((totalFiles - processedFiles) /
                      (processedFiles /
                          DateTime.now().difference(startTime).inMilliseconds *
                          1000))
                  .round()
                  .abs()),
          currentFile: file.name,
        );
      }

      yield CompressProgress(
        progress: 1.0,
        processedBytes: totalFiles,
        totalBytes: totalFiles,
        speedBytesPerSecond: totalFiles /
            DateTime.now().difference(startTime).inMilliseconds *
            1000,
        estimatedRemaining: Duration.zero,
        currentFile: '完成',
      );
    } catch (e) {
      throw Exception('无法解压此文件格式: $e');
    }
  }

  /// 检查压缩包是否需要密码
  Future<bool> requiresPassword(String archivePath) async {
    final file = File(archivePath);
    if (!await file.exists()) {
      throw Exception('文件不存在: $archivePath');
    }

    final bytes = await file.readAsBytes();
    if (bytes.length < 4) return false;

    final magic = utf8.decode(bytes.sublist(0, 4));
    return magic == 'SZ7E' || magic == 'SZPW';
  }

  /// 列出压缩包内容
  Future<List<String>> listArchiveContents(String archivePath) async {
    final file = File(archivePath);
    if (!await file.exists()) {
      throw Exception('文件不存在: $archivePath');
    }

    try {
      final archiveBytes = await file.readAsBytes();

      if (archiveBytes.length < 4) {
        return [];
      }

      final magic = utf8.decode(archiveBytes.sublist(0, 4));

      if (magic == 'SZ7Z') {
        // SecureZip 专属格式（未加密）
        int offset = 5; // 跳过魔数和算法标识
        offset += 8; // 跳过原始大小

        final compressedData = archiveBytes.sublist(offset);
        final decompressedData = ZLibCodec().decode(compressedData);

        int dataOffset = 0;
        final fileCount =
            _bytesToInt(decompressedData.sublist(dataOffset, dataOffset + 4));
        dataOffset += 4;

        final fileNames = <String>[];

        for (int i = 0; i < fileCount; i++) {
          final nameLen =
              _bytesToInt(decompressedData.sublist(dataOffset, dataOffset + 4));
          dataOffset += 4;
          final fileName = utf8.decode(
              decompressedData.sublist(dataOffset, dataOffset + nameLen));
          dataOffset += nameLen;

          final fileSize =
              _bytesToInt(decompressedData.sublist(dataOffset, dataOffset + 8));
          dataOffset += 8;
          dataOffset += fileSize;

          fileNames.add(fileName);
        }

        return fileNames;
      } else if (magic == 'SZ7E' || magic == 'SZPW') {
        // 加密格式，需要密码才能列出内容
        return ['(需要密码才能查看内容)'];
      } else {
        // 尝试作为 ZIP 格式
        final archive = ZipDecoder().decodeBytes(archiveBytes);
        return archive.files.map((f) => f.name).toList();
      }
    } catch (e) {
      return [];
    }
  }

  /// 验证密码是否正确
  Future<bool> verifyPassword(String archivePath, String password) async {
    try {
      final file = File(archivePath);
      if (!await file.exists()) return false;

      final bytes = await file.readAsBytes();
      if (bytes.length < 4) return false;

      final magic = utf8.decode(bytes.sublist(0, 4));

      if (magic == 'SZ7E') {
        // 尝试解密并验证
        int offset = 5;
        offset += 8;
        final encryptedData = bytes.sublist(offset);
        final decryptedData = _decryptData(encryptedData, password);

        try {
          ZLibCodec().decode(decryptedData);
          return true;
        } catch (e) {
          return false;
        }
      } else if (magic == 'SZPW') {
        final encryptedData = bytes.sublist(12);
        final decryptedData = _decryptData(encryptedData, password);

        try {
          ZipDecoder().decodeBytes(decryptedData);
          return true;
        } catch (e) {
          return false;
        }
      }

      return true;
    } catch (e) {
      return false;
    }
  }

  /// 将整数转换为字节数组
  static List<int> _intToBytes(int value, int length) {
    final bytes = <int>[];
    for (int i = 0; i < length; i++) {
      bytes.add((value >> (i * 8)) & 0xFF);
    }
    return bytes;
  }

  /// 将字节数组转换为整数
  static int _bytesToInt(List<int> bytes) {
    int value = 0;
    for (int i = 0; i < bytes.length; i++) {
      value |= (bytes[i] & 0xFF) << (i * 8);
    }
    return value;
  }
}
