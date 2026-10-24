use serde::{Deserialize, Serialize};

use crate::i18n::Msg;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RadioMode {
    #[default]
    Off,
    Wbfm,
    Nbfm,
    Am,
    Usb,
    Lsb,
    Cw,
}

impl RadioMode {
    pub fn msg(self) -> Msg {
        match self {
            RadioMode::Off => Msg::ModeOff,
            RadioMode::Wbfm => Msg::ModeWbfm,
            RadioMode::Nbfm => Msg::ModeNbfm,
            RadioMode::Am => Msg::ModeAm,
            RadioMode::Usb => Msg::ModeUsb,
            RadioMode::Lsb => Msg::ModeLsb,
            RadioMode::Cw => Msg::ModeCw,
        }
    }

    /// DSP atomic code matching `dsp::MODE_*`.
    pub fn code(self) -> u8 {
        match self {
            RadioMode::Off => 0,
            RadioMode::Wbfm => 1,
            RadioMode::Nbfm => 2,
            RadioMode::Am => 3,
            RadioMode::Usb => 4,
            RadioMode::Lsb => 5,
            RadioMode::Cw => 6,
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            1 => RadioMode::Wbfm,
            2 => RadioMode::Nbfm,
            3 => RadioMode::Am,
            4 => RadioMode::Usb,
            5 => RadioMode::Lsb,
            6 => RadioMode::Cw,
            _ => RadioMode::Off,
        }
    }

    /// Approximate occupied RF bandwidth for overlay / Carson checks.
    pub fn default_channel_bw_hz(self) -> u32 {
        match self {
            RadioMode::Off => 0,
            RadioMode::Wbfm => 180_000,
            RadioMode::Nbfm => 12_500,
            RadioMode::Am => 10_000,
            RadioMode::Usb | RadioMode::Lsb => 2_700,
            RadioMode::Cw => 400,
        }
    }

    pub fn next(self) -> Self {
        match self {
            RadioMode::Off => RadioMode::Wbfm,
            RadioMode::Wbfm => RadioMode::Nbfm,
            RadioMode::Nbfm => RadioMode::Am,
            RadioMode::Am => RadioMode::Usb,
            RadioMode::Usb => RadioMode::Lsb,
            RadioMode::Lsb => RadioMode::Cw,
            RadioMode::Cw => RadioMode::Off,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            RadioMode::Off => RadioMode::Cw,
            RadioMode::Wbfm => RadioMode::Off,
            RadioMode::Nbfm => RadioMode::Wbfm,
            RadioMode::Am => RadioMode::Nbfm,
            RadioMode::Usb => RadioMode::Am,
            RadioMode::Lsb => RadioMode::Usb,
            RadioMode::Cw => RadioMode::Lsb,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GainMode {
    #[default]
    Manual,
    Agc,
}

impl GainMode {
    pub fn msg(self) -> Msg {
        match self {
            GainMode::Manual => Msg::GainManual,
            GainMode::Agc => Msg::GainAgc,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuneParams {
    pub frequency_hz: u64,
    #[serde(default)]
    pub ppm: f64,
    #[serde(default)]
    pub raster_hz: u64,
}

impl Default for TuneParams {
    fn default() -> Self {
        Self {
            frequency_hz: 100_000_000,
            ppm: 0.0,
            raster_hz: 100_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowParams {
    pub sample_rate_hz: u32,
    pub channel_bw_hz: u32,
}

impl Default for WindowParams {
    fn default() -> Self {
        Self {
            sample_rate_hz: 1_024_000,
            channel_bw_hz: 180_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrengthParams {
    pub gain: GainMode,
    /// Manual gain in dB (R820T2 steps stored as float for TOML readability).
    pub gain_db: f64,
}

impl Default for StrengthParams {
    fn default() -> Self {
        Self {
            gain: GainMode::Manual,
            gain_db: 36.4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceParams {
    pub mode: RadioMode,
    #[serde(default = "default_deviation")]
    pub deviation_hz: u32,
    #[serde(default = "default_deemph")]
    pub deemphasis_us: u32,
    #[serde(default = "default_squelch")]
    pub squelch_db: f32,
    #[serde(default = "default_cw_pitch")]
    pub cw_pitch_hz: f32,
}

fn default_deviation() -> u32 {
    75_000
}
fn default_deemph() -> u32 {
    50
}
fn default_squelch() -> f32 {
    -50.0
}
fn default_cw_pitch() -> f32 {
    700.0
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            mode: RadioMode::Wbfm,
            deviation_hz: 75_000,
            deemphasis_us: 50,
            squelch_db: -50.0,
            cw_pitch_hz: 700.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadoutParams {
    #[serde(default = "default_true")]
    pub audio: bool,
    #[serde(default)]
    pub morse: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ReadoutParams {
    fn default() -> Self {
        Self {
            audio: true,
            morse: false,
        }
    }
}
