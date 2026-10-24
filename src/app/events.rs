use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::settings_apply::SettingChange;
use crate::app::state::{AppMode, AppState, DialogKind};
use crate::dialogs::{
    case_browser, findings, help, inspector, save_case, scan_progress, school,
};
use crate::radio::RadioMode;

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    /// Poll timed out with no input. Do not treat this as a UI change.
    Idle,
    /// A key was handled (or the terminal resized). Redraw even if the
    /// device is stopped — dialogs otherwise look frozen.
    Redraw,
    Quit,
    SettingChange(SettingChange),
}

pub struct EventHandler;

impl EventHandler {
    pub fn handle_events(
        app: &mut AppState,
        poll_timeout: std::time::Duration,
    ) -> std::io::Result<AppAction> {
        let mut handled = false;
        // Drain whatever is already queued so a burst of repeats cannot
        // sit behind a single-key-per-frame loop and feel stuck.
        for _ in 0..32 {
            if !event::poll(std::time::Duration::ZERO)? {
                break;
            }
            if Self::read_one(app, &mut handled)? {
                return Ok(AppAction::Quit);
            }
        }
        if !handled {
            if !event::poll(poll_timeout)? {
                return Ok(AppAction::Idle);
            }
            if Self::read_one(app, &mut handled)? {
                return Ok(AppAction::Quit);
            }
        }
        if handled {
            Ok(AppAction::Redraw)
        } else {
            Ok(AppAction::Idle)
        }
    }

    /// Returns true when the user asked to quit.
    fn read_one(app: &mut AppState, handled: &mut bool) -> std::io::Result<bool> {
        let ev = event::read()?;
        match ev {
            Event::Key(key)
                if matches!(
                    key.kind,
                    KeyEventKind::Press | KeyEventKind::Repeat
                ) =>
            {
                *handled = true;
                let action = Self::dispatch_key(app, key.code, key.modifiers);
                if matches!(action, AppAction::Quit) {
                    return Ok(true);
                }
                if let AppAction::SettingChange(change) = action {
                    super::apply_setting(app, change);
                }
            }
            Event::Resize(_, _) => {
                *handled = true;
            }
            _ => {}
        }
        Ok(false)
    }

