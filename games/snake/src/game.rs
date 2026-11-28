//! Snake screens with no engine code: ready, playing, paused and over.
//! `main.rs` feeds it input events and the clock and shows what
//! [`Game::draw`] paints.

use pixelfont::Rect;
use sandbox_ui::widgets::{
    BACKGROUND, DIM, GOLD, GREEN, RED, WHITE, centered, draw_box, draw_button, draw_tile, label,
    shade, title,
};
use sandbox_ui::{BLACK, Canvas, PointerId};
use snake_core::{COLUMNS, Cell, Direction, ROWS, Snake, Status, step_ms};
use touch::{Button, SWIPE_DISTANCE, Swipe, button_at, swipe};

/// Where the best score is saved.
pub const STORAGE_KEY: &str = "snake.best";
/// Longest step the clock may take, so a hidden browser tab does not come
/// back to a burst of steps.
const MAX_STEP_MS: u64 = 100;
/// The game over screen ignores input this long, so a late swipe does not
/// start the next game straight away.
pub const RESTART_DELAY_MS: u64 = 600;
const GRID_DOT: (u8, u8, u8) = (36, 36, 64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Space,
    Pause,
    Escape,
}

/// Input in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Key(Key),
    Press { id: PointerId, pos: (i32, i32) },
    Move { id: PointerId, pos: (i32, i32) },
    Release { id: PointerId },
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
    pub cell: i32,
    /// Top-left of the board.
    pub board: (i32, i32),
    pub title_pos: (i32, i32),
    /// SCORE and BEST.
    pub stat_boxes: [Rect; 2],
    pub buttons: Vec<Button>,
    /// Help lines and where the first one goes.
    pub help: &'static [&'static str],
    pub help_pos: (i32, i32),
}

impl Layout {
    pub fn board_rect(&self) -> Rect {
        Rect::new(
            self.board.0,
            self.board.1,
            COLUMNS * self.cell,
            ROWS * self.cell,
        )
    }

    pub fn cell_rect(&self, (x, y): Cell) -> Rect {
        Rect::new(
            self.board.0 + x * self.cell,
            self.board.1 + y * self.cell,
            self.cell,
            self.cell,
        )
    }
}

pub fn landscape() -> Layout {
    Layout {
        size: (480, 360),
        cell: 16,
        board: (16, 20),
        title_pos: (411, 30),
        stat_boxes: [Rect::new(356, 64, 112, 44), Rect::new(356, 124, 112, 44)],
        buttons: vec![
            Button::new("pause", "PAUSE", Rect::new(356, 262, 112, 36)),
            Button::new("end", "END", Rect::new(356, 306, 112, 36)),
        ],
        help: &["ARROWS: TURN", "SWIPE: TURN", "P: PAUSE", "ESC: END"],
        help_pos: (360, 184),
    }
}

pub fn portrait() -> Layout {
    Layout {
        size: (360, 640),
        cell: 17,
        board: (10, 62),
        title_pos: (60, 30),
        stat_boxes: [Rect::new(130, 10, 105, 40), Rect::new(245, 10, 105, 40)],
        buttons: vec![
            Button::new("pause", "PAUSE", Rect::new(10, 414, 105, 40)),
            Button::new("end", "END", Rect::new(245, 414, 105, 40)),
            Button::new("up", "UP", Rect::new(125, 414, 110, 70)),
            Button::new("left", "<", Rect::new(10, 490, 140, 70)),
            Button::new("right", ">", Rect::new(210, 490, 140, 70)),
            Button::new("down", "DOWN", Rect::new(125, 566, 110, 70)),
        ],
        help: &[],
        help_pos: (0, 0),
    }
}

fn direction(swipe: Swipe) -> Direction {
    match swipe {
        Swipe::Up => Direction::Up,
        Swipe::Down => Direction::Down,
        Swipe::Left => Direction::Left,
        Swipe::Right => Direction::Right,
    }
}

pub struct Game {
    pub web: bool,
    pub phase: Phase,
    pub snake: Snake,
    pub best: u32,
    /// The finished game set a new best.
    pub new_best: bool,
    rng: fastrand::Rng,
    now_ms: u64,
    last_ms: Option<u64>,
    /// Time owed to the next steps.
    owed_ms: u64,
    over_ms: u64,
    /// Where each pressed pointer last turned the snake (swipes re-arm there).
    anchors: Vec<(PointerId, (i32, i32))>,
}

