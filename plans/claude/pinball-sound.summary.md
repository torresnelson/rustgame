Add 8-bit sound effects to pinball

- Add `crates/sfx`: square, triangle and noise notes with pitch sweeps
  and volume ramps, rendered at 22050 Hz with 4 ms fades (no clicks) and
  to 16-bit WAV; the noise is an NES-style LFSR with a mixed seed (no
  offset thump)
- Add `sandbox_ui::audio`: an `Audio` resource of named sounds and a
  queue, played in `Last`; bevy `AudioSource` and `AudioPlayer` on the
  desktop, Web Audio buffers in the browser (bevy's audio stays out of
  the web build)
- Make the game page create or resume `window.gameAudio` on the first
  tap or key press, as browsers (and iOS inside the handler) require
- Record `Sound` events in `pinball-core`: flipper up (once per press),
  launch, bumper, slingshot, lane, bonus, drain, and hard wall knocks
  (at least `KNOCK_SPEED`, once a step)
- Map them to ten sounds in `games/pinball/src/sounds.rs`, add start and
  game over tunes (the tune stands in for the last drain), and an `S`
  key that turns sound off and on, with a SOUND OFF label
- Update the README, the template test and the crate docs

## Deferred

- No touch control to mute on phones (the device's volume does it); no
  sound in the other games yet, though `sandbox_ui::audio` is ready for
  them
- Not heard by a person: the desktop build and the sounds' feel were not
  checked by ear

## Stats

- 1 commit, 1 new crate (`sfx`)
- 353 tests passing (7 `sfx`, 3 more in `pinball-core`, 3 more in
  `pinball`, 2 in `sandbox-ui::audio`), 0 failed
- `cargo fmt --check`, `clippy -D warnings` (native and `wasm32`) clean
- Web build grows 9 KB (5.11 MB); in headless Firefox no sound before a
  gesture, then the context runs and the start, launch and flipper
  sounds start

(this patch was generated in some part by
[`claude-code`][claude-code-gh])
[claude-code-gh]: https://github.com/anthropics/claude-code
