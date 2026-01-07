import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';
import '../models/webdav_config.dart';
import '../services/webdav_service.dart';
import '../services/password_service.dart';
import '../services/mapping_service.dart';
import '../widgets/password_field.dart';

/// WebDAV 设置页面
class WebDavPage extends StatefulWidget {
  const WebDavPage({super.key});

  @override
  State<WebDavPage> createState() => _WebDavPageState();
}

class _WebDavPageState extends State<WebDavPage> {
  final _formKey = GlobalKey<FormState>();
  final _serverUrlController = TextEditingController();
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();
  final _remotePathController = TextEditingController();
  final _backupPasswordController = TextEditingController();

  bool _isTesting = false;
  bool _isSaving = false;
  bool _isBackingUp = false;
  bool _isRestoring = false;
  String? _testResult;
  bool _testSuccess = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadConfig();
    });
  }

  @override
  void dispose() {
    _serverUrlController.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
    _remotePathController.dispose();
    _backupPasswordController.dispose();
    super.dispose();
  }

  void _loadConfig() {
    final service = context.read<WebDavService>();
    if (!service.isLoaded) {
      service.load().then((_) => _populateFields());
    } else {
      _populateFields();
    }
  }

  void _populateFields() {
    final config = context.read<WebDavService>().config;
    if (config != null) {
      _serverUrlController.text = config.serverUrl;
      _usernameController.text = config.username;
      _passwordController.text = config.password;
      _remotePathController.text = config.remotePath;
    }
  }

  Future<void> _saveConfig() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _isSaving = true;
    });

    try {
      final config = WebDavConfig(
        serverUrl: _serverUrlController.text.trim(),
        username: _usernameController.text.trim(),
        password: _passwordController.text,
        remotePath: _remotePathController.text.trim().isEmpty
            ? '/'
            : _remotePathController.text.trim(),
      );

      await context.read<WebDavService>().saveConfig(config);

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('配置已保存')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('保存失败: $e')),
        );
      }
    } finally {
      if (mounted) {
        setState(() {
          _isSaving = false;
        });
      }
    }
  }

  Future<void> _testConnection() async {
    if (!_formKey.currentState!.validate()) return;

    // 先保存配置
    await _saveConfig();

    setState(() {
      _isTesting = true;
      _testResult = null;
    });

    try {
      final success = await context.read<WebDavService>().testConnection();

      if (mounted) {
        setState(() {
          _testSuccess = success;
          _testResult = success ? '连接成功！' : '连接失败';
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _testSuccess = false;
          _testResult = '连接失败: $e';
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _isTesting = false;
        });
      }
    }
  }

  Future<void> _backupAppData() async {
    // 显示备份密码输入对话框
    final password = await _showBackupPasswordDialog(isBackup: true);
    if (password == null) return;

    setState(() {
      _isBackingUp = true;
    });

    try {
      // 收集应用数据
      final passwordService = context.read<PasswordService>();
      final mappingService = context.read<MappingService>();
      final webdavService = context.read<WebDavService>();

      final backupData = {
        'version': 1,
        'timestamp': DateTime.now().toIso8601String(),
        'appName': 'SecureZip',
        'passwords': passwordService.exportData(),
        'mappings': mappingService.exportData(),
      };

      // 使用WebDAV服务加密备份
      await webdavService.backupAppData(
        password: password,
        backupData: backupData,
        onProgress: (progress) {
          // 可以添加进度显示
        },
      );

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('✓ 应用数据已加密备份到 WebDAV'),
            backgroundColor: Colors.green,
          ),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('备份失败: $e')),
        );
      }
    } finally {
      if (mounted) {
        setState(() {
          _isBackingUp = false;
        });
      }
    }
  }

  /// 显示备份文件选择对话框
  Future<String?> _showBackupSelectionDialog() async {
    final webdavService = context.read<WebDavService>();

    try {
      // 显示加载指示
      setState(() {
        _isRestoring = true;
      });

      final backups = await webdavService.listBackupFiles();

      setState(() {
        _isRestoring = false;
      });

      if (backups.isEmpty) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('没有找到备份文件')),
          );
        }
        return null;
      }

      // 如果只有一个备份，直接返回
      if (backups.length == 1) {
        return backups.first.fileName;
      }

      // 显示选择对话框
      if (!mounted) return null;
      return showDialog<String>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: const Text('选择要恢复的备份'),
            content: SizedBox(
              width: double.maxFinite,
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: backups.length,
                itemBuilder: (context, index) {
                  final backup = backups[index];
                  return ListTile(
                    leading: Icon(
                      Icons.backup,
                      color: index == 0 ? Colors.green : Colors.grey,
                    ),
                    title: Text(
                      backup.displayDate,
                      style: TextStyle(
                        fontWeight:
                            index == 0 ? FontWeight.bold : FontWeight.normal,
                      ),
                    ),
                    subtitle: Text(backup.displaySize),
                    trailing: index == 0
                        ? Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 2,
                            ),
                            decoration: BoxDecoration(
                              color: Colors.green,
                              borderRadius: BorderRadius.circular(10),
                            ),
                            child: const Text(
                              '最新',
                              style: TextStyle(
                                color: Colors.white,
                                fontSize: 12,
                              ),
                            ),
                          )
                        : null,
                    onTap: () {
                      Navigator.of(context).pop(backup.fileName);
                    },
                  );
                },
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('取消'),
              ),
            ],
          );
        },
      );
    } catch (e) {
      setState(() {
        _isRestoring = false;
      });
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('获取备份列表失败: $e')),
        );
      }
      return null;
    }
  }

  Future<void> _restoreAppData() async {
    // 先选择要恢复的备份文件
    final selectedFile = await _showBackupSelectionDialog();
    if (selectedFile == null) return;

    // 显示恢复密码输入对话框
    final password = await _showBackupPasswordDialog(isBackup: false);
    if (password == null) return;

    setState(() {
      _isRestoring = true;
    });

    try {
      final webdavService = context.read<WebDavService>();
      final passwordService = context.read<PasswordService>();
      final mappingService = context.read<MappingService>();

      // 从WebDAV下载并解密备份数据
      final backupData = await webdavService.restoreAppData(
        password: password,
        fileName: selectedFile,
        onProgress: (progress) {
          // 可以添加进度显示
        },
      );

      // 验证数据版本
      if (backupData['appName'] != 'SecureZip') {
        throw Exception('备份数据格式不正确');
      }

      // 恢复密码本
      if (backupData['passwords'] != null) {
        await passwordService.importData(backupData['passwords']);
      }

      // 恢复映射表
      if (backupData['mappings'] != null) {
        await mappingService.importData(backupData['mappings']);
      }

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('✓ 应用数据已从备份恢复'),
            backgroundColor: Colors.green,
          ),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('恢复失败: $e')),
        );
      }
    } finally {
      if (mounted) {
        setState(() {
          _isRestoring = false;
        });
      }
    }
  }

  Future<String?> _showBackupPasswordDialog({required bool isBackup}) async {
    _backupPasswordController.clear();

    return showDialog<String>(
      context: context,
      builder: (context) {
        return Consumer<PasswordService>(
          builder: (context, passwordService, _) {
            final passwords = passwordService.passwords;

            return AlertDialog(
              title: Text(isBackup ? '设置备份密码' : '输入备份密码'),
              content: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(isBackup
                        ? '请设置一个密码来加密备份数据。请牢记此密码，恢复时需要使用。'
                        : '请输入备份时设置的密码。'),
                    const SizedBox(height: 16),
                    PasswordField(
                      controller: _backupPasswordController,
                      labelText: '备份密码',
                    ),
                    if (passwords.isNotEmpty) ...[
                      const SizedBox(height: 16),
                      const Divider(),
                      const SizedBox(height: 8),
                      Text(
                        '从密码本快捷选择:',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Theme.of(context)
                                  .colorScheme
                                  .onSurfaceVariant,
                            ),
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: passwords.map((entry) {
                          return ActionChip(
                            avatar: const Icon(Icons.vpn_key, size: 16),
                            label: Text(entry.name),
                            onPressed: () {
                              _backupPasswordController.text = entry.password;
                            },
                          );
                        }).toList(),
                      ),
                    ],
                  ],
                ),
              ),
              actions: [
                TextButton(
                  onPressed: () => Navigator.pop(context),
                  child: const Text('取消'),
                ),
                FilledButton(
                  onPressed: () {
                    if (_backupPasswordController.text.isEmpty) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(content: Text('请输入密码')),
                      );
                      return;
                    }
                    Navigator.pop(context, _backupPasswordController.text);
                  },
                  child: const Text('确定'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('WebDAV 设置'),
        actions: [
          Consumer<WebDavService>(
            builder: (context, service, _) {
              if (service.isConfigured) {
                return IconButton(
                  icon: const Icon(Icons.folder_outlined),
                  onPressed: () => context.push('/webdav/files'),
                  tooltip: '浏览文件',
                );
              }
              return const SizedBox.shrink();
            },
          ),
        ],
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Form(
            key: _formKey,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // 连接设置
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Text(
                          '连接设置',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        const SizedBox(height: 16),
                        TextFormField(
                          controller: _serverUrlController,
                          decoration: const InputDecoration(
                            labelText: '服务器地址',
                            hintText: 'https://dav.example.com',
                            prefixIcon: Icon(Icons.link),
                          ),
                          keyboardType: TextInputType.url,
                          validator: (value) {
                            if (value == null || value.trim().isEmpty) {
                              return '请输入服务器地址';
                            }
                            if (!value.startsWith('http://') &&
                                !value.startsWith('https://')) {
                              return '请输入有效的URL';
                            }
                            return null;
                          },
                        ),
                        const SizedBox(height: 16),
                        TextFormField(
                          controller: _usernameController,
                          decoration: const InputDecoration(
                            labelText: '用户名',
                            prefixIcon: Icon(Icons.person_outline),
                          ),
                          validator: (value) {
                            if (value == null || value.trim().isEmpty) {
                              return '请输入用户名';
                            }
                            return null;
                          },
                        ),
                        const SizedBox(height: 16),
                        PasswordField(
                          controller: _passwordController,
                          labelText: '密码',
                          prefixIcon: Icons.lock_outline,
                          validator: (value) {
                            if (value == null || value.isEmpty) {
                              return '请输入密码';
                            }
                            return null;
                          },
                        ),
                        const SizedBox(height: 16),
                        TextFormField(
                          controller: _remotePathController,
                          decoration: const InputDecoration(
                            labelText: '远程目录（可选）',
                            hintText: '/SecureZip/',
                            prefixIcon: Icon(Icons.folder_outlined),
                          ),
                        ),
                        const SizedBox(height: 24),
                        Row(
                          children: [
                            Expanded(
                              child: OutlinedButton(
                                onPressed: _isTesting || _isSaving
                                    ? null
                                    : _testConnection,
                                child: _isTesting
                                    ? const SizedBox(
                                        width: 20,
                                        height: 20,
                                        child: CircularProgressIndicator(
                                            strokeWidth: 2),
                                      )
                                    : const Text('测试连接'),
                              ),
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: FilledButton(
                                onPressed: _isTesting || _isSaving
                                    ? null
                                    : _saveConfig,
                                child: _isSaving
                                    ? const SizedBox(
                                        width: 20,
                                        height: 20,
                                        child: CircularProgressIndicator(
                                            strokeWidth: 2),
                                      )
                                    : const Text('保存配置'),
                              ),
                            ),
                          ],
                        ),
                        if (_testResult != null) ...[
                          const SizedBox(height: 16),
                          Container(
                            padding: const EdgeInsets.all(12),
                            decoration: BoxDecoration(
                              color: _testSuccess
                                  ? colorScheme.primaryContainer
                                  : colorScheme.errorContainer,
                              borderRadius: BorderRadius.circular(8),
                            ),
                            child: Row(
                              children: [
                                Icon(
                                  _testSuccess
                                      ? Icons.check_circle_outline
                                      : Icons.error_outline,
                                  color: _testSuccess
                                      ? colorScheme.onPrimaryContainer
                                      : colorScheme.onErrorContainer,
                                ),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: Text(
                                    _testResult!,
                                    style: TextStyle(
                                      color: _testSuccess
                                          ? colorScheme.onPrimaryContainer
                                          : colorScheme.onErrorContainer,
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),
                ),

                const SizedBox(height: 16),

                // 数据备份与恢复
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Text(
                          '数据备份与恢复',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        const SizedBox(height: 8),
                        Text(
                          '将密码本和映射表等应用数据加密备份到 WebDAV',
                          style:
                              Theme.of(context).textTheme.bodySmall?.copyWith(
                                    color: colorScheme.onSurfaceVariant,
                                  ),
                        ),
                        const SizedBox(height: 16),
                        Row(
                          children: [
                            Expanded(
                              child: OutlinedButton.icon(
                                onPressed: _isBackingUp || _isRestoring
                                    ? null
                                    : _backupAppData,
                                icon: _isBackingUp
                                    ? const SizedBox(
                                        width: 18,
                                        height: 18,
                                        child: CircularProgressIndicator(
                                            strokeWidth: 2),
                                      )
                                    : const Icon(Icons.backup_outlined),
                                label: const Text('备份数据'),
                              ),
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: OutlinedButton.icon(
                                onPressed: _isBackingUp || _isRestoring
                                    ? null
                                    : _restoreAppData,
                                icon: _isRestoring
                                    ? const SizedBox(
                                        width: 18,
                                        height: 18,
                                        child: CircularProgressIndicator(
                                            strokeWidth: 2),
                                      )
                                    : const Icon(Icons.restore_outlined),
                                label: const Text('恢复数据'),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
