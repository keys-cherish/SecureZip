package com.sezip.sezip.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.sezip.sezip.model.CompressResult
import com.sezip.sezip.util.FormatUtils
import com.sezip.sezip.viewmodel.CompressProgressViewModel
import com.sezip.sezip.viewmodel.OperationState

/**
 * 压缩结果页
 *
 * 压缩成功后展示原始大小、压缩后大小、压缩率，
 * 以及输出路径（可一键复制）。底部提供"继续压缩"和"返回首页"两个操作。
 *
 * 与 [CompressProgressScreen] 共用同一个 [CompressProgressViewModel]，
 * 通过 state 中的 [OperationState.Completed] 获取压缩结果。
 */
@Composable
fun CompressResultScreen(
    onContinueCompress: () -> Unit,
    onNavigateHome: () -> Unit,
    viewModel: CompressProgressViewModel = viewModel(),
) {
    val state by viewModel.state.collectAsState()
    val result = (state as? OperationState.Completed)?.result
    val context = LocalContext.current

    Scaffold { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Spacer(Modifier.height(48.dp))

            // ── 成功图标 + 标题 ─────────────────────────────────────────
            Icon(
                Icons.Outlined.CheckCircle,
                contentDescription = null,
                modifier = Modifier.size(80.dp),
                tint = MaterialTheme.colorScheme.primary,
            )
            Spacer(Modifier.height(16.dp))
            Text("压缩完成", style = MaterialTheme.typography.headlineMedium)

            Spacer(Modifier.height(32.dp))

            if (result != null) {
                // ── 结果统计卡片 ─────────────────────────────────────
                ResultStatsCard(result)

                Spacer(Modifier.height(16.dp))

                // ── 输出路径卡片 ─────────────────────────────────────
                OutputPathCard(
                    outputPath = result.outputPath,
                    onCopyPath = {
                        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                        clipboard.setPrimaryClip(ClipData.newPlainText("path", result.outputPath))
                    },
                )
            }

            // 弹性空间，将按钮推到底部
            Spacer(Modifier.weight(1f))

            // ── 操作按钮 ────────────────────────────────────────────
            Button(
                onClick = onContinueCompress,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp),
            ) {
                Text("继续压缩")
            }

            Spacer(Modifier.height(12.dp))

            OutlinedButton(
                onClick = onNavigateHome,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp),
            ) {
                Text("返回首页")
            }

            Spacer(Modifier.height(16.dp))
        }
    }
}

// ── 私有子组件 ──────────────────────────────────────────────────────────

/** 压缩结果统计卡片：原始大小 / 压缩大小 / 压缩率 */
@Composable
private fun ResultStatsCard(result: CompressResult) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            ResultRow(
                label = "原始大小",
                value = FormatUtils.formatFileSize(result.originalSize),
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
            ResultRow(
                label = "压缩大小",
                value = FormatUtils.formatFileSize(result.compressedSize),
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
            ResultRow(
                label = "压缩率",
                value = FormatUtils.formatRatio(result.compressionRatio),
            )
        }
    }
}

/** 输出路径卡片：显示路径 + 复制按钮 */
@Composable
private fun OutputPathCard(
    outputPath: String,
    onCopyPath: () -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                "保存位置",
                style = MaterialTheme.typography.labelLarge,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                outputPath,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(8.dp))
            TextButton(onClick = onCopyPath) {
                Icon(
                    Icons.Outlined.ContentCopy,
                    contentDescription = null,
                    modifier = Modifier.size(16.dp),
                )
                Spacer(Modifier.width(4.dp))
                Text("复制路径")
            }
        }
    }
}

/** 统计行：左侧标签 + 右侧数值 */
@Composable
private fun ResultRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            value,
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}
