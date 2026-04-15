package com.sezip.sezip.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Compress
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.automirrored.outlined.InsertDriveFile
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.sezip.sezip.RustBridge
import com.sezip.sezip.model.CompressMode
import com.sezip.sezip.model.ObfuscationType
import com.sezip.sezip.model.RecoveryRatio
import com.sezip.sezip.model.SplitSizePreset
import com.sezip.sezip.ui.components.PasswordField
import com.sezip.sezip.util.FormatUtils
import com.sezip.sezip.viewmodel.CompressViewModel

/**
 * 压缩配置页
 *
 * 五个逻辑分区：文件选择 → 压缩模式 → 输出设置 → 安全设置 → 高级选项。
 * 所有选项通过 [CompressViewModel] 集中管理，点击"开始压缩"后
 * 将 selectedPaths + CompressOptions 传递给进度页。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CompressScreen(
    onNavigateToProgress: () -> Unit,
    onNavigateBack: () -> Unit,
    viewModel: CompressViewModel = viewModel(),
) {
    val selectedPaths by viewModel.selectedPaths.collectAsState()
    val fileCount by viewModel.fileCount.collectAsState()
    val totalSize by viewModel.totalSize.collectAsState()
    val outputName by viewModel.outputName.collectAsState()
    val compressMode by viewModel.compressMode.collectAsState()
    val password by viewModel.password.collectAsState()
    val compressionLevel by viewModel.compressionLevel.collectAsState()
    val encryptFilenames by viewModel.encryptFilenames.collectAsState()
    val enableRecovery by viewModel.enableRecovery.collectAsState()
    val recoveryRatio by viewModel.recoveryRatio.collectAsState()
    val splitSize by viewModel.splitSize.collectAsState()
    val enableObfuscation by viewModel.enableObfuscation.collectAsState()
    val obfuscationType by viewModel.obfuscationType.collectAsState()

    // SAF 文件选择器
    val filePickerLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenMultipleDocuments()
    ) { uris ->
        if (uris.isNotEmpty()) viewModel.setSelectedUris(uris)
    }

    // SAF 文件夹选择器
    val folderPickerLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocumentTree()
    ) { uri ->
        uri?.let { viewModel.setSelectedFolderUri(it) }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("压缩文件") },
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
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
        ) {
            // ── Section 1: 文件选择 ─────────────────────────────────────
            SectionHeader("文件选择")
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        OutlinedButton(
                            onClick = { filePickerLauncher.launch(arrayOf("*/*")) },
                            modifier = Modifier.weight(1f),
                        ) {
                            Icon(Icons.AutoMirrored.Outlined.InsertDriveFile, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("选择文件")
                        }
                        OutlinedButton(
                            onClick = { folderPickerLauncher.launch(null) },
                            modifier = Modifier.weight(1f),
                        ) {
                            Icon(Icons.Outlined.Folder, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("选择文件夹")
                        }
                    }
                    if (selectedPaths.isNotEmpty()) {
                        Spacer(Modifier.height(12.dp))
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(
                                "$fileCount 个文件",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Text(
                                FormatUtils.formatFileSize(totalSize),
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            Spacer(Modifier.height(12.dp))

            // ── Section 2: 压缩模式 ─────────────────────────────────────
            SectionHeader("压缩模式")
            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                CompressMode.entries.forEachIndexed { index, mode ->
                    SegmentedButton(
                        selected = compressMode == mode,
                        onClick = { viewModel.setCompressMode(mode) },
                        shape = SegmentedButtonDefaults.itemShape(
                            index = index,
                            count = CompressMode.entries.size,
                        ),
                    ) {
                        Text(mode.displayName)
                    }
                }
            }

            Spacer(Modifier.height(12.dp))

            // ── Section 3: 输出设置 ─────────────────────────────────────
            SectionHeader("输出设置")
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    OutlinedTextField(
                        value = outputName,
                        onValueChange = { viewModel.setOutputName(it) },
                        label = { Text("输出文件名") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    Spacer(Modifier.height(8.dp))
                    Text(
                        "输出目录: ${viewModel.getOutputDir()}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            Spacer(Modifier.height(12.dp))

            // ── Section 4: 安全设置 ─────────────────────────────────────
            SectionHeader("安全设置")
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    PasswordField(
                        value = password,
                        onValueChange = { viewModel.setPassword(it) },
                        modifier = Modifier.fillMaxWidth(),
                        label = "加密密码（可选）",
                        showGenerateButton = true,
                        onGenerate = {
                            val generated = try {
                                RustBridge.generateRandomPassword(16, true)
                            } catch (_: Exception) { "" }
                            if (generated.isNotBlank()) viewModel.setPassword(generated)
                        },
                    )

                    // 密码非空时才显示强度指示和文件名加密
                    if (password.isNotBlank()) {
                        Spacer(Modifier.height(12.dp))
                        PasswordStrengthBar(password)

                        Spacer(Modifier.height(12.dp))
                        SwitchRow(
                            label = "加密文件名",
                            subtitle = "隐藏压缩包内的文件名和目录结构",
                            checked = encryptFilenames,
                            onCheckedChange = { viewModel.setEncryptFilenames(it) },
                        )
                    }
                }
            }

            Spacer(Modifier.height(12.dp))

            // ── Section 5: 高级选项 ─────────────────────────────────────
            SectionHeader("高级选项")
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    // 压缩级别滑块
                    Text(
                        "压缩级别: $compressionLevel",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                    Slider(
                        value = compressionLevel.toFloat(),
                        onValueChange = { viewModel.setCompressionLevel(it.toInt()) },
                        valueRange = 1f..22f,
                        steps = 20,
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("快速", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text("高压缩", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }

                    // zbak 专有选项 (恢复记录 / 分卷 / 文件名混淆)
                    if (compressMode != CompressMode.LEGACY_7Z) {
                        HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))

                        // 恢复记录
                        SwitchRow(
                            label = "恢复记录",
                            subtitle = "Reed-Solomon 纠错，防止文件损坏",
                            checked = enableRecovery,
                            onCheckedChange = { viewModel.setEnableRecovery(it) },
                        )
                        if (enableRecovery) {
                            Spacer(Modifier.height(8.dp))
                            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                                RecoveryRatio.entries.forEachIndexed { idx, ratio ->
                                    SegmentedButton(
                                        selected = recoveryRatio == ratio,
                                        onClick = { viewModel.setRecoveryRatio(ratio) },
                                        shape = SegmentedButtonDefaults.itemShape(
                                            index = idx,
                                            count = RecoveryRatio.entries.size,
                                        ),
                                    ) {
                                        Text(ratio.displayName)
                                    }
                                }
                            }
                        }

                        HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))

                        // 分卷大小
                        SplitSizeSelector(
                            current = splitSize,
                            onSelect = { viewModel.setSplitSize(it) },
                        )

                        HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))

                        // 文件名混淆
                        SwitchRow(
                            label = "文件名混淆",
                            subtitle = "重命名输出文件以保护隐私",
                            checked = enableObfuscation,
                            onCheckedChange = { viewModel.setEnableObfuscation(it) },
                        )
                        if (enableObfuscation) {
                            Spacer(Modifier.height(8.dp))
                            ObfuscationTypeSelector(
                                current = obfuscationType,
                                onSelect = { viewModel.setObfuscationType(it) },
                            )
                        }
                    }
                }
            }

            Spacer(Modifier.height(24.dp))

            // ── 开始压缩 ────────────────────────────────────────────────
            Button(
                onClick = onNavigateToProgress,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(52.dp),
                enabled = selectedPaths.isNotEmpty() && outputName.isNotBlank(),
            ) {
                Icon(Icons.Outlined.Compress, contentDescription = null)
                Spacer(Modifier.width(8.dp))
                Text("开始压缩", style = MaterialTheme.typography.titleMedium)
            }

            Spacer(Modifier.height(24.dp))
        }
    }
}

