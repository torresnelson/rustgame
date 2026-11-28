//! Pinball screens with no engine code: ready, playing, paused and over.
//! Each 1/60 s [`Game::step`] takes the keys [`Held`]; presses and
//! releases act through [`Game::handle`]; frames come out on a [`Canvas`].

use pinball_core::{
    BALL_RADIUS, BUMPER_RADIUS, BUMPERS, Event as FieldEvent, FLIPPER_RADIUS, Field, GATE,
    HEIGHT as TABLE_H, Held, LANES, Line, PLUNGER_Y, SLINGS, START_BALLS, Sound, Vec2, WALLS,
    WIDTH as TABLE_W,
};
use pixelfont::Rect;
use sandbox_ui::widgets::{
    BACKGROUND, DIM, GOLD, GREEN, GREY, RED, WHITE, center, centered, draw_box, draw_button,
    fill_capsule, fill_disc, label, shade, title,
};
use sandbox_ui::{BLACK, Canvas, PointerId, Rgb};
use touch::{Button, button_at};

/// Where the best score is saved.
pub const STORAGE_KEY: &str = "pinball.best";
/// Banners ("BALL 2", "BONUS X3") show this many steps.
pub const BANNER_STEPS: u64 = 90;
/// The game over screen ignores taps this many steps.
pub const RESTART_STEPS: u64 = 45;

const WALL: Rgb = (124, 124, 188);
const BUMPER: Rgb = (0, 88, 248);
const FLIPPER: Rgb = (216, 40, 0);
const LANE_OFF: Rgb = (40, 40, 64);
const PLUNGER: Rgb = (188, 188, 188);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    /// Pull the plunger (also starts a game).
    Plunger,
    Start,
    Pause,
    /// Sound on or off.
    Sound,
    Escape,
}

/// The sound (in `sounds::sounds`) for something on the table.
pub fn sound_name(sound: Sound) -> &'static str {
    match sound {
        Sound::Flipper => "flipper",
        Sound::Launch => "launch",
        Sound::Bumper => "bumper",
        Sound::Sling => "sling",
        Sound::Lane => "lane",
        Sound::Bonus => "bonus",
        Sound::Knock => "knock",
        Sound::Drain => "drain",
    }
}

/// Input in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A key went down this frame.
    Key(Key),
    Press {
        id: PointerId,
        pos: (i32, i32),
    },
    Release {
        id: PointerId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Quit,
    /// A new best score: save `Game::best`.
    Save,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Playing,
    Paused,
    Over,
}

/// What a finger or mouse button held down works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Left,
    Right,
    Plunger,
}

/// Where things go on screen, for one orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: (i32, i32),
    /// Top-left of the table.
    pub table: (i32, i32),
    pub title_pos: Option<(i32, i32)>,
    /// SCORE, BEST (if shown), BALL and BONUS.
    pub stat_boxes: Vec<(&'static str, Rect)>,
    /// LAUNCH (hold) and PAUSE.
    pub buttons: Vec<Button>,
    pub help: &'static [&'static str],
    pub help_pos: (i32, i32),
}

impl Layout {
    pub fn table_rect(&self) -> Rect {
        Rect::new(self.table.0, self.table.1, TABLE_W as i32, TABLE_H as i32)
    }

    /// A table point in canvas pixels (fractional).
    pub fn to_canvas(&self, point: Vec2) -> (f32, f32) {
        (self.table.0 as f32 + point.x, self.table.1 as f32 + point.y)
    }

    fn line(&self, (a, b): Line) -> ((f32, f32), (f32, f32)) {
        (self.to_canvas(a.into()), self.to_canvas(b.into()))
    }
}

pub fn landscape() -> Layout {
    Layout {
        size: (640, 500),
        table: (24, 10),
        title_pos: Some((488, 40)),
        stat_boxes: vec![
            ("SCORE", Rect::new(348, 80, 280, 44)),
            ("BEST", Rect::new(348, 136, 280, 44)),
            ("BALL", Rect::new(348, 192, 136, 44)),
            ("BONUS", Rect::new(492, 192, 136, 44)),
        ],
        buttons: vec![
            Button::new("launch", "LAUNCH", Rect::new(348, 440, 136, 44)),
            Button::new("pause", "PAUSE", Rect::new(492, 440, 136, 44)),
        ],
        help: &[
            "LEFT, RIGHT: FLIPPERS",
            "OR THE MOUSE BUTTONS",
            "SPACE OR DOWN: HOLD TO PULL,",
            "LET GO TO LAUNCH",
            "P: PAUSE, S: SOUND ON/OFF",
            "",
            "TOUCH: HOLD EITHER SIDE",
            "OF THE SCREEN TO FLIP",
        ],
        help_pos: (352, 262),
    }
}

