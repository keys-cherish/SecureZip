/// 压缩模式枚举
enum CompressMode {
  /// 标准模式：生成 7z 标准格式，可被 7-Zip-zstd 等软件打开
  standard,

  /// 专属模式：SecureZip专用格式（.szp），tar+zstd+AES256，仅能被本软件打开
  exclusive,
}

/// 压缩算法枚举
/// 使用 7z + ZSTD 或 tar + ZSTD 高效压缩
enum CompressionAlgorithm {
  /// Zstd（速度快压缩率高）
  zstd,
}

extension CompressionAlgorithmExtension on CompressionAlgorithm {
  String get displayName {
    switch (this) {
      case CompressionAlgorithm.zstd:
        return 'ZSTD（速度快压缩率高）';
    }
  }

  String get shortName {
    switch (this) {
      case CompressionAlgorithm.zstd:
        return 'ZSTD';
    }
  }
}

/// 压缩选项模型
class CompressOptions {
  final String? password;
  final bool enableObfuscation;
  final ObfuscationType obfuscationType;
  final int compressionLevel;
  final CompressMode compressMode;
  final String fileExtension;
  final bool solidCompression;
  final bool encryptFilenames;
  final CompressionAlgorithm algorithm;

  /// 分包大小（字节），0 表示不分包
  final int splitSize;

  const CompressOptions({
    this.password,
    this.enableObfuscation = false,
    this.obfuscationType = ObfuscationType.sequential,
    this.compressionLevel = 6,
    this.compressMode = CompressMode.standard,
    this.fileExtension = '.7z',
    this.solidCompression = false,
    this.encryptFilenames = false,
    this.algorithm = CompressionAlgorithm.zstd, // 默认使用 Zstd
    this.splitSize = 0, // 默认不分包
  });

  /// 是否启用分包
  bool get enableSplit => splitSize > 0;
}

/// 分包大小预设
enum SplitSizePreset {
  none(0, '不分包'),
  mb100(100 * 1024 * 1024, '100 MB'),
  mb200(200 * 1024 * 1024, '200 MB'),
  mb500(500 * 1024 * 1024, '500 MB'),
  gb1(1024 * 1024 * 1024, '1 GB'),
  gb2(2 * 1024 * 1024 * 1024, '2 GB'),
  gb4(4 * 1024 * 1024 * 1024, '4 GB');

  final int bytes;
  final String displayName;

  const SplitSizePreset(this.bytes, this.displayName);
}

/// 混淆类型枚举
enum ObfuscationType {
  sequential,
  dateSequential,
  random,
  hash,
  encrypted,
}

/// 压缩进度信息
class CompressProgress {
  final double progress;
  final int processedBytes;
  final int totalBytes;
  final double speedBytesPerSecond;
  final Duration estimatedRemaining;
  final String currentFile;

  const CompressProgress({
    required this.progress,
    required this.processedBytes,
    required this.totalBytes,
    required this.speedBytesPerSecond,
    required this.estimatedRemaining,
    required this.currentFile,
  });

  String get displayProcessed {
    return _formatBytes(processedBytes);
  }

  String get displayTotal {
    return _formatBytes(totalBytes);
  }

  String get displaySpeed {
    return '${_formatBytes(speedBytesPerSecond.toInt())}/s';
  }

  String get displayRemaining {
    if (estimatedRemaining.inSeconds < 60) {
      return '约 ${estimatedRemaining.inSeconds} 秒';
    } else if (estimatedRemaining.inMinutes < 60) {
      return '约 ${estimatedRemaining.inMinutes} 分钟';
    } else {
      return '约 ${estimatedRemaining.inHours} 小时';
    }
  }

  static String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }
}

/// 压缩结果信息
class CompressResult {
  final bool success;
  final String outputPath;
  final int originalSize;
  final int compressedSize;
  final Duration duration;
  final String? errorMessage;

  const CompressResult({
    required this.success,
    required this.outputPath,
    required this.originalSize,
    required this.compressedSize,
    required this.duration,
    this.errorMessage,
  });

  double get compressionRatio {
    if (originalSize == 0) return 0;
    return (1 - compressedSize / originalSize) * 100;
  }

  String get displayOriginalSize => CompressProgress._formatBytes(originalSize);
  String get displayCompressedSize =>
      CompressProgress._formatBytes(compressedSize);
}
