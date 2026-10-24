#![allow(dead_code)]

mod app;
mod decode;
mod detect;
mod dialogs;
mod dsp;
mod i18n;
mod radio;
mod ui;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use ratatui::DefaultTerminal;

use app::{apply_setting, AppAction, AppState, ConnectionState, EventHandler};
use dsp::DspCommand;
use i18n::Lang;
use radio::save_settings;
use ui::toast::ToastKind;
use ui::Renderer;

#[derive(Parser, Debug)]
#[command(name = "sdr-digger", about = "Case-first RTL-SDR workbench")]
struct Cli {
    /// Run without hardware (synthetic spectrum + mock CW)
    #[arg(long)]
    test: bool,

    /// UI language: en | uk (ua is an alias for uk)
    #[arg(short = 'L', long = "lang", default_value = "en")]
    lang: String,
}

struct DspThread {
    handle: JoinHandle<()>,
    quit: Arc<AtomicBool>,
}

struct Supervisor {
    test_mode: bool,
    thread: Option<DspThread>,
    scan_thread: Option<DspThread>,
}

impl Supervisor {
    fn new(test_mode: bool) -> Self {
        Self {
            test_mode,
            thread: None,
            scan_thread: None,
        }
    }

    fn connect(
        &mut self,
        cfg: dsp::ConnectConfig,
    ) -> Result<(Receiver<Vec<f32>>, Receiver<Vec<f32>>, Sender<DspCommand>), String> {
        self.disconnect();

        let (tx, rx) = bounded::<Vec<f32>>(4);
        let (audio_tx, audio_rx) = bounded::<Vec<f32>>(4);
        let (cmd_tx, cmd_rx) = bounded::<DspCommand>(16);
        let quit = Arc::new(AtomicBool::new(false));

        let handle = if self.test_mode {
            dsp::spawn_mock_dsp_thread(cfg, tx, Some(audio_tx), cmd_rx, Arc::clone(&quit))
        } else {
            let dev = dsp::open_device()?;
            dsp::spawn_dsp_thread(dev, cfg, tx, Some(audio_tx), cmd_rx, Arc::clone(&quit))
        };

        self.thread = Some(DspThread { handle, quit });
        Ok((rx, audio_rx, cmd_tx))
    }

    fn disconnect(&mut self) {
        if let Some(t) = self.thread.take() {
            t.quit.store(true, Ordering::Relaxed);
            let _ = t.handle.join();
        }
    }

    fn start_scan(
        &mut self,
        case: radio::CaseFile,
    ) -> (Receiver<detect::ScanEvent>, Arc<AtomicBool>) {
        self.disconnect();
        if let Some(t) = self.scan_thread.take() {
            t.quit.store(true, Ordering::Relaxed);
            let _ = t.handle.join();
        }
        let (tx, rx) = bounded(16);
        let quit = Arc::new(AtomicBool::new(false));
        let handle = detect::spawn_scan(case, self.test_mode, tx, Arc::clone(&quit));
        self.scan_thread = Some(DspThread {
            handle,
            quit: Arc::clone(&quit),
        });
        (rx, quit)
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let lang = match Lang::from_cli(&cli.lang) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    if std::env::var_os("FUTURESDR_LOG").is_none() {
        unsafe { std::env::set_var("FUTURESDR_LOG", "off") };
    }
    if std::env::var_os("RUST_LOG").is_none() {
        unsafe { std::env::set_var("RUST_LOG", "off") };
    }

    radio::configure_linux_audio_env();

    let mut supervisor = Supervisor::new(cli.test);
    let mut app = AppState::new(lang, cli.test);

    match supervisor.connect(app.connect_config()) {
        Ok((rx, audio_rx, cmd_tx)) => app.set_connected(rx, audio_rx, cmd_tx),
        Err(e) => {
            app.set_disconnected(Some(e.clone()));
            if !cli.test {
                eprintln!("{}", i18n::t(lang, i18n::Msg::ToastDeviceError));
                eprintln!("{e}");
                eprintln!("Hint: try --test to explore the UI without a dongle.");
            }
        }
    }

    let mut terminal = ratatui::init();
    let result = run_ui(&mut terminal, &mut app, &mut supervisor);
    ratatui::restore();

    app.settings.last_case_slug = Some(app.case.slug.clone());
    app.settings.last_ppm = app.case.tune.ppm;
    app.settings.y_min = app.y_min;
    app.settings.y_max = app.y_max;
    app.settings.show_overlay = app.show_overlay;
    let _ = save_settings(&app.settings);

    supervisor.disconnect();
    result
}

fn run_ui(
    terminal: &mut DefaultTerminal,
    app: &mut AppState,
    supervisor: &mut Supervisor,
) -> io::Result<()> {
    let mut need_redraw = true;

    while app.running {
        app.poll_data();

        if app.take_disconnect_request() {
            supervisor.disconnect();
            app.set_disconnected(None);
            need_redraw = true;
        }

        if app.take_scan_request() {
            let (rx, quit) = supervisor.start_scan(app.case.clone());
            app.scan_rx = Some(rx);
            app.scan_quit = Some(quit);
            need_redraw = true;
        }

        if app.take_connect_request() {
            match supervisor.connect(app.connect_config()) {
                Ok((rx, audio_rx, cmd_tx)) => app.set_connected(rx, audio_rx, cmd_tx),
                Err(e) => {
                    app.set_disconnected(Some(e.clone()));
                    app.toast(&e, ToastKind::Alert);
                }
            }
            need_redraw = true;
        }

        if need_redraw {
            terminal.draw(|f| Renderer::draw_ui(f, app))?;
            need_redraw = false;
        }

        let timeout = app.event_poll_timeout();
        match EventHandler::handle_events(app, timeout)? {
            AppAction::Idle => {
                if matches!(app.connection, ConnectionState::Connected)
                    || app.timed_toast.is_some()
                    || app.scan_progress.is_some()
                {
                    need_redraw = true;
                }
            }
            AppAction::Redraw => {
                need_redraw = true;
            }
            AppAction::Quit => {
                app.running = false;
            }
            AppAction::SettingChange(change) => {
                apply_setting(app, change);
                need_redraw = true;
            }
        }
    }

    Ok(())
}
