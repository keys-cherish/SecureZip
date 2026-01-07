import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:file_picker/file_picker.dart';
import '../services/settings_service.dart';

/// 设置页面
class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  String _effectiveOutputDir = '';
  String _effectiveDecompressDir = '';

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      final settings = context.read<SettingsService>();
      await settings.load();
      _loadEffectiveDirs(settings);
    });
  }

  Future<void> _loadEffectiveDirs(SettingsService settings) async {
    final outputDir = await settings.getEffectiveOutputDir();
    final decompressDir = await settings.getEffectiveDecompressOutputDir();
    if (mounted) {
      setState(() {
        _effectiveOutputDir = outputDir;
        _effectiveDecompressDir = decompressDir;
      });
    }
  }

  Future<void> _pickOutputDir(SettingsService settings) async {
    final result = await FilePicker.platform.getDirectoryPath();
    if (result != null) {
      await settings.setOutputDir(result);
      setState(() {
        _effectiveOutputDir = result;
      });
    }
  }

  Future<void> _pickDecompressOutputDir(SettingsService settings) async {
    final result = await FilePicker.platform.getDirectoryPath();
    if (result != null) {
      await settings.setDecompressOutputDir(result);
      setState(() {
        _effectiveDecompressDir = result;
      });
    }
  }

  Future<void> _resetOutputDirs(SettingsService settings) async {
    await settings.resetOutputDirs();
    await _loadEffectiveDirs(settings);
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('输出目录已重置为默认值')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('设置'),
      ),
      body: SafeArea(
        child: Consumer<SettingsService>(
          builder: (context, settings, _) {
            if (!settings.isLoaded) {
              return const Center(child: CircularProgressIndicator());
            }

            return SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // 外观设置
                  _SettingsSection(
                    title: '外观',
                    children: [
                      _SettingsTile(
                        icon: Icons.palette_outlined,
                        title: '主题模式',
                        subtitle: settings
                            .getThemeModeDisplayName(settings.themeMode),
                        onTap: () => _showThemePicker(context, settings),
                      ),
                    ],
                  ),

                  const SizedBox(height: 16),

                  // 输出目录设置
                  _SettingsSection(
                    title: '输出目录',
                    children: [
                      _SettingsTile(
                        icon: Icons.folder_outlined,
                        title: '压缩输出目录',
                        subtitle: _effectiveOutputDir.isEmpty
                            ? '加载中...'
                            : _effectiveOutputDir,
                        onTap: () => _pickOutputDir(settings),
                      ),
                      _SettingsTile(
                        icon: Icons.folder_open_outlined,
                        title: '解压输出目录',
                        subtitle: _effectiveDecompressDir.isEmpty
                            ? '加载中...'
                            : _effectiveDecompressDir,
                        onTap: () => _pickDecompressOutputDir(settings),
                      ),
                      _SettingsTile(
                        icon: Icons.refresh,
                        title: '重置为默认目录',
                        subtitle: '恢复默认的输出目录设置',
                        onTap: () => _resetOutputDirs(settings),
                      ),
                    ],
                  ),

                  const SizedBox(height: 16),

                  // 压缩设置
                  _SettingsSection(
                    title: '压缩',
                    children: [
                      _SettingsTile(
                        icon: Icons.compress,
                        title: '压缩级别',
                        subtitle: '${settings.compressionLevel} (1-9, 越高压缩率越大)',
                        onTap: () =>
                            _showCompressionLevelPicker(context, settings),
                      ),
                      _SettingsTile(
                        icon: Icons.shuffle,
                        title: '默认混淆方案',
                        subtitle: _getSchemeDisplayName(
                            settings.defaultObfuscationScheme),
                        onTap: () => _showSchemePicker(context, settings),
                      ),
                    ],
                  ),

                  const SizedBox(height: 16),

                  // 关于
                  _SettingsSection(
                    title: '关于',
                    children: [
                      _SettingsTile(
                        icon: Icons.info_outline,
                        title: '版本',
                        subtitle: '1.0.0',
                        onTap: () {},
                      ),
                      _SettingsTile(
                        icon: Icons.code,
                        title: '技术栈',
                        subtitle: 'Flutter + Rust',
                        onTap: () {},
                      ),
                    ],
                  ),
                ],
              ),
            );
          },
        ),
      ),
    );
  }

  String _getSchemeDisplayName(String scheme) {
    switch (scheme) {
      case 'sequential':
        return '序号模式';
      case 'dateSequential':
        return '日期序号模式';
      case 'random':
        return '随机字符模式';
      case 'hash':
        return '哈希模式';
      case 'encrypted':
        return '加密模式';
      default:
        return scheme;
    }
  }

  void _showThemePicker(BuildContext context, SettingsService settings) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('选择主题'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: ThemeMode.values.map((mode) {
            return RadioListTile<ThemeMode>(
              title: Text(settings.getThemeModeDisplayName(mode)),
              value: mode,
              groupValue: settings.themeMode,
              onChanged: (value) {
                if (value != null) {
                  settings.setThemeMode(value);
                  Navigator.pop(context);
                }
              },
            );
          }).toList(),
        ),
      ),
    );
  }

  void _showCompressionLevelPicker(
      BuildContext context, SettingsService settings) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('压缩级别'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              '当前级别: ${settings.compressionLevel}',
              style: Theme.of(context).textTheme.bodyLarge,
            ),
            const SizedBox(height: 16),
            StatefulBuilder(
              builder: (context, setDialogState) {
                return Slider(
                  value: settings.compressionLevel.toDouble(),
                  min: 1,
                  max: 9,
                  divisions: 8,
                  label: settings.compressionLevel.toString(),
                  onChanged: (value) {
                    settings.setCompressionLevel(value.toInt());
                    setDialogState(() {});
                  },
                );
              },
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '快速',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                Text(
                  '最优',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ],
        ),
        actions: [
          FilledButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('确定'),
          ),
        ],
      ),
    );
  }

  void _showSchemePicker(BuildContext context, SettingsService settings) {
    final schemes = [
      ('sequential', '序号模式', '001.dat, 002.dat'),
      ('dateSequential', '日期序号模式', '20240115_001.dat'),
      ('random', '随机字符模式', 'a7x2k9m3.dat'),
      ('hash', '哈希模式', '8a3c2b1f.dat'),
      ('encrypted', '加密模式', 'Base64(AES).enc'),
    ];

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('默认混淆方案'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: schemes.map((scheme) {
              return RadioListTile<String>(
                title: Text(scheme.$2),
                subtitle: Text(scheme.$3),
                value: scheme.$1,
                groupValue: settings.defaultObfuscationScheme,
                onChanged: (value) {
                  if (value != null) {
                    settings.setDefaultObfuscationScheme(value);
                    Navigator.pop(context);
                  }
                },
              );
            }).toList(),
          ),
        ),
      ),
    );
  }
}

class _SettingsSection extends StatelessWidget {
  final String title;
  final List<Widget> children;

  const _SettingsSection({
    required this.title,
    required this.children,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: 4, bottom: 8),
          child: Text(
            title,
            style: Theme.of(context).textTheme.titleSmall?.copyWith(
                  color: Theme.of(context).colorScheme.primary,
                  fontWeight: FontWeight.w600,
                ),
          ),
        ),
        Card(
          child: Column(
            children: children,
          ),
        ),
      ],
    );
  }
}

class _SettingsTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  const _SettingsTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Row(
          children: [
            Icon(icon, color: colorScheme.onSurfaceVariant),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: Theme.of(context).textTheme.bodyLarge,
                  ),
                  Text(
                    subtitle,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
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
    );
  }
}
