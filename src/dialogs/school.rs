use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::events::AppAction;
use crate::app::state::{AppMode, AppState};
use crate::i18n::{t, Lang, Msg};
use crate::ui::dialog_layout::centered_dialog_rect;
use crate::ui::theme::UiPalette;

const PAGES: &[Msg] = &[
    Msg::SchoolIq,
    Msg::SchoolDb,
    Msg::SchoolCarson,
    Msg::SchoolPpm,
    Msg::SchoolAntenna,
    Msg::SchoolAirband,
    Msg::SchoolApt,
    Msg::SchoolCw,
    Msg::SchoolScan,
];

pub fn draw(f: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let rect = centered_dialog_rect(area, 78, 22);
    f.render_widget(Clear, rect);
    let page = app.school_page.min(PAGES.len() - 1);
    let title = t(app.lang, PAGES[page]);
    let body = page_body(app.lang, page);
    let header = format!(
        " {} — {} ({}/{}) ",
        t(app.lang, Msg::SchoolTitle),
        title,
        page + 1,
        PAGES.len()
    );
    f.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(header)
                    .title_bottom(format!(
                        " ←→  {} ",
                        t(app.lang, Msg::EscClose)
                    ))
                    .borders(Borders::ALL)
                    .border_style(palette.accent_style()),
            ),
        rect,
    );
}

fn page_body(lang: Lang, page: usize) -> String {
    match (lang, page) {
        (Lang::En, 0) => "IQ samples are pairs (I, Q) — cosine and sine of the same wave.\n\
Together they tell amplitude and phase. Your RTL-SDR streams millions of these per second.\n\
Nyquist: you can see about ± sample_rate/2 around the tuned centre.".into(),
        (Lang::En, 1) => "Decibels count zeros. +3 dB ≈ ×2 power, +10 dB ≈ ×10.\n\
Spectrum plots use dB so your eyes can compare strong and weak stations.".into(),
        (Lang::En, 2) => "Carson: FM bandwidth ≈ 2(Δf + fm). Broadcast FM: ±75 kHz + 15 kHz → ~180 kHz.\n\
That is why WBFM needs a wide channel and ≥ ~1 MSPS IQ.".into(),
        (Lang::En, 3) => "Cheap dongle crystals are often 20–80 ppm off.\n\
Park on a known station, press = to estimate PPM from the spectrum peak.".into(),
        (Lang::En, 4) => "Quarter wave: λ/4 ≈ 75 / f_MHz metres of wire.\n\
At 100 MHz that is ~75 cm. A 10 cm stub hears almost nothing.".into(),
        (Lang::En, 5) => "Airband (118–137 MHz) uses AM, not FM.\n\
Raster 25 kHz (or 8.33 near busy airports). Channels are quiet — use squelch.".into(),
        (Lang::En, 6) => "NOAA APT satellites sing a ticking warble near 137 MHz during a pass.\n\
Empty scan is normal. De-emphasis OFF. Image decode is a later experiment.".into(),
        (Lang::En, 7) => "CW is a thin needle. Fine-tune in 10 Hz steps.\n\
Enable Morse readout — ditdah translates clean audio into text.".into(),
        (Lang::En, 8) => "Scan takes a Welch PSD, then tests each legal channel:\n\
OS-CFAR noise → radiometer energy → occupancy / Beq shape → blob clustering.\n\
That is the SDR++ click, written as a hypothesis test. Then you finish with the VFO.".into(),
        (Lang::Uk, 0) => "IQ — пари (I, Q): косинус і синус однієї хвилі.\n\
Разом вони дають амплітуду й фазу. RTL-SDR ллє мільйони таких вибірок за секунду.\n\
Найквіст: видно приблизно ± sample_rate/2 навколо центру.".into(),
        (Lang::Uk, 1) => "Децибели рахують нулі. +3 дБ ≈ ×2 потужності, +10 дБ ≈ ×10.\n\
Спектр у дБ, щоб око порівнювало сильні й слабкі станції.".into(),
        (Lang::Uk, 2) => "Carson: смуга FM ≈ 2(Δf + fm). Мовленнєве FM: ±75 кГц + 15 кГц → ~180 кГц.\n\
Тому WBFM потребує широкого каналу і ≥ ~1 MSPS IQ.".into(),
        (Lang::Uk, 3) => "Дешевий кварц донгла часто 20–80 ppm.\n\
Встаньте на відому станцію, натисніть = щоб оцінити PPM за піком спектра.".into(),
        (Lang::Uk, 4) => "Чверть хвилі: λ/4 ≈ 75 / f_МГц метрів дроту.\n\
На 100 МГц це ~75 см. Штир 10 см майже нічого не чує.".into(),
        (Lang::Uk, 5) => "Авіація (118–137 МГц) — AM, не FM.\n\
Растр 25 кГц (або 8.33 біля аеропортів). Канали тихі — увімкніть сквелч.".into(),
        (Lang::Uk, 6) => "NOAA APT співає цокаючий «факс» біля 137 МГц під час прольоту.\n\
Порожній скан — норма. Деемфазис ВИМК. Декод зображення — пізніше.".into(),
        (Lang::Uk, 7) => "CW — тонка голка. Крок 10 Гц.\n\
Увімкніть Морзе — ditdah перекладає чисте аудіо в текст.".into(),
        (Lang::Uk, 8) => "Скан будує Welch PSD, потім перевіряє кожен легальний канал:\n\
OS-CFAR → радіометр → зайнятість / Beq → кластеризація.\n\
Це клік у SDR++, записаний як перевірка гіпотези. Далі — ваш VFO.".into(),
        _ => String::new(),
    }
}

pub fn handle_key(app: &mut AppState, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc => {
            app.dialog = None;
            app.mode = AppMode::Normal;
        }
        KeyCode::Left => {
            if app.school_page > 0 {
                app.school_page -= 1;
            }
        }
        KeyCode::Right => {
            if app.school_page + 1 < PAGES.len() {
                app.school_page += 1;
            }
        }
        _ => {}
    }
    AppAction::Idle
}
