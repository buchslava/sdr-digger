//! Morse translator wrapping `ditdah` on rolling audio buffers.

use ditdah::decode_samples;

#[derive(Debug, Clone, Default)]
pub struct MorseStatus {
    pub text: String,
    pub last_error: Option<String>,
}

pub struct MorseDecoder {
    buffer: Vec<f32>,
    sample_rate: u32,
    pub status: MorseStatus,
    max_samples: usize,
}

impl MorseDecoder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            buffer: Vec::with_capacity(sample_rate as usize * 2),
            sample_rate,
            status: MorseStatus::default(),
            max_samples: sample_rate as usize * 2,
        }
    }

    pub fn push(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);
        if self.buffer.len() > self.max_samples {
            let drop = self.buffer.len() - self.max_samples;
            self.buffer.drain(..drop);
        }
        // Decode when we have ~1.5 s of audio.
        if self.buffer.len() >= (self.sample_rate as usize * 3 / 2) {
            self.decode_now();
        }
    }

    pub fn decode_now(&mut self) {
        match decode_samples(&self.buffer, self.sample_rate) {
            Ok(text) => {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    self.status.text = trimmed;
                }
                self.status.last_error = None;
            }
            Err(e) => {
                self.status.last_error = Some(e.to_string());
            }
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.status = MorseStatus::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ditdah::generator::MorseGenerator;
    use std::path::PathBuf;

    #[test]
    fn decode_generated_sos_wav() {
        let gen = MorseGenerator::new(12_000, 600.0, 20.0);
        let path = std::env::temp_dir().join("sdr-digger-sos.wav");
        gen.generate_wav_file("SOS", &path).expect("wav");
        let text = ditdah::decode_wav_file(&path).expect("decode");
        let _ = std::fs::remove_file(&path);
        assert!(
            text.to_uppercase().contains('S') || text.to_uppercase().contains("SOS"),
            "got {text:?}"
        );
        let _ = PathBuf::from(".");
    }
}
