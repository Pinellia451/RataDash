use std::fmt::Debug;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Method, Response, StatusCode};
use serde::de::DeserializeOwned;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

use crate::error::{AppError, Result};
use crate::model::{
    ConfigSnapshot, ConnectionsSnapshot, DelayResponse, ModePatch, ProviderResponse, ProxyResponse,
    ProxySelection, RuleProviderResponse, VersionInfo,
};

pub type ControllerWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[async_trait]
pub trait ControllerApi: Clone + Send + Sync + 'static {
    async fn version(&self) -> Result<VersionInfo>;
    async fn configs(&self) -> Result<ConfigSnapshot>;
    async fn proxies(&self) -> Result<ProxyResponse>;
    async fn providers(&self) -> Result<ProviderResponse>;
    async fn rule_providers(&self) -> Result<RuleProviderResponse>;
    async fn connections(&self) -> Result<ConnectionsSnapshot>;
    async fn select_proxy(&self, group: &str, name: &str) -> Result<()>;
    async fn set_mode(&self, mode: &str) -> Result<()>;
    async fn proxy_delay(&self, proxy: &str, test_url: &str, timeout_ms: u64) -> Result<u32>;
    async fn group_delay(&self, group: &str, test_url: &str, timeout_ms: u64) -> Result<u32>;
    async fn refresh_provider(&self, provider: &str) -> Result<()>;
    async fn refresh_rule_provider(&self, provider: &str) -> Result<()>;
    async fn close_connection(&self, id: &str) -> Result<()>;
    async fn close_all_connections(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct ControllerClient {
    base: Url,
    secret: Option<String>,
    read_only: bool,
    http: reqwest::Client,
}

impl Debug for ControllerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControllerClient")
            .field("base", &self.base)
            .field("secret", &self.secret.as_ref().map(|_| "[REDACTED]"))
            .field("read_only", &self.read_only)
            .finish_non_exhaustive()
    }
}

impl ControllerClient {
    pub fn new(
        base: Url,
        secret: Option<String>,
        read_only: bool,
        timeout: Duration,
    ) -> Result<Self> {
        let http = reqwest::Client::builder().timeout(timeout).build()?;
        Ok(Self {
            base,
            secret,
            read_only,
            http,
        })
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn is_insecure_remote(&self) -> bool {
        if self.base.scheme() != "http" {
            return false;
        }
        self.base
            .host_str()
            .is_some_and(|host| !matches!(host, "127.0.0.1" | "localhost" | "::1"))
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base.clone();
        url.set_query(None);
        url.set_fragment(None);
        let mut path = url
            .path_segments_mut()
            .map_err(|_| AppError::Message("控制端 URL 不能作为接口基地址".to_string()))?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
        drop(path);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        let request = self.http.request(method, url);
        match &self.secret {
            Some(secret) => request.bearer_auth(secret),
            None => request,
        }
    }

    async fn parse<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::Unauthorized);
        }
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(AppError::HttpStatus {
                status: status.as_u16(),
                message: truncate_error(&message),
            });
        }
        Ok(response.json().await?)
    }

    async fn expect_empty(&self, response: Response) -> Result<()> {
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::Unauthorized);
        }
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(AppError::HttpStatus {
                status: status.as_u16(),
                message: truncate_error(&message),
            });
        }
        Ok(())
    }

    fn ensure_writable(&self) -> Result<()> {
        if self.read_only {
            Err(AppError::ReadOnly)
        } else {
            Ok(())
        }
    }

    pub async fn open_websocket(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
    ) -> Result<ControllerWebSocket> {
        let mut url = self.endpoint(segments)?;
        url.set_scheme(match url.scheme() {
            "http" => "ws",
            "https" => "wss",
            scheme => return Err(AppError::UnsupportedControllerScheme(scheme.to_string())),
        })
        .map_err(|_| AppError::Message("无法生成 WebSocket 地址".to_string()))?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query.iter().copied());
        }

        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|error| AppError::Message(format!("无法创建 WebSocket 请求: {error}")))?;
        if let Some(secret) = &self.secret {
            let value = HeaderValue::from_str(&format!("Bearer {secret}"))
                .map_err(|_| AppError::Message("Secret 不能转换为认证请求头".to_string()))?;
            request.headers_mut().insert("authorization", value);
        }
        let (stream, _) = connect_async(request).await?;
        Ok(stream)
    }
}

#[async_trait]
impl ControllerApi for ControllerClient {
    async fn version(&self) -> Result<VersionInfo> {
        let response = self
            .request(Method::GET, self.endpoint(&["version"])?)
            .send()
            .await?;
        self.parse(response).await
    }

    async fn configs(&self) -> Result<ConfigSnapshot> {
        let response = self
            .request(Method::GET, self.endpoint(&["configs"])?)
            .send()
            .await?;
        self.parse(response).await
    }

