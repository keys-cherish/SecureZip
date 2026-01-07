import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

/// 首页
/// 简洁的功能入口，不显示统计信息
class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('SecureZip'),
        actions: [
          IconButton(
            icon: const Icon(Icons.settings_outlined),
            onPressed: () => context.push('/settings'),
            tooltip: '设置',
          ),
        ],
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 压缩文件
              _FeatureCard(
                icon: Icons.archive_outlined,
                iconColor: colorScheme.primary,
                title: '压缩文件',
                subtitle: '选择文件或文件夹进行压缩',
                onTap: () => context.push('/compress'),
              ),
              const SizedBox(height: 12),

              // 解压文件
              _FeatureCard(
                icon: Icons.unarchive_outlined,
                iconColor: colorScheme.secondary,
                title: '解压文件',
                subtitle: '选择 .7z 文件进行解压',
                onTap: () => context.push('/decompress'),
              ),
              const SizedBox(height: 12),

              // 密码本
              _FeatureCard(
                icon: Icons.key_outlined,
                iconColor: colorScheme.tertiary,
                title: '密码本',
                subtitle: '管理压缩密码',
                onTap: () => context.push('/passwords'),
              ),
              const SizedBox(height: 12),

              // WebDAV
              _FeatureCard(
                icon: Icons.cloud_outlined,
                iconColor: colorScheme.primary,
                title: 'WebDAV',
                subtitle: '云端备份与同步',
                onTap: () => context.push('/webdav'),
              ),
              const SizedBox(height: 12),

              // 映射表
              _FeatureCard(
                icon: Icons.list_alt_outlined,
                iconColor: colorScheme.secondary,
                title: '映射表',
                subtitle: '查看文件名混淆记录与后缀密码配置',
                onTap: () => context.push('/mappings'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 功能卡片组件
class _FeatureCard extends StatelessWidget {
  final IconData icon;
  final Color iconColor;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  const _FeatureCard({
    required this.icon,
    required this.iconColor,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  color: iconColor.withAlpha(30),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  icon,
                  color: iconColor,
                  size: 24,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      subtitle,
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right,
                color: colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
