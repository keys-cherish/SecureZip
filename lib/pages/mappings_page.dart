import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:intl/intl.dart';
import '../models/mapping_entry.dart';
import '../services/mapping_service.dart';
import '../services/password_service.dart';

/// 映射表页面
/// 包含文件名混淆映射和后缀密码映射两个功能
class MappingsPage extends StatefulWidget {
  const MappingsPage({super.key});

  @override
  State<MappingsPage> createState() => _MappingsPageState();
}

class _MappingsPageState extends State<MappingsPage>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final TextEditingController _searchController = TextEditingController();
  String _searchQuery = '';

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<MappingService>().load();
      context.read<PasswordService>().load();
    });
  }

  @override
  void dispose() {
    _tabController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('映射表'),
        bottom: TabBar(
          controller: _tabController,
          tabs: const [
            Tab(text: '文件名混淆'),
            Tab(text: '后缀密码'),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: [
          _FilenameMappingsTab(
            searchQuery: _searchQuery,
            onSearchChanged: (query) {
              setState(() {
                _searchQuery = query;
              });
            },
          ),
          const _ExtensionPasswordTab(),
        ],
      ),
    );
  }
}

/// 文件名混淆映射标签页
class _FilenameMappingsTab extends StatelessWidget {
  final String searchQuery;
  final ValueChanged<String> onSearchChanged;

