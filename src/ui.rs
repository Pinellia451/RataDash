use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table, Wrap},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, ConnectionState, FocusTarget, Page, ToastKind};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const FOCUS_BORDER: Color = Color::Green;
const FOCUS_TITLE: Color = Color::LightGreen;
const SELECTED: Style = Style::new().fg(Color::Black).bg(Color::LightGreen);

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(frame, app, root[0]);
    if area.width >= 90 {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(22), Constraint::Min(0)])
            .split(root[1]);
        render_navigation(frame, app, body[0]);
        render_page(frame, app, body[1]);
    } else {
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(root[1]);
        render_navigation(frame, app, body[0]);
        render_page(frame, app, body[1]);
    }
    render_footer(frame, app, root[2]);

    if app.show_help {
        render_help(frame, app, area);
    }
    if let Some(confirm) = &app.confirm {
        render_dialog(frame, area, "确认操作", confirm.prompt(), Color::Yellow);
    }
    if let Some(toast) = &app.toast {
        render_toast(frame, area, &toast.message, toast.kind);
    }
}

fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let status_color = match app.connection_state {
        ConnectionState::Connected => Color::Green,
        ConnectionState::Connecting | ConnectionState::Reconnecting => Color::Yellow,
        ConnectionState::Offline | ConnectionState::Unauthorized => Color::Red,
    };
    let version = app
        .version
        .as_ref()
        .map(|version| version.version.as_str())
        .unwrap_or("未知版本");
    let mode = app
        .config
        .as_ref()
        .map(|config| config.mode.as_str())
        .unwrap_or("-");
    let mut right = vec![
        Span::styled(app.connection_state.label(), Style::new().fg(status_color)),
        Span::raw(format!("  {version}  mode:{mode}")),
    ];
    if app.read_only {
        right.push(Span::styled("  [只读]", Style::new().fg(Color::Yellow)));
    }
    let title = Line::from(vec![
        Span::styled(
            " RataDash ",
            Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled(app.controller.as_str(), Style::new().fg(DIM)),
        Span::raw("  "),
        right[0].clone(),
        right[1].clone(),
        right.get(2).cloned().unwrap_or_else(|| Span::raw("")),
        Span::raw("  │  "),
        Span::styled(breadcrumb(app), Style::new().fg(ACCENT)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(DIM));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if area.width >= 90 {
        frame.render_widget(Paragraph::new(title), inner);
    } else {
        let compact = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner);
        frame.render_widget(
            Paragraph::new(Span::styled(
                "RataDash",
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            compact[0],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(breadcrumb(app), Style::new().fg(ACCENT)),
                Span::raw(format!("  {}", app.connection_state.label())),
            ]))
            .alignment(Alignment::Right),
            compact[1],
        );
    }
}

fn render_navigation(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut lines = Vec::new();
    for page in Page::ALL {
        let label = format!(" {} ", page.title());
        if page == app.page {
            lines.push(Line::styled(label, SELECTED));
        } else {
            lines.push(Line::styled(label, Style::new().fg(Color::White)));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" 菜单 ")
                .borders(Borders::ALL)
                .border_style(Style::new().fg(if app.focus == FocusTarget::RootMenu {
                    FOCUS_BORDER
                } else {
                    DIM
                }))
                .title_style(Style::new().fg(if app.focus == FocusTarget::RootMenu {
                    FOCUS_TITLE
                } else {
                    DIM
                })),
        ),
        area,
    );
}

fn render_page(frame: &mut Frame<'_>, app: &App, area: Rect) {
    match app.page {
        Page::Overview => render_overview(frame, app, area),
        Page::Proxies => render_proxies(frame, app, area),
        Page::Connections => render_connections(frame, app, area),
        Page::Logs => render_logs(frame, app, area),
        Page::Settings => render_settings(frame, app, area),
    }
}

