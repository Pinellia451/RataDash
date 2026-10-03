use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tokio_tungstenite::tungstenite::Message;

use crate::app::{App, Command, ConnectionState, ToastKind};
use crate::cli::RuntimeOptions;
use crate::controller::{ControllerApi, ControllerClient};
use crate::error::{AppError, Result};
use crate::model::{
    ConfigSnapshot, ConnectionsSnapshot, DelayHistory, LogEntry, MemorySnapshot, ProviderResponse,
    ProxyResponse, RuleProviderResponse, TrafficSnapshot, VersionInfo,
};
use crate::terminal::TerminalSession;

const EVENT_BUFFER: usize = 1_024;
const DRAW_INTERVAL: Duration = Duration::from_millis(33);
const TEST_TIMEOUT_MS: u64 = 5_000;

pub async fn run(options: RuntimeOptions) -> Result<()> {
    let client = ControllerClient::new(
        options.controller.clone(),
        options.secret,
        options.read_only,
        options.timeout,
    )?;
    let initial = load_initial(&client).await?;

    let mut app = App::new(client.base_url().to_string(), client.is_read_only());
    app.set_initial_data(
        initial.version,
        initial.config,
        initial.proxies,
        initial.providers,
        initial.rule_providers,
        initial.connections,
    );
    if client.is_insecure_remote() {
        app.push_toast(
            "正在通过远程明文 HTTP 传输认证信息，建议使用 HTTPS 或 SSH 隧道",
            ToastKind::Warning,
        );
    }

    let mut terminal = TerminalSession::new()?;
    let (tx, mut rx) = mpsc::channel(EVENT_BUFFER);
    let input_running = Arc::new(AtomicBool::new(true));
    spawn_input(tx.clone(), input_running.clone());
    spawn_streams(client.clone(), tx.clone());

    let mut draw_tick = tokio::time::interval(DRAW_INTERVAL);
    draw_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut refresh_tick = tokio::time::interval(options.refresh_interval);
    refresh_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // The initial interval tick is immediate; consume it because startup just loaded everything.
    refresh_tick.tick().await;

    while !app.should_quit {
        tokio::select! {
            Some(message) = rx.recv() => {
                handle_runtime_event(message, &mut app, &client, &tx);
            }
            _ = refresh_tick.tick() => {
                spawn_periodic_refresh(client.clone(), tx.clone());
            }
            _ = draw_tick.tick() => {
                app.tick();
                if app.dirty {
                    terminal.draw(|frame| crate::ui::render(frame, &app))?;
                    app.dirty = false;
                }
            }
        }
    }

    input_running.store(false, Ordering::Relaxed);
    Ok(())
}

#[derive(Debug)]
struct InitialData {
    version: VersionInfo,
    config: ConfigSnapshot,
    proxies: ProxyResponse,
    providers: ProviderResponse,
    rule_providers: RuleProviderResponse,
    connections: ConnectionsSnapshot,
}

async fn load_initial(client: &ControllerClient) -> Result<InitialData> {
    let (version, config, proxies, connections) = tokio::try_join!(
        client.version(),
        client.configs(),
        client.proxies(),
        client.connections(),
    )?;
    let (providers, rule_providers) = tokio::join!(client.providers(), client.rule_providers());
    Ok(InitialData {
        version,
        config,
        proxies,
        providers: providers.unwrap_or_default(),
        rule_providers: rule_providers.unwrap_or_default(),
        connections,
    })
}

#[derive(Debug)]
enum RuntimeEvent {
    Input(CrosstermEvent),
    Traffic(TrafficSnapshot),
    Memory(MemorySnapshot),
    Connections(ConnectionsSnapshot),
    Log(LogEntry),
    StreamOnline,
    StreamError(String),
    Periodic(PeriodicSnapshot),
    CommandFinished(std::result::Result<CommandOutcome, String>),
}

#[derive(Debug, Default)]
struct PeriodicSnapshot {
    config: Option<ConfigSnapshot>,
    proxies: Option<ProxyResponse>,
    providers: Option<ProviderResponse>,
    rule_providers: Option<RuleProviderResponse>,
    failed: bool,
}

