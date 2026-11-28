//! Minesweeper screens with no engine code: the difficulty menu and the
//! board. `main.rs` feeds it input events and the clock and shows what
//! [`Game::draw`] paints.

use mines_core::{Cell, Cover, DIFFICULTIES, Field, Status};
use pixelfont::Rect;
use sandbox_ui::widgets::{
    BACKGROUND, DIM, GOLD, GREEN, RED, WHITE, center, centered, contains, draw_box, draw_button,
    draw_tile, label, title,
};
use sandbox_ui::{BLACK, Canvas, PointerId, Rgb};
use touch::{Button, TAP_DISTANCE, button_at, long_press};

/// Where the best times are saved, as seconds per difficulty ("0" for none).
pub const STORAGE_KEY: &str = "minesweeper.best";
/// Biggest cell, so the easy board does not get huge.
const MAX_CELL: i32 = 32;
/// The timer stops here.
const MAX_SECONDS: u32 = 999;

const HIDDEN: Rgb = (112, 112, 140);
const OPEN: Rgb = (28, 28, 48);
const OPEN_LINE: Rgb = (52, 52, 80);
const NUMBER_COLORS: [Rgb; 8] = [
    (80, 140, 255),
    (0, 184, 0),
    (248, 56, 56),
    (160, 100, 255),
    (200, 76, 12),
    (0, 200, 200),
    (252, 252, 252),
    (160, 160, 160),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Space,
    Flag,
    New,
    Escape,
    One,
    Two,
    Three,
}

/// Input in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Key(Key),
    /// The mouse moved here (pressed or not).
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Quit,
    /// A new best time: save `Game::best_text`.
    Save,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Menu { selected: usize },
    Play,
}

/// Where things go on screen, for one orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: (i32, i32),
    /// The board is centred in here, cells as big as fit.
    pub board_area: Rect,
    pub title_pos: (i32, i32),
    /// MINES and TIME.
    pub stat_boxes: [Rect; 2],
    /// Centre of the won or lost message.
    pub message_pos: (i32, i32),
    pub buttons: Vec<Button>,
    pub help: &'static [&'static str],
    pub help_pos: (i32, i32),
    pub menu_title_y: i32,
    pub menu_buttons: [Rect; 3],
    pub menu_hint_y: i32,
}

pub fn landscape() -> Layout {
    Layout {
        size: (480, 360),
        board_area: Rect::new(8, 8, 344, 344),
        title_pos: (416, 24),
        stat_boxes: [Rect::new(362, 50, 110, 40), Rect::new(362, 102, 110, 40)],
        message_pos: (416, 160),
        buttons: vec![
            Button::new("mode", "DIG", Rect::new(362, 216, 110, 36)),
            Button::new("new", "NEW", Rect::new(362, 260, 110, 36)),
            Button::new("menu", "MENU", Rect::new(362, 304, 110, 36)),
        ],
        help: &["RIGHT/HOLD: FLAG", "N: NEW  F: FLAG"],
        help_pos: (364, 180),
        menu_title_y: 50,
        menu_buttons: std::array::from_fn(|i| Rect::new(100, 96 + i as i32 * 70, 280, 56)),
        menu_hint_y: 318,
    }
}

pub fn portrait() -> Layout {
    Layout {
        size: (360, 640),
        board_area: Rect::new(8, 62, 344, 440),
        title_pos: (60, 30),
        stat_boxes: [Rect::new(130, 10, 105, 40), Rect::new(245, 10, 105, 40)],
        message_pos: (180, 518),
        buttons: vec![
            Button::new("mode", "DIG", Rect::new(8, 540, 110, 56)),
            Button::new("new", "NEW", Rect::new(125, 540, 110, 56)),
            Button::new("menu", "MENU", Rect::new(242, 540, 110, 56)),
        ],
        help: &["TAP: DIG   HOLD: FLAG"],
        help_pos: (52, 612),
        menu_title_y: 100,
        menu_buttons: std::array::from_fn(|i| Rect::new(30, 180 + i as i32 * 100, 300, 80)),
        menu_hint_y: 520,
    }
}