fn render_overview(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(area);
    let metrics_block = panel(" 指标 ", app.focus == FocusTarget::OverviewMetrics);
    let metrics_inner = metrics_block.inner(rows[0]);
    frame.render_widget(metrics_block, rows[0]);
    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(metrics_inner);
    metric_card(
        frame,
        cards[0],
        "上传",
        &format_rate(app.traffic.up),
        Color::Magenta,
    );
    metric_card(
        frame,
        cards[1],
        "下载",
        &format_rate(app.traffic.down),
        Color::Cyan,
    );
    metric_card(
        frame,
        cards[2],
        "内存",
        &format_bytes(app.memory.inuse.max(app.connections.memory)),
        Color::Yellow,
    );
    metric_card(
        frame,
        cards[3],
        "连接",
        &app.connections.connections.len().to_string(),
        Color::Green,
    );

    let charts_block = panel(" 流量图 ", app.focus == FocusTarget::OverviewCharts);
    let charts_inner = charts_block.inner(rows[1]);
    frame.render_widget(charts_block, rows[1]);
    let traffic = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(charts_inner);
    render_sparkline(
        frame,
        traffic[0],
        " 下载速率 ",
        &app.down_history,
        Color::Cyan,
    );
    render_sparkline(
        frame,
        traffic[1],
        " 上传速率 ",
        &app.up_history,
        Color::Magenta,
    );

    let config = app.config.as_ref();
    let details = vec![
        Line::from(format!("控制端：{}", app.controller)),
        Line::from(format!(
            "混合端口：{}    允许局域网：{}    IPv6：{}",
            config.map(|c| c.mixed_port).unwrap_or_default(),
            yes_no(config.is_some_and(|c| c.allow_lan)),
            yes_no(config.is_some_and(|c| c.ipv6))
        )),
        Line::from(format!(
            "TUN：{}    栈：{}    日志级别：{}",
            yes_no(
                config
                    .and_then(|c| c.tun.as_ref())
                    .is_some_and(|tun| tun.enable)
            ),
            config
                .and_then(|c| c.tun.as_ref())
                .map(|tun| tun.stack.as_str())
                .unwrap_or("-"),
            config.map(|c| c.log_level.as_str()).unwrap_or("-")
        )),
        Line::from(format!(
            "累计上传：{}    累计下载：{}",
            format_bytes(app.connections.upload_total),
            format_bytes(app.connections.download_total)
        )),
    ];
    frame.render_widget(
        Paragraph::new(details)
            .block(panel(
                " 核心状态 ",
                app.focus == FocusTarget::OverviewStatus,
            ))
            .wrap(Wrap { trim: true }),
        rows[2],
    );
}

fn render_proxies(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
        .split(area);
    let groups = app.proxy_groups();
    let group_inner_width = columns[0].width.saturating_sub(2);
    let group_name_width = (group_inner_width * 42 / 100).saturating_sub(2) as usize;
    let group_current_width = (group_inner_width * 40 / 100).saturating_sub(1) as usize;
    let group_rows = groups.iter().enumerate().map(|(index, proxy)| {
        let marker = if proxy.name == proxy.now { "●" } else { " " };
        Row::new(vec![
            Cell::from(format!(
                "{marker} {}",
                truncate_label(&proxy.name, group_name_width)
            )),
            Cell::from(proxy.kind.clone()),
            Cell::from(truncate_label(&proxy.now, group_current_width)),
        ])
        .style(if index == app.group_index {
            SELECTED
        } else {
            Style::default()
        })
    });
    frame.render_widget(
        Table::new(
            group_rows,
            [
                Constraint::Percentage(42),
                Constraint::Length(12),
                Constraint::Percentage(40),
            ],
        )
        .header(table_header(["代理组", "类型", "当前"]))
        .column_spacing(1)
        .block(panel(" 代理组 ", app.focus == FocusTarget::ProxyGroups)),
        columns[0],
    );

    let nodes = app.filtered_node_names();
    let node_inner_width = columns[1].width.saturating_sub(2);
    let node_name_width = (node_inner_width * 46 / 100) as usize;
    let node_rows = nodes.iter().enumerate().map(|(index, name)| {
        let proxy = app.proxies.get(*name);
        let selected = app
            .selected_group()
            .is_some_and(|group| group.now == **name);
        let alive = proxy.and_then(|proxy| proxy.alive);
        let delay = proxy
            .and_then(|proxy| proxy.latest_delay())
            .map(|delay| format!("{delay} ms"))
            .unwrap_or_else(|| "-".to_string());
        Row::new(vec![
            Cell::from(if selected { "●" } else { " " }),
            Cell::from(truncate_label(name, node_name_width)),
            Cell::from(proxy.map(|p| p.kind.clone()).unwrap_or_default()),
            Cell::from(match alive {
                Some(true) => "在线",
                Some(false) => "离线",
                None => "未知",
            }),
            Cell::from(delay),
        ])
        .style(if index == app.node_index {
            SELECTED
        } else if alive == Some(false) {
            Style::new().fg(Color::DarkGray)
        } else {
            Style::default()
        })
    });
    frame.render_widget(
        Table::new(
            node_rows,
            [
                Constraint::Length(2),
                Constraint::Percentage(46),
                Constraint::Length(14),
                Constraint::Length(6),
                Constraint::Length(10),
            ],
        )
        .header(table_header(["", "节点", "类型", "状态", "延迟"]))
        .column_spacing(1)
        .block(panel(
            " 节点 [Enter切换 t测速 T组测速] ",
            app.focus == FocusTarget::ProxyNodes,
        )),
        columns[1],
    );
}

