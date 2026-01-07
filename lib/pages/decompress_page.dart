import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:provider/provider.dart';
import '../services/rust_compress_service.dart';
import '../services/password_service.dart';
import '../services/settings_service.dart';
import '../models/compress_options.dart';
import '../widgets/password_field.dart';
import '../widgets/progress_card.dart';

/// 解压页面
class DecompressPage extends StatefulWidget {
  const DecompressPage({super.key});

  @override
  State<DecompressPage> createState() => _DecompressPageState();
}

class _DecompressPageState extends State<DecompressPage> {
  final RustCompressService _compressService = RustCompressService();
  final TextEditingController _passwordController = TextEditingController();

  // 状态变量 - 初始值均为 false 或空
  bool _isDecompressing = false;
  bool _isCheckingPassword = false;
  bool _isLoadingContents = false;
  bool _needsPassword = false;
  bool _usePasswordFromBook = false;
  String? _selectedPasswordId;
  String? _selectedArchivePath;
  String? _outputDir;
  CompressProgress? _currentProgress;
  String? _error;
  bool _isCompleted = false;
  List<String>? _extractedFiles;
  List<String>? _archiveContents;
  String _defaultOutputDir = '';

  @override
  void initState() {
    super.initState();
    _loadDefaultOutputDir();
  }

  Future<void> _loadDefaultOutputDir() async {
    final settings = context.read<SettingsService>();
    final dir = await settings.getEffectiveDecompressOutputDir();
    if (mounted) {
      setState(() {
        _defaultOutputDir = dir;
        _outputDir ??= dir;
      });
    }
  }

