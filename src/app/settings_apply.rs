use crate::app::state::AppState;
use crate::radio::{GainMode, RadioMode};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingChange {
    SampleRateCycle(i32),
    ChannelBwCycle(i32),
    GainCycle(i32),
    GainModeToggle,
    ModeCycle(i32),
    SquelchStep(i32),
    DeemphasisCycle,
    MorseToggle,
    AudioToggle,
    PpmStep(f64),
}

pub fn apply_setting(app: &mut AppState, change: SettingChange) {
    match change {
        SettingChange::SampleRateCycle(dir) => {
            let rates = crate::radio::RTL_SAMPLE_RATES;
            let cur = rates
                .iter()
                .position(|&r| r == app.case.window.sample_rate_hz)
                .unwrap_or(2);
            let next = if dir >= 0 {
                (cur + 1) % rates.len()
            } else {
                (cur + rates.len() - 1) % rates.len()
            };
            app.case.window.sample_rate_hz = rates[next];
            app.case_dirty = true;
            app.request_disconnect();
            app.request_connect();
        }
        SettingChange::ChannelBwCycle(dir) => {
            let step = 1_000i64 * dir as i64;
            let bw = (app.case.window.channel_bw_hz as i64 + step).clamp(200, 2_000_000);
            app.case.window.channel_bw_hz = bw as u32;
            app.case_dirty = true;
        }
        SettingChange::GainCycle(dir) => {
            let gains = crate::radio::rtl_gain_tenths();
            let tenths = crate::radio::nearest_gain_tenths(app.case.strength.gain_db);
            let cur = gains.iter().position(|&g| g == tenths).unwrap_or(0);
            let next = if dir >= 0 {
                (cur + 1).min(gains.len() - 1)
            } else {
                cur.saturating_sub(1)
            };
            app.case.strength.gain_db = gains[next] as f64 / 10.0;
            app.case.strength.gain = GainMode::Manual;
            app.case_dirty = true;
            app.push_live_params();
        }
        SettingChange::GainModeToggle => {
            app.case.strength.gain = match app.case.strength.gain {
                GainMode::Manual => GainMode::Agc,
                GainMode::Agc => GainMode::Manual,
            };
            app.case_dirty = true;
            app.push_live_params();
        }
        SettingChange::ModeCycle(dir) => {
            let mut m = app.case.voice.mode;
            m = if dir >= 0 { m.next() } else { m.prev() };
            if m != RadioMode::Off {
                app.last_radio_mode = m;
            }
            app.case.voice.mode = m;
            app.case.window.channel_bw_hz = m.default_channel_bw_hz().max(200);
            app.case_dirty = true;
            app.push_live_params();
        }
        SettingChange::SquelchStep(dir) => {
            app.case.voice.squelch_db =
                (app.case.voice.squelch_db + 5.0 * dir as f32).clamp(-100.0, 0.0);
            app.case_dirty = true;
            app.push_live_params();
        }
        SettingChange::DeemphasisCycle => {
            app.case.voice.deemphasis_us = match app.case.voice.deemphasis_us {
                0 => 50,
                50 => 75,
                _ => 0,
            };
            app.case_dirty = true;
            app.push_live_params();
        }
        SettingChange::MorseToggle => {
            app.case.readout.morse = !app.case.readout.morse;
            if !app.case.readout.morse {
                app.morse.clear();
            }
            app.case_dirty = true;
        }
        SettingChange::AudioToggle => {
            app.case.readout.audio = !app.case.readout.audio;
            app.case_dirty = true;
        }
        SettingChange::PpmStep(delta) => {
            app.case.tune.ppm = (app.case.tune.ppm + delta).clamp(-150.0, 150.0);
            app.case_dirty = true;
            app.push_live_params();
        }
    }
    app.refresh_coach();
}
