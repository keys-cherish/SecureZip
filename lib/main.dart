import 'dart:io';
import 'package:flutter/material.dart';
import 'package:permission_handler/permission_handler.dart';
import 'app.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // 请求存储权限
  await _requestStoragePermissions();

  runApp(const SecureZipApp());
}

/// 请求存储权限
Future<void> _requestStoragePermissions() async {
  if (!Platform.isAndroid) return;

  // Android 13+ 使用细粒度媒体权限
  if (await Permission.photos.status.isDenied) {
    await Permission.photos.request();
  }
  if (await Permission.videos.status.isDenied) {
    await Permission.videos.request();
  }
  if (await Permission.audio.status.isDenied) {
    await Permission.audio.request();
  }

  // Android 10 及以下使用传统存储权限
  if (await Permission.storage.status.isDenied) {
    await Permission.storage.request();
  }

  // Android 11+ 需要管理外部存储权限
  if (await Permission.manageExternalStorage.status.isDenied) {
    await Permission.manageExternalStorage.request();
  }
}
