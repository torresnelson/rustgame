//! The pinball sound effects, synthesised at start-up (see the `sfx` crate).

use sfx::{Note, Wave, render};

const PULSE: Wave = Wave::Square { duty: 0.25 };
const THIN: Wave = Wave::Square { duty: 0.125 };
const SQUARE: Wave = Wave::Square { duty: 0.5 };

/// Every sound by name, as samples.
pub fn sounds() -> Vec<(&'static str, Vec<f32>)> {
    let note = Note::new;
    vec![
        // A short thwack: a burst of noise, then a low blip.
        (
            "flipper",
            render(&[
                note(Wave::Noise, (3000.0, 900.0), 0.025, (0.35, 0.15)),
                note(PULSE, (170.0, 90.0), 0.05, (0.3, 0.0)),
            ]),
        ),
        // The plunger's spring: rising noise.
        (
            "launch",
            render(&[
                note(Wave::Noise, (600.0, 2400.0), 0.16, (0.1, 0.35)),
                note(Wave::Noise, (2400.0, 2400.0), 0.08, (0.35, 0.0)),
            ]),
        ),
        // A bright ding.
        (
            "bumper",
            render(&[
                note(SQUARE, (660.0, 990.0), 0.04, (0.3, 0.25)),
                note(SQUARE, (990.0, 990.0), 0.07, (0.25, 0.0)),
            ]),
        ),
        (
            "sling",
            render(&[note(PULSE, (520.0, 300.0), 0.06, (0.3, 0.0))]),
        ),
        // Two rising chimes.
        (
            "lane",
            render(&[
                note(Wave::Triangle, (1047.0, 1047.0), 0.05, (0.45, 0.35)),
                note(Wave::Triangle, (1319.0, 1319.0), 0.08, (0.4, 0.0)),
            ]),
        ),
        // A major arpeggio up to the octave.
        (
            "bonus",
            render(&[
                note(THIN, (523.0, 523.0), 0.07, (0.3, 0.3)),
                note(THIN, (659.0, 659.0), 0.07, (0.3, 0.3)),
                note(THIN, (784.0, 784.0), 0.07, (0.3, 0.3)),
                note(THIN, (1047.0, 1047.0), 0.16, (0.3, 0.0)),
            ]),
        ),
        // A springy boing that bends up.
        (
            "shuttle",
            render(&[
                note(SQUARE, (220.0, 660.0), 0.06, (0.3, 0.3)),
                note(SQUARE, (660.0, 440.0), 0.06, (0.3, 0.0)),
            ]),
        ),
        // A clack: a click of noise and a low thud.
        (
            "target",
            render(&[
                note(Wave::Noise, (4000.0, 2000.0), 0.015, (0.4, 0.2)),
                note(PULSE, (220.0, 140.0), 0.05, (0.3, 0.0)),
            ]),
        ),
        // A quick run up and down and up again.
        (
            "targets",
            render(&[
                note(PULSE, (784.0, 784.0), 0.06, (0.3, 0.3)),
                note(PULSE, (988.0, 988.0), 0.06, (0.3, 0.3)),
                note(PULSE, (784.0, 784.0), 0.06, (0.3, 0.3)),
                note(PULSE, (1175.0, 1175.0), 0.16, (0.3, 0.0)),
            ]),
        ),
        // Relief: a falling slide caught by a rising one.
        (
            "saved",
            render(&[
                note(Wave::Triangle, (660.0, 330.0), 0.15, (0.45, 0.4)),
                note(Wave::Triangle, (330.0, 880.0), 0.2, (0.4, 0.0)),
            ]),
        ),
        (
            "knock",
            render(&[note(Wave::Noise, (900.0, 400.0), 0.04, (0.25, 0.0))]),
        ),
        // A falling tone.
        (
            "drain",
            render(&[note(SQUARE, (440.0, 110.0), 0.45, (0.3, 0.05))]),
        ),
        (
            "start",
            render(&[
                note(PULSE, (523.0, 523.0), 0.08, (0.3, 0.3)),
                note(PULSE, (784.0, 784.0), 0.12, (0.3, 0.0)),
            ]),
        ),
        // A slow walk down.
        (
            "over",
            render(&[
                note(Wave::Triangle, (392.0, 392.0), 0.15, (0.45, 0.4)),
                note(Wave::Triangle, (330.0, 330.0), 0.15, (0.4, 0.4)),
                note(Wave::Triangle, (262.0, 262.0), 0.15, (0.4, 0.4)),
                note(Wave::Triangle, (196.0, 196.0), 0.35, (0.4, 0.0)),
            ]),
        ),
    ]
}
