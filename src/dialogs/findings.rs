use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::events::AppAction;
use crate::app::state::{AppMode, AppState};
use crate::i18n::{t, Msg};
use crate::ui::dialog_layout::centered_dialog_rect;
use crate::ui::theme::UiPalette;

pub fn draw(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let rect = centered_dialog_rect(area, 72, 20);
    f.render_widget(Clear, rect);
    if app.findings.is_empty() {
        f.render_widget(
            Paragraph::new(t(app.lang, Msg::FindingsEmpty)).block(
                Block::default()
                    .title(format!(" {} ", t(app.lang, Msg::FindingsTitle)))
                    .borders(Borders::ALL),
            ),
            rect,
        );
        return;
    }
    let items: Vec<ListItem> = app
        .findings
        .iter()
        .map(|h| {
            let label = h.label.clone().unwrap_or_default();
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>10.3} MHz  ", h.frequency_hz as f64 / 1e6),
                    palette.accent_style(),
                ),
                Span::styled(format!("SNR {:>5.1} dB  ", h.snr_db), palette.text_style()),
                Span::styled(
                    format!("B={:.0} Hz  ", h.bandwidth_hz),
                    palette.dim_style(),
                ),
                Span::raw(label),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.finding_idx));
    let hint = format!(
        " {} · {} · {} · {} ",
        t(app.lang, Msg::FindingsApply),
        t(app.lang, Msg::FindingsBookmark),
        t(app.lang, Msg::FindingsInspect),
        t(app.lang, Msg::EscClose),
    );
    let block = Block::default()
        .title(format!(" {} ", t(app.lang, Msg::FindingsTitle)))
        .title_bottom(Line::from(hint))
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
        KeyCode::Up => {
            if app.finding_idx > 0 {
                app.finding_idx -= 1;
            }
        }
        KeyCode::Down => {
            if app.finding_idx + 1 < app.findings.len() {
                app.finding_idx += 1;
            }
        }
        KeyCode::Enter => {
            app.apply_finding(app.finding_idx);
        }
        KeyCode::Char('a') => {
            app.bookmark_current_finding();
        }
        KeyCode::Char('f') => {
            if let Some(hit) = app.findings.get(app.finding_idx) {
                app.case.tune.frequency_hz = hit.frequency_hz;
                app.after_detect = true;
                app.case_dirty = true;
                app.push_live_params();
                app.refresh_coach();
            }
            app.dialog = None;
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
    AppAction::Idle
}
