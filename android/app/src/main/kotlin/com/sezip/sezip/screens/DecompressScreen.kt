package com.sezip.sezip.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.sezip.sezip.ui.components.PasswordField
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
        uri?.let {
            val realPath = com.sezip.sezip.util.FileUtils.uriToRealPath(it)
            if (realPath != null) viewModel.setArchivePath(realPath)
        }
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
            when (val currentState = state) {
                is DecompressState.SelectFile -> SelectFileContent(
                    onPickFile = { filePickerLauncher.launch(arrayOf("*/*")) },
                )

                is DecompressState.Ready -> ReadyContent(
                    archivePath = archivePath,
                    detectedFormat = detectedFormat,
                    requiresPassword = requiresPassword,
                    password = password,
                    outputDir = outputDir,
                    contents = contents,
                    onPasswordChange = { viewModel.setPassword(it) },
                    onReset = { viewModel.reset() },
                    onStartDecompress = { viewModel.startDecompress() },
                )

                is DecompressState.Running -> RunningContent(
                    progress = progress,
                    onCancel = { viewModel.requestCancel() },
                )

                is DecompressState.Completed -> CompletedContent(
                    fileCount = currentState.fileCount,
                    outputDir = currentState.outputDir,
                    context = context,
                    onContinue = { viewModel.reset() },
                    onBack = onNavigateBack,
                )

                is DecompressState.Error -> ErrorContent(
                    message = currentState.message,
                    onReset = { viewModel.reset() },
                    onBack = onNavigateBack,
                )
            }
            Spacer(Modifier.height(24.dp))
        }
    }
}

@Composable
private fun SelectFileContent(onPickFile: () -> Unit) {
    Spacer(Modifier.height(80.dp))
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
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
        Button(onClick = onPickFile) {
            Text("选择文件")
        }
    }
}

@Composable
private fun ReadyContent(
    archivePath: String,
    detectedFormat: String,
    requiresPassword: Boolean,
    password: String,
    outputDir: String,
    contents: List<String>,
    onPasswordChange: (String) -> Unit,
    onReset: () -> Unit,
    onStartDecompress: () -> Unit,
) {
    // 文件信息
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("已选文件", style = MaterialTheme.typography.labelMedium)
            Text(
                archivePath.substringAfterLast("/"),
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (detectedFormat.isNotBlank()) {
                Spacer(Modifier.height(8.dp))
                FilterChip(
                    selected = true,
                    onClick = {},
                    label = { Text(detectedFormat.uppercase()) },
                )
            }
        }
    }

    // 内容预览
    if (contents.isNotEmpty()) {
        Spacer(Modifier.height(12.dp))
        Text("文件列表", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.height(8.dp))
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(12.dp)) {
                contents.take(10).forEach { file ->
                    Text(
                        file,
                        style = MaterialTheme.typography.bodySmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(vertical = 2.dp),
                    )
                }
                if (contents.size > 10) {
                    Text(
                        "... 还有 ${contents.size - 10} 个文件",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }

    // 密码
    if (requiresPassword) {
        Spacer(Modifier.height(16.dp))
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(16.dp)) {
                PasswordField(
                    value = password,
                    onValueChange = onPasswordChange,
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

    // 按钮
    Spacer(Modifier.height(24.dp))
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        OutlinedButton(onClick = onReset, modifier = Modifier.weight(1f)) {
            Text("重新选择")
        }
        Button(
            onClick = onStartDecompress,
            modifier = Modifier.weight(1f),
            enabled = !requiresPassword || password.isNotBlank(),
        ) {
            Text("开始解压")
        }
    }
}

@Composable
private fun RunningContent(
    progress: com.sezip.sezip.model.CompressProgress,
    onCancel: () -> Unit,
) {
    Spacer(Modifier.height(40.dp))
    Text(
        "${progress.percentage}%",
        style = MaterialTheme.typography.headlineLarge,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.fillMaxWidth(),
        textAlign = TextAlign.Center,
    )
    Spacer(Modifier.height(16.dp))
    LinearProgressIndicator(
        progress = { progress.fraction },
        modifier = Modifier.fillMaxWidth(),
    )
    Spacer(Modifier.height(8.dp))
    if (!progress.currentFile.isNullOrBlank()) {
        Text(progress.currentFile!!, style = MaterialTheme.typography.bodySmall, maxLines = 1)
    }
    Spacer(Modifier.height(8.dp))
    Text(
        "${FormatUtils.formatFileSize(progress.current)} / ${FormatUtils.formatFileSize(progress.total)}",
        style = MaterialTheme.typography.bodySmall,
    )
    Spacer(Modifier.height(24.dp))
    OutlinedButton(
        onClick = onCancel,
        modifier = Modifier.fillMaxWidth(),
        colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
    ) {
        Icon(Icons.Filled.Close, null, modifier = Modifier.size(18.dp))
        Spacer(Modifier.width(6.dp))
        Text("取消")
    }
}

@Composable
private fun CompletedContent(
    fileCount: Int,
    outputDir: String,
    context: Context,
    onContinue: () -> Unit,
    onBack: () -> Unit,
) {
    Spacer(Modifier.height(60.dp))
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(
            Icons.Filled.CheckCircle, null,
            modifier = Modifier.size(72.dp),
            tint = MaterialTheme.colorScheme.primary,
        )
        Spacer(Modifier.height(16.dp))
        Text("解压完成", style = MaterialTheme.typography.headlineMedium)
        Spacer(Modifier.height(8.dp))
        Text("已提取 $fileCount 个文件", style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    Spacer(Modifier.height(24.dp))
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("输出目录", style = MaterialTheme.typography.labelMedium)
            Spacer(Modifier.height(4.dp))
            Text(outputDir, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
    Spacer(Modifier.height(24.dp))
    Button(onClick = onContinue, modifier = Modifier.fillMaxWidth()) { Text("继续解压") }
    Spacer(Modifier.height(12.dp))
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("返回") }
}

@Composable
private fun ErrorContent(
    message: String,
    onReset: () -> Unit,
    onBack: () -> Unit,
) {
    Spacer(Modifier.height(80.dp))
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(
            Icons.Filled.Warning, null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.error,
        )
        Spacer(Modifier.height(16.dp))
        Text("解压失败", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(8.dp))
        Text(message, style = MaterialTheme.typography.bodyMedium, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.height(24.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedButton(onClick = onReset) { Text("重新选择") }
            Button(onClick = onBack) { Text("返回") }
        }
    }
}
