//! Tetris screens as a state machine (the game loop of `tetris.py`): menu,
//! play, high scores and name entry.
//!
//! `main.rs` feeds it input events and the clock and shows what [`Game::draw`]
//! paints. The Python version's blocking waits (the line-clear flash, the
//! GAME OVER pause) are timed states here, so the app keeps running.

use std::collections::BTreeSet;

use pixelfont::Rect;
use sandbox_ui::{BLACK, Canvas, PointerId, Rgb};
use tetris_core::highscores::{self, DEFAULT_NAME, Entry, LIMIT, NAME_LENGTH};
use tetris_core::{
    COLUMNS, EMPTY, Grid, Locked, Piece, ROWS, SHAPE_COLORS, check_lost, clear_rows, create_grid,
    fall_interval, frame_positions, full_rows, hard_drop, level_for_lines, lock_piece, new_piece,
    piece_positions, score_for_lines, try_move,
};
use touch::{
    Button, Drag, Gesture, REPEAT_DELAY_MS, REPEAT_MS, button_at, drag_steps, release_gesture,
    repeats, start_drag,
};

use crate::draw::{
    BOARD_BG, DIM, GOLD, GREY, GRID_DOT, Paint, RANK_COLORS, View, WHITE, draw_box, draw_button,
    draw_tile, draw_title, shade,
};
use crate::layout::{
    HEIGHT, Layout, MENU_ROW, REPEATING, WIDTH, keyboard, menu_box, menu_index_at, menu_options,
};

/// Completed rows blink this many times, this long each, before they clear.
pub const FLASHES: u64 = 6;
pub const FLASH_MS: u64 = 55;
/// How long GAME OVER stays on screen.
pub const GAME_OVER_MS: u64 = 1500;
/// Longest step the clock may take, so a hidden browser tab does not come
/// back to a burst of falls.
const MAX_STEP_MS: u64 = 100;
/// Menu blocks fall `speed` pixels per frame at 60 frames a second.
const FRAME_MS: f32 = 1000.0 / 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Escape,
    Pause,
    Backspace,
    /// Any other key (the high score table closes on any key).
    Other,
}

/// Input in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Key(Key),
    /// A typed character (for the name).
    Char(char),
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
    /// The high score table changed: save `Game::entries`.
    Save,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameResult {
    pub score: u32,
    pub lines: u32,
    pub level: u32,
}

/// A falling block behind the menu.
#[derive(Clone, Debug, PartialEq)]
pub struct Drop {
    pub x: i32,
    pub y: f32,
    pub speed: f32,
    pub color: Rgb,
}

/// A repeating button held down by one pointer.
#[derive(Clone, Copy, Debug)]
struct Held {
    id: PointerId,
    action: &'static str,
    since: u64,
    fired: u64,
}

/// One game in progress.
#[derive(Clone, Debug)]
pub struct Play {
    pub locked: Locked,
    pub current: Piece,
    pub next: Piece,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    /// Best score in the table when the game started.
    pub top: u32,
    pub paused: bool,
    fall_ms: u64,
    drag: Option<(PointerId, Drag)>,
    held: Option<Held>,
    /// Completed rows, blinking since the given time before they clear.
    pub flash: Option<(BTreeSet<i32>, u64)>,
    /// When the game ended; GAME OVER shows for `GAME_OVER_MS`.
    pub over: Option<u64>,
}

#[derive(Clone, Debug)]
pub enum Screen {
    Menu { selected: usize, drops: Vec<Drop> },
    HighScores { highlight: Option<usize> },
    Play(Box<Play>),
    Name { result: GameResult, name: String },
}

pub struct Game {
    pub web: bool,
    pub entries: Vec<Entry>,
    pub screen: Screen,
    /// The name typed last time, offered again.
    pub last_name: String,
    /// Date stamped on new high scores (`YYYY-MM-DD`).
    pub day: String,
    keyboard: Vec<Button>,
    rng: fastrand::Rng,
    now_ms: u64,
    last_ms: Option<u64>,
}

// -- small pure helpers -------------------------------------------------------

