import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:intl/intl.dart';
import 'dart:io';
import 'package:file_picker/file_picker.dart';
import 'package:path_provider/path_provider.dart';
import 'package:share_plus/share_plus.dart';
import '../models/password_entry.dart';
import '../services/password_service.dart';
import '../widgets/password_field.dart';

/// 密码本页面
class PasswordsPage extends StatefulWidget {
  const PasswordsPage({super.key});

  @override
  State<PasswordsPage> createState() => _PasswordsPageState();
}

class _PasswordsPageState extends State<PasswordsPage> {
  final TextEditingController _searchController = TextEditingController();
  String _searchQuery = '';

  @override
  void initState() {
    super.initState();
    // 加载密码数据
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<PasswordService>().load();
    });
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _showAddDialog() {
    showDialog(
      context: context,
      builder: (context) => const _PasswordDialog(),
    );
  }

  void _showEditDialog(PasswordEntry entry) {
    showDialog(
      context: context,
      builder: (context) => _PasswordDialog(entry: entry),
    );
  }

  Future<void> _deletePassword(PasswordEntry entry) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('删除密码'),
        content: Text('确定要删除密码 "${entry.name}" 吗？此操作不可撤销。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('删除'),
          ),
        ],
      ),
    );

    if (confirmed == true && mounted) {
      await context.read<PasswordService>().deletePassword(entry.id);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('密码已删除')),
        );
      }
    }
  }

  void _copyPassword(PasswordEntry entry) {
    Clipboard.setData(ClipboardData(text: entry.password));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('密码已复制到剪贴板'),
        duration: Duration(seconds: 2),
      ),
    );
  }

  /// 显示导入导出菜单
  void _showImportExportMenu() {
    showModalBottomSheet(
      context: context,
      builder: (context) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: const Icon(Icons.file_upload),
              title: const Text('导入密码'),
              subtitle: const Text('从 TXT/JSON/CSV 文件导入'),
              onTap: () {
                Navigator.pop(context);
                _importPasswords();
              },
            ),
            ListTile(
              leading: const Icon(Icons.file_download),
              title: const Text('导出为 TXT'),
              subtitle: const Text('简单文本格式，每行一个密码'),
              onTap: () {
                Navigator.pop(context);
                _exportPasswords('txt');
              },
            ),
            ListTile(
              leading: const Icon(Icons.code),
              title: const Text('导出为 JSON'),
              subtitle: const Text('标准 JSON 格式，便于程序处理'),
              onTap: () {
                Navigator.pop(context);
                _exportPasswords('json');
              },
            ),
            ListTile(
              leading: const Icon(Icons.table_chart),
              title: const Text('导出为 CSV'),
              subtitle: const Text('表格格式，可用 Excel 打开'),
              onTap: () {
                Navigator.pop(context);
                _exportPasswords('csv');
              },
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }

  /// 导入密码
  Future<void> _importPasswords() async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['txt', 'json', 'csv'],
        allowMultiple: false,
      );

      if (result == null || result.files.isEmpty) return;

      final file = File(result.files.first.path!);
      final content = await file.readAsString();
      final extension = result.files.first.extension?.toLowerCase() ?? '';

      // 询问合并还是覆盖
      final merge = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('导入方式'),
          content: const Text('选择导入方式：\n\n'
              '• 合并：将新密码添加到现有密码（跳过重复）\n'
              '• 覆盖：用导入的密码替换所有现有密码'),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('取消'),
            ),
            TextButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('合并'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('覆盖'),
            ),
          ],
        ),
      );

      if (merge == null) return;

      final service = context.read<PasswordService>();
      int count = 0;

      switch (extension) {
        case 'txt':
          count = await service.importFromTxt(content, merge: merge);
          break;
        case 'json':
          count = await service.importFromJson(content, merge: merge);
          break;
        case 'csv':
          count = await service.importFromCsv(content, merge: merge);
          break;
        default:
          throw Exception('不支持的文件格式');
      }

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('成功导入 $count 条密码')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('导入失败: $e')),
        );
      }
    }
  }

  /// 导出密码
  Future<void> _exportPasswords(String format) async {
    final service = context.read<PasswordService>();

    if (service.passwords.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('没有密码可以导出')),
      );
      return;
    }

    try {
      String content;
      String fileName;

      switch (format) {
        case 'txt':
          content = service.exportToTxt();
          fileName =
              'securezip_passwords_${DateFormat('yyyyMMdd').format(DateTime.now())}.txt';
          break;
        case 'json':
          content = service.exportToJson();
          fileName =
              'securezip_passwords_${DateFormat('yyyyMMdd').format(DateTime.now())}.json';
          break;
        case 'csv':
          content = service.exportToCsv();
          fileName =
              'securezip_passwords_${DateFormat('yyyyMMdd').format(DateTime.now())}.csv';
          break;
        default:
          throw Exception('不支持的导出格式');
      }

      // 保存到临时目录
      final tempDir = await getTemporaryDirectory();
      final file = File('${tempDir.path}/$fileName');
      await file.writeAsString(content);

      // 分享文件
      await Share.shareXFiles(
        [XFile(file.path)],
        subject: 'SecureZip 密码本导出',
      );
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('导出失败: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('密码本'),
        actions: [
          IconButton(
            icon: const Icon(Icons.add),
            onPressed: _showAddDialog,
            tooltip: '添加密码',
          ),
          IconButton(
            icon: const Icon(Icons.import_export),
            onPressed: _showImportExportMenu,
            tooltip: '导入/导出',
          ),
        ],
      ),
      body: SafeArea(
        child: Column(
          children: [
            // 搜索栏
            Padding(
              padding: const EdgeInsets.all(16),
              child: TextField(
                controller: _searchController,
                decoration: InputDecoration(
                  hintText: '搜索密码...',
                  prefixIcon: const Icon(Icons.search),
                  suffixIcon: _searchQuery.isNotEmpty
                      ? IconButton(
                          icon: const Icon(Icons.clear),
                          onPressed: () {
                            _searchController.clear();
                            setState(() {
                              _searchQuery = '';
                            });
                          },
                        )
                      : null,
                ),
                onChanged: (value) {
                  setState(() {
                    _searchQuery = value;
                  });
                },
              ),
            ),

            // 密码列表
            Expanded(
              child: Consumer<PasswordService>(
                builder: (context, service, _) {
                  if (!service.isLoaded) {
                    return const Center(child: CircularProgressIndicator());
                  }

                  final passwords = _searchQuery.isEmpty
                      ? service.passwords
                      : service.search(_searchQuery);

                  if (passwords.isEmpty) {
                    return _buildEmptyState(colorScheme);
                  }

                  return ListView.builder(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    itemCount: passwords.length,
                    itemBuilder: (context, index) {
                      final entry = passwords[index];
                      return _PasswordCard(
                        key: ValueKey(entry.id),
                        entry: entry,
                        onEdit: () => _showEditDialog(entry),
                        onDelete: () => _deletePassword(entry),
                        onCopy: () => _copyPassword(entry),
                      );
                    },
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildEmptyState(ColorScheme colorScheme) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.key_off_outlined,
            size: 64,
            color: colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 16),
          Text(
            _searchQuery.isEmpty ? '暂无密码' : '未找到匹配的密码',
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
          ),
          const SizedBox(height: 8),
          Text(
            _searchQuery.isEmpty ? '点击右上角添加按钮开始' : '尝试其他搜索词',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
          ),
          if (_searchQuery.isEmpty) ...[
            const SizedBox(height: 24),
            FilledButton.icon(
              onPressed: _showAddDialog,
              icon: const Icon(Icons.add),
              label: const Text('添加密码'),
            ),
          ],
        ],
      ),
    );
  }
}

/// 密码卡片组件
class _PasswordCard extends StatelessWidget {
  final PasswordEntry entry;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onCopy;

  const _PasswordCard({
    super.key,
    required this.entry,
    required this.onEdit,
    required this.onDelete,
    required this.onCopy,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final dateFormat = DateFormat('yyyy-MM-dd');

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onEdit,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(
                  Icons.key,
                  color: colorScheme.onPrimaryContainer,
                  size: 20,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      entry.name,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '创建于 ${dateFormat.format(entry.createdAt)}',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                    ),
                    if (entry.remark != null && entry.remark!.isNotEmpty) ...[
                      const SizedBox(height: 4),
                      Text(
                        entry.remark!,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ],
                ),
              ),
              IconButton(
                icon: const Icon(Icons.copy),
                onPressed: onCopy,
                tooltip: '复制密码',
              ),
              PopupMenuButton<String>(
                onSelected: (value) {
                  switch (value) {
                    case 'edit':
                      onEdit();
                      break;
                    case 'delete':
                      onDelete();
                      break;
                  }
                },
                itemBuilder: (context) => [
                  const PopupMenuItem(
                    value: 'edit',
                    child: Row(
                      children: [
                        Icon(Icons.edit_outlined),
                        SizedBox(width: 12),
                        Text('编辑'),
                      ],
                    ),
                  ),
                  const PopupMenuItem(
                    value: 'delete',
                    child: Row(
                      children: [
                        Icon(Icons.delete_outline),
                        SizedBox(width: 12),
                        Text('删除'),
                      ],
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
}

/// 添加/编辑密码对话框
class _PasswordDialog extends StatefulWidget {
  final PasswordEntry? entry;

  const _PasswordDialog({this.entry});

  @override
  State<_PasswordDialog> createState() => _PasswordDialogState();
}

class _PasswordDialogState extends State<_PasswordDialog> {
  final _formKey = GlobalKey<FormState>();
  final _nameController = TextEditingController();
  final _passwordController = TextEditingController();
  final _remarkController = TextEditingController();
  bool _isSaving = false;

  bool get isEditing => widget.entry != null;

  @override
  void initState() {
    super.initState();
    if (widget.entry != null) {
      _nameController.text = widget.entry!.name;
      _passwordController.text = widget.entry!.password;
      _remarkController.text = widget.entry!.remark ?? '';
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _passwordController.dispose();
    _remarkController.dispose();
    super.dispose();
  }

  void _generateRandomPassword() {
    const chars =
        'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#\$%^&*';
    final random = List.generate(16, (index) {
      final randomIndex = DateTime.now().microsecondsSinceEpoch % chars.length;
      return chars[(randomIndex + index * 7) % chars.length];
    }).join();
    _passwordController.text = random;
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _isSaving = true;
    });

    try {
      final service = context.read<PasswordService>();

      if (isEditing) {
        final updated = widget.entry!.copyWith(
          name: _nameController.text.trim(),
          password: _passwordController.text,
          remark: _remarkController.text.trim().isEmpty
              ? null
              : _remarkController.text.trim(),
        );
        await service.updatePassword(updated);
      } else {
        final entry = PasswordEntry(
          name: _nameController.text.trim(),
          password: _passwordController.text,
          remark: _remarkController.text.trim().isEmpty
              ? null
              : _remarkController.text.trim(),
        );
        await service.addPassword(entry);
      }

      if (mounted) {
        Navigator.pop(context);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(isEditing ? '密码已更新' : '密码已添加')),
        );
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isSaving = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('保存失败: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(isEditing ? '编辑密码' : '添加密码'),
      content: SingleChildScrollView(
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              TextFormField(
                controller: _nameController,
                decoration: const InputDecoration(
                  labelText: '名称',
                  hintText: '例如：工作文件',
                ),
                validator: (value) {
                  if (value == null || value.trim().isEmpty) {
                    return '请输入名称';
                  }
                  return null;
                },
                textInputAction: TextInputAction.next,
              ),
              const SizedBox(height: 16),
              PasswordField(
                controller: _passwordController,
                labelText: '密码',
                showGenerateButton: true,
                onGenerate: _generateRandomPassword,
                validator: (value) {
                  if (value == null || value.isEmpty) {
                    return '请输入密码';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _remarkController,
                decoration: const InputDecoration(
                  labelText: '备注（可选）',
                  hintText: '添加备注信息',
                ),
                maxLines: 2,
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: _isSaving ? null : () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _isSaving ? null : _save,
          child: _isSaving
              ? const SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('保存'),
        ),
      ],
    );
  }
}
