package com.sezip.sezip.navigation

import androidx.compose.runtime.Composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.sezip.sezip.screens.*
import com.sezip.sezip.viewmodel.SettingsViewModel

/** 路由定义 */
object Routes {
    const val HOME = "home"
    const val COMPRESS = "compress"
    const val COMPRESS_PROGRESS = "compress_progress"
    const val COMPRESS_RESULT = "compress_result"
    const val DECOMPRESS = "decompress"
    const val PASSWORDS = "passwords"
    const val WEBDAV = "webdav"
    const val WEBDAV_FILES = "webdav_files"
    const val MAPPINGS = "mappings"
    const val SETTINGS = "settings"
    const val PHOTO_BACKUP = "photo_backup"
}

@Composable
fun NavGraph(settingsViewModel: SettingsViewModel) {
    val navController = rememberNavController()

    NavHost(navController = navController, startDestination = Routes.HOME) {
        composable(Routes.HOME) {
            HomeScreen(
                onNavigateToCompress = { navController.navigate(Routes.COMPRESS) },
                onNavigateToDecompress = { navController.navigate(Routes.DECOMPRESS) },
                onNavigateToPasswords = { navController.navigate(Routes.PASSWORDS) },
                onNavigateToWebDav = { navController.navigate(Routes.WEBDAV) },
                onNavigateToPhotoBackup = { navController.navigate(Routes.PHOTO_BACKUP) },
                onNavigateToMappings = { navController.navigate(Routes.MAPPINGS) },
                onNavigateToSettings = { navController.navigate(Routes.SETTINGS) },
            )
        }
        composable(Routes.COMPRESS) {
            CompressScreen(
                onNavigateToProgress = { navController.navigate(Routes.COMPRESS_PROGRESS) },
                onNavigateBack = { navController.popBackStack() },
            )
        }
        composable(Routes.COMPRESS_PROGRESS) {
            CompressProgressScreen(
                onNavigateToResult = {
                    navController.navigate(Routes.COMPRESS_RESULT) {
                        popUpTo(Routes.COMPRESS) { inclusive = true }
                    }
                },
                onNavigateBack = { navController.popBackStack() },
            )
        }
        composable(Routes.COMPRESS_RESULT) {
            CompressResultScreen(
                onContinueCompress = {
                    navController.navigate(Routes.COMPRESS) {
                        popUpTo(Routes.HOME)
                    }
                },
                onNavigateHome = {
                    navController.popBackStack(Routes.HOME, inclusive = false)
                },
            )
        }
        composable(Routes.DECOMPRESS) {
            DecompressScreen(onNavigateBack = { navController.popBackStack() })
        }
        composable(Routes.PASSWORDS) {
            PasswordsScreen(onNavigateBack = { navController.popBackStack() })
        }
        composable(Routes.WEBDAV) {
            WebDavScreen(
                onNavigateToFiles = { navController.navigate(Routes.WEBDAV_FILES) },
                onNavigateBack = { navController.popBackStack() },
            )
        }
        composable(Routes.WEBDAV_FILES) {
            WebDavFilesScreen(onNavigateBack = { navController.popBackStack() })
        }
        composable(Routes.MAPPINGS) {
            MappingsScreen(onNavigateBack = { navController.popBackStack() })
        }
        composable(Routes.SETTINGS) {
            SettingsScreen(
                viewModel = settingsViewModel,
                onNavigateBack = { navController.popBackStack() },
            )
        }
        composable(Routes.PHOTO_BACKUP) {
            PhotoBackupScreen(onNavigateBack = { navController.popBackStack() })
        }
    }
}