/// Cell size and the board's top-left for `field` in `layout`.
pub fn board_geometry(layout: &Layout, field: &Field) -> (i32, (i32, i32)) {
    let area = layout.board_area;
    let cell = (area.w / field.columns)
        .min(area.h / field.rows)
        .min(MAX_CELL);
    let (w, h) = (cell * field.columns, cell * field.rows);
    (cell, (area.x + (area.w - w) / 2, area.y + (area.h - h) / 2))
}

pub fn board_rect(layout: &Layout, field: &Field) -> Rect {
    let (cell, (x, y)) = board_geometry(layout, field);
    Rect::new(x, y, cell * field.columns, cell * field.rows)
}

/// The cell under `pos`, if any.
pub fn cell_at(layout: &Layout, field: &Field, pos: (i32, i32)) -> Option<Cell> {
    let (cell, (x, y)) = board_geometry(layout, field);
    let found = ((pos.0 - x).div_euclid(cell), (pos.1 - y).div_euclid(cell));
    field.in_bounds(found).then_some(found)
}

fn cell_rect(layout: &Layout, field: &Field, (cx, cy): Cell) -> Rect {
    let (cell, (x, y)) = board_geometry(layout, field);
    Rect::new(x + cx * cell, y + cy * cell, cell, cell)
}

/// Best times as saved text, and back; missing or broken entries are 0.
pub fn best_text(bests: &[u32; 3]) -> String {
    format!("{} {} {}\n", bests[0], bests[1], bests[2])
}

pub fn parse_bests(text: Option<&str>) -> [u32; 3] {
    let mut bests = [0; 3];
    let numbers = text.unwrap_or("").split_whitespace();
    for (best, number) in bests.iter_mut().zip(numbers) {
        *best = number.parse().unwrap_or(0);
    }
    bests
}

/// A pointer pressed on the board.
#[derive(Clone, Copy, Debug)]
struct Press {
    id: PointerId,
    start: (i32, i32),
    start_ms: u64,
    pos: (i32, i32),
    /// Already acted on (a long press), so the release does nothing.
    done: bool,
}

pub struct Game {
    pub web: bool,
    pub screen: Screen,
    pub difficulty: usize,
    pub field: Field,
    /// Taps flag (and long presses dig) instead of the other way round.
    pub flag_mode: bool,
    /// The keyboard cursor, once the keyboard is used.
    pub cursor: Option<Cell>,
    /// Best time in seconds per difficulty (0 for none).
    pub bests: [u32; 3],
    pub new_best: bool,
    started_ms: Option<u64>,
    ended_ms: Option<u64>,
    presses: Vec<Press>,
    rng: fastrand::Rng,
    now_ms: u64,
}

impl Game {
    pub fn new(web: bool, seed: u64, bests: [u32; 3]) -> Self {
        Self {
            web,
            screen: Screen::Menu { selected: 0 },
            difficulty: 0,
            field: Field::new(&DIFFICULTIES[0]),
            flag_mode: false,
            cursor: None,
            bests,
            new_best: false,
            started_ms: None,
            ended_ms: None,
            presses: Vec::new(),
            rng: fastrand::Rng::with_seed(seed),
            now_ms: 0,
        }
    }

    pub fn best_text(&self) -> String {
        best_text(&self.bests)
    }

    /// Whole seconds on the clock.
    pub fn seconds(&self) -> u32 {
        let Some(start) = self.started_ms else {
            return 0;
        };
        let end = self.ended_ms.unwrap_or(self.now_ms);
        ((end.saturating_sub(start) / 1000) as u32).min(MAX_SECONDS)
    }

    /// Advance the clock; a finger held still long enough does the other
    /// action (flag, or dig in flag mode).
    pub fn update(&mut self, now_ms: u64, layout: &Layout) -> Outcome {
        self.now_ms = now_ms;
        let due: Vec<(i32, i32)> = self
            .presses
            .iter_mut()
            .filter(|p| !p.done && long_press(p.start, p.start_ms, p.pos, now_ms))
            .map(|p| {
                p.done = true;
                p.start
            })
            .collect();
        let mut outcome = Outcome::Continue;
        for pos in due {
            if let Some(cell) = cell_at(layout, &self.field, pos) {
                outcome = self.act(cell, !self.flag_mode);
            }
        }
        outcome
    }