  @override
  void dispose() {
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _pickArchive() async {
    try {
      // 支持多种格式：7z, sz7z 等
      final result = await FilePicker.platform.pickFiles(
        type: FileType.any, // 允许所有文件，稍后验证扩展名
      );

      if (result != null &&
          result.files.isNotEmpty &&
          result.files.first.path != null) {
        final archivePath = result.files.first.path!;

        // 验证文件扩展名
        final lowerPath = archivePath.toLowerCase();
        final supportedFormats = [
          '.7z',
          '.sz7z',
          '.szp',
          '.tar.zst',
          '.zst',
          '.zip'
        ];
        final isSupported =
            supportedFormats.any((ext) => lowerPath.endsWith(ext));

        if (!isSupported) {
          _showError('不支持的格式，请选择 .szp, .7z, .sz7z, .tar.zst 或 .zip 格式');
          return;
        }

        setState(() {
          _selectedArchivePath = archivePath;
          _error = null;
          _isCompleted = false;
          _extractedFiles = null;
          _archiveContents = null;
        });

        // 检查是否需要密码，然后加载压缩包内容
        await _checkPassword(archivePath);
        await _loadArchiveContents(archivePath);
      }
    } catch (e) {
      _showError('选择文件失败: $e');
    }
  }

  /// 加载压缩包内容列表
  Future<void> _loadArchiveContents(String archivePath) async {
    setState(() {
      _isLoadingContents = true;
    });

    try {
      final contents = await _compressService.listArchiveContents(archivePath);
      if (mounted) {
        setState(() {
          _archiveContents = contents;
          _isLoadingContents = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoadingContents = false;
        });
        // 加载内容失败不影响解压，可能是加密文件
        debugPrint('加载压缩包内容失败: $e');
      }
    }
  }

  Future<void> _checkPassword(String archivePath) async {
    setState(() {
      _isCheckingPassword = true;
    });

    try {
      final needsPassword =
          await _compressService.requiresPassword(archivePath);
      if (mounted) {
        setState(() {
          _needsPassword = needsPassword;
          _isCheckingPassword = false;
        });

        if (needsPassword) {
          // 尝试自动匹配密码
          _tryAutoMatchPassword();
        }
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isCheckingPassword = false;
        });
        _showError('检查密码失败: $e');
      }
    }
  }

  void _tryAutoMatchPassword() {
    final passwordService = context.read<PasswordService>();
    if (!passwordService.isLoaded || passwordService.passwords.isEmpty) return;

    // 简单策略：提示用户可以从密码本选择
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('此压缩包需要密码，您可以从密码本选择或手动输入'),
        duration: Duration(seconds: 3),
      ),
    );
  }

  Future<void> _pickOutputDir() async {
    try {
      final result = await FilePicker.platform.getDirectoryPath();
      if (result != null) {
        setState(() {
          _outputDir = result;
        });
      }
    } catch (e) {
      _showError('选择目录失败: $e');
    }
  }

  Future<void> _startDecompress() async {
    if (_selectedArchivePath == null) {
      _showError('请先选择要解压的文件');
      return;
    }

    String? password;
    if (_needsPassword) {
      if (_usePasswordFromBook) {
        if (_selectedPasswordId == null) {
          _showError('请选择密码');
          return;
        }
        final passwordEntry = context
            .read<PasswordService>()
            .getPasswordById(_selectedPasswordId!);
        password = passwordEntry?.password;
      } else {
        password = _passwordController.text;
        if (password.isEmpty) {
          _showError('请输入密码');
          return;
        }
      }
    }

    // 使用设置中的输出目录，如果用户选择了自定义目录则使用自定义目录
    final outputDir = _outputDir ?? _defaultOutputDir;

    if (outputDir.isEmpty) {
      _showError('输出目录未设置');
      return;
    }

    setState(() {
      _isDecompressing = true;
      _error = null;
    });

    try {
      await for (final progress in _compressService.decompress(
        archivePath: _selectedArchivePath!,
        outputDir: outputDir,
        password: password,
      )) {
        if (!mounted) return;
        setState(() {
          _currentProgress = progress;
        });
      }

      // 获取解压的文件列表
      final files =
          await _compressService.listArchiveContents(_selectedArchivePath!);

      if (mounted) {
        setState(() {
          _isDecompressing = false;
          _isCompleted = true;
          _extractedFiles = files;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
          _isDecompressing = false;
        });
      }
    }
  }

  void _showError(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message)),
    );
  }

  void _reset() {
    setState(() {
      _selectedArchivePath = null;
      _outputDir = null;
      _currentProgress = null;
      _error = null;
      _isCompleted = false;
      _extractedFiles = null;
      _archiveContents = null;
      _isLoadingContents = false;
      _needsPassword = false;
      _usePasswordFromBook = false;
      _selectedPasswordId = null;
      _passwordController.clear();
    });
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('解压文件'),
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 选择文件和选项
              if (!_isDecompressing && !_isCompleted) ...[
                _buildFileSelectionSection(colorScheme),
                if (_selectedArchivePath != null) ...[
                  const SizedBox(height: 16),
                  _buildOutputSection(colorScheme),
                ],
                if (_needsPassword) ...[
                  const SizedBox(height: 16),
                  _buildPasswordSection(colorScheme),
                ],
                const SizedBox(height: 24),
                _buildActionButton(),
              ],

              // 检查密码中
              if (_isCheckingPassword)
                const Center(
                  child: Column(
                    children: [
                      CircularProgressIndicator(),
                      SizedBox(height: 16),
                      Text('正在检查压缩包...'),
                    ],
                  ),
                ),

              // 解压进度
              if (_isDecompressing && _currentProgress != null)
                ProgressCard(
                  title: '正在解压...',
                  progress: _currentProgress!.progress,
                  processedText:
                      '已处理: ${_currentProgress!.displayProcessed} / ${_currentProgress!.displayTotal}',
                  speedText: '速度: ${_currentProgress!.displaySpeed}',
                  remainingText: '剩余: ${_currentProgress!.displayRemaining}',
                  currentFile: _currentProgress!.currentFile,
                ),

              // 解压完成
              if (_isCompleted) _buildCompletedSection(colorScheme),

              // 错误信息
              if (_error != null)
                Card(
                  color: colorScheme.errorContainer,
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      children: [
                        Icon(Icons.error_outline,
                            color: colorScheme.error, size: 48),
                        const SizedBox(height: 8),
                        Text(
                          '解压失败',
                          style: TextStyle(
                            color: colorScheme.onErrorContainer,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          _error!,
                          style: TextStyle(color: colorScheme.onErrorContainer),
                          textAlign: TextAlign.center,
                        ),
                        const SizedBox(height: 16),
                        FilledButton(
                          onPressed: _reset,
                          child: const Text('重新开始'),
                        ),
                      ],
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildFileSelectionSection(ColorScheme colorScheme) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              '选择压缩包',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: _pickArchive,
              icon: const Icon(Icons.archive_outlined),
              label: const Text('选择压缩文件'),
            ),
            if (_selectedArchivePath != null) ...[
              const SizedBox(height: 16),
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.archive,
                      color: colorScheme.primary,
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        _selectedArchivePath!.split('/').last.split('\\').last,
                        style: Theme.of(context).textTheme.bodyMedium,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.close),
                      onPressed: _reset,
                      tooltip: '清除',
                    ),
                  ],
                ),
              ),
              // 显示压缩包内容预览
              const SizedBox(height: 12),
              _buildArchiveContentsPreview(colorScheme),
            ],
          ],
        ),
      ),
    );
  }

  /// 构建压缩包内容预览
  Widget _buildArchiveContentsPreview(ColorScheme colorScheme) {
    if (_isLoadingContents) {
      return Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHighest.withOpacity(0.5),
          borderRadius: BorderRadius.circular(8),
        ),
        child: const Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
            SizedBox(width: 12),
            Text('正在加载压缩包内容...'),
          ],
        ),
      );
    }

    if (_archiveContents == null || _archiveContents!.isEmpty) {
      // 可能是加密文件，不显示任何内容
      return const SizedBox.shrink();
    }

    final displayCount =
        _archiveContents!.length > 5 ? 5 : _archiveContents!.length;
    final hasMore = _archiveContents!.length > 5;

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withOpacity(0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: colorScheme.outline.withOpacity(0.3)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                Icons.folder_open,
                size: 16,
                color: colorScheme.primary,
              ),
              const SizedBox(width: 8),
              Text(
                '压缩包内容 (${_archiveContents!.length} 个文件)',
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                      color: colorScheme.primary,
                    ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          ...List.generate(displayCount, (index) {
            final fileName = _archiveContents![index];
            return Padding(
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: Row(
                children: [
                  Icon(
                    Icons.insert_drive_file_outlined,
                    size: 14,
                    color: colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      fileName,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
            );
          }),
          if (hasMore)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(
                '... 还有 ${_archiveContents!.length - 5} 个文件',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontStyle: FontStyle.italic,
                    ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildOutputSection(ColorScheme colorScheme) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              '解压位置',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: _pickOutputDir,
              icon: const Icon(Icons.folder_outlined),
              label: Text(_outputDir != null ? '更改目录' : '选择解压目录（可选）'),
            ),
            if (_outputDir != null) ...[
              const SizedBox(height: 8),
              Text(
                _outputDir!,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
              ),
            ] else ...[
              const SizedBox(height: 8),
              Text(
                '默认解压到压缩包所在目录',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildPasswordSection(ColorScheme colorScheme) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Icon(Icons.lock_outline, color: colorScheme.primary),
                const SizedBox(width: 8),
                Text(
                  '需要密码',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ],
            ),
            const SizedBox(height: 16),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('从密码本选择'),
              value: _usePasswordFromBook,
              onChanged: (value) {
                setState(() {
                  _usePasswordFromBook = value;
                  if (!value) {
                    _selectedPasswordId = null;
                  }
                });
              },
            ),
            const SizedBox(height: 8),
            if (_usePasswordFromBook)
              _buildPasswordSelector()
            else
              PasswordField(
                controller: _passwordController,
                labelText: '输入密码',
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildPasswordSelector() {
    return Consumer<PasswordService>(
      builder: (context, passwordService, _) {
        if (!passwordService.isLoaded) {
          passwordService.load();
          return const Center(child: CircularProgressIndicator());
        }

        final passwords = passwordService.passwords;

        if (passwords.isEmpty) {
          return Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: const Text('暂无保存的密码，请先在密码本中添加'),
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
    );
  }

  Widget _buildActionButton() {
    return FilledButton.icon(
      onPressed: _selectedArchivePath == null ? null : _startDecompress,
      icon: const Icon(Icons.unarchive),
      label: const Text('开始解压'),
    );
  }

  Widget _buildCompletedSection(ColorScheme colorScheme) {
    final effectiveOutputDir = _outputDir ?? _defaultOutputDir;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Icon(
              Icons.check_circle_outline,
              color: colorScheme.primary,
              size: 64,
            ),
            const SizedBox(height: 16),
            Text(
              '解压完成',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 16),
            // 显示输出目录
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer.withOpacity(0.3),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.folder_outlined,
                        size: 18,
                        color: colorScheme.primary,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        '文件解压位置:',
                        style:
                            Theme.of(context).textTheme.labelMedium?.copyWith(
                                  color: colorScheme.primary,
                                  fontWeight: FontWeight.bold,
                                ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  SelectableText(
                    effectiveOutputDir,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurface,
                        ),
                  ),
                ],
              ),
            ),
            if (_extractedFiles != null && _extractedFiles!.isNotEmpty) ...[
              const SizedBox(height: 16),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '解压的文件 (${_extractedFiles!.length} 个):',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 8),
                    ...(_extractedFiles!.take(10).map((file) => Padding(
                          padding: const EdgeInsets.only(bottom: 4),
                          child: Row(
                            children: [
                              Icon(
                                Icons.insert_drive_file_outlined,
                                size: 16,
                                color: colorScheme.onSurfaceVariant,
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  file,
                                  style: Theme.of(context).textTheme.bodySmall,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                        ))),
                    if (_extractedFiles!.length > 10)
                      Text(
                        '... 还有 ${_extractedFiles!.length - 10} 个文件',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                      ),
                  ],
                ),
              ),
            ],
            const SizedBox(height: 24),
            Row(
              children: [
                Expanded(
                  child: OutlinedButton(
                    onPressed: _reset,
                    child: const Text('继续解压'),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: FilledButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text('返回首页'),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
