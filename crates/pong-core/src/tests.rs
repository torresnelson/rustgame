//! Ports of `tests/test_game.c` from torresnelson/pong.

use super::computer::*;
use super::*;

fn rng() -> fastrand::Rng {
    fastrand::Rng::with_seed(7)
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn clamp_speed_caps_each_component() {
    let capped = clamp_speed(Vec2::new(-40.0, 20.0), MAX_SPEED);
    assert_eq!(capped, Vec2::new(-MAX_SPEED.x, MAX_SPEED.y));
    let same = clamp_speed(Vec2::new(3.0, -2.0), MAX_SPEED);
    assert_eq!(same, Vec2::new(3.0, -2.0));
}

#[test]
fn score_manager_awards_matches_and_ends_the_game() {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    assert_eq!(score_manager(&mut p1, &mut p2, 10, 3), State::Playing);
    for round in 1..=3 {
        p1.score = 10;
        p2.score = 4;
        let state = score_manager(&mut p1, &mut p2, 10, 3);
        // The deciding match must end the game immediately.
        let expected = if round < 3 {
            State::MatchOver
        } else {
            State::GameOver
        };
        assert_eq!(state, expected);
        assert_eq!(p1.match_score, round);
        assert_eq!((p1.score, p2.score), (0, 0));
    }
    reset_game(&mut p1, &mut p2);
    assert_eq!((p1.match_score, p2.match_score), (0, 0));
    p2.score = 10;
    assert_eq!(score_manager(&mut p1, &mut p2, 10, 1), State::GameOver);
    assert_eq!(p2.match_score, 1);
}

/// Fire the ball at a paddle and count how many times it changes direction.
fn paddle_flips(speed_x: f32, start_y: f32, right_side: bool) -> u32 {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    let (x, sx) = if right_side {
        (SCREEN_WIDTH - 100.0, speed_x)
    } else {
        (100.0, -speed_x)
    };
    let mut ball = Ball::new(Vec2::new(x, start_y), Vec2::new(sx, 0.05));
    let mut rng = rng();
    let mut flips = 0;
    let mut last = ball.speed.x;
    for _ in 0..(300.0 / speed_x) as i32 {
        collisions(&mut p1, &mut p2, &mut ball, &mut rng);
        ball_movement(&mut ball);
        if (ball.speed.x > 0.0) != (last > 0.0) {
            flips += 1;
        }
        last = ball.speed.x;
    }
    flips
}

#[test]
fn a_paddle_bounces_the_ball_once() {
    let mut speed_x = 1.0;
    while speed_x <= MAX_SPEED.x {
        let mut y = SCREEN_HEIGHT / 2.0 + 5.0;
        while y < SCREEN_HEIGHT / 2.0 + PLAYER_HEIGHT - 5.0 {
            assert_eq!(paddle_flips(speed_x, y, false), 1, "left {speed_x} {y}");
            assert_eq!(paddle_flips(speed_x, y, true), 1, "right {speed_x} {y}");
            y += 10.0;
        }
        speed_x += 0.5;
    }
}

#[test]
fn a_ball_behind_the_paddle_is_not_saved() {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    let mut ball = Ball::new(
        Vec2::new(3.0, SCREEN_HEIGHT / 2.0 + 50.0),
        Vec2::new(-5.0, 0.0),
    );
    collisions(&mut p1, &mut p2, &mut ball, &mut rng());
    assert_eq!(ball.speed.x, -5.0);
}

#[test]
fn a_floor_bounce_stays_in_the_field() {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    let mut ball = Ball::new(Vec2::new(400.0, SCREEN_HEIGHT - 2.0), Vec2::new(3.0, 0.3));
    collisions(&mut p1, &mut p2, &mut ball, &mut rng());
    assert_eq!(ball.position.y, SCREEN_HEIGHT - RADIUS);
    assert!(ball.speed.y < 0.0);
}

#[test]
fn predict_ball_y_folds_in_bounces() {
    let at = |x, y| Vec2::new(x, y);
    // A straight shot lands where it is.
    assert_eq!(
        predict_ball_y(at(100.0, 200.0), at(5.0, 0.0), 700.0, RADIUS),
        200.0
    );
    // A ball moving away keeps its current height.
    assert_eq!(
        predict_ball_y(at(100.0, 200.0), at(-5.0, 3.0), 700.0, RADIUS),
        200.0
    );
    // Bounces once off the floor: 200 + 300 = 500 is 60 past the floor (440).
    assert!(approx(
        predict_ball_y(at(400.0, 200.0), at(4.0, 4.0), 700.0, RADIUS),
        380.0
    ));
    // Bounces once off the roof.
    assert!(approx(
        predict_ball_y(at(400.0, 100.0), at(4.0, -4.0), 700.0, RADIUS),
        220.0
    ));
}

#[test]
fn step_toward_moves_at_most_max_step() {
    assert_eq!(step_toward(100.0, 105.0, 10.0), 105.0);
    assert_eq!(step_toward(100.0, 300.0, 10.0), 110.0);
    assert_eq!(step_toward(100.0, 0.0, 10.0), 90.0);
}

#[test]
fn computer_level_grows_with_points_and_matches() {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    assert_eq!(computer_level(&p1, &p2), 1);
    p1.score = 2;
    p2.score = 2;
    assert_eq!(computer_level(&p1, &p2), 2);
    p1.match_score = 1;
    assert_eq!(computer_level(&p1, &p2), 4);
    p2.match_score = 10;
    assert_eq!(computer_level(&p1, &p2), MAX_LEVEL);
}

#[test]
fn the_computer_gets_harder_each_level() {
    for level in 2..=MAX_LEVEL {
        assert!(computer_speed(level) > computer_speed(level - 1));
        assert!(computer_reaction_x(level) < computer_reaction_x(level - 1));
        assert!(computer_aim_error(level) < computer_aim_error(level - 1));
    }
    // Never faster than a human paddle, and perfect aim only at the top.
    assert!(computer_speed(MAX_LEVEL) < PADDLE_SPEED);
    assert_eq!(computer_aim_error(MAX_LEVEL), 0);
}

/// Serve towards the computer and report whether it sends the ball back.
fn computer_returns(level: u32, speed: Vec2, start_y: f32, rng: &mut fastrand::Rng) -> bool {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    p1.score = 4 * (level - 1);
    let mut computer = Computer::default();
    let mut ball = Ball::new(Vec2::new(SCREEN_WIDTH / 2.0, start_y), speed);
    let mut returned = false;
    while !returned && ball.position.x < SCREEN_WIDTH + 2.0 * RADIUS {
        computer_update(&mut computer, &mut p2, &p1, &ball, rng);
        collisions(&mut p1, &mut p2, &mut ball, rng);
        ball_movement(&mut ball);
        returned = ball.speed.x < 0.0;
    }
    assert_eq!(computer.level, level);
    returned
}

#[test]
fn the_top_level_returns_every_serve() {
    let mut rng = rng();
    let mut sx = 4.0;
    while sx <= MAX_SPEED.x {
        let mut sy = -MAX_SPEED.y;
        while sy <= MAX_SPEED.y {
            let mut y = 50.0;
            while y < SCREEN_HEIGHT {
                assert!(
                    computer_returns(MAX_LEVEL, Vec2::new(sx, sy), y, &mut rng),
                    "speed ({sx}, {sy}) from y {y}"
                );
                y += 100.0;
            }
            sy += 2.0;
        }
        sx += 1.5;
    }
}

#[test]
fn the_first_level_misses_sometimes() {
    let mut rng = rng();
    let mut misses = 0;
    let mut sy = -MAX_SPEED.y;
    while sy <= MAX_SPEED.y {
        if !computer_returns(1, Vec2::new(MAX_SPEED.x, sy), 50.0, &mut rng) {
            misses += 1;
        }
        sy += 2.0;
    }
    assert!(misses > 0);
}

#[test]
fn menu_select_wraps() {
    use MenuOption::*;
    assert_eq!(menu_select(OnePlayer, 1), TwoPlayers);
    assert_eq!(menu_select(TwoPlayers, 1), OnePlayer);
    assert_eq!(menu_select(OnePlayer, -1), TwoPlayers);
    assert_eq!(menu_select(TwoPlayers, -1), OnePlayer);
    assert_eq!(menu_select(OnePlayer, 0), OnePlayer);
}

#[test]
fn touch_controls_split_the_left_third_and_pause_on_top() {
    let at = |x, y| touch_control_at(Vec2::new(x, y));
    assert_eq!(at(50.0, 50.0), Some(TouchControl::Up));
    assert_eq!(at(50.0, SCREEN_HEIGHT - 10.0), Some(TouchControl::Down));
    let edge = SCREEN_WIDTH / 3.0 - 1.0;
    assert_eq!(at(edge, SCREEN_HEIGHT / 2.0 - 1.0), Some(TouchControl::Up));
    assert_eq!(
        at(edge, SCREEN_HEIGHT / 2.0 + 1.0),
        Some(TouchControl::Down)
    );
    assert_eq!(at(SCREEN_WIDTH / 2.0, 20.0), Some(TouchControl::Pause));
    // The rest of the field, where the ball and the computer play, is not a control.
    assert_eq!(at(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0), None);
    assert_eq!(at(SCREEN_WIDTH - 20.0, 20.0), None);
    assert_eq!(at(-1.0, -1.0), None);
    // Controls must not overlap.
    assert!(
        !touch_control_rect(TouchControl::Pause).overlaps(&touch_control_rect(TouchControl::Up))
    );
}

// Beyond the C suite: the pieces `referee` ties together.

#[test]
fn eye_ball_scores_and_serves_towards_the_scorer_s_opponent() {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    let mut ball = Ball::new(Vec2::new(SCREEN_WIDTH + 25.0, 100.0), Vec2::new(9.0, 1.0));
    eye_ball(&mut p1, &mut p2, &mut ball);
    assert_eq!((p1.score, p2.score), (1, 0));
    assert_eq!(ball.speed, Vec2::new(-SERVE_SPEED.x, -SERVE_SPEED.y));
    ball.position.x = -25.0;
    eye_ball(&mut p1, &mut p2, &mut ball);
    assert_eq!((p1.score, p2.score), (1, 1));
    assert_eq!(ball.speed, SERVE_SPEED);
    // Inside the field nothing happens.
    eye_ball(&mut p1, &mut p2, &mut ball);
    assert_eq!((p1.score, p2.score), (1, 1));
}

#[test]
fn collisions_keep_the_paddles_in_the_field() {
    let (mut p1, mut p2) = (Player::left(), Player::right());
    p1.bar.y = -40.0;
    p2.bar.y = SCREEN_HEIGHT;
    let mut ball = Ball::serve();
    collisions(&mut p1, &mut p2, &mut ball, &mut rng());
    assert_eq!(p1.bar.y, FIELD_TOP);
    assert_eq!(p2.bar.y, FIELD_BOTTOM);
}

#[test]
fn circle_touches_a_rect_by_its_closest_point() {
    let rect = FRect::new(0.0, 0.0, 10.0, 10.0);
    assert!(rect.touches_circle(Vec2::new(15.0, 5.0), 5.0));
    assert!(!rect.touches_circle(Vec2::new(15.1, 5.0), 5.0));
    // Diagonal from a corner: distance sqrt(18) ~ 4.24.
    assert!(rect.touches_circle(Vec2::new(13.0, 13.0), 4.3));
    assert!(!rect.touches_circle(Vec2::new(13.0, 13.0), 4.2));
}
