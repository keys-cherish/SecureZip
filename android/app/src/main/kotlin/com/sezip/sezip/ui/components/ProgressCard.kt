package com.sezip.sezip.ui.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.sezip.sezip.util.FormatUtils

@Composable
fun ProgressCard(
    title: String,
    current: Long,
    total: Long,
    currentFile: String?,
    speedBytesPerSecond: Long = 0,
    modifier: Modifier = Modifier,
) {
    val fraction = if (total > 0) current.toFloat() / total else 0f
    val animatedProgress by animateFloatAsState(targetValue = fraction, label = "progress")
    val percentage = (fraction * 100).toInt()
    val remaining = if (speedBytesPerSecond > 0 && total > current) {
        (total - current) / speedBytesPerSecond
    } else 0L

    Card(modifier = modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            // 标题 + 百分比
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(title, style = MaterialTheme.typography.titleMedium)
                Text("$percentage%", style = MaterialTheme.typography.headlineMedium)
            }
            Spacer(modifier = Modifier.height(12.dp))

            // 进度条
            LinearProgressIndicator(
                progress = { animatedProgress },
                modifier = Modifier.fillMaxWidth().height(8.dp),
            )
            Spacer(modifier = Modifier.height(12.dp))

            // 当前文件
            if (!currentFile.isNullOrBlank()) {
                Text(
                    text = currentFile,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                )
                Spacer(modifier = Modifier.height(8.dp))
            }

            // 信息行
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    "${FormatUtils.formatFileSize(current)} / ${FormatUtils.formatFileSize(total)}",
                    style = MaterialTheme.typography.bodySmall,
                )
                if (speedBytesPerSecond > 0) {
                    Text(
                        FormatUtils.formatSpeed(speedBytesPerSecond),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
                if (remaining > 0) {
                    Text(
                        "剩余 ${FormatUtils.formatDuration(remaining)}",
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
        }
    }
}
