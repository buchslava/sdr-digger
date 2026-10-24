//! Welch PSD + hop stitch (from sdr-kitchen) and synthetic fixtures for `--test`.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futuresdr::num_complex::Complex32;
use futuresdr::seify::{Device, Direction, GenericDevice};
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use super::plan::{ScanPhase, ScanPlan, ScanProgress};

pub const FFT_SIZE: usize = 4096;
pub const GRID_HZ: f64 = 5_000.0;
pub const DC_BLANK_HZ: f64 = 80_000.0;
const USABLE_NYQUIST_FRAC: f64 = 0.80;
const SPECTRUM_DWELL: Duration = Duration::from_millis(120);
const DEFAULT_GAIN_DB: f64 = 40.0;

#[derive(Debug, Clone)]
pub struct BandSpectrum {
    pub start_hz: f64,
    pub bin_hz: f64,
    pub power_db: Vec<f32>,
}

impl BandSpectrum {
    pub fn freq_hz(&self, idx: usize) -> f64 {
        self.start_hz + idx as f64 * self.bin_hz
    }

    pub fn index_of(&self, freq_hz: f64) -> Option<usize> {
        let idx = ((freq_hz - self.start_hz) / self.bin_hz).round();
        if idx < 0.0 {
            return None;
        }
        let idx = idx as usize;
        (idx < self.power_db.len()).then_some(idx)
    }

    pub fn empty_floor(start_hz: f64, end_hz: f64, floor_db: f32) -> Self {
        let n = ((end_hz - start_hz) / GRID_HZ).round().max(1.0) as usize + 1;
        Self {
            start_hz,
            bin_hz: GRID_HZ,
            power_db: vec![floor_db; n],
        }
    }
}

pub fn cmp_f32(a: f32, b: f32) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

pub fn percentile(values: &[f32], pct: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| cmp_f32(*a, *b));
    let idx = ((pct.clamp(0.0, 1.0) * (sorted.len() as f32 - 1.0)).round() as usize)
        .min(sorted.len() - 1);
    sorted[idx]
}

pub fn usable_half_hz(sample_rate: u32) -> f64 {
    (sample_rate as f64 / 2.0) * USABLE_NYQUIST_FRAC
}

pub fn hop_step_hz(usable_half: f64) -> f64 {
    (usable_half * 0.73).clamp(DC_BLANK_HZ * 2.0, usable_half)
}

pub fn hop_lo_hz(start_hz: f64, end_hz: f64, usable_half: f64, step: f64) -> Vec<f64> {
    let first = start_hz + usable_half;
    let last = end_hz - usable_half;
    if last <= first {
        return vec![(start_hz + end_hz) * 0.5];
    }
    let mut los = Vec::new();
    let mut lo = first;
    while lo < last - 1.0 {
        los.push(lo);
        lo += step;
    }
    match los.last_mut() {
        Some(prev) if (last - *prev).abs() < step * 0.25 => *prev = last,
        Some(_) => los.push(last),
        None => los.push(last),
    }
    los
}

struct SpectrumEngine {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    scratch: Vec<Complex<f32>>,
    buf: Vec<Complex<f32>>,
}

impl SpectrumEngine {
    fn new() -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();
        Self {
            scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            fft,
            window: hann(FFT_SIZE),
            buf: vec![Complex::new(0.0, 0.0); FFT_SIZE],
        }
    }

    fn welch_power(&mut self, iq: &[Complex32]) -> Vec<f64> {
        let n = self.fft.len();
        let hop = n / 2;
        let mut acc = vec![0.0f64; n];
        let mut count = 0usize;
        let mut offset = 0usize;
        while offset + n <= iq.len() {
            let frame = &iq[offset..offset + n];
            for ((slot, sample), &w) in self.buf.iter_mut().zip(frame).zip(&self.window) {
                *slot = Complex::new(sample.re * w, sample.im * w);
            }
            self.fft
                .process_with_scratch(&mut self.buf, &mut self.scratch);
            for (acc_bin, sample) in acc.iter_mut().zip(self.buf.iter()) {
                *acc_bin += sample.norm_sqr() as f64;
            }
            count += 1;
            offset += hop;
        }
        if count == 0 {
            return acc;
        }
        let scale = 1.0 / count as f64;
        for p in &mut acc {
            *p *= scale;
        }
        acc
    }
}

fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos())
        .collect()
}

fn bin_freq_hz(k: usize, fft_size: usize, sample_rate: u32, lo_hz: f64) -> f64 {
    let n = fft_size as f64;
    let fs = sample_rate as f64;
    let offset = if k < fft_size / 2 {
        k as f64 * fs / n
    } else {
        k as f64 * fs / n - fs
    };
    lo_hz + offset
}