    pub fn handle(&mut self, event: Event, layout: &Layout) -> Outcome {
        match self.screen {
            Screen::Menu { selected } => self.menu(event, layout, selected),
            Screen::Play => self.play(event, layout),
        }
    }

    fn menu(&mut self, event: Event, layout: &Layout, selected: usize) -> Outcome {
        let count = DIFFICULTIES.len();
        let at = |pos| layout.menu_buttons.iter().position(|&r| contains(r, pos));
        match event {
            Event::Key(Key::Up) => {
                self.screen = Screen::Menu {
                    selected: (selected + count - 1) % count,
                };
            }
            Event::Key(Key::Down) => {
                self.screen = Screen::Menu {
                    selected: (selected + 1) % count,
                };
            }
            Event::Key(Key::Enter | Key::Space) => self.start(selected),
            Event::Key(Key::One) => self.start(0),
            Event::Key(Key::Two) => self.start(1),
            Event::Key(Key::Three) => self.start(2),
            Event::Key(Key::Escape) if !self.web => return Outcome::Quit,
            Event::Hover(pos) => {
                if let Some(selected) = at(pos) {
                    self.screen = Screen::Menu { selected };
                }
            }
            Event::Press { id, pos } if id != PointerId::MouseRight => {
                if let Some(index) = at(pos) {
                    self.start(index);
                }
            }
            _ => {}
        }
        Outcome::Continue
    }

    fn start(&mut self, difficulty: usize) {
        self.difficulty = difficulty;
        self.screen = Screen::Play;
        self.new_game();
    }

    fn new_game(&mut self) {
        self.field = Field::new(&DIFFICULTIES[self.difficulty]);
        self.started_ms = None;
        self.ended_ms = None;
        self.new_best = false;
        self.presses.clear();
        if let Some((x, y)) = self.cursor {
            let (columns, rows) = (self.field.columns, self.field.rows);
            self.cursor = Some((x.min(columns - 1), y.min(rows - 1)));
        }
    }

    fn play(&mut self, event: Event, layout: &Layout) -> Outcome {
        match event {
            Event::Key(key) => return self.play_key(key),
            Event::Hover(_) => {}
            Event::Press { id, pos } => {
                self.presses.retain(|p| p.id != id);
                if let Some(action) = button_at(&layout.buttons, pos) {
                    self.button(action);
                    return Outcome::Continue;
                }
                if id == PointerId::MouseRight {
                    if let Some(cell) = cell_at(layout, &self.field, pos) {
                        return self.act(cell, true);
                    }
                    return Outcome::Continue;
                }
                self.presses.push(Press {
                    id,
                    start: pos,
                    start_ms: self.now_ms,
                    pos,
                    done: false,
                });
            }
            Event::Move { id, pos } => {
                if let Some(press) = self.presses.iter_mut().find(|p| p.id == id) {
                    press.pos = pos;
                }
            }
            Event::Release { id, pos } => {
                let Some(index) = self.presses.iter().position(|p| p.id == id) else {
                    return Outcome::Continue;
                };
                let press = self.presses.remove(index);
                let still = (pos.0 - press.start.0).abs() <= TAP_DISTANCE
                    && (pos.1 - press.start.1).abs() <= TAP_DISTANCE;
                if !press.done
                    && still
                    && let Some(cell) = cell_at(layout, &self.field, press.start)
                {
                    return self.act(cell, self.flag_mode);
                }
            }
        }
        Outcome::Continue
    }