    fn dispatch_key(
        app: &mut AppState,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> AppAction {
        if app.mode == AppMode::Dialog {
            if let Some(kind) = app.dialog {
                return match kind {
                    DialogKind::Cases => case_browser::handle_key(app, code),
                    DialogKind::Findings => findings::handle_key(app, code),
                    DialogKind::ScanProgress => scan_progress::handle_key(app, code),
                    DialogKind::Inspector => inspector::handle_key(app, code),
                    DialogKind::School => school::handle_key(app, code),
                    DialogKind::SaveCase => save_case::handle_key(app, code),
                    DialogKind::Help => help::handle_key(app, code),
                };
            }
        }

        if app.mode == AppMode::EditingFrequency {
            return Self::handle_vfo(app, code);
        }

        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => AppAction::Quit,
            KeyCode::Char(' ') => {
                app.toggle_stream();
                AppAction::Idle
            }
            KeyCode::Char('g') | KeyCode::F(5) => {
                app.start_scan();
                AppAction::Idle
            }
            KeyCode::Char('c') => {
                app.cases = crate::radio::list_cases();
                app.case_browser_idx = app
                    .cases
                    .iter()
                    .position(|c| c.slug == app.case.slug)
                    .unwrap_or(0);
                app.dialog = Some(DialogKind::Cases);
                app.mode = AppMode::Dialog;
                AppAction::Idle
            }
            KeyCode::Char('p') | KeyCode::F(9) => {
                app.dialog = Some(DialogKind::Inspector);
                app.mode = AppMode::Dialog;
                AppAction::Idle
            }
            KeyCode::Char('w') => {
                app.dialog = Some(DialogKind::SaveCase);
                app.mode = AppMode::Dialog;
                AppAction::Idle
            }
            KeyCode::F(1) => {
                app.dialog = Some(DialogKind::School);
                app.mode = AppMode::Dialog;
                AppAction::Idle
            }
            KeyCode::Char('h') | KeyCode::Char('?') => {
                app.dialog = Some(DialogKind::Help);
                app.mode = AppMode::Dialog;
                AppAction::Idle
            }
            KeyCode::Char('f') => {
                app.mode = AppMode::EditingFrequency;
                AppAction::Idle
            }
            KeyCode::Char('d') => {
                app.toggle_mute();
                AppAction::Idle
            }
            KeyCode::Char('o') => {
                app.show_overlay = !app.show_overlay;
                AppAction::Idle
            }
            KeyCode::Char('n') if !modifiers.contains(KeyModifiers::SHIFT) => {
                app.next_finding(true);
                AppAction::Idle
            }
            KeyCode::Char('N') => {
                app.next_finding(false);
                AppAction::Idle
            }
            KeyCode::Char('=') => {
                app.apply_ppm_from_peak();
                AppAction::Idle
            }
            KeyCode::Left => {
                app.nudge_freq(-1);
                AppAction::Idle
            }
            KeyCode::Right => {
                app.nudge_freq(1);
                AppAction::Idle
            }
            KeyCode::Char('[') => {
                let mut m = app.case.voice.mode.prev();
                while m != RadioMode::Off && !mode_ok_for_family(m, app.case.family) {
                    m = m.prev();
                }
                if m != RadioMode::Off {
                    app.last_radio_mode = m;
                }
                app.case.voice.mode = m;
                app.case_dirty = true;
                app.push_live_params();
                AppAction::Idle
            }
            KeyCode::Char(']') => {
                let mut m = app.case.voice.mode.next();
                while m != RadioMode::Off && !mode_ok_for_family(m, app.case.family) {
                    m = m.next();
                }
                if m != RadioMode::Off {
                    app.last_radio_mode = m;
                }
                app.case.voice.mode = m;
                app.case_dirty = true;
                app.push_live_params();
                AppAction::Idle
            }
            _ => AppAction::Idle,
        }
    }

    fn handle_vfo(app: &mut AppState, code: KeyCode) -> AppAction {
        match code {
            KeyCode::Esc | KeyCode::Enter => {
                app.mode = AppMode::Normal;
            }
            KeyCode::Left => {
                app.vfo_cursor = (app.vfo_cursor + 1).min(crate::app::state::VFO_DIGITS - 1);
            }
            KeyCode::Right => {
                app.vfo_cursor = app.vfo_cursor.saturating_sub(1);
            }
            KeyCode::Up => app.adjust_vfo_digit(1),
            KeyCode::Down => app.adjust_vfo_digit(-1),
            KeyCode::Char('z') => {
                let place = app.vfo_digit_value();
                app.case.tune.frequency_hz = (app.case.tune.frequency_hz / place) * place;
                app.case_dirty = true;
                app.push_live_params();
            }
            _ => {}
        }
        AppAction::Idle
    }
}

fn mode_ok_for_family(mode: RadioMode, family: crate::radio::Family) -> bool {
    use crate::radio::Family;
    match family {
        Family::BroadcastFm => matches!(mode, RadioMode::Wbfm | RadioMode::Off),
        Family::Airband => matches!(mode, RadioMode::Am | RadioMode::Off),
        Family::Marine | Family::WeatherSat | Family::AmateurFm => {
            matches!(mode, RadioMode::Nbfm | RadioMode::Off)
        }
        Family::AmateurSsb => matches!(mode, RadioMode::Usb | RadioMode::Lsb | RadioMode::Off),
        Family::AmateurCw => matches!(mode, RadioMode::Cw | RadioMode::Off),
        Family::Digital => matches!(mode, RadioMode::Off),
        Family::Lab => true,
    }
}
