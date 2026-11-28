# Repository analysis

Snapshot of torresnelson/rustgame at `master` `fc55711` (PR #10), taken
2026-11-28. The open PR #11 (`new-games`: Snake, Minesweeper, Merge,
Bricks) is noted where it matters but not reviewed. Local `main`
(`b172e03`) is 18 commits behind `master`, the renamed default branch;
it should be deleted or reset so nobody analyses or branches from it.

## Verdict

A small, tidy, well-tested workspace in good shape. The architecture
(pure logic crates, a thin shared bevy layer, one app per game, a CPU
pixel canvas) is sound and consistently applied. CI is green: 199 tests,
0 failures, fmt and `clippy -D warnings` clean, wasm builds for all four
games. There are no correctness bugs of consequence. The real issues are:

1. **Download size.** Each game ships a 16.6 MB wasm (about 5 MB
   compressed) that still contains bevy's whole wgpu render stack, which
   the browser build never starts.
2. **Frame-rate dependent sand.** The sand world steps once per rendered
   frame, so it runs 2x to 2.4x faster on 120/144 Hz screens.
3. **A dead high-score path.** `tetris-core`'s file API (`highscores.json`
   plus legacy `scores.txt` import) is unused; the desktop game saves
   elsewhere, so its Python-compatibility promise only holds in the
   browser.

## Shape

| Area | Files | Lines | Tests |
|---|---|---|---|
| `crates/life` | 2 | 496 | 20 |
| `crates/sand` (+ bench example) | 4 | 804 | 21 |
| `crates/tetris-core` | 2 | 692 | 29 |
| `crates/pong-core` | 3 | 721 | 16 |
| `crates/pixelfont` | 1 | 246 | 9 |
| `crates/touch` | 1 | 263 | 10 |
| `crates/sandbox-ui` (+ demo) | 10 | 1 253 | 17 |
| `games/*` (4 apps) | 14 | 4 480 | 65 |
| `xtask` | 2 | 463 | 12 |
| `web/` templates | 2 | 196 | - |

Lines are `.rs` and `.html` sources, tests included; test counts are
`#[test]` functions.

About 10 800 lines in total, Rust 2024 edition, toolchain pinned to
1.99.1, bevy `=0.20.1` with trimmed features, `unsafe_code = "forbid"`
workspace-wide. Dependencies beyond bevy: `fastrand`, `serde`,
`serde_json`, and `web-sys`/`js-sys`/`wasm-bindgen` on wasm only.

### Layers

```
pixelfont   touch ---------+          (no bevy; pure, unit-tested)
life  sand  tetris-core  pong-core
     \       |       /
      sandbox-ui  (bevy: Canvas, display, Pointers, Orientation,
     /    |    \    storage, clock, web glue)
game-of-life  tetris  sand-simulator  pong   (one bevy App each)
```

Each game is the same three-part shape:

- `main.rs`: one bevy `Update` system turns keys, typed characters and
  `Pointers` into game `Event`s.
- `game.rs`: a plain `Game` struct with `handle(event) -> Outcome`,
  `update(now_ms)` and `draw(&mut Canvas)`; no ECS inside.
- `game/tests.rs`: drives `Game` directly, including layout checks (fit,
  no overlap, every key has a button).

