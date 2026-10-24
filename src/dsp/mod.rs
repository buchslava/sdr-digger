mod command;
mod flowgraph;
mod mock;
pub mod silence;
mod sink;
mod tee;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use futuresdr::seify::{Device, GenericDevice};

pub use command::DspCommand;

pub const MODE_OFF: u8 = 0;
pub const MODE_WBFM: u8 = 1;
pub const MODE_NBFM: u8 = 2;
pub const MODE_AM: u8 = 3;
pub const MODE_USB: u8 = 4;
pub const MODE_LSB: u8 = 5;
pub const MODE_CW: u8 = 6;

pub const FFT_SIZE: usize = 1024;

pub const RTL_SDR_OPEN_ARGS: &[&str] = &["driver=soapy,soapy_driver=rtlsdr", "driver=rtlsdr"];

#[derive(Debug, Clone)]
pub struct ConnectConfig {
    pub sample_rate: u32,
    pub frequency_hz: u64,
    pub gain_db: f64,
    pub agc: bool,
    pub radio_mode: u8,
    pub squelch_db: f32,
    pub deemphasis_us: u32,
    pub cw_pitch_hz: f32,
}

pub fn open_device() -> Result<Device<GenericDevice>, String> {
    let mut last_err = String::new();
    for args in RTL_SDR_OPEN_ARGS {
        match silence::silenced(|| Device::from_args(*args)) {
            Ok(dev) => return Ok(dev),
            Err(err) => last_err = err.to_string(),
        }
    }
    Err(format!(
        "Failed to open RTL-SDR via SoapySDR ({last_err}). \
         Install soapysdr + soapyrtlsdr / soapysdr-module-rtlsdr."
    ))
}

pub fn spawn_dsp_thread(
    dev: Device<GenericDevice>,
    cfg: ConnectConfig,
    tx: Sender<Vec<f32>>,
    audio_tx: Option<Sender<Vec<f32>>>,
    cmd_rx: Receiver<DspCommand>,
    quit: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let _ = flowgraph::run(dev, cfg, tx, audio_tx, cmd_rx, quit);
    })
}

pub fn spawn_mock_dsp_thread(
    cfg: ConnectConfig,
    tx: Sender<Vec<f32>>,
    audio_tx: Option<Sender<Vec<f32>>>,
    cmd_rx: Receiver<DspCommand>,
    quit: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    mock::spawn(cfg, tx, audio_tx, cmd_rx, quit)
}
