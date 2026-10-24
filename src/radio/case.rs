use serde::{Deserialize, Serialize};

use super::family::Family;
use super::params::{
    GainMode, RadioMode, ReadoutParams, StrengthParams, TuneParams, VoiceParams, WindowParams,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    #[default]
    Beginner,
    Weekend,
    Lab,
}

impl Difficulty {
    pub fn msg(self) -> crate::i18n::Msg {
        match self {
            Difficulty::Beginner => crate::i18n::Msg::DifficultyBeginner,
            Difficulty::Weekend => crate::i18n::Msg::DifficultyWeekend,
            Difficulty::Lab => crate::i18n::Msg::DifficultyLab,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DetectMode {
    #[default]
    Hop,
    Poll,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DetectShape {
    #[default]
    WbfmHaystack,
    Nbfm,
    Am,
    Apt,
    CwNeedle,
    Energy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DetectIdentify {
    #[default]
    None,
    Rds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectProfile {
    #[serde(default)]
    pub mode: DetectMode,
    #[serde(default)]
    pub band_start_hz: Option<u64>,
    #[serde(default)]
    pub band_end_hz: Option<u64>,
    #[serde(default)]
    pub raster_hz: Option<u64>,
    #[serde(default)]
    pub shape: DetectShape,
    #[serde(default)]
    pub identify: DetectIdentify,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoachText {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub caveats: Vec<String>,
    /// Ukrainian translation, nested under `[coach.uk]` in TOML. This must be
    /// a field named `uk` on `CoachText` (not a top-level `"coach.uk"` key on
    /// `CaseFile`) because `[coach.uk]` in TOML syntax nests inside the
    /// `[coach]` table rather than creating a dotted top-level key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uk: Option<Box<CoachText>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub frequency_hz: u64,
}

/// Full on-disk / in-memory recipe for a radio experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub tune: TuneParams,
    pub window: WindowParams,
    pub strength: StrengthParams,
    pub voice: VoiceParams,
    pub readout: ReadoutParams,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            tune: TuneParams::default(),
            window: WindowParams::default(),
            strength: StrengthParams::default(),
            voice: VoiceParams::default(),
            readout: ReadoutParams::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseFile {
    pub schema: u32,
    pub name: String,
    #[serde(default)]
    pub name_uk: Option<String>,
    pub slug: String,
    pub family: Family,
    #[serde(default)]
    pub difficulty: Difficulty,
    pub tune: TuneParams,
    pub window: WindowParams,
    pub strength: StrengthParams,
    pub voice: VoiceParams,
    #[serde(default)]
    pub readout: ReadoutParams,
    #[serde(default)]
    pub detect: DetectProfile,
    #[serde(default)]
    pub coach: CoachText,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
}

impl CaseFile {
    pub fn recipe(&self) -> Recipe {
        Recipe {
            tune: self.tune.clone(),
            window: self.window.clone(),
            strength: self.strength.clone(),
            voice: self.voice.clone(),
            readout: self.readout.clone(),
        }
    }

    pub fn apply_recipe(&mut self, recipe: &Recipe) {
        self.tune = recipe.tune.clone();
        self.window = recipe.window.clone();
        self.strength = recipe.strength.clone();
        self.voice = recipe.voice.clone();
        self.readout = recipe.readout.clone();
    }

    pub fn display_name(&self, lang: crate::i18n::Lang) -> &str {
        match lang {
            crate::i18n::Lang::Uk => self.name_uk.as_deref().unwrap_or(&self.name),
            crate::i18n::Lang::En => &self.name,
        }
    }

    pub fn coach_for(&self, lang: crate::i18n::Lang) -> (&CoachText, bool) {
        match lang {
            crate::i18n::Lang::Uk => match &self.coach.uk {
                Some(uk) => (uk.as_ref(), false),
                None => (&self.coach, true),
            },
            crate::i18n::Lang::En => (&self.coach, false),
        }
    }
}

impl Default for CaseFile {
    fn default() -> Self {
        Self {
            schema: 1,
            name: "Lab custom".into(),
            name_uk: Some("Лабораторія".into()),
            slug: "lab-custom".into(),
            family: Family::Lab,
            difficulty: Difficulty::Lab,
            tune: TuneParams::default(),
            window: WindowParams::default(),
            strength: StrengthParams {
                gain: GainMode::Manual,
                gain_db: 36.4,
            },
            voice: VoiceParams {
                mode: RadioMode::Off,
                ..VoiceParams::default()
            },
            readout: ReadoutParams::default(),
            detect: DetectProfile {
                mode: DetectMode::Window,
                shape: DetectShape::Energy,
                ..DetectProfile::default()
            },
            coach: CoachText {
                summary: "All locks off. Dig carefully.".into(),
                hints: vec!["You own every knob.".into()],
                caveats: vec![],
                uk: Some(Box::new(CoachText {
                    summary: "Усі блокування знято. Копайте обережно.".into(),
                    hints: vec!["Усі крутилки ваші.".into()],
                    caveats: vec![],
                    uk: None,
                })),
            },
            bookmarks: vec![],
        }
    }
}