#[derive(Debug)]
enum CommandOutcome {
    Full(InitialData),
    Proxies(ProxyResponse, String),
    Connections(ConnectionsSnapshot, String),
    Providers(ProviderResponse, RuleProviderResponse, String),
    Config(ConfigSnapshot, String),
    ProviderRefreshed {
        providers: ProviderResponse,
        rule_providers: RuleProviderResponse,
        proxies: ProxyResponse,
        message: String,
    },
    RuleProviderRefreshed {
        rule_providers: RuleProviderResponse,
        message: String,
    },
    Delay {
        name: String,
        delay: u32,
    },
}

fn handle_runtime_event(
    event: RuntimeEvent,
    app: &mut App,
    client: &ControllerClient,
    tx: &mpsc::Sender<RuntimeEvent>,
) {
    match event {
        RuntimeEvent::Input(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
            if let Some(command) = app.handle_key(key) {
                dispatch_command(app, client.clone(), command, tx.clone());
            }
        }
        RuntimeEvent::Input(CrosstermEvent::Mouse(mouse)) => {
            let key = match mouse.kind {
                MouseEventKind::ScrollUp => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                MouseEventKind::ScrollDown => {
                    Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
                }
                _ => None,
            };
            if let Some(key) = key {
                if let Some(command) = app.handle_key(key) {
                    dispatch_command(app, client.clone(), command, tx.clone());
                }
            }
        }
        RuntimeEvent::Input(CrosstermEvent::Resize(_, _)) => app.dirty = true,
        RuntimeEvent::Input(_) => {}
        RuntimeEvent::Traffic(traffic) => {
            app.connection_state = ConnectionState::Connected;
            app.apply_traffic(traffic);
        }
        RuntimeEvent::Memory(memory) => {
            app.connection_state = ConnectionState::Connected;
            app.apply_memory(memory);
        }
        RuntimeEvent::Connections(connections) => {
            app.connection_state = ConnectionState::Connected;
            app.apply_connections(connections);
        }
        RuntimeEvent::Log(log) => {
            app.connection_state = ConnectionState::Connected;
            app.push_log(log);
        }
        RuntimeEvent::StreamOnline => {
            app.connection_state = ConnectionState::Connected;
            app.dirty = true;
        }
        RuntimeEvent::StreamError(error) => {
            if app.connection_state == ConnectionState::Connected {
                app.push_toast(format!("实时数据流正在重连：{error}"), ToastKind::Warning);
            }
            app.connection_state = ConnectionState::Reconnecting;
            app.dirty = true;
        }
        RuntimeEvent::Periodic(snapshot) => {
            if let Some(config) = snapshot.config {
                app.apply_config(config);
            }
            if let Some(proxies) = snapshot.proxies {
                app.apply_proxies(proxies);
            }
            if let Some(providers) = snapshot.providers {
                app.apply_providers(providers);
            }
            if let Some(rule_providers) = snapshot.rule_providers {
                app.apply_rule_providers(rule_providers);
            }
            if snapshot.failed && app.connection_state == ConnectionState::Connected {
                app.connection_state = ConnectionState::Reconnecting;
                app.dirty = true;
            }
        }
        RuntimeEvent::CommandFinished(result) => match result {
            Ok(outcome) => apply_command_outcome(app, outcome),
            Err(error) => {
                if error.contains("认证失败") {
                    app.connection_state = ConnectionState::Unauthorized;
                }
                app.command_failed(error);
            }
        },
    }
}

fn dispatch_command(
    app: &mut App,
    client: ControllerClient,
    command: Command,
    tx: mpsc::Sender<RuntimeEvent>,
) {
    if app.read_only && command.is_write() {
        app.push_toast("只读模式已阻止该操作", ToastKind::Warning);
        return;
    }
    app.command_started(&command);
    tokio::spawn(async move {
        let result = execute_command(&client, command)
            .await
            .map_err(|error| error.to_string());
        let _ = tx.send(RuntimeEvent::CommandFinished(result)).await;
    });
}

