use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::events::AppAction;
use crate::app::state::{AppMode, AppState};
use crate::i18n::{t, Msg};
use crate::ui::dialog_layout::centered_dialog_rect;
use crate::ui::theme::UiPalette;

pub fn draw(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let rect = centered_dialog_rect(area, 64, 18);
    f.render_widget(Clear, rect);
    let body = [
        t(app.lang, Msg::KeyStartStop),
        t(app.lang, Msg::KeyScan),
        t(app.lang, Msg::KeyCases),
        t(app.lang, Msg::KeyParams),
        t(app.lang, Msg::KeySave),
        t(app.lang, Msg::KeySchool),
        t(app.lang, Msg::KeyVfo),
        t(app.lang, Msg::KeyMute),
        t(app.lang, Msg::KeyOverlay),
        t(app.lang, Msg::KeyFindingsNext),
        t(app.lang, Msg::KeyFindingsPrev),
        "= PPM helper",
        t(app.lang, Msg::KeyQuit),
    ]
    .join("\n");
    f.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_style(palette.accent_style()),
            ),
        rect,
    );
}

pub fn handle_key(app: &mut AppState, code: KeyCode) -> AppAction {
    if matches!(
        code,
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('h') | KeyCode::Char('q')
    ) {
        app.dialog = None;
        app.mode = AppMode::Normal;
    }
    AppAction::Idle
}
