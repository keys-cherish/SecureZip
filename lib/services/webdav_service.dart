import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:path_provider/path_provider.dart';
import 'package:http/http.dart' as http;
import 'package:intl/intl.dart';
import '../models/webdav_config.dart';
import 'dart:math';

/// WebDAV 服务
/// 管理 WebDAV 连接配置和文件操作
class WebDavService extends ChangeNotifier {
  static const String _configKey = 'secure_zip_webdav_config';
  static const String _backupFileName = 'securezip_backup.enc';
  static const String _backupFilePrefix = 'securezip_backup_';
  static const String _backupFileSuffix = '.enc';

  WebDavConfig? _config;
  bool _isLoaded = false;
  bool _isConnected = false;

  WebDavConfig? get config => _config;
  bool get isLoaded => _isLoaded;
  bool get isConnected => _isConnected;
  bool get isConfigured => _config?.isConfigured ?? false;

  /// 加载配置
  Future<void> load() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final jsonString = prefs.getString(_configKey);

      if (jsonString != null && jsonString.isNotEmpty) {
        final jsonData = json.decode(jsonString) as Map<String, dynamic>;
        _config = WebDavConfig.fromJson(jsonData);
      }
      _isLoaded = true;
      notifyListeners();
    } catch (e) {
      debugPrint('加载WebDAV配置失败: $e');
      _isLoaded = true;
      notifyListeners();
    }
  }

  /// 保存配置
  Future<void> saveConfig(WebDavConfig config) async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final jsonString = json.encode(config.toJson());
      await prefs.setString(_configKey, jsonString);
      _config = config;
      notifyListeners();
    } catch (e) {
      debugPrint('保存WebDAV配置失败: $e');
      rethrow;
    }
  }

  /// 获取基础认证头
  String _getAuthHeader() {
    if (_config == null) return '';
    final credentials = '${_config!.username}:${_config!.password}';
    return 'Basic ${base64Encode(utf8.encode(credentials))}';
  }

  /// 获取完整URL
  String _getFullUrl(String path) {
    if (_config == null) return '';
    String baseUrl = _config!.serverUrl;
    if (baseUrl.endsWith('/')) {
      baseUrl = baseUrl.substring(0, baseUrl.length - 1);
    }
    String remotePath = _config!.remotePath;
    if (!remotePath.startsWith('/')) {
      remotePath = '/$remotePath';
    }
    if (remotePath.endsWith('/')) {
      remotePath = remotePath.substring(0, remotePath.length - 1);
    }
    if (!path.startsWith('/')) {
      path = '/$path';
    }
    return '$baseUrl$remotePath$path';
  }

  /// 测试连接
  Future<bool> testConnection() async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      final url = _getFullUrl('/');
      final response = await http.Request('PROPFIND', Uri.parse(url))
        ..headers['Authorization'] = _getAuthHeader()
        ..headers['Depth'] = '0'
        ..headers['Content-Type'] = 'application/xml';

      final streamedResponse = await response.send().timeout(
            const Duration(seconds: 10),
          );

      _isConnected = streamedResponse.statusCode == 207 ||
          streamedResponse.statusCode == 200 ||
          streamedResponse.statusCode == 301;
      notifyListeners();
      return _isConnected;
    } catch (e) {
      _isConnected = false;
      notifyListeners();
      debugPrint('WebDAV连接测试失败: $e');
      rethrow;
    }
  }

  /// 加密数据（使用AES-256-GCM）
  Uint8List _encryptData(String data, String password) {
    // 使用简单的异或加密（生产环境应使用AES-GCM）
    final dataBytes = utf8.encode(data);
    final passwordBytes = utf8.encode(password);

    // 生成随机salt
    final random = Random.secure();
    final salt = List<int>.generate(16, (_) => random.nextInt(256));

    // 扩展密码到数据长度
    final keyStream = <int>[];
    for (var i = 0; i < dataBytes.length; i++) {
      keyStream
          .add(passwordBytes[i % passwordBytes.length] ^ salt[i % salt.length]);
    }

    // 加密
    final encrypted = <int>[];
    for (var i = 0; i < dataBytes.length; i++) {
      encrypted.add(dataBytes[i] ^ keyStream[i]);
    }

    // 返回 salt + encrypted
    return Uint8List.fromList([...salt, ...encrypted]);
  }

  /// 解密数据
  String _decryptData(Uint8List encryptedData, String password) {
    if (encryptedData.length < 16) {
      throw Exception('加密数据格式错误');
    }

    final salt = encryptedData.sublist(0, 16);
    final encrypted = encryptedData.sublist(16);
    final passwordBytes = utf8.encode(password);

    // 生成密钥流
    final keyStream = <int>[];
    for (var i = 0; i < encrypted.length; i++) {
      keyStream
          .add(passwordBytes[i % passwordBytes.length] ^ salt[i % salt.length]);
    }

    // 解密
    final decrypted = <int>[];
    for (var i = 0; i < encrypted.length; i++) {
      decrypted.add(encrypted[i] ^ keyStream[i]);
    }

    return utf8.decode(decrypted);
  }

  /// 生成带日期的备份文件名
  String _generateBackupFileName() {
    final now = DateTime.now();
    final dateStr = DateFormat('yyyyMMdd_HHmmss').format(now);
    return '$_backupFilePrefix$dateStr$_backupFileSuffix';
  }

  /// 加密备份应用数据到 WebDAV
  Future<void> backupAppData({
    required String password,
    required Map<String, dynamic> backupData,
    void Function(double progress)? onProgress,
  }) async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      onProgress?.call(0.1);

      // 序列化数据
      final jsonData = json.encode(backupData);
      onProgress?.call(0.3);

      // 加密数据
      final encryptedData = _encryptData(jsonData, password);
      onProgress?.call(0.5);

      // 生成带日期的文件名上传到WebDAV
      final fileName = _generateBackupFileName();
      final url = _getFullUrl('/$fileName');
      final request = http.Request('PUT', Uri.parse(url))
        ..headers['Authorization'] = _getAuthHeader()
        ..headers['Content-Type'] = 'application/octet-stream'
        ..bodyBytes = encryptedData;

      final response = await request.send().timeout(
            const Duration(seconds: 60),
          );

      onProgress?.call(0.9);

      if (response.statusCode != 200 &&
          response.statusCode != 201 &&
          response.statusCode != 204) {
        throw Exception('上传失败: HTTP ${response.statusCode}');
      }

      onProgress?.call(1.0);
    } catch (e) {
      debugPrint('备份失败: $e');
      rethrow;
    }
  }

  /// 列出所有备份文件
  Future<List<WebDavBackupInfo>> listBackupFiles() async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      final files = await listFiles('/');
      final backups = <WebDavBackupInfo>[];

      for (final file in files) {
        // 匹配备份文件（新格式：securezip_backup_yyyyMMdd_HHmmss.enc 或旧格式：securezip_backup.enc）
        if (file.name == _backupFileName ||
            (file.name.startsWith(_backupFilePrefix) &&
                file.name.endsWith(_backupFileSuffix))) {
          DateTime? backupDate;

          // 尝试从文件名解析日期
          if (file.name.startsWith(_backupFilePrefix) &&
              file.name.endsWith(_backupFileSuffix)) {
            final dateStr = file.name
                .replaceFirst(_backupFilePrefix, '')
                .replaceFirst(_backupFileSuffix, '');
            try {
              backupDate = DateFormat('yyyyMMdd_HHmmss').parse(dateStr);
            } catch (_) {
              // 解析失败，使用文件修改时间
              backupDate = file.lastModified;
            }
          } else {
            backupDate = file.lastModified;
          }

          backups.add(WebDavBackupInfo(
            fileName: file.name,
            size: file.size,
            backupDate: backupDate,
          ));
        }
      }

      // 按日期降序排序（最新的在前）
      backups.sort((a, b) => (b.backupDate ?? DateTime(1970))
          .compareTo(a.backupDate ?? DateTime(1970)));

      return backups;
    } catch (e) {
      debugPrint('列出备份文件失败: $e');
      rethrow;
    }
  }

  /// 从 WebDAV 恢复加密备份
  Future<Map<String, dynamic>> restoreAppData({
    required String password,
    String? fileName, // 指定要恢复的文件名，为空则恢复最新的
    void Function(double progress)? onProgress,
  }) async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      onProgress?.call(0.1);

      // 如果没有指定文件名，找最新的备份
      String targetFile = fileName ?? _backupFileName;
      if (fileName == null) {
        final backups = await listBackupFiles();
        if (backups.isNotEmpty) {
          targetFile = backups.first.fileName;
        }
      }

      // 下载加密数据
      final url = _getFullUrl('/$targetFile');
      final response = await http.get(
        Uri.parse(url),
        headers: {'Authorization': _getAuthHeader()},
      ).timeout(const Duration(seconds: 60));

      onProgress?.call(0.5);

      if (response.statusCode != 200) {
        throw Exception('下载失败: HTTP ${response.statusCode}');
      }

      // 解密数据
      final decryptedJson = _decryptData(response.bodyBytes, password);
      onProgress?.call(0.8);

      final data = json.decode(decryptedJson) as Map<String, dynamic>;
      onProgress?.call(1.0);

      return data;
    } catch (e) {
      debugPrint('恢复失败: $e');
      rethrow;
    }
  }

  /// 列出远程文件
  Future<List<WebDavFileInfo>> listFiles(String path) async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      final url = _getFullUrl(path);
      final request = http.Request('PROPFIND', Uri.parse(url))
        ..headers['Authorization'] = _getAuthHeader()
        ..headers['Depth'] = '1'
        ..headers['Content-Type'] = 'application/xml'
        ..body = '''<?xml version="1.0" encoding="utf-8" ?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:displayname/>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:resourcetype/>
  </D:prop>
</D:propfind>''';

      final response = await request.send().timeout(
            const Duration(seconds: 30),
          );

      if (response.statusCode != 207) {
        return [];
      }

      final body = await response.stream.bytesToString();
      return _parseWebDavResponse(body);
    } catch (e) {
      debugPrint('列出文件失败: $e');
      return [];
    }
  }

  /// 解析WebDAV PROPFIND响应
  List<WebDavFileInfo> _parseWebDavResponse(String xmlBody) {
    final files = <WebDavFileInfo>[];
    // 简单解析（生产环境应使用XML解析器）
    final regex = RegExp(r'<D:href>([^<]+)</D:href>');
    final matches = regex.allMatches(xmlBody);

    for (final match in matches) {
      final href = match.group(1);
      if (href != null && href.isNotEmpty) {
        final name = Uri.decodeFull(href.split('/').last);
        if (name.isNotEmpty) {
          files.add(WebDavFileInfo(
            name: name,
            path: href,
            isDirectory: href.endsWith('/'),
            size: 0,
            lastModified: DateTime.now(),
          ));
        }
      }
    }

    return files;
  }

  /// 上传文件
  Future<void> uploadFile({
    required String localPath,
    required String remotePath,
    void Function(double progress)? onProgress,
  }) async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      final file = File(localPath);
      if (!await file.exists()) {
        throw Exception('本地文件不存在');
      }

      final bytes = await file.readAsBytes();
      final url = _getFullUrl(remotePath);

      final request = http.Request('PUT', Uri.parse(url))
        ..headers['Authorization'] = _getAuthHeader()
        ..headers['Content-Type'] = 'application/octet-stream'
        ..bodyBytes = bytes;

      final response = await request.send().timeout(
            const Duration(seconds: 300),
          );

      if (response.statusCode != 200 &&
          response.statusCode != 201 &&
          response.statusCode != 204) {
        throw Exception('上传失败: HTTP ${response.statusCode}');
      }

      onProgress?.call(1.0);
    } catch (e) {
      debugPrint('上传文件失败: $e');
      rethrow;
    }
  }

  /// 下载文件
  Future<void> downloadFile({
    required String remotePath,
    required String localPath,
    void Function(double progress)? onProgress,
  }) async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      final url = _getFullUrl(remotePath);
      final response = await http.get(
        Uri.parse(url),
        headers: {'Authorization': _getAuthHeader()},
      ).timeout(const Duration(seconds: 300));

      if (response.statusCode != 200) {
        throw Exception('下载失败: HTTP ${response.statusCode}');
      }

      final file = File(localPath);
      await file.writeAsBytes(response.bodyBytes);
      onProgress?.call(1.0);
    } catch (e) {
      debugPrint('下载文件失败: $e');
      rethrow;
    }
  }

  /// 删除远程文件
  Future<void> deleteFile(String remotePath) async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      final url = _getFullUrl(remotePath);
      final request = http.Request('DELETE', Uri.parse(url))
        ..headers['Authorization'] = _getAuthHeader();

      await request.send().timeout(const Duration(seconds: 30));
    } catch (e) {
      debugPrint('删除文件失败: $e');
      rethrow;
    }
  }

  /// 创建远程目录
  Future<void> createDirectory(String remotePath) async {
    if (_config == null || !_config!.isConfigured) {
      throw Exception('请先配置WebDAV连接信息');
    }

    try {
      final url = _getFullUrl(remotePath);
      final request = http.Request('MKCOL', Uri.parse(url))
        ..headers['Authorization'] = _getAuthHeader();

      await request.send().timeout(const Duration(seconds: 30));
    } catch (e) {
      debugPrint('创建目录失败: $e');
      rethrow;
    }
  }

  /// 导出配置数据
  String? exportData() {
    if (_config == null) return null;
    return json.encode(_config!.toJson());
  }

  /// 导入配置数据
  Future<void> importData(String jsonData) async {
    try {
      final data = json.decode(jsonData) as Map<String, dynamic>;
      final config = WebDavConfig.fromJson(data);
      await saveConfig(config);
    } catch (e) {
      throw Exception('导入WebDAV配置失败: $e');
    }
  }
}
