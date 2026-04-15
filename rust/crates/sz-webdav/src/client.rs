//! WebDAV 客户端实现

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use sz_core::{SzError, SzResult, WebDavConfig, WebDavFileInfo};

/// 创建自定义 WebDAV HTTP 方法，避免 unwrap panic
fn webdav_method(name: &[u8]) -> SzResult<reqwest::Method> {
    reqwest::Method::from_bytes(name)
        .map_err(|_| SzError::InvalidArgument(format!(
            "无效的 HTTP 方法: {}", String::from_utf8_lossy(name)
        )))
}

/// WebDAV 客户端
pub struct WebDavClient {
    config: WebDavConfig,
    client: Client,
}

impl WebDavClient {
    /// 创建新的 WebDAV 客户端
    pub fn new(config: WebDavConfig) -> SzResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| SzError::Network(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// 构建 Basic Auth 头
    fn auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.config.username, self.config.password);
        format!("Basic {}", BASE64.encode(credentials))
    }

    /// 构建完整 URL
    fn build_url(&self, path: &str) -> String {
        let base = self.config.server_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// 测试连接
    pub fn test_connection(&self) -> SzResult<bool> {
        let url = self.build_url(&self.config.remote_path);

        let response = self
            .client
            .request(webdav_method(b"PROPFIND")?, &url)
            .header(AUTHORIZATION, self.auth_header())
            .header("Depth", "0")
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        Ok(response.status().is_success() || response.status().as_u16() == 207)
    }

    /// 列出目录内容
    pub fn list_directory(&self, path: &str) -> SzResult<Vec<WebDavFileInfo>> {
        let url = self.build_url(path);

        let response = self
            .client
            .request(webdav_method(b"PROPFIND")?, &url)
            .header(AUTHORIZATION, self.auth_header())
            .header("Depth", "1")
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        if !response.status().is_success() && response.status().as_u16() != 207 {
            return Err(SzError::WebDav(format!(
                "列出目录失败: {}",
                response.status()
            )));
        }

        let body = response.text().map_err(|e| SzError::Network(e.to_string()))?;
        parse_propfind_response(&body, path)
    }

    /// 上传文件
    pub fn upload_file<F>(
        &self,
        local_path: &str,
        remote_path: &str,
        mut progress_callback: F,
    ) -> SzResult<()>
    where
        F: FnMut(f64),
    {
        let path = Path::new(local_path);
        if !path.exists() {
            return Err(SzError::FileNotFound(local_path.to_string()));
        }

        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let url = self.build_url(remote_path);

        let response = self
            .client
            .put(&url)
            .header(AUTHORIZATION, self.auth_header())
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(buffer)
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SzError::WebDav(format!(
                "上传失败: {}",
                response.status()
            )));
        }

        progress_callback(1.0);
        Ok(())
    }

    /// 下载文件
    pub fn download_file<F>(
        &self,
        remote_path: &str,
        local_path: &str,
        mut progress_callback: F,
    ) -> SzResult<()>
    where
        F: FnMut(f64),
    {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, self.auth_header())
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SzError::WebDav(format!(
                "下载失败: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .map_err(|e| SzError::Network(e.to_string()))?;

        let mut file = File::create(local_path)?;
        file.write_all(&bytes)?;

        progress_callback(1.0);
        Ok(())
    }

    /// 删除文件或目录
    pub fn delete(&self, remote_path: &str) -> SzResult<()> {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .delete(&url)
            .header(AUTHORIZATION, self.auth_header())
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SzError::WebDav(format!(
                "删除失败: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// 创建目录
    pub fn create_directory(&self, remote_path: &str) -> SzResult<()> {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .request(webdav_method(b"MKCOL")?, &url)
            .header(AUTHORIZATION, self.auth_header())
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        // 201 Created 或 405 Method Not Allowed (目录已存在)
        if !response.status().is_success() && response.status().as_u16() != 405 {
            return Err(SzError::WebDav(format!(
                "创建目录失败: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// 检查路径是否存在
    pub fn exists(&self, remote_path: &str) -> SzResult<bool> {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .request(webdav_method(b"PROPFIND")?, &url)
            .header(AUTHORIZATION, self.auth_header())
            .header("Depth", "0")
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        Ok(response.status().is_success() || response.status().as_u16() == 207)
    }

    /// 上传数据块（从内存）
    pub fn upload_data(
        &self,
        remote_path: &str,
        data: &[u8],
    ) -> SzResult<()> {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .put(&url)
            .header(AUTHORIZATION, self.auth_header())
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(data.to_vec())
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SzError::WebDav(format!(
                "上传失败: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// 下载数据到内存
    pub fn download_data(&self, remote_path: &str) -> SzResult<Vec<u8>> {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, self.auth_header())
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SzError::WebDav(format!(
                "下载失败: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .map_err(|e| SzError::Network(e.to_string()))?;

        Ok(bytes.to_vec())
    }

    /// HEAD 请求: 返回文件大小 (用于断点续传检查)
    pub fn head(&self, remote_path: &str) -> SzResult<Option<u64>> {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .head(&url)
            .header(AUTHORIZATION, self.auth_header())
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        if response.status().is_success() {
            let size = response
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            Ok(size)
        } else {
            Ok(None) // 文件不存在
        }
    }
}

// ============================================================================
// PROPFIND XML 解析
// ============================================================================

/// 解析 PROPFIND XML 响应为文件列表
fn parse_propfind_response(xml: &str, request_path: &str) -> SzResult<Vec<WebDavFileInfo>> {
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);

    let mut files = Vec::new();

    // 解析状态
    let mut _in_response = false;
    let mut in_href = false;
    let mut in_displayname = false;
    let mut in_getcontentlength = false;
    let mut in_resourcetype = false;
    let mut in_getlastmodified = false;

    let mut current_href = String::new();
    let mut current_name = String::new();
    let mut current_size: u64 = 0;
    let mut current_is_dir = false;
    let mut current_last_modified = String::new();

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match name {
                    "response" => {
                        _in_response = true;
                        current_href.clear();
                        current_name.clear();
                        current_size = 0;
                        current_is_dir = false;
                        current_last_modified.clear();
                    }
                    "href" => in_href = true,
                    "displayname" => in_displayname = true,
                    "getcontentlength" => in_getcontentlength = true,
                    "resourcetype" => in_resourcetype = true,
                    "collection" if in_resourcetype => current_is_dir = true,
                    "getlastmodified" => in_getlastmodified = true,
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match name {
                    "response" => {
                        _in_response = false;
                        // 跳过请求路径本身（第一个 response 通常是目录自己）
                        let href_path = urlencoding_decode(&current_href);
                        let req_normalized = request_path.trim_end_matches('/');
                        let href_normalized = href_path.trim_end_matches('/');
                        if !href_normalized.is_empty() && href_normalized != req_normalized {
                            let display_name = if current_name.is_empty() {
                                // 从 href 提取文件名
                                href_path.trim_end_matches('/')
                                    .rsplit('/')
                                    .next()
                                    .unwrap_or(&href_path)
                                    .to_string()
                            } else {
                                current_name.clone()
                            };

                            files.push(WebDavFileInfo {
                                name: display_name,
                                path: href_path,
                                is_directory: current_is_dir,
                                size: current_size,
                                last_modified: None, // 简化：不解析日期
                            });
                        }
                    }
                    "href" => in_href = false,
                    "displayname" => in_displayname = false,
                    "getcontentlength" => in_getcontentlength = false,
                    "resourcetype" => in_resourcetype = false,
                    "getlastmodified" => in_getlastmodified = false,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.unescape() {
                    if in_href { current_href = text.to_string(); }
                    if in_displayname { current_name = text.to_string(); }
                    if in_getcontentlength {
                        current_size = text.parse().unwrap_or(0);
                    }
                    if in_getlastmodified {
                        current_last_modified = text.to_string();
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(files)
}

/// URL 解码（处理 %XX 编码，正确支持多字节 UTF-8）
fn urlencoding_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut iter = input.bytes();
    while let Some(b) = iter.next() {
        if b == b'%' {
            let h = iter.next().unwrap_or(b'0');
            let l = iter.next().unwrap_or(b'0');
            bytes.push(hex_val(h) * 16 + hex_val(l));
        } else {
            bytes.push(b);
        }
    }
    // 正确处理多字节 UTF-8（如中文路径），避免 as char 逐字节转换错误
    String::from_utf8(bytes).unwrap_or_else(|e| {
        String::from_utf8_lossy(e.as_bytes()).into_owned()
    })
}

fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let config = WebDavConfig {
            server_url: "https://dav.example.com".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            remote_path: "/".to_string(),
        };
        let client = WebDavClient::new(config).unwrap();

        assert_eq!(
            client.build_url("/test/file.txt"),
            "https://dav.example.com/test/file.txt"
        );
    }
}
