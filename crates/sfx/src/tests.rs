use super::*;

const SQUARE: Wave = Wave::Square { duty: 0.5 };

/// Sign changes in `samples`: two per cycle of a clean wave.
fn crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|w| (w[0] > 0.0) != (w[1] > 0.0) && w[0] != 0.0 && w[1] != 0.0)
        .count()
}

#[test]
fn notes_last_as_long_as_asked_one_after_another() {
    let samples = render(&[
        Note::new(SQUARE, (440.0, 440.0), 0.1, (0.5, 0.5)),
        Note::rest(0.05),
    ]);
    assert_eq!(samples.len(), 2205 + 1103);
    assert!(
        samples[2205..].iter().all(|&s| s == 0.0),
        "the rest is silent"
    );
}

#[test]
fn samples_stay_within_the_volume() {
    for wave in [SQUARE, Wave::Triangle, Wave::Noise] {
        let samples = render(&[Note::new(wave, (300.0, 900.0), 0.2, (0.6, 0.2))]);
        assert!(samples.iter().all(|s| s.abs() <= 0.6 + 1e-6), "{wave:?}");
        assert!(samples.iter().any(|s| s.abs() > 0.5), "{wave:?} is audible");
    }
}

#[test]
fn a_square_wave_has_the_asked_pitch() {
    // 441 Hz for half a second: 220.5 cycles, two crossings each.
    let samples = render(&[Note::new(SQUARE, (441.0, 441.0), 0.5, (0.5, 0.5))]);
    let count = crossings(&samples) as i32;
    assert!((count - 441).abs() <= 3, "{count} crossings");
}

#[test]
fn a_rising_sweep_speeds_up() {
    let samples = render(&[Note::new(Wave::Triangle, (200.0, 800.0), 0.4, (0.5, 0.5))]);
    let (first, second) = samples.split_at(samples.len() / 2);
    assert!(crossings(second) > crossings(first) * 3 / 2);
}

#[test]
fn notes_fade_in_and_out_without_a_click() {
    let samples = render(&[Note::new(SQUARE, (300.0, 300.0), 0.1, (0.8, 0.8))]);
    assert_eq!(samples[0], 0.0);
    assert_eq!(*samples.last().unwrap(), 0.0);
    assert!(samples[1].abs() < 0.05 && samples[samples.len() - 2].abs() < 0.05);
}

#[test]
fn noise_is_repeatable_and_balanced() {
    let note = Note::new(Wave::Noise, (2000.0, 2000.0), 0.3, (0.5, 0.5));
    let samples = render(&[note]);
    assert_eq!(samples, render(&[note]), "same notes, same sound");
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    assert!(mean.abs() < 0.1, "mean {mean}");
    assert!(crossings(&samples) > 100, "it changes");
}

#[test]
fn wav_bytes_are_a_16_bit_mono_wav() {
    let bytes = wav_bytes(&[0.0, 1.0, -1.0]);
    let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let u16_at = |i: usize| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
    let i16_at = |i: usize| i16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(u32_at(4), bytes.len() as u32 - 8);
    assert_eq!(&bytes[8..16], b"WAVEfmt ");
    assert_eq!((u16_at(20), u16_at(22)), (1, 1), "PCM, mono");
    assert_eq!(u32_at(24), SAMPLE_RATE);
    assert_eq!(u16_at(34), 16);
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(u32_at(40), 6);
    assert_eq!((i16_at(44), i16_at(46), i16_at(48)), (0, 32767, -32767));
}
