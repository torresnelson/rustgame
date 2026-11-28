//! Screen and input tests for minesweeper.

use super::*;

const FINGER: PointerId = PointerId::Touch(1);

fn game() -> Game {
    Game::new(false, 2, [0; 3])
}

fn key(game: &mut Game, key: Key) -> Outcome {
    game.handle(Event::Key(key), &landscape())
}

/// Mines walling in the safe corner (0, 0), so a flood from elsewhere opens
/// every other safe cell but that one.
const POCKET: [Cell; 3] = [(1, 0), (0, 1), (1, 1)];

/// An easy board with the `POCKET` mines.
fn playing() -> Game {
    let mut game = game();
    key(&mut game, Key::One);
    assert_eq!(game.screen, Screen::Play);
    game.field.lay_mines_at(&POCKET);
    game
}

fn middle_of(layout: &Layout, game: &Game, cell: Cell) -> (i32, i32) {
    center(cell_rect(layout, &game.field, cell))
}

fn tap(game: &mut Game, layout: &Layout, pos: (i32, i32)) -> Outcome {
    game.handle(Event::Press { id: FINGER, pos }, layout);
    game.handle(Event::Release { id: FINGER, pos }, layout)
}

fn tap_cell(game: &mut Game, layout: &Layout, cell: Cell) -> Outcome {
    let pos = middle_of(layout, game, cell);
    tap(game, layout, pos)
}

fn button(layout: &Layout, action: &str) -> (i32, i32) {
    center(
        layout
            .buttons
            .iter()
            .find(|b| b.action == action)
            .unwrap()
            .rect,
    )
}

#[test]
fn the_menu_picks_a_difficulty_by_key_or_tap() {
    let layout = landscape();
    let mut game = game();
    key(&mut game, Key::Down);
    key(&mut game, Key::Down);
    assert_eq!(game.screen, Screen::Menu { selected: 2 });
    key(&mut game, Key::Down);
    assert_eq!(game.screen, Screen::Menu { selected: 0 });
    key(&mut game, Key::Up);
    key(&mut game, Key::Enter);
    assert_eq!((game.screen, game.difficulty), (Screen::Play, 2));
    assert_eq!(game.field.columns, DIFFICULTIES[2].columns);

    let mut game = self::game();
    let pos = center(layout.menu_buttons[1]);
    game.handle(Event::Hover(pos), &layout);
    assert_eq!(game.screen, Screen::Menu { selected: 1 });
    game.handle(Event::Press { id: FINGER, pos }, &layout);
    assert_eq!((game.screen, game.difficulty), (Screen::Play, 1));
}

#[test]
fn every_board_fits_its_area_in_both_layouts() {
    for layout in [landscape(), portrait()] {
        for difficulty in DIFFICULTIES {
            let field = Field::new(&difficulty);
            let board = board_rect(&layout, &field);
            let area = layout.board_area;
            assert!(board.x >= area.x && board.x + board.w <= area.x + area.w);
            assert!(board.y >= area.y && board.y + board.h <= area.y + area.h);
            let (cell, _) = board_geometry(&layout, &field);
            assert!(cell >= 15, "{} cells of {cell}", difficulty.name);
            assert_eq!(cell_at(&layout, &field, (board.x, board.y)), Some((0, 0)));
            assert_eq!(cell_at(&layout, &field, (board.x - 1, board.y)), None);
        }
    }
}

#[test]
fn a_tap_digs_and_starts_the_clock() {
    let layout = landscape();
    let mut game = playing();
    game.update(1000, &layout);
    tap_cell(&mut game, &layout, (8, 8));
    assert_eq!(game.field.cover((8, 8)), Cover::Open);
    game.update(4500, &layout);
    assert_eq!(game.seconds(), 3);
}

#[test]
fn the_first_dig_lays_the_mines_away_from_it() {
    let layout = portrait();
    let mut game = game();
    key(&mut game, Key::Two);
    let pos = middle_of(&layout, &game, (3, 3));
    game.handle(Event::Press { id: FINGER, pos }, &layout);
    game.handle(Event::Release { id: FINGER, pos }, &layout);
    assert!(game.field.laid);
    assert_eq!(game.field.status, Status::Playing);
    assert_eq!(game.field.count((3, 3)), 0);
}

#[test]
fn a_drag_is_not_a_tap() {
    let layout = landscape();
    let mut game = playing();
    let pos = middle_of(&layout, &game, (5, 5));
    game.handle(Event::Press { id: FINGER, pos }, &layout);
    game.handle(
        Event::Release {
            id: FINGER,
            pos: (pos.0 + 40, pos.1),
        },
        &layout,
    );
    assert_eq!(game.field.cover((5, 5)), Cover::Hidden);
}

#[test]
fn holding_a_finger_flags_and_the_release_does_nothing_more() {
    let layout = landscape();
    let mut game = playing();
    game.update(100, &layout);
    let pos = middle_of(&layout, &game, (0, 0));
    game.handle(Event::Press { id: FINGER, pos }, &layout);
    game.update(100 + touch::LONG_PRESS_MS - 1, &layout);
    assert_eq!(game.field.cover((0, 0)), Cover::Hidden);
    game.update(100 + touch::LONG_PRESS_MS, &layout);
    assert_eq!(game.field.cover((0, 0)), Cover::Flagged);
    game.update(5000, &layout);
    assert_eq!(game.field.cover((0, 0)), Cover::Flagged, "flags once");
    game.handle(Event::Release { id: FINGER, pos }, &layout);
    assert_eq!(game.field.status, Status::Playing);
    assert_eq!(game.field.cover((0, 0)), Cover::Flagged);
}

