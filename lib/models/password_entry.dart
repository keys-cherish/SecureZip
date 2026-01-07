import 'package:uuid/uuid.dart';

/// 密码条目模型
class PasswordEntry {
  final String id;
  final String name;
  final String password;
  final DateTime createdAt;
  final String? remark;

  PasswordEntry({
    String? id,
    required this.name,
    required this.password,
    DateTime? createdAt,
    this.remark,
  })  : id = id ?? const Uuid().v4(),
        createdAt = createdAt ?? DateTime.now();

  /// 从JSON创建
  factory PasswordEntry.fromJson(Map<String, dynamic> json) {
    return PasswordEntry(
      id: json['id'] as String,
      name: json['name'] as String,
      password: json['password'] as String,
      createdAt: DateTime.parse(json['createdAt'] as String),
      remark: json['remark'] as String?,
    );
  }

  /// 转换为JSON
  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'password': password,
      'createdAt': createdAt.toIso8601String(),
      'remark': remark,
    };
  }

  /// 创建副本
  PasswordEntry copyWith({
    String? name,
    String? password,
    String? remark,
  }) {
    return PasswordEntry(
      id: id,
      name: name ?? this.name,
      password: password ?? this.password,
      createdAt: createdAt,
      remark: remark ?? this.remark,
    );
  }
}