  const _FilenameMappingsTab({
    required this.searchQuery,
    required this.onSearchChanged,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Column(
      children: [
        // 搜索栏
        Padding(
          padding: const EdgeInsets.all(16),
          child: TextField(
            decoration: InputDecoration(
              hintText: '搜索原名或混淆名...',
              prefixIcon: const Icon(Icons.search),
              suffixIcon: searchQuery.isNotEmpty
                  ? IconButton(
                      icon: const Icon(Icons.clear),
                      onPressed: () => onSearchChanged(''),
                    )
                  : null,
            ),
            onChanged: onSearchChanged,
          ),
        ),

        // 映射列表
        Expanded(
          child: Consumer<MappingService>(
            builder: (context, service, _) {
              if (!service.isLoaded) {
                return const Center(child: CircularProgressIndicator());
              }

              final mappings = searchQuery.isEmpty
                  ? service.mappings
                  : service.searchMappings(searchQuery);

              if (mappings.isEmpty) {
                return Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        Icons.list_alt_outlined,
                        size: 64,
                        color: colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(height: 16),
                      Text(
                        searchQuery.isEmpty ? '暂无混淆记录' : '未找到匹配的记录',
                        style:
                            Theme.of(context).textTheme.titleMedium?.copyWith(
                                  color: colorScheme.onSurfaceVariant,
                                ),
                      ),
                      const SizedBox(height: 8),
                      Text(
                        searchQuery.isEmpty ? '压缩文件时启用文件名混淆后会在此显示' : '尝试其他搜索词',
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                      ),
                    ],
                  ),
                );
              }

              return Column(
                children: [
                  // 统计信息
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    child: Row(
                      children: [
                        Text(
                          '共 ${mappings.length} 条记录',
                          style:
                              Theme.of(context).textTheme.bodySmall?.copyWith(
                                    color: colorScheme.onSurfaceVariant,
                                  ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 8),

                  // 列表
                  Expanded(
                    child: ListView.builder(
                      padding: const EdgeInsets.symmetric(horizontal: 16),
                      itemCount: mappings.length,
                      itemBuilder: (context, index) {
                        final mapping = mappings[index];
                        return _MappingCard(
                          key: ValueKey(mapping.id),
                          mapping: mapping,
                          onDelete: () async {
                            await service.deleteMapping(mapping.id);
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(content: Text('已删除')),
                              );
                            }
                          },
                        );
                      },
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}

/// 文件名映射卡片
class _MappingCard extends StatelessWidget {
  final MappingEntry mapping;
  final VoidCallback onDelete;

  const _MappingCard({
    super.key,
    required this.mapping,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final dateFormat = DateFormat('yyyy-MM-dd HH:mm');

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          mapping.originalName,
                          style: Theme.of(context).textTheme.bodyMedium,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 8),
                        child: Icon(
                          Icons.arrow_forward,
                          size: 16,
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                      Expanded(
                        child: Text(
                          mapping.obfuscatedName,
                          style:
                              Theme.of(context).textTheme.bodyMedium?.copyWith(
                                    color: colorScheme.primary,
                                  ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    dateFormat.format(mapping.createdAt),
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                  ),
                ],
              ),
            ),
            IconButton(
              icon: const Icon(Icons.delete_outline),
              onPressed: onDelete,
              tooltip: '删除',
            ),
          ],
        ),
      ),
    );
  }
}

/// 后缀密码映射标签页
class _ExtensionPasswordTab extends StatelessWidget {
  const _ExtensionPasswordTab();

  void _showAddDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => const _ExtensionPasswordDialog(),
    );
  }

  void _showEditDialog(BuildContext context, ExtensionPasswordMapping mapping) {
    showDialog(
      context: context,
      builder: (context) => _ExtensionPasswordDialog(mapping: mapping),
    );
  }

  Future<void> _deleteMapping(
      BuildContext context, ExtensionPasswordMapping mapping) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('删除映射'),
        content: Text('确定要删除 ".${mapping.extension}" 的密码映射吗？'),
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

    if (confirmed == true && context.mounted) {
      await context.read<MappingService>().deleteExtensionMapping(mapping.id);
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('已删除')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Column(
      children: [
        // 说明
        Container(
          width: double.infinity,
          margin: const EdgeInsets.all(16),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: colorScheme.primaryContainer,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Row(
            children: [
              Icon(Icons.info_outline, color: colorScheme.onPrimaryContainer),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  '设置文件后缀与密码的映射关系，压缩时会自动匹配对应密码',
                  style: TextStyle(color: colorScheme.onPrimaryContainer),
                ),
              ),
            ],
          ),
        ),

        // 添加按钮
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: FilledButton.icon(
            onPressed: () => _showAddDialog(context),
            icon: const Icon(Icons.add),
            label: const Text('添加后缀映射'),
          ),
        ),

        const SizedBox(height: 16),

        // 映射列表
        Expanded(
          child: Consumer2<MappingService, PasswordService>(
            builder: (context, mappingService, passwordService, _) {
              if (!mappingService.isLoaded || !passwordService.isLoaded) {
                return const Center(child: CircularProgressIndicator());
              }

              final mappings = mappingService.extensionMappings;

              if (mappings.isEmpty) {
                return Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        Icons.extension_outlined,
                        size: 64,
                        color: colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(height: 16),
                      Text(
                        '暂无后缀映射',
                        style:
                            Theme.of(context).textTheme.titleMedium?.copyWith(
                                  color: colorScheme.onSurfaceVariant,
                                ),
                      ),
                      const SizedBox(height: 8),
                      Text(
                        '点击上方按钮添加',
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                      ),
                    ],
                  ),
                );
              }

              return ListView.builder(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                itemCount: mappings.length,
                itemBuilder: (context, index) {
                  final mapping = mappings[index];
                  final password =
                      passwordService.getPasswordById(mapping.passwordId);

                  return _ExtensionMappingCard(
                    key: ValueKey(mapping.id),
                    mapping: mapping,
                    passwordName: password?.name ?? '(已删除)',
                    onEdit: () => _showEditDialog(context, mapping),
                    onDelete: () => _deleteMapping(context, mapping),
                  );
                },
              );
            },
          ),
        ),
      ],
    );
  }
}

