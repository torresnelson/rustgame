//! 8-bit sound effects synthesised in code, so the games ship no audio
//! files. A sound is a list of [`Note`]s played one after another, each a
//! square, triangle or noise wave sweeping between two pitches and two
//! volumes, as on the NES. [`render`] turns them into mono samples and
//! [`wav_bytes`] into a WAV file.

/// Samples per second of every rendered sound.
pub const SAMPLE_RATE: u32 = 22_050;
/// Every note fades in and out over this long, so it starts and stops
/// without a click.
const FADE_SECONDS: f32 = 0.004;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    /// High for `duty` of each cycle (0.125, 0.25 or 0.5 on the NES).
    Square {
        duty: f32,
    },
    Triangle,
    /// NES-style pseudo-random noise, clocked at the note's pitch.
    Noise,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub wave: Wave,
    /// Pitch at the start and the end, in Hz (swept smoothly).
    pub hz: (f32, f32),
    pub seconds: f32,
    /// Volume at the start and the end, 0 to 1.
    pub volume: (f32, f32),
}

impl Note {
    pub const fn new(wave: Wave, hz: (f32, f32), seconds: f32, volume: (f32, f32)) -> Self {
        Self {
            wave,
            hz,
            seconds,
            volume,
        }
    }

    /// Silence for `seconds`.
    pub const fn rest(seconds: f32) -> Self {
        Self::new(Wave::Triangle, (1.0, 1.0), seconds, (0.0, 0.0))
    }

    fn samples(&self) -> usize {
        (self.seconds * SAMPLE_RATE as f32).round() as usize
    }
}

/// The NES noise channel's 15-bit shift register.
struct Lfsr(u16);

impl Lfsr {
    fn step(&mut self) {
        let feedback = (self.0 ^ (self.0 >> 1)) & 1;
        self.0 = (self.0 >> 1) | (feedback << 14);
    }

    fn output(&self) -> f32 {
        if self.0 & 1 == 0 { 1.0 } else { -1.0 }
    }
}

/// The notes one after another, as samples from -1 to 1.
pub fn render(notes: &[Note]) -> Vec<f32> {
    let mut samples = Vec::with_capacity(notes.iter().map(Note::samples).sum());
    // A mixed seed: from 1 (the NES power-on value) the register is mostly
    // zeros for a few hundred steps, so the output leans one way: a thump.
    let mut noise = Lfsr(0x5B59);
    let fade = FADE_SECONDS * SAMPLE_RATE as f32;
    for note in notes {
        let count = note.samples();
        let mut phase = 0.0f32;
        for i in 0..count {
            let t = i as f32 / count.max(1) as f32;
            // Exponential sweep: equal steps in musical pitch.
            let hz = note.hz.0 * (note.hz.1 / note.hz.0).powf(t);
            phase += hz / SAMPLE_RATE as f32;
            let wrapped = phase >= 1.0;
            phase = phase.fract();
            let wave = match note.wave {
                Wave::Square { duty } => {
                    if phase < duty {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                Wave::Noise => {
                    if wrapped {
                        noise.step();
                    }
                    noise.output()
                }
            };
            let volume = note.volume.0 + (note.volume.1 - note.volume.0) * t;
            let edge = (i as f32).min((count - 1 - i) as f32);
            let envelope = (edge / fade).min(1.0);
            samples.push((wave * volume * envelope).clamp(-1.0, 1.0));
        }
    }
    samples
}

/// `samples` as a 16-bit mono WAV file at [`SAMPLE_RATE`].
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    bytes.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for &sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests;
