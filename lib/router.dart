import 'package:go_router/go_router.dart';
import 'pages/home_page.dart';
import 'pages/compress_page.dart';
import 'pages/decompress_page.dart';
import 'pages/passwords_page.dart';
import 'pages/webdav_page.dart';
import 'pages/mappings_page.dart';
import 'pages/settings_page.dart';
import 'pages/webdav_files_page.dart';

/// 应用路由配置
final appRouter = GoRouter(
  initialLocation: '/',
  routes: [
    GoRoute(
      path: '/',
      builder: (context, state) => const HomePage(),
    ),
    GoRoute(
      path: '/compress',
      builder: (context, state) => const CompressPage(),
    ),
    GoRoute(
      path: '/decompress',
      builder: (context, state) => const DecompressPage(),
    ),
    GoRoute(
      path: '/passwords',
      builder: (context, state) => const PasswordsPage(),
    ),
    GoRoute(
      path: '/webdav',
      builder: (context, state) => const WebDavPage(),
    ),
    GoRoute(
      path: '/webdav/files',
      builder: (context, state) => const WebDavFilesPage(),
    ),
    GoRoute(
      path: '/mappings',
      builder: (context, state) => const MappingsPage(),
    ),
    GoRoute(
      path: '/settings',
      builder: (context, state) => const SettingsPage(),
    ),
  ],
);
