use std::time::{Duration, Instant};

use ratatui::style::Color;

pub(crate) const WORK_DURATION: Duration = Duration::from_secs(25 * 60);
pub(crate) const SHORT_BREAK: Duration = Duration::from_secs(5 * 60);
pub(crate) const LONG_BREAK: Duration = Duration::from_secs(15 * 60);
pub(crate) const POMODOROS_BEFORE_LONG_BREAK: u32 = 4;

/// 番茄钟的阶段
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn total(&self) -> Duration {
        match self {
            Phase::Work => WORK_DURATION,
            Phase::ShortBreak => SHORT_BREAK,
            Phase::LongBreak => LONG_BREAK,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Phase::Work => "🍅 工作时间",
            Phase::ShortBreak => "☕ 短休息",
            Phase::LongBreak => "🌿 长休息",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Phase::Work => Color::Red,
            Phase::ShortBreak => Color::Green,
            Phase::LongBreak => Color::Cyan,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Running,
    Paused,
}

/// 应用状态:持有当前阶段、计时基准、已完成番茄数。
pub struct App {
    pub phase: Phase,
    pub state: State,
    /// 当前阶段开始时(或最近一次恢复时)的 Instant
    pub phase_started_at: Instant,
    /// 暂停开始时刻,None 表示当前不在暂停
    pub pause_started_at: Option<Instant>,
    /// 已完成的番茄数(只统计工作阶段)
    pub completed: u32,
}

impl App {
    pub fn new() -> Self {
        Self {
            phase: Phase::Work,
            state: State::Running,
            phase_started_at: Instant::now(),
            pause_started_at: None,
            completed: 0,
        }
    }

    /// 当前进度(0.0 - 1.0),表示已经消耗的比例
    pub fn progress(&self) -> f64 {
        let total = self.phase.total().as_secs_f64();
        if total <= 0.0 {
            return 0.0;
        }
        let elapsed = self.elapsed().as_secs_f64();
        (elapsed / total).clamp(0.0, 1.0)
    }

    /// 当前阶段已消耗的时长(暂停时间不计入)
    pub fn elapsed(&self) -> Duration {
        let now = self.now();
        let effective_now = match self.pause_started_at {
            // 暂停时把"现在"夹到 pause_started_at,使显示冻结
            Some(p) => p,
            None => now,
        };
        effective_now.duration_since(self.phase_started_at)
    }

    /// 当前显示应该使用的时间基准
    pub(crate) fn now(&self) -> Instant {
        Instant::now()
    }

    pub fn remaining(&self) -> Duration {
        self.phase.total().saturating_sub(self.elapsed())
    }

    /// 每帧更新逻辑
    pub fn on_tick(&mut self) {
        if self.state == State::Paused {
            return;
        }
        if self.remaining().is_zero() {
            self.advance();
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            State::Running => {
                self.pause_started_at = Some(self.now());
                self.state = State::Paused;
            }
            State::Paused => {
                if let Some(start) = self.pause_started_at.take() {
                    // 把暂停时长累加回 phase_started_at,使 elapsed() = now - phase_started_at 仍成立,无需累加 delta
                    let paused_for = self.now().duration_since(start);
                    self.phase_started_at += paused_for;
                }
                self.state = State::Running;
            }
        }
    }

    pub fn reset(&mut self) {
        self.phase_started_at = self.now();
        self.pause_started_at = None;
        self.state = State::Paused;
    }

    pub fn skip(&mut self) {
        self.advance();
    }

    /// 推进到下一个阶段(无论是否暂停)
    pub fn advance(&mut self) {
        match self.phase {
            Phase::Work => {
                self.completed += 1;
                // 每完成 4 个工作阶段后进入长休;completed 不在长休后清零,规则本身即保证轮转
                if self.completed % POMODOROS_BEFORE_LONG_BREAK == 0 {
                    self.phase = Phase::LongBreak;
                } else {
                    self.phase = Phase::ShortBreak;
                }
            }
            Phase::ShortBreak | Phase::LongBreak => {
                self.phase = Phase::Work;
            }
        }
        self.phase_started_at = self.now();
        self.pause_started_at = None;
        self.state = State::Running;
    }
}