/// Append `char` (upper-cased) if the pixel font can draw it and there is room.
pub fn accept_char(name: &str, char: char) -> String {
    let upper = char.to_ascii_uppercase();
    let mut name = name.to_owned();
    if name.chars().count() < NAME_LENGTH && pixelfont::charset().any(|c| c == upper) {
        name.push(upper);
    }
    name
}

/// Apply an on-screen key (other than OK) to `name`.
pub fn press_key(name: &str, action: &str) -> String {
    if action == "DEL" {
        let mut name = name.to_owned();
        name.pop();
        return name;
    }
    let mut chars = action.chars();
    match (chars.next(), chars.next()) {
        (Some(char), None) => accept_char(name, char),
        _ => name.to_owned(),
    }
}

/// Apply a move (`left`, `right`, `rotate`, `down`, `drop`); true if the
/// piece must lock.
pub fn apply_move(piece: &mut Piece, grid: &Grid, action: &str) -> bool {
    let (dx, dy, rotate) = match action {
        "left" => (-1, 0, 0),
        "right" => (1, 0, 0),
        "rotate" => (0, 0, 1),
        "down" => (0, 1, 0),
        "drop" => {
            hard_drop(piece, grid);
            return true;
        }
        _ => return false,
    };
    try_move(piece, grid, dx, dy, rotate);
    false
}

/// Move `piece` one cell at a time so it stops at the first obstacle.
fn shift(piece: &mut Piece, grid: &Grid, columns: i32, rows: i32) {
    for _ in 0..columns.abs() {
        try_move(piece, grid, columns.signum(), 0, 0);
    }
    for _ in 0..rows {
        try_move(piece, grid, 0, 1, 0);
    }
}

pub fn blink(now_ms: u64, period_ms: u64) -> bool {
    (now_ms / period_ms).is_multiple_of(2)
}

pub fn cycle_color(now_ms: u64) -> Rgb {
    SHAPE_COLORS[(now_ms / 120) as usize % SHAPE_COLORS.len()]
}

/// A falling block above the top, or anywhere in the first `height` pixels.
pub fn new_drop(rng: &mut fastrand::Rng, height: i32) -> Drop {
    let y = if height > 0 {
        rng.f32() * height as f32
    } else {
        -10.0
    };
    Drop {
        x: rng.i32(0..WIDTH / 10) * 10,
        y,
        speed: 0.3 + rng.f32() * 0.8,
        color: SHAPE_COLORS[rng.usize(..SHAPE_COLORS.len())],
    }
}

fn drops_for(rng: &mut fastrand::Rng, layout: &Layout) -> Vec<Drop> {
    let height = layout.size.1;
    (0..28 * height / HEIGHT)
        .map(|_| new_drop(rng, height))
        .collect()
}

// -- play ---------------------------------------------------------------------

impl Play {
    pub fn new(rng: &mut fastrand::Rng, top: u32) -> Self {
        let current = new_piece(rng);
        let next = new_piece(rng);
        Self {
            locked: Locked::new(),
            current,
            next,
            score: 0,
            lines: 0,
            level: 0,
            top,
            paused: false,
            fall_ms: 0,
            drag: None,
            held: None,
            flash: None,
            over: None,
        }
    }

    pub fn result(&self) -> GameResult {
        GameResult {
            score: self.score,
            lines: self.lines,
            level: self.level,
        }
    }

    /// Input and gravity stop while rows flash and once the game is over.
    fn frozen(&self) -> bool {
        self.flash.is_some() || self.over.is_some()
    }

    fn act(&mut self, action: &str, now: u64, rng: &mut fastrand::Rng) {
        match action {
            "pause" => self.paused = !self.paused,
            "end" => self.over = Some(now),
            _ if !self.paused => {
                let grid = create_grid(&self.locked);
                if apply_move(&mut self.current, &grid, action) {
                    self.lock(now, rng);
                }
            }
            _ => {}
        }
    }

    fn lock(&mut self, now: u64, rng: &mut fastrand::Rng) {
        self.locked = lock_piece(&self.current, &self.locked);
        let completed = full_rows(&self.locked);
        if completed.is_empty() {
            self.finish_lock(now, rng);
        } else {
            self.flash = Some((completed, now));
        }
    }

