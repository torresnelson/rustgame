//! The pong screens (port of `src/main.c` from torresnelson/pong), with no
//! engine code: each 1/60 s step takes the [`Event`]s since the last step
//! and what is [`Held`], frames come out on a [`Canvas`].

use pixelfont::Rect;
use pong_core::computer::{Computer, MAX_LEVEL, computer_update};
use pong_core::{
    Ball, FRect, MenuOption, PADDLE_SPEED, Player, RADIUS, SCREEN_HEIGHT, SCREEN_WIDTH, State,
    TOTAL_MATCHES, TouchControl, Vec2, menu_select, referee, reset_game, touch_control_at,
    touch_control_rect,
};
use sandbox_ui::{Canvas, Rgb};

pub const WIDTH: i32 = SCREEN_WIDTH as i32;
pub const HEIGHT: i32 = SCREEN_HEIGHT as i32;
/// Logic steps per second; every speed in `pong_core` is per step.
pub const STEPS_PER_SECOND: f32 = 60.0;
/// At most this many steps catch up after a slow frame, so a stalled tab
/// does not fast-forward the game.
pub const MAX_STEPS_PER_FRAME: u32 = 4;

// raylib's colours, as in the C version.
pub const BLACK: Rgb = (0, 0, 0);
pub const WHITE: Rgb = (255, 255, 255);
pub const RED: Rgb = (230, 41, 55);
pub const BLUE: Rgb = (0, 121, 241);
pub const YELLOW: Rgb = (253, 249, 0);
pub const GRAY: Rgb = (130, 130, 130);
pub const GREEN: Rgb = (0, 228, 48);
/// White at 15 %, 30 %, 45 % and 60 % over black: the C touch controls are
/// translucent, the canvas is opaque.
const FADE_15: Rgb = (38, 38, 38);
const FADE_30: Rgb = (77, 77, 77);
const FADE_45: Rgb = (115, 115, 115);
const FADE_60: Rgb = (153, 153, 153);

const LINE_WIDTH: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    W,
    S,
    Enter,
    Space,
    M,
    One,
    Two,
    Escape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A key press.
    Key(Key),
    /// A finger landed, in canvas pixels.
    Tap((i32, i32)),
    /// A left mouse click, in canvas pixels.
    Click((i32, i32)),
    /// The mouse moved to here.
    Hover((i32, i32)),
}

/// Keys and fingers held down during a step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub up: bool,
    pub down: bool,
    pub w: bool,
    pub s: bool,
    pub touches: Vec<(i32, i32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Quit,
}

/// Whole steps due after `delta` seconds, plus the `carry` left from earlier
/// frames; the remainder is carried on.
pub fn steps_due(carry: &mut f32, delta: f32) -> u32 {
    *carry += delta * STEPS_PER_SECOND;
    let steps = carry.floor();
    if steps > MAX_STEPS_PER_FRAME as f32 {
        *carry = 0.0;
        return MAX_STEPS_PER_FRAME;
    }
    *carry -= steps;
    steps as u32
}

/// Where each main menu option is drawn (also its click and tap area).
pub fn menu_option_rect(option: MenuOption) -> Rect {
    Rect::new(WIDTH / 2 - 110, 355 + option as i32 * 30, 220, 26)
}

fn contains(rect: Rect, (x, y): (i32, i32)) -> bool {
    rect.x <= x && x < rect.x + rect.w && rect.y <= y && y < rect.y + rect.h
}

fn point((x, y): (i32, i32)) -> Vec2 {
    Vec2::new(x as f32, y as f32)
}

fn rect(r: FRect) -> Rect {
    Rect::new(
        r.x.round() as i32,
        r.y.round() as i32,
        r.w.round() as i32,
        r.h.round() as i32,
    )
}

pub struct Game {
    pub state: State,
    pub player1: Player,
    pub player2: Player,
    pub ball: Ball,
    /// Plays player 2 in one player mode.
    pub computer: Computer,
    pub one_player: bool,
    pub menu_option: MenuOption,
    /// Steps since the start, for blinking texts.
    pub frames: u64,
    /// Touch controls and hints show once the screen has been touched.
    pub touch_seen: bool,
    touch_up: bool,
    touch_down: bool,
    /// A browser tab cannot be closed by the game: no Esc to exit.
    pub web: bool,
    rng: fastrand::Rng,
}

impl Game {
    pub fn new(web: bool, seed: u64) -> Self {
        Self {
            state: State::Menu,
            player1: Player::left(),
            player2: Player::right(),
            ball: Ball::serve(),
            computer: Computer::default(),
            one_player: true,
            menu_option: MenuOption::OnePlayer,
            frames: 0,
            touch_seen: false,
            touch_up: false,
            touch_down: false,
            web,
            rng: fastrand::Rng::with_seed(seed),
        }
    }

