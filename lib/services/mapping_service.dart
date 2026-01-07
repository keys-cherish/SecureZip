import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/mapping_entry.dart';

/// 映射表服务
/// 管理文件名混淆映射和后缀密码映射
class MappingService extends ChangeNotifier {
  static const String _mappingsKey = 'secure_zip_mappings';
  static const String _extMappingsKey = 'secure_zip_ext_mappings';

  List<MappingEntry> _mappings = [];
  List<ExtensionPasswordMapping> _extensionMappings = [];
  bool _isLoaded = false;

  List<MappingEntry> get mappings => List.unmodifiable(_mappings);
  List<ExtensionPasswordMapping> get extensionMappings =>
      List.unmodifiable(_extensionMappings);
  bool get isLoaded => _isLoaded;

  /// 加载数据
  Future<void> load() async {
    try {
      final prefs = await SharedPreferences.getInstance();

      // 加载文件名映射
      final mappingsJson = prefs.getString(_mappingsKey);
      if (mappingsJson != null && mappingsJson.isNotEmpty) {
        final List<dynamic> jsonList = json.decode(mappingsJson);
        _mappings = jsonList
            .map((e) => MappingEntry.fromJson(e as Map<String, dynamic>))
            .toList();
      }

      // 加载后缀密码映射
      final extMappingsJson = prefs.getString(_extMappingsKey);
      if (extMappingsJson != null && extMappingsJson.isNotEmpty) {
        final List<dynamic> jsonList = json.decode(extMappingsJson);
        _extensionMappings = jsonList
            .map((e) =>
                ExtensionPasswordMapping.fromJson(e as Map<String, dynamic>))
            .toList();
      }

      _isLoaded = true;
      notifyListeners();
    } catch (e) {
      debugPrint('加载映射数据失败: $e');
      _isLoaded = true;
      notifyListeners();
    }
  }

  /// 保存文件名映射
  Future<void> _saveMappings() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final jsonString = json.encode(_mappings.map((e) => e.toJson()).toList());
      await prefs.setString(_mappingsKey, jsonString);
    } catch (e) {
      debugPrint('保存映射数据失败: $e');
    }
  }

  /// 保存后缀密码映射
  Future<void> _saveExtMappings() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final jsonString =
          json.encode(_extensionMappings.map((e) => e.toJson()).toList());
      await prefs.setString(_extMappingsKey, jsonString);
    } catch (e) {
      debugPrint('保存后缀映射数据失败: $e');
    }
  }

  // ========== 文件名映射操作 ==========

  /// 添加文件名映射
  Future<void> addMapping(MappingEntry entry) async {
    _mappings.add(entry);
    notifyListeners();
    await _saveMappings();
  }

  /// 批量添加文件名映射
  Future<void> addMappings(List<MappingEntry> entries) async {
    _mappings.addAll(entries);
    notifyListeners();
    await _saveMappings();
  }

  /// 删除文件名映射
  Future<void> deleteMapping(String id) async {
    _mappings.removeWhere((e) => e.id == id);
    notifyListeners();
    await _saveMappings();
  }

  /// 清空指定压缩包的映射
  Future<void> clearMappingsForArchive(String archivePath) async {
    _mappings.removeWhere((e) => e.archivePath == archivePath);
    notifyListeners();
    await _saveMappings();
  }

  /// 搜索文件名映射
  List<MappingEntry> searchMappings(String query) {
    if (query.isEmpty) return _mappings;
    final lowerQuery = query.toLowerCase();
    return _mappings.where((e) {
      return e.originalName.toLowerCase().contains(lowerQuery) ||
          e.obfuscatedName.toLowerCase().contains(lowerQuery);
    }).toList();
  }

  // ========== 后缀密码映射操作 ==========

  /// 添加后缀密码映射
  Future<void> addExtensionMapping(ExtensionPasswordMapping entry) async {
    // 检查是否已存在相同后缀的映射
    final existingIndex =
        _extensionMappings.indexWhere((e) => e.extension == entry.extension);
    if (existingIndex != -1) {
      _extensionMappings[existingIndex] = entry;
    } else {
      _extensionMappings.add(entry);
    }
    notifyListeners();
    await _saveExtMappings();
  }

  /// 更新后缀密码映射
  Future<void> updateExtensionMapping(ExtensionPasswordMapping entry) async {
    final index = _extensionMappings.indexWhere((e) => e.id == entry.id);
    if (index != -1) {
      _extensionMappings[index] = entry;
      notifyListeners();
      await _saveExtMappings();
    }
  }

  /// 删除后缀密码映射
  Future<void> deleteExtensionMapping(String id) async {
    _extensionMappings.removeWhere((e) => e.id == id);
    notifyListeners();
    await _saveExtMappings();
  }

  /// 根据文件后缀获取对应的密码ID
  String? getPasswordIdForExtension(String filename) {
    final ext = _getFileExtension(filename);
    if (ext.isEmpty) return null;

    try {
      final mapping = _extensionMappings.firstWhere(
        (e) => e.extension.toLowerCase() == ext.toLowerCase(),
      );
      return mapping.passwordId;
    } catch (e) {
      return null;
    }
  }

  /// 获取文件扩展名
  String _getFileExtension(String filename) {
    final dotIndex = filename.lastIndexOf('.');
    if (dotIndex == -1 || dotIndex == filename.length - 1) return '';
    return filename.substring(dotIndex + 1);
  }

  // ========== 映射重复检查 ==========

  /// 检查混淆名是否已存在
  bool isObfuscatedNameExists(String obfuscatedName) {
    return _mappings.any((e) => e.obfuscatedName == obfuscatedName);
  }

  /// 获取下一个可用的序号（避免重复）
  int getNextAvailableCounter(String prefix, String suffix) {
    int counter = 1;
    while (true) {
      final name = '$prefix${counter.toString().padLeft(3, '0')}$suffix';
      if (!isObfuscatedNameExists(name)) {
        return counter;
      }
      counter++;
      // 防止无限循环
      if (counter > 99999) break;
    }
    return counter;
  }

  /// 获取所有已使用的混淆名集合
  Set<String> getUsedObfuscatedNames() {
    return _mappings.map((e) => e.obfuscatedName).toSet();
  }

  // ========== 数据导入导出 ==========

  /// 导出所有映射数据
  String exportData() {
    return json.encode({
      'mappings': _mappings.map((e) => e.toJson()).toList(),
      'extensionMappings': _extensionMappings.map((e) => e.toJson()).toList(),
    });
  }

  /// 导入映射数据
  Future<void> importData(String jsonData) async {
    try {
      final data = json.decode(jsonData) as Map<String, dynamic>;

      if (data.containsKey('mappings')) {
        final List<dynamic> mappingsList = data['mappings'];
        _mappings = mappingsList
            .map((e) => MappingEntry.fromJson(e as Map<String, dynamic>))
            .toList();
      }

      if (data.containsKey('extensionMappings')) {
        final List<dynamic> extList = data['extensionMappings'];
        _extensionMappings = extList
            .map((e) =>
                ExtensionPasswordMapping.fromJson(e as Map<String, dynamic>))
            .toList();
      }

      notifyListeners();
      await _saveMappings();
      await _saveExtMappings();
    } catch (e) {
      throw Exception('导入映射数据失败: $e');
    }
  }
}
