# Pinball: a heavier ball and a livelier table

Request: make the ball slower, like a heavier one, and make the table
more fun.

## Heavier ball (`crates/pinball-core`)

All speeds are per 1/60 s step, so the feel is in a few constants:

| Constant         | Was  | Now  | Why                                   |
|------------------|------|------|---------------------------------------|
| `GRAVITY`        | 0.2  | 0.12 | falls slower (about 0.8x the speed)   |
| `MAX_SPEED`      | 15   | 12   | no blur shots                         |
| `LAUNCH_MIN/MAX` | 10/15| 8/12 | a full pull still clears the lane top |
| `WALL_BOUNCE`    | 0.45 | 0.35 | a heavy ball bounces less             |
| `FLIPPER_BOUNCE` | 0.3  | 0.2  | it dies on a flipper, easier to catch |
| `BUMPER_KICK`    | 6    | 4.5  | kicks scaled to the slower ball       |
| `SLING_KICK`     | 4    | 3.2  |                                       |
| `KNOCK_SPEED`    | 4    | 3    | knock sounds at the new speeds        |

The launch must still reach the top-right corner from the plunger
(about 410 px up): `v^2 / (2 g)` with drag needs about 11 at `g = 0.12`,
so the top speed cannot go below 12.

## Livelier table

- **Shuttle**: a small green bumper sliding left and right between the
  lanes and the pop bumpers (y 104, x 70 to 206, 0.8 px a step); 250
  points, kicks with its own motion. Gaps to the posts and bumpers stay
  wider than the ball, so it cannot trap it.
- **Drop targets**: three stretches of the left wall (y 186 to 262). A
  hit at least 1 px/step into the wall knocks one down (200 points);
  the last one scores 2000 and raises the bank. They are part of the
  wall, so rolling along it does nothing and the table's shape never
  changes (no new resting spots or ledges).
- **Ball save**: each new ball is saved for 10 s once it leaves the
  plunger lane; a drain then puts it back on the plunger with the
  multiplier kept, a BALL SAVED banner and a sound. Saved once per ball.
  A SAVE light between the flippers shows it, blinking for the last 2 s.
- Sounds: `shuttle`, `target`, `targets`, `saved`; banners TARGETS! and
  BALL SAVED.

## Tests

- Core: a slow fall, the shuttle's range, its kick and points, target
  hit vs roll, the bank bonus, the save (once) and its run-out; the
  existing containment and no-stall sweeps cover the new pieces.
- Game: a saved ball's banner and sound, the new sounds exist, drawing
  with a lit shuttle and a dropped target.