    /// Clear full rows, score them and bring in the next piece.
    fn finish_lock(&mut self, now: u64, rng: &mut fastrand::Rng) {
        let (locked, cleared) = clear_rows(&self.locked);
        self.locked = locked;
        self.score += score_for_lines(cleared, self.level);
        self.lines += cleared;
        self.level = level_for_lines(self.lines);
        self.current = std::mem::replace(&mut self.next, new_piece(rng));
        if check_lost(&self.locked) {
            self.over = Some(now);
        }
    }

    fn key(&mut self, key: Key, now: u64, rng: &mut fastrand::Rng) {
        let action = match key {
            Key::Pause => "pause",
            Key::Escape => "end",
            Key::Left => "left",
            Key::Right => "right",
            Key::Up => "rotate",
            Key::Down => "down",
            Key::Space => "drop",
            _ => return,
        };
        if !self.frozen() {
            self.act(action, now, rng);
        }
    }

    fn press(
        &mut self,
        id: PointerId,
        pos: (i32, i32),
        layout: &Layout,
        now: u64,
        rng: &mut fastrand::Rng,
    ) {
        if self.frozen() {
            return;
        }
        match button_at(&layout.buttons, pos) {
            None => self.drag = Some((id, start_drag(pos, now))),
            Some(action) => {
                if REPEATING.contains(&action) {
                    self.held = Some(Held {
                        id,
                        action,
                        since: now,
                        fired: 0,
                    });
                }
                self.act(action, now, rng);
            }
        }
    }

    fn move_to(&mut self, id: PointerId, pos: (i32, i32), layout: &Layout) {
        if self.frozen() {
            return;
        }
        if let Some((drag_id, drag)) = &mut self.drag
            && *drag_id == id
        {
            let (columns, rows) = drag_steps(drag, pos, layout.block);
            if !self.paused {
                let grid = create_grid(&self.locked);
                shift(&mut self.current, &grid, columns, rows);
            }
        }
    }

    fn release(&mut self, id: PointerId, pos: (i32, i32), now: u64, rng: &mut fastrand::Rng) {
        if self.held.is_some_and(|held| held.id == id) {
            self.held = None;
        }
        let Some((_, drag)) = self.drag.take_if(|(drag_id, _)| *drag_id == id) else {
            return;
        };
        if self.frozen() {
            return;
        }
        let gesture = release_gesture(&drag, pos, now);
        if self.paused {
            // A tap on the board resumes.
            self.paused = gesture != Some(Gesture::Tap);
            return;
        }
        match gesture {
            Some(Gesture::Tap) => self.act("rotate", now, rng),
            Some(Gesture::FlickDown) => self.act("drop", now, rng),
            None => {}
        }
    }

    /// Repeats the held button has come due for since the last call.
    fn take_repeats(&mut self, now: u64) -> u64 {
        let Some(held) = &mut self.held else {
            return 0;
        };
        let due = repeats(now.saturating_sub(held.since), REPEAT_DELAY_MS, REPEAT_MS);
        let new = due.saturating_sub(held.fired);
        held.fired = due;
        new
    }

    /// Advance the clock; the result once GAME OVER has been shown.
    fn update(&mut self, now: u64, elapsed: u64, rng: &mut fastrand::Rng) -> Option<GameResult> {
        // Taken first so repeats due while paused or flashing are dropped,
        // not fired all at once afterwards.
        let repeats = self.take_repeats(now);
        if let Some(since) = self.over {
            return (now.saturating_sub(since) >= GAME_OVER_MS).then(|| self.result());
        }
        if let Some((_, since)) = self.flash {
            if now.saturating_sub(since) >= FLASHES * FLASH_MS {
                self.flash = None;
                self.finish_lock(now, rng);
            }
            return None;
        }
        if self.paused {
            return None;
        }
        if let Some(held) = self.held {
            let grid = create_grid(&self.locked);
            for _ in 0..repeats {
                apply_move(&mut self.current, &grid, held.action);
            }
        }
        self.fall_ms += elapsed;
        if self.fall_ms as f32 / 1000.0 >= fall_interval(self.level) {
            self.fall_ms = 0;
            let grid = create_grid(&self.locked);
            if !try_move(&mut self.current, &grid, 0, 1, 0) && self.current.y > 0 {
                self.lock(now, rng);
            }
        }
        None
    }

