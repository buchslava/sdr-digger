//! FutureSDR listen graph: FFT + WBFM + narrow (NBFM/AM) + SSB/CW chains.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::seify::Builder as SeifyBuilder;
use futuresdr::blocks::{Apply, Combine, Fft, FftDirection, FirBuilder};
use futuresdr::num_complex::Complex32;
use futuresdr::prelude::*;
use futuresdr::seify::{Device, GenericDevice};

use super::command::{apply_command, Atoms, DspCommand};
use super::silence;
use super::sink;
use super::tee;
use super::{
    ConnectConfig, MODE_AM, MODE_CW, MODE_LSB, MODE_NBFM, MODE_USB, MODE_WBFM, FFT_SIZE,
};

const AUDIO_RATE: u32 = 48_000;
const WBFM_TARGET_RATE: u32 = 256_000;
const NB_TARGET_RATE: u32 = 32_000;
const SSB_TARGET_RATE: u32 = 8_000;

pub(super) fn run(
    dev: Device<GenericDevice>,
    cfg: ConnectConfig,
    tx: Sender<Vec<f32>>,
    audio_tx: Option<Sender<Vec<f32>>>,
    cmd_rx: Receiver<DspCommand>,
    quit: Arc<AtomicBool>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut fg = Flowgraph::new();
    let sample_rate = cfg.sample_rate;

    let wbfm_decim = ((sample_rate as f64) / (WBFM_TARGET_RATE as f64))
        .round()
        .max(3.0) as usize;
    let wbfm_rate = sample_rate / wbfm_decim as u32;
    let nb_decim = ((sample_rate as f64) / (NB_TARGET_RATE as f64))
        .round()
        .max(3.0) as usize;
    let nb_rate = sample_rate / nb_decim as u32;
    let ssb_decim = ((sample_rate as f64) / (SSB_TARGET_RATE as f64))
        .round()
        .max(3.0) as usize;
    let ssb_rate = sample_rate / ssb_decim as u32;

    let src = silence::silenced(|| {
        SeifyBuilder::from_device(dev)
            .frequency(cfg.frequency_hz as f64)
            .sample_rate(sample_rate as f64)
            .gain(cfg.gain_db)
            .build_source()
    })?;

    let atoms = Atoms::new(
        cfg.squelch_db,
        cfg.radio_mode,
        cfg.deemphasis_us,
        cfg.cw_pitch_hz,
    );

    let tee1: tee::Tee = tee::Tee::new();
    let tee2: tee::Tee = tee::Tee::new();
    let tee3: tee::Tee = tee::Tee::new();

    let fft = Fft::with_options(FFT_SIZE, FftDirection::Forward, true, None);
    let mag = Apply::new(|c: &Complex32| -> f32 { 10.0 * (c.norm_sqr() + 1e-20).log10() });
    let snk = sink::ChunkSink::new(FFT_SIZE, tx);

    // ── WBFM ────────────────────────────────────────────────────────────
    let wbfm_decim_block =
        FirBuilder::decimating::<Complex32, Complex32, Vec<f32>>(wbfm_decim);
    let mut last_wbfm = Complex32::new(0.0, 0.0);
    let wbfm_gain = (wbfm_rate as f32) / (2.0 * std::f32::consts::PI * 75_000.0);
    let wbfm_demod = Apply::new(move |c: &Complex32| -> f32 {
        let phase = (c * last_wbfm.conj()).arg();
        last_wbfm = *c;
        phase * wbfm_gain
    });
    let wbfm_resamp =
        FirBuilder::resampling::<f32, f32>(AUDIO_RATE as usize, wbfm_rate as usize);

    let deemph_atom = Arc::clone(&atoms.deemphasis_us);
    let mut y_prev: f32 = 0.0;
    let deemph = Apply::new(move |x: &f32| -> f32 {
        let us = deemph_atom.load(Ordering::Relaxed);
        if us == 0 {
            return *x;
        }
        let tau_s = us as f32 * 1e-6;
        let alpha = (-1.0 / (AUDIO_RATE as f32 * tau_s)).exp();
        let y = (1.0 - alpha) * *x + alpha * y_prev;
        y_prev = y;
        y
    });

    let vol_a_mode = Arc::clone(&atoms.radio_mode);
    let vol_a_pm = Arc::clone(&atoms.signal_power_db);
    let vol_a_sq = Arc::clone(&atoms.squelch_db);
    let volume_a = Apply::new(move |s: &f32| -> f32 {
        if vol_a_mode.load(Ordering::Relaxed) != MODE_WBFM {
            return 0.0;
        }
        let power = f32::from_bits(vol_a_pm.load(Ordering::Relaxed));
        let threshold = f32::from_bits(vol_a_sq.load(Ordering::Relaxed));
        if power < threshold {
            0.0
        } else {
            *s * 0.3
        }
    });

    // ── Narrow NBFM / AM ────────────────────────────────────────────────
    let nb_decim_block = FirBuilder::decimating::<Complex32, Complex32, Vec<f32>>(nb_decim);
    let pm_w = Arc::clone(&atoms.signal_power_db);
    let mut pm_sum = 0.0_f32;
    let mut pm_count: u32 = 0;
    let power_meter = Apply::new(move |c: &Complex32| -> Complex32 {
        pm_sum += c.norm_sqr();
        pm_count += 1;
        if pm_count >= 256 {
            let avg = pm_sum / pm_count as f32;
            let db = 10.0 * (avg + 1e-20).log10();
            pm_w.store(f32::to_bits(db), Ordering::Relaxed);
            pm_sum = 0.0;
            pm_count = 0;
        }
        *c
    });

    let nb_mode = Arc::clone(&atoms.radio_mode);
    let nbfm_gain = (nb_rate as f32) / (2.0 * std::f32::consts::PI * 5_000.0);
    let mut last_nb = Complex32::new(0.0, 0.0);
    let mut am_dc: f32 = 0.0;
    let dc_alpha: f32 = 0.999;
    let nb_demod = Apply::new(move |c: &Complex32| -> f32 {
        match nb_mode.load(Ordering::Relaxed) {
            MODE_NBFM => {
                let phase = (c * last_nb.conj()).arg();
                last_nb = *c;
                phase * nbfm_gain
            }
            MODE_AM => {
                let mag = c.norm();
                am_dc = dc_alpha * am_dc + (1.0 - dc_alpha) * mag;
                mag - am_dc
            }
            _ => 0.0,
        }
    });
    let nb_resamp = FirBuilder::resampling::<f32, f32>(AUDIO_RATE as usize, nb_rate as usize);
    let vol_b_mode = Arc::clone(&atoms.radio_mode);
    let vol_b_pm = Arc::clone(&atoms.signal_power_db);
    let vol_b_sq = Arc::clone(&atoms.squelch_db);
    let volume_b = Apply::new(move |s: &f32| -> f32 {
        let m = vol_b_mode.load(Ordering::Relaxed);
        if m != MODE_NBFM && m != MODE_AM {
            return 0.0;
        }
        let power = f32::from_bits(vol_b_pm.load(Ordering::Relaxed));
        let threshold = f32::from_bits(vol_b_sq.load(Ordering::Relaxed));
        if power < threshold {
            0.0
        } else {
            *s * 0.3
        }
    });

    // ── SSB / CW product detector ───────────────────────────────────────
    let ssb_decim_block =
        FirBuilder::decimating::<Complex32, Complex32, Vec<f32>>(ssb_decim);
    let ssb_mode = Arc::clone(&atoms.radio_mode);
    let ssb_pitch = Arc::clone(&atoms.cw_pitch_hz);
    let mut nco_phase: f32 = 0.0;
    let ssb_demod = Apply::new(move |c: &Complex32| -> f32 {
        let m = ssb_mode.load(Ordering::Relaxed);
        let pitch = f32::from_bits(ssb_pitch.load(Ordering::Relaxed));
        match m {
            MODE_USB => c.re,
            MODE_LSB => c.re, // product after imag flip approximation
            MODE_CW => {
                let w = 2.0 * std::f32::consts::PI * pitch / ssb_rate as f32;
                nco_phase = (nco_phase + w) % (2.0 * std::f32::consts::PI);
                let (s, cos) = nco_phase.sin_cos();
                c.re * cos - c.im * s
            }
            _ => 0.0,
        }
    });
    let ssb_resamp =
        FirBuilder::resampling::<f32, f32>(AUDIO_RATE as usize, ssb_rate as usize);
    let vol_c_mode = Arc::clone(&atoms.radio_mode);
    let vol_c_pm = Arc::clone(&atoms.signal_power_db);
    let vol_c_sq = Arc::clone(&atoms.squelch_db);
    let audio_tap = audio_tx;
    let mut tap_buf: Vec<f32> = Vec::with_capacity(AUDIO_RATE as usize);
    let volume_c = Apply::new(move |s: &f32| -> f32 {
        let m = vol_c_mode.load(Ordering::Relaxed);
        let active = m == MODE_USB || m == MODE_LSB || m == MODE_CW;
        let out = if !active {
            0.0
        } else {
            let power = f32::from_bits(vol_c_pm.load(Ordering::Relaxed));
            let threshold = f32::from_bits(vol_c_sq.load(Ordering::Relaxed));
            if power < threshold {
                0.0
            } else {
                *s * 0.35
            }
        };
        if active {
            if let Some(ref tx) = audio_tap {
                tap_buf.push(out);
                if tap_buf.len() >= 2048 {
                    let frame = std::mem::take(&mut tap_buf);
                    let _ = tx.try_send(frame);
                    tap_buf = Vec::with_capacity(2048);
                }
            }
        }
        out
    });

    let mixer_ab = Combine::new(|a: &f32, b: &f32| -> f32 { a + b });
    let mixer_abc = Combine::new(|a: &f32, b: &f32| -> f32 { a + b });
    let audio_sink = AudioSink::new(AUDIO_RATE, 1)?;

    let src = fg.add(src);
    let tee1 = fg.add(tee1);
    let tee2 = fg.add(tee2);
    let tee3 = fg.add(tee3);
    let fft = fg.add(fft);
    let mag = fg.add(mag);
    let snk = fg.add(snk);
    let wbfm_decim_block = fg.add(wbfm_decim_block);
    let wbfm_demod = fg.add(wbfm_demod);
    let wbfm_resamp = fg.add(wbfm_resamp);
    let deemph = fg.add(deemph);
    let volume_a = fg.add(volume_a);
    let nb_decim_block = fg.add(nb_decim_block);
    let power_meter = fg.add(power_meter);
    let nb_demod = fg.add(nb_demod);
    let nb_resamp = fg.add(nb_resamp);
    let volume_b = fg.add(volume_b);
    let ssb_decim_block = fg.add(ssb_decim_block);
    let ssb_demod = fg.add(ssb_demod);
    let ssb_resamp = fg.add(ssb_resamp);
    let volume_c = fg.add(volume_c);
    let mixer_ab = fg.add(mixer_ab);
    let mixer_abc = fg.add(mixer_abc);
    let audio_sink = fg.add(audio_sink);

    fg.stream(&src, |b| b.outputs().get_mut(0).unwrap(), &tee1, |b| b.input())?;
    fg.stream(&tee1, |b| b.out_a(), &fft, |b| b.input())?;
    fg.stream(&fft, |b| b.output(), &mag, |b| b.input())?;
    fg.stream(&mag, |b| b.output(), &snk, |b| b.input())?;
    fg.stream(&tee1, |b| b.out_b(), &tee2, |b| b.input())?;

    fg.stream(&tee2, |b| b.out_a(), &wbfm_decim_block, |b| b.input())?;
    fg.stream(&wbfm_decim_block, |b| b.output(), &wbfm_demod, |b| b.input())?;
    fg.stream(&wbfm_demod, |b| b.output(), &wbfm_resamp, |b| b.input())?;
    fg.stream(&wbfm_resamp, |b| b.output(), &deemph, |b| b.input())?;
    fg.stream(&deemph, |b| b.output(), &volume_a, |b| b.input())?;

    fg.stream(&tee2, |b| b.out_b(), &tee3, |b| b.input())?;
    fg.stream(&tee3, |b| b.out_a(), &nb_decim_block, |b| b.input())?;
    fg.stream(&nb_decim_block, |b| b.output(), &power_meter, |b| b.input())?;
    fg.stream(&power_meter, |b| b.output(), &nb_demod, |b| b.input())?;
    fg.stream(&nb_demod, |b| b.output(), &nb_resamp, |b| b.input())?;
    fg.stream(&nb_resamp, |b| b.output(), &volume_b, |b| b.input())?;

    fg.stream(&tee3, |b| b.out_b(), &ssb_decim_block, |b| b.input())?;
    fg.stream(&ssb_decim_block, |b| b.output(), &ssb_demod, |b| b.input())?;
    fg.stream(&ssb_demod, |b| b.output(), &ssb_resamp, |b| b.input())?;
    fg.stream(&ssb_resamp, |b| b.output(), &volume_c, |b| b.input())?;

    fg.stream(&volume_a, |b| b.output(), &mixer_ab, |b| b.in0())?;
    fg.stream(&volume_b, |b| b.output(), &mixer_ab, |b| b.in1())?;
    fg.stream(&mixer_ab, |b| b.output(), &mixer_abc, |b| b.in0())?;
    fg.stream(&volume_c, |b| b.output(), &mixer_abc, |b| b.in1())?;
    fg.stream(&mixer_abc, |b| b.output(), &audio_sink, |b| b.input())?;

    let src_id: BlockId = (&src).into();
    let rt = Runtime::new();
    let running = silence::silenced(|| rt.start(fg))?;
    let handle = running.handle();

    // Apply AGC preference after start if requested.
    if cfg.agc {
        let atoms_c = atoms.clone();
        let h = handle.clone();
        futuresdr::futures::executor::block_on(apply_command(
            &h,
            src_id,
            &atoms_c,
            DspCommand::SetAgcMode(true),
        ));
    }

    let cmd_quit = Arc::clone(&quit);
    let atoms_pump = atoms.clone();
    let pump_handle = handle.clone();
    let pump_thread = std::thread::spawn(move || {
        use futuresdr::futures::executor::block_on;
        loop {
            if cmd_quit.load(Ordering::Relaxed) {
                break;
            }
            match cmd_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => block_on(apply_command(&pump_handle, src_id, &atoms_pump, cmd)),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    while !quit.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = pump_thread.join();
    let _ = futuresdr::futures::executor::block_on(running.stop_and_wait());
    drop(rt);
    Ok(())
}
