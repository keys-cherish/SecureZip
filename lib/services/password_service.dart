import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/password_entry.dart';

/// 密码本服务
/// 管理密码条目的增删改查，使用本地加密存储
class PasswordService extends ChangeNotifier {
  static const String _storageKey = 'secure_zip_passwords';

  List<PasswordEntry> _passwords = [];
  bool _isLoaded = false;

  List<PasswordEntry> get passwords => List.unmodifiable(_passwords);
  bool get isLoaded => _isLoaded;

  /// 加载密码数据
  Future<void> load() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final jsonString = prefs.getString(_storageKey);

      if (jsonString != null && jsonString.isNotEmpty) {
        final List<dynamic> jsonList = json.decode(jsonString);
        _passwords = jsonList
            .map((e) => PasswordEntry.fromJson(e as Map<String, dynamic>))
            .toList();
      }
      _isLoaded = true;
      notifyListeners();
    } catch (e) {
      debugPrint('加载密码数据失败: $e');
      _isLoaded = true;
      notifyListeners();
    }
  }

  /// 保存密码数据
  Future<void> _save() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final jsonString =
          json.encode(_passwords.map((e) => e.toJson()).toList());
      await prefs.setString(_storageKey, jsonString);
    } catch (e) {
      debugPrint('保存密码数据失败: $e');
    }
  }

  /// 添加密码
  Future<void> addPassword(PasswordEntry entry) async {
    _passwords.add(entry);
    notifyListeners();
    await _save();
  }

  /// 更新密码
  Future<void> updatePassword(PasswordEntry entry) async {
    final index = _passwords.indexWhere((e) => e.id == entry.id);
    if (index != -1) {
      _passwords[index] = entry;
      notifyListeners();
      await _save();
    }
  }

  /// 删除密码
  Future<void> deletePassword(String id) async {
    _passwords.removeWhere((e) => e.id == id);
    notifyListeners();
    await _save();
  }

  /// 根据ID获取密码
  PasswordEntry? getPasswordById(String id) {
    try {
      return _passwords.firstWhere((e) => e.id == id);
    } catch (e) {
      return null;
    }
  }

  /// 搜索密码
  List<PasswordEntry> search(String query) {
    if (query.isEmpty) return _passwords;
    final lowerQuery = query.toLowerCase();
    return _passwords.where((e) {
      return e.name.toLowerCase().contains(lowerQuery) ||
          (e.remark?.toLowerCase().contains(lowerQuery) ?? false);
    }).toList();
  }

  /// 导出数据（用于备份）- JSON 格式
  String exportData() {
    return json.encode(_passwords.map((e) => e.toJson()).toList());
  }

  /// 导出为 TXT 格式（每行一个密码，格式：名称|密码|备注）
  String exportToTxt() {
    final lines = <String>[];
    lines.add('# SecureZip 密码本导出');
    lines.add('# 格式: 名称|密码|备注');
    lines.add('# 导出时间: ${DateTime.now().toIso8601String()}');
    lines.add('');

    for (final entry in _passwords) {
      final remark = entry.remark?.replaceAll('\n', ' ') ?? '';
      lines.add('${entry.name}|${entry.password}|$remark');
    }

    return lines.join('\n');
  }

  /// 导出为 JSON 格式（格式化便于阅读）
  String exportToJson() {
    final encoder = const JsonEncoder.withIndent('  ');
    final data = {
      'app': 'SecureZip',
      'version': '1.0',
      'exportTime': DateTime.now().toIso8601String(),
      'passwords': _passwords
          .map((e) => {
                'name': e.name,
                'password': e.password,
                'remark': e.remark ?? '',
                'createdAt': e.createdAt.toIso8601String(),
              })
          .toList(),
    };
    return encoder.convert(data);
  }

  /// 导出为 CSV 格式
  String exportToCsv() {
    final lines = <String>[];
    lines.add('名称,密码,备注,创建时间');

    for (final entry in _passwords) {
      final name = _escapeCsv(entry.name);
      final password = _escapeCsv(entry.password);
      final remark = _escapeCsv(entry.remark ?? '');
      final createdAt = entry.createdAt.toIso8601String();
      lines.add('$name,$password,$remark,$createdAt');
    }

    return lines.join('\n');
  }

  String _escapeCsv(String value) {
    if (value.contains(',') || value.contains('"') || value.contains('\n')) {
      return '"${value.replaceAll('"', '""')}"';
    }
    return value;
  }

  /// 导入数据（用于恢复）
  Future<void> importData(String jsonData) async {
    try {
      final List<dynamic> jsonList = json.decode(jsonData);
      _passwords = jsonList
          .map((e) => PasswordEntry.fromJson(e as Map<String, dynamic>))
          .toList();
      notifyListeners();
      await _save();
    } catch (e) {
      throw Exception('导入密码数据失败: $e');
    }
  }

  /// 从 TXT 导入（每行格式：名称|密码|备注）
  Future<int> importFromTxt(String txtData, {bool merge = true}) async {
    final lines = txtData.split('\n');
    final newPasswords = <PasswordEntry>[];

    for (final line in lines) {
      // 跳过空行和注释
      final trimmed = line.trim();
      if (trimmed.isEmpty || trimmed.startsWith('#')) continue;

      final parts = trimmed.split('|');
      if (parts.isEmpty || parts[0].isEmpty) continue;

      final name = parts[0].trim();
      final password = parts.length > 1 ? parts[1].trim() : '';
      final remark =
          parts.length > 2 ? parts.sublist(2).join('|').trim() : null;

      if (name.isNotEmpty && password.isNotEmpty) {
        newPasswords.add(PasswordEntry(
          name: name,
          password: password,
          remark: remark?.isEmpty == true ? null : remark,
        ));
      }
    }

    if (newPasswords.isEmpty) {
      throw Exception('没有找到有效的密码条目');
    }

    if (merge) {
      // 合并模式：添加不重复的
      for (final entry in newPasswords) {
        final exists = _passwords
            .any((e) => e.name == entry.name && e.password == entry.password);
        if (!exists) {
          _passwords.add(entry);
        }
      }
    } else {
      // 覆盖模式
      _passwords = newPasswords;
    }

    notifyListeners();
    await _save();
    return newPasswords.length;
  }

  /// 从 JSON 导入（支持多种格式）
  Future<int> importFromJson(String jsonData, {bool merge = true}) async {
    final dynamic parsed = json.decode(jsonData);
    final newPasswords = <PasswordEntry>[];

    // 支持格式 1: 数组格式 [{name, password, ...}]
    if (parsed is List) {
      for (final item in parsed) {
        if (item is Map<String, dynamic>) {
          final entry = _parsePasswordFromMap(item);
          if (entry != null) newPasswords.add(entry);
        }
      }
    }
    // 支持格式 2: 对象格式 {passwords: [...]}
    else if (parsed is Map<String, dynamic>) {
      final passwords = parsed['passwords'];
      if (passwords is List) {
        for (final item in passwords) {
          if (item is Map<String, dynamic>) {
            final entry = _parsePasswordFromMap(item);
            if (entry != null) newPasswords.add(entry);
          }
        }
      }
      // 支持格式 3: KeePass/Bitwarden 等常见格式
      final items = parsed['items'] ?? parsed['entries'];
      if (items is List) {
        for (final item in items) {
          if (item is Map<String, dynamic>) {
            final entry = _parsePasswordFromMap(item);
            if (entry != null) newPasswords.add(entry);
          }
        }
      }
    }

    if (newPasswords.isEmpty) {
      throw Exception('没有找到有效的密码条目');
    }

    if (merge) {
      for (final entry in newPasswords) {
        final exists = _passwords
            .any((e) => e.name == entry.name && e.password == entry.password);
        if (!exists) {
          _passwords.add(entry);
        }
      }
    } else {
      _passwords = newPasswords;
    }

    notifyListeners();
    await _save();
    return newPasswords.length;
  }

  /// 从 CSV 导入
  Future<int> importFromCsv(String csvData, {bool merge = true}) async {
    final lines = csvData.split('\n');
    final newPasswords = <PasswordEntry>[];

    bool isHeader = true;
    for (final line in lines) {
      final trimmed = line.trim();
      if (trimmed.isEmpty) continue;

      // 跳过表头
      if (isHeader) {
        isHeader = false;
        if (trimmed.toLowerCase().contains('name') || trimmed.contains('名称')) {
          continue;
        }
      }

      final parts = _parseCsvLine(trimmed);
      if (parts.length >= 2) {
        final name = parts[0].trim();
        final password = parts[1].trim();
        final remark = parts.length > 2 ? parts[2].trim() : null;

        if (name.isNotEmpty && password.isNotEmpty) {
          newPasswords.add(PasswordEntry(
            name: name,
            password: password,
            remark: remark?.isEmpty == true ? null : remark,
          ));
        }
      }
    }

    if (newPasswords.isEmpty) {
      throw Exception('没有找到有效的密码条目');
    }

    if (merge) {
      for (final entry in newPasswords) {
        final exists = _passwords
            .any((e) => e.name == entry.name && e.password == entry.password);
        if (!exists) {
          _passwords.add(entry);
        }
      }
    } else {
      _passwords = newPasswords;
    }

    notifyListeners();
    await _save();
    return newPasswords.length;
  }

  List<String> _parseCsvLine(String line) {
    final result = <String>[];
    var current = StringBuffer();
    var inQuotes = false;

    for (var i = 0; i < line.length; i++) {
      final char = line[i];

      if (char == '"') {
        if (inQuotes && i + 1 < line.length && line[i + 1] == '"') {
          current.write('"');
          i++;
        } else {
          inQuotes = !inQuotes;
        }
      } else if (char == ',' && !inQuotes) {
        result.add(current.toString());
        current = StringBuffer();
      } else {
        current.write(char);
      }
    }
    result.add(current.toString());

    return result;
  }

  PasswordEntry? _parsePasswordFromMap(Map<String, dynamic> map) {
    // 支持多种字段名
    final name =
        map['name'] ?? map['title'] ?? map['username'] ?? map['account'];
    final password = map['password'] ?? map['pwd'] ?? map['pass'];
    final remark = map['remark'] ?? map['note'] ?? map['notes'] ?? map['memo'];

    if (name != null &&
        password != null &&
        name.toString().isNotEmpty &&
        password.toString().isNotEmpty) {
      return PasswordEntry(
        name: name.toString(),
        password: password.toString(),
        remark: remark?.toString(),
      );
    }
    return null;
  }
}
