import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/compress_options.dart';
import '../services/rust_compress_service.dart';
import '../services/mapping_service.dart';
import '../ffi/rust_compress_ffi.dart';
import 'compress_result_page.dart';

/// 压缩进度页面
class CompressProgressPage extends StatefulWidget {
  final List<String> inputPaths;
  final String outputPath;
  final CompressOptions options;

  const CompressProgressPage({
    super.key,
    required this.inputPaths,
    required this.outputPath,
    required this.options,
  });

  @override
  State<CompressProgressPage> createState() => _CompressProgressPageState();
}

class _CompressProgressPageState extends State<CompressProgressPage> {
  final RustCompressService _compressService = RustCompressService();
  CompressProgress? _currentProgress;
  StreamSubscription<CompressProgress>? _subscription;
  bool _isCancelled = false;
  bool _isCancelling = false; // 防止重复取消
  String? _error;

  @override
  void initState() {
    super.initState();
    _startCompress();
  }

  @override
  void dispose() {
    _isCancelled = true;
    _subscription?.cancel();
    super.dispose();
  }

  Future<void> _startCompress() async {
    try {
      final stream = _compressService.compress(
        inputPaths: widget.inputPaths,
        outputPath: widget.outputPath,
        options: widget.options,
      );

      _subscription = stream.listen(
        (progress) {
          if (!_isCancelled && mounted) {
            setState(() {
              _currentProgress = progress;
            });
          }
        },
        onError: (error) {
          if (mounted) {
            setState(() {
              _error = error.toString();
            });
          }
        },
        onDone: () async {
          if (!_isCancelled && mounted) {
            // 保存文件名混淆映射（如果有的话）
            if (widget.options.enableObfuscation &&
                _compressService.lastMappings.isNotEmpty) {
              try {
                final mappingService = context.read<MappingService>();
                await mappingService.addMappings(_compressService.lastMappings);
                _compressService.clearMappings();
              } catch (e) {
                print('保存映射失败: $e');
              }
            }

            // 获取结果并跳转到结果页面
            final result = await _compressService.getResult(
              inputPaths: widget.inputPaths,
              outputPath: widget.outputPath,
            );

            if (mounted) {
              Navigator.of(context).pushReplacement(
                MaterialPageRoute(
                  builder: (context) => CompressResultPage(
                    result: result,
                    algorithm: widget.options.algorithm,
                  ),
                ),
              );
            }
          }
        },
      );
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
        });
      }
    }
  }

  Future<void> _cancelCompress() async {
    // 防止重复取消
    if (_isCancelling) return;

    setState(() {
      _isCancelling = true;
      _isCancelled = true;
    });

    // 请求 Rust 端取消操作
    RustCompressLib.instance.requestCancel();

    // 立即取消订阅，停止接收进度更新
    _subscription?.cancel();
    _subscription = null;

    // 立即返回上一页（异步）
    if (mounted) {
      Navigator.of(context).pop();
    }

    // 后台清理临时文件（不阻塞 UI）
    _cleanupOutputFilesAsync();
  }

  /// 异步清理输出文件（后台执行，不阻塞 UI）
  void _cleanupOutputFilesAsync() {
    Future.microtask(() async {
      // 稍微等待让 Rust 端完成取消
      await Future.delayed(const Duration(milliseconds: 500));

      try {
        final outputFile = File(widget.outputPath);
        if (await outputFile.exists()) {
          await outputFile.delete();
        }
        // 也清理可能的其他格式文件
        final basePath =
            widget.outputPath.replaceAll(RegExp(r'\.(szp|7z|zip|sz7z)$'), '');
        for (final ext in [
          '.szp',
          '.7z',
          '.zip',
          '.sz7z',
          '.szp.tmp',
          '.7z.tmp'
        ]) {
          final file = File('$basePath$ext');
          if (await file.exists()) {
            await file.delete();
          }
        }
      } catch (e) {
        // 忽略清理错误
        print('清理临时文件失败: $e');
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final progress = _currentProgress;

    return Scaffold(
      appBar: AppBar(
        title: const Text('压缩中'),
        automaticallyImplyLeading: false,
      ),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: _error != null
              ? _buildErrorView(colorScheme)
              : _buildProgressView(colorScheme, progress),
        ),
      ),
    );
  }

  Widget _buildProgressView(
      ColorScheme colorScheme, CompressProgress? progress) {
    final percentage = progress != null ? (progress.progress * 100).toInt() : 0;

    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        const Spacer(),

        // 百分比显示
        Text(
          '$percentage%',
          style: Theme.of(context).textTheme.displayLarge?.copyWith(
                fontWeight: FontWeight.bold,
                color: colorScheme.primary,
              ),
        ),

        const SizedBox(height: 32),

        // 进度条
        ClipRRect(
          borderRadius: BorderRadius.circular(8),
          child: LinearProgressIndicator(
            value: progress?.progress ?? 0,
            minHeight: 12,
            backgroundColor: colorScheme.surfaceContainerHighest,
          ),
        ),

        const SizedBox(height: 32),

        // 详细信息卡片
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                _InfoRow(
                  icon: Icons.folder_outlined,
                  label: '已处理',
                  value: progress != null
                      ? '${progress.displayProcessed} / ${progress.displayTotal}'
                      : '计算中...',
                ),
                const Divider(height: 24),
                _InfoRow(
                  icon: Icons.speed_outlined,
                  label: '速度',
                  value: progress?.displaySpeed ?? '计算中...',
                ),
                const Divider(height: 24),
                _InfoRow(
                  icon: Icons.timer_outlined,
                  label: '剩余时间',
                  value: progress?.displayRemaining ?? '计算中...',
                ),
                if (progress?.currentFile != null &&
                    progress!.currentFile != '完成') ...[
                  const Divider(height: 24),
                  _InfoRow(
                    icon: Icons.insert_drive_file_outlined,
                    label: '当前文件',
                    value: progress.currentFile,
                  ),
                ],
              ],
            ),
          ),
        ),

        const Spacer(),

        // 取消按钮
        SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            onPressed: _cancelCompress,
            icon: const Icon(Icons.close),
            label: const Text('取消压缩'),
            style: OutlinedButton.styleFrom(
              foregroundColor: colorScheme.error,
              side: BorderSide(color: colorScheme.error),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildErrorView(ColorScheme colorScheme) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.error_outline,
            size: 64,
            color: colorScheme.error,
          ),
          const SizedBox(height: 16),
          Text(
            '压缩失败',
            style: Theme.of(context).textTheme.titleLarge?.copyWith(
                  color: colorScheme.error,
                ),
          ),
          const SizedBox(height: 8),
          Text(
            _error ?? '未知错误',
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
          ),
          const SizedBox(height: 24),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('返回'),
          ),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;

  const _InfoRow({
    required this.icon,
    required this.label,
    required this.value,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, size: 20, color: Theme.of(context).colorScheme.primary),
        const SizedBox(width: 12),
        Text(
          label,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
        const Spacer(),
        Flexible(
          child: Text(
            value,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }
}
