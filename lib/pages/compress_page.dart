import 'dart:io';
import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:provider/provider.dart';
import '../models/mapping_entry.dart';
import '../services/password_service.dart';
import '../services/mapping_service.dart';
import '../services/settings_service.dart';
import '../services/rust_compress_service.dart';
import '../models/compress_options.dart';
import '../widgets/password_field.dart';
import 'compress_progress_page.dart';

/// 压缩页面
class CompressPage extends StatefulWidget {
  const CompressPage({super.key});

  @override
  State<CompressPage> createState() => _CompressPageState();
}

class _CompressPageState extends State<CompressPage> {
  final TextEditingController _passwordController = TextEditingController();
  final TextEditingController _outputNameController = TextEditingController();
  final TextEditingController _customExtensionController =
      TextEditingController();

  // 状态变量
  bool _usePassword = false;
  bool _usePasswordFromBook = false;
  bool _enableObfuscation = false;
  bool _enableFilenameEncryption = false;
  bool _enableSolidCompression = false;
  String? _selectedPasswordId;
  ObfuscationScheme _selectedScheme = ObfuscationScheme.sequential;
  CompressMode _compressMode = CompressMode.standard;
  CompressionAlgorithm _algorithm = CompressionAlgorithm.zstd;

  // 后缀选择
  String _selectedExtension = '.7z'; // 默认 7z 格式（标准通用）
  bool _useCustomExtension = false;

  List<String> _selectedPaths = [];
  String _outputDir = '';
  String _previewOutputPath = '';

  // 文件加载状态
  bool _isLoadingFiles = false;
  int _totalFileCount = 0;
  int _totalFileSize = 0;
  bool _fileInfoLoaded = false;

  // 可用的后缀选项（包含映射表中的后缀）
  List<String> get _availableExtensions {
    final List<String> extensions = ['.7z', '.zip'];
    // 从映射表中获取自定义后缀
    try {
      final mappingService = context.read<MappingService>();
      if (mappingService.isLoaded) {
        for (final mapping in mappingService.extensionMappings) {
          // 映射表存储的是不带点的后缀，需要添加点号前缀
          final extWithDot = mapping.extension.startsWith('.')
              ? mapping.extension
              : '.${mapping.extension}';
          if (!extensions.contains(extWithDot)) {
            extensions.add(extWithDot);
          }
        }
      }
    } catch (e) {
      // 映射服务未初始化时忽略
    }
    return extensions;
  }

  @override
  void initState() {
    super.initState();
    _loadOutputDir();
  }

  Future<void> _loadOutputDir() async {
    final settings = context.read<SettingsService>();
    final dir = await settings.getEffectiveOutputDir();
    if (mounted) {
      setState(() {
        _outputDir = dir;
        _updatePreviewPath();
      });
    }
  }

  void _updatePreviewPath() {
    if (_outputNameController.text.isNotEmpty && _outputDir.isNotEmpty) {
      final ext = _getEffectiveExtension();
      setState(() {
        _previewOutputPath = '$_outputDir/${_outputNameController.text}$ext';
      });
    } else {
      setState(() {
        _previewOutputPath = '';
      });
    }
  }

  String _getEffectiveExtension() {
    if (_compressMode == CompressMode.exclusive) {
      return '.szp'; // 专属模式固定使用 .szp (Tar+Zstd+AES256 格式)
    }
    if (_useCustomExtension && _customExtensionController.text.isNotEmpty) {
      final customExt = _customExtensionController.text;
      return customExt.startsWith('.') ? customExt : '.$customExt';
    }
    return _selectedExtension;
  }

  @override
  void dispose() {
    _passwordController.dispose();
    _outputNameController.dispose();
    _customExtensionController.dispose();
    super.dispose();
  }

  /// 异步加载文件信息（文件数量、总大小）
  Future<void> _loadFileInfo() async {
    if (_selectedPaths.isEmpty) {
      setState(() {
        _totalFileCount = 0;
        _totalFileSize = 0;
        _fileInfoLoaded = false;
      });
      return;
    }

    setState(() {
      _isLoadingFiles = true;
      _fileInfoLoaded = false;
    });

    try {
      int count = 0;
      int size = 0;

      for (final path in _selectedPaths) {
        final type = FileSystemEntity.typeSync(path);
        if (type == FileSystemEntityType.file) {
          count++;
          size += await File(path).length();
        } else if (type == FileSystemEntityType.directory) {
          await for (final entity in Directory(path).list(recursive: true)) {
            if (entity is File) {
              count++;
              size += await entity.length();
            }
          }
        }
      }

      if (mounted) {
        setState(() {
          _totalFileCount = count;
          _totalFileSize = size;
          _fileInfoLoaded = true;
          _isLoadingFiles = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoadingFiles = false;
        });
      }
    }
  }

