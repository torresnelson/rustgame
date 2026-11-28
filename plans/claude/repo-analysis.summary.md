Fix the nine findings of the repo analysis

Findings 1 to 3 (PR #12, `22681fa`):

- Build the browser games without bevy's renderer: the workspace `bevy`
  keeps only `std`, `async_executor`, `bevy_log` and `bevy_winit`;
  `sandbox-ui` adds the old desktop set on native targets and `web` on
  `wasm32`, and `game_app` drops the `RenderPlugin` override and sets
  `ClearColor` on the desktop only
- Cut pong's wasm from 16.57 MB to 5.10 MB (5.08 MB to 1.50 MB gzipped);
  confirm in headless Firefox that the loading screen goes, the canvas is
  sized and the menu draws
- Move `steps_due` into `sandbox_ui::clock`, replacing the copies in
  `games/pong` and `games/bricks`, and pace the sand physics with it via
  `Game::update(paints, steps)`
- Delete the unused `highscores::load`/`save` and the `scores.txt` import:
  the Python game keeps its file beside its own source
- Make `cargo xtask build-web` honour `CARGO_TARGET_DIR`

Findings 4 to 9 (PR #13):

- Keep Game of Life to its speed: `last_step_ms` advances by one interval
  and resets to now only when a whole interval behind
- Run `pages.yml` on `workflow_run` of a successful CI on `master`,
  checking out the commit CI tested
- Read the `wasm-bindgen-cli` version from `Cargo.lock` in Pages; have
  `cargo xtask build-web` compare it with the installed CLI first and
  print the install command on a mismatch (`locked_version`,
  `check_bindgen`)
- Stream the wasm into `init` through a counting `TransformStream`, so the
  browser compiles while downloading and the progress bar stays
- Skip the browser frame upload when `PixelCanvas` is unchanged, and the
  page redraw unless it or the page size changed
- Save desktop data under `rustgame/`, still loading saves from the old
  `pygame-sandbox/` folder (`load_text_from`); rename the shell and crate
  description; warn in `shell.nix` when `rustc` differs from
  `rust-toolchain.toml`; set `rust-version` to the pinned 1.99.1; document
  the no-zoom viewport

## Deferred

- No change for the two `clippy::too_many_arguments` allows on bevy
  systems and life's `HashSet` board: the analysis flags them only for
  later growth
- Not checked by hand: a desktop window, and whether Firefox took the
  streaming path or wasm-bindgen's silent fallback (the page loads and
  draws either way)

## Stats

- 3 commits (analysis, fixes 1 to 3, fixes 4 to 9) across PRs #12 and #13
- 306 tests passing (303 at the start), 0 failed
- `cargo fmt --check`, `clippy -D warnings` (native and `wasm32`) clean

(this patch was generated in some part by
[`claude-code`][claude-code-gh])
[claude-code-gh]: https://github.com/anthropics/claude-code
