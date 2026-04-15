package com.sezip.sezip.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.InsertDriveFile
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.sezip.sezip.ui.components.PasswordField
import com.sezip.sezip.ui.components.ProgressCard
import com.sezip.sezip.util.FormatUtils
import com.sezip.sezip.viewmodel.DecompressState
import com.sezip.sezip.viewmodel.DecompressViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DecompressScreen(
    onNavigateBack: () -> Unit,
    viewModel: DecompressViewModel = viewModel(),
) {
    val state by viewModel.state.collectAsState()
    val archivePath by viewModel.archivePath.collectAsState()
    val detectedFormat by viewModel.detectedFormat.collectAsState()
    val requiresPassword by viewModel.requiresPassword.collectAsState()
    val password by viewModel.password.collectAsState()
    val outputDir by viewModel.outputDir.collectAsState()
    val contents by viewModel.contents.collectAsState()
    val progress by viewModel.progress.collectAsState()
    val context = LocalContext.current

    val filePickerLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri ->
        uri?.path?.let { viewModel.setArchivePath(it) }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("解压文件") },
                navigationIcon = {
                    IconButton(onClick = {
                        if (state is DecompressState.Running) viewModel.requestCancel()
                        onNavigateBack()
                    }) {
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
            when (state) {
                is DecompressState.SelectFile -> {
                    // 空状态 + 文件选择
                    Spacer(Modifier.height(80.dp))
                    Column(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Icon(
                            Icons.Outlined.Unarchive,
                            null,
                            modifier = Modifier.size(80.dp),
                            tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f),
                        )
                        Spacer(Modifier.height(16.dp))
                        Text(
                            "选择要解压的文件",
                            style = MaterialTheme.typography.headlineSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.height(8.dp))
                        Text(
                            "支持 .zbak / .7z / .sz7z 格式",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
                        )
                        Spacer(Modifier.height(32.dp))
                        Button(onClick = {
                            filePickerLauncher.launch(arrayOf("*/*"))
                        }) {
                            Icon(Icons.Outlined.FileOpen, null)
                            Spacer(Modifier.width(8.dp))
                            Text("选择文件")
                        }
                    }
                }

                is DecompressState.Ready -> {
                    // 文件信息 + 格式检测
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(16.dp)) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text("已选文件", style = MaterialTheme.typography.labelMedium)
                                    Text(
                                        archivePath.substringAfterLast("/"),
                                        style = MaterialTheme.typography.bodyLarge,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                                if (detectedFormat.isNotBlank()) {
                                    FilterChip(
                                        selected = true,
                                        onClick = {},
                                        label = { Text(detectedFormat.uppercase()) },
                                    )
                                }
                            }
                        }
                    }

                    // 内容预览
                    if (contents.isNotEmpty()) {
                        Spacer(Modifier.height(12.dp))
                        Text("文件列表", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                        Spacer(Modifier.height(8.dp))
                        Card(modifier = Modifier.fillMaxWidth().heightIn(max = 200.dp)) {
                            Column(modifier = Modifier.padding(8.dp).verticalScroll(rememberScrollState())) {
                                contents.take(10).forEach { file ->
                                    Row(
                                        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp, horizontal = 8.dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Icon(Icons.AutoMirrored.Outlined.InsertDriveFile, null, modifier = Modifier.size(16.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                                        Spacer(Modifier.width(8.dp))
                                        Text(file, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                    }
                                }
                                if (contents.size > 10) {
                                    Text(
                                        "... 还有 ${contents.size - 10} 个文件",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        modifier = Modifier.padding(8.dp),
                                    )
                                }
                            }
                        }
                    }

                    // 密码
                    if (requiresPassword) {
                        Spacer(Modifier.height(16.dp))
                        Text("密码", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                        Spacer(Modifier.height(8.dp))
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.padding(16.dp)) {
                                PasswordField(
                                    value = password,
                                    onValueChange = { viewModel.setPassword(it) },
                                    modifier = Modifier.fillMaxWidth(),
                                    label = "输入密码",
                                )
                            }
                        }
                    }

                    // 输出目录
                    Spacer(Modifier.height(16.dp))
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(16.dp)) {
                            Text("输出目录", style = MaterialTheme.typography.labelMedium)
                            Spacer(Modifier.height(4.dp))
                            Text(outputDir, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }

                    // 操作按钮
                    Spacer(Modifier.height(24.dp))
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        OutlinedButton(
                            onClick = { viewModel.reset() },
                            modifier = Modifier.weight(1f),
                        ) { Text("重新选择") }
                        Button(
                            onClick = { viewModel.startDecompress() },
                            modifier = Modifier.weight(1f),
                            enabled = !requiresPassword || password.isNotBlank(),
                        ) {
                            Icon(Icons.Outlined.Unarchive, null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("开始解压")
                        }
                    }
                }

                is DecompressState.Running -> {
                    Spacer(Modifier.height(40.dp))
                    Text(
                        "${progress.percentage}%",
                        style = MaterialTheme.typography.headlineLarge.copy(fontSize = 56.sp),
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.fillMaxWidth(),
                        textAlign = TextAlign.Center,
                    )
                    Spacer(Modifier.height(24.dp))
                    ProgressCard(
                        title = "解压进度",
                        current = progress.current,
                        total = progress.total,
                        currentFile = progress.currentFile,
                    )
                    Spacer(Modifier.height(24.dp))
                    OutlinedButton(
                        onClick = { viewModel.requestCancel() },
                        modifier = Modifier.fillMaxWidth(),
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                    ) {
                        Icon(Icons.Outlined.Cancel, null)
                        Spacer(Modifier.width(8.dp))
                        Text("取消")
                    }
                }

                is DecompressState.Completed -> {
                    val completed = state as DecompressState.Completed
                    Spacer(Modifier.height(60.dp))
                    Column(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Icon(
                            Icons.Outlined.CheckCircle,
                            null,
                            modifier = Modifier.size(72.dp),
                            tint = MaterialTheme.colorScheme.primary,
                        )
                        Spacer(Modifier.height(16.dp))
                        Text("解压完成", style = MaterialTheme.typography.headlineMedium)
                        Spacer(Modifier.height(8.dp))
                        Text("已提取 ${completed.fileCount} 个文件", style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Spacer(Modifier.height(24.dp))
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(16.dp)) {
                            Text("输出目录", style = MaterialTheme.typography.labelMedium)
                            Spacer(Modifier.height(4.dp))
                            Text(completed.outputDir, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Spacer(Modifier.height(8.dp))
                            TextButton(onClick = {
                                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                clipboard.setPrimaryClip(ClipData.newPlainText("path", completed.outputDir))
                            }) {
                                Icon(Icons.Outlined.ContentCopy, null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("复制路径")
                            }
                        }
                    }
                    Spacer(Modifier.height(24.dp))
                    Button(
                        onClick = { viewModel.reset() },
                        modifier = Modifier.fillMaxWidth(),
                    ) { Text("继续解压") }
                    Spacer(Modifier.height(12.dp))
                    OutlinedButton(
                        onClick = onNavigateBack,
                        modifier = Modifier.fillMaxWidth(),
                    ) { Text("返回") }
                }

                is DecompressState.Error -> {
                    val error = state as DecompressState.Error
                    Spacer(Modifier.height(80.dp))
                    Column(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Icon(
                            Icons.Outlined.Error,
                            null,
                            modifier = Modifier.size(64.dp),
                            tint = MaterialTheme.colorScheme.error,
                        )
                        Spacer(Modifier.height(16.dp))
                        Text("解压失败", style = MaterialTheme.typography.headlineSmall)
                        Spacer(Modifier.height(8.dp))
                        Text(error.message, style = MaterialTheme.typography.bodyMedium, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Spacer(Modifier.height(24.dp))
                        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            OutlinedButton(onClick = { viewModel.reset() }) { Text("重新选择") }
                            Button(onClick = onNavigateBack) { Text("返回") }
                        }
                    }
                }
            }

            Spacer(Modifier.height(24.dp))
        }
    }
}
