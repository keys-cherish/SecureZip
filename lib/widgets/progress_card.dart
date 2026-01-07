import 'package:flutter/material.dart';

/// 进度卡片组件
/// 用于显示压缩/解压/上传/下载进度
class ProgressCard extends StatelessWidget {
  final String title;
  final double progress;
  final String processedText;
  final String speedText;
  final String remainingText;
  final String? currentFile;
  final VoidCallback? onCancel;

  const ProgressCard({
    super.key,
    required this.title,
    required this.progress,
    required this.processedText,
    required this.speedText,
    required this.remainingText,
    this.currentFile,
    this.onCancel,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final progressPercent = (progress * 100).toInt();

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // 标题和百分比
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  title,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                Text(
                  '$progressPercent%',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        color: colorScheme.primary,
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // 进度条
            ClipRRect(
              borderRadius: BorderRadius.circular(4),
              child: LinearProgressIndicator(
                value: progress,
                minHeight: 8,
                backgroundColor: colorScheme.surfaceContainerHighest,
              ),
            ),
            const SizedBox(height: 16),

            // 当前文件
            if (currentFile != null && currentFile!.isNotEmpty) ...[
              Row(
                children: [
                  Icon(
                    Icons.insert_drive_file_outlined,
                    size: 16,
                    color: colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      currentFile!,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
            ],

            // 详细信息
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                children: [
                  _InfoRow(label: '进度', value: processedText),
                  const SizedBox(height: 4),
                  _InfoRow(label: '速度', value: speedText),
                  const SizedBox(height: 4),
                  _InfoRow(label: '剩余时间', value: remainingText),
                ],
              ),
            ),

            // 取消按钮
            if (onCancel != null) ...[
              const SizedBox(height: 16),
              OutlinedButton(
                onPressed: onCancel,
                child: const Text('取消'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;

  const _InfoRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          label,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
        Text(
          value,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                fontWeight: FontWeight.w500,
              ),
        ),
      ],
    );
  }
}
