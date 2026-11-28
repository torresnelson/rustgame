# rustgame

Small 8-bit games in Rust and [bevy](https://bevy.org), playable on the
desktop and in the browser, phones included:
https://torresnelson.github.io/rustgame/

- **Game of Life**: pick one of 13 classic patterns (or draw your own) and
  watch it evolve.
- **Tetris**: the classic falling blocks, with a top-10 table of named
  high scores.
- **Sand Simulator**: pour sand and water, build stone walls. Every finger
  paints on its own, so one can paint while another picks a tool.
- **Pong**: against a computer that levels up as the game goes on, or
  two players on one keyboard; first to three matches of ten points.
- **Snake**: eat, grow and speed up on a walled 20x20 grid.
- **Minesweeper**: easy, medium and hard boards; the first opening is
  always safe, and the best time of each is kept.
- **Merge**: slide number tiles on a 4x4 board; equal tiles merge, aim for
  2048.
- **Bricks**: a paddle, a ball and four levels of bricks, faster every
  round.

Game of Life, Tetris and Sand Simulator are ports of the Python/pygame
versions in [torresnelson/pygame](https://github.com/torresnelson/pygame)
(plan: `plans/claude/rust-rewrite.md`). Pong is a port of the C/raylib game
in [torresnelson/pong](https://github.com/torresnelson/pong) (plan:
`plans/claude/pong-port.md`). Snake, Minesweeper, Merge and Bricks are
new to this repository, with their own look; each keeps its best score
(best times for Minesweeper) like Tetris keeps its table.

## Controls

- Game of Life: tap a pattern to preview it and START to load it (a mouse
  click loads it straight away). On the board the buttons run/pause, step,
  change speed, reset, clear and go back; tap cells while paused. Keys:
  up/down (or k/j) + enter, space, n, + -, r, c, esc.
- Tetris: tap rotates, drag sideways moves, drag down soft drops, flick down
  hard drops; PAUSE and END buttons. Held upright, thumb buttons `<` `ROT`
  `>` `DOWN` `DROP` sit under the board (left, right and down repeat while
  held). A high score asks for a name on an on-screen keyboard. Keys:
  arrows, enter, space, p, esc. Scores are kept in the browser's local storage, or
  in `~/.local/share/rustgame/` on the desktop (saves from the old
  `pygame-sandbox` folder still load).
- Sand Simulator: paint with a finger or the left mouse button, erase with
  the right one; the toolbar picks sand, water, stone or the eraser, the
  brush size, pause, clear and menu. Keys: 1-4, [ ], mouse wheel, space, c,
  esc.
- Pong: pick ONE PLAYER or TWO PLAYERS with up/down or w/s and enter, 1 or
  2, a click or a tap. Player one moves with w/s (and the arrows when
  playing alone), player two with the arrows; space pauses, m goes back
  to the menu when paused or after a win, enter asks for a rematch, esc
  quits on the desktop. On a phone, held sideways and against the
  computer: the left third of the screen is the paddle controller (top
  half up, bottom half down), the button at the top centre pauses, and a
  tap continues.

- Snake: arrows or WASD turn, enter starts, space or p pauses, esc ends
  the game. Swipe anywhere to turn and tap to start or go on; held
  upright, thumb buttons sit under the board.
- Minesweeper: pick a board with up/down + enter, 1 2 3, or a tap. Click
  or tap digs, right click or a held finger flags; DIG/FLAG swaps the two
  for taps. Digging an open number whose flags are all placed opens its
  other neighbours. Keys: arrows move a cursor, enter or space dig, f, n,
  esc.
- Merge: arrows or WASD slide, or swipe anywhere; NEW (or n) starts over.
- Bricks: left/right, A/D or the mouse move the paddle; space, enter, up
  or a click launch the ball, p pauses. On a phone drag anywhere to move
  the paddle (upright, there is a strip under the field for it) and tap
  to launch.

In every game but Pong, a phone held upright gets a portrait layout with
bigger buttons, and the game follows the phone when it is turned; on the
desktop, `--portrait` opens that layout. Pong is landscape only.

## Layout

```
crates/pixelfont     5x7 bitmap font
crates/touch         buttons, gestures, canvas fitting
crates/life          Game of Life rules and patterns
crates/sand          falling-sand physics
crates/tetris-core   tetris rules and high score table
crates/pong-core     pong rules and computer opponent
crates/snake-core    snake rules
crates/mines-core    minesweeper rules: safe first opening, flood, chords
crates/merge-core    sliding and merging tiles
crates/bricks-core   paddle, ball and brick physics, levels
crates/sandbox-ui    shared bevy layer: pixel canvas, pointers, orientation,
                     saves, 8-bit widgets
games/*              one bevy app per game
web/                 page templates (landing page, game page with loading screen)
xtask/               build tasks (`cargo xtask`, aliased in .cargo/config.toml)
plans/               design plans and their summaries
ai/                  prompt records for AI-assisted changes
```

The game logic crates do not depend on bevy and carry most of the tests.

## Run

Bevy needs some native libraries. On NixOS `shell.nix` provides them, the
pinned toolchain (`rust-toolchain.toml`, with the `wasm32-unknown-unknown`
target) and the browser build tools:

    nix-shell
    cargo run -p game-of-life        # or tetris, sand-simulator, pong,
                                     # snake, minesweeper, merge, bricks
    cargo test --workspace

On Debian or Ubuntu, install them with rustup and:

    sudo apt install pkg-config libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev

The first build compiles bevy and takes several minutes.

## Browser build

    cargo xtask build-web            # every game + landing page into site/
    cargo xtask build-web sandsimulator
    python3 -m http.server -d site   # then open http://localhost:8000

It needs `wasm-bindgen-cli` at the version of `wasm-bindgen` in
`Cargo.lock` (0.2.130) and, optionally, `wasm-opt` from binaryen for a
smaller download; `nix-shell` has both. Each game is about 1.5 MB compressed.

Every pull request and push to `master` runs `.github/workflows/ci.yml`
(format, clippy, tests, wasm build); every push to `master` also runs
`.github/workflows/pages.yml`, which publishes the site.
