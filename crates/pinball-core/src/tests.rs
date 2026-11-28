use super::*;

const NONE: Held = Held {
    left: false,
    right: false,
    plunger: false,
};
const LEFT: Held = Held {
    left: true,
    right: false,
    plunger: false,
};
const PULL: Held = Held {
    left: false,
    right: false,
    plunger: true,
};

/// A field with the ball in play at `pos`, moving at `vel`.
fn in_play(pos: (f32, f32), vel: (f32, f32)) -> Field {
    let mut field = Field::new();
    field.in_lane = false;
    field.ball = Ball {
        pos: pos.into(),
        vel: vel.into(),
    };
    field
}

/// Step with `held` until `done` or `limit` steps; the events seen.
fn run(field: &mut Field, held: Held, limit: u32, done: impl Fn(&Field) -> bool) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..limit {
        if done(field) {
            break;
        }
        events.push(field.step(held));
    }
    events
}

/// Distance of the ball's centre above the left flipper's line.
fn above_left_flipper(field: &Field) -> f32 {
    let flipper = field.flippers[0];
    let along = Vec2::new(flipper.angle.cos(), flipper.angle.sin());
    let up = Vec2::new(along.y, -along.x);
    (field.ball.pos - flipper.pivot).dot(up)
}

#[test]
fn a_new_game_waits_on_the_plunger() {
    let field = Field::new();
    assert!(field.ball_ready());
    assert_eq!(field.balls, START_BALLS);
    assert_eq!((field.score, field.multiplier), (0, 1));
}

