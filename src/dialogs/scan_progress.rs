use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph};
use ratatui::Frame;

use crate::app::events::AppAction;
use crate::app::state::{AppMode, AppState};
use crate::detect::ScanPhase;
use crate::i18n::{t, Msg};
use crate::ui::dialog_layout::centered_dialog_rect;
use crate::ui::theme::UiPalette;

pub fn draw(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let rect = centered_dialog_rect(area, 50, 8);
    f.render_widget(Clear, rect);
    let (ratio, label) = if let Some(ref p) = app.scan_progress {
        let phase = match p.phase {
            ScanPhase::Spectrum => t(app.lang, Msg::ScanPhaseSpectrum),
            ScanPhase::Identify => t(app.lang, Msg::ScanPhaseIdentify),
            ScanPhase::Done => t(app.lang, Msg::ScanPhaseDone),
        };
        let r = if p.total == 0 {
            0.0
        } else {
            p.current as f64 / p.total as f64
        };
        (
            r,
            format!("{phase}  {}/{}  {:.3} MHz", p.current, p.total, p.hz / 1e6),
        )
    } else {
        (0.0, "…".into())
    };
    let block = Block::default()
        .title(format!(" {} ", t(app.lang, Msg::ScanProgressTitle)))
        .borders(Borders::ALL)
        .border_style(palette.accent_style());
    f.render_widget(block, rect);
    let inner = Rect {
        x: rect.x + 2,
        y: rect.y + 2,
        width: rect.width.saturating_sub(4),
        height: 1,
    };
    f.render_widget(
        Gauge::default()
            .ratio(ratio.clamp(0.0, 1.0))
            .label(label.clone())
            .gauge_style(palette.accent_style()),
        inner,
    );
    f.render_widget(
        Paragraph::new(t(app.lang, Msg::EscClose)).style(palette.dim_style()),
        Rect {
            x: rect.x + 2,
            y: rect.y + 4,
            width: rect.width.saturating_sub(4),
            height: 1,
        },
    );
}

pub fn handle_key(app: &mut AppState, code: KeyCode) -> AppAction {
    if code == KeyCode::Esc {
        if let Some(q) = app.scan_quit.take() {
            q.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        app.dialog = None;
        app.mode = AppMode::Normal;
        app.connect_request = true;
    }
    AppAction::Idle
}
