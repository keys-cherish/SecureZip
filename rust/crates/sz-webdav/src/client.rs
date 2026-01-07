//! WebDAV 客户端实现

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use sz_core::{SzError, SzResult, WebDavConfig, WebDavFileInfo};

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
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
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
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
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

        // TODO: 解析 XML 响应
        // 这里返回空列表作为占位
        Ok(vec![])
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
        let file_size = file.metadata()?.len();
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
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url)
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
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header(AUTHORIZATION, self.auth_header())
            .header("Depth", "0")
            .send()
            .map_err(|e| SzError::Network(e.to_string()))?;

        Ok(response.status().is_success() || response.status().as_u16() == 207)
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