async fn execute_command(client: &ControllerClient, command: Command) -> Result<CommandOutcome> {
    match command {
        Command::RefreshAll => Ok(CommandOutcome::Full(load_initial(client).await?)),
        Command::RefreshProxies => Ok(CommandOutcome::Proxies(
            client.proxies().await?,
            "代理列表已刷新".to_string(),
        )),
        Command::RefreshConnections => Ok(CommandOutcome::Connections(
            client.connections().await?,
            "连接列表已刷新".to_string(),
        )),
        Command::RefreshProviders => {
            let (providers, rule_providers) =
                tokio::try_join!(client.providers(), client.rule_providers())?;
            Ok(CommandOutcome::Providers(
                providers,
                rule_providers,
                "Provider 列表已刷新".to_string(),
            ))
        }
        Command::SwitchProxy { group, node } => {
            client.select_proxy(&group, &node).await?;
            Ok(CommandOutcome::Proxies(
                client.proxies().await?,
                format!("{group} 已切换到 {node}"),
            ))
        }
        Command::TestProxy { name, test_url } => {
            let delay = client
                .proxy_delay(&name, &test_url, TEST_TIMEOUT_MS)
                .await?;
            Ok(CommandOutcome::Delay { name, delay })
        }
        Command::TestGroup { name, test_url } => {
            let delay = client
                .group_delay(&name, &test_url, TEST_TIMEOUT_MS)
                .await?;
            Ok(CommandOutcome::Delay { name, delay })
        }
        Command::CloseConnection { id } => {
            client.close_connection(&id).await?;
            Ok(CommandOutcome::Connections(
                client.connections().await?,
                "连接已关闭".to_string(),
            ))
        }
        Command::CloseAllConnections => {
            client.close_all_connections().await?;
            Ok(CommandOutcome::Connections(
                client.connections().await?,
                "全部连接已关闭".to_string(),
            ))
        }
        Command::SetMode { mode } => {
            client.set_mode(&mode).await?;
            Ok(CommandOutcome::Config(
                client.configs().await?,
                format!("运行模式已切换为 {mode}"),
            ))
        }
        Command::RefreshProvider { name } => {
            client.refresh_provider(&name).await?;
            let (providers, rule_providers, proxies) = tokio::try_join!(
                client.providers(),
                client.rule_providers(),
                client.proxies()
            )?;
            Ok(CommandOutcome::ProviderRefreshed {
                providers,
                rule_providers,
                proxies,
                message: format!("Provider {name} 已刷新"),
            })
        }
        Command::RefreshRuleProvider { name } => {
            client.refresh_rule_provider(&name).await?;
            Ok(CommandOutcome::RuleProviderRefreshed {
                rule_providers: client.rule_providers().await?,
                message: format!("规则 Provider {name} 已刷新"),
            })
        }
    }
}

fn apply_command_outcome(app: &mut App, outcome: CommandOutcome) {
    match outcome {
        CommandOutcome::Full(initial) => {
            app.set_initial_data(
                initial.version,
                initial.config,
                initial.proxies,
                initial.providers,
                initial.rule_providers,
                initial.connections,
            );
            app.command_finished("全部数据已刷新");
        }
        CommandOutcome::Proxies(proxies, message) => {
            app.apply_proxies(proxies);
            app.command_finished(message);
        }
        CommandOutcome::Connections(connections, message) => {
            app.apply_connections(connections);
            app.command_finished(message);
        }
        CommandOutcome::Providers(providers, rule_providers, message) => {
            app.apply_providers(providers);
            app.apply_rule_providers(rule_providers);
            app.command_finished(message);
        }
        CommandOutcome::Config(config, message) => {
            app.apply_config(config);
            app.command_finished(message);
        }
        CommandOutcome::ProviderRefreshed {
            providers,
            rule_providers,
            proxies,
            message,
        } => {
            app.apply_providers(providers);
            app.apply_rule_providers(rule_providers);
            app.apply_proxies(proxies);
            app.command_finished(message);
        }
        CommandOutcome::RuleProviderRefreshed {
            rule_providers,
            message,
        } => {
            app.apply_rule_providers(rule_providers);
            app.command_finished(message);
        }
        CommandOutcome::Delay { name, delay } => {
            if let Some(proxy) = app.proxies.get_mut(&name) {
                proxy.history.push(DelayHistory {
                    time: "now".to_string(),
                    delay,
                });
            }
            app.command_finished(format!("{name} 延迟：{delay} ms"));
        }
    }
}

