use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::events::AppAction;
use crate::app::state::{AppMode, AppState};
use crate::i18n::{t, Lang, Msg};
use crate::ui::dialog_layout::centered_dialog_rect;
use crate::ui::theme::UiPalette;

pub fn draw(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let rect = centered_dialog_rect(area, 70, 22);
    f.render_widget(Clear, rect);
    let items: Vec<ListItem> = app
        .cases
        .iter()
        .map(|c| {
            let diff = t(app.lang, c.difficulty.msg());
            let fam = t(app.lang, c.family.msg());
            let name = c.display_name(app.lang);
            ListItem::new(Line::from(vec![
                Span::styled(format!("{name:<28}"), palette.text_style()),
                Span::styled(format!(" {fam:<16}"), palette.dim_style()),
                Span::styled(diff, palette.accent_style()),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.case_browser_idx));
    let block = Block::default()
        .title(format!(" {} ", t(app.lang, Msg::CaseBrowserTitle)))
        .borders(Borders::ALL)
        .border_style(palette.accent_style());
    f.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> "),
        rect,
        &mut state,
    );
}

pub fn handle_key(app: &mut AppState, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc => {
            app.dialog = None;
            app.mode = AppMode::Normal;
        }
        KeyCode::Up | KeyCode::BackTab => {
            if app.case_browser_idx > 0 {
                app.case_browser_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Tab => {
            if app.case_browser_idx + 1 < app.cases.len() {
                app.case_browser_idx += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(c) = app.cases.get(app.case_browser_idx) {
                let slug = c.slug.clone();
                app.load_case_slug(&slug);
            }
            app.dialog = None;
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
    AppAction::Idle
}

#[allow(dead_code)]
fn _lang_hint(_: Lang) {}