fn render_connections(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(area);
    let connections = app.filtered_connections();
    let connection_inner_width = columns[0].width.saturating_sub(2);
    let host_width = (connection_inner_width * 24 / 100) as usize;
    let target_width = (connection_inner_width * 20 / 100) as usize;
    let rule_width = 13usize;
    let chain_width = (connection_inner_width * 24 / 100) as usize;
    let rows = connections.iter().enumerate().map(|(index, connection)| {
        let host = if connection.metadata.host.is_empty() {
            connection.metadata.destination_ip.clone()
        } else {
            connection.metadata.host.clone()
        };
        Row::new(vec![
            Cell::from(truncate_label(&host, host_width)),
            Cell::from(truncate_label(
                &format!(
                    "{}:{}",
                    connection.metadata.destination_ip, connection.metadata.destination_port
                ),
                target_width,
            )),
            Cell::from(connection.metadata.network.clone()),
            Cell::from(truncate_label(&connection.rule, rule_width)),
            Cell::from(truncate_label(&connection.chains.join(" → "), chain_width)),
            Cell::from(format_bytes(connection.download)),
            Cell::from(format_bytes(connection.upload)),
        ])
        .style(if index == app.connection_index {
            SELECTED
        } else {
            Style::default()
        })
    });
    let title = if app.filter.is_empty() {
        format!(" 活动连接 {} [d关闭 D全部] ", connections.len())
    } else {
        format!(" 活动连接 {} / 筛选:{} ", connections.len(), app.filter)
    };
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(24),
                Constraint::Percentage(20),
                Constraint::Length(7),
                Constraint::Length(13),
                Constraint::Percentage(24),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .header(table_header([
            "主机",
            "目标",
            "网络",
            "规则",
            "代理链",
            "下载",
            "上传",
        ]))
        .column_spacing(1)
        .block(panel(&title, app.focus == FocusTarget::ConnectionsTable)),
        columns[0],
    );

    let details = if let Some(connection) = app.selected_connection() {
        vec![
            Line::from(format!("ID：{}", connection.id)),
            Line::from(format!(
                "网络：{} / {}",
                connection.metadata.network, connection.metadata.kind
            )),
            Line::from(format!(
                "来源：{}:{}",
                connection.metadata.source_ip, connection.metadata.source_port
            )),
            Line::from(format!(
                "目标：{}:{}",
                connection.metadata.destination_ip, connection.metadata.destination_port
            )),
            Line::from(format!("进程：{}", connection.metadata.process)),
            Line::from(format!(
                "规则：{} {}",
                connection.rule, connection.rule_payload
            )),
            Line::from(format!("代理链：{}", connection.chains.join(" → "))),
            Line::from(format!(
                "流量：↓{} ↑{}",
                format_bytes(connection.download),
                format_bytes(connection.upload)
            )),
            Line::from(format!("开始：{}", connection.start)),
        ]
    } else {
        vec![Line::from("暂无选中的连接")]
    };
    frame.render_widget(
        Paragraph::new(details)
            .block(panel(
                " 连接详情 ",
                app.focus == FocusTarget::ConnectionDetails,
            ))
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn render_logs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(76), Constraint::Percentage(24)])
        .split(area);
    let visible = app.visible_logs();
    let capacity = columns[0].height.saturating_sub(2) as usize;
    let start = visible.len().saturating_sub(capacity);
    let lines = visible[start..]
        .iter()
        .map(|entry| {
            let color = match entry.level.to_ascii_lowercase().as_str() {
                "debug" => Color::DarkGray,
                "warning" | "warn" => Color::Yellow,
                "error" => Color::Red,
                _ => Color::White,
            };
            Line::from(vec![
                Span::styled(format!("[{:<7}] ", entry.level), Style::new().fg(color)),
                Span::raw(entry.payload.as_str()),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(" 实时日志 ", app.focus == FocusTarget::LogsStream))
            .wrap(Wrap { trim: false }),
        columns[0],
    );
    let controls = vec![
        Line::from(format!("级别：{}", app.log_level.as_str())),
        Line::from(format!(
            "状态：{}",
            if app.logs_paused {
                "已暂停"
            } else {
                "运行中"
            }
        )),
        Line::from(format!(
            "筛选：{}",
            if app.filter.is_empty() {
                "无"
            } else {
                &app.filter
            }
        )),
        Line::from(""),
        Line::styled("Space", Style::new().fg(ACCENT)),
        Line::from("暂停/继续"),
        Line::styled("l", Style::new().fg(ACCENT)),
        Line::from("切换级别"),
        Line::styled("c", Style::new().fg(ACCENT)),
        Line::from("清屏"),
    ];
    frame.render_widget(
        Paragraph::new(controls)
            .block(panel(
                " 筛选 / 日志操作 ",
                app.focus == FocusTarget::LogsControls,
            ))
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn render_settings(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Percentage(50),
            Constraint::Min(5),
        ])
        .split(area);
    let config = app.config.as_ref();
    let details = vec![
        Line::from(vec![
            Span::styled("控制地址：", Style::new().fg(DIM)),
            Span::raw(app.controller.as_str()),
        ]),
        Line::from(vec![
            Span::styled("运行模式：", Style::new().fg(DIM)),
            Span::styled(
                config.map(|c| c.mode.as_str()).unwrap_or("-"),
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   [m 切换 rule → global → direct]"),
        ]),
        Line::from(format!(
            "HTTP端口：{}   SOCKS端口：{}   混合端口：{}",
            config.map(|c| c.port).unwrap_or_default(),
            config.map(|c| c.socks_port).unwrap_or_default(),
            config.map(|c| c.mixed_port).unwrap_or_default()
        )),
        Line::from(format!(
            "写操作：{}",
            if app.read_only {
                "已禁用（只读）"
            } else {
                "已启用"
            }
        )),
    ];
    frame.render_widget(
        Paragraph::new(details).block(panel(
            " 控制端设置 [m切换模式] ",
            app.focus == FocusTarget::SettingsCore,
        )),
        sections[0],
    );

    let providers = app.provider_list();
    let provider_name_width = (sections[1].width.saturating_sub(2) * 30 / 100) as usize;
    let rows = providers.iter().enumerate().map(|(index, provider)| {
        let usage = provider
            .subscription_info
            .as_ref()
            .map(|info| {
                format!(
                    "{} / {}",
                    format_bytes(info.upload.saturating_add(info.download)),
                    format_bytes(info.total)
                )
            })
            .unwrap_or_else(|| "-".to_string());
        Row::new(vec![
            Cell::from(truncate_label(&provider.name, provider_name_width)),
            Cell::from(provider.vehicle_type.clone()),
            Cell::from(provider.proxies.len().to_string()),
            Cell::from(usage),
            Cell::from(provider.updated_at.clone()),
        ])
        .style(if index == app.provider_index {
            SELECTED
        } else {
            Style::default()
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(30),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Percentage(24),
                Constraint::Percentage(28),
            ],
        )
        .header(table_header([
            "Provider",
            "来源",
            "节点",
            "已用/总量",
            "更新时间",
        ]))
        .column_spacing(1)
        .block(panel(
            " 订阅 Provider [Enter/p刷新] ",
            app.focus == FocusTarget::SettingsProviders,
        )),
        sections[1],
    );

    let rule_providers = app.rule_provider_list();
    let rule_provider_name_width = (sections[2].width.saturating_sub(2) * 28 / 100) as usize;
    let rows = rule_providers.iter().enumerate().map(|(index, provider)| {
        Row::new(vec![
            Cell::from(truncate_label(&provider.name, rule_provider_name_width)),
            Cell::from(provider.behavior.clone()),
            Cell::from(provider.format.clone()),
            Cell::from(provider.rule_count.to_string()),
            Cell::from(if provider.size == 0 {
                "-".to_string()
            } else {
                format_bytes(provider.size)
            }),
            Cell::from(provider.updated_at.clone()),
        ])
        .style(if index == app.rule_provider_index {
            SELECTED
        } else {
            Style::default()
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(28),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Percentage(24),
            ],
        )
        .header(table_header([
            "规则 Provider",
            "行为",
            "格式",
            "规则数",
            "大小",
            "更新时间",
        ]))
        .column_spacing(1)
        .block(panel(
            " 规则 Provider [Enter/p刷新] ",
            app.focus == FocusTarget::SettingsRuleProviders,
        )),
        sections[2],
    );
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let filter = if app.editing_filter {
        format!("筛选输入：{}▌", app.filter)
    } else if !app.filter.is_empty() {
        format!("筛选：{}  ", app.filter)
    } else {
        String::new()
    };
    let pending = if app.pending > 0 {
        format!("任务:{}  ", app.pending)
    } else {
        String::new()
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(filter, Style::new().fg(Color::Yellow)),
            Span::styled(pending, Style::new().fg(Color::Yellow)),
            Span::raw("↑↓/jk选择  ←/→切换面板  Enter进入  /筛选  r刷新  ?帮助  q退出"),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::new().fg(DIM)),
        ),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let help = vec![
        Line::styled("导航", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Line::from("一级菜单：↑↓/jk 选择；Enter 进入当前页面面板"),
        Line::from("面板焦点：↑↓/jk 操作列表；←/→ 循环切换面板；Enter 进入下一个面板"),
        Line::from("q 退出；Esc 在帮助、确认框和筛选输入中仍用于关闭或取消"),
        Line::from(""),
        Line::styled("代理", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Line::from("代理内容：Enter 切换节点   t 节点测速   T 代理组测速"),
        Line::from(""),
        Line::styled("连接", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Line::from("d 关闭选中连接   D 关闭全部连接（需要确认）"),
        Line::from(""),
        Line::styled(
            "日志 / 设置",
            Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Line::from("Space 暂停日志   l 日志级别   c 清屏"),
        Line::from("m 切换运行模式   Enter/p 刷新选中的订阅或规则 Provider"),
        Line::from(""),
        Line::styled(
            if app.read_only {
                "当前为只读模式，所有写操作都会被阻止。"
            } else {
                "写操作已启用；关闭全部连接始终需要二次确认。"
            },
            Style::new().fg(Color::Yellow),
        ),
        Line::from("按 ?、Esc 或 q 关闭帮助"),
    ];
    let popup = centered_rect(76, 72, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(help)
            .block(
                Block::default()
                    .title(" 快捷键帮助 ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(ACCENT)),
            )
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn breadcrumb(app: &App) -> String {
    if app.focus == FocusTarget::RootMenu {
        app.page.title().to_string()
    } else {
        format!("{} / {}", app.page.title(), app.focus.label())
    }
}

fn render_dialog(frame: &mut Frame<'_>, area: Rect, title: &str, message: &str, color: Color) {
    let popup = centered_rect(58, 28, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(message).alignment(Alignment::Center),
            Line::from(""),
            Line::styled("Enter/y 确认    n/Esc 取消", Style::new().fg(DIM))
                .alignment(Alignment::Center),
        ])
        .block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::new().fg(color)),
        ),
        popup,
    );
}

fn render_toast(frame: &mut Frame<'_>, area: Rect, message: &str, kind: ToastKind) {
    let color = match kind {
        ToastKind::Info => Color::Cyan,
        ToastKind::Success => Color::Green,
        ToastKind::Warning => Color::Yellow,
        ToastKind::Error => Color::Red,
    };
    let width = (message.chars().count() as u16 + 6).clamp(24, area.width.saturating_sub(4));
    let popup = Rect {
        x: area.right().saturating_sub(width + 2),
        y: area.bottom().saturating_sub(5),
        width,
        height: 3,
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(message).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(color)),
        ),
        popup,
    );
}

fn metric_card(frame: &mut Frame<'_>, area: Rect, title: &str, value: &str, color: Color) {
    frame.render_widget(
        Paragraph::new(Span::styled(
            value,
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center)
        .block(panel(&format!(" {title} "), false)),
        area,
    );
}

fn render_sparkline(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    values: &std::collections::VecDeque<u64>,
    color: Color,
) {
    let values = values.iter().copied().collect::<Vec<_>>();
    let max = values.iter().copied().max().unwrap_or(1).max(1);
    frame.render_widget(
        Sparkline::default()
            .block(panel(title, false))
            .data(&values)
            .max(max)
            .style(Style::new().fg(color)),
        area,
    );
}

fn panel<'a>(title: &'a str, focused: bool) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(if focused { FOCUS_BORDER } else { DIM }))
        .title_style(Style::new().fg(if focused { FOCUS_TITLE } else { DIM }))
}

fn table_header<const N: usize>(titles: [&str; N]) -> Row<'static> {
    Row::new(
        titles
            .into_iter()
            .map(|title| Cell::from(title.to_string()))
            .collect::<Vec<_>>(),
    )
    .style(Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))
    .bottom_margin(1)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn truncate_label(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if value.width() <= max_width {
        value.to_string()
    } else if max_width == 1 {
        "…".to_string()
    } else {
        let mut visible = String::new();
        let mut width = 0;
        for ch in value.chars() {
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if width + char_width > max_width - 1 {
                break;
            }
            visible.push(ch);
            width += char_width;
        }
        format!("{visible}…")
    }
}

fn format_rate(bytes: u64) -> String {
    format!("{}/s", format_bytes(bytes))
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "是"
    } else {
        "否"
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::{format_bytes, render};
    use crate::app::{App, FocusTarget, Page};

    #[test]
    fn formats_binary_units() {
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
    }

    #[test]
    fn focused_panel_uses_green_border() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        let mut app = App::new("http://localhost:9090".to_string(), false);
        app.page = Page::Proxies;
        app.focus = FocusTarget::ProxyGroups;
        terminal.draw(|frame| render(frame, &app)).expect("render");

        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer.cell((22, 3)).map(|cell| cell.fg),
            Some(ratatui::style::Color::Green)
        );
        assert_eq!(
            buffer.cell((50, 3)).map(|cell| cell.fg),
            Some(ratatui::style::Color::DarkGray)
        );

        app.focus = FocusTarget::ProxyNodes;
        terminal.draw(|frame| render(frame, &app)).expect("render");
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer.cell((22, 3)).map(|cell| cell.fg),
            Some(ratatui::style::Color::DarkGray)
        );
        assert_eq!(
            buffer.cell((50, 3)).map(|cell| cell.fg),
            Some(ratatui::style::Color::Green)
        );
    }
}
