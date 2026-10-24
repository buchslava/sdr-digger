use std::collections::VecDeque;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

use crate::decode::MorseDecoder;
use crate::detect::{Hit, ScanEvent, ScanProgress};
use crate::dsp::{ConnectConfig, DspCommand, FFT_SIZE};
use crate::i18n::Lang;
use crate::radio::{
    apply_ppm, band_label_msg, build_coach_lines, check_recipe, ensure_user_dir, list_cases,
    load_case, load_settings, nearest_gain_tenths, save_case, save_settings, snap_to_raster,
    AppSettings, CaseFile, CoachContext, Family, GainMode, RadioMode,
};
use crate::ui::toast::{TimedToast, ToastKind};

pub const WATERFALL_MAX_ROWS: usize = 80;
pub const VFO_DIGITS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    EditingFrequency,
    Dialog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    Cases,
    Findings,
    ScanProgress,
    Inspector,
    School,
    SaveCase,
    Help,
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Disconnected { last_error: Option<String> },
    Connected,
}

pub struct AppState {
    pub running: bool,
    pub lang: Lang,
    pub test_mode: bool,
    pub mode: AppMode,
    pub dialog: Option<DialogKind>,
    pub case: CaseFile,
    pub cases: Vec<CaseFile>,
    pub case_dirty: bool,
    pub settings: AppSettings,
    pub connection: ConnectionState,
    pub spectrum: Vec<f32>,
    pub waterfall: VecDeque<Vec<f32>>,
    pub show_overlay: bool,
    pub y_min: f64,
    pub y_max: f64,
    pub vfo_cursor: usize,
    pub last_radio_mode: RadioMode,
    pub findings: Vec<Hit>,
    pub finding_idx: usize,
    pub after_detect: bool,
    pub scan_progress: Option<ScanProgress>,
    pub school_page: usize,
    pub inspector_section: usize,
    pub inspector_row: usize,
    pub case_browser_idx: usize,
    pub timed_toast: Option<TimedToast>,
    pub morse: MorseDecoder,
    pub coach_lines: Vec<String>,
    pub overload: bool,
    pub empty_grass: bool,

