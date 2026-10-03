#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::{
    ConfigSnapshot, Connection, ConnectionsSnapshot, LogEntry, LogLevel, MemorySnapshot, Provider,
    ProviderResponse, Proxy, ProxyResponse, RuleProvider, RuleProviderResponse, TrafficSnapshot,
    VersionInfo,
};

const MAX_CHART_SAMPLES: usize = 60;
const MAX_LOG_LINES: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Proxies,
    Connections,
    Logs,
    Settings,
}

impl Page {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Proxies,
        Self::Connections,
        Self::Logs,
        Self::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "总览",
            Self::Proxies => "代理",
            Self::Connections => "连接",
            Self::Logs => "日志",
            Self::Settings => "设置",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    RootMenu,
    OverviewMetrics,
    OverviewCharts,
    OverviewStatus,
    ProxyGroups,
    ProxyNodes,
    ConnectionsTable,
    ConnectionDetails,
    LogsStream,
    LogsControls,
    SettingsCore,
    SettingsProviders,
    SettingsRuleProviders,
}

impl FocusTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::RootMenu => "菜单",
            Self::OverviewMetrics => "指标",
            Self::OverviewCharts => "流量图",
            Self::OverviewStatus => "核心状态",
            Self::ProxyGroups => "代理组",
            Self::ProxyNodes => "节点",
            Self::ConnectionsTable => "连接表",
            Self::ConnectionDetails => "连接详情",
            Self::LogsStream => "日志流",
            Self::LogsControls => "日志操作",
            Self::SettingsCore => "核心设置",
            Self::SettingsProviders => "Provider",
            Self::SettingsRuleProviders => "规则 Provider",
        }
    }

    pub fn chain(page: Page) -> &'static [Self] {
        match page {
            Page::Overview => &[
                Self::RootMenu,
                Self::OverviewMetrics,
                Self::OverviewCharts,
                Self::OverviewStatus,
            ],
            Page::Proxies => &[Self::RootMenu, Self::ProxyGroups, Self::ProxyNodes],
            Page::Connections => &[
                Self::RootMenu,
                Self::ConnectionsTable,
                Self::ConnectionDetails,
            ],
            Page::Logs => &[Self::RootMenu, Self::LogsStream, Self::LogsControls],
            Page::Settings => &[
                Self::RootMenu,
                Self::SettingsCore,
                Self::SettingsProviders,
                Self::SettingsRuleProviders,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Reconnecting,
    Offline,
    Unauthorized,
}

impl ConnectionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Connecting => "连接中",
            Self::Connected => "已连接",
            Self::Reconnecting => "重连中",
            Self::Offline => "离线",
            Self::Unauthorized => "认证失败",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    RefreshAll,
    RefreshProxies,
    RefreshConnections,
    RefreshProviders,
    SwitchProxy { group: String, node: String },
    TestProxy { name: String, test_url: String },
    TestGroup { name: String, test_url: String },
    CloseConnection { id: String },
    CloseAllConnections,
    SetMode { mode: String },
    RefreshProvider { name: String },
    RefreshRuleProvider { name: String },
}

impl Command {
    pub fn is_write(&self) -> bool {
        matches!(
            self,
            Self::SwitchProxy { .. }
                | Self::CloseConnection { .. }
                | Self::CloseAllConnections
                | Self::SetMode { .. }
                | Self::RefreshProvider { .. }
                | Self::RefreshRuleProvider { .. }
        )
    }

    pub fn pending_label(&self) -> &'static str {
        match self {
            Self::RefreshAll
            | Self::RefreshProxies
            | Self::RefreshConnections
            | Self::RefreshProviders => "正在刷新…",
            Self::SwitchProxy { .. } => "正在切换节点…",
            Self::TestProxy { .. } | Self::TestGroup { .. } => "正在测速…",
            Self::CloseConnection { .. } => "正在关闭连接…",
            Self::CloseAllConnections => "正在关闭全部连接…",
            Self::SetMode { .. } => "正在切换模式…",
            Self::RefreshProvider { .. } => "正在刷新 Provider…",
            Self::RefreshRuleProvider { .. } => "正在刷新规则 Provider…",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    CloseConnection(Command),
    CloseAll(Command),
}

impl ConfirmAction {
    pub fn prompt(&self) -> &'static str {
        match self {
            Self::CloseConnection(_) => "确认关闭选中的连接？",
            Self::CloseAll(_) => "确认关闭全部活动连接？此操作无法撤销。",
        }
    }

