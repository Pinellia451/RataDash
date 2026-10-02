#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VersionInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub meta: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConfigSnapshot {
    #[serde(default)]
    pub mode: String,
    #[serde(default, rename = "mixed-port")]
    pub mixed_port: u16,
    #[serde(default, rename = "socks-port")]
    pub socks_port: u16,
    #[serde(default)]
    pub port: u16,
    #[serde(default, rename = "allow-lan")]
    pub allow_lan: bool,
    #[serde(default)]
    pub ipv6: bool,
    #[serde(default, rename = "log-level")]
    pub log_level: String,
    #[serde(default)]
    pub tun: Option<TunSnapshot>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TunSnapshot {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub stack: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProxyResponse {
    #[serde(default)]
    pub proxies: BTreeMap<String, Proxy>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Proxy {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub now: String,
    #[serde(default)]
    pub all: Vec<String>,
    #[serde(default)]
    pub alive: Option<bool>,
    #[serde(default)]
    pub history: Vec<DelayHistory>,
    #[serde(default, rename = "provider-name")]
    pub provider_name: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Proxy {
    pub fn is_group(&self) -> bool {
        !self.all.is_empty()
            || matches!(
                self.kind.as_str(),
                "Selector" | "URLTest" | "Fallback" | "LoadBalance"
            )
    }

    pub fn latest_delay(&self) -> Option<u32> {
        self.history
            .iter()
            .rev()
            .find_map(|item| (item.delay > 0).then_some(item.delay))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DelayHistory {
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub delay: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DelayResponse {
    #[serde(default)]
    pub delay: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProviderResponse {
    #[serde(default)]
    pub providers: BTreeMap<String, Provider>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RuleProviderResponse {
    #[serde(default)]
    pub providers: BTreeMap<String, RuleProvider>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Provider {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default, rename = "vehicleType")]
    pub vehicle_type: String,
    #[serde(default, rename = "updatedAt")]
    pub updated_at: String,
    #[serde(default)]
    pub proxies: Vec<Proxy>,
    #[serde(default, rename = "subscriptionInfo")]
    pub subscription_info: Option<SubscriptionInfo>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Provider {
    pub fn is_subscription(&self) -> bool {
        !self.vehicle_type.eq_ignore_ascii_case("compatible")
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RuleProvider {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub behavior: String,
    #[serde(default)]
    pub format: String,
    #[serde(default, rename = "ruleCount")]
    pub rule_count: u64,
    #[serde(default, rename = "updatedAt")]
    pub updated_at: String,
    #[serde(default)]
    pub size: u64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SubscriptionInfo {
    #[serde(default, rename = "Upload", alias = "upload")]
    pub upload: u64,
    #[serde(default, rename = "Download", alias = "download")]
    pub download: u64,
    #[serde(default, rename = "Total", alias = "total")]
    pub total: u64,
    #[serde(default, rename = "Expire", alias = "expire")]
    pub expire: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConnectionsSnapshot {
    #[serde(default, rename = "uploadTotal")]
    pub upload_total: u64,
    #[serde(default, rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(default)]
    pub memory: u64,
    #[serde(default)]
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Connection {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub metadata: ConnectionMetadata,
    #[serde(default)]
    pub upload: u64,
    #[serde(default)]
    pub download: u64,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub chains: Vec<String>,
    #[serde(default, rename = "providerChains")]
    pub provider_chains: Vec<String>,
    #[serde(default)]
    pub rule: String,
    #[serde(default, rename = "rulePayload")]
    pub rule_payload: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConnectionMetadata {
    #[serde(default)]
    pub network: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub host: String,
    #[serde(default, rename = "destinationIP")]
    pub destination_ip: String,
    #[serde(default, rename = "destinationPort")]
    pub destination_port: String,
    #[serde(default, rename = "sourceIP")]
    pub source_ip: String,
    #[serde(default, rename = "sourcePort")]
    pub source_port: String,
    #[serde(default)]
    pub process: String,
    #[serde(default, rename = "processPath")]
    pub process_path: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TrafficSnapshot {
    #[serde(default)]
    pub up: u64,
    #[serde(default)]
    pub down: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MemorySnapshot {
    #[serde(default)]
    pub inuse: u64,
    #[serde(default)]
    pub oslimit: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LogEntry {
    #[serde(default, rename = "type")]
    pub level: String,
    #[serde(default)]
    pub payload: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn next(self) -> Self {
        match self {
            Self::Debug => Self::Info,
            Self::Info => Self::Warning,
            Self::Warning => Self::Error,
            Self::Error => Self::Debug,
        }
    }

    pub fn accepts(self, level: &str) -> bool {
        rank(level) >= rank(self.as_str())
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

fn rank(level: &str) -> u8 {
    match level.to_ascii_lowercase().as_str() {
        "debug" => 0,
        "info" => 1,
        "warning" | "warn" => 2,
        "error" => 3,
        _ => 1,
    }
}

#[derive(Debug, Serialize)]
pub struct ProxySelection<'a> {
    pub name: &'a str,
}

#[derive(Debug, Serialize)]
pub struct ModePatch<'a> {
    pub mode: &'a str,
}

#[cfg(test)]
mod tests {
    use super::{LogLevel, Proxy, ProxyResponse, SubscriptionInfo};

    #[test]
    fn parses_unknown_proxy_fields_without_failure() {
        let response: ProxyResponse = serde_json::from_str(
            r#"{"proxies":{"AUTO":{"name":"AUTO","type":"Selector","all":["A"],"now":"A","future":true}}}"#,
        )
        .expect("parse response");
        let proxy: &Proxy = response.proxies.get("AUTO").expect("proxy");
        assert!(proxy.is_group());
        assert_eq!(
            proxy.extra.get("future").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn log_level_filter_is_ordered() {
        assert!(LogLevel::Info.accepts("error"));
        assert!(!LogLevel::Warning.accepts("info"));
        assert!(LogLevel::Debug.accepts("unknown"));
    }

    #[test]
    fn parses_mihomo_subscription_info_capitalized_fields() {
        let info: SubscriptionInfo = serde_json::from_str(
            r#"{"Upload":176972096147,"Download":224755536444,"Total":429496729600,"Expire":1793196042}"#,
        )
        .expect("subscription info");
        assert_eq!(info.upload, 176_972_096_147);
        assert_eq!(info.download, 224_755_536_444);
        assert_eq!(info.total, 429_496_729_600);
        assert_eq!(info.expire, 1_793_196_042);
    }
}
