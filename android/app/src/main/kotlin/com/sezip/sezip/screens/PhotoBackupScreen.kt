package com.sezip.sezip.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.sezip.sezip.ui.components.PasswordField
import com.sezip.sezip.ui.components.ProgressCard
import com.sezip.sezip.util.FormatUtils
import com.sezip.sezip.viewmodel.BackupTarget
import com.sezip.sezip.viewmodel.PhotoBackupViewModel

private val exifLevels = listOf(
    "保留全部 EXIF",
    "去除 GPS 信息",
    "去除设备信息",
    "全部去除",
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PhotoBackupScreen(
    onNavigateBack: () -> Unit,
    viewModel: PhotoBackupViewModel = viewModel(),
) {
    val directories by viewModel.directories.collectAsState()
    val includeVideos by viewModel.includeVideos.collectAsState()
    val exifStripLevel by viewModel.exifStripLevel.collectAsState()
    val password by viewModel.password.collectAsState()
    val backupTarget by viewModel.backupTarget.collectAsState()
    val scanResult by viewModel.scanResult.collectAsState()
    val syncStats by viewModel.syncStats.collectAsState()
    val isScanning by viewModel.isScanning.collectAsState()
    val isBackingUp by viewModel.isBackingUp.collectAsState()
    val progress by viewModel.progress.collectAsState()
    val error by viewModel.error.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("照片备份") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "返回")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
        ) {
            // 同步统计
            syncStats?.let { stats ->
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("同步概况", style = MaterialTheme.typography.titleMedium)
                        Spacer(Modifier.height(8.dp))
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("已备份", style = MaterialTheme.typography.bodySmall)
                            Text("${stats.totalBackedUp} 个文件", style = MaterialTheme.typography.bodySmall)
                        }
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("总大小", style = MaterialTheme.typography.bodySmall)
                            Text(FormatUtils.formatFileSize(stats.totalBytes), style = MaterialTheme.typography.bodySmall)
                        }
                        if (stats.lastSync != null) {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text("上次同步", style = MaterialTheme.typography.bodySmall)
                                Text(stats.lastSync, style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
                Spacer(Modifier.height(16.dp))
            }

            // 目录选择
            Text("照片目录", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
            Spacer(Modifier.height(8.dp))
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    if (directories.isEmpty()) {
                        Text("未选择目录", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    } else {
                        directories.forEach { dir ->
                            Text(dir, style = MaterialTheme.typography.bodySmall, maxLines = 1)
                        }
                    }
                    Spacer(Modifier.height(8.dp))
                    OutlinedButton(onClick = {
                        // 默认使用 DCIM 和 Pictures
                        viewModel.setDirectories(listOf(
                            "/storage/emulated/0/DCIM",
                            "/storage/emulated/0/Pictures",
                        ))
                    }) {
                        Icon(Icons.Outlined.PhotoLibrary, null, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(6.dp))
                        Text("使用默认目录 (DCIM + Pictures)")
                    }
                }
            }

            Spacer(Modifier.height(16.dp))

            // 选项
            Text("备份选项", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
            Spacer(Modifier.height(8.dp))
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    // 包含视频
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                        Text("包含视频", style = MaterialTheme.typography.bodyMedium)
                        Switch(checked = includeVideos, onCheckedChange = { viewModel.setIncludeVideos(it) })
                    }

                    HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

                    // EXIF 隐私级别
                    Text("EXIF 隐私保护", style = MaterialTheme.typography.bodyMedium)
                    Spacer(Modifier.height(8.dp))
                    SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                        (0..3).forEach { level ->
                            SegmentedButton(
                                selected = exifStripLevel == level,
                                onClick = { viewModel.setExifStripLevel(level) },
                                shape = SegmentedButtonDefaults.itemShape(level, 4),
                            ) { Text("L$level", style = MaterialTheme.typography.labelSmall) }
                        }
                    }
                    Text(
                        exifLevels.getOrElse(exifStripLevel) { "" },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )

                    HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

                    // 备份目标
                    Text("备份目标", style = MaterialTheme.typography.bodyMedium)
                    Spacer(Modifier.height(8.dp))
                    SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                        BackupTarget.entries.forEachIndexed { idx, target ->
                            SegmentedButton(
                                selected = backupTarget == target,
                                onClick = { viewModel.setBackupTarget(target) },
                                shape = SegmentedButtonDefaults.itemShape(idx, BackupTarget.entries.size),
                            ) { Text(target.displayName) }
                        }
                    }

                    HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

                    // 密码
                    PasswordField(
                        value = password, onValueChange = { viewModel.setPassword(it) },
                        modifier = Modifier.fillMaxWidth(), label = "加密密码（可选）",
                    )
                }
            }

            Spacer(Modifier.height(16.dp))

            // 扫描结果
            scanResult?.let { scan ->
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("扫描结果", style = MaterialTheme.typography.titleMedium)
                        Spacer(Modifier.height(8.dp))
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("总文件数")
                            Text("${scan.totalFiles}")
                        }
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("新增文件")
                            Text("${scan.newFiles}")
                        }
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("需要传输")
                            Text(FormatUtils.formatFileSize(scan.transferBytes))
                        }
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("已跳过")
                            Text("${scan.skippedFiles}")
                        }
                    }
                }
                Spacer(Modifier.height(16.dp))
            }

            // 进度
            if (isBackingUp) {
                ProgressCard(
                    title = "备份进度",
                    current = progress.current,
                    total = progress.total,
                    currentFile = progress.currentFile,
                )
                Spacer(Modifier.height(12.dp))
                OutlinedButton(
                    onClick = { viewModel.requestCancel() },
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) { Text("取消") }
                Spacer(Modifier.height(16.dp))
            }

            // 错误
            error?.let { msg ->
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
                ) {
                    Text(msg, modifier = Modifier.padding(16.dp), color = MaterialTheme.colorScheme.onErrorContainer)
                }
                Spacer(Modifier.height(16.dp))
            }

            // 操作按钮
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                OutlinedButton(
                    onClick = { viewModel.scan() },
                    modifier = Modifier.weight(1f),
                    enabled = directories.isNotEmpty() && !isScanning && !isBackingUp,
                ) {
                    if (isScanning) {
                        CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                        Spacer(Modifier.width(8.dp))
                    }
                    Text("扫描")
                }
                Button(
                    onClick = { viewModel.startBackup() },
                    modifier = Modifier.weight(1f),
                    enabled = scanResult != null && (scanResult?.newFiles ?: 0) > 0 && !isBackingUp,
                ) { Text("开始备份") }
            }

            Spacer(Modifier.height(32.dp))
        }
    }
}
