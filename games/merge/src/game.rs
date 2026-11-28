//! Merge screens with no engine code: the board, the goal reached and no
//! moves left. `main.rs` feeds it input events and the clock and shows what
//! [`Game::draw`] paints.

use merge_core::{Direction, Puzzle, SIZE, value};
use pixelfont::Rect;
use sandbox_ui::widgets::{
    BACKGROUND, DIM, GOLD, WHITE, center, centered, draw_box, draw_button, draw_tile, label, shade,
    title,
};
use sandbox_ui::{BLACK, Canvas, PointerId, Rgb};
use touch::{Button, SWIPE_DISTANCE, Swipe, button_at, swipe};

/// Where the best score is saved.
pub const STORAGE_KEY: &str = "merge.best";
/// Space between tiles.
pub const GAP: i32 = 8;
/// A new tile grows in over this long.
pub const POP_MS: u64 = 120;

/// One colour per tile, 2 to 2048, from the NES palette; bigger tiles are white.
const TILE_COLORS: [Rgb; 11] = [
    (0, 88, 248),
    (0, 168, 0),
    (216, 40, 0),
    (136, 20, 176),
    (0, 136, 136),
    (228, 92, 16),
    (168, 0, 32),
    (88, 148, 0),
    (248, 120, 88),
    (60, 188, 252),
    (248, 184, 0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    New,
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
    Playing,
    /// The 2048 tile is on the board; any input goes on playing.
    Goal,
    /// No move is left.
    Over,
}

/// Where things go on screen, for one orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: (i32, i32),
    pub board: Rect,
    pub title_pos: (i32, i32),
    /// SCORE and BEST.
    pub stat_boxes: [Rect; 2],
    pub buttons: Vec<Button>,
    pub help: &'static [&'static str],
    pub help_pos: (i32, i32),
}

impl Layout {
    pub fn tile(&self) -> i32 {
        (self.board.w - GAP * (SIZE as i32 + 1)) / SIZE as i32
    }

    pub fn tile_rect(&self, row: usize, column: usize) -> Rect {
        let tile = self.tile();
        Rect::new(
            self.board.x + GAP + column as i32 * (tile + GAP),
            self.board.y + GAP + row as i32 * (tile + GAP),
            tile,
            tile,
        )
    }
}

pub fn landscape() -> Layout {
    Layout {
        size: (480, 360),
        board: Rect::new(16, 16, 328, 328),
        title_pos: (414, 30),
        stat_boxes: [Rect::new(358, 64, 112, 44), Rect::new(358, 124, 112, 44)],
        buttons: vec![Button::new("new", "NEW", Rect::new(358, 300, 112, 40))],
        help: &["ARROWS: SLIDE", "SWIPE: SLIDE", "N: NEW GAME", "GOAL: 2048"],
        help_pos: (362, 188),
    }
}