pub fn portrait() -> Layout {
    Layout {
        size: (316, 620),
        table: (8, 64),
        title_pos: None,
        stat_boxes: vec![
            ("SCORE", Rect::new(4, 8, 116, 44)),
            ("BALL", Rect::new(124, 8, 60, 44)),
            ("BONUS", Rect::new(188, 8, 64, 44)),
        ],
        buttons: vec![
            Button::new("pause", "II", Rect::new(256, 8, 56, 44)),
            Button::new("launch", "HOLD TO LAUNCH", Rect::new(8, 556, 300, 56)),
        ],
        help: &[],
        help_pos: (0, 0),
    }
}

/// A finger or mouse button held down.
#[derive(Clone, Copy, Debug)]
struct Press {
    id: PointerId,
    role: Role,
}

pub struct Game {
    pub web: bool,
    pub phase: Phase,
    pub field: Field,
    pub best: u32,
    pub new_best: bool,
    /// Steps since the start, for banners.
    pub steps: u64,
    /// A message over the table until the given step.
    pub banner: Option<(String, u64)>,
    pub sound_on: bool,
    /// Sounds to play, oldest first ([`Game::take_sounds`]).
    sounds: Vec<&'static str>,
    over_step: u64,
    presses: Vec<Press>,
}

impl Game {
    pub fn new(web: bool, best: u32) -> Self {
        Self {
            web,
            phase: Phase::Ready,
            field: Field::new(),
            best,
            new_best: false,
            steps: 0,
            banner: None,
            sound_on: true,
            sounds: Vec::new(),
            over_step: 0,
            presses: Vec::new(),
        }
    }

    /// The sounds to play since the last call, by name.
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    /// What is held: the keys, plus every finger and mouse button down.
    pub fn held(&self, keys: Held) -> Held {
        let holds = |role: Role| self.presses.iter().any(|p| p.role == role);
        Held {
            left: keys.left || holds(Role::Left),
            right: keys.right || holds(Role::Right),
            plunger: keys.plunger || holds(Role::Plunger),
        }
    }

    /// One 1/60 s step.
    pub fn step(&mut self, keys: Held) -> Outcome {
        self.steps += 1;
        if self.phase != Phase::Playing {
            return Outcome::Continue;
        }
        let until = self.steps + BANNER_STEPS;
        let event = self.field.step(self.held(keys));
        let sounds = self.field.take_sounds();
        // The game over tune stands in for the last drain.
        let over = event == FieldEvent::GameOver;
        let sounds = sounds.into_iter().filter(|&s| !(over && s == Sound::Drain));
        self.sounds.extend(sounds.map(sound_name));
        match event {
            FieldEvent::Nothing => {}
            FieldEvent::Bonus => {
                let text = format!("BONUS X{}", self.field.multiplier);
                self.banner = Some((text, until));
            }
            FieldEvent::Drained => {
                let ball = START_BALLS - self.field.balls + 1;
                self.banner = Some((format!("BALL {ball}"), until));
            }
            FieldEvent::GameOver => return self.finish(),
        }
        Outcome::Continue
    }

