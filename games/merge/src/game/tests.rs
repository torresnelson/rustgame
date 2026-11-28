//! Screen and input tests for merge.

use super::*;
use merge_core::{Board, GOAL, empty_cells};

const FINGER: PointerId = PointerId::Touch(1);

fn game() -> Game {
    Game::new(false, 8, 100)
}

fn key(game: &mut Game, key: Key) -> Outcome {
    game.handle(Event::Key(key), &landscape())
}

fn with_board(board: Board) -> Game {
    let mut game = game();
    game.puzzle.board = board;
    game
}

const ONE_PAIR: Board = [[1, 1, 0, 0], [0; 4], [0; 4], [0; 4]];

/// Full, with one merge left (the 8s top left). Sliding left opens the top
/// right cell; whatever comes there (a 2 or a 4) nothing can merge again.
const LAST_MOVE: Board = [[3, 3, 5, 6], [7, 8, 7, 8], [8, 7, 8, 7], [7, 8, 7, 8]];

#[test]
fn keys_slide_and_score() {
    let mut game = with_board(ONE_PAIR);
    key(&mut game, Key::Left);
    assert_eq!(game.puzzle.board[0][0], 2);
    assert_eq!(game.puzzle.score, 4);
    assert_eq!(empty_cells(&game.puzzle.board).len(), 14);
}

#[test]
fn a_swipe_slides_once_per_press() {
    let layout = portrait();
    let mut game = with_board(ONE_PAIR);
    game.handle(
        Event::Press {
            id: FINGER,
            pos: (200, 200),
        },
        &layout,
    );
    game.handle(
        Event::Move {
            id: FINGER,
            pos: (210, 205),
        },
        &layout,
    );
    assert_eq!(game.puzzle.board, ONE_PAIR, "too short");
    game.handle(
        Event::Move {
            id: FINGER,
            pos: (150, 205),
        },
        &layout,
    );
    assert_eq!(game.puzzle.board[0][0], 2);
    let after = game.puzzle.board;
    game.handle(
        Event::Move {
            id: FINGER,
            pos: (150, 400),
        },
        &layout,
    );
    assert_eq!(game.puzzle.board, after, "the same press slides once");
    game.handle(Event::Release { id: FINGER }, &layout);
}

#[test]
fn a_new_best_asks_to_save() {
    let mut game = with_board([[6, 6, 0, 0], [0; 4], [0; 4], [0; 4]]);
    assert_eq!(key(&mut game, Key::Left), Outcome::Save);
    assert_eq!(game.best, 128);
    let mut game = with_board(ONE_PAIR);
    assert_eq!(key(&mut game, Key::Left), Outcome::Continue);
    assert_eq!(game.best, 100);
}

#[test]
fn the_goal_stops_once_then_play_goes_on() {
    let layout = landscape();
    let mut game = with_board([[GOAL - 1, GOAL - 1, 0, 0], [0; 4], [0; 4], [0; 4]]);
    key(&mut game, Key::Left);
    assert_eq!(game.phase, Phase::Goal);
    game.handle(
        Event::Press {
            id: FINGER,
            pos: (100, 100),
        },
        &layout,
    );
    assert_eq!(game.phase, Phase::Playing);
    assert!(game.kept_going);
    game.puzzle.board[1] = [GOAL, GOAL, 0, 0];
    key(&mut game, Key::Up);
    key(&mut game, Key::Left);
    assert_eq!(game.phase, Phase::Playing, "no second stop");
}

#[test]
fn a_slide_on_the_goal_screen_goes_on_and_slides() {
    let mut game = with_board([[GOAL - 1, GOAL - 1, 0, 0], [0; 4], [0; 4], [0; 4]]);
    key(&mut game, Key::Left);
    let before = game.puzzle.board;
    key(&mut game, Key::Right);
    assert_eq!(game.phase, Phase::Playing);
    assert_ne!(game.puzzle.board, before);
}

#[test]
fn no_moves_left_ends_and_new_starts_over() {
    let layout = landscape();
    let mut game = with_board(LAST_MOVE);
    key(&mut game, Key::Up);
    assert_eq!(game.phase, Phase::Playing, "a move that changes nothing");
    key(&mut game, Key::Left);
    assert_eq!(game.phase, Phase::Over);
    let stuck = game.puzzle.board;
    key(&mut game, Key::Right);
    assert_eq!(game.puzzle.board, stuck, "slides do nothing");
    let new = layout.buttons[0].rect;
    game.handle(
        Event::Press {
            id: FINGER,
            pos: center(new),
        },
        &layout,
    );
    assert_eq!(game.phase, Phase::Playing);
    assert_eq!(game.puzzle.score, 0);
    assert_eq!(empty_cells(&game.puzzle.board).len(), SIZE * SIZE - 2);
}

#[test]
fn enter_retries_after_the_end() {
    let mut game = with_board(LAST_MOVE);
    key(&mut game, Key::Left);
    key(&mut game, Key::Enter);
    assert_eq!(game.phase, Phase::Playing);
    assert_eq!(game.puzzle.score, 0);
}

#[test]
fn escape_quits_on_the_desktop_only() {
    assert_eq!(key(&mut game(), Key::Escape), Outcome::Quit);
    let mut web = Game::new(true, 8, 0);
    assert_eq!(key(&mut web, Key::Escape), Outcome::Continue);
}

#[test]
fn number_scale_shrinks_long_numbers() {
    let tile = landscape().tile();
    assert_eq!(number_scale("2", tile), 4);
    assert_eq!(number_scale("128", tile), 3);
    assert_eq!(number_scale("2048", tile), 2);
    assert!(number_scale("131072", tile) >= 1);
}

#[test]
fn tile_colors_differ_up_to_the_goal() {
    for a in 1..=GOAL {
        for b in a + 1..=GOAL {
            assert_ne!(tile_color(a), tile_color(b), "{a} and {b}");
        }
    }
    assert_eq!(tile_color(GOAL + 1), WHITE);
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
        let mut rects: Vec<Rect> = layout.buttons.iter().map(|b| b.rect).collect();
        rects.extend(layout.stat_boxes);
        rects.push(layout.board);
        for (i, a) in rects.iter().enumerate() {
            assert!(inside(*a, layout.size), "{a:?}");
            for b in &rects[i + 1..] {
                assert!(!overlaps(*a, *b), "{a:?} and {b:?}");
            }
        }
        let last = layout.tile_rect(SIZE - 1, SIZE - 1);
        assert!(last.x + last.w + GAP <= layout.board.x + layout.board.w);
    }
}

#[test]
fn every_screen_draws_in_both_layouts() {
    for layout in [landscape(), portrait()] {
        let mut canvas = Canvas::new(layout.size.0, layout.size.1);
        let mut game = game();
        game.draw(&mut canvas, &layout);
        // Every tile up to 131072, and a tile popping in.
        let mut board = [[0; SIZE]; SIZE];
        for (i, cell) in board.iter_mut().flatten().enumerate() {
            *cell = i as u8 + 1;
        }
        game.puzzle.board = board;
        game.puzzle.score = 9_999_999;
        game.draw(&mut canvas, &layout);
        game.phase = Phase::Goal;
        game.draw(&mut canvas, &layout);
        game.phase = Phase::Over;
        game.draw(&mut canvas, &layout);
        let mut game = with_board(ONE_PAIR);
        game.update(1000);
        key(&mut game, Key::Left);
        game.draw(&mut canvas, &layout);
        let (r, c) = game.puzzle.spawned.unwrap();
        let tile = layout.tile_rect(r, c);
        assert_ne!(canvas.get(tile.x + 1, tile.y + 1), Some(BLACK));
    }
}