    /// One 1/60 s step.
    pub fn step(&mut self, events: &[Event], held: &Held) -> Outcome {
        self.frames += 1;
        let pressed = |key: Key| events.contains(&Event::Key(key));
        let taps: Vec<(i32, i32)> = events
            .iter()
            .filter_map(|event| match event {
                Event::Tap(pos) => Some(*pos),
                _ => None,
            })
            .collect();
        let tapped = !taps.is_empty();
        if tapped || !held.touches.is_empty() {
            self.touch_seen = true;
        }
        let held_control = |control| {
            held.touches
                .iter()
                .any(|&pos| touch_control_at(point(pos)) == Some(control))
        };
        self.touch_up = held_control(TouchControl::Up);
        self.touch_down = held_control(TouchControl::Down);
        let tap_pause = taps
            .iter()
            .any(|&pos| touch_control_at(point(pos)) == Some(TouchControl::Pause));

        if !self.web && pressed(Key::Escape) {
            return Outcome::Quit;
        }
        match self.state {
            State::Menu => self.menu(events),
            State::Playing => {
                if pressed(Key::Space) || tap_pause {
                    self.state = State::Paused;
                } else {
                    self.play(held);
                }
            }
            State::Paused => {
                if pressed(Key::Space) || tapped {
                    self.state = State::Playing;
                } else if pressed(Key::M) {
                    self.back_to_menu();
                }
            }
            State::MatchOver => {
                if pressed(Key::Space) || tapped {
                    self.state = State::Playing;
                }
            }
            State::GameOver => {
                if pressed(Key::Enter) {
                    reset_game(&mut self.player1, &mut self.player2);
                    self.state = State::Playing;
                } else if pressed(Key::M) || tapped {
                    self.back_to_menu();
                }
            }
        }
        Outcome::Continue
    }

    /// Choose with the arrows, W/S or the mouse; start with Enter, M or a
    /// click. 1 and 2 start that option right away; a tap on an option
    /// starts it.
    fn menu(&mut self, events: &[Event]) {
        let mut start = false;
        for &event in events {
            match event {
                Event::Key(Key::Enter | Key::M) => start = true,
                Event::Key(Key::Up | Key::W) => {
                    self.menu_option = menu_select(self.menu_option, -1);
                }
                Event::Key(Key::Down | Key::S) => {
                    self.menu_option = menu_select(self.menu_option, 1);
                }
                Event::Key(Key::One) => {
                    self.menu_option = MenuOption::OnePlayer;
                    start = true;
                }
                Event::Key(Key::Two) => {
                    self.menu_option = MenuOption::TwoPlayers;
                    start = true;
                }
                Event::Tap(pos) | Event::Click(pos) | Event::Hover(pos) => {
                    let hit = MenuOption::ALL
                        .into_iter()
                        .find(|&option| contains(menu_option_rect(option), pos));
                    if let Some(option) = hit {
                        self.menu_option = option;
                        start |= !matches!(event, Event::Hover(_));
                    }
                }
                Event::Key(_) => {}
            }
        }
        if start {
            self.one_player = self.menu_option == MenuOption::OnePlayer;
            self.state = State::Playing;
        }
    }

    fn play(&mut self, held: &Held) {
        // Alone against the computer, the arrows and the touch controls move player 1 too.
        let alone = self.one_player;
        let bar = &mut self.player1.bar;
        if held.w || (alone && (held.up || self.touch_up)) {
            bar.y -= PADDLE_SPEED;
        } else if held.s || (alone && (held.down || self.touch_down)) {
            bar.y += PADDLE_SPEED;
        }

        if alone {
            computer_update(
                &mut self.computer,
                &mut self.player2,
                &self.player1,
                &self.ball,
                &mut self.rng,
            );
        } else if held.up {
            self.player2.bar.y -= PADDLE_SPEED;
        } else if held.down {
            self.player2.bar.y += PADDLE_SPEED;
        }

        self.state = referee(
            &mut self.player1,
            &mut self.player2,
            &mut self.ball,
            &mut self.rng,
        );
    }

    fn back_to_menu(&mut self) {
        reset_game(&mut self.player1, &mut self.player2);
        self.state = State::Menu;
    }

    /// Blinking texts show during every other half second.
    fn blink_on(&self) -> bool {
        (self.frames / 30) % 2 == 1
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        canvas.fill(BLACK);
        match self.state {
            State::Playing | State::Paused | State::MatchOver => self.draw_field(canvas),
            State::Menu => self.draw_menu(canvas),
            State::GameOver => self.draw_game_over(canvas),
        }
    }

