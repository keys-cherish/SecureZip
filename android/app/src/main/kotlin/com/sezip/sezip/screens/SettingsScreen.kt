package com.sezip.sezip.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Code
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Memory
import androidx.compose.material.icons.outlined.Palette
import androidx.compose.material.icons.outlined.RestartAlt
import androidx.compose.material.icons.outlined.Shuffle
import androidx.compose.material.icons.outlined.Speed
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import com.sezip.sezip.model.ObfuscationType
import com.sezip.sezip.ui.theme.ThemeMode
import com.sezip.sezip.viewmodel.SettingsViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    viewModel: SettingsViewModel,
    onNavigateBack: () -> Unit,
) {
    val themeMode by viewModel.themeMode.collectAsState()
    val compressionLevel by viewModel.compressionLevel.collectAsState()
    val defaultScheme by viewModel.defaultScheme.collectAsState()
    val outputDir by viewModel.outputDir.collectAsState()
    val decompressDir by viewModel.decompressDir.collectAsState()

    var showThemeDialog by remember { mutableStateOf(false) }
    var showLevelDialog by remember { mutableStateOf(false) }
    var showSchemeDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("设置") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState()),
        ) {
            // ---- 外观 ----
            SettingsSection(title = "外观") {
                SettingsTile(
                    icon = Icons.Outlined.Palette,
                    title = "主题模式",
                    subtitle = themeMode.displayName,
                    onClick = { showThemeDialog = true },
                )
            }

            // ---- 存储 ----
            SettingsSection(title = "存储") {
                SettingsTile(
                    icon = Icons.Outlined.FolderOpen,
                    title = "压缩输出目录",
                    subtitle = outputDir,
                    onClick = { /* TODO: SAF folder picker */ },
                )
                SettingsTile(
                    icon = Icons.Outlined.FolderOpen,
                    title = "解压输出目录",
                    subtitle = decompressDir,
                    onClick = { /* TODO: SAF folder picker */ },
                )
                SettingsTile(
                    icon = Icons.Outlined.RestartAlt,
                    title = "重置为默认目录",
                    subtitle = "恢复默认的 SecureZip 目录",
                    onClick = { viewModel.resetOutputDirs() },
                )
            }

            // ---- 压缩 ----
            SettingsSection(title = "压缩") {
                SettingsTile(
                    icon = Icons.Outlined.Speed,
                    title = "默认压缩级别",
                    subtitle = "$compressionLevel (1=最快, 22=最高压缩)",
                    onClick = { showLevelDialog = true },
                )
                SettingsTile(
                    icon = Icons.Outlined.Shuffle,
                    title = "默认混淆方案",
                    subtitle = ObfuscationType.entries
                        .find { it.name.lowercase() == defaultScheme }
                        ?.displayName ?: defaultScheme,
                    onClick = { showSchemeDialog = true },
                )
            }

            // ---- 关于 ----
            SettingsSection(title = "关于") {
                SettingsTile(
                    icon = Icons.Outlined.Info,
                    title = "版本",
                    subtitle = "2.0.0 (Kotlin Native)",
                )
                SettingsTile(
                    icon = Icons.Outlined.Memory,
                    title = "Rust 库版本",
                    subtitle = viewModel.rustVersion,
                )
                SettingsTile(
                    icon = Icons.Outlined.Code,
                    title = "技术栈",
                    subtitle = "Kotlin + Jetpack Compose + Rust (JNI)",
                )
            }

            Spacer(modifier = Modifier.height(32.dp))
        }
    }

    // ---- 对话框 ----

    if (showThemeDialog) {
        ThemeModeDialog(
            currentMode = themeMode,
            onSelect = { mode ->
                viewModel.setThemeMode(mode)
                showThemeDialog = false
            },
            onDismiss = { showThemeDialog = false },
        )
    }

    if (showLevelDialog) {
        CompressionLevelDialog(
            currentLevel = compressionLevel,
            onConfirm = { level ->
                viewModel.setCompressionLevel(level)
                showLevelDialog = false
            },
            onDismiss = { showLevelDialog = false },
        )
    }

    if (showSchemeDialog) {
        ObfuscationSchemeDialog(
            currentScheme = defaultScheme,
            onSelect = { scheme ->
                viewModel.setDefaultScheme(scheme)
                showSchemeDialog = false
            },
            onDismiss = { showSchemeDialog = false },
        )
    }
}

// ---- 对话框组件 ----

/** 主题模式选择对话框 */
@Composable
private fun ThemeModeDialog(
    currentMode: ThemeMode,
    onSelect: (ThemeMode) -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("主题模式") },
        text = {
            Column {
                ThemeMode.entries.forEach { mode ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onSelect(mode) }
                            .padding(vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(
                            selected = currentMode == mode,
                            onClick = { onSelect(mode) },
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(mode.displayName)
                    }
                }
            }
        },
        confirmButton = {},
    )
}

/** 压缩级别 Slider 对话框 */
@Composable
private fun CompressionLevelDialog(
    currentLevel: Int,
    onConfirm: (Int) -> Unit,
    onDismiss: () -> Unit,
) {
    var tempLevel by remember { mutableIntStateOf(currentLevel) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("压缩级别") },
        text = {
            Column {
                Text(
                    text = "级别: $tempLevel",
                    style = MaterialTheme.typography.bodyLarge,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Slider(
                    value = tempLevel.toFloat(),
                    onValueChange = { tempLevel = it.toInt() },
                    valueRange = 1f..22f,
                    steps = 20,
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("1 (最快)", style = MaterialTheme.typography.labelSmall)
                    Text("22 (最高)", style = MaterialTheme.typography.labelSmall)
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(tempLevel) }) { Text("确定") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("取消") }
        },
    )
}

/** 文件名混淆方案选择对话框 */
@Composable
private fun ObfuscationSchemeDialog(
    currentScheme: String,
    onSelect: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("默认混淆方案") },
        text = {
            Column {
                ObfuscationType.entries.forEach { type ->
                    val schemeKey = type.name.lowercase()
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onSelect(schemeKey) }
                            .padding(vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(
                            selected = currentScheme == schemeKey,
                            onClick = { onSelect(schemeKey) },
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Column {
                            Text(
                                text = type.displayName,
                                style = MaterialTheme.typography.bodyLarge,
                            )
                            Text(
                                text = type.description,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        },
        confirmButton = {},
    )
}

// ---- 设置页私有组件 ----

/** 设置分组：标题 + Card 容器 */
@Composable
private fun SettingsSection(
    title: String,
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(modifier = Modifier.padding(top = 16.dp)) {
        Text(
            text = title,
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        )
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp),
        ) {
            Column(content = content)
        }
    }
}

/** 设置项行：图标 + 标题/副标题 + 可选右箭头 */
@Composable
private fun SettingsTile(
    icon: ImageVector,
    title: String,
    subtitle: String,
    onClick: (() -> Unit)? = null,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(24.dp),
        )
        Spacer(modifier = Modifier.width(16.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(text = title, style = MaterialTheme.typography.bodyLarge)
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 2,
            )
        }
        if (onClick != null) {
            Icon(
                imageVector = Icons.Outlined.ChevronRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
