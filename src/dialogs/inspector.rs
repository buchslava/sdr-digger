use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::events::AppAction;
use crate::app::settings_apply::SettingChange;
use crate::app::state::{AppMode, AppState};
use crate::i18n::{t, Msg};
use crate::ui::dialog_layout::centered_dialog_rect;
use crate::ui::theme::UiPalette;

const SECTIONS: &[Msg] = &[
    Msg::GroupTune,
    Msg::GroupWindow,
    Msg::GroupStrength,
    Msg::GroupVoice,
    Msg::GroupReadout,
];

pub fn draw(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let rect = centered_dialog_rect(area, 76, 18);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .title(format!(" {} ", t(app.lang, Msg::InspectorTitle)))
        .borders(Borders::ALL)
        .border_style(palette.accent_style());
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(20)])
        .split(inner);

    let items: Vec<ListItem> = SECTIONS
        .iter()
        .map(|m| ListItem::new(t(app.lang, *m)))
        .collect();
    f.render_widget(
        List::new(items).highlight_style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        cols[0],
    );
    // Manual highlight line
    let y = cols[0].y + app.inspector_section as u16;
    if let Some(cell) = f.buffer_mut().cell_mut((cols[0].x, y)) {
        cell.set_style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        );
    }

    let body = section_body(app);
    f.render_widget(
        Paragraph::new(body).style(palette.text_style()),
        cols[1],
    );
}

fn section_body(app: &AppState) -> Vec<Line<'static>> {
    let lang = app.lang;
    match app.inspector_section {
        0 => vec![
            Line::from(format!(
                "freq  {:.6} MHz   ←→ nudge  f digit",
                app.case.tune.frequency_hz as f64 / 1e6
            )),
            Line::from(format!("ppm   {:+.1}   ←→ / = helper", app.case.tune.ppm)),
            Line::from(format!("raster {} Hz", app.case.tune.raster_hz)),
            Line::from(Span::styled(
                t(lang, Msg::PpmHelperHint).to_string(),
                Style::default(),
            )),
        ],
        1 => vec![
            Line::from(format!(
                "sample rate  {} Hz   ←→",
                app.case.window.sample_rate_hz
            )),
            Line::from(format!(
                "channel bw   {} Hz   ←→",
                app.case.window.channel_bw_hz
            )),
            Line::from(t(lang, Msg::ConstraintNyquist).to_string()),
        ],
        2 => vec![
            Line::from(format!(
                "gain mode  {}   Enter toggle",
                t(lang, app.case.strength.gain.msg())
            )),
            Line::from(format!("gain      {:.1} dB   ←→", app.case.strength.gain_db)),
        ],
        3 => vec![
            Line::from(format!(
                "mode       {}   ←→",
                t(lang, app.case.voice.mode.msg())
            )),
            Line::from(format!("squelch    {:.0} dB   ←→", app.case.voice.squelch_db)),
            Line::from(format!(
                "de-emphasis {} µs   Enter cycle",
                app.case.voice.deemphasis_us
            )),
            Line::from(format!("cw pitch   {:.0} Hz", app.case.voice.cw_pitch_hz)),
        ],
        _ => vec![
            Line::from(format!(
                "audio  {}   Enter",
                if app.case.readout.audio { "on" } else { "off" }
            )),
            Line::from(format!(
                "morse  {}   Enter",
                if app.case.readout.morse { "on" } else { "off" }
            )),
        ],
    }
}

pub fn handle_key(app: &mut AppState, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc => {
            app.dialog = None;
            app.mode = AppMode::Normal;
            AppAction::Idle
        }
        KeyCode::Up => {
            if app.inspector_section > 0 {
                app.inspector_section -= 1;
            }
            AppAction::Idle
        }
        KeyCode::Down | KeyCode::Tab => {
            if app.inspector_section + 1 < SECTIONS.len() {
                app.inspector_section += 1;
            }
            AppAction::Idle
        }
        KeyCode::Left => AppAction::SettingChange(cycle_left(app.inspector_section)),
        KeyCode::Right => AppAction::SettingChange(cycle_right(app.inspector_section)),
        KeyCode::Enter => AppAction::SettingChange(cycle_enter(app.inspector_section)),
        _ => AppAction::Idle,
    }
}

fn cycle_left(section: usize) -> SettingChange {
    match section {
        0 => SettingChange::PpmStep(-1.0),
        1 => SettingChange::SampleRateCycle(-1),
        2 => SettingChange::GainCycle(-1),
        3 => SettingChange::ModeCycle(-1),
        _ => SettingChange::AudioToggle,
    }
}

fn cycle_right(section: usize) -> SettingChange {
    match section {
        0 => SettingChange::PpmStep(1.0),
        1 => SettingChange::SampleRateCycle(1),
        2 => SettingChange::GainCycle(1),
        3 => SettingChange::SquelchStep(1),
        _ => SettingChange::MorseToggle,
    }
}

fn cycle_enter(section: usize) -> SettingChange {
    match section {
        2 => SettingChange::GainModeToggle,
        3 => SettingChange::DeemphasisCycle,
        4 => SettingChange::MorseToggle,
        1 => SettingChange::ChannelBwCycle(5),
        _ => SettingChange::PpmStep(0.0),
    }
}