    pub rx: Option<Receiver<Vec<f32>>>,
    pub audio_rx: Option<Receiver<Vec<f32>>>,
    pub cmd_tx: Option<Sender<DspCommand>>,
    pub scan_rx: Option<Receiver<ScanEvent>>,
    pub connect_request: bool,
    pub disconnect_request: bool,
    pub scan_request: bool,
    pub scan_quit: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl AppState {
    pub fn new(lang: Lang, test_mode: bool) -> Self {
        let _ = ensure_user_dir();
        let settings = load_settings();
        let cases = list_cases();
        let slug = settings
            .last_case_slug
            .clone()
            .unwrap_or_else(|| "fm-broadcast-r1".into());
        let mut case = load_case(&slug).unwrap_or_else(|_| CaseFile::default());
        if settings.last_ppm != 0.0 {
            case.tune.ppm = settings.last_ppm;
        }
        let mut app = Self {
            running: true,
            lang,
            test_mode,
            mode: AppMode::Normal,
            dialog: None,
            case,
            cases,
            case_dirty: false,
            settings,
            connection: ConnectionState::Disconnected { last_error: None },
            spectrum: vec![-100.0; FFT_SIZE],
            waterfall: VecDeque::with_capacity(WATERFALL_MAX_ROWS),
            show_overlay: true,
            y_min: -10.0,
            y_max: 100.0,
            vfo_cursor: 6,
            last_radio_mode: RadioMode::Wbfm,
            findings: Vec::new(),
            finding_idx: 0,
            after_detect: false,
            scan_progress: None,
            school_page: 0,
            inspector_section: 0,
            inspector_row: 0,
            case_browser_idx: 0,
            timed_toast: None,
            morse: MorseDecoder::new(48_000),
            coach_lines: Vec::new(),
            overload: false,
            empty_grass: false,
            rx: None,
            audio_rx: None,
            cmd_tx: None,
            scan_rx: None,
            connect_request: false,
            disconnect_request: false,
            scan_request: false,
            scan_quit: None,
        };
        app.y_min = app.settings.y_min;
        app.y_max = app.settings.y_max;
        app.show_overlay = app.settings.show_overlay;
        if app.case.voice.mode != RadioMode::Off {
            app.last_radio_mode = app.case.voice.mode;
        }
        app.refresh_coach();
        app
    }

    pub fn refresh_coach(&mut self) {
        let issues = check_recipe(&self.case.recipe());
        let ctx = CoachContext {
            lang: self.lang,
            after_detect: self.after_detect,
            overload: self.overload,
            empty_grass: self.empty_grass,
            near_dc_spike: false,
        };
        self.coach_lines = build_coach_lines(&self.case, &ctx, &issues);
    }

    pub fn connect_config(&self) -> ConnectConfig {
        ConnectConfig {
            sample_rate: self.case.window.sample_rate_hz,
            frequency_hz: apply_ppm(self.case.tune.frequency_hz, self.case.tune.ppm),
            gain_db: self.case.strength.gain_db,
            agc: self.case.strength.gain == GainMode::Agc,
            radio_mode: self.case.voice.mode.code(),
            squelch_db: self.case.voice.squelch_db,
            deemphasis_us: self.case.voice.deemphasis_us,
            cw_pitch_hz: self.case.voice.cw_pitch_hz,
        }
    }

    pub fn set_connected(
        &mut self,
        rx: Receiver<Vec<f32>>,
        audio_rx: Receiver<Vec<f32>>,
        cmd_tx: Sender<DspCommand>,
    ) {
        self.rx = Some(rx);
        self.audio_rx = Some(audio_rx);
        self.cmd_tx = Some(cmd_tx);
        self.connection = ConnectionState::Connected;
        self.push_live_params();
    }

    pub fn set_disconnected(&mut self, err: Option<String>) {
        self.rx = None;
        self.audio_rx = None;
        self.cmd_tx = None;
        self.connection = ConnectionState::Disconnected { last_error: err };
    }

    pub fn push_live_params(&self) {
        let Some(ref tx) = self.cmd_tx else { return };
        let freq = apply_ppm(self.case.tune.frequency_hz, self.case.tune.ppm);
        let _ = tx.try_send(DspCommand::TuneFrequency(freq));
        let _ = tx.try_send(DspCommand::SetRadioMode(self.case.voice.mode.code()));
        let _ = tx.try_send(DspCommand::SetSquelchDb(self.case.voice.squelch_db));
        let _ = tx.try_send(DspCommand::SetDeemphasisUs(self.case.voice.deemphasis_us));
        let _ = tx.try_send(DspCommand::SetCwPitchHz(self.case.voice.cw_pitch_hz));
        if self.case.strength.gain == GainMode::Manual {
            let tenths = nearest_gain_tenths(self.case.strength.gain_db);
            let _ = tx.try_send(DspCommand::SetManualGain(tenths));
            let _ = tx.try_send(DspCommand::SetAgcMode(false));
        } else {
            let _ = tx.try_send(DspCommand::SetAgcMode(true));
        }
    }

    pub fn poll_data(&mut self) {
        let mut frames = Vec::new();
        if let Some(ref rx) = self.rx {
            while let Ok(frame) = rx.try_recv() {
                frames.push(frame);
            }
        }
        for frame in frames {
            self.ingest_spectrum(frame);
        }

        let mut audio_chunks = Vec::new();
        if let Some(ref rx) = self.audio_rx {
            while let Ok(audio) = rx.try_recv() {
                audio_chunks.push(audio);
            }
        }
        if self.case.readout.morse {
            for audio in audio_chunks {
                self.morse.push(&audio);
            }
        }

        let mut scan_events = Vec::new();
        if let Some(ref rx) = self.scan_rx {
            while let Ok(ev) = rx.try_recv() {
                scan_events.push(ev);
            }
        }
        for ev in scan_events {
            self.handle_scan_event(ev);
        }

        let _ = TimedToast::clear_if_expired(&mut self.timed_toast);
    }

    fn ingest_spectrum(&mut self, frame: Vec<f32>) {
        if frame.len() != FFT_SIZE {
            return;
        }
        // Gain advisor heuristics
        let max = frame.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let p20 = {
            let mut s = frame.clone();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            s[s.len() / 5]
        };
        self.overload = max > 90.0;
        self.empty_grass = max < p20 + 6.0;
        self.spectrum = frame.clone();
        self.waterfall.push_front(frame);
        while self.waterfall.len() > WATERFALL_MAX_ROWS {
            self.waterfall.pop_back();
        }
        self.refresh_coach();
    }

    fn handle_scan_event(&mut self, ev: ScanEvent) {
        match ev {
            ScanEvent::Progress(p) => {
                self.scan_progress = Some(p);
                self.dialog = Some(DialogKind::ScanProgress);
                self.mode = AppMode::Dialog;
            }
            ScanEvent::Finished(hits) => {
                self.findings = hits;
                self.finding_idx = 0;
                self.scan_progress = None;
                self.scan_rx = None;
                self.scan_quit = None;
                if self.findings.is_empty() {
                    self.toast(
                        crate::i18n::t(self.lang, crate::i18n::Msg::ToastScanEmpty),
                        ToastKind::Info,
                    );
                    self.dialog = None;
                    self.mode = AppMode::Normal;
                } else {
                    self.toast(
                        crate::i18n::t(self.lang, crate::i18n::Msg::ToastScanDone),
                        ToastKind::Info,
                    );
                    self.dialog = Some(DialogKind::Findings);
                    self.mode = AppMode::Dialog;
                }
                // Restart listen after scan
                self.connect_request = true;
            }
            ScanEvent::Failed(e) => {
                self.scan_progress = None;
                self.scan_rx = None;
                self.toast(&e, ToastKind::Alert);
                self.dialog = None;
                self.mode = AppMode::Normal;
                self.connect_request = true;
            }
        }
    }

    pub fn toast(&mut self, msg: &str, kind: ToastKind) {
        self.timed_toast = Some(TimedToast::with_kind(
            Duration::from_secs(4),
            msg.to_string(),
            kind,
        ));
    }

    pub fn request_connect(&mut self) {
        self.connect_request = true;
    }

    pub fn request_disconnect(&mut self) {
        self.disconnect_request = true;
    }

    pub fn take_connect_request(&mut self) -> bool {
        let v = self.connect_request;
        self.connect_request = false;
        v
    }

    pub fn take_disconnect_request(&mut self) -> bool {
        let v = self.disconnect_request;
        self.disconnect_request = false;
        v
    }

    pub fn take_scan_request(&mut self) -> bool {
        let v = self.scan_request;
        self.scan_request = false;
        v
    }

    pub fn toggle_stream(&mut self) {
        match self.connection {
            ConnectionState::Connected => self.request_disconnect(),
            ConnectionState::Disconnected { .. } => self.request_connect(),
        }
    }

    pub fn start_scan(&mut self) {
        // Release device for exclusive scan
        self.request_disconnect();
        self.scan_request = true;
        self.dialog = Some(DialogKind::ScanProgress);
        self.mode = AppMode::Dialog;
    }

    pub fn apply_finding(&mut self, idx: usize) {
        if let Some(hit) = self.findings.get(idx).cloned() {
            self.case.tune.frequency_hz = hit.frequency_hz;
            self.finding_idx = idx;
            self.after_detect = true;
            self.case_dirty = true;
            self.refresh_coach();
            self.push_live_params();
            self.dialog = None;
            self.mode = AppMode::Normal;
            if matches!(self.connection, ConnectionState::Disconnected { .. }) {
                self.request_connect();
            }
        }
    }

    pub fn next_finding(&mut self, forward: bool) {
        if self.findings.is_empty() {
            return;
        }
        if forward {
            self.finding_idx = (self.finding_idx + 1) % self.findings.len();
        } else {
            self.finding_idx = (self.finding_idx + self.findings.len() - 1) % self.findings.len();
        }
        self.apply_finding(self.finding_idx);
    }

    pub fn nudge_freq(&mut self, dir: i64) {
        let step = if self.after_detect {
            self.case.family.fine_tune_hz()
        } else if self.case.tune.raster_hz > 0 {
            self.case.tune.raster_hz
        } else {
            self.case.family.fine_tune_hz()
        };
        let delta = step as i64 * dir;
        let f = (self.case.tune.frequency_hz as i64 + delta).max(500_000) as u64;
        self.case.tune.frequency_hz = if self.case.tune.raster_hz > 0 {
            snap_to_raster(f, self.case.tune.raster_hz)
        } else {
            f
        };
        self.case_dirty = true;
        self.push_live_params();
        self.refresh_coach();
    }

    pub fn toggle_mute(&mut self) {
        if self.case.voice.mode == RadioMode::Off {
            self.case.voice.mode = self.last_radio_mode;
        } else {
            self.last_radio_mode = self.case.voice.mode;
            self.case.voice.mode = RadioMode::Off;
        }
        self.case_dirty = true;
        self.push_live_params();
    }

    pub fn load_case_slug(&mut self, slug: &str) {
        match load_case(slug) {
            Ok(c) => {
                self.case = c;
                self.case_dirty = false;
                self.after_detect = false;
                self.settings.last_case_slug = Some(slug.to_string());
                let _ = save_settings(&self.settings);
                self.refresh_coach();
                self.push_live_params();
                // sample rate change needs reconnect
                self.request_disconnect();
                self.request_connect();
            }
            Err(e) => self.toast(&e, ToastKind::Alert),
        }
    }

    pub fn save_current_case(&mut self) {
        match save_case(&self.case) {
            Ok(_) => {
                self.case_dirty = false;
                self.settings.last_case_slug = Some(self.case.slug.clone());
                self.settings.last_ppm = self.case.tune.ppm;
                let _ = save_settings(&self.settings);
                self.cases = list_cases();
                self.toast(
                    crate::i18n::t(self.lang, crate::i18n::Msg::ToastSaved),
                    ToastKind::Info,
                );
            }
            Err(e) => self.toast(&e, ToastKind::Alert),
        }
    }

    pub fn bookmark_current_finding(&mut self) {
        if let Some(hit) = self.findings.get(self.finding_idx) {
            let name = hit
                .label
                .clone()
                .unwrap_or_else(|| format!("{:.3} MHz", hit.frequency_hz as f64 / 1e6));
            self.case.bookmarks.push(crate::radio::Bookmark {
                name,
                frequency_hz: hit.frequency_hz,
            });
            self.case_dirty = true;
        }
    }

    /// PPM helper: estimate from spectrum peak vs dial frequency.
    pub fn apply_ppm_from_peak(&mut self) {
        let peak_bin = self
            .spectrum
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(FFT_SIZE / 2);
        let sr = self.case.window.sample_rate_hz as f64;
        let offset = (peak_bin as f64 - (FFT_SIZE as f64 / 2.0)) * (sr / FFT_SIZE as f64);
        let dial = self.case.tune.frequency_hz as f64;
        if dial < 1.0 {
            return;
        }
        let measured = dial + offset;
        let ppm = (measured / dial - 1.0) * 1_000_000.0;
        // Only accept modest corrections
        if ppm.abs() < 200.0 {
            self.case.tune.ppm = ppm;
            self.settings.last_ppm = ppm;
            self.case_dirty = true;
            self.push_live_params();
            self.toast(
                crate::i18n::t(self.lang, crate::i18n::Msg::ToastPpmApplied),
                ToastKind::Info,
            );
            self.refresh_coach();
        }
    }

    pub fn band_label(&self) -> &'static str {
        crate::i18n::t(
            self.lang,
            band_label_msg(self.case.family, self.case.tune.frequency_hz),
        )
    }

    pub fn event_poll_timeout(&self) -> Duration {
        if self.mode != AppMode::Normal || self.timed_toast.is_some() || self.scan_progress.is_some()
        {
            Duration::from_millis(16)
        } else if matches!(self.connection, ConnectionState::Connected) {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(80)
        }
    }

    pub fn vfo_digit_value(&self) -> u64 {
        10u64.pow(self.vfo_cursor as u32)
    }

    pub fn adjust_vfo_digit(&mut self, delta: i64) {
        let place = self.vfo_digit_value() as i64;
        let f = (self.case.tune.frequency_hz as i64 + delta * place).max(500_000) as u64;
        self.case.tune.frequency_hz = f.min(1_766_000_000);
        self.case_dirty = true;
        self.push_live_params();
        self.refresh_coach();
    }

    pub fn family_filter_idx(&self) -> Option<usize> {
        Family::ALL.iter().position(|f| *f == self.case.family)
    }
}
