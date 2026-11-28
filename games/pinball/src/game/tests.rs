//! Screen and input tests for pinball.

use super::*;
use pinball_core::Ball;

const FINGER: PointerId = PointerId::Touch(1);
const NOTHING: Held = Held {
    left: false,
    right: false,
    plunger: false,
};

fn game() -> Game {
    Game::new(false, 500)
}

fn key(game: &mut Game, key: Key) -> Outcome {
    game.handle(Event::Key(key), &landscape())
}

/// A game on the table, the ball on the plunger.
fn playing() -> Game {
    let mut game = game();
    key(&mut game, Key::Start);
    assert_eq!(game.phase, Phase::Playing);
    game
}

fn press(game: &mut Game, layout: &Layout, id: PointerId, pos: (i32, i32)) {
    game.handle(Event::Press { id, pos }, layout);
}

fn button(layout: &Layout, action: &str) -> (i32, i32) {
    let button = layout.buttons.iter().find(|b| b.action == action).unwrap();
    center(button.rect)
}

/// Drop the ball into the drain and step until it is gone.
fn drain(game: &mut Game) -> Outcome {
    game.field.in_lane = false;
    game.field.ball = Ball {
        pos: Vec2::new(138.0, 470.0),
        vel: Vec2::new(0.0, 4.0),
    };
    for _ in 0..30 {
        let outcome = game.step(NOTHING);
        if game.field.ball_ready() || game.phase == Phase::Over {
            return outcome;
        }
    }
    panic!("the ball never drained");
}

#[test]
fn any_start_key_or_tap_begins_a_game() {
    let mut game = game();
    key(&mut game, Key::Plunger);
    assert_eq!(game.phase, Phase::Playing);
    let mut game = Game::new(true, 0);
    let layout = portrait();
    press(&mut game, &layout, FINGER, (150, 300));
    assert_eq!(game.phase, Phase::Playing);
    // The tap that starts does not hold a flipper.
    assert_eq!(game.held(NOTHING), NOTHING);
}

#[test]
fn flipper_keys_shift_the_lit_lanes() {
    let mut game = playing();
    game.field.lanes = [true, false, false];
    key(&mut game, Key::Right);
    assert_eq!(game.field.lanes, [false, true, false]);
    key(&mut game, Key::Left);
    assert_eq!(game.field.lanes, [true, false, false]);
}

#[test]
fn held_keys_raise_the_flippers() {
    let mut game = playing();
    let rest = game.field.flippers[0].angle;
    for _ in 0..4 {
        game.step(Held {
            left: true,
            ..NOTHING
        });
    }
    assert!(game.field.flippers[0].angle < rest);
    assert_eq!(game.field.flippers[1].angle, game.field.flippers[1].rest);
}

#[test]
fn each_half_of_the_screen_holds_its_flipper() {
    for layout in [landscape(), portrait()] {
        let mut game = playing();
        let (w, h) = layout.size;
        press(&mut game, &layout, FINGER, (w - 10, h / 2));
        assert!(game.held(NOTHING).right && !game.held(NOTHING).left);
        press(&mut game, &layout, PointerId::Touch(2), (10, h / 2));
        assert!(game.held(NOTHING).left && game.held(NOTHING).right);
        game.handle(Event::Release { id: FINGER }, &layout);
        assert!(game.held(NOTHING).left && !game.held(NOTHING).right);
    }
}

#[test]
fn mouse_buttons_are_the_flippers_wherever_they_click() {
    let layout = landscape();
    let mut game = playing();
    press(&mut game, &layout, PointerId::MouseRight, (30, 200));
    assert!(game.held(NOTHING).right);
    press(&mut game, &layout, PointerId::MouseLeft, (600, 200));
    assert!(game.held(NOTHING).left);
}

#[test]
fn holding_launch_pulls_the_plunger_and_letting_go_shoots() {
    for layout in [landscape(), portrait()] {
        let mut game = playing();
        press(&mut game, &layout, FINGER, button(&layout, "launch"));
        for _ in 0..60 {
            game.step(NOTHING);
        }
        assert_eq!(game.field.plunger, 1.0);
        game.handle(Event::Release { id: FINGER }, &layout);
        for _ in 0..200 {
            game.step(NOTHING);
            if !game.field.in_lane {
                break;
            }
        }
        assert!(!game.field.in_lane, "the ball never entered the table");
    }
}

#[test]
fn pause_by_key_or_button_freezes_the_ball() {
    let layout = portrait();
    let mut game = playing();
    game.field.in_lane = false;
    game.field.ball.pos = Vec2::new(138.0, 100.0);
    key(&mut game, Key::Pause);
    let ball = game.field.ball;
    game.step(NOTHING);
    assert_eq!(game.field.ball, ball);
    press(&mut game, &layout, FINGER, button(&layout, "pause"));
    assert_eq!(game.phase, Phase::Playing);
    game.step(NOTHING);
    assert_ne!(game.field.ball, ball);
}

#[test]
fn escape_quits_on_the_desktop_and_pauses_in_the_browser() {
    assert_eq!(key(&mut playing(), Key::Escape), Outcome::Quit);
    let mut game = Game::new(true, 0);
    key(&mut game, Key::Start);
    assert_eq!(key(&mut game, Key::Escape), Outcome::Continue);
    assert_eq!(game.phase, Phase::Paused);
}

