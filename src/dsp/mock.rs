//! Working mock DSP: synthetic spectrum frames + optional CW audio for Morse.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use rand::Rng;

use super::command::DspCommand;
use super::{ConnectConfig, MODE_CW, FFT_SIZE};

pub fn spawn(
    cfg: ConnectConfig,
    tx: Sender<Vec<f32>>,
    audio_tx: Option<Sender<Vec<f32>>>,
    cmd_rx: Receiver<DspCommand>,
    quit: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let freq = Arc::new(AtomicU64::new(cfg.frequency_hz));
        let mode = Arc::new(std::sync::atomic::AtomicU8::new(cfg.radio_mode));
        let freq_cmd = Arc::clone(&freq);
        let mode_cmd = Arc::clone(&mode);
        let quit_cmd = Arc::clone(&quit);
        let cmd_thread = thread::spawn(move || loop {
            if quit_cmd.load(Ordering::Relaxed) {
                break;
            }
            match cmd_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(DspCommand::TuneFrequency(hz)) => {
                    freq_cmd.store(hz, Ordering::Relaxed);
                }
                Ok(DspCommand::SetRadioMode(m)) => {
                    mode_cmd.store(m, Ordering::Relaxed);
                }
                Ok(_) => {}
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        });

        let mut rng = rand::rng();
        let mut t = 0u64;
        while !quit.load(Ordering::Relaxed) {
            let center = freq.load(Ordering::Relaxed) as f64;
            let sr = cfg.sample_rate as f64;
            let mut frame = vec![-70.0f32; FFT_SIZE];
            // Noise floor
            for v in &mut frame {
                *v += rng.random_range(-2.0..2.0);
            }
            // Synthetic carriers relative to centre
            paint_peak(&mut frame, FFT_SIZE / 2, -40.0, 8); // DC-ish
            // Offset “station”
            let bin = (FFT_SIZE as f64 * 0.15) as usize;
            paint_peak(&mut frame, FFT_SIZE / 2 + bin, -35.0, 20);
            let _ = center;
            let _ = sr;
            let _ = tx.try_send(frame);

            if mode.load(Ordering::Relaxed) == MODE_CW {
                if let Some(ref atx) = audio_tx {
                    // Simple Morse "CQ" audio tone bursts for ditdah.
                    let chunk = synth_cw_chunk(t, 48_000, cfg.cw_pitch_hz);
                    let _ = atx.try_send(chunk);
                }
            }
            t = t.wrapping_add(2048);
            thread::sleep(Duration::from_millis(33));
        }
        let _ = cmd_thread.join();
    })
}

fn paint_peak(frame: &mut [f32], center: usize, peak: f32, half: usize) {
    let n = frame.len();
    for i in center.saturating_sub(half)..=(center + half).min(n.saturating_sub(1)) {
        let d = i.abs_diff(center) as f32 / half.max(1) as f32;
        let v = peak + 30.0 * (1.0 - d);
        frame[i] = frame[i].max(v);
    }
}

/// Generate a short audio chunk with Morse-like on/off at ~15 WPM.
fn synth_cw_chunk(sample_pos: u64, sample_rate: u32, pitch_hz: f32) -> Vec<f32> {
    let dit = (sample_rate as f64 * 1.2 / 15.0) as u64; // dit length ~80ms at 15 WPM
    // Pattern for "CQ " as bits of dits/dahs (simplified square envelope).
    // C = -.-.  Q = --.-
    let pattern: &[(bool, u64)] = &[
        (true, dit),
        (false, dit),
        (true, 3 * dit),
        (false, dit),
        (true, dit),
        (false, dit),
        (true, dit),
        (false, 3 * dit),
        (true, 3 * dit),
        (false, dit),
        (true, 3 * dit),
        (false, dit),
        (true, dit),
        (false, dit),
        (true, 3 * dit),
        (false, 7 * dit),
    ];
    let cycle: u64 = pattern.iter().map(|(_, d)| d).sum();
    let mut out = Vec::with_capacity(2048);
    for i in 0..2048u64 {
        let pos = (sample_pos + i) % cycle.max(1);
        let mut acc = 0u64;
        let mut on = false;
        for &(key, dur) in pattern {
            if pos < acc + dur {
                on = key;
                break;
            }
            acc += dur;
        }
        let phase = 2.0 * PI * pitch_hz * (sample_pos + i) as f32 / sample_rate as f32;
        let s = if on { phase.sin() * 0.4 } else { 0.0 };
        out.push(s);
    }
    out
}
