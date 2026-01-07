import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/compress_options.dart';

/// 压缩结果信息（简化版，用于页面间传递）
class CompressResultData {
  final bool success;
  final String outputPath;
  final int originalSize;
  final int compressedSize;
  final Duration duration;
  final String? errorMessage;

  const CompressResultData({
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

  String get displayOriginalSize => _formatBytes(originalSize);
  String get displayCompressedSize => _formatBytes(compressedSize);

  static String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }
}

/// 压缩结果页面
class CompressResultPage extends StatelessWidget {
  final dynamic result; // CompressResult from compress_options.dart
  final CompressionAlgorithm algorithm;

  const CompressResultPage({
    super.key,
    required this.result,
    required this.algorithm,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('压缩完成'),
        automaticallyImplyLeading: false,
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Column(
            children: [
              const SizedBox(height: 24),

              // 成功图标
              Container(
                width: 80,
                height: 80,
                decoration: BoxDecoration(
                  color: colorScheme.primaryContainer,
                  shape: BoxShape.circle,
                ),
                child: Icon(
                  Icons.check,
                  size: 48,
                  color: colorScheme.primary,
                ),
              ),

              const SizedBox(height: 24),

              Text(
                '压缩完成',
                style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
              ),

              const SizedBox(height: 32),

              // 结果详情卡片
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    children: [
                      _ResultRow(
                        label: '原始大小',
                        value: result.displayOriginalSize,
                      ),
                      const Divider(height: 24),
                      _ResultRow(
                        label: '压缩后大小',
                        value: result.displayCompressedSize,
                      ),
                      const Divider(height: 24),
                      _ResultRow(
                        label: '压缩率',
                        value: '${result.compressionRatio.toStringAsFixed(1)}%',
                        valueColor: colorScheme.primary,
                      ),
                      const Divider(height: 24),
                      _ResultRow(
                        label: '压缩算法',
                        value: algorithm.shortName,
                      ),
                    ],
                  ),
                ),
              ),

              const SizedBox(height: 16),

              // 保存位置卡片
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(
                            Icons.folder_outlined,
                            size: 20,
                            color: colorScheme.primary,
                          ),
                          const SizedBox(width: 8),
                          Text(
                            '保存位置',
                            style: Theme.of(context)
                                .textTheme
                                .titleSmall
                                ?.copyWith(
                                  color: colorScheme.primary,
                                ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      Container(
                        width: double.infinity,
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: colorScheme.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: SelectableText(
                          result.outputPath,
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Row(
                        children: [
                          Expanded(
                            child: OutlinedButton.icon(
                              onPressed: () => _copyPath(context),
                              icon: const Icon(Icons.copy, size: 18),
                              label: const Text('复制路径'),
                            ),
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: OutlinedButton.icon(
                              onPressed: () => _openDirectory(context),
                              icon: const Icon(Icons.folder_open, size: 18),
                              label: const Text('打开目录'),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),

              const SizedBox(height: 32),

              // 底部按钮
              Row(
                children: [
                  Expanded(
                    child: OutlinedButton(
                      onPressed: () {
                        // 返回到压缩页面
                        Navigator.of(context).pop();
                      },
                      child: const Text('继续压缩'),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: FilledButton(
                      onPressed: () {
                        // 返回首页
                        Navigator.of(context)
                            .popUntil((route) => route.isFirst);
                      },
                      child: const Text('返回首页'),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _copyPath(BuildContext context) {
    Clipboard.setData(ClipboardData(text: result.outputPath));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('路径已复制到剪贴板')),
    );
  }

  void _openDirectory(BuildContext context) {
    // 获取目录路径
    final file = File(result.outputPath);
    final directory = file.parent.path;

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('文件位于: $directory')),
    );
  }
}

class _ResultRow extends StatelessWidget {
  final String label;
  final String value;
  final Color? valueColor;

  const _ResultRow({
    required this.label,
    required this.value,
    this.valueColor,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          label,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
        Text(
          value,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                fontWeight: FontWeight.w600,
                color: valueColor,
              ),
        ),
      ],
    );
  }
}
