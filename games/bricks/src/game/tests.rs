//! Screen and input tests for bricks.

use super::*;
use bricks_core::{Ball, PADDLE_W, START_LIVES, ball_speed};

const FINGER: PointerId = PointerId::Touch(1);

fn game() -> Game {
    Game::new(false, 50)
}

fn key(game: &mut Game, key: Key) -> Outcome {
    game.handle(Event::Key(key), &landscape())
}

/// A game on the field, the ball still on the paddle.
fn playing() -> Game {
    let mut game = game();
    key(&mut game, Key::Launch);
    assert_eq!(game.phase, Phase::Playing);
    assert!(game.field.stuck);
    game
}

fn tap(game: &mut Game, layout: &Layout, pos: (i32, i32)) {
    game.handle(Event::Press { id: FINGER, pos }, layout);
    game.handle(Event::Release { id: FINGER, pos }, layout);
}

/// Step until the ball drops out (the paddle moved out of its way).
fn lose_ball(game: &mut Game) -> Outcome {
    game.field.paddle_x = 0.0;
    game.field.stuck = false;
    game.field.ball = Ball {
        x: 300.0,
        y: 300.0,
        vx: 0.0,
        vy: 4.0,
    };
    for _ in 0..200 {
        let outcome = game.step(Held::default());
        if game.field.stuck || game.phase == Phase::Over {
            return outcome;
        }
    }
    panic!("the ball never dropped out");
}

#[test]
fn launch_starts_then_sends_the_ball_up() {
    let mut game = playing();
    key(&mut game, Key::Launch);
    assert!(!game.field.stuck);
    assert!(game.field.ball.vy < 0.0);
}

#[test]
fn held_keys_move_the_paddle_each_step() {
    let mut game = playing();
    let x = game.field.paddle_x;
    game.step(Held {
        left: true,
        right: false,
    });
    assert_eq!(game.field.paddle_x, x - PADDLE_SPEED);
    game.step(Held {
        left: true,
        right: true,
    });
    assert_eq!(game.field.paddle_x, x - PADDLE_SPEED, "both cancel out");
}

#[test]
fn the_mouse_steers_by_hovering_and_launches_by_click() {
    let layout = landscape();
    let mut game = playing();
    game.handle(Event::Hover((layout.field.0 + 100, 200)), &layout);
    assert_eq!(game.field.paddle_x, 100.0 - PADDLE_W / 2.0);
    let click = (layout.field.0 + 100, 200);
    game.handle(
        Event::Press {
            id: PointerId::MouseLeft,
            pos: click,
        },
        &layout,
    );
    game.handle(
        Event::Move {
            id: PointerId::MouseLeft,
            pos: (click.0 + 50, click.1),
        },
        &layout,
    );
    assert_eq!(
        game.field.paddle_x,
        100.0 - PADDLE_W / 2.0,
        "a mouse drag does not move it"
    );
    game.handle(
        Event::Release {
            id: PointerId::MouseLeft,
            pos: click,
        },
        &layout,
    );
    assert!(game.field.stuck, "the press moved too far to be a click");
    tap(&mut game, &layout, click);
    assert!(!game.field.stuck);
}

#[test]
fn a_finger_drags_the_paddle_from_anywhere_and_a_tap_launches() {
    let layout = portrait();
    let mut game = playing();
    let strip = layout.drag_strip.unwrap();
    let start = center(strip);
    let x = game.field.paddle_x;
    game.handle(
        Event::Press {
            id: FINGER,
            pos: start,
        },
        &layout,
    );
    game.handle(
        Event::Move {
            id: FINGER,
            pos: (start.0 - 30, start.1),
        },
        &layout,
    );
    assert_eq!(game.field.paddle_x, x - 30.0);
    game.handle(
        Event::Release {
            id: FINGER,
            pos: (start.0 - 30, start.1),
        },
        &layout,
    );
    assert!(game.field.stuck, "a drag does not launch");
    tap(&mut game, &layout, start);
    assert!(!game.field.stuck);
}

#[test]
fn pause_by_key_or_button_freezes_the_ball() {
    let layout = portrait();
    let mut game = playing();
    key(&mut game, Key::Launch);
    tap(&mut game, &layout, center(layout.buttons[0].rect));
    assert_eq!(game.phase, Phase::Paused);
    let ball = game.field.ball;
    game.step(Held::default());
    assert_eq!(game.field.ball, ball);
    key(&mut game, Key::Pause);
    assert_eq!(game.phase, Phase::Playing);
    game.step(Held::default());
    assert_ne!(game.field.ball, ball);
}

