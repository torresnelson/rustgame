Add Pinball as the ninth game

- Add `crates/pinball-core`: a 300 x 480 table of line-segment walls, a
  circle ball under gravity moved in 2 px substeps, capsule flippers whose
  surface velocity carries the ball, a plunger lane with a one-way gate,
  three pop bumpers, two slingshots, three rollover lanes raising a bonus
  multiplier up to 5x, and 3 balls a game (`Field::step(Held) -> Event`)
- Test the table without an engine: plunger shots in and back, drains and
  game over, a flipper shot, bumper and slingshot kicks, lanes and bonus,
  lane rotation, no tunnelling through a flipper, 110 throws that stay on
  the table, and no resting spot (every untouched ball drains)
- Add `games/pinball`: ready, playing, paused and over screens, landscape
  and portrait layouts, keys, mouse buttons and touch halves for the
  flippers, a hold-to-launch button, banners, best score saved under
  `pinball.best`
- Move pong's `fill_disc` to `sandbox_ui::widgets` and add
  `fill_capsule` for flippers and walls
- Register the game in the workspace, `xtask` `GAMES` (landing page and
  `/pinball/`), the README and the `pages.yml` comment

## Deferred

- Sound effects, tilt, a ball saver and multiball: not planned for this
  first table
- Portrait checked by the layout tests only, not in a phone browser

## Stats

- 1 commit, 2 new crates (`pinball-core`, `pinball`)
- 338 tests passing (17 in `pinball-core`, 13 in `pinball`), 0 failed
- `cargo fmt --check`, `clippy -D warnings` (native and `wasm32`) clean
- Web build 5.1 MB wasm; played in headless Firefox: loads, launches,
  draws

(this patch was generated in some part by
[`claude-code`][claude-code-gh])
[claude-code-gh]: https://github.com/anthropics/claude-code
