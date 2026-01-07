import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/webdav_config.dart';
import '../services/webdav_service.dart';

/// WebDAV 文件浏览页面
class WebDavFilesPage extends StatefulWidget {
  const WebDavFilesPage({super.key});

  @override
  State<WebDavFilesPage> createState() => _WebDavFilesPageState();
}

class _WebDavFilesPageState extends State<WebDavFilesPage> {
  bool _isLoading = false;
  String? _error;
  List<WebDavFileInfo> _files = [];
  String _currentPath = '/';
  final List<String> _pathHistory = ['/'];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadFiles();
    });
  }

  Future<void> _loadFiles() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });

    try {
      final files = await context.read<WebDavService>().listFiles(_currentPath);
      if (mounted) {
        setState(() {
          _files = files;
          _isLoading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
          _isLoading = false;
        });
      }
    }
  }

  void _navigateTo(String path) {
    setState(() {
      _currentPath = path;
      _pathHistory.add(path);
    });
    _loadFiles();
  }

  void _navigateUp() {
    if (_pathHistory.length > 1) {
      _pathHistory.removeLast();
      setState(() {
        _currentPath = _pathHistory.last;
      });
      _loadFiles();
    }
  }

  Future<void> _deleteFile(WebDavFileInfo file) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('删除文件'),
        content: Text('确定要删除 "${file.name}" 吗？此操作不可撤销。'),
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

    if (confirmed == true) {
      try {
        await context.read<WebDavService>().deleteFile(file.path);
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('文件已删除')),
          );
          _loadFiles();
        }
      } catch (e) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('删除失败: $e')),
          );
        }
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('远程文件'),
        leading: _pathHistory.length > 1
            ? IconButton(
                icon: const Icon(Icons.arrow_back),
                onPressed: _navigateUp,
              )
            : null,
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _isLoading ? null : _loadFiles,
            tooltip: '刷新',
          ),
        ],
      ),
      body: SafeArea(
        child: Column(
          children: [
            // 当前路径
            Container(
              width: double.infinity,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: colorScheme.surfaceContainerHighest,
              child: Text(
                '当前路径: $_currentPath',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
              ),
            ),

            // 内容区域
            Expanded(
              child: _buildContent(colorScheme),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildContent(ColorScheme colorScheme) {
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_error != null) {
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
              '加载失败',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(
              _error!,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: _loadFiles,
              child: const Text('重试'),
            ),
          ],
        ),
      );
    }

    if (_files.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.folder_open_outlined,
              size: 64,
              color: colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 16),
            Text(
              '文件夹为空',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
            ),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: _loadFiles,
      child: ListView.builder(
        padding: const EdgeInsets.symmetric(vertical: 8),
        itemCount: _files.length,
        itemBuilder: (context, index) {
          final file = _files[index];
          return _FileListTile(
            key: ValueKey(file.path),
            file: file,
            onTap: () {
              if (file.isDirectory) {
                _navigateTo(file.path);
              }
            },
            onDelete: () => _deleteFile(file),
          );
        },
      ),
    );
  }
}

class _FileListTile extends StatelessWidget {
  final WebDavFileInfo file;
  final VoidCallback onTap;
  final VoidCallback onDelete;

  const _FileListTile({
    super.key,
    required this.file,
    required this.onTap,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListTile(
      leading: Icon(
        file.isDirectory ? Icons.folder : Icons.insert_drive_file_outlined,
        color: file.isDirectory
            ? colorScheme.primary
            : colorScheme.onSurfaceVariant,
      ),
      title: Text(
        file.name,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(
        file.displaySize,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
      ),
      trailing: file.isDirectory
          ? const Icon(Icons.chevron_right)
          : PopupMenuButton<String>(
              onSelected: (value) {
                if (value == 'delete') {
                  onDelete();
                }
              },
              itemBuilder: (context) => [
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
      onTap: onTap,
    );
  }
}