struct SpectrumAccumulator {
    start_hz: f64,
    end_hz: f64,
    bin_hz: f64,
    power: Vec<f64>,
    weight: Vec<f64>,
}

impl SpectrumAccumulator {
    fn new(start_hz: f64, end_hz: f64, bin_hz: f64) -> Self {
        let n = ((end_hz - start_hz) / bin_hz).round() as usize + 1;
        Self {
            start_hz,
            end_hz,
            bin_hz,
            power: vec![0.0; n],
            weight: vec![0.0; n],
        }
    }

    fn accumulate_hop(
        &mut self,
        lo_hz: f64,
        power: &[f64],
        sample_rate: u32,
        usable_half: f64,
    ) {
        let fft_size = power.len();
        for (k, &p) in power.iter().enumerate() {
            if p <= 0.0 {
                continue;
            }
            let freq = bin_freq_hz(k, fft_size, sample_rate, lo_hz);
            if freq < self.start_hz || freq > self.end_hz {
                continue;
            }
            let df = (freq - lo_hz).abs();
            if df <= DC_BLANK_HZ || df > usable_half {
                continue;
            }
            let idx = ((freq - self.start_hz) / self.bin_hz).round();
            if idx < 0.0 {
                continue;
            }
            let idx = idx as usize;
            if let (Some(acc), Some(w)) = (self.power.get_mut(idx), self.weight.get_mut(idx)) {
                *acc += p;
                *w += 1.0;
            }
        }
    }

    fn finish(self) -> BandSpectrum {
        let mut power_db = vec![f32::NEG_INFINITY; self.power.len()];
        for ((acc, weight), out) in self.power.iter().zip(&self.weight).zip(power_db.iter_mut()) {
            if *weight > 0.0 {
                *out = (10.0 * (*acc / *weight).log10()) as f32;
            }
        }
        fill_gaps(&mut power_db);
        BandSpectrum {
            start_hz: self.start_hz,
            bin_hz: self.bin_hz,
            power_db,
        }
    }
}

fn fill_gaps(power_db: &mut [f32]) {
    let finite: Vec<f32> = power_db.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        power_db.fill(-200.0);
        return;
    }
    let fill = percentile(&finite, 0.2);
    for v in power_db.iter_mut() {
        if !v.is_finite() {
            *v = fill;
        }
    }
}

/// Synthetic spectrum for mock scans / unit tests.
pub fn synthetic_for_shape(plan: &ScanPlan) -> BandSpectrum {
    let mut spec = BandSpectrum::empty_floor(
        plan.band_start_hz as f64,
        plan.band_end_hz as f64,
        -80.0,
    );
    let mid = (plan.band_start_hz + plan.band_end_hz) / 2;
    match plan.shape {
        crate::radio::DetectShape::WbfmHaystack => {
            paint_haystack(&mut spec, 88_000_000.0, -55.0, -80.0, 60_000.0, 100_000.0);
            paint_haystack(&mut spec, 101_500_000.0, -52.0, -80.0, 60_000.0, 100_000.0);
            paint_haystack(&mut spec, 105_200_000.0, -54.0, -80.0, 60_000.0, 100_000.0);
        }
        crate::radio::DetectShape::Nbfm | crate::radio::DetectShape::Apt => {
            paint_haystack(&mut spec, mid as f64, -50.0, -80.0, 8_000.0, 15_000.0);
            if plan.band_start_hz < 138_000_000 {
                paint_haystack(&mut spec, 137_100_000.0, -48.0, -80.0, 15_000.0, 25_000.0);
            }
        }
        crate::radio::DetectShape::Am => {
            paint_haystack(&mut spec, mid as f64, -50.0, -80.0, 3_000.0, 6_000.0);
        }
        crate::radio::DetectShape::CwNeedle => {
            paint_spur(&mut spec, mid as f64, -45.0, 300.0);
            paint_spur(&mut spec, mid as f64 + 1_200.0, -48.0, 250.0);
        }
        crate::radio::DetectShape::Energy => {
            paint_spur(&mut spec, mid as f64, -40.0, 80_000.0);
        }
    }
    spec
}

