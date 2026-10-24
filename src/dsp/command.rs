use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

use futuresdr::prelude::*;

use super::silence;

pub enum DspCommand {
    TuneFrequency(u64),
    SetManualGain(i32),
    SetAgcMode(bool),
    SetRadioMode(u8),
    SetSquelchDb(f32),
    SetDeemphasisUs(u32),
    SetCwPitchHz(f32),
}

#[derive(Clone)]
pub(super) struct Atoms {
    pub signal_power_db: Arc<AtomicU32>,
    pub squelch_db: Arc<AtomicU32>,
    pub radio_mode: Arc<AtomicU8>,
    pub deemphasis_us: Arc<AtomicU32>,
    pub cw_pitch_hz: Arc<AtomicU32>,
}

impl Atoms {
    pub(super) fn new(squelch_db: f32, mode: u8, deemph_us: u32, cw_pitch: f32) -> Self {
        Self {
            signal_power_db: Arc::new(AtomicU32::new(f32::to_bits(-100.0))),
            squelch_db: Arc::new(AtomicU32::new(f32::to_bits(squelch_db))),
            radio_mode: Arc::new(AtomicU8::new(mode)),
            deemphasis_us: Arc::new(AtomicU32::new(deemph_us)),
            cw_pitch_hz: Arc::new(AtomicU32::new(f32::to_bits(cw_pitch))),
        }
    }
}

pub(super) async fn apply_command(
    handle: &FlowgraphHandle,
    src: BlockId,
    atoms: &Atoms,
    cmd: DspCommand,
) {
    let (port, pmt) = match cmd {
        DspCommand::TuneFrequency(hz) => ("freq", Pmt::F64(hz as f64)),
        DspCommand::SetManualGain(tenths) => ("gain", Pmt::F64(tenths as f64 / 10.0)),
        DspCommand::SetAgcMode(on) => {
            // seify may not expose AGC as a message port on all drivers;
            // try "agc" and ignore failure.
            let fut = handle.post(src, "agc", Pmt::Bool(on));
            let _guard = silence::SilencedStderr::new();
            let _ = fut.await;
            return;
        }
        DspCommand::SetRadioMode(mode) => {
            atoms.radio_mode.store(mode, Ordering::Relaxed);
            return;
        }
        DspCommand::SetSquelchDb(db) => {
            atoms.squelch_db.store(f32::to_bits(db), Ordering::Relaxed);
            return;
        }
        DspCommand::SetDeemphasisUs(us) => {
            atoms.deemphasis_us.store(us, Ordering::Relaxed);
            return;
        }
        DspCommand::SetCwPitchHz(hz) => {
            atoms.cw_pitch_hz.store(f32::to_bits(hz), Ordering::Relaxed);
            return;
        }
    };
    let fut = handle.post(src, port, pmt);
    let _guard = silence::SilencedStderr::new();
    let _ = fut.await;
}