    fn draw(&self, canvas: &mut Canvas, layout: &Layout, web: bool, now: u64) {
        let mut grid = create_grid(&self.locked);
        if self.flash.is_none() {
            for (x, y) in piece_positions(&self.current) {
                if (0..COLUMNS).contains(&x) && (0..ROWS).contains(&y) {
                    grid[y as usize][x as usize] = self.current.color;
                }
            }
        }
        let flashing = self
            .flash
            .as_ref()
            .filter(|(_, since)| blink(now.saturating_sub(*since), FLASH_MS))
            .map(|(rows, _)| rows);
        draw_game(canvas, layout, &grid, self, flashing);
        if self.over.is_some() {
            draw_overlay(
                canvas,
                layout,
                "GAME OVER",
                &format!("{} POINTS", self.score),
            );
        } else if self.paused {
            let hint = if web {
                "TAP RESUME"
            } else {
                "P RESUME  ESC END"
            };
            draw_overlay(canvas, layout, "PAUSED", hint);
        }
    }
}

// -- the whole game -----------------------------------------------------------

impl Game {
    pub fn new(web: bool, seed: u64, entries: Vec<Entry>, day: String, layout: &Layout) -> Self {
        let mut rng = fastrand::Rng::with_seed(seed);
        let drops = drops_for(&mut rng, layout);
        Self {
            web,
            entries,
            screen: Screen::Menu { selected: 0, drops },
            last_name: DEFAULT_NAME.to_owned(),
            day,
            keyboard: keyboard(),
            rng,
            now_ms: 0,
            last_ms: None,
        }
    }

    fn back_to_menu(&mut self, layout: &Layout) {
        let drops = drops_for(&mut self.rng, layout);
        self.screen = Screen::Menu { selected: 0, drops };
    }

    fn choose(&mut self, option: &str) -> Outcome {
        match option {
            "PLAY" => {
                let play = Play::new(&mut self.rng, highscores::best(&self.entries));
                self.screen = Screen::Play(Box::new(play));
            }
            "HIGH SCORES" => self.screen = Screen::HighScores { highlight: None },
            _ => return Outcome::Quit,
        }
        Outcome::Continue
    }

    fn submit(&mut self, name: String, result: GameResult) -> Outcome {
        let entry =
            highscores::new_entry(&name, result.score, result.lines, result.level, &self.day);
        let (entries, rank) = highscores::insert(&self.entries, entry, LIMIT);
        self.entries = entries;
        self.last_name = name;
        self.screen = Screen::HighScores { highlight: rank };
        Outcome::Save
    }

    /// The name screen for a qualifying score, else the table.
    fn finish(&mut self, result: GameResult) {
        self.screen = if highscores::qualifies(&self.entries, result.score, LIMIT) {
            Screen::Name {
                result,
                name: self.last_name.to_uppercase(),
            }
        } else {
            Screen::HighScores { highlight: None }
        };
    }

