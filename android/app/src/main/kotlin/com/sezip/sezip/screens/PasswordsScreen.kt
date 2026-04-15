package com.sezip.sezip.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.sezip.sezip.model.PasswordEntry
import com.sezip.sezip.ui.components.PasswordField
import com.sezip.sezip.viewmodel.PasswordsViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PasswordsScreen(
    onNavigateBack: () -> Unit,
    viewModel: PasswordsViewModel = viewModel(),
) {
    val passwords by viewModel.passwords.collectAsState()
    var showAddDialog by remember { mutableStateOf(false) }
    var editingEntry by remember { mutableStateOf<PasswordEntry?>(null) }
    val context = LocalContext.current

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("密码本") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "返回")
                    }
                },
            )
        },
        floatingActionButton = {
            FloatingActionButton(onClick = { showAddDialog = true }) {
                Icon(Icons.Outlined.Add, "添加密码")
            }
        },
    ) { padding ->
        if (passwords.isEmpty()) {
            // 空状态
            Box(
                modifier = Modifier.fillMaxSize().padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Outlined.Key, null, modifier = Modifier.size(64.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f))
                    Spacer(Modifier.height(16.dp))
                    Text("暂无保存的密码", style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(Modifier.height(8.dp))
                    Text("点击右下角按钮添加", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f))
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize().padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(passwords, key = { it.id }) { entry ->
                    var expanded by remember { mutableStateOf(false) }
                    Card(
                        modifier = Modifier.fillMaxWidth().animateContentSize(),
                    ) {
                        Column(
                            modifier = Modifier.clickable { expanded = !expanded }.padding(16.dp),
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(entry.name, style = MaterialTheme.typography.titleMedium)
                                    Text(
                                        if (expanded) entry.password else "••••••••",
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                                IconButton(onClick = {
                                    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                    clipboard.setPrimaryClip(ClipData.newPlainText("password", entry.password))
                                }) {
                                    Icon(Icons.Outlined.ContentCopy, "复制密码")
                                }
                            }
                            if (expanded) {
                                if (entry.note.isNotBlank()) {
                                    Spacer(Modifier.height(8.dp))
                                    Text(entry.note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                                Spacer(Modifier.height(8.dp))
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    TextButton(onClick = { editingEntry = entry }) {
                                        Icon(Icons.Outlined.Edit, null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(4.dp))
                                        Text("编辑")
                                    }
                                    TextButton(
                                        onClick = { viewModel.delete(entry.id) },
                                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                    ) {
                                        Icon(Icons.Outlined.Delete, null, modifier = Modifier.size(16.dp))
                                        Spacer(Modifier.width(4.dp))
                                        Text("删除")
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 添加对话框
    if (showAddDialog) {
        PasswordEditDialog(
            title = "添加密码",
            onDismiss = { showAddDialog = false },
            onConfirm = { name, pwd, note ->
                viewModel.add(name, pwd, note)
                showAddDialog = false
            },
            onGenerate = { viewModel.generatePassword() },
        )
    }

    // 编辑对话框
    editingEntry?.let { entry ->
        PasswordEditDialog(
            title = "编辑密码",
            initialName = entry.name,
            initialPassword = entry.password,
            initialNote = entry.note,
            onDismiss = { editingEntry = null },
            onConfirm = { name, pwd, note ->
                viewModel.update(entry.copy(name = name, password = pwd, note = note))
                editingEntry = null
            },
            onGenerate = { viewModel.generatePassword() },
        )
    }
}

@Composable
private fun PasswordEditDialog(
    title: String,
    initialName: String = "",
    initialPassword: String = "",
    initialNote: String = "",
    onDismiss: () -> Unit,
    onConfirm: (name: String, password: String, note: String) -> Unit,
    onGenerate: () -> String,
) {
    var name by remember { mutableStateOf(initialName) }
    var password by remember { mutableStateOf(initialPassword) }
    var note by remember { mutableStateOf(initialNote) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column {
                OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("名称") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
                Spacer(Modifier.height(8.dp))
                PasswordField(
                    value = password, onValueChange = { password = it },
                    modifier = Modifier.fillMaxWidth(), showGenerateButton = true,
                    onGenerate = { password = onGenerate() },
                )
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(value = note, onValueChange = { note = it }, label = { Text("备注（可选）") }, modifier = Modifier.fillMaxWidth(), maxLines = 3)
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(name, password, note) },
                enabled = name.isNotBlank() && password.isNotBlank(),
            ) { Text("保存") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("取消") } },
    )
}