  /// 格式化文件大小
  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  Future<void> _pickFiles() async {
    try {
      // 显示加载状态
      setState(() {
        _isLoadingFiles = true;
      });

      final result = await FilePicker.platform.pickFiles(
        allowMultiple: true,
        type: FileType.any,
        withData: false, // 不立即加载文件数据，提高性能
        withReadStream: false,
      );

      if (result != null && result.files.isNotEmpty) {
        // 先快速更新UI显示文件名
        setState(() {
          _selectedPaths = result.files
              .where((f) => f.path != null)
              .map((f) => f.path!)
              .toList();
          _isLoadingFiles = false;

          // 自动设置输出文件名
          if (_outputNameController.text.isEmpty && _selectedPaths.isNotEmpty) {
            final firstName =
                _selectedPaths.first.split('/').last.split('\\').last;
            final baseName = firstName.contains('.')
                ? firstName.substring(0, firstName.lastIndexOf('.'))
                : firstName;
            _outputNameController.text = baseName;
            _updatePreviewPath();
          }
        });

        // 异步加载文件详细信息
        _loadFileInfo();

        // 检查是否有后缀密码映射
        _checkExtensionPasswordMapping();
      } else {
        setState(() {
          _isLoadingFiles = false;
        });
      }
    } catch (e) {
      setState(() {
        _isLoadingFiles = false;
      });
      _showError('选择文件失败: $e');
    }
  }

  Future<void> _pickFolder() async {
    try {
      setState(() {
        _isLoadingFiles = true;
      });

      final result = await FilePicker.platform.getDirectoryPath();

      if (result != null) {
        setState(() {
          _selectedPaths = [result];
          _isLoadingFiles = false;

          // 自动设置输出文件名
          if (_outputNameController.text.isEmpty) {
            final folderName = result.split('/').last.split('\\').last;
            _outputNameController.text = folderName;
          }
        });

        // 异步加载文件夹详细信息
        _loadFileInfo();
      } else {
        setState(() {
          _isLoadingFiles = false;
        });
      }
    } catch (e) {
      setState(() {
        _isLoadingFiles = false;
      });
      _showError('选择文件夹失败: $e');
    }
  }