    pub fn handle(&mut self, event: Event, layout: &Layout) -> Outcome {
        let now = self.now_ms;
        let offset = layout.view_offset();
        let view = |(x, y): (i32, i32)| (x, y - offset);
        match &mut self.screen {
            Screen::Menu { selected, .. } => {
                let options = menu_options(self.web);
                let chosen = match event {
                    Event::Key(Key::Up) => {
                        *selected = (*selected + options.len() - 1) % options.len();
                        None
                    }
                    Event::Key(Key::Down) => {
                        *selected = (*selected + 1) % options.len();
                        None
                    }
                    Event::Key(Key::Enter | Key::Space) => Some(*selected),
                    // A browser tab has nothing to quit to.
                    Event::Key(Key::Escape) if !self.web => return Outcome::Quit,
                    Event::Hover(pos) => {
                        if let Some(index) = menu_index_at(view(pos), self.web) {
                            *selected = index;
                        }
                        None
                    }
                    Event::Press { pos, .. } => menu_index_at(view(pos), self.web),
                    _ => None,
                };
                if let Some(index) = chosen {
                    return self.choose(options[index]);
                }
            }
            Screen::HighScores { .. } => {
                if matches!(event, Event::Key(_) | Event::Press { .. }) {
                    self.back_to_menu(layout);
                }
            }
            Screen::Name { result, name } => {
                let result = *result;
                let submitted = match event {
                    Event::Press { pos, .. } => match button_at(&self.keyboard, view(pos)) {
                        Some("OK") => Some(name.clone()),
                        Some(action) => {
                            *name = press_key(name, action);
                            None
                        }
                        None => None,
                    },
                    Event::Key(Key::Enter) => Some(name.clone()),
                    Event::Key(Key::Escape) => Some(self.last_name.clone()),
                    Event::Key(Key::Backspace) => {
                        name.pop();
                        None
                    }
                    Event::Char(char) => {
                        *name = accept_char(name, char);
                        None
                    }
                    _ => None,
                };
                if let Some(name) = submitted {
                    return self.submit(name, result);
                }
            }
            Screen::Play(play) => match event {
                Event::Key(key) => play.key(key, now, &mut self.rng),
                Event::Press { id, pos } => play.press(id, pos, layout, now, &mut self.rng),
                Event::Move { id, pos } => play.move_to(id, pos, layout),
                Event::Release { id, pos } => play.release(id, pos, now, &mut self.rng),
                Event::Char(_) | Event::Hover(_) => {}
            },
        }
        Outcome::Continue
    }

