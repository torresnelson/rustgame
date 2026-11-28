Give pinball a heavy ball and a livelier table

- Slow the ball in `pinball-core`: `GRAVITY` 0.2 -> 0.12, `MAX_SPEED`
  15 -> 12, launch 10-15 -> 8-12, softer wall and flipper bounces,
  weaker bumper and slingshot kicks, `KNOCK_SPEED` 4 -> 3
- Add a shuttle: a small bumper sliding across under the lanes that
  kicks with its own motion (250 points); bumpers share `hit_round`
- Add three drop targets as stretches of the left wall: a hard hit knocks
  one down (200), the last scores 2000 and raises the bank; the wall
  still bounces, so the table's shape never changes
- Add a ball save: a new ball draining in its first 10 s out of the
  lane is put back on the plunger once, multiplier kept
- Add `Event::Targets` and `Event::Saved` with TARGETS! and BALL SAVED
  banners, and `shuttle`, `target`, `targets` and `saved` sounds
- Draw the shuttle, the targets (dim once down) and a SAVE light between
  the flippers that blinks for its last 2 s
- Update the README

## Deferred

- Not played by a person: the new feel and the sounds were not checked
  by hand or by ear

## Stats

- 1 commit
- 361 tests passing (8 new), 0 failed
- `cargo fmt --check`, `clippy -D warnings` (native and `wasm32`) clean

(this patch was generated in some part by
[`claude-code`][claude-code-gh])
[claude-code-gh]: https://github.com/anthropics/claude-code
