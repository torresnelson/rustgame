# Pinball

Add an original 8-bit pinball table as the ninth game, built like the
others: rules and physics in a bevy-free `crates/pinball-core` with unit
tests, screens and input in `games/pinball` on `sandbox-ui`, published by
`cargo xtask build-web` at `/pinball/`.

## Table

Field 300 x 480 pixels, origin top-left, speeds per 1/60 s step
(`sandbox_ui::clock::steps_due` paces it, like pong and bricks).

```
 ____________________________
/  48  | 78 |138 |198 | 228 \  rollover lanes (posts between them)
|       ( )   ( )           |  gate | plunger lane, x 276..300
|            ( )            |  pop bumpers at (88,150), (188,150), (138,205)
|  \                    /   |  slingshots above the flippers
|   \__              __/    |  inlane slants to the flipper pivots
|      \__/      \__/       |  flippers, pivots (80,428) and (196,428)
|            drain          |[plunger]
```

- **Ball**: a circle of radius 5 under gravity 0.2 px/step^2, speed capped
  at 15 px/step, moved in substeps of at most 2 px so it cannot tunnel.
- **Walls**: line segments (zero thickness, the ball's radius does the
  rest) with restitution 0.45 and no bounce below 0.6 px/step, so a ball
  can rest and roll.
- **Flippers**: capsules 46 px long, radius 4, rest 0.5 rad below
  horizontal, raised 0.45 rad above, swinging 0.3 rad per step. Contact
  uses the flipper's surface velocity at the contact point, so a swing
  launches the ball and a held flipper cradles it.
- **Plunger lane**: the ball starts resting on the plunger; holding the
  launch control charges it over 0.75 s, releasing shoots it at 6 to 15
  px/step. A corner slant turns it into the table; a one-way gate closes
  the lane once the ball has left it. A weak shot falls back to the
  plunger.
- **Pop bumpers**: radius 14, kick the ball away at 6 px/step or more,
  100 points, flash when hit.
- **Slingshots**: kick at 4 px/step, 10 points.
- **Rollover lanes**: passing through lights a lane (50 points); all three
  lit raises the bonus multiplier (up to 5x) with a 1000 point bonus and
  clears them. Each flipper press shifts the lit lanes, as on real tables.
- **Balls**: 3 per game; a ball below the bottom edge drains and the next
  one waits on the plunger.

All points are multiplied by the bonus multiplier.

## Controls

- Keys: left/A/Z and right/D/M (or `/`) flippers, space or down pulls the
  plunger, enter or space starts, p pauses, esc quits on the desktop.
- Mouse: left and right buttons work the flippers.
- Touch: hold the left or right half of the screen for that flipper; a
  LAUNCH button (hold, release) works the plunger; a pause button.
- Portrait layout for phones held upright, `--portrait` on the desktop.
- Best score saved under `pinball.best` (localStorage / data folder).

## Steps

1. `crates/pinball-core`: geometry (segment and circle contacts), `Field`
   with `step(Held) -> Event`, flippers, plunger, bumpers, lanes, scoring,
   tests (containment, no tunnelling, drain, flipper shot, bumper kick,
   lanes, plunger).
2. Move pong's `fill_disc` to `sandbox_ui::widgets` and add a capsule
   helper for flippers and walls.
3. `games/pinball`: `Game` with Ready/Playing/Paused/Over, landscape and
   portrait `Layout`, input, drawing, tests (layouts fit, touch halves,
   launch button, pause, best score).
4. Register: workspace, `xtask` `GAMES`, README, `pages.yml` comment.
5. Check: fmt, clippy (native and wasm), tests, web build, headless
   Firefox run.