    fn draw_field(&self, canvas: &mut Canvas) {
        let touch_controls = self.touch_seen && self.one_player;
        if touch_controls {
            draw_touch_arrows(canvas, self.touch_up, self.touch_down);
        }
        canvas.fill_rect(
            Rect::new(WIDTH / 2 - LINE_WIDTH / 2, 0, LINE_WIDTH, HEIGHT),
            WHITE,
        );
        if touch_controls {
            draw_pause_button(canvas, self.state != State::Playing);
        }
        fill_disc(canvas, self.ball.position, RADIUS, RED);
        canvas.fill_rect(rect(self.player1.bar), WHITE);
        canvas.fill_rect(rect(self.player2.bar), BLUE);

        let (left, right) = (WIDTH / 30, WIDTH - WIDTH / 30);
        let opponent = if self.one_player {
            "COMPUTER"
        } else {
            "PLAYER 2"
        };
        let (p1, p2) = (&self.player1, &self.player2);
        label(
            canvas,
            &format!("SCORE PLAYER 1: {}", p1.score),
            (left, 25),
            WHITE,
            2,
        );
        let matches = |won: u32| format!("MATCHES: {won} / {TOTAL_MATCHES}");
        label(canvas, &matches(p1.match_score), (left, 50), WHITE, 2);
        let score = format!("SCORE {opponent}: {}", p2.score);
        label_right(canvas, &score, (right, 25), BLUE, 2);
        label_right(canvas, &matches(p2.match_score), (right, 50), BLUE, 2);
        if self.one_player {
            let level = format!("LEVEL: {} / {MAX_LEVEL}", self.computer.level);
            label_right(canvas, &level, (right, 75), BLUE, 2);
        }

        let middle = WIDTH / 2;
        if self.state == State::Paused {
            if self.blink_on() {
                centered(canvas, "PAUSED", (middle, HEIGHT / 3 + 10), GRAY, 3);
            }
            if self.touch_seen {
                centered(
                    canvas,
                    "TAP TO CONTINUE.",
                    (middle, HEIGHT / 3 + 52),
                    GRAY,
                    2,
                );
            }
        }
        if self.state == State::MatchOver && self.blink_on() {
            let next = if self.touch_seen {
                "TAP TO CONTINUE."
            } else {
                "PRESS SPACE TO CONTINUE."
            };
            centered(canvas, next, (middle, HEIGHT / 2 + 15), GREEN, 3);
        }
    }

    fn draw_menu(&self, canvas: &mut Canvas) {
        let middle = WIDTH / 2;
        // The C title is a red "Pong" with a slightly smaller blue one on top.
        centered(canvas, "PONG", (middle + 4, 170), RED, 16);
        centered(canvas, "PONG", (middle, 170), BLUE, 16);
        centered(
            canvas,
            "TITOREBOOT INC.",
            (middle, 3 * HEIGHT / 4 + 3),
            WHITE,
            1,
        );
        for option in MenuOption::ALL {
            let area = menu_option_rect(option);
            let selected = option == self.menu_option;
            let text = if selected {
                format!("> {} <", option.label())
            } else {
                option.label().to_owned()
            };
            let color = if selected { YELLOW } else { GRAY };
            centered(
                canvas,
                &text,
                (area.x + area.w / 2, area.y + area.h / 2),
                color,
                2,
            );
        }
        let hint = if self.touch_seen {
            "TAP AN OPTION TO START."
        } else {
            "UP/DOWN TO CHOOSE, ENTER TO START, SPACE TO PAUSE IN GAME."
        };
        centered(canvas, hint, (middle, 429), WHITE, 2);
    }

    fn draw_game_over(&self, canvas: &mut Canvas) {
        if self.player1.match_score >= TOTAL_MATCHES {
            label(
                canvas,
                "PLAYER 1 WINS",
                (WIDTH / 3 - 200, HEIGHT / 2),
                WHITE,
                3,
            );
        } else {
            let winner = if self.one_player {
                "COMPUTER WINS"
            } else {
                "PLAYER 2 WINS"
            };
            label(canvas, winner, (WIDTH / 2 + 100, HEIGHT / 2), BLUE, 3);
        }
        let x = WIDTH / 2 - 305;
        if self.touch_seen {
            label(canvas, "TAP TO GO MAIN MENU", (x, 3 * HEIGHT / 4), WHITE, 2);
        } else {
            let rematch = if self.web {
                "PRESS ENTER FOR REMATCH"
            } else {
                "PRESS ENTER FOR REMATCH   PRESS ESC TO EXIT"
            };
            label(canvas, rematch, (x, 3 * HEIGHT / 4), WHITE, 2);
            label(
                canvas,
                "PRESS M TO GO MAIN MENU",
                (x, 6 * HEIGHT / 7),
                WHITE,
                2,
            );
        }
    }
}

