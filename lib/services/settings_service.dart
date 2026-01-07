import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:path_provider/path_provider.dart';

/// 设置服务
/// 管理应用全局设置
class SettingsService extends ChangeNotifier {
  static const String _themeModeKey = 'secure_zip_theme_mode';
  static const String _defaultSchemeKey = 'secure_zip_default_scheme';
  static const String _compressionLevelKey = 'secure_zip_compression_level';
  static const String _outputDirKey = 'secure_zip_output_dir';
  static const String _decompressOutputDirKey =
      'secure_zip_decompress_output_dir';

  /// 默认压缩输出目录
  static const String defaultAndroidCompressDir =
      '/storage/emulated/0/SecureZip/compressed';

  /// 默认解压输出目录
  static const String defaultAndroidDecompressDir =
      '/storage/emulated/0/SecureZip/extracted';

  ThemeMode _themeMode = ThemeMode.system;
  String _defaultObfuscationScheme = 'sequential';
  int _compressionLevel = 6;
  String _outputDir = '';
  String _decompressOutputDir = '';
  bool _isLoaded = false;

  ThemeMode get themeMode => _themeMode;
  String get defaultObfuscationScheme => _defaultObfuscationScheme;
  int get compressionLevel => _compressionLevel;
  String get outputDir => _outputDir;
  String get decompressOutputDir => _decompressOutputDir;
  bool get isLoaded => _isLoaded;

  /// 获取有效的压缩输出目录
  Future<String> getEffectiveOutputDir() async {
    if (_outputDir.isNotEmpty) {
      return _outputDir;
    }
    return await _getDefaultOutputDir();
  }

  /// 获取有效的解压输出目录
  Future<String> getEffectiveDecompressOutputDir() async {
    if (_decompressOutputDir.isNotEmpty) {
      return _decompressOutputDir;
    }
    return await _getDefaultDecompressOutputDir();
  }

  Future<String> _getDefaultOutputDir() async {
    if (Platform.isAndroid) {
      final dir = Directory(defaultAndroidCompressDir);
      if (!await dir.exists()) {
        await dir.create(recursive: true);
      }
      return defaultAndroidCompressDir;
    } else {
      final docDir = await getApplicationDocumentsDirectory();
      final outputDir = Directory('${docDir.path}/SecureZip/compressed');
      if (!await outputDir.exists()) {
        await outputDir.create(recursive: true);
      }
      return outputDir.path;
    }
  }

  Future<String> _getDefaultDecompressOutputDir() async {
    if (Platform.isAndroid) {
      final dir = Directory(defaultAndroidDecompressDir);
      if (!await dir.exists()) {
        await dir.create(recursive: true);
      }
      return defaultAndroidDecompressDir;
    } else {
      final docDir = await getApplicationDocumentsDirectory();
      final outputDir = Directory('${docDir.path}/SecureZip/extracted');
      if (!await outputDir.exists()) {
        await outputDir.create(recursive: true);
      }
      return outputDir.path;
    }
  }

  /// 加载设置
  Future<void> load() async {
    try {
      final prefs = await SharedPreferences.getInstance();

      // 加载主题模式
      final themeModeIndex = prefs.getInt(_themeModeKey);
      if (themeModeIndex != null && themeModeIndex < ThemeMode.values.length) {
        _themeMode = ThemeMode.values[themeModeIndex];
      }

      // 加载默认混淆方案
      _defaultObfuscationScheme =
          prefs.getString(_defaultSchemeKey) ?? 'sequential';

      // 加载压缩级别
      _compressionLevel = prefs.getInt(_compressionLevelKey) ?? 6;

      // 加载输出目录
      _outputDir = prefs.getString(_outputDirKey) ?? '';
      _decompressOutputDir = prefs.getString(_decompressOutputDirKey) ?? '';

      _isLoaded = true;
      notifyListeners();
    } catch (e) {
      debugPrint('加载设置失败: $e');
      _isLoaded = true;
      notifyListeners();
    }
  }

  /// 设置压缩输出目录
  Future<void> setOutputDir(String dir) async {
    _outputDir = dir;
    notifyListeners();

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_outputDirKey, dir);
    } catch (e) {
      debugPrint('保存输出目录设置失败: $e');
    }
  }

  /// 设置解压输出目录
  Future<void> setDecompressOutputDir(String dir) async {
    _decompressOutputDir = dir;
    notifyListeners();

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_decompressOutputDirKey, dir);
    } catch (e) {
      debugPrint('保存解压输出目录设置失败: $e');
    }
  }

  /// 重置输出目录为默认值
  Future<void> resetOutputDirs() async {
    _outputDir = '';
    _decompressOutputDir = '';
    notifyListeners();

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.remove(_outputDirKey);
      await prefs.remove(_decompressOutputDirKey);
    } catch (e) {
      debugPrint('重置输出目录设置失败: $e');
    }
  }

  /// 设置主题模式
  Future<void> setThemeMode(ThemeMode mode) async {
    _themeMode = mode;
    notifyListeners();

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setInt(_themeModeKey, mode.index);
    } catch (e) {
      debugPrint('保存主题设置失败: $e');
    }
  }

  /// 设置默认混淆方案
  Future<void> setDefaultObfuscationScheme(String scheme) async {
    _defaultObfuscationScheme = scheme;
    notifyListeners();

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_defaultSchemeKey, scheme);
    } catch (e) {
      debugPrint('保存混淆方案设置失败: $e');
    }
  }

  /// 设置压缩级别
  Future<void> setCompressionLevel(int level) async {
    _compressionLevel = level.clamp(1, 9);
    notifyListeners();

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setInt(_compressionLevelKey, _compressionLevel);
    } catch (e) {
      debugPrint('保存压缩级别设置失败: $e');
    }
  }

  /// 获取主题模式显示名称
  String getThemeModeDisplayName(ThemeMode mode) {
    switch (mode) {
      case ThemeMode.system:
        return '跟随系统';
      case ThemeMode.light:
        return '浅色模式';
      case ThemeMode.dark:
        return '深色模式';
    }
  }
}
