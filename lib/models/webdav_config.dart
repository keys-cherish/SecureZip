/// WebDAV 配置模型
class WebDavConfig {
  final String serverUrl;
  final String username;
  final String password;
  final String remotePath;

  const WebDavConfig({
    required this.serverUrl,
    required this.username,
    required this.password,
    this.remotePath = '/',
  });

  factory WebDavConfig.fromJson(Map<String, dynamic> json) {
    return WebDavConfig(
      serverUrl: json['serverUrl'] as String,
      username: json['username'] as String,
      password: json['password'] as String,
      remotePath: json['remotePath'] as String? ?? '/',
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'serverUrl': serverUrl,
      'username': username,
      'password': password,
      'remotePath': remotePath,
    };
  }

  WebDavConfig copyWith({
    String? serverUrl,
    String? username,
    String? password,
    String? remotePath,
  }) {
    return WebDavConfig(
      serverUrl: serverUrl ?? this.serverUrl,
      username: username ?? this.username,
      password: password ?? this.password,
      remotePath: remotePath ?? this.remotePath,
    );
  }

  bool get isConfigured =>
      serverUrl.isNotEmpty && username.isNotEmpty && password.isNotEmpty;
}

/// WebDAV 远程文件信息
class WebDavFileInfo {
  final String name;
  final String path;
  final bool isDirectory;
  final int size;
  final DateTime? lastModified;

  const WebDavFileInfo({
    required this.name,
    required this.path,
    required this.isDirectory,
    this.size = 0,
    this.lastModified,
  });

  String get displaySize {
    if (isDirectory) return '-';
    if (size < 1024) return '$size B';
    if (size < 1024 * 1024) return '${(size / 1024).toStringAsFixed(1)} KB';
    if (size < 1024 * 1024 * 1024) {
      return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(size / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }
}

/// WebDAV 备份文件信息
class WebDavBackupInfo {
  final String fileName;
  final int size;
  final DateTime? backupDate;

  const WebDavBackupInfo({
    required this.fileName,
    this.size = 0,
    this.backupDate,
  });

  String get displaySize {
    if (size < 1024) return '$size B';
    if (size < 1024 * 1024) return '${(size / 1024).toStringAsFixed(1)} KB';
    if (size < 1024 * 1024 * 1024) {
      return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(size / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  String get displayDate {
    if (backupDate == null) return '未知';
    return '${backupDate!.year}-${backupDate!.month.toString().padLeft(2, '0')}-${backupDate!.day.toString().padLeft(2, '0')} '
        '${backupDate!.hour.toString().padLeft(2, '0')}:${backupDate!.minute.toString().padLeft(2, '0')}';
  }
}
