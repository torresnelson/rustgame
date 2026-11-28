//! Screen and input tests for the pong port.

use super::*;

fn game() -> Game {
    Game::new(false, 1)
}

fn step(game: &mut Game, events: &[Event]) -> Outcome {
    game.step(events, &Held::default())
}

fn key(key: Key) -> Event {
    Event::Key(key)
}

fn center(r: Rect) -> (i32, i32) {
    (r.x + r.w / 2, r.y + r.h / 2)
}

fn playing(one_player: bool) -> Game {
    let mut game = game();
    let start = if one_player { Key::One } else { Key::Two };
    step(&mut game, &[key(start)]);
    assert_eq!(game.state, State::Playing);
    game
}

#[test]
fn the_menu_chooses_with_keys_and_starts() {
    let mut game = game();
    step(&mut game, &[key(Key::Down)]);
    assert_eq!(game.menu_option, MenuOption::TwoPlayers);
    assert_eq!(game.state, State::Menu);
    step(&mut game, &[key(Key::W)]);
    assert_eq!(game.menu_option, MenuOption::OnePlayer);
    step(&mut game, &[key(Key::S), key(Key::Enter)]);
    assert_eq!(game.state, State::Playing);
    assert!(!game.one_player);
}

#[test]
fn number_keys_start_an_option_directly() {
    assert!(playing(true).one_player);
    assert!(!playing(false).one_player);
}

#[test]
fn hover_selects_and_a_click_or_tap_starts() {
    let two = center(menu_option_rect(MenuOption::TwoPlayers));
    let mut game = game();
    step(&mut game, &[Event::Hover(two)]);
    assert_eq!(
        (game.state, game.menu_option),
        (State::Menu, MenuOption::TwoPlayers)
    );
    step(&mut game, &[Event::Click(two)]);
    assert_eq!(game.state, State::Playing);
    assert!(!game.one_player);

    let mut game = self::game();
    step(&mut game, &[Event::Tap(two)]);
    assert_eq!(game.state, State::Playing);
    assert!(game.touch_seen);
    // A click outside the options does nothing.
    let mut game = self::game();
    step(&mut game, &[Event::Click((10, 10))]);
    assert_eq!(game.state, State::Menu);
}

#[test]
fn menu_options_do_not_overlap() {
    let (one, two) = (
        menu_option_rect(MenuOption::OnePlayer),
        menu_option_rect(MenuOption::TwoPlayers),
    );
    assert!(one.y + one.h <= two.y);
    assert!(two.y + two.h <= HEIGHT);
}

#[test]
fn space_or_the_pause_button_pauses_and_a_tap_continues() {
    let mut game = playing(true);
    step(&mut game, &[key(Key::Space)]);
    assert_eq!(game.state, State::Paused);
    step(&mut game, &[key(Key::Space)]);
    assert_eq!(game.state, State::Playing);

    let pause = center(rect(touch_control_rect(TouchControl::Pause)));
    step(&mut game, &[Event::Tap((700, 300))]);
    assert_eq!(game.state, State::Playing);
    step(&mut game, &[Event::Tap(pause)]);
    assert_eq!(game.state, State::Paused);
    let ball = game.ball;
    step(&mut game, &[]);
    assert_eq!(game.ball, ball, "the ball stays still while paused");
    step(&mut game, &[Event::Tap((700, 300))]);
    assert_eq!(game.state, State::Playing);
}

#[test]
fn m_while_paused_goes_back_to_the_menu_and_resets_the_scores() {
    let mut game = playing(true);
    game.player1.score = 4;
    game.player2.match_score = 1;
    step(&mut game, &[key(Key::Space)]);
    step(&mut game, &[key(Key::M)]);
    assert_eq!(game.state, State::Menu);
    assert_eq!((game.player1.score, game.player2.match_score), (0, 0));
}

fn paddle_moves(one_player: bool, held: Held) -> (f32, f32) {
    let mut game = playing(one_player);
    let (y1, y2) = (game.player1.bar.y, game.player2.bar.y);
    game.step(&[], &held);
    (game.player1.bar.y - y1, game.player2.bar.y - y2)
}

#[test]
fn keys_move_the_paddles() {
    let w = Held {
        w: true,
        ..Held::default()
    };
    assert_eq!(paddle_moves(false, w).0, -PADDLE_SPEED);
    let s = Held {
        s: true,
        ..Held::default()
    };
    assert_eq!(paddle_moves(false, s).0, PADDLE_SPEED);
    // With two players the arrows are player 2's.
    let up = Held {
        up: true,
        ..Held::default()
    };
    assert_eq!(paddle_moves(false, up.clone()), (0.0, -PADDLE_SPEED));
    // Alone, they move player 1.
    assert_eq!(paddle_moves(true, up).0, -PADDLE_SPEED);
}

