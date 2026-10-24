use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::events::AppAction;
use crate::app::state::{AppMode, AppState};
use crate::i18n::{t, Msg};
use crate::ui::dialog_layout::centered_dialog_rect;
use crate::ui::theme::UiPalette;

pub fn draw(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let rect = centered_dialog_rect(area, 56, 7);
    f.render_widget(Clear, rect);
    let text = format!(
        "{}\n\n{} → ~/.sdr-digger/cases/{}.toml\n\n{} · {}",
        t(app.lang, Msg::SaveCaseTitle),
        app.case.display_name(app.lang),
        app.case.slug,
        t(app.lang, Msg::EnterConfirm),
        t(app.lang, Msg::EscClose),
    );
    f.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(palette.accent_style())
                .title(format!(" {} ", t(app.lang, Msg::SaveCaseTitle))),
        ),
        rect,
    );
}

pub fn handle_key(app: &mut AppState, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc => {
            app.dialog = None;
            app.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            app.save_current_case();
            app.dialog = None;
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
    AppAction::Idle
}
