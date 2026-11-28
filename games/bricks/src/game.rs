//! Bricks screens with no engine code: ready, playing, paused and over.
//! Each 1/60 s [`Game::step`] takes what is [`Held`]; presses, drags and
//! the mouse act at once through [`Game::handle`]; frames come out on a
//! [`Canvas`].

use bricks_core::{
    BALL, Event as FieldEvent, FRect, Field, HEIGHT as FIELD_H, PADDLE_SPEED, WIDTH as FIELD_W,
};
use pixelfont::Rect;
use sandbox_ui::widgets::{
    BACKGROUND, DIM, GOLD, GREY, WHITE, center, centered, draw_box, draw_button, draw_tile, label,
    shade, title,
};
use sandbox_ui::{BLACK, Canvas, PointerId, Rgb};
use touch::{Button, TAP_DISTANCE, button_at};

/// Where the best score is saved.
pub const STORAGE_KEY: &str = "bricks.best";
/// Banners ("LEVEL 2", "BALL LOST") show this many steps.
pub const BANNER_STEPS: u64 = 90;
/// The game over screen ignores taps this many steps.
pub const RESTART_STEPS: u64 = 45;

/// Brick colour per row, top first (the NES palette).
const ROW_COLORS: [Rgb; 7] = [
    (216, 40, 0),
    (228, 92, 16),
    (248, 184, 0),
    (0, 168, 0),
    (0, 88, 248),
    (136, 20, 176),
    (0, 136, 136),
];
const PADDLE: Rgb = (60, 188, 252);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Launch,
    Pause,
    Escape,
}

/// Input in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Key(Key),
    /// The mouse moved here.
    Hover((i32, i32)),
    Press {
        id: PointerId,
        pos: (i32, i32),
    },
    Move {
        id: PointerId,
        pos: (i32, i32),
    },
    Release {
        id: PointerId,
        pos: (i32, i32),
    },
}

/// Keys held during a step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub left: bool,
    pub right: bool,
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

/// Where things go on screen, for one orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: (i32, i32),
    /// Top-left of the field.
    pub field: (i32, i32),
    pub title_pos: Option<(i32, i32)>,
    /// SCORE, BEST (if shown), LEVEL and LIVES.
    pub stat_boxes: Vec<(&'static str, Rect)>,
    pub buttons: Vec<Button>,
    pub help: &'static [&'static str],
    pub help_pos: (i32, i32),
    /// A strip under the field to drag the paddle from (portrait).
    pub drag_strip: Option<Rect>,
}

impl Layout {
    pub fn field_rect(&self) -> Rect {
        Rect::new(self.field.0, self.field.1, FIELD_W as i32, FIELD_H as i32)
    }

    /// A field rectangle in canvas pixels.
    pub fn to_canvas(&self, r: FRect) -> Rect {
        Rect::new(
            self.field.0 + r.x.round() as i32,
            self.field.1 + r.y.round() as i32,
            r.w.round() as i32,
            r.h.round() as i32,
        )
    }
}

pub fn landscape() -> Layout {
    Layout {
        size: (640, 500),
        field: (24, 10),
        title_pos: Some((516, 40)),
        stat_boxes: vec![
            ("SCORE", Rect::new(404, 80, 224, 44)),
            ("BEST", Rect::new(404, 136, 224, 44)),
            ("LEVEL", Rect::new(404, 192, 108, 44)),
            ("LIVES", Rect::new(520, 192, 108, 44)),
        ],
        buttons: vec![Button::new("pause", "PAUSE", Rect::new(404, 440, 224, 44))],
        help: &[
            "ARROWS OR MOUSE: MOVE",
            "SPACE OR CLICK: LAUNCH",
            "P: PAUSE",
            "",
            "TOUCH: DRAG TO MOVE,",
            "TAP TO LAUNCH",
        ],
        help_pos: (408, 264),
        drag_strip: None,
    }
}