#[test]
fn the_left_third_is_a_touch_controller_when_alone() {
    let top = Held {
        touches: vec![(50, 50)],
        ..Held::default()
    };
    assert_eq!(paddle_moves(true, top.clone()).0, -PADDLE_SPEED);
    let bottom = Held {
        touches: vec![(50, HEIGHT - 10)],
        ..Held::default()
    };
    assert_eq!(paddle_moves(true, bottom).0, PADDLE_SPEED);
    // Not in a two player game, and not elsewhere on the field.
    assert_eq!(paddle_moves(false, top).0, 0.0);
    let middle = Held {
        touches: vec![(WIDTH / 2, HEIGHT / 2)],
        ..Held::default()
    };
    assert_eq!(paddle_moves(true, middle).0, 0.0);
}

#[test]
fn the_computer_plays_player_two_when_alone() {
    let mut game = playing(true);
    game.ball = Ball::new(Vec2::new(700.0, 40.0), Vec2::new(5.0, 0.0));
    let before = game.player2.bar.y;
    step(&mut game, &[]);
    assert!(game.player2.bar.y < before, "it moves towards the ball");
}

#[test]
fn ten_points_end_a_match_and_three_matches_the_game() {
    let mut game = playing(true);
    game.player1.score = 10;
    step(&mut game, &[]);
    assert_eq!(game.state, State::MatchOver);
    assert_eq!(game.player1.match_score, 1);
    step(&mut game, &[key(Key::Space)]);
    assert_eq!(game.state, State::Playing);

    game.player1.match_score = TOTAL_MATCHES - 1;
    game.player1.score = 10;
    step(&mut game, &[]);
    assert_eq!(game.state, State::GameOver);
    // Enter asks for a rematch.
    step(&mut game, &[key(Key::Enter)]);
    assert_eq!(game.state, State::Playing);
    assert_eq!(game.player1.match_score, 0);

    game.state = State::GameOver;
    step(&mut game, &[Event::Tap((400, 200))]);
    assert_eq!(game.state, State::Menu);
}

#[test]
fn escape_quits_on_the_desktop_only() {
    assert_eq!(step(&mut game(), &[key(Key::Escape)]), Outcome::Quit);
    let mut web = Game::new(true, 1);
    assert_eq!(step(&mut web, &[key(Key::Escape)]), Outcome::Continue);
}

/// Every screen, in every mode, at both blink phases. `label` and
/// `centered` assert that each text is drawable and on screen.
#[test]
fn every_screen_draws_its_texts_on_screen() {
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    for web in [false, true] {
        for touch_seen in [false, true] {
            for one_player in [false, true] {
                for state in [
                    State::Menu,
                    State::Playing,
                    State::Paused,
                    State::MatchOver,
                    State::GameOver,
                ] {
                    for winner_is_one in [false, true] {
                        for frames in [0, 30] {
                            let mut game = Game::new(web, 1);
                            game.state = state;
                            game.touch_seen = touch_seen;
                            game.one_player = one_player;
                            game.frames = frames;
                            game.player1.score = 10;
                            game.player2.score = 10;
                            game.computer.level = MAX_LEVEL;
                            let winner = if winner_is_one {
                                &mut game.player1
                            } else {
                                &mut game.player2
                            };
                            winner.match_score = TOTAL_MATCHES;
                            game.draw(&mut canvas);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn the_field_shows_paddles_ball_and_line() {
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut game = playing(true);
    game.ball = Ball::new(Vec2::new(200.0, 300.0), Vec2::new(1.0, 0.0));
    game.draw(&mut canvas);
    assert_eq!(canvas.get(200, 300), Some(RED));
    assert_eq!(canvas.get(WIDTH / 2, HEIGHT - 1), Some(WHITE));
    let (left, right) = (rect(game.player1.bar), rect(game.player2.bar));
    assert_eq!(canvas.get(left.x + 1, left.y + 1), Some(WHITE));
    assert_eq!(canvas.get(right.x + 1, right.y + 1), Some(BLUE));
    // No touch controls until the screen is touched.
    assert_eq!(canvas.get(133, 110), Some(BLACK));
    game.touch_seen = true;
    game.draw(&mut canvas);
    assert_eq!(canvas.get(133, 110), Some(FADE_15));
}

#[test]
fn the_ball_is_a_round_disc() {
    let mut canvas = Canvas::new(60, 60);
    fill_disc(&mut canvas, Vec2::new(30.0, 30.0), 10.0, RED);
    let lit = (0..60)
        .flat_map(|y| (0..60).map(move |x| (x, y)))
        .filter(|&(x, y)| canvas.get(x, y) == Some(RED))
        .count() as f32;
    let area = std::f32::consts::PI * 100.0;
    assert!((lit - area).abs() < area * 0.05, "{lit} pixels");
    assert_eq!(canvas.get(30, 30), Some(RED));
    assert_eq!(canvas.get(21, 30), Some(RED));
    assert_eq!(canvas.get(41, 30), Some(BLACK));
    // Corners of the bounding box stay empty.
    assert_eq!(canvas.get(21, 21), Some(BLACK));
}