/// Pixel font text with its top-left at `pos`.
fn label(canvas: &mut Canvas, text: &str, pos: (i32, i32), color: Rgb, scale: i32) {
    let area = canvas.text(text, pos, color, scale, None, false);
    checked(canvas, text, area);
}

/// Pixel font text with its top-right at `pos`.
fn label_right(canvas: &mut Canvas, text: &str, (x, y): (i32, i32), color: Rgb, scale: i32) {
    let (width, _) = pixelfont::text_size(text, scale);
    label(canvas, text, (x - width, y), color, scale);
}

/// Pixel font text centred on `pos`.
fn centered(canvas: &mut Canvas, text: &str, pos: (i32, i32), color: Rgb, scale: i32) {
    let area = canvas.text(text, pos, color, scale, None, true);
    checked(canvas, text, area);
}

/// Every text must be drawable and on screen; the tests draw every screen.
fn checked(canvas: &Canvas, text: &str, area: Rect) {
    debug_assert!(
        text.chars().all(|c| pixelfont::glyph_rows(c).is_some()),
        "{text:?} has a character the pixel font lacks"
    );
    debug_assert!(
        area.x >= 0
            && area.y >= 0
            && area.x + area.w <= canvas.width()
            && area.y + area.h <= canvas.height(),
        "{text:?} at {area:?} leaves the screen"
    );
}

/// A filled circle, one pixel row at a time.
pub fn fill_disc(canvas: &mut Canvas, center: Vec2, radius: f32, color: Rgb) {
    let top = (center.y - radius).floor() as i32;
    let bottom = (center.y + radius).ceil() as i32;
    for y in top..bottom {
        let dy = y as f32 + 0.5 - center.y;
        if dy.abs() > radius {
            continue;
        }
        let half = (radius * radius - dy * dy).sqrt();
        let (x0, x1) = (
            (center.x - half).round() as i32,
            (center.x + half).round() as i32,
        );
        canvas.fill_rect(Rect::new(x0, y, x1 - x0, 1), color);
    }
}

/// A triangle with a flat top or bottom edge, one pixel row at a time:
/// `width_at(t)` gives the row width for `t` from 0 (top) to 1 (bottom).
fn fill_rows(
    canvas: &mut Canvas,
    center_x: f32,
    top: f32,
    height: f32,
    color: Rgb,
    width_at: impl Fn(f32) -> f32,
) {
    for row in 0..height as i32 {
        let width = width_at((row as f32 + 0.5) / height);
        let x = (center_x - width / 2.0).round() as i32;
        canvas.fill_rect(
            Rect::new(x, (top as i32) + row, width.round() as i32, 1),
            color,
        );
    }
}

/// Player 1's up and down arrows on the left third, brighter while held.
fn draw_touch_arrows(canvas: &mut Canvas, up_held: bool, down_held: bool) {
    let shade = |held: bool| if held { FADE_45 } else { FADE_15 };
    let up = touch_control_rect(TouchControl::Up);
    let down = touch_control_rect(TouchControl::Down);
    let cx = up.x + up.w / 2.0;
    let up_cy = up.y + up.h / 2.0;
    fill_rows(canvas, cx, up_cy - 35.0, 60.0, shade(up_held), |t| 80.0 * t);
    let down_cy = down.y + down.h / 2.0;
    fill_rows(canvas, cx, down_cy - 25.0, 60.0, shade(down_held), |t| {
        80.0 * (1.0 - t)
    });
}

/// The pause button at the top centre: pause bars, or a play arrow while
/// the game is stopped.
fn draw_pause_button(canvas: &mut Canvas, paused: bool) {
    let button = rect(touch_control_rect(TouchControl::Pause));
    canvas.fill_rect(button, BLACK);
    canvas.outline(button, FADE_30, 2);
    if paused {
        let (px, py) = (button.x + button.w / 2, button.y + button.h / 2);
        for row in -15..15 {
            let width = (25.0 * (1.0 - (row as f32 + 0.5).abs() / 15.0)).round() as i32;
            canvas.fill_rect(Rect::new(px - 10, py + row, width, 1), FADE_60);
        }
    } else {
        canvas.fill_rect(Rect::new(button.x + 24, button.y + 15, 8, 30), FADE_60);
        canvas.fill_rect(Rect::new(button.x + 38, button.y + 15, 8, 30), FADE_60);
    }
}

#[cfg(test)]
mod tests;