pub fn portrait() -> Layout {
    Layout {
        size: (376, 680),
        field: (8, 64),
        title_pos: None,
        stat_boxes: vec![
            ("SCORE", Rect::new(4, 10, 120, 44)),
            ("LEVEL", Rect::new(132, 10, 70, 44)),
            ("LIVES", Rect::new(210, 10, 70, 44)),
        ],
        buttons: vec![Button::new("pause", "II", Rect::new(288, 10, 84, 44))],
        help: &[],
        help_pos: (0, 0),
        drag_strip: Some(Rect::new(8, 560, 360, 112)),
    }
}

/// A finger or mouse button held down.
#[derive(Clone, Copy, Debug)]
struct Press {
    id: PointerId,
    start: (i32, i32),
    last_x: i32,
    moved: bool,
}

pub struct Game {
    pub web: bool,
    pub phase: Phase,
    pub field: Field,
    pub best: u32,
    pub new_best: bool,
    /// Steps since the start, for banners and blinking.
    pub steps: u64,
    /// A message over the field until the given step.
    pub banner: Option<(&'static str, u64)>,
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
            over_step: 0,
            presses: Vec::new(),
        }
    }

    /// One 1/60 s step.
    pub fn step(&mut self, held: Held) -> Outcome {
        self.steps += 1;
        if self.phase != Phase::Playing {
            return Outcome::Continue;
        }
        if held.left != held.right {
            let dx = if held.left {
                -PADDLE_SPEED
            } else {
                PADDLE_SPEED
            };
            self.field.move_paddle(dx);
        }
        let until = self.steps + BANNER_STEPS;
        match self.field.step() {
            FieldEvent::Nothing => {}
            FieldEvent::LifeLost => self.banner = Some(("BALL LOST", until)),
            FieldEvent::Cleared => self.banner = Some(("LEVEL CLEARED", until)),
            FieldEvent::GameOver => return self.finish(),
        }
        Outcome::Continue
    }

    pub fn handle(&mut self, event: Event, layout: &Layout) -> Outcome {
        match event {
            Event::Key(Key::Launch) => self.go(),
            Event::Key(Key::Pause) => self.pause(),
            Event::Key(Key::Escape) if !self.web => return Outcome::Quit,
            Event::Key(Key::Escape) => {
                if self.phase == Phase::Playing {
                    self.phase = Phase::Paused;
                }
            }
            Event::Hover((x, _)) => {
                if self.phase == Phase::Playing {
                    self.field.paddle_to((x - layout.field.0) as f32);
                }
            }
            Event::Press { id, pos } => {
                self.presses.retain(|p| p.id != id);
                if id == PointerId::MouseRight {
                    return Outcome::Continue;
                }
                if button_at(&layout.buttons, pos) == Some("pause") {
                    self.pause();
                    return Outcome::Continue;
                }
                self.presses.push(Press {
                    id,
                    start: pos,
                    last_x: pos.0,
                    moved: false,
                });
                if self.phase != Phase::Playing {
                    self.go();
                }
            }
            Event::Move { id, pos } => {
                let Some(press) = self.presses.iter_mut().find(|p| p.id == id) else {
                    return Outcome::Continue;
                };
                let dx = pos.0 - press.last_x;
                press.last_x = pos.0;
                let (sx, sy) = press.start;
                if (pos.0 - sx).abs() > TAP_DISTANCE || (pos.1 - sy).abs() > TAP_DISTANCE {
                    press.moved = true;
                }
                // The mouse moves the paddle by hovering; fingers drag it.
                if id.is_touch() && self.phase == Phase::Playing {
                    self.field.move_paddle(dx as f32);
                }
            }
            Event::Release { id, .. } => {
                let Some(index) = self.presses.iter().position(|p| p.id == id) else {
                    return Outcome::Continue;
                };
                let press = self.presses.remove(index);
                if !press.moved && self.phase == Phase::Playing {
                    self.field.launch();
                }
            }
        }
        Outcome::Continue
    }

    /// Start, go on, launch or play again, whichever fits the screen.
    fn go(&mut self) {
        match self.phase {
            Phase::Ready | Phase::Paused => self.phase = Phase::Playing,
            Phase::Playing => self.field.launch(),
            Phase::Over if self.steps >= self.over_step + RESTART_STEPS => {
                self.field = Field::new();
                self.phase = Phase::Playing;
                self.new_best = false;
                self.banner = None;
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
    }

    fn finish(&mut self) -> Outcome {
        self.phase = Phase::Over;
        self.over_step = self.steps;
        self.banner = None;
        if self.field.score > self.best {
            self.best = self.field.score;
            self.new_best = true;
            return Outcome::Save;
        }
        Outcome::Continue
    }

    pub fn draw(&self, canvas: &mut Canvas, layout: &Layout) {
        canvas.fill(BLACK);
        if let Some(pos) = layout.title_pos {
            title(canvas, "BRICKS", pos, GOLD, 4);
        }
        for &(name, rect) in &layout.stat_boxes {
            let value = match name {
                "SCORE" => self.field.score,
                "BEST" => self.best,
                "LEVEL" => self.field.round + 1,
                _ => self.field.lives,
            };
            draw_box(canvas, rect, Some(name));
            let (x, y) = center(rect);
            centered(canvas, &value.to_string(), (x, y + 2), WHITE, 2);
        }
        for (i, line) in layout.help.iter().enumerate() {
            let (x, y) = layout.help_pos;
            label(canvas, line, (x, y + i as i32 * 14), DIM, 1);
        }
        for button in &layout.buttons {
            draw_button(canvas, button, WHITE);
        }
        if let Some(strip) = layout.drag_strip {
            draw_box(canvas, strip, None);
            centered(canvas, "DRAG HERE TO MOVE", center(strip), DIM, 2);
        }
        self.draw_field(canvas, layout);
        self.draw_message(canvas, layout);
    }

    fn draw_field(&self, canvas: &mut Canvas, layout: &Layout) {
        let field = layout.field_rect();
        draw_box(
            canvas,
            Rect::new(field.x - 4, field.y - 4, field.w + 8, field.h + 8),
            None,
        );
        canvas.fill_rect(field, BACKGROUND);
        for brick in self.field.bricks.iter().filter(|b| b.standing()) {
            let rect = layout.to_canvas(brick.rect());
            let inner = Rect::new(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2);
            let color = if brick.solid {
                GREY
            } else {
                let base = ROW_COLORS[brick.row % ROW_COLORS.len()];
                shade(base, 0.2 * f32::from(brick.hits - 1))
            };
            draw_tile(canvas, inner, color);
            if brick.hits > 1 {
                let (x, y) = center(inner);
                canvas.fill_rect(Rect::new(x - 1, y - 1, 3, 3), shade(color, -0.5));
            }
        }
        draw_tile(canvas, layout.to_canvas(self.field.paddle()), PADDLE);
        let ball = self.field.ball;
        let ball = layout.to_canvas(FRect::new(ball.x, ball.y, BALL, BALL));
        if self.phase != Phase::Over {
            canvas.fill_rect(ball, WHITE);
        }
    }

    fn draw_message(&self, canvas: &mut Canvas, layout: &Layout) {
        let field = layout.field_rect();
        let middle = field.x + field.w / 2;
        let best = format!("BEST {}", self.best);
        let score = format!("SCORE {}", self.field.score);
        let lines: Vec<(&str, Rgb)> = match self.phase {
            Phase::Ready => vec![
                ("BRICKS", GOLD),
                ("TAP OR SPACE", WHITE),
                (best.as_str(), DIM),
            ],
            Phase::Paused => vec![("PAUSED", GOLD), ("TAP TO GO ON", WHITE)],
            Phase::Over => {
                let mut lines = vec![("GAME OVER", GOLD), (score.as_str(), WHITE)];
                if self.new_best {
                    lines.push(("NEW BEST!", (0, 184, 0)));
                }
                lines.push(("TAP TO PLAY", DIM));
                lines
            }
            Phase::Playing => {
                let banner = self.banner.filter(|&(_, until)| self.steps < until);
                if let Some((text, _)) = banner {
                    centered(canvas, text, (middle, field.y + 300), GOLD, 2);
                } else if self.field.stuck {
                    let hint = "LAUNCH: TAP OR SPACE";
                    centered(canvas, hint, (middle, field.y + 300), DIM, 2);
                }
                return;
            }
        };
        let height = 20 + lines.len() as i32 * 26;
        let rect = Rect::new(
            field.x + 30,
            field.y + 260 - height / 2,
            field.w - 60,
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