    pub fn handle(&mut self, event: Event, layout: &Layout) -> Outcome {
        match event {
            Event::Key(Key::Escape) if !self.web => return Outcome::Quit,
            Event::Key(Key::Escape) => {
                if self.phase == Phase::Playing {
                    self.phase = Phase::Paused;
                }
            }
            Event::Key(Key::Pause) => self.pause(),
            Event::Key(Key::Sound) => {
                self.sound_on = !self.sound_on;
                if self.sound_on {
                    self.sounds.push("lane"); // a chime to show it is on
                }
            }
            Event::Key(Key::Start | Key::Plunger) if self.phase != Phase::Playing => self.go(),
            Event::Key(Key::Left) => self.flip(Role::Left),
            Event::Key(Key::Right) => self.flip(Role::Right),
            Event::Key(_) => {}
            Event::Press { id, pos } => {
                self.presses.retain(|p| p.id != id);
                let button = if id == PointerId::MouseRight {
                    None
                } else {
                    button_at(&layout.buttons, pos)
                };
                if button == Some("pause") {
                    self.pause();
                    return Outcome::Continue;
                }
                if self.phase != Phase::Playing {
                    self.go();
                    return Outcome::Continue;
                }
                let role = match (button, id) {
                    (Some("launch"), _) => Role::Plunger,
                    (_, PointerId::MouseLeft) => Role::Left,
                    (_, PointerId::MouseRight) => Role::Right,
                    _ if pos.0 < layout.size.0 / 2 => Role::Left,
                    _ => Role::Right,
                };
                self.flip(role);
                self.presses.push(Press { id, role });
            }
            Event::Release { id } => self.presses.retain(|p| p.id != id),
        }
        Outcome::Continue
    }

    /// A flipper button went down: it shifts the lit lanes.
    fn flip(&mut self, role: Role) {
        if self.phase != Phase::Playing {
            return;
        }
        match role {
            Role::Left => self.field.rotate_lanes(-1),
            Role::Right => self.field.rotate_lanes(1),
            Role::Plunger => {}
        }
    }

    /// Start, go on or play again, whichever fits the screen.
    fn go(&mut self) {
        match self.phase {
            Phase::Ready => {
                self.phase = Phase::Playing;
                self.sounds.push("start");
            }
            Phase::Paused => self.phase = Phase::Playing,
            Phase::Playing => {}
            Phase::Over if self.steps >= self.over_step + RESTART_STEPS => {
                self.field = Field::new();
                self.phase = Phase::Playing;
                self.new_best = false;
                self.banner = None;
                self.sounds.push("start");
            }
            Phase::Over => {}
        }
    }

    fn pause(&mut self) {
        match self.phase {
            Phase::Playing => self.phase = Phase::Paused,
            Phase::Paused | Phase::Ready => self.phase = Phase::Playing,
            Phase::Over => {}
        }
        self.presses.clear();
    }

    fn finish(&mut self) -> Outcome {
        self.phase = Phase::Over;
        self.over_step = self.steps;
        self.banner = None;
        self.presses.clear();
        self.sounds.push("over");
        if self.field.score > self.best {
            self.best = self.field.score;
            self.new_best = true;
            return Outcome::Save;
        }
        Outcome::Continue
    }

    /// The ball number shown (1 to 3).
    pub fn ball_number(&self) -> u32 {
        (START_BALLS + 1 - self.field.balls.max(1)).min(START_BALLS)
    }

    pub fn draw(&self, canvas: &mut Canvas, layout: &Layout) {
        canvas.fill(BLACK);
        if let Some(pos) = layout.title_pos {
            title(canvas, "PINBALL", pos, GOLD, 4);
        }
        for &(name, rect) in &layout.stat_boxes {
            let value = match name {
                "SCORE" => self.field.score.to_string(),
                "BEST" => self.best.to_string(),
                "BALL" => self.ball_number().to_string(),
                _ => format!("X{}", self.field.multiplier),
            };
            draw_box(canvas, rect, Some(name));
            let (x, y) = center(rect);
            centered(canvas, &value, (x, y + 2), WHITE, 2);
        }
        for (i, line) in layout.help.iter().enumerate() {
            let (x, y) = layout.help_pos;
            label(canvas, line, (x, y + i as i32 * 14), DIM, 1);
        }
        for button in &layout.buttons {
            let ready = button.action == "launch" && self.field.ball_ready();
            draw_button(canvas, button, if ready { GOLD } else { WHITE });
        }
        self.draw_table(canvas, layout);
        self.draw_message(canvas, layout);
    }

