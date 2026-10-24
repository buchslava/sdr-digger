# SDR Digger

Case-first **RTL-SDR** workbench for radio experiments — broadcast FM, airband, marine, NOAA APT, amateur FM/SSB/CW, and digital *watch* modes. Terminal UI with Ratatui. No Tauri.

Built so beginners can dig the air with **coach text**, **family auto-detect**, and **savable case files**, then finish with manual VFO tuning.

## Features

- **Case profiles** — TOML recipes under `~/.sdr-digger/cases/` (bundled starters + your own)
- **Parameter groups** — Tune / Window / Strength / Voice / Readout (physics + intent, not SDR++ subsystem soup)
- **Family auto-detect** — kitchen-style Welch → OS-CFAR → shape test (WBFM haystack, NBFM, AM, APT poll, CW needles, energy)
- **Findings → listen → fine-tune** — detector parks you; family-specific step sizes unlock next
- **Demods** — WBFM, NBFM, AM, USB/LSB, CW (+ Morse translator via `ditdah`)
- **Coach panel** — always-on beginner guidance (antenna λ/4, PPM, Carson, gain overload)
- **Ukrainian UI** — `--lang uk` (or `ua`)
- **Mock mode** — `--test` without a dongle

## Prerequisites

### Build

- Rust 1.75+ (edition 2021)
- On Linux: `libasound2-dev` (ALSA) for audio

### RTL-SDR (when not using `--test`)

| Package | Purpose |
|---------|---------|
| `soapysdr` | SDR HAL |
| `librtlsdr` | RTL-SDR driver |
| `soapyrtlsdr` / `soapysdr-module-rtlsdr` | Soapy plugin (**required**) |

**macOS:** `brew install soapysdr librtlsdr soapyrtlsdr`  
**Debian/Ubuntu:** `sudo apt install libsoapysdr-dev soapysdr-module-rtlsdr librtlsdr-dev libasound2-dev`

Verify:

```bash
SoapySDRUtil --find
SoapySDRUtil --probe="driver=rtlsdr"
```

## Build & run

```bash
cargo run --release -- --test              # UI + mock spectrum, no dongle
cargo run --release -- --test --lang uk    # Ukrainian chrome + coach
cargo run --release                        # real RTL-SDR
```

## Keys

| Key | Action |
|-----|--------|
| `Space` | Start / stop stream |
| `g` / `F5` | Family scan |
| `c` | Case browser |
| `p` / `F9` | Parameter inspector |
| `w` | Save case |
| `F1` | Radio school |
| `f` | Digit VFO |
| `n` / `N` | Next / prev finding |
| `=` | PPM helper (peak vs dial) |
| `d` | Mute |
| `o` | Spectrum overlay |
| `←` / `→` | Nudge frequency |
| `q` | Quit |

## Cases & settings

```
~/.sdr-digger/settings.toml
~/.sdr-digger/cases/*.toml
```

Bundled cases are copied on first run (`fm-broadcast-r1`, `airband-am`, `noaa-apt`, `ham-10m-cw`, …).

## Architecture

```
UI thread (Ratatui)  ←FFT / audio→  DSP thread (FutureSDR)
        │                                │
        └──── DspCommand / ScanEvent ────┘
```

Scan is a separate hop/poll session (exclusive USB), then listen reconnects.

## License

MIT
