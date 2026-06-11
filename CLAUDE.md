# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

A terminal Pomodoro timer TUI written in Rust. Three source files under `src/`:

- [src/main.rs](src/main.rs) — terminal lifecycle + event loop (~70 lines).
- [src/app.rs](src/app.rs) — data model: `Phase`, `State`, `App` + time-tracking methods.
- [src/ui.rs](src/ui.rs) — layout, `format_remaining`, `ui()` orchestrator, five `render_*` helpers.

Depends on `ratatui = "0.30"` and `crossterm = "0.29"` only. No tests, no `[features]`, no `dev-dependencies`.

## Commands

```bash
cargo build            # debug build
cargo build --release  # release build
cargo run              # build + launch the TUI
cargo run --release    # release build + launch

# Smoke-test the TUI in this sandbox (which has no real TTY):
script -q /dev/null ./target/release/todo-rust </dev/null &
sleep 1 && kill %1
# Expect: ANSI codes for EnterAlternateScreen, EnableMouseCapture, hide-cursor; exit 143.
```

`cargo` warnings/errors are the only signal to watch — no clippy/rustfmt config, no test runner.

## Module map

- **[src/main.rs](src/main.rs)** — owns the terminal handle. `fn main` does enable/disable raw mode + alternate screen + mouse capture. `fn run_app` is the single draw/poll/tick loop. The only data-model import is `App`; the only render call is `ui::ui(f, app)`. The `TICK_RATE` constant lives here because it is the loop's local pacing parameter.
- **[src/app.rs](src/app.rs)** — owns the timer state machine. `Phase` and `State` are the only enums. `App` is the single source of truth: every visible UI fact (`remaining`, `progress`, current `state`, `completed`) is *derived* from `phase`, `phase_started_at`, `pause_started_at`, `completed` — no cached fields.
- **[src/ui.rs](src/ui.rs)** — owns the rendering. Pure function of `&App`; never holds state, never mutates. Layout sizes (`OUTER_MARGIN`, `TITLE_HEIGHT`, etc.) are the only constants here because they describe the rendering grid, not the timer logic.

## Architecture

### Data model ([src/app.rs](src/app.rs))

- `Phase::{Work, ShortBreak, LongBreak}` — variant carries its own duration via `Phase::total()`.
- `State::{Running, Paused}` — orthogonal to phase. A `Running` work phase that reaches zero flips to a `Running` break; `Paused` is the user-controlled brake.
- `App` fields: `phase`, `state`, `phase_started_at: Instant`, `pause_started_at: Option<Instant>`, `completed: u32`. There is **no** `remaining` field — it is always recomputed from `elapsed()`.
- Public surface used by `ui.rs`: `phase`, `state`, `completed` (fields), plus `elapsed()`, `remaining()`, `progress()` (reads), plus `toggle_pause()`, `reset()`, `skip()`, `advance()`, `on_tick()` (writes; not called from `ui.rs` but exposed for `main.rs`).

### Event loop ([src/main.rs](src/main.rs) `run_app`)

A single `loop` that interleaves draw, input, and tick:

1. `terminal.draw(|f| ui::ui(f, app))` — repaint.
2. `let timeout = TICK_RATE - last_tick.elapsed()` — non-negative deadline for the next `event::poll`. Shorter than `TICK_RATE` after a recent draw, so input response is bounded by redraw rate.
3. `event::poll(timeout)` blocks up to `timeout`. If an event is ready, read it and handle keys (`q`/`Esc` → quit; `Space` → toggle pause; `r`/`R` → reset; `s`/`S` → skip). Release events are filtered.
4. If `last_tick.elapsed() >= TICK_RATE`, call `app.on_tick()` and reset `last_tick`. Note: `on_tick` is *not* tick-aligned — its real cadence is whatever survives the `poll` deadline.

### Render layer ([src/ui.rs](src/ui.rs))

- `ui(f, app)` is a 5-line orchestrator: `let chunks = layout(f.area()); render_title(...); render_timer(...); render_gauge(...); render_status(...); render_help(...);`.
- `layout(area) -> [Rect; 5]` builds the vertical `Layout` and converts the `Rc<[Rect]>` returned by ratatui 0.30 into a fixed `[Rect; 5]` so the chunk count is enforced at the type level. The `expect("layout 始终产生 5 块区域")` would fire only if the constraints array and the return type drift apart.
- Each `render_*` helper is the only place that knows the widget details for its row.
- `format_remaining(Duration) -> String` produces the `MM:SS` string used by `render_timer`.

## Input handling ([src/main.rs](src/main.rs))

| Key | Action | Notes |
|-----|--------|-------|
| `q` / `Esc` | Quit | The only key that returns from `run_app`. |
| `Space` | Toggle pause | Sets/clears `pause_started_at`; see "Time tracking" below. |
| `r` / `R` | Reset current phase | Stops the clock (`state = Paused`) and rewinds `phase_started_at` to `now`. |
| `s` / `S` | Skip to next phase | Calls `App::advance()`, which forces `state = Running` (asymmetric to `r`). |