  void _checkExtensionPasswordMapping() {
    if (_selectedPaths.isEmpty) return;

    final mappingService = context.read<MappingService>();
    final passwordService = context.read<PasswordService>();

    // 检查第一个文件的后缀是否有密码映射
    for (final path in _selectedPaths) {
      final fileName = path.split('/').last.split('\\').last;
      final passwordId = mappingService.getPasswordIdForExtension(fileName);

      if (passwordId != null) {
        final password = passwordService.getPasswordById(passwordId);
        if (password != null) {
          setState(() {
            _usePassword = true;
            _usePasswordFromBook = true;
            _selectedPasswordId = passwordId;
          });

          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('已自动匹配密码: ${password.name}'),
              duration: const Duration(seconds: 2),
            ),
          );
          return;
        }
      }
    }
  }

  Future<void> _startCompress() async {
    if (_selectedPaths.isEmpty) {
      _showError('请先选择要压缩的文件或文件夹');
      return;
    }

    if (_outputNameController.text.trim().isEmpty) {
      _showError('请输入输出文件名');
      return;
    }

    if (_outputDir.isEmpty) {
      _showError('输出目录未设置');
      return;
    }

    String? password;
    if (_usePassword) {
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

    // 构建输出路径和选项
    final ext = _getEffectiveExtension();
    String outputFileName = _outputNameController.text;
    String originalOutputName = outputFileName; // 保存原始名称用于映射

    // 如果启用混淆，则输出文件名使用混淆名称（压缩包外部名称混淆）
    if (_enableObfuscation) {
      final obfuscationType = ObfuscationType.values[_selectedScheme.index];
      final compressService = context.read<RustCompressService>();
      final mappingService = context.read<MappingService>();

      // 获取已使用的混淆名，避免重复
      final usedNames = mappingService.getUsedObfuscatedNames();

      outputFileName = compressService.generateObfuscatedName(
        outputFileName,
        obfuscationType,
        1,
        usedNames: usedNames,
      );
      // 去掉 .dat 或 .enc 后缀，只保留混淆的基本名称
      if (outputFileName.endsWith('.dat')) {
        outputFileName = outputFileName.substring(0, outputFileName.length - 4);
      } else if (outputFileName.endsWith('.enc')) {
        outputFileName = outputFileName.substring(0, outputFileName.length - 4);
      }

      // 设置外部文件名映射（混淆后的压缩包名 -> 原始输入文件名）
      final originalNames = _selectedPaths
          .map((p) => p.split('/').last.split('\\').last)
          .toList();
      compressService.setExternalNameMapping(
        obfuscatedArchiveName: '$outputFileName$ext',
        originalInputNames: originalNames,
        archivePath: '$_outputDir/$outputFileName$ext',
      );
    }

    final outputPath = '$_outputDir/$outputFileName$ext';

    final options = CompressOptions(
      password: password,
      enableObfuscation: _enableObfuscation,
      obfuscationType: ObfuscationType.values[_selectedScheme.index],
      compressMode: _compressMode,
      fileExtension: ext,
      solidCompression: _enableSolidCompression,
      encryptFilenames: _enableFilenameEncryption,
      algorithm: _algorithm,
    );

    // 跳转到进度页面（不使用灰色遮罩）
    if (mounted) {
      Navigator.of(context).push(
        MaterialPageRoute(
          builder: (context) => CompressProgressPage(
            inputPaths: _selectedPaths,
            outputPath: outputPath,
            options: options,
          ),
        ),
      );
    }
  }

  void _showError(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message)),
    );
  }

  void _reset() {
    setState(() {
      _selectedPaths = [];
      _outputNameController.clear();
      _passwordController.clear();
      _customExtensionController.clear();
      _selectedExtension = '.7z';
      _useCustomExtension = false;
      _usePassword = false;
      _usePasswordFromBook = false;
      _selectedPasswordId = null;
      _enableObfuscation = false;
      _enableFilenameEncryption = false;
      _enableSolidCompression = false;
      _compressMode = CompressMode.standard;
      _algorithm = CompressionAlgorithm.zstd;
      _previewOutputPath = '';
    });
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('压缩文件'),
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _buildFileSelectionSection(colorScheme),
              const SizedBox(height: 16),
              _buildOptionsSection(colorScheme),
              const SizedBox(height: 24),
              _buildActionButton(),
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
              '选择要压缩的内容',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: _pickFiles,
                    icon: const Icon(Icons.insert_drive_file_outlined),
                    label: const Text('选择文件'),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: _pickFolder,
                    icon: const Icon(Icons.folder_outlined),
                    label: const Text('选择文件夹'),
                  ),
                ),
              ],
            ),
            if (_isLoadingFiles) ...[
              const SizedBox(height: 16),
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                    const SizedBox(width: 12),
                    Text(
                      '正在加载文件信息...',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
            ] else if (_selectedPaths.isNotEmpty) ...[
              const SizedBox(height: 16),
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // 显示文件统计信息
                    Row(
                      children: [
                        Icon(
                          Icons.folder_zip_outlined,
                          size: 20,
                          color: colorScheme.primary,
                        ),
                        const SizedBox(width: 8),
                        Text(
                          '已选择 ${_selectedPaths.length} 个项目',
                          style:
                              Theme.of(context).textTheme.titleSmall?.copyWith(
                                    fontWeight: FontWeight.bold,
                                  ),
                        ),
                        if (_fileInfoLoaded) ...[
                          const SizedBox(width: 8),
                          Text(
                            '($_totalFileCount 个文件, ${_formatBytes(_totalFileSize)})',
                            style:
                                Theme.of(context).textTheme.bodySmall?.copyWith(
                                      color: colorScheme.onSurfaceVariant,
                                    ),
                          ),
                        ],
                      ],
                    ),
                    const SizedBox(height: 8),
                    ...(_selectedPaths.take(5).map((path) => Padding(
                          padding: const EdgeInsets.only(bottom: 4),
                          child: Row(
                            children: [
                              Icon(
                                FileSystemEntity.isDirectorySync(path)
                                    ? Icons.folder_outlined
                                    : Icons.insert_drive_file_outlined,
                                size: 16,
                                color: colorScheme.onSurfaceVariant,
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  path.split('/').last.split('\\').last,
                                  style: Theme.of(context).textTheme.bodySmall,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                        ))),
                    if (_selectedPaths.length > 5)
                      Text(
                        '... 还有 ${_selectedPaths.length - 5} 个项目',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                      ),
                  ],
                ),
              ),
            ],
            const SizedBox(height: 16),

            // 输出文件名
            TextField(
              controller: _outputNameController,
              decoration: const InputDecoration(
                labelText: '输出文件名',
                hintText: '输入压缩包名称',
              ),
              onChanged: (value) => _updatePreviewPath(),
            ),

            const SizedBox(height: 16),

            // 后缀选择器
            _buildExtensionSelector(colorScheme),

            if (_previewOutputPath.isNotEmpty) ...[
              const SizedBox(height: 12),
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: colorScheme.primaryContainer.withOpacity(0.3),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: colorScheme.primary.withOpacity(0.3),
                  ),
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.save_outlined,
                      size: 18,
                      color: colorScheme.primary,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            '输出路径预览:',
                            style: Theme.of(context)
                                .textTheme
                                .labelSmall
                                ?.copyWith(
                                  color: colorScheme.primary,
                                ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            _previewOutputPath,
                            style:
                                Theme.of(context).textTheme.bodySmall?.copyWith(
                                      color: colorScheme.onSurface,
                                    ),
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildOptionsSection(ColorScheme colorScheme) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              '压缩选项',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 16),

            // 压缩模式选择
            Text(
              '压缩模式',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 8),
            SegmentedButton<CompressMode>(
              segments: const [
                ButtonSegment(
                  value: CompressMode.standard,
                  label: Text('标准模式'),
                  icon: Icon(Icons.folder_zip_outlined),
                ),
                ButtonSegment(
                  value: CompressMode.exclusive,
                  label: Text('专属模式'),
                  icon: Icon(Icons.lock_outlined),
                ),
              ],
              selected: {_compressMode},
              onSelectionChanged: (Set<CompressMode> selected) {
                setState(() {
                  _compressMode = selected.first;
                });
              },
            ),
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: _compressMode == CompressMode.standard
                    ? colorScheme.primaryContainer.withOpacity(0.3)
                    : colorScheme.tertiaryContainer.withOpacity(0.3),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.info_outline,
                    size: 18,
                    color: _compressMode == CompressMode.standard
                        ? colorScheme.primary
                        : colorScheme.tertiary,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _compressMode == CompressMode.standard
                          ? 'Tar+Zstd格式，高效压缩，可用标准工具解压'
                          : '专属.szp格式（Tar+Zstd+AES-256加密），仅能被SecureZip打开',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: _compressMode == CompressMode.standard
                                ? colorScheme.primary
                                : colorScheme.tertiary,
                          ),
                    ),
                  ),
                ],
              ),
            ),

            const Divider(height: 32),

            // 压缩算法选择
            Text(
              '压缩算法',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 8),
            Card(
              child: Column(
                children: [
                  RadioListTile<CompressionAlgorithm>(
                    title: const Text('ZSTD'),
                    subtitle: const Text('Zstandard算法，压缩大文件速度快、压缩率高'),
                    value: CompressionAlgorithm.zstd,
                    groupValue: _algorithm,
                    onChanged: (value) {
                      setState(() {
                        _algorithm = value!;
                      });
                    },
                  ),
                ],
              ),
            ),

            const Divider(height: 32),

            // 密码选项
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('加密压缩'),
              subtitle: const Text('使用密码保护压缩包内容'),
              value: _usePassword,
              onChanged: (value) {
                setState(() {
                  _usePassword = value;
                  if (!value) {
                    _usePasswordFromBook = false;
                    _selectedPasswordId = null;
                    _passwordController.clear();
                  }
                });
              },
            ),

            if (_usePassword) ...[
              const SizedBox(height: 8),
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

              // 文件名加密（仅在密码保护时可用）
              const SizedBox(height: 8),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('文件名加密'),
                subtitle: const Text('加密压缩包内的文件名（需要密码才能查看文件列表）'),
                value: _enableFilenameEncryption,
                onChanged: (value) {
                  setState(() {
                    _enableFilenameEncryption = value;
                  });
                },
              ),
            ],

            const Divider(height: 32),

            // 高级压缩选项标题
            Text(
              '高级选项',
              style: Theme.of(context).textTheme.titleSmall?.copyWith(
                    color: colorScheme.primary,
                  ),
            ),
            const SizedBox(height: 12),

            // 固实压缩
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('固实压缩'),
              subtitle: const Text('将多个文件作为整体压缩，提高压缩率'),
              value: _enableSolidCompression,
              onChanged: (value) {
                setState(() {
                  _enableSolidCompression = value;
                });
              },
            ),

            const Divider(height: 32),

            // 文件名混淆选项 - 与加密区分开
            Text(
              '文件名保护',
              style: Theme.of(context).textTheme.titleSmall?.copyWith(
                    color: colorScheme.tertiary,
                  ),
            ),
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.tertiaryContainer.withOpacity(0.3),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.info_outline,
                          size: 16, color: colorScheme.tertiary),
                      const SizedBox(width: 8),
                      Text(
                        '加密 vs 混淆的区别：',
                        style:
                            Theme.of(context).textTheme.labelMedium?.copyWith(
                                  color: colorScheme.tertiary,
                                  fontWeight: FontWeight.bold,
                                ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    '• 文件名加密：需要密码，安全性高，无密码无法查看文件列表\n'
                    '• 文件名混淆：不需要密码，替换为随机名称，可查看列表但无法识别',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colorScheme.onTertiaryContainer,
                        ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 12),

            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('文件名混淆'),
              subtitle: const Text('将真实文件名替换为随机字符（不需要密码）'),
              value: _enableObfuscation,
              onChanged: (value) {
                setState(() {
                  _enableObfuscation = value;
                });
              },
            ),

            if (_enableObfuscation) ...[
              const SizedBox(height: 8),
              DropdownMenu<ObfuscationScheme>(
                width: double.infinity,
                label: const Text('混淆方案'),
                initialSelection: _selectedScheme,
                onSelected: (scheme) {
                  if (scheme != null) {
                    setState(() {
                      _selectedScheme = scheme;
                    });
                  }
                },
                dropdownMenuEntries: ObfuscationScheme.values
                    .map((scheme) => DropdownMenuEntry(
                          value: scheme,
                          label: scheme.displayName,
                          trailingIcon: Tooltip(
                            message: scheme.description,
                            child: const Icon(Icons.info_outline, size: 16),
                          ),
                        ))
                    .toList(),
              ),
            ],
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

  Widget _buildExtensionSelector(ColorScheme colorScheme) {
    // 专属模式时禁用后缀选择
    if (_compressMode == CompressMode.exclusive) {
      return Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: colorScheme.tertiaryContainer.withOpacity(0.3),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          children: [
            Icon(Icons.lock_outline, size: 18, color: colorScheme.tertiary),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                '专属模式固定使用 .sz7z 后缀',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.tertiary,
                    ),
              ),
            ),
          ],
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '输出格式',
          style: Theme.of(context).textTheme.titleSmall,
        ),
        const SizedBox(height: 8),

        // 预设后缀选择
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            ..._availableExtensions.map((ext) => ChoiceChip(
                  label: Text(ext),
                  selected: !_useCustomExtension && _selectedExtension == ext,
                  onSelected: (selected) {
                    if (selected) {
                      setState(() {
                        _useCustomExtension = false;
                        _selectedExtension = ext;
                        _updatePreviewPath();
                        // 检查是否有对应的密码映射
                        _checkExtensionMapping(ext);
                      });
                    }
                  },
                )),
            ChoiceChip(
              label: const Text('自定义'),
              selected: _useCustomExtension,
              onSelected: (selected) {
                setState(() {
                  _useCustomExtension = selected;
                  _updatePreviewPath();
                });
              },
            ),
          ],
        ),

        // 自定义后缀输入
        if (_useCustomExtension) ...[
          const SizedBox(height: 12),
          TextField(
            controller: _customExtensionController,
            decoration: const InputDecoration(
              labelText: '自定义后缀',
              hintText: '例如: .archive',
              prefixText: '.',
            ),
            onChanged: (value) => _updatePreviewPath(),
          ),
        ],
      ],
    );
  }

  Widget _buildActionButton() {
    final isReady =
        _selectedPaths.isNotEmpty && _outputNameController.text.isNotEmpty;

    return FilledButton.icon(
      onPressed: isReady ? _startCompress : null,
      icon: const Icon(Icons.compress),
      label: const Text('开始压缩'),
      style: FilledButton.styleFrom(
        minimumSize: const Size.fromHeight(48),
      ),
    );
  }

  void _checkExtensionMapping(String extension) {
    final mappingService = context.read<MappingService>();
    final passwordService = context.read<PasswordService>();

    // 查找该后缀的密码映射
    for (final mapping in mappingService.extensionMappings) {
      if (mapping.extension == extension) {
        final password = passwordService.getPasswordById(mapping.passwordId);
        if (password != null) {
          setState(() {
            _usePassword = true;
            _usePasswordFromBook = true;
            _selectedPasswordId = mapping.passwordId;
          });
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('已自动匹配密码: ${password.name}'),
              duration: const Duration(seconds: 2),
            ),
          );
          return;
        }
      }
    }
  }
}
