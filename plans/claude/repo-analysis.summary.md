Fix the top three findings of the repo analysis

- Build the browser games without bevy's renderer: the workspace `bevy`
  keeps only `std`, `async_executor`, `bevy_log` and `bevy_winit`;
  `sandbox-ui` adds the old desktop set on native targets and `web` on
  `wasm32`, and `game_app` drops the `RenderPlugin` override and sets
  `ClearColor` on the desktop only
- Cut pong's wasm from 16.57 MB to 5.10 MB (5.08 MB to 1.50 MB gzipped);
  confirm in headless Firefox that the loading screen goes, the canvas is
  sized and the menu draws
- Move `steps_due` with `STEPS_PER_SECOND` and `MAX_STEPS_PER_FRAME` into
  `sandbox_ui::clock`, replacing the copies in `games/pong` and
  `games/bricks`
- Pace the sand physics by time: `Game::update(paints, steps)` steps the
  world `steps` times, fed by `steps_due`, so sand falls at the same speed
  on 60, 120 and 144 Hz screens
- Delete the unused `highscores::load`/`save` and the `scores.txt` import:
  the Python game keeps its file beside its own source, so a shared
  desktop file is not possible; the module doc now promises browser
  compatibility only
- Make `cargo xtask build-web` honour `CARGO_TARGET_DIR` via `target_dir`,
  found while measuring the sizes
- Update the README's download size and the analysis's status lines

## Deferred

- Findings 4 to 9 of `repo-analysis.md`: drift-free life timing, gating
  Pages on CI, one source for the wasm-bindgen version, streaming wasm
  compilation, skipping unchanged web frames, hygiene items
- Running the desktop builds by hand: their bevy features are unchanged
  and they pass clippy and tests, but no window was opened

## Stats

- 2 commits (analysis, fixes); the fixes touch 20 files, +189 -212 in code
- 301 tests passing (303 before: -5 dead or duplicate, +3 new), 0 failed
- `cargo fmt --check`, `clippy -D warnings` (native and `wasm32`) clean

(this patch was generated in some part by
[`claude-code`][claude-code-gh])
[claude-code-gh]: https://github.com/anthropics/claude-code