impl Game {
    pub fn new(web: bool, seed: u64, best: u32) -> Self {
        let mut rng = fastrand::Rng::with_seed(seed);
        Self {
            web,
            phase: Phase::Ready,
            snake: Snake::new(&mut rng),
            best,
            new_best: false,
            rng,
            now_ms: 0,
            last_ms: None,
            owed_ms: 0,
            over_ms: 0,
            anchors: Vec::new(),
        }
    }

    /// Advance the clock to `now_ms`, stepping the snake as often as its
    /// speed asks.
    pub fn update(&mut self, now_ms: u64) -> Outcome {
        let delta = self
            .last_ms
            .map_or(0, |last| now_ms.saturating_sub(last).min(MAX_STEP_MS));
        self.last_ms = Some(now_ms);
        self.now_ms = now_ms;
        if self.phase != Phase::Playing {
            return Outcome::Continue;
        }
        self.owed_ms += delta;
        loop {
            let interval = step_ms(self.snake.score);
            if self.owed_ms < interval {
                return Outcome::Continue;
            }
            self.owed_ms -= interval;
            self.snake.step(&mut self.rng);
            if self.snake.status != Status::Running {
                return self.finish();
            }
        }
    }

    pub fn handle(&mut self, event: Event, layout: &Layout) -> Outcome {
        match event {
            Event::Key(key) => self.key(key),
            Event::Press { id, pos } => {
                if id == PointerId::MouseRight {
                    return Outcome::Continue;
                }
                self.anchors.retain(|&(known, _)| known != id);
                if let Some(action) = button_at(&layout.buttons, pos) {
                    return self.action(action);
                }
                self.anchors.push((id, pos));
                self.go();
                Outcome::Continue
            }
            Event::Move { id, pos } => {
                let turn = self
                    .anchors
                    .iter_mut()
                    .find(|(known, _)| *known == id)
                    .and_then(|(_, anchor)| {
                        let found = swipe(*anchor, pos, SWIPE_DISTANCE);
                        if found.is_some() {
                            *anchor = pos;
                        }
                        found
                    });
                if let Some(turn) = turn {
                    self.steer(direction(turn));
                }
                Outcome::Continue
            }
            Event::Release { id } => {
                self.anchors.retain(|&(known, _)| known != id);
                Outcome::Continue
            }
        }
    }

    fn key(&mut self, key: Key) -> Outcome {
        match key {
            Key::Up => self.steer(Direction::Up),
            Key::Down => self.steer(Direction::Down),
            Key::Left => self.steer(Direction::Left),
            Key::Right => self.steer(Direction::Right),
            Key::Enter => self.go(),
            Key::Space | Key::Pause => return self.action("pause"),
            Key::Escape => {
                if matches!(self.phase, Phase::Playing | Phase::Paused) {
                    return self.finish();
                }
                if !self.web {
                    return Outcome::Quit;
                }
            }
        }
        Outcome::Continue
    }

    fn action(&mut self, action: &str) -> Outcome {
        match action {
            "up" => self.steer(Direction::Up),
            "down" => self.steer(Direction::Down),
            "left" => self.steer(Direction::Left),
            "right" => self.steer(Direction::Right),
            "pause" => match self.phase {
                Phase::Playing => self.phase = Phase::Paused,
                _ => self.go(),
            },
            "end" if matches!(self.phase, Phase::Playing | Phase::Paused) => {
                return self.finish();
            }
            _ => {}
        }
        Outcome::Continue
    }

    /// Start, continue or restart, whichever fits the screen.
    fn go(&mut self) {
        match self.phase {
            Phase::Ready => self.phase = Phase::Playing,
            Phase::Paused => self.phase = Phase::Playing,
            Phase::Over if self.now_ms >= self.over_ms + RESTART_DELAY_MS => self.restart(),
            Phase::Playing | Phase::Over => {}
        }
    }

    /// Turn while playing; a turn on the ready screen also starts.
    fn steer(&mut self, direction: Direction) {
        match self.phase {
            Phase::Ready => {
                self.phase = Phase::Playing;
                self.snake.turn(direction);
            }
            Phase::Playing => self.snake.turn(direction),
            Phase::Paused | Phase::Over => {}
        }
    }

