use super::*;

/// A field with one brick in the top-right corner (so it is not cleared)
/// and the ball at (x, y) moving (vx, vy).
fn open_field(x: f32, y: f32, vx: f32, vy: f32) -> Field {
    let mut field = Field::new();
    field.bricks = parse_level(&[".........1"]);
    field.stuck = false;
    field.ball = Ball { x, y, vx, vy };
    field
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn parse_level_reads_hits_and_solid_bricks() {
    let bricks = parse_level(&["1.#", ".3"]);
    assert_eq!(bricks.len(), 3);
    assert_eq!(
        bricks[0],
        Brick {
            column: 0,
            row: 0,
            hits: 1,
            solid: false
        }
    );
    assert!(bricks[1].solid);
    assert_eq!((bricks[2].column, bricks[2].row, bricks[2].hits), (1, 1, 3));
}

#[test]
fn every_level_fits_and_can_be_cleared() {
    for (i, level) in LEVELS.iter().enumerate() {
        assert!(level.iter().all(|row| row.len() == BRICK_COLUMNS), "{i}");
        let bricks = parse_level(level);
        assert!(bricks.iter().any(|b| !b.solid), "{i} has nothing to break");
        // No row of solid bricks shuts the ball out.
        for row in level.iter() {
            assert!(row.contains(|c| c != '#'), "{i}: {row}");
        }
        let bottom = BRICKS_TOP + level.len() as f32 * BRICK_H;
        assert!(bottom < PADDLE_Y - 100.0, "{i} is too low");
    }
}

#[test]
fn the_ball_rides_the_paddle_until_launched() {
    let mut field = Field::new();
    assert!(field.stuck);
    field.move_paddle(-500.0);
    assert_eq!(field.paddle_x, 0.0);
    assert!(approx(field.ball.x, (PADDLE_W - BALL) / 2.0));
    field.paddle_to(WIDTH);
    assert_eq!(field.paddle_x, WIDTH - PADDLE_W);
    assert_eq!(field.step(), Event::Nothing);
    assert!(approx(field.ball.y, PADDLE_Y - BALL));
    field.launch();
    assert!(!field.stuck);
    assert!(field.ball.vy < 0.0);
    assert!(approx(field.ball.vx.hypot(field.ball.vy), ball_speed(0)));
}

#[test]
fn walls_and_ceiling_bounce() {
    let mut field = open_field(1.0, 300.0, -3.0, 0.5);
    field.step();
    assert!(field.ball.vx > 0.0);
    assert!(field.ball.x >= 0.0);
    let mut field = open_field(WIDTH - BALL - 1.0, 300.0, 3.0, 0.5);
    field.step();
    assert!(field.ball.vx < 0.0);
    assert!(field.ball.x + BALL <= WIDTH);
    let mut field = open_field(200.0, 1.0, 0.5, -3.0);
    field.step();
    assert!(field.ball.vy > 0.0);
    assert!(field.ball.y >= 0.0);
}

#[test]
fn a_brick_breaks_scores_and_turns_the_ball_back() {
    let mut field = Field::new();
    field.bricks = parse_level(&["..1.......", "..1......."]);
    field.stuck = false;
    let below = BRICKS_TOP + 2.0 * BRICK_H + 1.0;
    field.ball = Ball {
        x: 2.5 * BRICK_W,
        y: below,
        vx: 0.0,
        vy: -3.0,
    };
    field.step();
    assert!(field.ball.vy > 0.0);
    assert_eq!(field.score, POINTS_PER_HIT);
    assert_eq!(field.bricks[1].hits, 0);
    assert_eq!(field.bricks[0].hits, 1);
}

#[test]
fn tough_bricks_take_several_hits_and_solid_ones_none() {
    let mut field = Field::new();
    field.bricks = parse_level(&["2#"]);
    field.stuck = false;
    let below = BRICKS_TOP + BRICK_H + 0.5;
    for (column, hits_after) in [(0, 1), (1, 0)] {
        field.ball = Ball {
            x: (column as f32 + 0.5) * BRICK_W,
            y: below,
            vx: 0.0,
            vy: -3.0,
        };
        field.step();
        assert!(field.ball.vy > 0.0);
        assert_eq!(field.bricks[column].hits, hits_after);
    }
    assert_eq!(field.score, POINTS_PER_HIT);
    assert!(field.bricks[1].standing());
}

#[test]
fn the_paddle_angles_the_ball_by_where_it_lands() {
    for (offset, goes_right) in [(-0.8, false), (0.8, true)] {
        let mut field = Field::new();
        field.stuck = false;
        let middle = field.paddle_x + PADDLE_W / 2.0;
        field.ball = Ball {
            x: middle + offset * PADDLE_W / 2.0 - BALL / 2.0,
            y: PADDLE_Y - BALL - 2.0,
            vx: 0.0,
            vy: 3.0,
        };
        field.step();
        assert!(field.ball.vy < 0.0, "{offset}");
        assert_eq!(field.ball.vx > 0.0, goes_right, "{offset}");
        assert!(approx(field.ball.vx.hypot(field.ball.vy), ball_speed(0)));
    }
}

#[test]
fn a_ball_from_the_side_is_not_caught() {
    let mut field = Field::new();
    field.stuck = false;
    // Already below the paddle's top edge.
    field.ball = Ball {
        x: field.paddle_x - BALL + 1.0,
        y: PADDLE_Y + 2.0,
        vx: 0.0,
        vy: 2.0,
    };
    field.step();
    assert!(field.ball.vy > 0.0);
}

#[test]
fn falling_out_costs_a_life_then_the_game() {
    let mut field = open_field(10.0, HEIGHT - 1.0, 0.0, 3.0);
    assert_eq!(field.step(), Event::LifeLost);
    assert_eq!(field.lives, START_LIVES - 1);
    assert!(field.stuck);
    field.lives = 1;
    field.stuck = false;
    field.ball = Ball {
        x: 10.0,
        y: HEIGHT - 1.0,
        vx: 0.0,
        vy: 3.0,
    };
    assert_eq!(field.step(), Event::GameOver);
    assert_eq!(field.lives, 0);
}

#[test]
fn clearing_the_bricks_lays_out_the_next_level_faster() {
    let mut field = Field::new();
    field.bricks = parse_level(&["1"]);
    field.stuck = false;
    field.ball = Ball {
        x: BRICK_W / 2.0,
        y: BRICKS_TOP + BRICK_H + 0.5,
        vx: 0.0,
        vy: -3.0,
    };
    assert_eq!(field.step(), Event::Cleared);
    assert_eq!(field.round, 1);
    assert_eq!(field.level(), 1);
    assert_eq!(field.bricks, parse_level(LEVELS[1]));
    assert!(field.stuck);
    assert!(ball_speed(1) > ball_speed(0));
    assert_eq!(ball_speed(100), 8.0);
}

#[test]
fn a_fast_ball_does_not_tunnel_through_a_brick() {
    let mut field = Field::new();
    field.bricks = parse_level(&["1"]);
    field.bricks.push(Brick {
        column: 9,
        row: 9,
        hits: 1,
        solid: false,
    });
    field.stuck = false;
    field.ball = Ball {
        x: BRICK_W / 2.0,
        y: BRICKS_TOP + BRICK_H + 2.0,
        vx: 0.0,
        vy: -8.0,
    };
    for _ in 0..3 {
        field.step();
    }
    assert_eq!(field.bricks[0].hits, 0);
    assert!(field.ball.vy > 0.0);
}