    async fn proxies(&self) -> Result<ProxyResponse> {
        let response = self
            .request(Method::GET, self.endpoint(&["proxies"])?)
            .send()
            .await?;
        self.parse(response).await
    }

    async fn providers(&self) -> Result<ProviderResponse> {
        let response = self
            .request(Method::GET, self.endpoint(&["providers", "proxies"])?)
            .send()
            .await?;
        self.parse(response).await
    }

    async fn rule_providers(&self) -> Result<RuleProviderResponse> {
        let response = self
            .request(Method::GET, self.endpoint(&["providers", "rules"])?)
            .send()
            .await?;
        self.parse(response).await
    }

    async fn connections(&self) -> Result<ConnectionsSnapshot> {
        let response = self
            .request(Method::GET, self.endpoint(&["connections"])?)
            .send()
            .await?;
        self.parse(response).await
    }

    async fn select_proxy(&self, group: &str, name: &str) -> Result<()> {
        self.ensure_writable()?;
        let response = self
            .request(Method::PUT, self.endpoint(&["proxies", group])?)
            .json(&ProxySelection { name })
            .send()
            .await?;
        self.expect_empty(response).await
    }

    async fn set_mode(&self, mode: &str) -> Result<()> {
        self.ensure_writable()?;
        let response = self
            .request(Method::PATCH, self.endpoint(&["configs"])?)
            .json(&ModePatch { mode })
            .send()
            .await?;
        self.expect_empty(response).await
    }

    async fn proxy_delay(&self, proxy: &str, test_url: &str, timeout_ms: u64) -> Result<u32> {
        let mut url = self.endpoint(&["proxies", proxy, "delay"])?;
        url.query_pairs_mut()
            .append_pair("url", test_url)
            .append_pair("timeout", &timeout_ms.to_string());
        let response = self.request(Method::GET, url).send().await?;
        let result: DelayResponse = self.parse(response).await?;
        Ok(result.delay)
    }

    async fn group_delay(&self, group: &str, test_url: &str, timeout_ms: u64) -> Result<u32> {
        let mut url = self.endpoint(&["group", group, "delay"])?;
        url.query_pairs_mut()
            .append_pair("url", test_url)
            .append_pair("timeout", &timeout_ms.to_string());
        let response = self.request(Method::GET, url).send().await?;
        let result: DelayResponse = self.parse(response).await?;
        Ok(result.delay)
    }

    async fn refresh_provider(&self, provider: &str) -> Result<()> {
        self.ensure_writable()?;
        let response = self
            .request(
                Method::PUT,
                self.endpoint(&["providers", "proxies", provider])?,
            )
            .send()
            .await?;
        self.expect_empty(response).await
    }

    async fn refresh_rule_provider(&self, provider: &str) -> Result<()> {
        self.ensure_writable()?;
        let response = self
            .request(
                Method::PUT,
                self.endpoint(&["providers", "rules", provider])?,
            )
            .send()
            .await?;
        self.expect_empty(response).await
    }

    async fn close_connection(&self, id: &str) -> Result<()> {
        self.ensure_writable()?;
        let response = self
            .request(Method::DELETE, self.endpoint(&["connections", id])?)
            .send()
            .await?;
        self.expect_empty(response).await
    }

    async fn close_all_connections(&self) -> Result<()> {
        self.ensure_writable()?;
        let response = self
            .request(Method::DELETE, self.endpoint(&["connections"])?)
            .send()
            .await?;
        self.expect_empty(response).await
    }
}

fn truncate_error(message: &str) -> String {
    const MAX_CHARS: usize = 240;
    let mut chars = message.trim().chars();
    let truncated: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else if truncated.is_empty() {
        "无错误详情".to_string()
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use url::Url;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    use super::{ControllerApi, ControllerClient};
    use crate::error::AppError;

    #[tokio::test]
    async fn sends_bearer_auth_and_preserves_secondary_path() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/control/version"))
            .and(matchers::header("authorization", "Bearer top-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "version": "v1.2.3",
                "meta": true
            })))
            .mount(&server)
            .await;

        let base = Url::parse(&format!("{}/control/", server.uri())).expect("base url");
        let client = ControllerClient::new(
            base,
            Some("top-secret".to_string()),
            false,
            Duration::from_secs(1),
        )
        .expect("client");
        let version = client.version().await.expect("version response");
        assert_eq!(version.version, "v1.2.3");
    }

    #[tokio::test]
    async fn blocks_writes_in_read_only_mode_before_network() {
        let client = ControllerClient::new(
            Url::parse("http://127.0.0.1:1/").expect("url"),
            None,
            true,
            Duration::from_millis(50),
        )
        .expect("client");
        let error = client
            .set_mode("global")
            .await
            .expect_err("write should be blocked");
        assert!(matches!(error, AppError::ReadOnly));
    }
}
