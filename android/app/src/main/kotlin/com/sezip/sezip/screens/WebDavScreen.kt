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
import com.sezip.sezip.model.WebDavConfig
import com.sezip.sezip.ui.components.PasswordField
import com.sezip.sezip.viewmodel.WebDavViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WebDavScreen(
    onNavigateToFiles: () -> Unit,
    onNavigateBack: () -> Unit,
    viewModel: WebDavViewModel = viewModel(),
) {
    val config by viewModel.config.collectAsState()
    val connectionState by viewModel.connectionState.collectAsState()
    val backups by viewModel.backups.collectAsState()

    var url by remember(config) { mutableStateOf(config.serverUrl) }
    var username by remember(config) { mutableStateOf(config.username) }
    var password by remember(config) { mutableStateOf(config.password) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("WebDAV") },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "返回")
                    }
                },
                actions = {
                    IconButton(onClick = onNavigateToFiles) {
                        Icon(Icons.Outlined.Folder, "文件浏览")
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
            // 服务器配置
            Text("服务器配置", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
            Spacer(Modifier.height(8.dp))
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    OutlinedTextField(
                        value = url, onValueChange = { url = it },
                        label = { Text("服务器地址") },
                        placeholder = { Text("https://dav.example.com/remote.php/webdav") },
                        modifier = Modifier.fillMaxWidth(), singleLine = true,
                    )
                    Spacer(Modifier.height(8.dp))
                    OutlinedTextField(
                        value = username, onValueChange = { username = it },
                        label = { Text("用户名") },
                        modifier = Modifier.fillMaxWidth(), singleLine = true,
                    )
                    Spacer(Modifier.height(8.dp))
                    PasswordField(
                        value = password, onValueChange = { password = it },
                        modifier = Modifier.fillMaxWidth(), label = "密码",
                    )
                    Spacer(Modifier.height(16.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        OutlinedButton(
                            onClick = {
                                viewModel.updateConfig(WebDavConfig(url, username, password))
                                viewModel.testConnection()
                            },
                            modifier = Modifier.weight(1f),
                        ) {
                            Icon(Icons.Outlined.Wifi, null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("测试连接")
                        }
                        Button(
                            onClick = { viewModel.updateConfig(WebDavConfig(url, username, password)) },
                            modifier = Modifier.weight(1f),
                        ) {
                            Icon(Icons.Outlined.Save, null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("保存配置")
                        }
                    }

                    // 连接状态
                    Spacer(Modifier.height(12.dp))
                    when (connectionState) {
                        is WebDavViewModel.ConnectionState.Testing -> {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                                Spacer(Modifier.width(8.dp))
                                Text("正在测试...", style = MaterialTheme.typography.bodySmall)
                            }
                        }
                        is WebDavViewModel.ConnectionState.Connected -> {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Outlined.CheckCircle, null, modifier = Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary)
                                Spacer(Modifier.width(8.dp))
                                Text("连接成功", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                            }
                        }
                        is WebDavViewModel.ConnectionState.Failed -> {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Outlined.Error, null, modifier = Modifier.size(16.dp), tint = MaterialTheme.colorScheme.error)
                                Spacer(Modifier.width(8.dp))
                                Text((connectionState as WebDavViewModel.ConnectionState.Failed).message, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                            }
                        }
                        else -> {}
                    }
                }
            }

            Spacer(Modifier.height(24.dp))

            // 备份列表
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("远程备份", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                TextButton(onClick = { viewModel.loadBackups() }) { Text("刷新") }
            }
            Spacer(Modifier.height(8.dp))
            if (backups.isEmpty()) {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Box(modifier = Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                        Text("暂无备份", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            } else {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(8.dp)) {
                        backups.forEach { backup ->
                            ListItem(
                                headlineContent = { Text(backup, maxLines = 1, overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis) },
                                leadingContent = { Icon(Icons.Outlined.CloudDownload, null) },
                            )
                        }
                    }
                }
            }

            Spacer(Modifier.height(24.dp))
        }
    }
}