    fn restart(&mut self) {
        self.snake = Snake::new(&mut self.rng);
        self.phase = Phase::Playing;
        self.owed_ms = 0;
        self.new_best = false;
    }

    fn finish(&mut self) -> Outcome {
        self.phase = Phase::Over;
        self.over_ms = self.now_ms;
        self.owed_ms = 0;
        if self.snake.score > self.best {
            self.best = self.snake.score;
            self.new_best = true;
            return Outcome::Save;
        }
        Outcome::Continue
    }

    pub fn draw(&self, canvas: &mut Canvas, layout: &Layout) {
        canvas.fill(BLACK);
        title(canvas, "SNAKE", layout.title_pos, GREEN, 3);
        let stats = [("SCORE", self.snake.score), ("BEST", self.best)];
        for (rect, (name, value)) in layout.stat_boxes.iter().zip(stats) {
            draw_box(canvas, *rect, Some(name));
            let middle = (rect.x + rect.w / 2, rect.y + rect.h / 2 + 2);
            centered(canvas, &value.to_string(), middle, WHITE, 2);
        }
        for (i, line) in layout.help.iter().enumerate() {
            let (x, y) = layout.help_pos;
            label(canvas, line, (x, y + i as i32 * 14), DIM, 1);
        }
        for button in &layout.buttons {
            let label = if button.action == "pause" && self.phase != Phase::Playing {
                "GO"
            } else {
                button.label
            };
            let shown = Button::new(button.action, label, button.rect);
            draw_button(canvas, &shown, WHITE);
        }
        self.draw_board(canvas, layout);
        self.draw_message(canvas, layout);
    }

    fn draw_board(&self, canvas: &mut Canvas, layout: &Layout) {
        let board = layout.board_rect();
        draw_box(
            canvas,
            Rect::new(board.x - 4, board.y - 4, board.w + 8, board.h + 8),
            None,
        );
        canvas.fill_rect(board, BACKGROUND);
        for y in 1..ROWS {
            for x in 1..COLUMNS {
                let cell = layout.cell_rect((x, y));
                canvas.set(cell.x, cell.y, GRID_DOT);
            }
        }
        let inset = |r: Rect| Rect::new(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
        if let Some(food) = self.snake.food {
            draw_tile(canvas, inset(layout.cell_rect(food)), RED);
        }
        let dead = self.snake.status == Status::Dead;
        for (i, &cell) in self.snake.body.iter().enumerate().rev() {
            let color = match (i, dead) {
                (0, true) => GOLD,
                (0, false) => shade(GREEN, 0.3),
                _ => GREEN,
            };
            draw_tile(canvas, inset(layout.cell_rect(cell)), color);
        }
    }

    fn draw_message(&self, canvas: &mut Canvas, layout: &Layout) {
        let score = format!("SCORE {}", self.snake.score);
        let lines: Vec<(&str, _)> = match self.phase {
            Phase::Playing => return,
            Phase::Ready => vec![
                ("READY?", GOLD),
                ("ENTER OR TAP", WHITE),
                ("ARROWS OR SWIPE", DIM),
            ],
            Phase::Paused => vec![("PAUSED", GOLD), ("TAP TO GO ON", WHITE)],
            Phase::Over => {
                let headline = if self.snake.status == Status::Won {
                    "YOU WIN!"
                } else {
                    "GAME OVER"
                };
                let mut lines = vec![(headline, GOLD), (score.as_str(), WHITE)];
                if self.new_best {
                    lines.push(("NEW BEST!", GREEN));
                }
                lines.push(("TAP TO PLAY", DIM));
                lines
            }
        };
        let board = layout.board_rect();
        let height = 20 + lines.len() as i32 * 24;
        let rect = Rect::new(
            board.x + 20,
            board.y + (board.h - height) / 2,
            board.w - 40,
            height,
        );
        draw_box(canvas, rect, None);
        for (i, (text, color)) in lines.into_iter().enumerate() {
            let y = rect.y + 22 + i as i32 * 24;
            centered(canvas, text, (rect.x + rect.w / 2, y), color, 2);
        }
    }
}

#[cfg(test)]
mod tests;
