import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'theme.dart';
import 'router.dart';
import 'services/password_service.dart';
import 'services/mapping_service.dart';
import 'services/webdav_service.dart';
import 'services/settings_service.dart';
import 'services/rust_compress_service.dart';

/// SecureZip 应用根组件
class SecureZipApp extends StatelessWidget {
  const SecureZipApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => PasswordService()),
        ChangeNotifierProvider(create: (_) => MappingService()),
        ChangeNotifierProvider(create: (_) => WebDavService()),
        ChangeNotifierProvider(create: (_) => SettingsService()),
        Provider(create: (_) => RustCompressService()),
      ],
      child: Consumer<SettingsService>(
        builder: (context, settings, _) {
          return MaterialApp.router(
            title: 'SecureZip',
            debugShowCheckedModeBanner: false,
            theme: AppTheme.lightTheme,
            darkTheme: AppTheme.darkTheme,
            themeMode: settings.themeMode,
            routerConfig: appRouter,
          );
        },
      ),
    );
  }
}
