use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod ui;

use crate::app::App;

/// 事件循环的轮询/重绘节拍。`run_app` 既用它给 `event::poll` 算 timeout,
/// 也用它决定是否触发 `App::on_tick`,所以输入响应和倒计时刷新走的是同一条循环。
const TICK_RATE: Duration = Duration::from_millis(200);

fn main() -> io::Result<()> {
    // 进入原始模式 + 备用屏幕,接管整块终端作为渲染面。
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    // 反向恢复,任何提前 return 也要执行到底。
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

// 固定为 CrosstermBackend<Stdout>,避免引入 io::Error: From<B::Error> 约束
fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        // 给 `event::poll` 的 timeout:如果上次 tick 还没到 TICK_RATE,
        // 就只等到差值那段时间,顺便缩短输入响应延迟。
        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // 忽略键释放事件,避免按住时重复触发 toggle/reset/skip
                if key.is_release() {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char(' ') => app.toggle_pause(),
                    KeyCode::Char('r') | KeyCode::Char('R') => app.reset(),
                    KeyCode::Char('s') | KeyCode::Char('S') => app.skip(),
                    _ => {}
                }
            }
        }

        // on_tick 频率由 poll 超时决定,不严格等于 TICK_RATE;on_tick 只检查 remaining().is_zero()
        if last_tick.elapsed() >= TICK_RATE {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}