pub fn paint_haystack(
    spec: &mut BandSpectrum,
    center_hz: f64,
    peak_db: f32,
    floor_db: f32,
    half_top: f64,
    half_bot: f64,
) {
    let start = spec.start_hz;
    let step = spec.bin_hz;
    for (i, bin) in spec.power_db.iter_mut().enumerate() {
        let f = start + i as f64 * step;
        let d = (f - center_hz).abs();
        let excess = if d <= half_top {
            peak_db - floor_db
        } else if d >= half_bot {
            0.0
        } else {
            let t = ((half_bot - d) / (half_bot - half_top)) as f32;
            (peak_db - floor_db) * t
        };
        *bin = bin.max(floor_db + excess);
    }
}

pub fn paint_spur(spec: &mut BandSpectrum, center_hz: f64, peak_db: f32, width_hz: f64) {
    let start = spec.start_hz;
    let step = spec.bin_hz;
    for (i, bin) in spec.power_db.iter_mut().enumerate() {
        let f = start + i as f64 * step;
        if (f - center_hz).abs() <= width_hz * 0.5 {
            *bin = bin.max(peak_db);
        }
    }
}

/// Single-frequency dwell for poll mode.
pub fn dwell_power(freq_hz: u64, sample_rate: u32) -> Result<BandSpectrum, String> {
    let iq = capture_iq(freq_hz, sample_rate, SPECTRUM_DWELL)?;
    let mut eng = SpectrumEngine::new();
    let power = eng.welch_power(&iq);
    let half = usable_half_hz(sample_rate);
    let start = freq_hz as f64 - half;
    let end = freq_hz as f64 + half;
    let mut acc = SpectrumAccumulator::new(start, end, GRID_HZ);
    acc.accumulate_hop(freq_hz as f64, &power, sample_rate, half);
    Ok(acc.finish())
}

/// Hop-and-stitch survey across `plan` band.
pub fn survey_band(
    plan: &ScanPlan,
    quit: &AtomicBool,
    mut progress: impl FnMut(ScanProgress),
) -> Result<BandSpectrum, String> {
    let sample_rate = plan.sample_rate_hz;
    let usable = usable_half_hz(sample_rate);
    let step = hop_step_hz(usable);
    let los = hop_lo_hz(
        plan.band_start_hz as f64,
        plan.band_end_hz as f64,
        usable,
        step,
    );
    let mut eng = SpectrumEngine::new();
    let mut acc = SpectrumAccumulator::new(
        plan.band_start_hz as f64,
        plan.band_end_hz as f64,
        GRID_HZ,
    );
    let total = los.len() as u32;
    for (i, &lo) in los.iter().enumerate() {
        if quit.load(Ordering::Relaxed) {
            break;
        }
        progress(ScanProgress {
            phase: ScanPhase::Spectrum,
            current: (i + 1) as u32,
            total,
            hz: lo,
        });
        let iq = capture_iq(lo.round() as u64, sample_rate, SPECTRUM_DWELL)?;
        let power = eng.welch_power(&iq);
        acc.accumulate_hop(lo, &power, sample_rate, usable);
    }
    Ok(acc.finish())
}

fn capture_iq(
    freq_hz: u64,
    sample_rate: u32,
    dwell: Duration,
) -> Result<Vec<Complex32>, String> {
    use crate::dsp::silence;

    let dev = silence::silenced(|| Device::<GenericDevice>::from_args("driver=soapy,soapy_driver=rtlsdr"))
        .or_else(|_| silence::silenced(|| Device::<GenericDevice>::from_args("driver=rtlsdr")))
        .map_err(|e| format!("open device: {e}"))?;

    silence::silenced(|| {
        let _ = dev.set_frequency(Direction::Rx, 0, freq_hz as f64);
        let _ = dev.set_sample_rate(Direction::Rx, 0, sample_rate as f64);
        let _ = dev.set_gain(Direction::Rx, 0, DEFAULT_GAIN_DB);
    });

    let mut rx = dev
        .rx_streamer(&[0])
        .map_err(|e| format!("rx streamer: {e}"))?;
    rx.activate()
        .map_err(|e| format!("activate: {e}"))?;

    let need = ((sample_rate as f64) * dwell.as_secs_f64()) as usize;
    let need = need.max(FFT_SIZE * 4);
    let mut out = Vec::with_capacity(need);
    let mut buf = vec![Complex32::new(0.0, 0.0); 16_384];
    let deadline = std::time::Instant::now() + dwell + Duration::from_millis(200);
    while out.len() < need && std::time::Instant::now() < deadline {
        match rx.read(&mut [&mut buf[..]], 200_000) {
            Ok(n) if n > 0 => out.extend_from_slice(&buf[..n]),
            _ => {}
        }
    }
    let _ = rx.deactivate();
    if out.len() < FFT_SIZE {
        return Err("not enough IQ samples".into());
    }
    Ok(out)
}
