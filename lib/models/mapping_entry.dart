import 'package:uuid/uuid.dart';

/// 文件名混淆映射条目
class MappingEntry {
  final String id;
  final String originalName;
  final String obfuscatedName;
  final DateTime createdAt;
  final String archivePath;

  MappingEntry({
    String? id,
    required this.originalName,
    required this.obfuscatedName,
    required this.archivePath,
    DateTime? createdAt,
  })  : id = id ?? const Uuid().v4(),
        createdAt = createdAt ?? DateTime.now();

  factory MappingEntry.fromJson(Map<String, dynamic> json) {
    return MappingEntry(
      id: json['id'] as String,
      originalName: json['originalName'] as String,
      obfuscatedName: json['obfuscatedName'] as String,
      archivePath: json['archivePath'] as String,
      createdAt: DateTime.parse(json['createdAt'] as String),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'originalName': originalName,
      'obfuscatedName': obfuscatedName,
      'archivePath': archivePath,
      'createdAt': createdAt.toIso8601String(),
    };
  }
}

/// 文件后缀与密码的映射
class ExtensionPasswordMapping {
  final String id;
  final String extension;
  final String passwordId;
  final String description;
  final DateTime createdAt;

  ExtensionPasswordMapping({
    String? id,
    required this.extension,
    required this.passwordId,
    this.description = '',
    DateTime? createdAt,
  })  : id = id ?? const Uuid().v4(),
        createdAt = createdAt ?? DateTime.now();

  factory ExtensionPasswordMapping.fromJson(Map<String, dynamic> json) {
    return ExtensionPasswordMapping(
      id: json['id'] as String,
      extension: json['extension'] as String,
      passwordId: json['passwordId'] as String,
      description: json['description'] as String? ?? '',
      createdAt: DateTime.parse(json['createdAt'] as String),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'extension': extension,
      'passwordId': passwordId,
      'description': description,
      'createdAt': createdAt.toIso8601String(),
    };
  }

  ExtensionPasswordMapping copyWith({
    String? extension,
    String? passwordId,
    String? description,
  }) {
    return ExtensionPasswordMapping(
      id: id,
      extension: extension ?? this.extension,
      passwordId: passwordId ?? this.passwordId,
      description: description ?? this.description,
      createdAt: createdAt,
    );
  }
}

/// 混淆方案类型
enum ObfuscationScheme {
  /// 序号模式：001.dat, 002.dat
  sequential,

  /// 日期序号模式：20240115_001.dat
  dateSequential,

  /// 随机字符模式：a7x2k9m3.dat
  random,

  /// 哈希模式：8a3c2b1f.dat
  hash,

  /// 加密模式：Base64(AES(原名)).enc
  encrypted,
}

extension ObfuscationSchemeExtension on ObfuscationScheme {
  String get displayName {
    switch (this) {
      case ObfuscationScheme.sequential:
        return '序号模式';
      case ObfuscationScheme.dateSequential:
        return '日期序号模式';
      case ObfuscationScheme.random:
        return '随机字符模式';
      case ObfuscationScheme.hash:
        return '哈希模式';
      case ObfuscationScheme.encrypted:
        return '加密模式';
    }
  }

  String get description {
    switch (this) {
      case ObfuscationScheme.sequential:
        return '简单序号：001.dat, 002.dat';
      case ObfuscationScheme.dateSequential:
        return '包含日期：20240115_001.dat';
      case ObfuscationScheme.random:
        return '随机字符：a7x2k9m3.dat';
      case ObfuscationScheme.hash:
        return 'SHA256前8位：8a3c2b1f.dat';
      case ObfuscationScheme.encrypted:
        return '加密可逆：Base64(AES).enc';
    }
  }
}