/// 后缀密码映射卡片
class _ExtensionMappingCard extends StatelessWidget {
  final ExtensionPasswordMapping mapping;
  final String passwordName;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _ExtensionMappingCard({
    super.key,
    required this.mapping,
    required this.passwordName,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onEdit,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: colorScheme.secondaryContainer,
                  borderRadius: BorderRadius.circular(16),
                ),
                child: Text(
                  '.${mapping.extension}',
                  style: TextStyle(
                    color: colorScheme.onSecondaryContainer,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Icon(
                Icons.arrow_forward,
                size: 16,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(
                          Icons.key,
                          size: 16,
                          color: colorScheme.primary,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          passwordName,
                          style:
                              Theme.of(context).textTheme.bodyMedium?.copyWith(
                                    fontWeight: FontWeight.w500,
                                  ),
                        ),
                      ],
                    ),
                    if (mapping.description.isNotEmpty) ...[
                      const SizedBox(height: 4),
                      Text(
                        mapping.description,
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
                icon: const Icon(Icons.delete_outline),
                onPressed: onDelete,
                tooltip: '删除',
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 后缀密码映射对话框
class _ExtensionPasswordDialog extends StatefulWidget {
  final ExtensionPasswordMapping? mapping;

  const _ExtensionPasswordDialog({this.mapping});

  @override
  State<_ExtensionPasswordDialog> createState() =>
      _ExtensionPasswordDialogState();
}

class _ExtensionPasswordDialogState extends State<_ExtensionPasswordDialog> {
  final _formKey = GlobalKey<FormState>();
  final _extensionController = TextEditingController();
  final _descriptionController = TextEditingController();
  String? _selectedPasswordId;
  bool _isSaving = false;

  bool get isEditing => widget.mapping != null;

  @override
  void initState() {
    super.initState();
    if (widget.mapping != null) {
      _extensionController.text = widget.mapping!.extension;
      _descriptionController.text = widget.mapping!.description;
      _selectedPasswordId = widget.mapping!.passwordId;
    }
  }

  @override
  void dispose() {
    _extensionController.dispose();
    _descriptionController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    if (_selectedPasswordId == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('请选择密码')),
      );
      return;
    }

    setState(() {
      _isSaving = true;
    });

    try {
      final service = context.read<MappingService>();

      if (isEditing) {
        final updated = widget.mapping!.copyWith(
          extension: _extensionController.text.trim().toLowerCase(),
          passwordId: _selectedPasswordId,
          description: _descriptionController.text.trim(),
        );
        await service.updateExtensionMapping(updated);
      } else {
        final mapping = ExtensionPasswordMapping(
          extension: _extensionController.text.trim().toLowerCase(),
          passwordId: _selectedPasswordId!,
          description: _descriptionController.text.trim(),
        );
        await service.addExtensionMapping(mapping);
      }

      if (mounted) {
        Navigator.pop(context);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(isEditing ? '映射已更新' : '映射已添加')),
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
      title: Text(isEditing ? '编辑后缀映射' : '添加后缀映射'),
      content: SingleChildScrollView(
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              TextFormField(
                controller: _extensionController,
                decoration: const InputDecoration(
                  labelText: '文件后缀',
                  hintText: '例如：sh、py、doc',
                  prefixText: '.',
                ),
                validator: (value) {
                  if (value == null || value.trim().isEmpty) {
                    return '请输入文件后缀';
                  }
                  return null;
                },
                textInputAction: TextInputAction.next,
              ),
              const SizedBox(height: 16),
              Consumer<PasswordService>(
                builder: (context, service, _) {
                  if (!service.isLoaded) {
                    return const Center(child: CircularProgressIndicator());
                  }

                  final passwords = service.passwords;

                  if (passwords.isEmpty) {
                    return Container(
                      padding: const EdgeInsets.all(16),
                      decoration: BoxDecoration(
                        color: Theme.of(context)
                            .colorScheme
                            .surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Text('请先在密码本中添加密码'),
                    );
                  }

                  return DropdownMenu<String>(
                    width: double.infinity,
                    label: const Text('选择密码'),
                    initialSelection: _selectedPasswordId,
                    onSelected: (id) {
                      setState(() {
                        _selectedPasswordId = id;
                      });
                    },
                    dropdownMenuEntries: passwords
                        .map((p) => DropdownMenuEntry(
                              value: p.id,
                              label: p.name,
                            ))
                        .toList(),
                  );
                },
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _descriptionController,
                decoration: const InputDecoration(
                  labelText: '描述（可选）',
                  hintText: '例如：Shell 脚本文件',
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
