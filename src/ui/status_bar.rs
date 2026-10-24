use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::state::{AppMode, DialogKind};
use crate::app::AppState;
use crate::i18n::{t, Msg};
use crate::ui::theme::UiPalette;

pub fn draw(frame: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let hints = context_hints(app);
    let text = fit_hints(&hints, area.width as usize);
    frame.render_widget(
        Paragraph::new(text).style(palette.dim_style().bg(palette.header_bg)),
        area,
    );
}

/// Keys that actually do something in the current mode. Hidden actions are
/// omitted so the bar never advertises a dead key.
fn context_hints(app: &AppState) -> Vec<&'static str> {
    let lang = app.lang;
    match app.mode {
        AppMode::EditingFrequency => vec![
            t(lang, Msg::KeyVfoKeys),
            t(lang, Msg::EscClose),
        ],
        AppMode::Dialog => match app.dialog {
            Some(DialogKind::Cases) => vec![
                t(lang, Msg::KeyNav),
                t(lang, Msg::EnterConfirm),
                t(lang, Msg::EscClose),
            ],
            Some(DialogKind::Findings) => vec![
                t(lang, Msg::KeyNav),
                t(lang, Msg::FindingsApply),
                t(lang, Msg::FindingsBookmark),
                t(lang, Msg::FindingsInspect),
                t(lang, Msg::EscClose),
            ],
            Some(DialogKind::ScanProgress) => vec![t(lang, Msg::EscClose)],
            Some(DialogKind::Inspector) => vec![
                t(lang, Msg::KeyNav),
                t(lang, Msg::KeyPage),
                t(lang, Msg::EnterConfirm),
                t(lang, Msg::EscClose),
            ],
            Some(DialogKind::School) => vec![
                t(lang, Msg::KeyPage),
                t(lang, Msg::EscClose),
            ],
            Some(DialogKind::SaveCase) => vec![
                t(lang, Msg::EnterConfirm),
                t(lang, Msg::EscClose),
            ],
            Some(DialogKind::Help) | None => vec![t(lang, Msg::EscClose)],
        },
        AppMode::Normal => {
            let mut keys = vec![
                t(lang, Msg::KeyStartStop),
                t(lang, Msg::KeyScan),
                t(lang, Msg::KeyCases),
                t(lang, Msg::KeyParams),
                t(lang, Msg::KeySave),
                t(lang, Msg::KeySchool),
                t(lang, Msg::KeyVfo),
                t(lang, Msg::KeyMute),
                t(lang, Msg::KeyOverlay),
            ];
            if !app.findings.is_empty() {
                keys.push(t(lang, Msg::KeyFindingsNext));
                keys.push(t(lang, Msg::KeyFindingsPrev));
            }
            keys.push(t(lang, Msg::KeyPpm));
            keys.push(t(lang, Msg::KeyQuit));
            keys
        }
    }
}

fn fit_hints(hints: &[&str], width: usize) -> String {
    if width == 0 || hints.is_empty() {
        return String::new();
    }
    let mut used = 0usize;
    let mut out = String::new();
    for hint in hints {
        let extra = hint.chars().count() + if out.is_empty() { 0 } else { 2 };
        if used + extra > width {
            break;
        }
        if !out.is_empty() {
            out.push_str("  ");
        }
        out.push_str(hint);
        used += extra;
    }
    out
}
