use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::state::{AppMode, ConnectionState, DialogKind};
use crate::app::AppState;
use crate::dialogs::{
    case_browser, findings, help, inspector, save_case, scan_progress, school,
};
use crate::i18n::{t, Msg};
use crate::ui::{coach, spectrum, status_bar, toast, waterfall};
use crate::ui::theme::UiPalette;

pub struct Renderer;

impl Renderer {
    pub fn draw_ui(f: &mut Frame, app: &mut AppState) {
        let palette = UiPalette::digger();
        let area = f.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(area);

        draw_header(f, chunks[0], app, &palette);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
            .split(chunks[1]);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Percentage(40),
                Constraint::Length(1),
                Constraint::Percentage(60),
            ])
            .split(body[0]);

        spectrum::draw(f, left[0], left[1], app, &palette);
        waterfall::draw(f, left[2], left[3], app, &palette);
        coach::draw(f, body[1], app, &palette);

        draw_param_strip(f, chunks[2], app, &palette);
        status_bar::draw(f, chunks[3], app, &palette);

        if let Some(ref toast_msg) = app.timed_toast {
            toast::draw_timed_bottom_left(f, area, &palette, toast_msg);
        }

        if app.mode == AppMode::Dialog {
            if let Some(kind) = app.dialog {
                crate::ui::dialog_layout::paint_dim(f, area);
                match kind {
                    DialogKind::Cases => case_browser::draw(f, area, app, &palette),
                    DialogKind::Findings => findings::draw(f, area, app, &palette),
                    DialogKind::ScanProgress => scan_progress::draw(f, area, app, &palette),
                    DialogKind::Inspector => inspector::draw(f, area, app, &palette),
                    DialogKind::School => school::draw(f, area, app, &palette),
                    DialogKind::SaveCase => save_case::draw(f, area, app, &palette),
                    DialogKind::Help => help::draw(f, area, app, &palette),
                }
            }
        }
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let status = match &app.connection {
        ConnectionState::Connected if app.test_mode => t(app.lang, Msg::StatusMock),
        ConnectionState::Connected => t(app.lang, Msg::StatusConnected),
        ConnectionState::Disconnected { .. } => t(app.lang, Msg::StatusDisconnected),
    };
    let dirty = if app.case_dirty {
        format!(" [{}]", t(app.lang, Msg::DirtyCase))
    } else {
        String::new()
    };
    let mode = t(app.lang, app.case.voice.mode.msg());
    let mhz = app.case.tune.frequency_hz as f64 / 1e6;
    let name = app.case.display_name(app.lang);
    let band = app.band_label();
    let findings = if app.findings.is_empty() {
        String::new()
    } else {
        format!(" · {} {}", app.findings.len(), t(app.lang, Msg::FindingsTitle))
    };
    let title = format!(
        " {} · {name} · {mhz:.3} MHz · {mode} {}k · ● {status} · {band}{findings}{dirty} ",
        t(app.lang, Msg::AppTitle),
        app.case.window.channel_bw_hz / 1000
    );
    f.render_widget(
        Paragraph::new(title).style(palette.title_style().add_modifier(Modifier::BOLD)),
        area,
    );
}

fn draw_param_strip(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let groups = [
        Msg::GroupTune,
        Msg::GroupWindow,
        Msg::GroupStrength,
        Msg::GroupVoice,
        Msg::GroupReadout,
    ];
    let text = format!(
        " {}  |  {:.1} dB gain  |  PPM {:+.1}  |  SR {}  |  {}",
        groups
            .iter()
            .map(|m| t(app.lang, *m))
            .collect::<Vec<_>>()
            .join(" · "),
        app.case.strength.gain_db,
        app.case.tune.ppm,
        app.case.window.sample_rate_hz,
        t(app.lang, app.case.strength.gain.msg()),
    );
    f.render_widget(
        Paragraph::new(text).style(palette.dim_style()),
        area,
    );
}
