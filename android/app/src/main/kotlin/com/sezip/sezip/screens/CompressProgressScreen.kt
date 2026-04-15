package com.sezip.sezip.screens

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Cancel
import androidx.compose.material.icons.outlined.Error
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.sezip.sezip.ui.components.ProgressCard
import com.sezip.sezip.viewmodel.CompressProgressViewModel
import com.sezip.sezip.viewmodel.OperationState

/**
 * 压缩进度页
 *
 * 显示大百分比数字 + 动画进度条 + 当前文件信息，
 * 支持取消操作。压缩完成后自动跳转至结果页。
 * 错误状态就地展示错误信息和返回按钮。
 */
@Composable
fun CompressProgressScreen(
    onNavigateToResult: () -> Unit,
    onNavigateBack: () -> Unit,
    viewModel: CompressProgressViewModel = viewModel(),
) {
    val state by viewModel.state.collectAsState()
    val progress by viewModel.progress.collectAsState()

    // 进入页面时自动启动压缩
    LaunchedEffect(Unit) {
        val task = com.sezip.sezip.model.PendingCompressTask
        if (task.consume()) {
            viewModel.startCompress(task.inputPaths, task.outputDir, task.outputName, task.options)
            task.clear()
        }
    }

    // 压缩完成后自动导航到结果页
    LaunchedEffect(state) {
        if (state is OperationState.Completed) {
            onNavigateToResult()
        }
    }

    Scaffold { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp),
            contentAlignment = Alignment.Center,
        ) {
            when (val currentState = state) {
                is OperationState.Idle,
                is OperationState.Running -> {
                    RunningContent(
                        percentage = progress.percentage,
                        current = progress.current,
                        total = progress.total,
                        currentFile = progress.currentFile,
                        onCancel = {
                            viewModel.requestCancel()
                            onNavigateBack()
                        },
                    )
                }

                is OperationState.Error -> {
                    ErrorContent(
                        message = currentState.message,
                        onBack = onNavigateBack,
                    )
                }

                is OperationState.Completed -> {
                    // LaunchedEffect 已触发跳转，短暂显示加载指示器作为过渡
                    CircularProgressIndicator()
                }
            }
        }
    }
}

// ── 私有子组件 ──────────────────────────────────────────────────────────

/** 压缩进行中：大百分比 + 进度卡片 + 取消按钮 */
@Composable
private fun RunningContent(
    percentage: Int,
    current: Long,
    total: Long,
    currentFile: String?,
    onCancel: () -> Unit,
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxWidth(),
    ) {
        // 醒目的百分比数字
        Text(
            text = "${percentage}%",
            style = MaterialTheme.typography.headlineLarge.copy(fontSize = 64.sp),
            color = MaterialTheme.colorScheme.primary,
        )

        Spacer(Modifier.height(8.dp))

        Text(
            "压缩中...",
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(Modifier.height(32.dp))

        // 复用公共进度卡片组件
        ProgressCard(
            title = "压缩进度",
            current = current,
            total = total,
            currentFile = currentFile,
        )

        Spacer(Modifier.height(32.dp))

        // 取消按钮，用 error 色调提示破坏性操作
        OutlinedButton(
            onClick = onCancel,
            colors = ButtonDefaults.outlinedButtonColors(
                contentColor = MaterialTheme.colorScheme.error,
            ),
        ) {
            Icon(Icons.Outlined.Cancel, contentDescription = null)
            Spacer(Modifier.width(8.dp))
            Text("取消")
        }
    }
}

/** 压缩失败：错误图标 + 消息 + 返回按钮 */
@Composable
private fun ErrorContent(
    message: String,
    onBack: () -> Unit,
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Icon(
            Icons.Outlined.Error,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.error,
        )

        Spacer(Modifier.height(16.dp))

        Text(
            "压缩失败",
            style = MaterialTheme.typography.headlineSmall,
        )

        Spacer(Modifier.height(8.dp))

        Text(
            message,
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(Modifier.height(24.dp))

        Button(onClick = onBack) {
            Text("返回")
        }
    }
}