#[test]
fn a_drain_shows_the_next_ball_and_the_last_one_ends_the_game() {
    let mut game = playing();
    drain(&mut game);
    assert_eq!(game.ball_number(), 2);
    assert_eq!(
        game.banner.as_ref().map(|(text, _)| text.as_str()),
        Some("BALL 2")
    );
    drain(&mut game);
    game.field.score = 900;
    assert_eq!(drain(&mut game), Outcome::Save);
    assert_eq!(game.phase, Phase::Over);
    assert_eq!(game.best, 900);
    assert!(game.new_best);
}

#[test]
fn a_low_score_keeps_the_best() {
    let mut game = playing();
    game.field.balls = 1;
    assert_eq!(drain(&mut game), Outcome::Continue);
    assert_eq!((game.best, game.new_best), (500, false));
}

#[test]
fn the_game_over_screen_waits_before_a_new_game() {
    let mut game = playing();
    game.field.balls = 1;
    game.field.score = 70;
    drain(&mut game);
    key(&mut game, Key::Start);
    assert_eq!(game.phase, Phase::Over, "too soon");
    for _ in 0..RESTART_STEPS {
        game.step(NOTHING);
    }
    key(&mut game, Key::Start);
    assert_eq!(game.phase, Phase::Playing);
    assert_eq!((game.field.score, game.field.balls), (0, START_BALLS));
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
        let table = layout.table_rect();
        let frame = Rect::new(table.x - 4, table.y - 4, table.w + 8, table.h + 8);
        let mut rects: Vec<Rect> = layout.buttons.iter().map(|b| b.rect).collect();
        rects.extend(layout.stat_boxes.iter().map(|&(_, r)| r));
        rects.push(frame);
        for (i, a) in rects.iter().enumerate() {
            assert!(inside(*a, layout.size), "{a:?}");
            for b in &rects[i + 1..] {
                assert!(!overlaps(*a, *b), "{a:?} and {b:?}");
            }
        }
        let actions: Vec<&str> = layout.buttons.iter().map(|b| b.action).collect();
        assert!(actions.contains(&"launch") && actions.contains(&"pause"));
    }
}

#[test]
fn every_screen_draws_in_both_layouts() {
    for layout in [landscape(), portrait()] {
        let mut canvas = Canvas::new(layout.size.0, layout.size.1);
        let mut game = Game::new(false, 99_999);
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Start);
        game.draw(&mut canvas, &layout);
        // A ball in play, a lit lane, a flashing bumper and slingshot.
        game.field.in_lane = false;
        game.field.ball.pos = Vec2::new(150.0, 300.0);
        game.field.lanes = [true, false, true];
        game.field.bumper_flash = [3, 0, 0];
        game.field.sling_flash = [0, 3];
        game.field.multiplier = 5;
        game.draw(&mut canvas, &layout);
        let (x, y) = layout.to_canvas(game.field.ball.pos);
        assert_eq!(
            canvas.get(x as i32 - 2, y as i32 - 2),
            Some(WHITE),
            "ball glint"
        );
        key(&mut game, Key::Pause);
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Pause);
        game.field.score = 123_456;
        game.field.balls = 1;
        drain(&mut game);
        game.draw(&mut canvas, &layout);
        assert_eq!(game.phase, Phase::Over);
    }
}

#[test]
fn every_sound_the_game_plays_exists_and_is_audible() {
    let sounds = crate::sounds::sounds();
    let names: Vec<&str> = sounds.iter().map(|(name, _)| *name).collect();
    let table = [
        Sound::Flipper,
        Sound::Launch,
        Sound::Bumper,
        Sound::Sling,
        Sound::Lane,
        Sound::Bonus,
        Sound::Knock,
        Sound::Drain,
    ];
    let played = table.map(sound_name);
    for name in played.iter().chain(&["start", "over"]) {
        assert!(names.contains(name), "no sound called {name}");
    }
    for (i, (name, samples)) in sounds.iter().enumerate() {
        assert!(!names[i + 1..].contains(name), "{name} twice");
        assert!(samples.iter().all(|s| s.abs() <= 1.0), "{name} clips");
        assert!(samples.iter().any(|s| s.abs() > 0.1), "{name} is silent");
        assert!(
            samples.len() < sfx::SAMPLE_RATE as usize,
            "{name} is too long"
        );
    }
}

#[test]
fn starting_flipping_and_losing_make_their_sounds() {
    let mut game = playing();
    assert_eq!(game.take_sounds(), ["start"]);
    for _ in 0..3 {
        game.step(Held {
            left: true,
            ..NOTHING
        });
    }
    assert_eq!(game.take_sounds(), ["flipper"], "once, not while held");

    game.field.balls = 1;
    drain(&mut game);
    let sounds = game.take_sounds();
    assert!(
        sounds.contains(&"over") && !sounds.contains(&"drain"),
        "{sounds:?}"
    );
}

#[test]
fn the_sound_key_turns_sound_off_and_on() {
    let layout = landscape();
    let mut game = playing();
    game.take_sounds();
    key(&mut game, Key::Sound);
    assert!(!game.sound_on);
    let mut quiet = Canvas::new(layout.size.0, layout.size.1);
    game.draw(&mut quiet, &layout);
    key(&mut game, Key::Sound);
    assert!(game.sound_on);
    assert_eq!(game.take_sounds(), ["lane"], "a chime says it is back");
    let mut loud = Canvas::new(layout.size.0, layout.size.1);
    game.draw(&mut loud, &layout);
    assert_ne!(quiet, loud, "SOUND OFF is shown while off");
}