Rendering: games paint a CPU RGBA `Canvas` every frame. On desktop
`display::gpu` uploads it to a nearest-sampled texture on one sprite (plus
a scanline sprite). In the browser `web_display` skips bevy's renderer
(`backends: None`, `crates/sandbox-ui/src/lib.rs:60`) and blits through
the page canvas's 2D context. This is the fix for the phone black screen
(`plans/claude/mobile-black-screen.md`, PR #7).

## Strengths

- **Logic is engine-free.** All rules, physics, scoring, gestures and
  font geometry live in crates without bevy, so tests run in milliseconds
  and port one-to-one from the Python suites.
- **Games are testable without a GPU.** The `Game` structs take events
  and draw to a `Canvas`, so even menus and layouts have tests.
- **Low panic surface.** Outside tests there are three `expect`s, all on
  real invariants (`'?'` glyph exists, serde of plain structs,
  `xtask` has a parent dir). No `unsafe`.
- **Careful sand engine.** Flat row-major vectors, per-cell tick stamps
  (one move per tick), alternating scan direction, density displacement,
  friction and scatter; a bench example (`crates/sand/examples/bench.rs`).
- **Good input layer.** One `Pointers` list for mouse and multi-touch with
  per-id press/move/release, and positions mapped through the canvas's
  CSS size to dodge device-pixel-ratio mismatches.
- **Solid build and CI.** `cargo xtask build-web` is small, tested and
  fails soft without `wasm-opt`. CI runs fmt, clippy `-D warnings`,
  tests and a wasm build; Pages publishes on push to `master`.
- **Documented decisions.** `plans/claude/` records the rewrite plan, the
  pong port and the black-screen investigation with evidence.

## Findings

Ordered by impact. Each has a location and a suggested fix.

### 1. The browser build ships an unused renderer (size)

`Cargo.toml` enables bevy's `2d` and `webgl2` features for every target.
In the browser the renderer is never started (`backends: None`), yet
wgpu, naga, the sprite and core pipelines and their shaders are all
compiled into each 16.6 MB wasm. All four games carry their own copy.

Fix: give the games a wasm-only bevy feature set without
`bevy_render`/`bevy_sprite`/`bevy_core_pipeline`/`webgl2` (resolver 3
keeps target-specific feature sets apart), and gate `display::gpu` and
the `RenderPlugin` override on desktop only. `DefaultPlugins` then drops
the render plugins by itself. A size check in the Pages log would show
the gain. Expect a large cut, but measure: bevy's window, input and
time plugins remain.

### 2. Sand physics speed depends on the frame rate (gameplay)

`games/sand-simulator/src/game.rs:304` calls `world.step()` once per
`update`, which runs once per frame. At 120 Hz sand falls twice as fast
as at 60 Hz, and slow phones simulate slower. Pong already solves this
with a fixed 60 Hz accumulator (`games/pong/src/game.rs:83`,
`steps_due`).

Fix: move `steps_due` into `sandbox-ui` (or `touch`) and use it in sand,
with the same per-frame cap so a slow device drops time rather than
spiralling.

### 3. Desktop high scores bypass the tested file API (dead code, parity)

`tetris-core::highscores::{load, save}` (with the one-time `scores.txt`
import) are only called by their own tests. The game uses
`sandbox_ui::storage::{load_text, save_text}` with key
`tetris.highscores`, which on desktop writes
`~/.local/share/pygame-sandbox/tetris.highscores`. The module doc's
claim that a table saved by either version loads in the other is
therefore true for localStorage only.

Fix, either:
- desktop uses `highscores::load/save` on the Python file location, or
- delete `load`/`save` and the legacy import, and narrow the doc to the
  browser.

### 4. Game of Life loses time between generations (minor)

`games/game-of-life/src/game.rs:365` sets `last_step_ms = now_ms`, which
discards how late the step was. A generation only fires on a frame, so
every step is up to one frame late and the lateness never comes back:
with frame jitter, 20 generations/s (50 ms) often lands on the fourth
60 Hz frame and runs nearer 15/s, and 60/s never exceeds one per frame.
Advancing by the interval instead (`last_step_ms += interval`, reset when
far behind) keeps the speed shown in the status bar.

### 5. Pages can deploy a build CI rejected (process)

`pages.yml` and `ci.yml` both run on push to `master` independently. A
merge that fails clippy or tests still publishes. Fix: run Pages on
`workflow_run` of CI succeeding, or add the test step to the Pages job.

### 6. `wasm-bindgen-cli` version is duplicated (fragile)

`.github/workflows/pages.yml:40` hard-codes `0.2.130`, which must match
`Cargo.lock`. A `cargo update` breaks Pages with a schema-mismatch
error. Fix: read it from the lockfile in the workflow (e.g.
`cargo metadata` + `jq`), or have `xtask` check the installed version
and print the exact install command. `shell.nix` takes wasm-bindgen from
nixpkgs, which has the same problem locally.

### 7. The loader gives up streaming compilation (load time)

`web/game.html:154` buffers the whole wasm for the progress bar, then
`init({ module_or_path: bytes })` compiles it. Wrapping the progress
reader in a `new Response(stream, { headers: { 'content-type':
'application/wasm' } })` and passing that to `init` keeps the bar and
lets the browser compile while downloading. Worth more once finding 1
is fixed and the network part is shorter.

### 8. The browser display redraws unchanged frames (efficiency)

`web_display::present` (`crates/sandbox-ui/src/web_display.rs:144`) runs
`putImageData` and two `drawImage`s every frame and reads
`devicePixelRatio`/client size each time. The desktop path checks
`is_changed()`. Games currently repaint every frame, so the gain needs
both sides; the cheap half is skipping the work when neither the canvas
nor the page size changed.

### 9. Stale names and small inconsistencies (hygiene)

- The `pygame-sandbox` name survives in `storage.rs:27` (desktop data
  folder), `shell.nix:20` and `sandbox-ui`'s description. The data folder
  is intentional if it is meant to share with Python; otherwise
  `rustgame` is clearer. Changing it needs a one-time migration.
- `shell.nix` uses nixpkgs `rustc`/`cargo`, not the pinned 1.99.1, so the
  local toolchain can differ from CI.
- `rust-version = "1.97.1"` is never tested; CI only runs 1.99.1. Either
  test the MSRV or set it to the pinned version.
- `web/game.html:7` disables zoom (`user-scalable=no`); deliberate for
  gestures, but worth noting for accessibility.
- Two `#[allow(clippy::too_many_arguments)]` on bevy systems
  (`games/tetris/src/main.rs:77`, `crates/sandbox-ui/src/pointer.rs:114`);
  normal for bevy, could be a `SystemParam` struct if more are added.
- Life uses `HashSet<(i32, i32)>` on a fixed 50x50 board. Fine at this
  size; a flat `Vec<bool>` would be faster and simpler if boards grow.

## Recommended order

| # | Change | Effort | Value |
|---|---|---|---|
| 1 | wasm-only bevy features without the renderer | M | High: much smaller downloads |
| 2 | Fixed-step sand via shared `steps_due` | S | High: same feel on every screen |
| 5 | Gate Pages on CI | S | Medium: no broken deploys |
| 3 | Resolve the desktop high-score path | S | Medium: removes dead code or keeps a promise |
| 6 | Single source for the wasm-bindgen version | S | Medium: no surprise Pages failures |
| 7 | Streaming compile in the loader | S | Medium after 1 |
| 4 | Drift-free life timing | XS | Low |
| 8 | Skip unchanged web frames | S | Low |
| 9 | Hygiene items | XS each | Low |

Housekeeping outside the code: delete or reset local `main` to
`origin/master`, and decide whether `origin/main` should be removed now
that `master` is the default.

## Unknowns

- Real size gain of finding 1; it depends on what bevy's non-render
  plugins pull in. Measure with the Pages log's `MB wasm` lines.
- Whether the Python desktop version is still used on the same machines;
  that decides the fix for finding 3.
- Tests and the wasm build were not re-run locally (no cargo outside
  `nix-shell`); figures come from CI run 46146287104 on `fc55711`.