// ── 私有子组件 ──────────────────────────────────────────────────────────

/** 区块标题，统一用 primary 色 labelLarge */
@Composable
private fun SectionHeader(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(bottom = 8.dp),
    )
}

/** 带标签和可选副标题的 Switch 行 */
@Composable
private fun SwitchRow(
    label: String,
    subtitle: String? = null,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.bodyMedium)
            if (subtitle != null) {
                Text(
                    subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Switch(checked = checked, onCheckedChange = onCheckedChange)
    }
}

/**
 * 密码强度指示条
 *
 * 调用 Rust 侧 calculatePasswordStrength 获取 0~4 的强度等级，
 * 映射为进度条颜色和中文标签。
 */
@Composable
private fun PasswordStrengthBar(password: String) {
    val strength = remember(password) {
        try { RustBridge.calculatePasswordStrength(password) } catch (_: Exception) { 0 }
    }

    val labels = listOf("极弱", "弱", "中等", "强", "极强")
    val colors = listOf(
        MaterialTheme.colorScheme.error,
        MaterialTheme.colorScheme.error,
        MaterialTheme.colorScheme.tertiary,
        MaterialTheme.colorScheme.primary,
        MaterialTheme.colorScheme.primary,
    )

    val safeIndex = strength.coerceIn(0, labels.lastIndex)

    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        LinearProgressIndicator(
            progress = { (safeIndex + 1) / labels.size.toFloat() },
            modifier = Modifier
                .weight(1f)
                .height(4.dp),
            color = colors[safeIndex],
        )
        Spacer(Modifier.width(8.dp))
        Text(
            labels[safeIndex],
            style = MaterialTheme.typography.labelSmall,
            color = colors[safeIndex],
        )
    }
}

/** 分卷大小下拉选择器 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SplitSizeSelector(
    current: SplitSizePreset,
    onSelect: (SplitSizePreset) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("分卷大小", style = MaterialTheme.typography.bodyMedium)
        ExposedDropdownMenuBox(
            expanded = expanded,
            onExpandedChange = { expanded = it },
        ) {
            OutlinedTextField(
                value = current.displayName,
                onValueChange = {},
                readOnly = true,
                modifier = Modifier
                    .menuAnchor(MenuAnchorType.PrimaryNotEditable, enabled = true)
                    .width(140.dp),
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
                textStyle = MaterialTheme.typography.bodySmall,
            )
            ExposedDropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false },
            ) {
                SplitSizePreset.entries.forEach { preset ->
                    DropdownMenuItem(
                        text = { Text(preset.displayName) },
                        onClick = {
                            onSelect(preset)
                            expanded = false
                        },
                    )
                }
            }
        }
    }
}

/** 文件名混淆方案下拉选择器，每项显示方案名和示例 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ObfuscationTypeSelector(
    current: ObfuscationType,
    onSelect: (ObfuscationType) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
    ) {
        OutlinedTextField(
            value = current.displayName,
            onValueChange = {},
            readOnly = true,
            label = { Text("混淆方案") },
            modifier = Modifier
                .fillMaxWidth()
                .menuAnchor(MenuAnchorType.PrimaryNotEditable, enabled = true),
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            ObfuscationType.entries.forEach { type ->
                DropdownMenuItem(
                    text = {
                        Column {
                            Text(type.displayName, style = MaterialTheme.typography.bodyMedium)
                            Text(
                                type.description,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    },
                    onClick = {
                        onSelect(type)
                        expanded = false
                    },
                )
            }
        }
    }
}