`key.is_release()` is filtered: crossterm reports both press and release events on terminals that support it; without the filter, holding `Space` would fire `toggle_pause` on every repeat and visibly flicker between paused and running.

## Time tracking ([src/app.rs](src/app.rs))

The whole timer is the formula `remaining = phase.total() - elapsed()`, where `elapsed` is the time since `phase_started_at`. Two invariants keep this formula correct under pause and under render lag:

- **Pause uses time-shifting, not delta accumulation.** `toggle_pause` writes `pause_started_at` on pause, and on resume does `phase_started_at += now - pause_started_at`. Because `elapsed = now - phase_started_at`, this holds `elapsed` constant across the pause without ever summing per-tick deltas. Skipped ticks or render lag cannot drift the clock.
- **`elapsed()` freezes the "now" pointer while paused.** Inside `elapsed`, `match pause_started_at { Some(p) => p, None => now }` rewrites the time source so the getter returns the same value on every call during a pause. This is what makes the displayed timer stop counting down — no state flag inside the renderer is needed.

The `App::now()` wrapper is currently a trivial passthrough to `Instant::now()`. Its purpose is to leave a single seam for future clock injection (so the time math above could be unit-tested without sleeping). It is `pub(crate)` for that reason.

## Render layer — non-obvious bits ([src/ui.rs](src/ui.rs))

- The layout is `[3, 7, 3, 3, 3]` rows with a 2-cell outer margin. The 7-row timer chunk is taller than the text it contains so the centered `MM:SS` reads at a comfortable vertical position; do not "fix" it to 3 without checking the look.
- `render_status` matches on `app.state` to pick a `Span` color (`Green` for running, `Yellow` for paused). The "已完成 N 个番茄" suffix is `Gray` so it visually recedes.
- The help line colors every bracketed key `[Space] [r] [s] [q]` in `Cyan` to distinguish keys from the action labels next to them.

## Non-obvious design decisions

- **Pause is implemented by time-shifting, not by accumulating a delta.** `toggle_pause` writes `pause_started_at` on pause, then on resume does `phase_started_at += now - pause_started_at`. This keeps `remaining = total - (now - phase_started_at)` correct without ever summing tick deltas, so render lag or skipped ticks can't drift the clock.
- **`elapsed()` / `remaining()` freeze at the pause instant while paused** — the timer is read by `ui`, so freezing in the getters is what makes the display stop counting down.
- **Auto-advance lives in `on_tick`**, not in the UI. When `remaining()` hits zero, `App::advance()` flips the phase, resets `phase_started_at`, and forces `state = Running`. The `r` / `s` keys reuse `reset` / `advance`.
- **Long-break cadence**: every 4th completed `Work` phase goes to `LongBreak` (15 min); the other 3 go to `ShortBreak` (5 min). Counter resets implicitly because the rule is `completed % 4 == 0` — there is no "reset on long break" code path. If you want the counter to reset after a long break, that's a behavior change, not a bug fix.
- **`run_app` is not generic over `Backend`.** The signature fixes `Terminal<CrosstermBackend<io::Stdout>>` to avoid a `where io::Error: From<B::Error>` bound on the `Result`. The cost is the binary cannot be retargeted at, say, `TermionBackend` without editing `main.rs`; for a single-binary TUI that's the right trade.
- **Mouse capture is enabled but not handled.** `EnableMouseCapture` / `DisableMouseCapture` are paired symmetrically; no mouse events are read in `run_app`. This is harmless (mouse events arrive as `Event::Mouse`, which the `if let Event::Key` arm ignores) and was left in for symmetry. If the file ever grows a `Click` handler, the plumbing is already there.

## Visibility cheat sheet

- `Phase`, `State`, and all `App` fields/methods are `pub` because they cross the `app.rs` → `ui.rs` / `main.rs` boundary.
- `App::now()` is `pub(crate)` — only called inside `app.rs`. It's a clock-injection seam that no external module needs today.
- Duration constants (`WORK_DURATION`, `SHORT_BREAK`, `LONG_BREAK`, `POMODOROS_BEFORE_LONG_BREAK`) and layout constants (`OUTER_MARGIN`, `*_HEIGHT`) are `pub(crate)` / module-private. They are colocated with their only consumer.
- The five `render_*` helpers in `ui.rs` are private — they are an internal decomposition of `ui`, not a public surface.

## Testing caveats

- The TUI needs a real TTY. Running the binary in this sandbox fails with `Os { code: 6, kind: "Device not configured" }` — that's an environment limitation, not a bug. The `script(1)` smoke-test in "Commands" works around it.
- There are no unit tests. Time math (`elapsed` / `remaining` / pause-resume) is the only piece with non-trivial logic; if you add tests, target that first. To make it testable, the natural next step is a `Clock` trait that `App::now()` consults instead of calling `Instant::now()` directly — the seam is already in place.