fn spawn_input(tx: mpsc::Sender<RuntimeEvent>, running: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        while running.load(Ordering::Relaxed) {
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => match event::read() {
                    Ok(event) => {
                        if tx.blocking_send(RuntimeEvent::Input(event)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });
}

fn spawn_streams(client: ControllerClient, tx: mpsc::Sender<RuntimeEvent>) {
    spawn_json_stream::<TrafficSnapshot, _>(
        client.clone(),
        &["traffic"],
        &[],
        tx.clone(),
        RuntimeEvent::Traffic,
    );
    spawn_json_stream::<MemorySnapshot, _>(
        client.clone(),
        &["memory"],
        &[],
        tx.clone(),
        RuntimeEvent::Memory,
    );
    spawn_json_stream::<ConnectionsSnapshot, _>(
        client.clone(),
        &["connections"],
        &[("interval", "1000")],
        tx.clone(),
        RuntimeEvent::Connections,
    );
    spawn_json_stream::<LogEntry, _>(
        client,
        &["logs"],
        &[("level", "debug")],
        tx,
        RuntimeEvent::Log,
    );
}

fn spawn_json_stream<T, F>(
    client: ControllerClient,
    path: &'static [&'static str],
    query: &'static [(&'static str, &'static str)],
    tx: mpsc::Sender<RuntimeEvent>,
    map: F,
) where
    T: DeserializeOwned + Send + 'static,
    F: Fn(T) -> RuntimeEvent + Send + Copy + 'static,
{
    tokio::spawn(async move {
        let mut backoff = Duration::from_millis(500);
        loop {
            match client.open_websocket(path, query).await {
                Ok(mut stream) => {
                    let _ = tx.send(RuntimeEvent::StreamOnline).await;
                    backoff = Duration::from_millis(500);
                    while let Some(message) = stream.next().await {
                        let decoded = match message {
                            Ok(Message::Text(text)) => serde_json::from_str::<T>(text.as_str()),
                            Ok(Message::Binary(bytes)) => serde_json::from_slice::<T>(&bytes),
                            Ok(Message::Close(_)) => break,
                            Ok(_) => continue,
                            Err(error) => {
                                let _ = tx.send(RuntimeEvent::StreamError(error.to_string())).await;
                                break;
                            }
                        };
                        match decoded {
                            Ok(value) => {
                                if tx.send(map(value)).await.is_err() {
                                    return;
                                }
                            }
                            Err(error) => {
                                let _ = tx
                                    .send(RuntimeEvent::StreamError(format!(
                                        "数据格式错误: {error}"
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    let _ = tx.send(RuntimeEvent::StreamError(error.to_string())).await;
                }
            }
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(10));
        }
    });
}

fn spawn_periodic_refresh(client: ControllerClient, tx: mpsc::Sender<RuntimeEvent>) {
    tokio::spawn(async move {
        let (config, proxies, providers, rule_providers) = tokio::join!(
            client.configs(),
            client.proxies(),
            client.providers(),
            client.rule_providers(),
        );
        let failed = config.is_err() || proxies.is_err();
        let snapshot = PeriodicSnapshot {
            config: config.ok(),
            proxies: proxies.ok(),
            providers: providers.ok(),
            rule_providers: rule_providers.ok(),
            failed,
        };
        let _ = tx.send(RuntimeEvent::Periodic(snapshot)).await;
    });
}

impl From<AppError> for RuntimeEvent {
    fn from(error: AppError) -> Self {
        Self::StreamError(error.to_string())
    }
}
