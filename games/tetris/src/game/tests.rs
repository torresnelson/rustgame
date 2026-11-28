//! Ports of `tests/test_tetris_ui.py`, plus behaviour tests for the screens.

use super::*;
use crate::layout::{KEY_ROWS, landscape, portrait};
use tetris_core::{O, RED, YELLOW};

const DAY: &str = "2026-11-27";
const BOARD_TAP: (i32, i32) = (200, 190); // middle of the landscape board

fn new_game(web: bool) -> Game {
    Game::new(web, 7, Vec::new(), DAY.to_owned(), &landscape())
}

/// A game on the play screen at time `now`.
fn playing(now: u64) -> (Game, Layout) {
    let layout = landscape();
    let mut game = new_game(false);
    game.update(now, &layout);
    assert_eq!(
        game.handle(Event::Key(Key::Enter), &layout),
        Outcome::Continue
    );
    (game, layout)
}

fn play(game: &mut Game) -> &mut Play {
    match &mut game.screen {
        Screen::Play(play) => play,
        other => panic!("not playing: {other:?}"),
    }
}

fn inside(rect: Rect, size: (i32, i32)) -> bool {
    rect.x >= 0 && rect.y >= 0 && rect.x + rect.w <= size.0 && rect.y + rect.h <= size.1
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn center(rect: Rect) -> (i32, i32) {
    (rect.x + rect.w / 2, rect.y + rect.h / 2)
}

#[test]
fn shade_blends_towards_white_and_black() {
    assert_eq!(shade((100, 100, 100), 0.5), (177, 177, 177));
    assert_eq!(shade((100, 100, 100), -0.5), (50, 50, 50));
    assert_eq!(shade((10, 20, 30), 0.0), (10, 20, 30));
}

#[test]
fn accept_char_uppercases_and_filters() {
    assert_eq!(accept_char("NEL", 's'), "NELS");
    assert_eq!(accept_char("A", '~'), "A");
    assert_eq!(accept_char("A", '\t'), "A");
    let full = "X".repeat(NAME_LENGTH);
    assert_eq!(accept_char(&full, 'Y'), full);
}

#[test]
fn press_key_types_and_deletes() {
    assert_eq!(press_key("AN", "A"), "ANA");
    assert_eq!(press_key("ANA", "DEL"), "AN");
    assert_eq!(press_key("", "DEL"), "");
    assert_eq!(press_key("A", "ab"), "A");
    let full = "X".repeat(NAME_LENGTH);
    assert_eq!(press_key(&full, "Q"), full);
}

#[test]
fn menu_index_at_maps_rows_inside_the_box() {
    for web in [false, true] {
        let menu = menu_box(web);
        for i in 0..menu_options(web).len() {
            let y = menu.y + 14 + i as i32 * MENU_ROW + 6;
            assert_eq!(menu_index_at((menu.x + menu.w / 2, y), web), Some(i));
        }
        assert_eq!(menu_index_at((menu.x - 5, menu.y + menu.h / 2), web), None);
        assert_eq!(menu_index_at((menu.x + 10, menu.y + menu.h + 5), web), None);
    }
    assert!(!menu_options(true).contains(&"QUIT"));
    assert!(menu_options(false).contains(&"QUIT"));
}

#[test]
fn keyboard_fits_the_view_without_overlapping() {
    let keys = keyboard();
    assert!(keys.iter().all(|key| inside(key.rect, (WIDTH, HEIGHT))));
    for (i, a) in keys.iter().enumerate() {
        for b in &keys[i + 1..] {
            assert!(!overlaps(a.rect, b.rect), "{} {}", a.action, b.action);
        }
    }
}

#[test]
fn keyboard_has_every_character_plus_del_and_ok() {
    let actions: Vec<String> = keyboard().iter().map(|k| k.action.to_owned()).collect();
    let mut expected: Vec<String> = KEY_ROWS.concat().chars().map(String::from).collect();
    expected.extend(["DEL".to_owned(), "OK".to_owned()]);
    assert_eq!(actions, expected);
    let keys = keyboard();
    for key in &keys {
        assert_eq!(button_at(&keys, center(key.rect)), Some(key.action));
    }
}

/// Every box drawn on the game screen; the board includes its 5px frame.
fn layout_parts(layout: &Layout) -> Vec<(String, Rect)> {
    let board = layout.board_rect();
    let frame = Rect::new(board.x - 5, board.y - 5, board.w + 10, board.h + 10);
    let mut parts = vec![
        ("board".to_owned(), frame),
        ("next".to_owned(), layout.next_box),
    ];
    parts.extend(
        layout
            .stat_boxes
            .iter()
            .enumerate()
            .map(|(i, r)| (format!("stat{i}"), *r)),
    );
    parts.extend(
        layout
            .help_boxes
            .iter()
            .map(|(r, title, _)| (title.to_string(), *r)),
    );
    parts.extend(layout.buttons.iter().map(|b| (b.action.to_owned(), b.rect)));
    parts
}

#[test]
fn layout_parts_fit_the_canvas_without_overlapping() {
    for layout in [landscape(), portrait()] {
        let parts = layout_parts(&layout);
        for (name, rect) in &parts {
            assert!(inside(*rect, layout.size), "{} {name}", layout.name);
        }
        for (i, (a_name, a)) in parts.iter().enumerate() {
            for (b_name, b) in &parts[i + 1..] {
                assert!(!overlaps(*a, *b), "{} {a_name} {b_name}", layout.name);
            }
        }
    }
}

#[test]
fn layouts_can_pause_and_end_and_centre_the_view() {
    for layout in [landscape(), portrait()] {
        let actions: Vec<&str> = layout.buttons.iter().map(|b| b.action).collect();
        assert!(actions.contains(&"pause") && actions.contains(&"end"));
        let offset = layout.view_offset();
        assert!(offset >= 0 && offset + HEIGHT <= layout.size.1);
        assert_eq!(layout.size.0, WIDTH);
    }
}

#[test]
fn portrait_is_taller_than_wide_and_has_thumb_controls() {
    let (tall, wide) = (portrait(), landscape());
    assert!(tall.size.1 > tall.size.0);
    let actions: Vec<&str> = tall.buttons.iter().map(|b| b.action).collect();
    for action in ["left", "right", "rotate", "down", "drop"]
        .iter()
        .chain(&REPEATING)
    {
        assert!(actions.contains(action), "{action}");
    }
    assert!(tall.block > wide.block);
}

#[test]
fn apply_move_moves_and_drop_locks() {
    let grid = create_grid(&Locked::new());
    let mut piece = new_piece(&mut fastrand::Rng::with_seed(1));
    let x = piece.x;
    assert!(!apply_move(&mut piece, &grid, "left"));
    assert_eq!(piece.x, x - 1);
    assert!(!apply_move(&mut piece, &grid, "right"));
    assert_eq!(piece.x, x);
    assert!(apply_move(&mut piece, &grid, "drop"));
    assert!(piece.y > 10);
    assert!(!apply_move(&mut piece, &grid, "nothing"));
}

#[test]
fn escape_quits_the_menu_on_desktop_only() {
    let layout = landscape();
    assert_eq!(
        new_game(false).handle(Event::Key(Key::Escape), &layout),
        Outcome::Quit
    );
    let mut web = new_game(true);
    assert_eq!(
        web.handle(Event::Key(Key::Escape), &layout),
        Outcome::Continue
    );
    assert!(matches!(web.screen, Screen::Menu { .. }));
}

#[test]
fn menu_follows_keys_hover_and_taps() {
    let layout = landscape();
    let mut game = new_game(false);
    game.handle(Event::Key(Key::Down), &layout);
    game.handle(Event::Key(Key::Down), &layout);
    assert!(matches!(game.screen, Screen::Menu { selected: 2, .. }));
    game.handle(Event::Key(Key::Down), &layout);
    assert!(matches!(game.screen, Screen::Menu { selected: 0, .. }));

    let menu = menu_box(false);
    let row = |i: i32| (menu.x + 40, menu.y + 14 + i * MENU_ROW + 6);
    game.handle(Event::Hover(row(1)), &layout);
    assert!(matches!(game.screen, Screen::Menu { selected: 1, .. }));
    let press = Event::Press {
        id: PointerId::Touch(1),
        pos: row(1),
    };
    game.handle(press, &layout);
    assert!(matches!(
        game.screen,
        Screen::HighScores { highlight: None }
    ));
    game.handle(Event::Key(Key::Other), &layout);
    assert!(matches!(game.screen, Screen::Menu { .. }));
}

#[test]
fn menu_taps_land_in_the_centred_view_in_portrait() {
    let layout = portrait();
    let mut game = Game::new(true, 3, Vec::new(), DAY.to_owned(), &layout);
    let menu = menu_box(true);
    let pos = (menu.x + 40, menu.y + 20 + layout.view_offset());
    game.handle(
        Event::Press {
            id: PointerId::Touch(1),
            pos,
        },
        &layout,
    );
    assert!(matches!(game.screen, Screen::Play(_)));
}

#[test]
fn a_tap_on_the_board_rotates_and_a_drag_moves() {
    let (mut game, layout) = playing(1000);
    let id = PointerId::Touch(1);
    let before = play(&mut game).current.rotation;
    game.handle(Event::Press { id, pos: BOARD_TAP }, &layout);
    game.update(1100, &layout);
    game.handle(Event::Release { id, pos: BOARD_TAP }, &layout);
    assert_eq!(play(&mut game).current.rotation, before + 1);

    let x = play(&mut game).current.x;
    game.handle(Event::Press { id, pos: BOARD_TAP }, &layout);
    let dragged = (BOARD_TAP.0 - 2 * layout.block - 3, BOARD_TAP.1);
    game.handle(Event::Move { id, pos: dragged }, &layout);
    assert_eq!(play(&mut game).current.x, x - 2);
    // Another finger's movement does not drag the piece.
    let other = PointerId::Touch(2);
    game.handle(
        Event::Move {
            id: other,
            pos: (0, 0),
        },
        &layout,
    );
    assert_eq!(play(&mut game).current.x, x - 2);
}

#[test]
fn held_thumb_buttons_repeat_until_released() {
    let layout = portrait();
    let mut game = Game::new(true, 5, Vec::new(), DAY.to_owned(), &layout);
    game.update(0, &layout);
    game.handle(Event::Key(Key::Enter), &layout);
    let left = layout.buttons.iter().find(|b| b.action == "left").unwrap();
    let id = PointerId::Touch(9);
    let x = play(&mut game).current.x;
    game.handle(
        Event::Press {
            id,
            pos: center(left.rect),
        },
        &layout,
    );
    assert_eq!(play(&mut game).current.x, x - 1);
    game.update(150, &layout);
    assert_eq!(play(&mut game).current.x, x - 1);
    game.update(260, &layout); // 200 ms delay, then every 60 ms: two repeats
    assert_eq!(play(&mut game).current.x, x - 3);
    game.handle(
        Event::Release {
            id,
            pos: center(left.rect),
        },
        &layout,
    );
    game.update(400, &layout);
    assert_eq!(play(&mut game).current.x, x - 3);
}

#[test]
fn a_button_held_through_a_pause_does_not_catch_up_afterwards() {
    let layout = portrait();
    let mut game = Game::new(true, 5, Vec::new(), DAY.to_owned(), &layout);
    game.update(0, &layout);
    game.handle(Event::Key(Key::Enter), &layout);
    let right = layout.buttons.iter().find(|b| b.action == "right").unwrap();
    let id = PointerId::Touch(3);
    let pos = center(right.rect);
    let x = play(&mut game).current.x;
    game.handle(Event::Press { id, pos }, &layout);
    assert_eq!(play(&mut game).current.x, x + 1);
    game.handle(Event::Key(Key::Pause), &layout);
    for t in (100..=2000).step_by(100) {
        game.update(t, &layout);
    }
    game.handle(Event::Key(Key::Pause), &layout);
    game.update(2010, &layout);
    assert_eq!(play(&mut game).current.x, x + 1);
    game.update(2060, &layout); // the next repeat, and only that one
    assert_eq!(play(&mut game).current.x, x + 2);
}

#[test]
fn pause_freezes_the_piece_and_a_board_tap_resumes() {
    let (mut game, layout) = playing(0);
    game.handle(Event::Key(Key::Pause), &layout);
    let y = play(&mut game).current.y;
    game.handle(Event::Key(Key::Left), &layout);
    for t in (100..3000).step_by(100) {
        game.update(t, &layout);
    }
    assert!(play(&mut game).paused);
    assert_eq!(play(&mut game).current.y, y);

    let id = PointerId::MouseLeft;
    game.handle(Event::Press { id, pos: BOARD_TAP }, &layout);
    game.handle(Event::Release { id, pos: BOARD_TAP }, &layout);
    assert!(!play(&mut game).paused);
    game.update(3100, &layout);
    game.update(3200, &layout);
    game.update(3300, &layout);
    game.update(3400, &layout);
    game.update(3500, &layout);
    assert_eq!(play(&mut game).current.y, y + 1);
}

#[test]
fn full_rows_flash_then_clear_and_score() {
    let (mut game, layout) = playing(1000);
    let mut rng = fastrand::Rng::with_seed(2);
    let p = play(&mut game);
    for x in 0..COLUMNS {
        p.locked.insert((x, ROWS - 1), RED);
    }
    p.current = Piece::new(5, 5, O, YELLOW);
    p.lock(1000, &mut rng);
    assert_eq!(p.flash.as_ref().map(|(rows, _)| rows.len()), Some(1));

    // Frozen while flashing.
    let x = p.next.x;
    game.handle(Event::Key(Key::Left), &layout);
    game.update(1000 + FLASHES * FLASH_MS - 1, &layout);
    let p = play(&mut game);
    assert!(p.flash.is_some());
    assert_eq!(p.lines, 0);

    game.update(1000 + FLASHES * FLASH_MS, &layout);
    let p = play(&mut game);
    assert!(p.flash.is_none());
    assert_eq!((p.lines, p.score, p.level), (1, 40, 0));
    assert!(!p.locked.contains_key(&(0, ROWS - 1)));
    assert_eq!(p.locked.len(), 4); // only the O piece, one row lower
    assert_eq!(p.current.x, x);
}

#[test]
fn a_high_score_asks_for_a_name_then_saves_and_highlights_it() {
    let (mut game, layout) = playing(0);
    play(&mut game).score = 500;
    game.handle(Event::Key(Key::Escape), &layout);
    game.update(GAME_OVER_MS - 1, &layout);
    assert!(matches!(game.screen, Screen::Play(_)));
    game.update(GAME_OVER_MS + 50, &layout);
    let Screen::Name { name, .. } = &game.screen else {
        panic!("no name entry: {:?}", game.screen);
    };
    assert_eq!(name, DEFAULT_NAME);

    for _ in 0..DEFAULT_NAME.len() {
        game.handle(Event::Key(Key::Backspace), &layout);
    }
    for char in ['a', '~', 'b'] {
        game.handle(Event::Char(char), &layout);
    }
    let ok = keyboard().into_iter().find(|k| k.action == "OK").unwrap();
    let id = PointerId::Touch(1);
    let outcome = game.handle(
        Event::Press {
            id,
            pos: center(ok.rect),
        },
        &layout,
    );
    assert_eq!(outcome, Outcome::Save);
    // The same finger lifting does not leave the table.
    game.handle(
        Event::Release {
            id,
            pos: center(ok.rect),
        },
        &layout,
    );
    assert!(matches!(
        game.screen,
        Screen::HighScores { highlight: Some(0) }
    ));
    assert_eq!(game.last_name, "AB");
    assert_eq!(game.entries.len(), 1);
    assert_eq!(game.entries[0].name, "AB");
    assert_eq!(game.entries[0].score, 500);
    assert_eq!(game.entries[0].day, DAY);
}

#[test]
fn saving_happens_on_submit() {
    let (mut game, layout) = playing(0);
    play(&mut game).score = 10;
    game.handle(Event::Key(Key::Escape), &layout);
    game.update(GAME_OVER_MS, &layout);
    assert_eq!(game.handle(Event::Key(Key::Enter), &layout), Outcome::Save);
    assert_eq!(game.entries[0].name, DEFAULT_NAME);
}

#[test]
fn no_score_goes_straight_to_the_table() {
    let (mut game, layout) = playing(0);
    game.handle(Event::Key(Key::Escape), &layout);
    game.update(GAME_OVER_MS, &layout);
    assert!(matches!(
        game.screen,
        Screen::HighScores { highlight: None }
    ));
}

#[test]
fn every_screen_draws_in_both_layouts() {
    for layout in [landscape(), portrait()] {
        let mut canvas = Canvas::new(layout.size.0, layout.size.1);
        let (mut game, _) = playing(0);
        game.draw(&mut canvas, &layout);
        // The board frame's top-left tile has a light bevel.
        let board = layout.board_rect();
        assert_eq!(canvas.get(board.x - 5, board.y - 5), Some(shade(GREY, 0.5)));
        assert_eq!(canvas.get(board.x, board.y), Some(BOARD_BG));

        play(&mut game).paused = true;
        game.draw(&mut canvas, &layout);
        for screen in [
            Screen::Menu {
                selected: 0,
                drops: vec![new_drop(&mut fastrand::Rng::with_seed(1), 100)],
            },
            Screen::HighScores { highlight: None },
            Screen::Name {
                result: GameResult {
                    score: 1,
                    lines: 2,
                    level: 3,
                },
                name: "AB".to_owned(),
            },
        ] {
            game.screen = screen;
            game.draw(&mut canvas, &layout);
        }
    }
}
