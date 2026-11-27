
# Black screen on phones: analysis

Status: **fixed** by option B (PR #7, `deed678`). In the browser the games
are drawn with the page canvas's 2D context, and bevy's renderer is not
started. Confirmed working by the owner on the phone in Firefox for
Android, and in headless Firefox with no WebGL at all. The exact cause
inside bevy's 2D pass (H2 or H5 below) was not pinned down; the fix does
not depend on it.
Bevy 0.20.1, wgpu 30 (GL backend on WebGL2), rustgame `main` at `b172e03`.

## Symptom

On the owner's phone (Android 16, Firefox 161), all three games load, the
loading screen goes away, and the screen stays black. Desktop Chromium,
desktop Firefox and Chromium's phone emulation all draw the games.

## Evidence

Gathered with the `?debug` page mode (PRs #2 to #5), on the phone:

| Fact | Source | Meaning |
|---|---|---|
| WebGL2 context created, `WebGL 2.0`, max texture 16384 | `[webgl2]` | WebGL2 works |
| GPU reported as "PowerVR SGX 540, or similar" | `[webgl2]`, `AdapterInfo` | Firefox's privacy bucket, not the real GPU; the real one is unknown |
| Limits: uniform block 65536, 12 blocks per stage, 1024 uniform vectors, 16 vertex attributes, 24 texture units, offset alignment 128, highp 23 bits | `[limits]` | Nothing close to what bevy needs; not a limits problem |
| Canvas CSS 432x826, backing 1080x2064, DPR 2.5 | `[canvas]` | Sized correctly |
| No panic, no `ERROR`, no WebGL context loss | log | The app runs normally |
| No shader compile or program link failure | `[gl]` hooks | The driver accepts every bevy shader |
| No `getError` after the first 300 draws of each kind | `[gl]` hooks | No draw is rejected |
| Per frame: 1 `drawArrays`, 1 `drawArraysInstanced`, 3 `drawElementsInstanced` (282 / 281 / 841 in about 6 s) | `[draws]` | Same call pattern as desktop Firefox, at about half its frame rate |
| `ClearColor` purple fills the screen; the orange check sprite and the game do not show | render check, PR #3 | Frames are presented; sprite draws produce no pixels |

The three `drawElementsInstanced` per frame are the three sprites: the game
canvas, the scanline overlay and the orange check square. So the sprite
draws are issued every frame, accepted by the driver, and leave no trace.

The purple is the 2D main pass clearing its colour target, after which the
frame reaches the screen. That clear, and any copy to the canvas, work. The
failure is limited to the draws inside the main pass.

## What a bevy sprite draw needs

From `bevy_sprite_render-0.20.1/src/render/mod.rs` and `sprite.wesl`:

1. **Instance-only vertex input.** All five attributes (locations 0 to 4,
   `Float32x4`) use `VertexStepMode::Instance`. There is no per-vertex
   attribute at all, so attribute 0 is instanced.
2. **Corners from `gl_VertexID`.** The quad's corners come from
   `vertex_index & 1` and `vertex_index & 2`, drawn with
   `draw_indexed(0..6, 0, range)`, which becomes `drawElementsInstanced`.
3. **First-instance emulation.** WebGL has no base instance, so wgpu-hal
   (`gles/command.rs`, `rebind_vertex_data`) rebinds the instance buffer at
   an offset per batch instead.
4. **Depth test.** `depth_compare: GreaterEqual`, no depth writes, against
   the 2D pass's `Depth32Float` depth texture, cleared to 0.0 (reverse Z).
5. Texture sampling and alpha blending.

The pass's clear and copy also use (5); they work. The draws differ from
them in (1) to (4).

## Hypotheses

Ranked by fit with the evidence:

**H1. Instanced draws with no per-vertex attribute misbehave on this
driver.** (1) to (3) are an unusual combination: an instanced attribute 0,
`gl_VertexID` with indexed instanced draws, and instance buffers rebound at
an offset. Firefox for Android passes WebGL to the native GLES driver
without ANGLE, so driver bugs surface directly. Desktop GL compatibility
contexts and some mobile drivers have known problems with an instanced
attribute 0. A wrong `gl_VertexID` or instance data would place every
vertex at the same point, giving zero-area triangles: no pixels, no error.
That matches the evidence exactly.

**H2. The depth test rejects every fragment.** If the `Depth32Float`
attachment is not really cleared to 0.0 on this driver (or the clear is
lost, or the format is emulated badly), `GreaterEqual` fails everywhere
while the colour clear still works. Again no pixels and no error.

**H3. Something about the canvas texture upload.** This is ruled out as the
*first* cause: the plain orange sprite fails too, and it uses bevy's
built-in white texture, not ours.

**H4. Limits, shader precision, context loss, canvas sizing, input.** Ruled
out by the logs above.

## Deciding test (live since PR #5)

`?debug` now draws an orange **sprite** and a green **2D mesh**
(`Mesh2d` + `ColorMaterial`). The mesh is depth-tested the same way
(`GreaterEqual`) but has a per-vertex position buffer at location 0, takes
its corners from that buffer (not `gl_VertexID`), and reads per-object data
from a uniform array by instance index.

| Phone shows | Points to | Fix in our code |
|---|---|---|
| green, not orange | H1 (instancing or `gl_VertexID`) | Show the game canvas as a `Mesh2d` quad instead of a sprite |
| neither | H2 (depth) | Draw without the depth-tested 2D pass, or present without wgpu (below) |
| both | Neither; the sprite failure is intermittent or order dependent | More checks: one sprite only, no scanlines |

## Result of the deciding test (2026-11-27, 18:51)

**Neither square shows** on the phone in Firefox: purple only. On the
**same phone in Chrome, everything works.**

- `[draws]`: `drawElementsInstanced` 954 in 239 frames is 4 per frame, the
  game canvas, the scanlines, the sprite and the mesh. All four are issued
  and accepted, and none produces pixels. The mesh is drawn instanced too
  (bevy batches 2D meshes).
- **H1 is ruled out.** The mesh has a per-vertex attribute 0 and takes its
  corners from a vertex buffer, and it fails all the same.
- **It is Firefox, not the GPU.** Firefox for Android passes WebGL to the
  native GLES driver; Chrome goes through its own translation layer.

Two causes remain. Both are shared by every draw in the 2D main pass, and
by nothing that works (the clear and the copy to the canvas):

- **H2, the depth test:** `GreaterEqual` against the `Depth32Float`
  attachment, cleared to 0.0.
- **H5, the view uniform:** every 2D draw reads `view.clip_from_world` from
  a uniform buffer bound with a dynamic offset (`bindBufferRange`; this
  device's alignment is 128). If that binding reads as zeros under
  Firefox's GLES path, every vertex lands on one point. No pixels, no error.

Telling H2 from H5 apart needs one more debug round. Either way, a fix
inside bevy's 2D pass works around one Firefox and driver behaviour, in a
renderer the games use only to scale one image. Option B avoids both.

## Options

**A. Mesh2d display** (if H1). A few lines in `sandbox-ui/src/display.rs`:
a `Rectangle` mesh with a `ColorMaterial` holding the canvas texture, and
the same for the scanlines. Keeps everything else as is.

**B. Present with Canvas 2D on the web** (works whatever the cause). Our
games already draw every frame on a CPU canvas; bevy's GPU renderer only
scales that image to the screen. On the web, a second `<canvas>` with a
2D context could show it with `putImageData` plus `drawImage` (smoothing
off), and the scanlines as a CSS overlay. Bevy would keep the window, input,
time and audio, with its renderer disabled on the web.
- Pros: no dependence on WebGL drivers (pygbag's version, which uses a 2D
  canvas, works on this phone). wgpu and naga leave the wasm, likely cutting
  the download well below today's 5 MB. Less GPU and battery use.
- Cons: a web-only display path to maintain next to the desktop one.
  Moderate work in `sandbox-ui`: a display module per target, canvas sizing,
  and the pointer mapping onto the 2D canvas.

**C. Report upstream.** With the deciding test's result, a minimal repro
(bevy's 2D sprite example) and the device name, file an issue on bevy or
wgpu. Worth doing in any case, but it does not unblock the games.

## Recommendation

1. Read the deciding test on the phone. It is already live.
2. If it is H1, ship A at once, since it is the smallest change. Then
   consider B as the long-term web path.
3. If it is H2, go straight to B. It removes the whole class of problem,
   rather than working around one driver behaviour at a time.
4. Ask for the phone model. Firefox hides the GPU, and the model gives it,
   for an upstream report and for knowing how common the problem is.

## Unknowns

- The real GPU (Firefox reports a privacy bucket).
- Whether other phones or browsers are affected. Only one device has been
  tested; Chromium's phone emulation runs on the desktop GPU and proves
  nothing about phone drivers.
- Whether bevy's own web examples fail the same way on this phone (asked,
  not answered).
