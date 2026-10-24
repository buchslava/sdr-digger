use std::time::{Duration, Instant};

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::UiPalette;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastKind {
    #[default]
    Info,
    Alert,
}

#[derive(Debug, Clone)]
pub struct TimedToast {
    pub until: Instant,
    pub message: String,
    pub kind: ToastKind,
}

impl TimedToast {
    pub fn with_kind(duration: Duration, message: String, kind: ToastKind) -> Self {
        Self {
            until: Instant::now() + duration,
            message,
            kind,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.until
    }

    pub fn clear_if_expired(slot: &mut Option<Self>) -> bool {
        if slot.as_ref().is_some_and(Self::is_expired) {
            *slot = None;
            true
        } else {
            false
        }
    }
}

pub fn draw_timed_bottom_left(
    f: &mut Frame,
    area: Rect,
    palette: &UiPalette,
    toast: &TimedToast,
) {
    const PAD: u16 = 1;
    let w = (toast.message.chars().count() as u16)
        .min(area.width.saturating_sub(PAD))
        .max(1);
    let row = area.bottom().saturating_sub(1);
    let rect = Rect {
        x: area.x + PAD,
        y: row,
        width: w,
        height: 1,
    };
    let style = match toast.kind {
        ToastKind::Info => Style::default().fg(palette.ok).bg(Color::Black),
        ToastKind::Alert => Style::default().fg(Color::White).bg(palette.alert),
    };
    f.render_widget(Paragraph::new(toast.message.as_str()).style(style), rect);
}