    pub fn command(&self) -> Command {
        match self {
            Self::CloseConnection(command) | Self::CloseAll(command) => command.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    created_at: Instant,
}

#[derive(Debug)]
pub struct App {
    pub controller: String,
    pub read_only: bool,
    pub page: Page,
    pub focus: FocusTarget,
    pub connection_state: ConnectionState,
    pub version: Option<VersionInfo>,
    pub config: Option<ConfigSnapshot>,
    pub proxies: BTreeMap<String, Proxy>,
    pub proxy_order: Vec<String>,
    pub providers: BTreeMap<String, Provider>,
    pub rule_providers: BTreeMap<String, RuleProvider>,
    pub connections: ConnectionsSnapshot,
    pub traffic: TrafficSnapshot,
    pub memory: MemorySnapshot,
    pub up_history: VecDeque<u64>,
    pub down_history: VecDeque<u64>,
    pub memory_history: VecDeque<u64>,
    pub logs: VecDeque<LogEntry>,
    pub log_level: LogLevel,
    pub logs_paused: bool,
    pub group_index: usize,
    pub node_index: usize,
    pub connection_index: usize,
    pub provider_index: usize,
    pub rule_provider_index: usize,
    pub filter: String,
    pub editing_filter: bool,
    pub show_help: bool,
    pub confirm: Option<ConfirmAction>,
    pub toast: Option<Toast>,
    pub pending: usize,
    pub should_quit: bool,
    pub dirty: bool,
}

impl App {
    pub fn new(controller: String, read_only: bool) -> Self {
        Self {
            controller,
            read_only,
            page: Page::Overview,
            focus: FocusTarget::RootMenu,
            connection_state: ConnectionState::Connecting,
            version: None,
            config: None,
            proxies: BTreeMap::new(),
            proxy_order: Vec::new(),
            providers: BTreeMap::new(),
            rule_providers: BTreeMap::new(),
            connections: ConnectionsSnapshot::default(),
            traffic: TrafficSnapshot::default(),
            memory: MemorySnapshot::default(),
            up_history: VecDeque::with_capacity(MAX_CHART_SAMPLES),
            down_history: VecDeque::with_capacity(MAX_CHART_SAMPLES),
            memory_history: VecDeque::with_capacity(MAX_CHART_SAMPLES),
            logs: VecDeque::with_capacity(MAX_LOG_LINES),
            log_level: LogLevel::Info,
            logs_paused: false,
            group_index: 0,
            node_index: 0,
            connection_index: 0,
            provider_index: 0,
            rule_provider_index: 0,
            filter: String::new(),
            editing_filter: false,
            show_help: false,
            confirm: None,
            toast: None,
            pending: 0,
            should_quit: false,
            dirty: true,
        }
    }

    pub fn set_initial_data(
        &mut self,
        version: VersionInfo,
        config: ConfigSnapshot,
        proxies: ProxyResponse,
        providers: ProviderResponse,
        rule_providers: RuleProviderResponse,
        connections: ConnectionsSnapshot,
    ) {
        self.version = Some(version);
        self.config = Some(config);
        self.proxy_order = proxies.order;
        self.proxies = proxies.proxies;
        self.providers = providers.providers;
        for (name, provider) in &mut self.providers {
            if provider.name.is_empty() {
                provider.name = name.clone();
            }
        }
        self.rule_providers = rule_providers.providers;
        for (name, provider) in &mut self.rule_providers {
            if provider.name.is_empty() {
                provider.name = name.clone();
            }
        }
        self.connections = connections;
        self.connection_state = ConnectionState::Connected;
        self.clamp_selections();
        self.dirty = true;
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.connection_state = ConnectionState::Offline;
        self.push_toast(message, ToastKind::Error);
    }

    pub fn proxy_groups(&self) -> Vec<&Proxy> {
        let mut groups: Vec<&Proxy> = Vec::new();
        // Mihomo's GLOBAL group is the canonical ordering source used by
        // zashboard.  The response order is only a fallback for groups that
        // are not listed in GLOBAL.all (for example newly-added groups).
        let mut ordered_names = self
            .proxies
            .get("GLOBAL")
            .or_else(|| {
                self.proxies
                    .values()
                    .find(|proxy| proxy.name.eq_ignore_ascii_case("GLOBAL"))
            })
            .map(|global| global.all.clone())
            .unwrap_or_default();
        ordered_names.extend(self.proxy_order.iter().cloned());

        for name in &ordered_names {
            if let Some(proxy) = self.proxies.get(name) {
                if !name.eq_ignore_ascii_case("GLOBAL")
                    && !proxy.hidden
                    && !proxy.all.is_empty()
                    && !groups.iter().any(|group| group.name == proxy.name)
                {
                    groups.push(proxy);
                }
            }
        }
        for proxy in self.proxies.values() {
            if !proxy.name.eq_ignore_ascii_case("GLOBAL")
                && !proxy.hidden
                && !proxy.all.is_empty()
                && !groups.iter().any(|group| group.name == proxy.name)
            {
                groups.push(proxy);
            }
        }
        groups
    }

    pub fn selected_group(&self) -> Option<&Proxy> {
        self.proxy_groups().get(self.group_index).copied()
    }

    pub fn test_url_for_proxy(&self, name: &str) -> String {
        const DEFAULT_TEST_URL: &str = "https://www.gstatic.com/generate_204";

        // Mihomo proxy-group configuration is the core-level equivalent of
        // zashboard's per-group test URL override.
        if let Some(url) = self
            .config
            .as_ref()
            .and_then(|config| config.proxy_groups.iter().find(|group| group.name == name))
            .and_then(|group| group.test_url())
        {
            return url.to_string();
        }

        if let Some(url) = self
            .proxies
            .get(name)
            .and_then(|proxy| proxy.test_url.as_deref())
            .filter(|url| !url.trim().is_empty())
        {
            return url.to_string();
        }

        // Provider proxy entries are merged into zashboard's proxy map. If
        // /proxies omits testUrl, use the provider-side copy as a fallback.
        if let Some(url) = self
            .providers
            .values()
            .flat_map(|provider| provider.proxies.iter())
            .find(|proxy| proxy.name == name)
            .and_then(|proxy| proxy.test_url.as_deref())
            .filter(|url| !url.trim().is_empty())
        {
            return url.to_string();
        }

        DEFAULT_TEST_URL.to_string()
    }

    pub fn filtered_node_names(&self) -> Vec<&str> {
        let filter = self.filter.to_ascii_lowercase();
        self.selected_group()
            .map(|group| {
                group
                    .all
                    .iter()
                    .filter(|name| filter.is_empty() || name.to_ascii_lowercase().contains(&filter))
                    .map(String::as_str)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn selected_node_name(&self) -> Option<&str> {
        self.filtered_node_names().get(self.node_index).copied()
    }

    pub fn selected_node(&self) -> Option<&Proxy> {
        self.selected_node_name()
            .and_then(|name| self.proxies.get(name))
    }

    pub fn filtered_connections(&self) -> Vec<&Connection> {
        let filter = self.filter.to_ascii_lowercase();
        self.connections
            .connections
            .iter()
            .filter(|connection| {
                filter.is_empty()
                    || connection
                        .metadata
                        .host
                        .to_ascii_lowercase()
                        .contains(&filter)
                    || connection
                        .metadata
                        .destination_ip
                        .to_ascii_lowercase()
                        .contains(&filter)
                    || connection.rule.to_ascii_lowercase().contains(&filter)
                    || connection
                        .chains
                        .iter()
                        .any(|chain| chain.to_ascii_lowercase().contains(&filter))
            })
            .collect()
    }

    pub fn selected_connection(&self) -> Option<&Connection> {
        self.filtered_connections()
            .get(self.connection_index)
            .copied()
    }

    pub fn provider_list(&self) -> Vec<&Provider> {
        self.providers
            .values()
            .filter(|provider| provider.is_subscription())
            .collect()
    }

    pub fn selected_provider(&self) -> Option<&Provider> {
        self.provider_list().get(self.provider_index).copied()
    }

    pub fn rule_provider_list(&self) -> Vec<&RuleProvider> {
        self.rule_providers.values().collect()
    }

    pub fn selected_rule_provider(&self) -> Option<&RuleProvider> {
        self.rule_provider_list()
            .get(self.rule_provider_index)
            .copied()
    }

    pub fn visible_logs(&self) -> Vec<&LogEntry> {
        let filter = self.filter.to_ascii_lowercase();
        self.logs
            .iter()
            .filter(|entry| self.log_level.accepts(&entry.level))
            .filter(|entry| {
                filter.is_empty() || entry.payload.to_ascii_lowercase().contains(&filter)
            })
            .collect()
    }

    pub fn apply_traffic(&mut self, traffic: TrafficSnapshot) {
        self.traffic = traffic;
        push_bounded(&mut self.up_history, self.traffic.up, MAX_CHART_SAMPLES);
        push_bounded(&mut self.down_history, self.traffic.down, MAX_CHART_SAMPLES);
        self.dirty = true;
    }

    pub fn apply_memory(&mut self, memory: MemorySnapshot) {
        self.memory = memory;
        push_bounded(
            &mut self.memory_history,
            self.memory.inuse,
            MAX_CHART_SAMPLES,
        );
        self.dirty = true;
    }

    pub fn apply_connections(&mut self, connections: ConnectionsSnapshot) {
        self.connections = connections;
        self.clamp_selections();
        self.dirty = true;
    }

    pub fn push_log(&mut self, log: LogEntry) {
        if self.logs_paused {
            return;
        }
        if self.logs.len() == MAX_LOG_LINES {
            self.logs.pop_front();
        }
        self.logs.push_back(log);
        if self.page == Page::Logs {
            self.dirty = true;
        }
    }

    pub fn apply_proxies(&mut self, response: ProxyResponse) {
        self.proxy_order = response.order;
        self.proxies = response.proxies;
        self.clamp_selections();
        self.dirty = true;
    }

    pub fn apply_providers(&mut self, response: ProviderResponse) {
        self.providers = response.providers;
        for (name, provider) in &mut self.providers {
            if provider.name.is_empty() {
                provider.name = name.clone();
            }
        }
        self.clamp_selections();
        self.dirty = true;
    }

    pub fn apply_rule_providers(&mut self, response: RuleProviderResponse) {
        self.rule_providers = response.providers;
        for (name, provider) in &mut self.rule_providers {
            if provider.name.is_empty() {
                provider.name = name.clone();
            }
        }
        self.clamp_selections();
        self.dirty = true;
    }

    pub fn apply_config(&mut self, config: ConfigSnapshot) {
        self.config = Some(config);
        self.dirty = true;
    }

    pub fn command_started(&mut self, command: &Command) {
        self.pending = self.pending.saturating_add(1);
        self.push_toast(command.pending_label(), ToastKind::Info);
    }

    pub fn command_finished(&mut self, message: impl Into<String>) {
        self.pending = self.pending.saturating_sub(1);
        self.push_toast(message, ToastKind::Success);
    }

    pub fn command_failed(&mut self, message: impl Into<String>) {
        self.pending = self.pending.saturating_sub(1);
        self.push_toast(message, ToastKind::Error);
    }

    pub fn tick(&mut self) {
        if self
            .toast
            .as_ref()
            .is_some_and(|toast| toast.created_at.elapsed() > Duration::from_secs(4))
        {
            self.toast = None;
            self.dirty = true;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Command> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return None;
        }

        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
                self.dirty = true;
            }
            return None;
        }

        if let Some(confirm) = self.confirm.clone() {
            return match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.confirm = None;
                    self.dirty = true;
                    Some(confirm.command())
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.confirm = None;
                    self.dirty = true;
                    None
                }
                _ => None,
            };
        }

        if self.editing_filter {
            match key.code {
                KeyCode::Esc => {
                    self.editing_filter = false;
                }
                KeyCode::Enter => {
                    self.editing_filter = false;
                    self.clamp_selections();
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.clamp_selections();
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter.push(ch);
                    self.clamp_selections();
                }
                _ => {}
            }
            self.dirty = true;
            return None;
        }

        if key.code == KeyCode::Left {
            self.retreat_focus();
            return None;
        }

        if key.code == KeyCode::Right {
            self.advance_focus_wrapping();
            return None;
        }

        if self.focus == FocusTarget::RootMenu {
            return self.handle_root_navigation(key);
        }

        if key.code == KeyCode::Enter && self.advance_focus_if_available() {
            return None;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('/') if matches!(self.page, Page::Connections | Page::Logs) => {
                self.editing_filter = true;
            }
            KeyCode::Char('r') => return Some(self.refresh_command()),
            _ => return self.handle_page_key(key),
        }
        self.dirty = true;
        None
    }

    fn handle_root_navigation(&mut self, key: KeyEvent) -> Option<Command> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let current = Page::ALL
                    .iter()
                    .position(|page| *page == self.page)
                    .unwrap_or_default();
                let next = current.saturating_sub(1);
                self.page = Page::ALL[next];
                self.filter.clear();
                self.focus = FocusTarget::RootMenu;
                self.dirty = true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let current = Page::ALL
                    .iter()
                    .position(|page| *page == self.page)
                    .unwrap_or_default();
                let next = (current + 1).min(Page::ALL.len().saturating_sub(1));
                self.page = Page::ALL[next];
                self.filter.clear();
                self.focus = FocusTarget::RootMenu;
                self.dirty = true;
            }
            KeyCode::Enter => {
                self.advance_focus_if_available();
            }
            _ => {}
        }
        None
    }

    fn advance_focus_if_available(&mut self) -> bool {
        let chain = FocusTarget::chain(self.page);
        let Some(index) = chain.iter().position(|target| *target == self.focus) else {
            return false;
        };
        let Some(next) = chain.get(index + 1).copied() else {
            return false;
        };
        self.focus = next;
        self.dirty = true;
        true
    }

    fn advance_focus_wrapping(&mut self) {
        let chain = FocusTarget::chain(self.page);
        let Some(index) = chain.iter().position(|target| *target == self.focus) else {
            return;
        };
        self.focus = chain[(index + 1) % chain.len()];
        self.dirty = true;
    }

    fn retreat_focus(&mut self) {
        let chain = FocusTarget::chain(self.page);
        let Some(index) = chain.iter().position(|target| *target == self.focus) else {
            return;
        };
        self.focus = if index == 0 {
            *chain.last().unwrap_or(&FocusTarget::RootMenu)
        } else {
            chain[index - 1]
        };
        self.dirty = true;
    }

    fn handle_page_key(&mut self, key: KeyEvent) -> Option<Command> {
        match self.page {
            Page::Overview => None,
            Page::Proxies => self.handle_proxy_key(key),
            Page::Connections => self.handle_connection_key(key),
            Page::Logs => self.handle_log_key(key),
            Page::Settings => self.handle_settings_key(key),
        }
    }

    fn handle_proxy_key(&mut self, key: KeyEvent) -> Option<Command> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => match self.focus {
                FocusTarget::ProxyGroups => {
                    self.group_index = self.group_index.saturating_sub(1);
                    self.node_index = 0;
                }
                FocusTarget::ProxyNodes => self.node_index = self.node_index.saturating_sub(1),
                _ => {}
            },
            KeyCode::Down | KeyCode::Char('j') => match self.focus {
                FocusTarget::ProxyGroups => {
                    let max = self.proxy_groups().len().saturating_sub(1);
                    self.group_index = (self.group_index + 1).min(max);
                    self.node_index = 0;
                }
                FocusTarget::ProxyNodes => {
                    let max = self.filtered_node_names().len().saturating_sub(1);
                    self.node_index = (self.node_index + 1).min(max);
                }
                _ => {}
            },
            KeyCode::Enter if self.focus == FocusTarget::ProxyNodes => {
                let group = self.selected_group()?.name.clone();
                let node = self.selected_node_name()?.to_string();
                return Some(Command::SwitchProxy { group, node });
            }
            KeyCode::Char('t') if self.focus == FocusTarget::ProxyNodes => {
                let name = self.selected_node_name()?.to_string();
                let test_url = self.test_url_for_proxy(&name);
                return Some(Command::TestProxy { name, test_url });
            }
            KeyCode::Char('T') if self.focus == FocusTarget::ProxyGroups => {
                let name = self.selected_group()?.name.clone();
                let test_url = self.test_url_for_proxy(&name);
                return Some(Command::TestGroup { name, test_url });
            }
            _ => return None,
        }
        self.dirty = true;
        None
    }

    fn handle_connection_key(&mut self, key: KeyEvent) -> Option<Command> {
        if self.focus != FocusTarget::ConnectionsTable {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.connection_index = self.connection_index.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.filtered_connections().len().saturating_sub(1);
                self.connection_index = (self.connection_index + 1).min(max);
            }
            KeyCode::Char('d') => {
                let id = self.selected_connection()?.id.clone();
                self.confirm = Some(ConfirmAction::CloseConnection(Command::CloseConnection {
                    id,
                }));
            }
            KeyCode::Char('D') => {
                self.confirm = Some(ConfirmAction::CloseAll(Command::CloseAllConnections));
            }
            _ => return None,
        }
        self.dirty = true;
        None
    }

    fn handle_log_key(&mut self, key: KeyEvent) -> Option<Command> {
        if self.focus != FocusTarget::LogsControls {
            return None;
        }
        match key.code {
            KeyCode::Char(' ') => self.logs_paused = !self.logs_paused,
            KeyCode::Char('c') => self.logs.clear(),
            KeyCode::Char('l') => self.log_level = self.log_level.next(),
            _ => return None,
        }
        self.dirty = true;
        None
    }

    fn handle_settings_key(&mut self, key: KeyEvent) -> Option<Command> {
        if self.focus == FocusTarget::SettingsCore {
            if key.code != KeyCode::Char('m') {
                return None;
            }
        } else if !matches!(
            self.focus,
            FocusTarget::SettingsProviders | FocusTarget::SettingsRuleProviders
        ) {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') if self.focus == FocusTarget::SettingsProviders => {
                self.provider_index = self.provider_index.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') if self.focus == FocusTarget::SettingsProviders => {
                let max = self.provider_list().len().saturating_sub(1);
                self.provider_index = (self.provider_index + 1).min(max);
            }
            KeyCode::Enter | KeyCode::Char('p') if self.focus == FocusTarget::SettingsProviders => {
                let name = self.selected_provider()?.name.clone();
                return Some(Command::RefreshProvider { name });
            }
            KeyCode::Up | KeyCode::Char('k')
                if self.focus == FocusTarget::SettingsRuleProviders =>
            {
                self.rule_provider_index = self.rule_provider_index.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j')
                if self.focus == FocusTarget::SettingsRuleProviders =>
            {
                let max = self.rule_provider_list().len().saturating_sub(1);
                self.rule_provider_index = (self.rule_provider_index + 1).min(max);
            }
            KeyCode::Enter | KeyCode::Char('p')
                if self.focus == FocusTarget::SettingsRuleProviders =>
            {
                let name = self.selected_rule_provider()?.name.clone();
                return Some(Command::RefreshRuleProvider { name });
            }
            KeyCode::Char('m') if self.focus == FocusTarget::SettingsCore => {
                let current = self
                    .config
                    .as_ref()
                    .map(|config| config.mode.as_str())
                    .unwrap_or("rule");
                let mode = match current.to_ascii_lowercase().as_str() {
                    "rule" => "global",
                    "global" => "direct",
                    _ => "rule",
                };
                return Some(Command::SetMode {
                    mode: mode.to_string(),
                });
            }
            _ => return None,
        }
        self.dirty = true;
        None
    }

    fn refresh_command(&self) -> Command {
        match self.page {
            Page::Overview => Command::RefreshAll,
            Page::Proxies => Command::RefreshProxies,
            Page::Connections => Command::RefreshConnections,
            Page::Logs => Command::RefreshAll,
            Page::Settings => Command::RefreshProviders,
        }
    }

    pub fn push_toast(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            message: message.into(),
            kind,
            created_at: Instant::now(),
        });
        self.dirty = true;
    }

    fn clamp_selections(&mut self) {
        self.group_index = self
            .group_index
            .min(self.proxy_groups().len().saturating_sub(1));
        self.node_index = self
            .node_index
            .min(self.filtered_node_names().len().saturating_sub(1));
        self.connection_index = self
            .connection_index
            .min(self.filtered_connections().len().saturating_sub(1));
        self.provider_index = self
            .provider_index
            .min(self.provider_list().len().saturating_sub(1));
        self.rule_provider_index = self
            .rule_provider_index
            .min(self.rule_provider_list().len().saturating_sub(1));
    }
}