#[test]
fn closest_point_clamps_to_the_ends() {
    let (a, b) = (Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
    assert_eq!(
        closest_point(Vec2::new(4.0, 3.0), a, b),
        Vec2::new(4.0, 0.0)
    );
    assert_eq!(closest_point(Vec2::new(-5.0, 3.0), a, b), a);
    assert_eq!(closest_point(Vec2::new(15.0, -2.0), a, b), b);
}

#[test]
fn a_full_pull_sends_the_ball_into_play() {
    let mut field = Field::new();
    run(&mut field, PULL, 60, |_| false);
    assert_eq!(field.plunger, 1.0);
    run(&mut field, NONE, 200, |f| !f.in_lane);
    assert!(!field.in_lane, "the ball never left the lane");
    assert!(field.ball.pos.x < LANE_WALL_X);
    assert!(field.ball.vel.x < 0.0, "turned into the table");
}

#[test]
fn a_tap_on_the_plunger_falls_back_to_it() {
    let mut field = Field::new();
    field.step(PULL);
    run(&mut field, NONE, 5, |_| false);
    assert!(field.ball.vel.y < 0.0, "launched");
    run(&mut field, NONE, 300, |f| f.ball_ready());
    assert!(field.ball_ready() && field.in_lane);
    assert_eq!(field.balls, START_BALLS);
}

#[test]
fn the_ball_drains_between_resting_flippers() {
    let mut field = in_play((138.0, 400.0), (0.0, 0.0));
    let events = run(&mut field, NONE, 120, |f| f.ball_ready());
    assert!(events.contains(&Event::Drained));
    assert_eq!(field.balls, START_BALLS - 1);
    assert!(field.in_lane);
}

#[test]
fn the_last_ball_ends_the_game() {
    let mut field = in_play((138.0, 400.0), (0.0, 0.0));
    field.balls = 1;
    field.multiplier = 3;
    let events = run(&mut field, NONE, 120, |f| f.balls == 0);
    assert_eq!(events.last(), Some(&Event::GameOver));
    assert_eq!(field.step(NONE), Event::GameOver); // and stays over
}

#[test]
fn a_flipper_shoots_a_resting_ball_up() {
    let flipper = Flipper::left();
    let middle = flipper.pivot + (flipper.tip() - flipper.pivot) * 0.5;
    let up = Vec2::new(FLIPPER_REST.sin(), -FLIPPER_REST.cos());
    let start = middle + up * (BALL_RADIUS + FLIPPER_RADIUS + 0.5);
    let mut field = in_play((start.x, start.y), (0.0, 0.0));
    run(&mut field, LEFT, 4, |_| false);
    assert!(field.ball.vel.y < -6.0, "{:?}", field.ball.vel);
}

#[test]
fn a_ball_rolls_off_an_idle_flipper_into_the_drain() {
    let mut field = in_play((95.0, 420.0), (0.0, 0.0));
    let events = run(&mut field, NONE, 400, |f| f.balls < START_BALLS);
    assert!(events.contains(&Event::Drained));
}

#[test]
fn a_falling_ball_does_not_pass_through_a_flipper() {
    let mut field = in_play((100.0, 380.0), (0.0, MAX_SPEED));
    for _ in 0..6 {
        field.step(NONE);
        assert!(above_left_flipper(&field) > 0.0, "{:?}", field.ball);
    }
}

#[test]
fn bumpers_kick_the_ball_away_and_score() {
    let (x, y) = BUMPERS[0];
    let mut field = in_play((x, y - 30.0), (0.0, 3.0));
    run(&mut field, NONE, 20, |f| f.score > 0);
    assert_eq!(field.score, BUMPER_POINTS);
    assert!(field.bumper_flash[0] > 0);
    assert!(
        field.ball.vel.y < -BUMPER_KICK * 0.8,
        "{:?}",
        field.ball.vel
    );
}

#[test]
fn slingshots_kick_and_score() {
    let mut field = in_play((60.0, 330.0), (-4.0, 0.0));
    run(&mut field, NONE, 30, |f| f.score > 0);
    assert_eq!(field.score, SLING_POINTS);
    assert!(field.sling_flash[0] > 0);
    assert!(field.ball.vel.x > 0.0, "kicked back to the right");
}

#[test]
fn rolling_through_a_lane_lights_it_once() {
    let mut field = in_play((LANES[0], 20.0), (0.0, 2.0));
    run(&mut field, NONE, 40, |f| f.ball.pos.y > LANE_BOTTOM + 5.0);
    assert_eq!(field.lanes, [true, false, false]);
    assert_eq!(field.score, LANE_POINTS);
}

#[test]
fn lighting_every_lane_raises_the_multiplier() {
    let mut field = in_play((LANES[2], 20.0), (0.0, 2.0));
    field.lanes = [true, true, false];
    let events = run(&mut field, NONE, 40, |f| f.multiplier > 1);
    assert!(events.contains(&Event::Bonus));
    assert_eq!(field.lanes, [false; 3]);
    assert_eq!(field.multiplier, 2);
    assert_eq!(field.score, LANE_POINTS + 2 * LANES_BONUS);
}

#[test]
fn the_multiplier_is_capped_and_resets_on_a_drain() {
    let mut field = in_play((LANES[0], 20.0), (0.0, 2.0));
    field.lanes = [false, true, true];
    field.multiplier = MAX_MULTIPLIER;
    run(&mut field, NONE, 40, |f| f.ball.pos.y > LANE_BOTTOM + 5.0);
    assert_eq!(field.multiplier, MAX_MULTIPLIER);
    field.ball = Ball {
        pos: Vec2::new(138.0, 470.0),
        vel: Vec2::new(0.0, 3.0),
    };
    run(&mut field, NONE, 30, |f| f.ball_ready());
    assert_eq!(field.multiplier, 1);
}

#[test]
fn flipper_presses_rotate_the_lit_lanes() {
    let mut field = Field::new();
    field.lanes = [true, false, false];
    field.rotate_lanes(1);
    assert_eq!(field.lanes, [false, true, false]);
    field.rotate_lanes(-1);
    field.rotate_lanes(-1);
    assert_eq!(field.lanes, [false, false, true]);
}

/// Throws from points inside the table: the upper field, between the
/// bumpers, and the lower field between the slingshots.
fn throws() -> impl Iterator<Item = ((f32, f32), (f32, f32))> {
    let upper = [30.0, 70.0, 120.0, 160.0, 230.0, 260.0]
        .into_iter()
        .flat_map(|x| [100.0, 250.0, 300.0].map(|y| (x, y)));
    let lower = [
        (70.0, 360.0),
        (120.0, 380.0),
        (160.0, 380.0),
        (206.0, 360.0),
    ];
    let speeds = [
        (12.0, -10.0),
        (-14.0, 5.0),
        (3.0, -14.0),
        (-9.0, -9.0),
        (0.0, 0.0),
    ];
    upper.chain(lower).flat_map(move |s| speeds.map(|t| (s, t)))
}

#[test]
fn an_untouched_ball_always_drains() {
    // No resting spot anywhere: without the flippers every ball ends in
    // the drain (a ball that stopped would hang the game).
    for (start, throw) in throws() {
        let mut field = in_play(start, throw);
        let events = run(&mut field, NONE, 3600, |f| f.balls < START_BALLS);
        assert!(
            events.contains(&Event::Drained),
            "{start:?} {throw:?} stuck at {:?}",
            field.ball
        );
    }
}

#[test]
fn the_ball_stays_on_the_table_whatever_happens() {
    for (start, throw) in throws() {
        let mut field = in_play(start, throw);
        for step in 0..900u32 {
            // Flap both flippers to keep the ball moving.
            let flap = step % 50 < 8;
            let held = Held {
                left: flap,
                right: step % 70 < 8,
                plunger: false,
            };
            if field.step(held) != Event::Nothing {
                break;
            }
            let Ball { pos, vel } = field.ball;
            let case = format!("{start:?} {throw:?} step {step}: {pos:?}");
            assert!(pos.is_finite(), "{case}");
            assert!(pos.x > 0.0 && pos.x < LANE_WALL_X, "{case}");
            assert!(pos.y > 0.0, "{case}");
            assert!(vel.length() <= MAX_SPEED + 1e-3, "{case}");
            assert!(!field.in_lane, "{case}: back in the plunger lane");
        }
    }
}

#[test]
fn a_flipper_sounds_once_per_press() {
    let mut field = Field::new();
    for _ in 0..5 {
        field.step(LEFT);
    }
    field.step(NONE);
    field.step(LEFT);
    let flips = field.take_sounds();
    assert_eq!(flips, [Sound::Flipper, Sound::Flipper], "not while held");
    assert!(field.take_sounds().is_empty(), "taken");
}

#[test]
fn launching_hitting_and_draining_make_their_sounds() {
    let mut field = Field::new();
    run(&mut field, PULL, 10, |_| false);
    field.step(NONE);
    assert!(field.take_sounds().contains(&Sound::Launch));

    let (x, y) = BUMPERS[0];
    let mut field = in_play((x, y - 30.0), (0.0, 3.0));
    run(&mut field, NONE, 20, |f| f.score > 0);
    assert!(field.take_sounds().contains(&Sound::Bumper));

    let mut field = in_play((60.0, 330.0), (-4.0, 0.0));
    run(&mut field, NONE, 30, |f| f.score > 0);
    assert!(field.take_sounds().contains(&Sound::Sling));

    let mut field = in_play((LANES[2], 20.0), (0.0, 2.0));
    field.lanes = [true, true, false];
    run(&mut field, NONE, 40, |f| f.multiplier > 1);
    let sounds = field.take_sounds();
    let lane = sounds.iter().position(|&s| s == Sound::Lane);
    let bonus = sounds.iter().position(|&s| s == Sound::Bonus);
    assert!(lane.is_some() && lane < bonus, "{sounds:?}");

    let mut field = in_play((138.0, 470.0), (0.0, 3.0));
    run(&mut field, NONE, 30, |f| f.ball_ready());
    assert!(field.take_sounds().contains(&Sound::Drain));
}

#[test]
fn only_hard_wall_hits_knock() {
    let mut field = in_play((20.0, 200.0), (-12.0, 0.0));
    run(&mut field, NONE, 4, |_| false);
    assert_eq!(field.take_sounds(), [Sound::Knock]);

    // Rolling along a wall, or resting on the plunger, is quiet.
    let mut field = in_play((8.0, 200.0), (-1.0, 0.0));
    run(&mut field, NONE, 30, |_| false);
    assert!(!field.take_sounds().contains(&Sound::Knock));
    let mut field = Field::new();
    run(&mut field, NONE, 30, |_| false);
    assert!(field.take_sounds().is_empty());
}