    fn play_key(&mut self, key: Key) -> Outcome {
        let moves = |key| match key {
            Key::Up => Some((0, -1)),
            Key::Down => Some((0, 1)),
            Key::Left => Some((-1, 0)),
            Key::Right => Some((1, 0)),
            _ => None,
        };
        if let Some((dx, dy)) = moves(key) {
            let (x, y) = self
                .cursor
                .unwrap_or((self.field.columns / 2, self.field.rows / 2));
            let moved = (x + dx, y + dy);
            let keep = self.cursor.is_some() && !self.field.in_bounds(moved);
            self.cursor = Some(if self.cursor.is_none() || keep {
                (x, y)
            } else {
                moved
            });
            return Outcome::Continue;
        }
        match key {
            Key::Enter | Key::Space => {
                if self.field.status != Status::Playing {
                    self.new_game();
                } else if let Some(cell) = self.cursor {
                    return self.act(cell, false);
                } else {
                    self.cursor = Some((self.field.columns / 2, self.field.rows / 2));
                }
            }
            Key::Flag => {
                if let Some(cell) = self.cursor {
                    return self.act(cell, true);
                }
            }
            Key::New => self.new_game(),
            Key::Escape => {
                self.screen = Screen::Menu {
                    selected: self.difficulty,
                }
            }
            _ => {}
        }
        Outcome::Continue
    }

    fn button(&mut self, action: &str) {
        match action {
            "mode" => self.flag_mode = !self.flag_mode,
            "new" => self.new_game(),
            "menu" => {
                self.screen = Screen::Menu {
                    selected: self.difficulty,
                };
            }
            _ => {}
        }
    }

    /// Flag (`flag`) or open `cell`, start the clock on the first opening
    /// and stop it at the end.
    fn act(&mut self, cell: Cell, flag: bool) -> Outcome {
        if self.field.status != Status::Playing {
            return Outcome::Continue;
        }
        if flag {
            self.field.toggle_flag(cell);
            return Outcome::Continue;
        }
        self.field.open(cell, &mut self.rng);
        if self.started_ms.is_none() && self.field.laid {
            self.started_ms = Some(self.now_ms);
        }
        if self.field.status == Status::Playing {
            return Outcome::Continue;
        }
        self.ended_ms = Some(self.now_ms);
        let time = self.seconds().max(1);
        let best = &mut self.bests[self.difficulty];
        if self.field.status == Status::Won && (*best == 0 || time < *best) {
            *best = time;
            self.new_best = true;
            return Outcome::Save;
        }
        Outcome::Continue
    }

    pub fn draw(&self, canvas: &mut Canvas, layout: &Layout) {
        canvas.fill(BLACK);
        match self.screen {
            Screen::Menu { selected } => self.draw_menu(canvas, layout, selected),
            Screen::Play => self.draw_play(canvas, layout),
        }
    }

    fn draw_menu(&self, canvas: &mut Canvas, layout: &Layout, selected: usize) {
        let middle = layout.size.0 / 2;
        title(
            canvas,
            "MINESWEEPER",
            (middle, layout.menu_title_y),
            GOLD,
            4,
        );
        for (i, (rect, difficulty)) in layout.menu_buttons.iter().zip(DIFFICULTIES).enumerate() {
            draw_box(canvas, *rect, None);
            let color = if i == selected { GOLD } else { WHITE };
            let name = if i == selected {
                format!("> {} <", difficulty.name)
            } else {
                difficulty.name.to_owned()
            };
            let (x, y) = center(*rect);
            centered(canvas, &name, (x, y - 8), color, 2);
            let best = match self.bests[i] {
                0 => "-".to_owned(),
                seconds => format!("{seconds}S"),
            };
            let detail = format!(
                "{}X{}, {} MINES, BEST {best}",
                difficulty.columns, difficulty.rows, difficulty.mines
            );
            centered(canvas, &detail, (x, y + 12), DIM, 1);
        }
        let hint = if self.web {
            "TAP A BOARD TO PLAY"
        } else {
            "1, 2, 3 OR CLICK TO PLAY"
        };
        centered(canvas, hint, (middle, layout.menu_hint_y), DIM, 2);
    }

