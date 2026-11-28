# Pinball sound effects

Give pinball 8-bit sound effects, synthesised in code (no audio files),
playing on the desktop and in the browser, with a mute key.

## Design

- **`crates/sfx`** (new, no bevy): a sound is a list of notes (square,
  triangle or noise wave; start and end pitch; length; volume), rendered
  to mono `f32` samples at 22050 Hz with short fades so notes do not
  click, and to 16-bit WAV bytes. Noise is an NES-style LFSR, so renders
  are deterministic and testable.
- **`sandbox_ui::audio`**: an `Audio` resource holding the game's named
  sounds and a queue; `audio.play(name)` queues, a `Last` system plays.
  - Desktop: bevy `AudioSource` from the WAV bytes (the `audio` and `wav`
    features are already on there), one `AudioPlayer` entity per sound
    played, despawned when done.
  - Browser: bevy's audio stays out of the build (no size cost). The page
    creates an `AudioContext` on the first tap or key press (browsers only
    allow sound after a gesture, and iOS only inside the handler); the game
    copies its samples into Web Audio buffers and plays them with buffer
    source nodes.
- **`pinball-core`**: the field records what happened each step as
  `Sound`s (flipper up, bumper, slingshot, lane, launch, hard wall knock,
  drain, bonus) for the game to take; wall knocks only above a speed and
  at most one per step.
- **`games/pinball`**: maps those to sounds, adds start and game over
  jingles, and `S` toggles the sound (help text and README updated).

## Checks

Unit tests for `sfx` (lengths, bounds, pitch by zero crossings, fades,
WAV header), for the field's sound events, and for the game's mute and
mapping; fmt, clippy (native and wasm), tests; a web build played in
headless Firefox with `AudioBufferSourceNode.start` counted, to show
sounds are started after a key press.
