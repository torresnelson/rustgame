# Pong port

Port Pong from torresnelson/pong (C + raylib, 800x450 at 60 fps, beta
0.6.1) to rustgame as a fourth game, playable on the desktop and in the
browser like the others, and publish it on the landing page.

## Source

```
include/pong.h     constants, Player/Ball/Computer, GameState, MenuOption, TouchControl
src/game.c         scores, ball reset, collisions, paddle bounce, menu wrap, touch areas
src/computer.c     levelled computer opponent (speed, reaction, aim error, prediction)
src/main.c         raylib loop: menu, playing, paused, match over, game over, drawing
tests/test_game.c  logic tests (no window)
```

## Decisions

- Same split as tetris: a bevy-free logic crate `crates/pong-core`
  (`game.c` + `computer.c`, tests from `test_game.c`) and a bevy app
  `games/pong` (`main.c`).
- Fixed 60 Hz logic: all C speeds are pixels per frame. The app adds up
  frame time and runs whole 1/60 s steps (at most a few per frame, so a
  stalled tab does not fast-forward). Presses that arrive between steps are
  queued for the next step; held keys and fingers are read each step.
- Randomness: raylib `GetRandomValue(min, max)` (inclusive) becomes a
  seeded `fastrand::Rng`; tests use fixed seeds.
- Collision helpers follow raylib: `CheckCollisionPointRec` is half-open,
  `CheckCollisionCircleRec` tests the closest point of the rectangle.
- Drawing goes on the shared CPU `Canvas` (opaque rectangle fills, pixel
  font) like the other games:
  - the ball is a pixel disc of radius 10, built from row fills;
  - raylib `DrawText` becomes pixel font text (scale 1 to 3, big title at
    scale 16), with texts shortened or right-aligned where the wider pixel
    font would leave the 800 px screen;
  - the translucent touch arrows become solid dim greys (white at 15 % and
    45 % over black), drawn under the field;
  - raylib colours (RED, BLUE, YELLOW, GRAY, GREEN) keep their values;
  - `DrawFPS` is dropped (debug only).
- Input parity with `main.c`: W/S and arrows, Enter/M/1/2 in the menu,
  mouse hover and click on menu options, Space pauses, M to menu, Enter
  rematch, Esc quits on the desktop only. Touches (not the mouse) drive the
  left-third paddle control, the pause button and the "tap to continue"
  taps, and switch the hints to their touch wording.
- Landscape only: the 800x450 canvas is letterboxed in a portrait window,
  as the C version asks to hold the phone sideways.
- Site path `/pong/`, title `Pong`, added last to `xtask`'s `GAMES` so the
  landing page lists it after the sand simulator.

## Steps

1. `crates/pong-core`: constants, `Player`, `Ball`, `State`,
   `MenuOption`, `TouchControl`, `score_manager`, `ball_reset`,
   `eye_ball`, `collisions`, `clamp_speed`, `paddle_bounce`, `referee`,
   `menu_select`, touch control areas; `computer` module. Port all 13 C
   tests.
2. `games/pong`: `game.rs` (screens, step, draw; no engine code) with
   tests for state changes, text fitting and the pixel font charset;
   `main.rs` (keys, pointers, fixed step, exit).
3. Workspace: `pong-core` workspace dependency; `xtask` `GAMES` entry and
   doc comment; `pages.yml` comment.
4. Docs: README game list, controls, layout, run line.
5. Checks in `nix-shell`: `cargo fmt --check`, `cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`, a wasm build
   of `pong` (`cargo xtask build-web pong --site <tmp>`), and a desktop
   launch screenshot if a display is available.

## Risks

- Text widths: the 6 px/char pixel font at scale 2 is wider than raylib's
  20 px font for long labels; tests check every label fits the screen.
- Frame pacing: the fixed step keeps the C speeds on 120 Hz screens and
  slow phones alike.