    fn draw_table(&self, canvas: &mut Canvas, layout: &Layout) {
        let table = layout.table_rect();
        draw_box(
            canvas,
            Rect::new(table.x - 4, table.y - 4, table.w + 8, table.h + 8),
            None,
        );
        canvas.fill_rect(table, BACKGROUND);
        let field = &self.field;

        for (i, &x) in LANES.iter().enumerate() {
            let color = if field.lanes[i] { GOLD } else { LANE_OFF };
            fill_disc(canvas, layout.to_canvas(Vec2::new(x, 78.0)), 4.0, color);
        }
        for &wall in &WALLS {
            let (a, b) = layout.line(wall);
            fill_capsule(canvas, a, b, 1.5, WALL);
        }
        if !field.in_lane {
            let (a, b) = layout.line(GATE);
            fill_capsule(canvas, a, b, 1.0, DIM);
        }
        for (i, &sling) in SLINGS.iter().enumerate() {
            let color = if field.sling_flash[i] > 0 { WHITE } else { RED };
            let (a, b) = layout.line(sling);
            fill_capsule(canvas, a, b, 2.5, color);
        }
        for (i, &bumper) in BUMPERS.iter().enumerate() {
            let at = layout.to_canvas(bumper.into());
            let lit = field.bumper_flash[i] > 0;
            fill_disc(canvas, at, BUMPER_RADIUS, if lit { WHITE } else { BUMPER });
            fill_disc(
                canvas,
                at,
                BUMPER_RADIUS - 5.0,
                if lit { GOLD } else { GREY },
            );
            fill_disc(canvas, at, 3.0, WHITE);
        }

        // The plunger: drawn lower the further it is pulled.
        let pull = (field.plunger * 14.0).round() as i32;
        let (x, y) = layout.to_canvas(Vec2::new(280.0, PLUNGER_Y));
        let plunger = Rect::new(x as i32, y as i32 + pull, 16, 18 - pull);
        canvas.fill_rect(plunger, PLUNGER);
        canvas.fill_rect(Rect::new(plunger.x, plunger.y, plunger.w, 2), WHITE);

        for flipper in &field.flippers {
            let (pivot, tip) = (
                layout.to_canvas(flipper.pivot),
                layout.to_canvas(flipper.tip()),
            );
            fill_capsule(canvas, pivot, tip, FLIPPER_RADIUS, FLIPPER);
            fill_disc(canvas, pivot, 1.5, WHITE);
        }

        if self.phase != Phase::Over {
            let (x, y) = layout.to_canvas(field.ball.pos);
            fill_disc(canvas, (x, y), BALL_RADIUS, shade(WHITE, -0.15));
            canvas.fill_rect(Rect::new(x as i32 - 2, y as i32 - 2, 2, 2), WHITE);
        }
        if !self.sound_on {
            // Below the left inlane, a corner the ball never reaches.
            label(canvas, "SOUND OFF", (table.x + 4, table.y + 468), DIM, 1);
        }
    }

    fn draw_message(&self, canvas: &mut Canvas, layout: &Layout) {
        let table = layout.table_rect();
        let middle = table.x + table.w / 2;
        let best = format!("BEST {}", self.best);
        let score = format!("SCORE {}", self.field.score);
        let lines: Vec<(&str, Rgb)> = match self.phase {
            Phase::Ready => vec![
                ("PINBALL", GOLD),
                ("TAP OR SPACE", WHITE),
                (best.as_str(), DIM),
            ],
            Phase::Paused => vec![("PAUSED", GOLD), ("TAP TO GO ON", WHITE)],
            Phase::Over => {
                let mut lines = vec![("GAME OVER", GOLD), (score.as_str(), WHITE)];
                if self.new_best {
                    lines.push(("NEW BEST!", GREEN));
                }
                lines.push(("TAP TO PLAY", DIM));
                lines
            }
            Phase::Playing => {
                let banner = self
                    .banner
                    .as_ref()
                    .filter(|(_, until)| self.steps < *until);
                if let Some((text, _)) = banner {
                    centered(canvas, text, (middle, table.y + 290), GOLD, 2);
                } else if self.field.ball_ready() {
                    centered(canvas, "HOLD LAUNCH", (middle, table.y + 290), DIM, 2);
                }
                return;
            }
        };
        let height = 20 + lines.len() as i32 * 26;
        let rect = Rect::new(
            table.x + 30,
            table.y + 270 - height / 2,
            table.w - 60,
            height,
        );
        draw_box(canvas, rect, None);
        for (i, (text, color)) in lines.into_iter().enumerate() {
            let y = rect.y + 23 + i as i32 * 26;
            centered(canvas, text, (middle, y), color, 2);
        }
    }
}

#[cfg(test)]
mod tests;
