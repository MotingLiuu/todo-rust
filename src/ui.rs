use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

use crate::app::{App, State};

const OUTER_MARGIN: u16 = 2;
const TITLE_HEIGHT: u16 = 3;
const TIMER_HEIGHT: u16 = 7;
const GAUGE_HEIGHT: u16 = 3;
const STATUS_HEIGHT: u16 = 3;
const HELP_HEIGHT: u16 = 3;

/// 把 `Duration` 格式化成 `MM:SS` 字符串。
pub fn format_remaining(d: Duration) -> String {
    let total = d.as_secs();
    let mins = total / 60;
    let secs = total % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// 把整个可用区域切成 5 个垂直排布的矩形,顺序为:标题、倒计时、进度条、状态行、操作提示。
fn layout(area: Rect) -> [Rect; 5] {
    // ratatui 0.30 把 `Layout::split` 的结果改成 `Rc<[Rect]>` 以避免每帧拷贝;
    // 我们在边界做一次定长数组转换,把"恰好 5 块"这件事固化到类型上。
    Layout::default()
        .direction(Direction::Vertical)
        .margin(OUTER_MARGIN)
        .constraints([
            Constraint::Length(TITLE_HEIGHT),
            Constraint::Length(TIMER_HEIGHT),
            Constraint::Length(GAUGE_HEIGHT),
            Constraint::Length(STATUS_HEIGHT),
            Constraint::Length(HELP_HEIGHT),
        ])
        .split(area)
        .as_ref()
        .try_into()
        .expect("layout 始终产生 5 块区域")
}

/// 渲染整屏 UI。`ui` 本身只是编排:计算布局 + 顺序调用 5 个 `render_*` 子函数。
/// 每个子函数只知道自己负责的那一块怎么画,互不耦合。
pub fn ui(f: &mut Frame, app: &App) {
    let chunks = layout(f.area());
    render_title(f, chunks[0], app);
    render_timer(f, chunks[1], app);
    render_gauge(f, chunks[2], app);
    render_status(f, chunks[3], app);
    render_help(f, chunks[4]);
}

// 阶段标题
fn render_title(f: &mut Frame, area: Rect, app: &App) {
    let title = Paragraph::new(app.phase.label())
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(app.phase.color())
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default());
    f.render_widget(title, area);
}

// 倒计时数字
fn render_timer(f: &mut Frame, area: Rect, app: &App) {
    let timer = Paragraph::new(format_remaining(app.remaining()))
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(timer, area);
}

// 进度条
fn render_gauge(f: &mut Frame, area: Rect, app: &App) {
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("进度"))
        .gauge_style(
            Style::default()
                .fg(app.phase.color())
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .ratio(app.progress());
    f.render_widget(gauge, area);
}

// 状态行
fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let state_text = match app.state {
        State::Running => Span::styled("● 运行中", Style::default().fg(Color::Green)),
        State::Paused => Span::styled("‖ 已暂停", Style::default().fg(Color::Yellow)),
    };
    let stats = Line::from(vec![
        state_text,
        Span::raw("    "),
        Span::styled(
            format!("已完成 {} 个番茄", app.completed),
            Style::default().fg(Color::Gray),
        ),
    ]);
    f.render_widget(Paragraph::new(stats).alignment(Alignment::Center), area);
}

// 操作提示
fn render_help(f: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Space]", Style::default().fg(Color::Cyan)),
        Span::raw(" 开始/暂停  "),
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw(" 重置  "),
        Span::styled("[s]", Style::default().fg(Color::Cyan)),
        Span::raw(" 跳过  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" 退出"),
    ]))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    f.render_widget(help, area);
}