    fn draw_play(&self, canvas: &mut Canvas, layout: &Layout) {
        title(canvas, "MINES", layout.title_pos, GOLD, 3);
        let stats = [
            ("MINES", self.field.mines_left().to_string()),
            ("TIME", self.seconds().to_string()),
        ];
        for (rect, (name, value)) in layout.stat_boxes.iter().zip(stats) {
            draw_box(canvas, *rect, Some(name));
            let (x, y) = center(*rect);
            centered(canvas, &value, (x, y + 2), WHITE, 2);
        }
        let message = match self.field.status {
            Status::Playing => None,
            Status::Won if self.new_best => Some(("NEW BEST!", GREEN)),
            Status::Won => Some(("CLEARED!", GREEN)),
            Status::Lost => Some(("BOOM!", RED)),
        };
        if let Some((text, color)) = message {
            centered(canvas, text, layout.message_pos, color, 2);
        }
        for (i, line) in layout.help.iter().enumerate() {
            let (x, y) = layout.help_pos;
            label(canvas, line, (x, y + i as i32 * 14), DIM, 1);
        }
        for button in &layout.buttons {
            let shown = match button.action {
                "mode" if self.flag_mode => Button::new("mode", "FLAG", button.rect),
                _ => button.clone(),
            };
            let color = if button.action == "mode" && self.flag_mode {
                RED
            } else {
                WHITE
            };
            draw_button(canvas, &shown, color);
        }
        self.draw_board(canvas, layout);
    }

    fn draw_board(&self, canvas: &mut Canvas, layout: &Layout) {
        let field = &self.field;
        let board = board_rect(layout, field);
        canvas.fill_rect(board, BACKGROUND);
        let lost = field.status == Status::Lost;
        for cell in field.cells() {
            let rect = cell_rect(layout, field, cell);
            match field.cover(cell) {
                Cover::Hidden if lost && field.is_mine(cell) => {
                    draw_open(canvas, rect);
                    draw_mine(canvas, rect);
                }
                Cover::Hidden => draw_tile(canvas, rect, HIDDEN),
                Cover::Flagged => {
                    draw_tile(canvas, rect, HIDDEN);
                    draw_flag(canvas, rect);
                    if lost && !field.is_mine(cell) {
                        centered(canvas, "X", center(rect), RED, 1);
                    }
                }
                Cover::Open if field.is_mine(cell) => {
                    let exploded = Some(cell) == field.exploded;
                    canvas.fill_rect(rect, if exploded { RED } else { OPEN });
                    draw_mine(canvas, rect);
                }
                Cover::Open => {
                    draw_open(canvas, rect);
                    let count = field.count(cell);
                    if count > 0 {
                        let scale = if rect.w >= 21 { 2 } else { 1 };
                        let color = NUMBER_COLORS[usize::from(count) - 1];
                        let (x, y) = center(rect);
                        centered(canvas, &count.to_string(), (x + 1, y + 1), color, scale);
                    }
                }
            }
        }
        if let Some(cell) = self.cursor {
            canvas.outline(cell_rect(layout, field, cell), GOLD, 2);
        }
    }
}

fn draw_open(canvas: &mut Canvas, rect: Rect) {
    canvas.fill_rect(rect, OPEN_LINE);
    canvas.fill_rect(
        Rect::new(rect.x + 1, rect.y + 1, rect.w - 1, rect.h - 1),
        OPEN,
    );
}

/// A black ball with spikes and a glint.
fn draw_mine(canvas: &mut Canvas, rect: Rect) {
    let s = rect.w;
    let (cx, cy) = center(rect);
    let r = (s / 4).max(2);
    canvas.fill_rect(Rect::new(cx - r, cy - r, 2 * r, 2 * r), BLACK);
    canvas.fill_rect(Rect::new(cx - r - 2, cy - 1, 2 * r + 4, 2), BLACK);
    canvas.fill_rect(Rect::new(cx - 1, cy - r - 2, 2, 2 * r + 4), BLACK);
    canvas.fill_rect(Rect::new(cx - r + 1, cy - r + 1, 2, 2), WHITE);
}

/// A red flag on a black pole.
fn draw_flag(canvas: &mut Canvas, rect: Rect) {
    let s = rect.w;
    let thin = (s / 10).max(2);
    let (x, y) = (rect.x, rect.y);
    canvas.fill_rect(Rect::new(x + s / 5, y + s / 6, s * 2 / 5, s / 3), RED);
    canvas.fill_rect(Rect::new(x + s * 3 / 5, y + s / 6, thin, s * 3 / 5), BLACK);
    canvas.fill_rect(Rect::new(x + s / 4, y + s * 3 / 4, s / 2, thin), BLACK);
}

#[cfg(test)]
mod tests;