pub fn portrait() -> Layout {
    Layout {
        size: (360, 640),
        board: Rect::new(12, 80, 336, 336),
        title_pos: (70, 34),
        stat_boxes: [Rect::new(150, 12, 95, 44), Rect::new(255, 12, 95, 44)],
        buttons: vec![Button::new("new", "NEW", Rect::new(110, 444, 140, 52))],
        help: &[
            "SWIPE TO SLIDE THE TILES.",
            "EQUAL TILES MERGE.",
            "MAKE A 2048!",
        ],
        help_pos: (30, 530),
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

/// Tile colour for an exponent.
pub fn tile_color(exponent: u8) -> Rgb {
    TILE_COLORS
        .get(usize::from(exponent).wrapping_sub(1))
        .copied()
        .unwrap_or(WHITE)
}

/// Biggest pixel font scale that fits `text` in a tile `size` wide.
pub fn number_scale(text: &str, size: i32) -> i32 {
    (1..=4)
        .rev()
        .find(|&scale| pixelfont::text_size(text, scale).0 + 2 * scale <= size - 8)
        .unwrap_or(1)
}

/// A pointer pressed on the screen; one swipe per press.
#[derive(Clone, Copy, Debug)]
struct Press {
    id: PointerId,
    start: (i32, i32),
    used: bool,
}

pub struct Game {
    pub web: bool,
    pub puzzle: Puzzle,
    pub best: u32,
    pub phase: Phase,
    /// Played on past the goal.
    pub kept_going: bool,
    rng: fastrand::Rng,
    now_ms: u64,
    moved_ms: u64,
    presses: Vec<Press>,
}

impl Game {
    pub fn new(web: bool, seed: u64, best: u32) -> Self {
        let mut rng = fastrand::Rng::with_seed(seed);
        Self {
            web,
            puzzle: Puzzle::new(&mut rng),
            best,
            phase: Phase::Playing,
            kept_going: false,
            rng,
            now_ms: 0,
            moved_ms: 0,
            presses: Vec::new(),
        }
    }

    pub fn update(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    pub fn handle(&mut self, event: Event, layout: &Layout) -> Outcome {
        match event {
            Event::Key(Key::Up) => self.slide(Direction::Up),
            Event::Key(Key::Down) => self.slide(Direction::Down),
            Event::Key(Key::Left) => self.slide(Direction::Left),
            Event::Key(Key::Right) => self.slide(Direction::Right),
            Event::Key(Key::Enter) => {
                match self.phase {
                    Phase::Goal => self.keep_going(),
                    Phase::Over => self.new_game(),
                    Phase::Playing => {}
                }
                Outcome::Continue
            }
            Event::Key(Key::New) => {
                self.new_game();
                Outcome::Continue
            }
            Event::Key(Key::Escape) if !self.web => Outcome::Quit,
            Event::Key(Key::Escape) => Outcome::Continue,
            Event::Press { id, pos } => {
                self.presses.retain(|p| p.id != id);
                if id == PointerId::MouseRight {
                    return Outcome::Continue;
                }
                if button_at(&layout.buttons, pos) == Some("new") {
                    self.new_game();
                    return Outcome::Continue;
                }
                if self.phase == Phase::Goal {
                    self.keep_going();
                }
                self.presses.push(Press {
                    id,
                    start: pos,
                    used: false,
                });
                Outcome::Continue
            }
            Event::Move { id, pos } => {
                let found = self
                    .presses
                    .iter_mut()
                    .find(|p| p.id == id && !p.used)
                    .and_then(|press| {
                        let found = swipe(press.start, pos, SWIPE_DISTANCE);
                        press.used = found.is_some();
                        found
                    });
                match found {
                    Some(way) => self.slide(direction(way)),
                    None => Outcome::Continue,
                }
            }
            Event::Release { id } => {
                self.presses.retain(|p| p.id != id);
                Outcome::Continue
            }
        }
    }

    fn keep_going(&mut self) {
        self.phase = Phase::Playing;
        self.kept_going = true;
    }

    fn new_game(&mut self) {
        self.puzzle = Puzzle::new(&mut self.rng);
        self.phase = Phase::Playing;
        self.kept_going = false;
        self.moved_ms = self.now_ms;
    }

    fn slide(&mut self, direction: Direction) -> Outcome {
        match self.phase {
            Phase::Over => return Outcome::Continue,
            Phase::Goal => self.keep_going(),
            Phase::Playing => {}
        }
        if !self.puzzle.play(direction, &mut self.rng) {
            return Outcome::Continue;
        }
        self.moved_ms = self.now_ms;
        if self.puzzle.reached_goal() && !self.kept_going {
            self.phase = Phase::Goal;
        } else if self.puzzle.over() {
            self.phase = Phase::Over;
        }
        if self.puzzle.score > self.best {
            self.best = self.puzzle.score;
            return Outcome::Save;
        }
        Outcome::Continue
    }

    pub fn draw(&self, canvas: &mut Canvas, layout: &Layout) {
        canvas.fill(BLACK);
        title(canvas, "MERGE", layout.title_pos, GOLD, 3);
        let stats = [("SCORE", self.puzzle.score), ("BEST", self.best)];
        for (rect, (name, number)) in layout.stat_boxes.iter().zip(stats) {
            draw_box(canvas, *rect, Some(name));
            let (x, y) = center(*rect);
            let text = number.to_string();
            let scale = if text.len() > 6 { 1 } else { 2 };
            centered(canvas, &text, (x, y + 2), WHITE, scale);
        }
        for (i, line) in layout.help.iter().enumerate() {
            let (x, y) = layout.help_pos;
            label(canvas, line, (x, y + i as i32 * 16), DIM, 1);
        }
        for button in &layout.buttons {
            draw_button(canvas, button, WHITE);
        }
        self.draw_board(canvas, layout);
        self.draw_message(canvas, layout);
    }

    fn draw_board(&self, canvas: &mut Canvas, layout: &Layout) {
        let board = layout.board;
        draw_box(canvas, board, None);
        canvas.fill_rect(
            Rect::new(board.x + 4, board.y + 4, board.w - 8, board.h - 8),
            BACKGROUND,
        );
        let popping = self.now_ms < self.moved_ms + POP_MS;
        for row in 0..SIZE {
            for column in 0..SIZE {
                let rect = layout.tile_rect(row, column);
                let exponent = self.puzzle.board[row][column];
                if exponent == 0 {
                    canvas.fill_rect(rect, shade(BACKGROUND, 0.12));
                    continue;
                }
                let rect = if popping && self.puzzle.spawned == Some((row, column)) {
                    let inset = rect.w / 4;
                    Rect::new(
                        rect.x + inset,
                        rect.y + inset,
                        rect.w - 2 * inset,
                        rect.h - 2 * inset,
                    )
                } else {
                    rect
                };
                let color = tile_color(exponent);
                draw_tile(canvas, rect, color);
                let text = value(exponent).to_string();
                let scale = number_scale(&text, rect.w);
                let ink = if color == WHITE { BLACK } else { WHITE };
                let area = canvas.text(
                    &text,
                    center(rect),
                    ink,
                    scale,
                    Some(shade(color, -0.6)),
                    true,
                );
                debug_assert!(area.w <= rect.w, "{text} overflows its tile");
            }
        }
    }

    fn draw_message(&self, canvas: &mut Canvas, layout: &Layout) {
        let lines: [(&str, Rgb); 2] = match self.phase {
            Phase::Playing => return,
            Phase::Goal => [("YOU MADE 2048!", GOLD), ("TAP TO KEEP GOING", WHITE)],
            Phase::Over => [("NO MOVES LEFT", GOLD), ("TAP NEW TO RETRY", WHITE)],
        };
        let board = layout.board;
        let rect = Rect::new(board.x + 12, board.y + board.h / 2 - 34, board.w - 24, 68);
        draw_box(canvas, rect, None);
        let (x, y) = center(rect);
        centered(canvas, lines[0].0, (x, y - 12), lines[0].1, 2);
        centered(canvas, lines[1].0, (x, y + 14), lines[1].1, 2);
    }
}

#[cfg(test)]
mod tests;
