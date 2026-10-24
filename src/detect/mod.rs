mod am;
mod cfar;
mod cluster;
mod cw;
mod energy;
mod identify;
mod nbfm;
mod plan;
mod spectrum;
mod wbfm;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;

use crate::radio::{CaseFile, DetectMode, DetectShape, Family};

pub use plan::{ScanPhase, ScanPlan, ScanProgress};
pub use spectrum::BandSpectrum;

/// One detector hit ready for the findings list.
#[derive(Debug, Clone)]
pub struct Hit {
    pub frequency_hz: u64,
    pub snr_db: f32,
    pub occupancy: f32,
    pub bandwidth_hz: f32,
    pub label: Option<String>,
    pub family: Family,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Progress(ScanProgress),
    Finished(Vec<Hit>),
    Failed(String),
}

/// Run a family scan on a background thread. Uses mock synthetic PSD when
/// `test_mode` is true (no hardware).
pub fn spawn_scan(
    case: CaseFile,
    test_mode: bool,
    tx: Sender<ScanEvent>,
    quit: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let plan = ScanPlan::from_case(&case);
        let result = if test_mode {
            run_mock_scan(&case, &plan, &tx, &quit)
        } else {
            run_hardware_scan(&case, &plan, &tx, &quit)
        };
        match result {
            Ok(hits) => {
                let _ = tx.send(ScanEvent::Progress(ScanProgress {
                    phase: ScanPhase::Done,
                    current: 1,
                    total: 1,
                    hz: case.tune.frequency_hz as f64,
                }));
                let _ = tx.send(ScanEvent::Finished(hits));
            }
            Err(e) => {
                let _ = tx.send(ScanEvent::Failed(e));
            }
        }
    })
}

fn run_mock_scan(
    case: &CaseFile,
    plan: &ScanPlan,
    tx: &Sender<ScanEvent>,
    quit: &AtomicBool,
) -> Result<Vec<Hit>, String> {
    let steps = match plan.mode {
        DetectMode::Poll => case.bookmarks.len().max(1),
        DetectMode::Hop => 8,
        DetectMode::Window => 1,
    };
    for i in 0..steps {
        if quit.load(Ordering::Relaxed) {
            return Ok(vec![]);
        }
        let hz = plan.band_start_hz as f64
            + (plan.band_end_hz.saturating_sub(plan.band_start_hz) as f64)
                * (i as f64 / steps.max(1) as f64);
        let _ = tx.send(ScanEvent::Progress(ScanProgress {
            phase: ScanPhase::Spectrum,
            current: (i + 1) as u32,
            total: steps as u32,
            hz,
        }));
        thread::sleep(Duration::from_millis(40));
    }

    let spec = spectrum::synthetic_for_shape(plan);
    let mut hits = detect_on_spectrum(&spec, case, plan);
    if case.detect.identify == crate::radio::DetectIdentify::Rds && !hits.is_empty() {
        let _ = tx.send(ScanEvent::Progress(ScanProgress {
            phase: ScanPhase::Identify,
            current: 1,
            total: hits.len().min(3) as u32,
            hz: hits[0].frequency_hz as f64,
        }));
        for (i, h) in hits.iter_mut().take(3).enumerate() {
            h.label = Some(format!("Mock-FM-{}", i + 1));
        }
    }
    Ok(hits)
}

fn run_hardware_scan(
    case: &CaseFile,
    plan: &ScanPlan,
    tx: &Sender<ScanEvent>,
    quit: &AtomicBool,
) -> Result<Vec<Hit>, String> {
    // Hardware hop/poll via seify streamer — keep listen graph stopped.
    match plan.mode {
        DetectMode::Poll => {
            let bookmarks: Vec<(String, u64)> = if case.bookmarks.is_empty() {
                vec![(String::new(), case.tune.frequency_hz)]
            } else {
                case.bookmarks
                    .iter()
                    .map(|b| (b.name.clone(), b.frequency_hz))
                    .collect()
            };
            let mut hits = Vec::new();
            let total = bookmarks.len() as u32;
            for (i, (name, freq)) in bookmarks.iter().enumerate() {
                if quit.load(Ordering::Relaxed) {
                    break;
                }
                let _ = tx.send(ScanEvent::Progress(ScanProgress {
                    phase: ScanPhase::Spectrum,
                    current: (i + 1) as u32,
                    total,
                    hz: *freq as f64,
                }));
                match spectrum::dwell_power(*freq, plan.sample_rate_hz) {
                    Ok(spec) => {
                        let local_hits = detect_on_spectrum(&spec, case, plan);
                        for mut h in local_hits {
                            if h.label.is_none() && !name.is_empty() {
                                h.label = Some(name.clone());
                            }
                            hits.push(h);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(cluster_unique(hits))
        }
        DetectMode::Hop | DetectMode::Window => {
            let progress = |p: ScanProgress| {
                let _ = tx.send(ScanEvent::Progress(p));
            };
            let spec = spectrum::survey_band(plan, quit, progress)?;
            let mut hits = detect_on_spectrum(&spec, case, plan);
            if case.detect.identify == crate::radio::DetectIdentify::Rds {
                identify::label_rds(&mut hits, quit, tx);
            }
            Ok(hits)
        }
    }
}

fn detect_on_spectrum(spec: &BandSpectrum, case: &CaseFile, plan: &ScanPlan) -> Vec<Hit> {
    match plan.shape {
        DetectShape::WbfmHaystack => wbfm::detect(spec, plan, case.family),
        DetectShape::Nbfm | DetectShape::Apt => nbfm::detect(spec, plan, case.family),
        DetectShape::Am => am::detect(spec, plan, case.family),
        DetectShape::CwNeedle => cw::detect(spec, plan, case.family),
        DetectShape::Energy => energy::detect(spec, plan, case.family),
    }
}

fn cluster_unique(mut hits: Vec<Hit>) -> Vec<Hit> {
    hits.sort_by(|a, b| {
        b.snr_db
            .partial_cmp(&a.snr_db)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out = Vec::new();
    for h in hits {
        if out
            .iter()
            .any(|o: &Hit| o.frequency_hz.abs_diff(h.frequency_hz) < 50_000)
        {
            continue;
        }
        out.push(h);
    }
    out.sort_by_key(|h| h.frequency_hz);
    out
}