fn push_bounded(queue: &mut VecDeque<u64>, value: u64, limit: usize) {
    if queue.len() == limit {
        queue.pop_front();
    }
    queue.push_back(value);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{App, Command, FocusTarget};
    use crate::model::{ConfigSnapshot, Proxy, ProxyGroupConfig};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn proxy_selection_emits_switch_command() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.proxies = BTreeMap::from([
            (
                "GROUP".to_string(),
                Proxy {
                    name: "GROUP".to_string(),
                    kind: "Selector".to_string(),
                    now: "A".to_string(),
                    all: vec!["A".to_string(), "B".to_string()],
                    ..Proxy::default()
                },
            ),
            (
                "A".to_string(),
                Proxy {
                    name: "A".to_string(),
                    kind: "Shadowsocks".to_string(),
                    ..Proxy::default()
                },
            ),
        ]);
        app.page = super::Page::Proxies;
        app.focus = FocusTarget::ProxyNodes;
        app.node_index = 1;

        let command = app.handle_key(key(KeyCode::Enter));
        assert!(matches!(
            command,
            Some(Command::SwitchProxy { group, node }) if group == "GROUP" && node == "B"
        ));
    }

    #[test]
    fn proxy_groups_follow_global_order_and_skip_global() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.proxies = BTreeMap::from([
            (
                "GLOBAL".to_string(),
                Proxy {
                    name: "GLOBAL".to_string(),
                    all: vec!["Streaming".to_string(), "Fallback".to_string()],
                    ..Proxy::default()
                },
            ),
            (
                "Fallback".to_string(),
                Proxy {
                    name: "Fallback".to_string(),
                    kind: "Fallback".to_string(),
                    all: vec!["A".to_string()],
                    ..Proxy::default()
                },
            ),
            (
                "Streaming".to_string(),
                Proxy {
                    name: "Streaming".to_string(),
                    kind: "Selector".to_string(),
                    all: vec!["A".to_string()],
                    ..Proxy::default()
                },
            ),
            (
                "A".to_string(),
                Proxy {
                    name: "A".to_string(),
                    kind: "Shadowsocks".to_string(),
                    ..Proxy::default()
                },
            ),
        ]);
        app.proxy_order = vec![
            "GLOBAL".to_string(),
            "Fallback".to_string(),
            "Streaming".to_string(),
            "A".to_string(),
        ];

        let names: Vec<_> = app
            .proxy_groups()
            .into_iter()
            .map(|proxy| proxy.name.as_str())
            .collect();
        assert_eq!(names, ["Streaming", "Fallback"]);
    }

    #[test]
    fn proxy_test_uses_node_configured_url_before_default() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.proxies = BTreeMap::from([
            (
                "GROUP".to_string(),
                Proxy {
                    name: "GROUP".to_string(),
                    kind: "Selector".to_string(),
                    all: vec!["NODE".to_string()],
                    ..Proxy::default()
                },
            ),
            (
                "NODE".to_string(),
                Proxy {
                    name: "NODE".to_string(),
                    test_url: Some("https://example.com/ping".to_string()),
                    ..Proxy::default()
                },
            ),
        ]);
        app.page = super::Page::Proxies;
        app.focus = FocusTarget::ProxyNodes;

        let command = app.handle_key(key(KeyCode::Char('t')));
        assert!(matches!(
            command,
            Some(Command::TestProxy { name, test_url })
                if name == "NODE" && test_url == "https://example.com/ping"
        ));
    }

    #[test]
    fn group_test_prefers_core_configured_url() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.config = Some(ConfigSnapshot {
            proxy_groups: vec![ProxyGroupConfig {
                name: "GROUP".to_string(),
                url: Some("https://example.com/group-check".to_string()),
                ..ProxyGroupConfig::default()
            }],
            ..ConfigSnapshot::default()
        });
        app.proxies = BTreeMap::from([(
            "GROUP".to_string(),
            Proxy {
                name: "GROUP".to_string(),
                kind: "Selector".to_string(),
                all: vec!["NODE".to_string()],
                test_url: Some("https://example.com/proxy-check".to_string()),
                ..Proxy::default()
            },
        )]);
        assert_eq!(
            app.test_url_for_proxy("GROUP"),
            "https://example.com/group-check"
        );
    }

    #[test]
    fn filter_input_never_quits_on_q() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.page = super::Page::Logs;
        app.editing_filter = true;
        app.handle_key(key(KeyCode::Char('q')));
        assert_eq!(app.filter, "q");
        assert!(!app.should_quit);
    }

    #[test]
    fn root_menu_uses_enter_and_arrow_focus_navigation() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.page, super::Page::Proxies);
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.focus, FocusTarget::ProxyGroups);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.focus, FocusTarget::ProxyNodes);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.focus, FocusTarget::ProxyGroups);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.focus, FocusTarget::RootMenu);
    }

    #[test]
    fn every_page_has_a_rooted_focus_chain() {
        let expected = [
            (
                super::Page::Overview,
                [
                    FocusTarget::RootMenu,
                    FocusTarget::OverviewMetrics,
                    FocusTarget::OverviewCharts,
                    FocusTarget::OverviewStatus,
                ]
                .as_slice(),
            ),
            (
                super::Page::Proxies,
                [
                    FocusTarget::RootMenu,
                    FocusTarget::ProxyGroups,
                    FocusTarget::ProxyNodes,
                ]
                .as_slice(),
            ),
            (
                super::Page::Connections,
                [
                    FocusTarget::RootMenu,
                    FocusTarget::ConnectionsTable,
                    FocusTarget::ConnectionDetails,
                ]
                .as_slice(),
            ),
            (
                super::Page::Logs,
                [
                    FocusTarget::RootMenu,
                    FocusTarget::LogsStream,
                    FocusTarget::LogsControls,
                ]
                .as_slice(),
            ),
            (
                super::Page::Settings,
                [
                    FocusTarget::RootMenu,
                    FocusTarget::SettingsCore,
                    FocusTarget::SettingsProviders,
                    FocusTarget::SettingsRuleProviders,
                ]
                .as_slice(),
            ),
        ];
        for (page, chain) in expected {
            assert_eq!(FocusTarget::chain(page), chain);
        }
    }

    #[test]
    fn enter_at_proxy_node_runs_action_without_advancing_focus() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.proxies = BTreeMap::from([(
            "GROUP".to_string(),
            Proxy {
                name: "GROUP".to_string(),
                kind: "Selector".to_string(),
                now: "A".to_string(),
                all: vec!["A".to_string()],
                ..Proxy::default()
            },
        )]);
        app.page = super::Page::Proxies;
        app.focus = FocusTarget::ProxyNodes;
        assert!(matches!(
            app.handle_key(key(KeyCode::Enter)),
            Some(Command::SwitchProxy { .. })
        ));
        assert_eq!(app.focus, FocusTarget::ProxyNodes);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.focus, FocusTarget::RootMenu);
    }

    #[test]
    fn left_and_right_wrap_focus_chain() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.page = super::Page::Proxies;
        app.focus = FocusTarget::ProxyNodes;
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.focus, FocusTarget::RootMenu);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.focus, FocusTarget::ProxyNodes);
    }

    #[test]
    fn right_arrow_enters_first_panel_from_root_menu() {
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.page = super::Page::Proxies;
        app.focus = FocusTarget::RootMenu;
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.focus, FocusTarget::ProxyGroups);
    }

    #[test]
    fn compatible_proxy_entries_are_not_subscription_providers() {
        use crate::model::{Provider, ProviderResponse};

        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.apply_providers(ProviderResponse {
            providers: BTreeMap::from([
                (
                    "remote".to_string(),
                    Provider {
                        vehicle_type: "HTTP".to_string(),
                        ..Provider::default()
                    },
                ),
                (
                    "group".to_string(),
                    Provider {
                        vehicle_type: "Compatible".to_string(),
                        ..Provider::default()
                    },
                ),
            ]),
        });
        assert_eq!(app.provider_list().len(), 1);
        assert_eq!(app.provider_list()[0].name, "remote");
    }
}
