//! Screen and input tests for snake.

use super::*;

const FINGER: PointerId = PointerId::Touch(1);

fn game() -> Game {
    Game::new(false, 4, 3)
}

fn key(game: &mut Game, key: Key) -> Outcome {
    game.handle(Event::Key(key), &landscape())
}

fn button(layout: &Layout, action: &str) -> (i32, i32) {
    let rect = layout
        .buttons
        .iter()
        .find(|b| b.action == action)
        .unwrap()
        .rect;
    (rect.x + rect.w / 2, rect.y + rect.h / 2)
}

fn press(game: &mut Game, layout: &Layout, pos: (i32, i32)) -> Outcome {
    game.handle(Event::Press { id: FINGER, pos }, layout)
}

/// A game that is playing, with the clock started at 0.
fn playing() -> Game {
    let mut game = game();
    game.update(0);
    key(&mut game, Key::Enter);
    assert_eq!(game.phase, Phase::Playing);
    game
}

#[test]
fn enter_or_a_tap_starts() {
    let layout = landscape();
    let mut game = game();
    press(&mut game, &layout, (100, 100));
    assert_eq!(game.phase, Phase::Playing);
    assert_eq!(playing().phase, Phase::Playing);
}

#[test]
fn a_turn_on_the_ready_screen_starts_that_way() {
    let mut game = game();
    key(&mut game, Key::Up);
    assert_eq!(game.phase, Phase::Playing);
    assert_eq!(game.snake.next_heading(), Direction::Up);
}

#[test]
fn the_snake_steps_with_the_clock() {
    let mut game = playing();
    let head = game.snake.head();
    // 16 ms frames: the first step is due after the 10th.
    for frame in 1..10 {
        game.update(frame * 16);
    }
    assert_eq!(game.snake.head(), head);
    game.update(160);
    assert_eq!(game.snake.head(), (head.0 + 1, head.1));
}

#[test]
fn a_long_stall_catches_up_only_a_little() {
    let mut game = playing();
    let head = game.snake.head();
    game.update(60_000);
    assert_eq!(game.snake.head(), head, "a 100 ms cap is less than a step");
}

#[test]
fn swipes_turn_and_rearm_while_held() {
    let layout = landscape();
    let mut game = playing();
    press(&mut game, &layout, (100, 100));
    game.handle(
        Event::Move {
            id: FINGER,
            pos: (100, 70),
        },
        &layout,
    );
    assert_eq!(game.snake.next_heading(), Direction::Up);
    game.handle(
        Event::Move {
            id: FINGER,
            pos: (70, 70),
        },
        &layout,
    );
    assert_eq!(game.snake.next_heading(), Direction::Left);
    game.handle(Event::Release { id: FINGER }, &layout);
    game.handle(
        Event::Move {
            id: FINGER,
            pos: (70, 200),
        },
        &layout,
    );
    assert_eq!(game.snake.next_heading(), Direction::Left);
}

#[test]
fn thumb_buttons_turn_in_portrait() {
    let layout = portrait();
    let mut game = playing();
    press(&mut game, &layout, button(&layout, "down"));
    assert_eq!(game.snake.next_heading(), Direction::Down);
    press(&mut game, &layout, button(&layout, "left"));
    assert_eq!(game.snake.next_heading(), Direction::Left);
}

#[test]
fn pause_stops_the_clock_and_a_tap_goes_on() {
    let layout = landscape();
    let mut game = playing();
    key(&mut game, Key::Space);
    assert_eq!(game.phase, Phase::Paused);
    let head = game.snake.head();
    game.update(50);
    game.update(100);
    assert_eq!(game.snake.head(), head);
    press(&mut game, &layout, button(&layout, "pause"));
    assert_eq!(game.phase, Phase::Playing);
    key(&mut game, Key::Pause);
    press(&mut game, &layout, (100, 100));
    assert_eq!(game.phase, Phase::Playing);
}

fn crash(game: &mut Game) -> Outcome {
    let mut now = 0;
    loop {
        now += 50;
        let outcome = game.update(now);
        if game.phase == Phase::Over {
            return outcome;
        }
        assert!(now < 10_000, "never crashed");
    }
}

#[test]
fn crashing_ends_and_saves_a_new_best() {
    let mut game = playing();
    game.snake.score = 5;
    game.snake.food = None;
    assert_eq!(crash(&mut game), Outcome::Save);
    assert_eq!(game.best, 5);
    assert!(game.new_best);

    let mut game = playing();
    game.snake.food = None;
    assert_eq!(crash(&mut game), Outcome::Continue);
    assert_eq!(game.best, 3);
    assert!(!game.new_best);
}

#[test]
fn the_game_over_screen_waits_before_a_restart() {
    let mut game = playing();
    game.snake.food = None;
    crash(&mut game);
    let ended = game.over_ms;
    key(&mut game, Key::Enter);
    assert_eq!(game.phase, Phase::Over);
    game.update(ended + RESTART_DELAY_MS);
    key(&mut game, Key::Enter);
    assert_eq!(game.phase, Phase::Playing);
    assert_eq!(game.snake.score, 0);
    assert_eq!(game.snake.status, Status::Running);
}

#[test]
fn escape_ends_a_game_then_quits_on_the_desktop_only() {
    let mut game = playing();
    assert_eq!(key(&mut game, Key::Escape), Outcome::Continue);
    assert_eq!(game.phase, Phase::Over);
    assert_eq!(key(&mut game, Key::Escape), Outcome::Quit);
    let mut web = Game::new(true, 4, 0);
    assert_eq!(key(&mut web, Key::Escape), Outcome::Continue);
}

#[test]
fn the_end_button_ends_the_game() {
    let layout = landscape();
    let mut game = playing();
    press(&mut game, &layout, button(&layout, "end"));
    assert_eq!(game.phase, Phase::Over);
}

#[test]
fn the_right_mouse_button_does_nothing() {
    let mut game = game();
    game.handle(
        Event::Press {
            id: PointerId::MouseRight,
            pos: (100, 100),
        },
        &landscape(),
    );
    assert_eq!(game.phase, Phase::Ready);
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
        let board = layout.board_rect();
        let frame = Rect::new(board.x - 4, board.y - 4, board.w + 8, board.h + 8);
        assert!(inside(frame, layout.size));
        let mut rects: Vec<Rect> = layout.buttons.iter().map(|b| b.rect).collect();
        rects.extend(layout.stat_boxes);
        for (i, a) in rects.iter().enumerate() {
            assert!(inside(*a, layout.size), "{a:?}");
            assert!(!overlaps(*a, frame), "{a:?} covers the board");
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
        let mut game = game();
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Enter);
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Pause);
        game.draw(&mut canvas, &layout);
        key(&mut game, Key::Pause);
        game.snake.score = 99;
        game.snake.food = None;
        crash(&mut game);
        game.draw(&mut canvas, &layout);
        game.snake.status = Status::Won;
        game.draw(&mut canvas, &layout);
        // The food is drawn red on the board.
        let game = Game::new(false, 4, 0);
        game.draw(&mut canvas, &layout);
        let food = layout.cell_rect(game.snake.food.unwrap());
        assert_eq!(
            canvas.get(food.x + food.w / 2, food.y + food.h / 2),
            Some(RED)
        );
    }
}
