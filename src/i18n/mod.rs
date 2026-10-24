//! UI language tables. Selected once at process start via `--lang`.

mod en;
mod uk;

/// Process language. Not persisted — restart with `--lang` to switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Uk,
}

impl Lang {
    /// Parse CLI token. Accepts `en`, `uk`, and `ua` (alias for `uk`).
    pub fn from_cli(code: &str) -> Result<Self, String> {
        match code.trim().to_ascii_lowercase().as_str() {
            "en" | "eng" | "english" => Ok(Lang::En),
            "uk" | "ua" | "ukr" | "ukrainian" | "українська" => Ok(Lang::Uk),
            other => Err(format!(
                "unknown --lang '{other}' (use: en, uk)"
            )),
        }
    }
}

/// Closed set of UI strings. Add a variant here and fill both `en` and `uk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    AppTitle,
    StatusDisconnected,
    StatusConnected,
    StatusMock,
    KeyQuit,
    KeyStartStop,
    KeyScan,
    KeyCases,
    KeyParams,
    KeySave,
    KeySchool,
    KeyVfo,
    KeyMute,
    KeyOverlay,
    KeyFindingsNext,
    KeyFindingsPrev,
    KeyPpm,
    KeyNav,
    KeyPage,
    KeyVfoKeys,
    GroupTune,
    GroupWindow,
    GroupStrength,
    GroupVoice,
    GroupReadout,
    GroupDisplay,
    CoachTitle,
    SpectrumTitle,
    WaterfallTitle,
    FindingsTitle,
    CaseBrowserTitle,
    InspectorTitle,
    SchoolTitle,
    ScanProgressTitle,
    SaveCaseTitle,
    ToastUnsaved,
    ToastSaved,
    ToastScanEmpty,
    ToastScanDone,
    ToastOverload,
    ToastEmptyGrass,
    ToastPpmApplied,
    ToastDeviceError,
    ConstraintNyquist,
    ConstraintCarson,
    ConstraintRtlRate,
    PostDetectTune,
    AntennaHint,
    PpmHelperHint,
    DcSpikeHint,
    FamilyBroadcastFm,
    FamilyAirband,
    FamilyMarine,
    FamilyWeatherSat,
    FamilyAmateurFm,
    FamilyAmateurSsb,
    FamilyAmateurCw,
    FamilyDigital,
    FamilyLab,
    DifficultyBeginner,
    DifficultyWeekend,
    DifficultyLab,
    ModeOff,
    ModeWbfm,
    ModeNbfm,
    ModeAm,
    ModeUsb,
    ModeLsb,
    ModeCw,
    ScanPhaseSpectrum,
    ScanPhaseIdentify,
    ScanPhaseDone,
    FindingsApply,
    FindingsBookmark,
    FindingsInspect,
    FindingsEmpty,
    SchoolIq,
    SchoolDb,
    SchoolCarson,
    SchoolPpm,
    SchoolAntenna,
    SchoolAirband,
    SchoolApt,
    SchoolCw,
    SchoolScan,
    BandVhfIi,
    BandAirband,
    BandMarine,
    BandNoaaApt,
    BandHam2m,
    BandHam70cm,
    BandHf,
    BandCustom,
    GainManual,
    GainAgc,
    MorseStrip,
    EscClose,
    EnterConfirm,
    DirtyCase,
    CoachFallbackUk,
}

/// Look up a static UI string for `lang`.
pub fn t(lang: Lang, msg: Msg) -> &'static str {
    match lang {
        Lang::En => en::lookup(msg),
        Lang::Uk => uk::lookup(msg),
    }
}
