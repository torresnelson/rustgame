Port pong to Rust and add it to the site

- Add `crates/pong-core`: rules from `game.c` (`score_manager`,
  `ball_reset`, `eye_ball`, `collisions`, `paddle_bounce`, `referee`,
  `menu_select`, touch control areas) and the levelled computer opponent
  from `computer.c` (`computer_update`, `predict_ball_y`, ...), with
  raylib's point and circle collision rules and a seeded `fastrand::Rng`
  for `GetRandomValue`
- Port all 13 tests of `tests/test_game.c` to `crates/pong-core/src/tests.rs`,
  plus tests for scoring, paddle clamping and circle collisions
- Add `games/pong`: `game.rs` holds the menu, playing, paused, match over
  and game over screens and draws them on the shared `Canvas` (pixel font
  text, pixel disc ball, solid grey touch arrows and pause button);
  `main.rs` maps keys, mouse and touches to events and runs the logic in
  fixed 60 Hz steps, queueing presses between steps
- Test the screens: menu keys, hover, click and tap; pause and continue;
  paddles by keys and by the left-third touch controller; the computer;
  match and game end; Esc only on the desktop; every text drawable and on
  screen in every state and mode
- Add `pong` to `xtask`'s `GAMES` (site path `/pong/`, last on the landing
  page); switch the unknown game example in the xtask tests to `snake`
- Document the game, its controls and `crates/pong-core` in `README.md`;
  list `/pong/` in the `pages.yml` header

## Deferred

- Portrait layout: Pong stays 800x450 landscape, letterboxed when the
  phone is upright (the C version asks to hold it sideways)
- Desktop and phone play-testing by the owner; screens were checked by
  rendering the canvas headlessly, not in a window or browser
- Sound effects (none in the C version either)

## Stats

- 1 commit, 16 files changed (10 new)
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`
  and `cargo test --workspace` clean; 31 new tests (16 core, 15 game)
- `cargo xtask build-web pong` builds the browser version

(this patch was generated in some part by
[`claude-code`][claude-code-gh])
[claude-code-gh]: https://github.com/anthropics/claude-code