#[test]
fn right_click_flags_and_flag_mode_swaps_taps() {
    let layout = landscape();
    let mut game = playing();
    let pos = middle_of(&layout, &game, (1, 0));
    game.handle(
        Event::Press {
            id: PointerId::MouseRight,
            pos,
        },
        &layout,
    );
    assert_eq!(game.field.cover((1, 0)), Cover::Flagged);
    tap(&mut game, &layout, button(&layout, "mode"));
    assert!(game.flag_mode);
    tap_cell(&mut game, &layout, (0, 1));
    assert_eq!(game.field.cover((0, 1)), Cover::Flagged);
    assert_eq!(game.field.mines_left(), 1);
}

#[test]
fn the_keyboard_cursor_digs_and_flags() {
    let mut game = playing();
    key(&mut game, Key::Left);
    assert_eq!(game.cursor, Some((4, 4)));
    for _ in 0..10 {
        key(&mut game, Key::Up);
    }
    assert_eq!(game.cursor, Some((4, 0)), "stops at the edge");
    key(&mut game, Key::Left);
    key(&mut game, Key::Left);
    key(&mut game, Key::Left);
    key(&mut game, Key::Flag);
    assert_eq!(game.field.cover((1, 0)), Cover::Flagged);
    key(&mut game, Key::Down);
    key(&mut game, Key::Down);
    key(&mut game, Key::Space);
    assert_eq!(game.field.cover((1, 2)), Cover::Open);
}

#[test]
fn clearing_the_board_saves_a_best_time() {
    let layout = landscape();
    let mut game = playing();
    game.update(0, &layout);
    tap_cell(&mut game, &layout, (8, 8));
    game.update(12_400, &layout);
    assert_eq!(game.field.status, Status::Playing);
    let outcome = tap_cell(&mut game, &layout, (0, 0));
    assert_eq!(game.field.status, Status::Won);
    assert_eq!(outcome, Outcome::Save);
    assert_eq!(game.bests, [12, 0, 0]);
    assert!(game.new_best);
    assert_eq!(parse_bests(Some(&game.best_text())), [12, 0, 0]);
    // The clock stopped.
    game.update(60_000, &layout);
    assert_eq!(game.seconds(), 12);
}

#[test]
fn a_slower_win_keeps_the_best() {
    let layout = landscape();
    let mut game = Game::new(false, 2, [5, 0, 0]);
    key(&mut game, Key::One);
    game.field.lay_mines_at(&POCKET);
    tap_cell(&mut game, &layout, (8, 8));
    game.update(9000, &layout);
    let outcome = tap_cell(&mut game, &layout, (0, 0));
    assert_eq!(outcome, Outcome::Continue);
    assert_eq!(game.bests[0], 5);
}

#[test]
fn a_mine_loses_and_enter_deals_a_new_board() {
    let layout = landscape();
    let mut game = playing();
    tap_cell(&mut game, &layout, (1, 1));
    assert_eq!(game.field.status, Status::Lost);
    // Nothing more happens on the board.
    tap_cell(&mut game, &layout, (5, 5));
    assert_eq!(game.field.cover((5, 5)), Cover::Hidden);
    key(&mut game, Key::Enter);
    assert_eq!(game.field.status, Status::Playing);
    assert!(!game.field.laid);
    assert_eq!(game.seconds(), 0);
}

#[test]
fn buttons_and_escape_go_back_to_the_menu_then_quit() {
    let layout = portrait();
    let mut game = playing();
    tap(&mut game, &layout, button(&layout, "menu"));
    assert_eq!(game.screen, Screen::Menu { selected: 0 });
    key(&mut game, Key::Three);
    key(&mut game, Key::Escape);
    assert_eq!(game.screen, Screen::Menu { selected: 2 });
    assert_eq!(key(&mut game, Key::Escape), Outcome::Quit);
    let mut web = Game::new(true, 2, [0; 3]);
    assert_eq!(key(&mut web, Key::Escape), Outcome::Continue);
}

#[test]
fn parse_bests_survives_bad_text() {
    assert_eq!(parse_bests(None), [0; 3]);
    assert_eq!(parse_bests(Some("7 x")), [7, 0, 0]);
    assert_eq!(parse_bests(Some("1 2 3 4")), [1, 2, 3]);
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
        rects.push(layout.board_area);
        for (i, a) in rects.iter().enumerate() {
            assert!(inside(*a, layout.size), "{a:?}");
            for b in &rects[i + 1..] {
                assert!(!overlaps(*a, *b), "{a:?} and {b:?}");
            }
        }
        for rect in layout.menu_buttons {
            assert!(inside(rect, layout.size));
        }
    }
}

#[test]
fn every_screen_draws_in_both_layouts() {
    for layout in [landscape(), portrait()] {
        let mut canvas = Canvas::new(layout.size.0, layout.size.1);
        let game = Game::new(true, 2, [12, 345, 999]);
        game.draw(&mut canvas, &layout);
        let mut game = Game::new(false, 2, [12, 345, 999]);
        game.draw(&mut canvas, &layout);
        for difficulty in [Key::One, Key::Two, Key::Three] {
            key(&mut game, difficulty);
            game.field.lay_mines_at(&POCKET);
            game.draw(&mut canvas, &layout);
            key(&mut game, Key::Right);
            // A wrong flag, shown crossed out after the loss.
            game.field.toggle_flag((5, 5));
            tap_cell(&mut game, &layout, (8, 8));
            game.flag_mode = true;
            tap_cell(&mut game, &layout, (0, 1));
            game.draw(&mut canvas, &layout);
            game.flag_mode = false;
            tap_cell(&mut game, &layout, (1, 1));
            assert_eq!(game.field.status, Status::Lost);
            game.draw(&mut canvas, &layout);
            key(&mut game, Key::Escape);
        }
    }
}