#[test]
fn losing_a_ball_shows_a_banner_and_the_last_one_ends_the_game() {
    let mut game = playing();
    assert_eq!(lose_ball(&mut game), Outcome::Continue);
    assert_eq!(game.field.lives, START_LIVES - 1);
    assert_eq!(game.banner.map(|(text, _)| text), Some("BALL LOST"));
    game.field.score = 120;
    game.field.lives = 1;
    assert_eq!(lose_ball(&mut game), Outcome::Save);
    assert_eq!(game.phase, Phase::Over);
    assert_eq!(game.best, 120);
    assert!(game.new_best);
}

#[test]
fn a_low_score_keeps_the_best() {
    let mut game = playing();
    game.field.lives = 1;
    assert_eq!(lose_ball(&mut game), Outcome::Continue);
    assert_eq!(game.best, 50);
}

#[test]
fn the_game_over_screen_waits_before_a_new_game() {
    let layout = landscape();
    let mut game = playing();
    game.field.lives = 1;
    lose_ball(&mut game);
    tap(&mut game, &layout, (100, 100));
    assert_eq!(game.phase, Phase::Over);
    for _ in 0..RESTART_STEPS {
        game.step(Held::default());
    }
    tap(&mut game, &layout, (100, 100));
    assert_eq!(game.phase, Phase::Playing);
    assert_eq!(game.field.lives, START_LIVES);
    assert_eq!(game.field.score, 0);
}

#[test]
fn clearing_a_level_shows_a_banner_and_speeds_up() {
    let mut game = playing();
    game.field.bricks.retain(|b| b.row == 1 && b.column == 0);
    game.field.stuck = false;
    let brick = game.field.bricks[0].rect();
    game.field.ball = Ball {
        x: brick.x + 10.0,
        y: brick.y + brick.h + 1.0,
        vx: 0.0,
        vy: -3.0,
    };
    game.step(Held::default());
    assert_eq!(game.banner.map(|(text, _)| text), Some("LEVEL CLEARED"));
    assert_eq!(game.field.round, 1);
    game.handle(Event::Key(Key::Launch), &landscape());
    let speed = game.field.ball.vx.hypot(game.field.ball.vy);
    assert!((speed - ball_speed(1)).abs() < 1e-3);
}

#[test]
fn escape_quits_on_the_desktop_and_pauses_in_the_browser() {
    assert_eq!(key(&mut playing(), Key::Escape), Outcome::Quit);
    let mut web = Game::new(true, 0);
    key(&mut web, Key::Launch);
    assert_eq!(key(&mut web, Key::Escape), Outcome::Continue);
    assert_eq!(web.phase, Phase::Paused);
}

fn inside(rect: Rect, size: (i32, i32)) -> bool {
    rect.x >= 0 && rect.y >= 0 && rect.x + rect.w <= size.0 && rect.y + rect.h <= size.1
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

#[test]
fn layouts_fit_the_canvas_without_overlaps() {
    for layout in [landscape(), portrait()] {
        let field = layout.field_rect();
        let frame = Rect::new(field.x - 4, field.y - 4, field.w + 8, field.h + 8);
        let mut rects: Vec<Rect> = layout.buttons.iter().map(|b| b.rect).collect();
        rects.extend(layout.stat_boxes.iter().map(|&(_, r)| r));
        rects.extend(layout.drag_strip);
        rects.push(frame);
        for (i, a) in rects.iter().enumerate() {
            assert!(inside(*a, layout.size), "{a:?}");
            for b in &rects[i + 1..] {
                assert!(!overlaps(*a, *b), "{a:?} and {b:?}");
            }
        }
    }
}

#[test]
fn every_screen_draws_in_both_layouts() {
    for layout in [landscape(), portrait()] {
        let mut canvas = Canvas::new(layout.size.0, layout.size.1);
        let mut game = Game::new(false, 99_999);
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Launch);
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Launch);
        game.step(Held::default());
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Pause);
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Pause);
        lose_ball(&mut game);
        game.draw(&mut canvas, &layout);
        // Every level, with its tough and solid bricks.
        for round in 0..LEVELS_SHOWN {
            game.field.round = round;
            game.field.bricks = bricks_core::parse_level(bricks_core::LEVELS[round as usize]);
            game.draw(&mut canvas, &layout);
        }
        game.field.score = 123_456;
        game.field.lives = 1;
        lose_ball(&mut game);
        game.draw(&mut canvas, &layout);
        assert_eq!(game.phase, Phase::Over);
    }
}

const LEVELS_SHOWN: u32 = bricks_core::LEVELS.len() as u32;
