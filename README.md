# rustgame

Small 8-bit games in Rust and [bevy](https://bevy.org), playable on the
desktop and in the browser, phones included:
https://torresnelson.github.io/rustgame/

- **Game of Life**: pick one of 13 classic patterns (or draw your own) and
  watch it evolve.
- **Sand Simulator**: pour sand and water, build stone walls. Every finger
  paints on its own, so one can paint while another picks a tool.
- **Tetris**: being ported.

They are a port of the Python/pygame versions in
[torresnelson/pygame](https://github.com/torresnelson/pygame); the plan is in
`plans/claude/rust-rewrite.md`.

## Controls

- Game of Life: tap a pattern to preview it and START to load it (a mouse
  click loads it straight away). On the board the buttons run/pause, step,
  change speed, reset, clear and go back; tap cells while paused. Keys:
  up/down + enter, space, n, + -, r, c, esc.
- Sand Simulator: paint with a finger or the left mouse button, erase with
  the right one; the toolbar picks sand, water, stone or the eraser, the
  brush size, pause, clear and menu. Keys: 1-4, [ ], mouse wheel, space, c,
  esc.

A phone held upright gets a portrait layout with bigger buttons, and the
games follow the phone when it is turned. On the desktop, `--portrait`
opens that layout.

## Layout

```
crates/pixelfont     5x7 bitmap font
crates/touch         buttons, gestures, canvas fitting
crates/life          Game of Life rules and patterns
crates/sand          falling-sand physics
crates/tetris-core   tetris rules and high score table
crates/sandbox-ui    shared bevy layer: pixel canvas, pointers, orientation, saves
games/*              one bevy app per game
web/                 page templates (landing page, game page with loading screen)
xtask/               build tasks
```

The game logic crates do not depend on bevy and carry most of the tests.

## Run

Bevy needs some native libraries; on NixOS `shell.nix` provides them and the
pinned toolchain (`rust-toolchain.toml`, with the `wasm32-unknown-unknown`
target):

    nix-shell
    cargo run -p game-of-life        # or sand-simulator
    cargo test --workspace

The first build compiles bevy and takes several minutes.

## Browser build

    cargo xtask build-web            # every game + landing page into site/
    cargo xtask build-web sandsimulator
    python3 -m http.server -d site   # then open http://localhost:8000

It needs `wasm-bindgen` (the version in `Cargo.lock`) and, optionally,
`wasm-opt` from binaryen for a smaller download. Each game is about 5 MB
compressed.

Every push to `main` runs `.github/workflows/ci.yml` (format, clippy, tests,
wasm build) and `.github/workflows/pages.yml`, which publishes the site.