    /// Advance to `now_ms` (milliseconds since start).
    pub fn update(&mut self, now_ms: u64, layout: &Layout) {
        let elapsed = self
            .last_ms
            .map_or(0, |last| now_ms.saturating_sub(last).min(MAX_STEP_MS));
        self.last_ms = Some(now_ms);
        self.now_ms = now_ms;
        match &mut self.screen {
            Screen::Menu { drops, .. } => {
                let frames = elapsed as f32 / FRAME_MS;
                for drop in drops.iter_mut() {
                    drop.y += drop.speed * frames;
                    if drop.y > layout.size.1 as f32 {
                        *drop = new_drop(&mut self.rng, 0);
                    }
                }
            }
            Screen::Play(play) => {
                if let Some(result) = play.update(now_ms, elapsed, &mut self.rng) {
                    self.finish(result);
                }
            }
            Screen::HighScores { .. } | Screen::Name { .. } => {}
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, layout: &Layout) {
        canvas.fill(BLACK);
        let now = self.now_ms;
        let dy = layout.view_offset();
        match &self.screen {
            Screen::Menu { selected, drops } => {
                for drop in drops {
                    draw_tile(canvas, shade(drop.color, -0.6), drop.x, drop.y as i32, 10);
                }
                draw_menu(
                    &mut View { canvas, dy },
                    *selected,
                    &self.entries,
                    self.web,
                    now,
                );
            }
            Screen::HighScores { highlight } => draw_highscores(
                &mut View { canvas, dy },
                &self.entries,
                *highlight,
                self.web,
                now,
            ),
            Screen::Name { result, name } => draw_name_entry(
                &mut View { canvas, dy },
                &self.keyboard,
                result,
                name,
                self.web,
                now,
            ),
            Screen::Play(play) => play.draw(canvas, layout, self.web, now),
        }
    }
}

// -- drawing ------------------------------------------------------------------

fn draw_menu(view: &mut impl Paint, selected: usize, entries: &[Entry], web: bool, now: u64) {
    draw_title(view, 70, 6);
    let top_line = entries.first().map_or_else(
        || "NO HIGH SCORES YET".to_owned(),
        |top| format!("TOP {}  {}", top.score, top.name),
    );
    view.text(&top_line, (WIDTH / 2, 122), GOLD, 1, None, true);

    let menu = menu_box(web);
    draw_box(view, menu, None);
    for (i, option) in menu_options(web).iter().enumerate() {
        let y = menu.y + 14 + i as i32 * MENU_ROW;
        let color = if i == selected { WHITE } else { GREY };
        view.text(option, (menu.x + 36, y), color, 2, None, false);
        if i == selected && blink(now, 400) {
            view.text(">", (menu.x + 14, y), GOLD, 2, None, false);
        }
    }
    if blink(now, 500) {
        let prompt = if web { "TAP TO CHOOSE" } else { "PRESS ENTER" };
        view.text(prompt, (WIDTH / 2, 278), WHITE, 2, None, true);
    }
    if !web {
        view.text(
            "UP/DOWN + ENTER OR MOUSE",
            (WIDTH / 2, 330),
            DIM,
            1,
            None,
            true,
        );
    }
}

const SCORE_COLUMNS: [(i32, &str); 5] = [
    (16, "RANK"),
    (50, "NAME"),
    (178, "SCORE"),
    (284, "LINES"),
    (350, "LV"),
];

fn draw_highscores(
    view: &mut impl Paint,
    entries: &[Entry],
    highlight: Option<usize>,
    web: bool,
    now: u64,
) {
    view.text(
        "HIGH SCORES",
        (WIDTH / 2, 22),
        GOLD,
        3,
        Some(shade(GOLD, -0.7)),
        true,
    );
    draw_box(view, Rect::new(8, 46, WIDTH - 16, 268), None);
    for (x, title) in SCORE_COLUMNS {
        view.text(title, (x, 56), DIM, 1, None, false);
    }
    if entries.is_empty() {
        view.text("NO SCORES YET", (WIDTH / 2, 170), WHITE, 2, None, true);
    }
    for (i, entry) in entries.iter().enumerate() {
        let y = 72 + i as i32 * 24;
        let color = if highlight == Some(i) {
            cycle_color(now)
        } else {
            RANK_COLORS.get(i).copied().unwrap_or(WHITE)
        };
        let values = [
            (i + 1).to_string(),
            entry.name.clone(),
            entry.score.to_string(),
            entry.lines.to_string(),
            entry.level.to_string(),
        ];
        for ((x, _), value) in SCORE_COLUMNS.iter().zip(values) {
            view.text(&value, (*x, y), color, 2, None, false);
        }
    }
    if blink(now, 500) {
        let prompt = if web {
            "TAP TO CONTINUE"
        } else {
            "PRESS ANY KEY"
        };
        view.text(prompt, (WIDTH / 2, 332), WHITE, 1, None, true);
    }
}

fn draw_name_entry(
    view: &mut impl Paint,
    keyboard: &[Button],
    result: &GameResult,
    name: &str,
    web: bool,
    now: u64,
) {
    view.text(
        "NEW HIGH SCORE!",
        (WIDTH / 2, 36),
        cycle_color(now),
        3,
        Some(BLACK),
        true,
    );
    let score = result.score.to_string();
    view.text(
        &score,
        (WIDTH / 2, 78),
        GOLD,
        4,
        Some(shade(GOLD, -0.7)),
        true,
    );
    let details = format!("{} LINES  LEVEL {}", result.lines, result.level);
    view.text(&details, (WIDTH / 2, 104), DIM, 1, None, true);
    view.text("ENTER YOUR NAME", (WIDTH / 2, 126), WHITE, 2, None, true);
    let field = Rect::new(80, 140, 240, 36);
    draw_box(view, field, None);
    let text = view.text(name, (field.x + 14, field.y + 11), WHITE, 2, None, false);
    if blink(now, 300) && name.chars().count() < NAME_LENGTH {
        let x = if name.is_empty() {
            field.x + 14
        } else {
            text.x + text.w + 2
        };
        view.fill_rect(Rect::new(x, field.y + 11, 10, 14), GOLD);
    }
    for key in keyboard {
        let color = if key.action == "OK" { GOLD } else { WHITE };
        draw_button(view, key.rect, key.label, color);
    }
    let hint = if web {
        "TAP OK TO SAVE"
    } else {
        "TYPE OR CLICK, ENTER TO SAVE"
    };
    view.text(hint, (WIDTH / 2, 310), DIM, 1, None, true);
}

fn draw_board(canvas: &mut Canvas, layout: &Layout, grid: &Grid, flash: Option<&BTreeSet<i32>>) {
    let board = layout.board_rect();
    let block = layout.block;
    // Frame made of small grey tiles.
    for x in (board.x - 5..board.x + board.w + 5).step_by(5) {
        draw_tile(canvas, GREY, x, board.y - 5, 5);
        draw_tile(canvas, GREY, x, board.y + board.h, 5);
    }
    for y in (board.y..board.y + board.h).step_by(5) {
        draw_tile(canvas, GREY, board.x - 5, y, 5);
        draw_tile(canvas, GREY, board.x + board.w, y, 5);
    }
    canvas.fill_rect(board, BOARD_BG);
    for (y, row) in grid.iter().enumerate() {
        for (x, &color) in row.iter().enumerate() {
            let (px, py) = (board.x + x as i32 * block, board.y + y as i32 * block);
            if flash.is_some_and(|rows| rows.contains(&(y as i32))) {
                draw_tile(canvas, WHITE, px, py, block);
            } else if color != EMPTY {
                draw_tile(canvas, color, px, py, block);
            } else {
                canvas.set(px + block / 2, py + block / 2, GRID_DOT);
            }
        }
    }
}

fn draw_stats(canvas: &mut Canvas, layout: &Layout, play: &Play) {
    let rows = [
        ("TOP", play.top.max(play.score), GOLD),
        ("SCORE", play.score, WHITE),
        ("LEVEL", play.level, WHITE),
        ("LINES", play.lines, WHITE),
    ];
    for (rect, (label, value, color)) in layout.stat_boxes.iter().zip(rows) {
        draw_box(canvas, *rect, Some(label));
        canvas.text(
            &value.to_string(),
            (rect.x + 10, rect.y + 14),
            color,
            2,
            None,
            false,
        );
    }
}

fn draw_next(canvas: &mut Canvas, layout: &Layout, piece: &Piece) {
    let rect = layout.next_box;
    draw_box(canvas, rect, Some("NEXT"));
    let cells = frame_positions(piece.frame(), 0, 0);
    let size = layout.next_block;
    let min_x = cells.iter().map(|c| c.0).min().unwrap_or(0);
    let max_x = cells.iter().map(|c| c.0).max().unwrap_or(0);
    let min_y = cells.iter().map(|c| c.1).min().unwrap_or(0);
    let max_y = cells.iter().map(|c| c.1).max().unwrap_or(0);
    let left = rect.x + rect.w / 2 - ((max_x - min_x + 1) * size).div_euclid(2);
    let top = rect.y + rect.h / 2 - ((max_y - min_y + 1) * size).div_euclid(2) + 2;
    for (x, y) in cells {
        let (px, py) = (left + (x - min_x) * size, top + (y - min_y) * size);
        draw_tile(canvas, piece.color, px, py, size);
    }
}

fn draw_game(
    canvas: &mut Canvas,
    layout: &Layout,
    grid: &Grid,
    play: &Play,
    flash: Option<&BTreeSet<i32>>,
) {
    draw_title(canvas, layout.title_y, 2);
    draw_board(canvas, layout, grid, flash);
    draw_stats(canvas, layout, play);
    draw_next(canvas, layout, &play.next);
    for &(rect, title, lines) in &layout.help_boxes {
        draw_box(canvas, rect, Some(title));
        for (i, line) in lines.iter().enumerate() {
            let pos = (rect.x + 10, rect.y + 12 + i as i32 * 14);
            canvas.text(line, pos, DIM, 1, None, false);
        }
    }
    for button in &layout.buttons {
        let label = if play.paused && button.action == "pause" {
            "RESUME"
        } else {
            button.label
        };
        draw_button(canvas, button.rect, label, WHITE);
    }
    if let Some(y) = layout.hint_y {
        let hint = "OR TAP, DRAG AND FLICK THE BOARD";
        canvas.text(hint, (WIDTH / 2, y), DIM, 1, None, true);
    }
}

fn draw_overlay(canvas: &mut Canvas, layout: &Layout, title: &str, subtitle: &str) {
    let board = layout.board_rect();
    let rect = Rect::new(board.x + 5, board.y + board.h / 2 - 30, board.w - 10, 60);
    draw_box(canvas, rect, None);
    let center_x = rect.x + rect.w / 2;
    canvas.text(title, (center_x, rect.y + 20), WHITE, 2, None, true);
    canvas.text(subtitle, (center_x, rect.y + 42), DIM, 1, None, true);
}

#[cfg(test)]
mod tests;
