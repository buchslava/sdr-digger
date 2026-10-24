use crate::i18n::{t, Lang, Msg};

use super::case::CaseFile;
use super::constraints::{check_recipe, ConstraintIssue};
use super::family::band_label_msg;
use super::params::RadioMode;
use super::Family;

#[derive(Debug, Clone)]
pub struct CoachContext {
    pub lang: Lang,
    pub after_detect: bool,
    pub overload: bool,
    pub empty_grass: bool,
    pub near_dc_spike: bool,
}

impl Default for CoachContext {
    fn default() -> Self {
        Self {
            lang: Lang::En,
            after_detect: false,
            overload: false,
            empty_grass: false,
            near_dc_spike: false,
        }
    }
}

/// Build coach panel lines for the current case + live context.
pub fn build_coach_lines(
    case: &CaseFile,
    ctx: &CoachContext,
    issues: &[ConstraintIssue],
) -> Vec<String> {
    let mut lines = Vec::new();
    let (coach, missing_uk) = case.coach_for(ctx.lang);
    if !coach.summary.is_empty() {
        lines.push(coach.summary.clone());
    }
    for h in coach.hints.iter().take(3) {
        lines.push(h.clone());
    }
    for c in &coach.caveats {
        lines.push(format!("! {c}"));
    }
    if missing_uk {
        let note = t(ctx.lang, Msg::CoachFallbackUk);
        if !note.is_empty() {
            lines.push(note.to_string());
        }
    }

    let freq_mhz = case.tune.frequency_hz as f64 / 1e6;
    let band = t(ctx.lang, band_label_msg(case.family, case.tune.frequency_hz));
    lines.push(format!("{band} · {freq_mhz:.3} MHz"));

    let qtr = 75.0 / freq_mhz.max(0.001);
    lines.push(format!(
        "{} ({qtr:.2} m)",
        t(ctx.lang, Msg::AntennaHint)
    ));

    if case.tune.frequency_hz < 24_000_000
        && matches!(
            case.family,
            Family::AmateurSsb | Family::AmateurCw | Family::Lab
        )
    {
        lines.push(t(ctx.lang, Msg::BandHf).to_string());
    }

    if ctx.after_detect {
        lines.push(t(ctx.lang, Msg::PostDetectTune).to_string());
    }
    if ctx.overload {
        lines.push(t(ctx.lang, Msg::ToastOverload).to_string());
    }
    if ctx.empty_grass {
        lines.push(t(ctx.lang, Msg::ToastEmptyGrass).to_string());
    }
    if ctx.near_dc_spike {
        lines.push(t(ctx.lang, Msg::DcSpikeHint).to_string());
    }
    if case.voice.mode == RadioMode::Off && case.family == Family::Digital {
        lines.push(t(ctx.lang, Msg::FamilyDigital).to_string());
    }

    for issue in issues {
        lines.push(t(ctx.lang, issue.msg).to_string());
    }

    // Always offer PPM tip for beginners on FM.
    if case.family == Family::BroadcastFm {
        lines.push(t(ctx.lang, Msg::PpmHelperHint).to_string());
    }

    lines
}

pub fn issues_for_case(case: &CaseFile) -> Vec<ConstraintIssue> {
    check_recipe(&case.recipe())
}
