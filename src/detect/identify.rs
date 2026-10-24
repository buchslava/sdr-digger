//! Optional post-detect identity. RDS dwell is expensive; mock fills labels.
//! Full fmradio RDS path can be wired here later — v1 labels strongest FM hits
//! with a placeholder when hardware identify is unavailable.

use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use super::{Hit, ScanEvent, ScanPhase, ScanProgress};

pub fn label_rds(hits: &mut [Hit], quit: &AtomicBool, tx: &Sender<ScanEvent>) {
    let n = hits.len().min(5);
    for (i, hit) in hits.iter_mut().take(n).enumerate() {
        if quit.load(Ordering::Relaxed) {
            break;
        }
        let _ = tx.send(ScanEvent::Progress(ScanProgress {
            phase: ScanPhase::Identify,
            current: (i + 1) as u32,
            total: n as u32,
            hz: hit.frequency_hz as f64,
        }));
        // Hardware RDS dwell (fmradio) is optional; leave label empty if unknown.
        // A future revision ports kitchen's 2.5 s RDS path here.
        let _ = hit;
    }
}
