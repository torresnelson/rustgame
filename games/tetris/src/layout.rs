//! Where things go on screen (port of the layout part of `tetris.py`).
//!
//! The game screen has a layout per orientation; the menu, high score and
//! name screens use a `WIDTH` x `HEIGHT` view centred on the canvas.

use pixelfont::Rect;
use tetris_core::{COLUMNS, ROWS};
use touch::Button;

pub const WIDTH: i32 = 400;
pub const HEIGHT: i32 = 350;

pub const MENU_ROW: i32 = 26;

/// Menu choices; a browser tab has nothing to quit to.
pub fn menu_options(web: bool) -> &'static [&'static str] {
    if web {
        &["PLAY", "HIGH SCORES"]
    } else {
        &["PLAY", "HIGH SCORES", "QUIT"]
    }
}

pub fn menu_box(web: bool) -> Rect {
    Rect::new(
        105,
        150,
        190,
        14 + menu_options(web).len() as i32 * MENU_ROW,
    )
}

/// Menu row under `pos` (view pixels).
pub fn menu_index_at((x, y): (i32, i32), web: bool) -> Option<usize> {
    let menu = menu_box(web);
    let index = (y - menu.y - 10).div_euclid(MENU_ROW);
    let inside = (menu.x..menu.x + menu.w).contains(&x) && (menu.y..menu.y + menu.h).contains(&y);
    (inside && (0..menu_options(web).len() as i32).contains(&index)).then_some(index as usize)
}

/// A help box: frame, title and lines.
pub type Help = (Rect, &'static str, &'static [&'static str]);

/// Where everything on the game screen goes, for one orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub name: &'static str,
    pub size: (i32, i32),
    pub block: i32,
    pub board: (i32, i32),
    pub title_y: i32,
    pub next_box: Rect,
    pub next_block: i32,
    /// TOP, SCORE, LEVEL, LINES.
    pub stat_boxes: [Rect; 4],
    pub help_boxes: Vec<Help>,
    pub buttons: Vec<Button>,
    pub hint_y: Option<i32>,
}

impl Layout {
    pub fn board_rect(&self) -> Rect {
        Rect::new(
            self.board.0,
            self.board.1,
            COLUMNS * self.block,
            ROWS * self.block,
        )
    }

    /// Top of the centred `WIDTH` x `HEIGHT` view in canvas pixels.
    pub fn view_offset(&self) -> i32 {
        (self.size.1 - HEIGHT).div_euclid(2)
    }
}

pub fn landscape() -> Layout {
    Layout {
        name: "landscape",
        size: (WIDTH, HEIGHT),
        block: 15,
        board: (125, 40),
        title_y: 17,
        next_box: Rect::new(290, 40, 100, 80),
        next_block: 12,
        stat_boxes: std::array::from_fn(|i| Rect::new(10, 40 + i as i32 * 52, 100, 40)),
        help_boxes: vec![
            (
                Rect::new(290, 144, 100, 104),
                "KEYS",
                &[
                    "<> MOVE",
                    "UP ROTATE",
                    "DN SOFT",
                    "SPC DROP",
                    "P  PAUSE",
                    "ESC END",
                ],
            ),
            (
                Rect::new(10, 260, 100, 76),
                "TOUCH",
                &["TAP  ROTATE", "DRAG MOVE", "DRAG DN SOFT", "FLICK DROP"],
            ),
        ],
        buttons: vec![
            Button::new("pause", "PAUSE", Rect::new(290, 262, 100, 32)),
            Button::new("end", "END", Rect::new(290, 302, 100, 32)),
        ],
        hint_y: None,
    }
}

pub fn portrait() -> Layout {
    Layout {
        name: "portrait",
        size: (WIDTH, 760),
        block: 22,
        board: (15, 44),
        title_y: 20,
        next_box: Rect::new(250, 44, 140, 90),
        next_block: 16,
        stat_boxes: std::array::from_fn(|i| Rect::new(250, 154 + i as i32 * 52, 140, 40)),
        help_boxes: Vec::new(),
        buttons: vec![
            Button::new("pause", "PAUSE", Rect::new(250, 370, 140, 48)),
            Button::new("end", "END", Rect::new(250, 430, 140, 48)),
            Button::new("left", "<", Rect::new(12, 502, 120, 120)),
            Button::new("rotate", "ROT", Rect::new(140, 502, 120, 120)),
            Button::new("right", ">", Rect::new(268, 502, 120, 120)),
            Button::new("down", "DOWN", Rect::new(12, 632, 184, 96)),
            Button::new("drop", "DROP", Rect::new(204, 632, 184, 96)),
        ],
        hint_y: Some(744),
    }
}

/// Held buttons keep moving the piece, like a held arrow key.
pub const REPEATING: [&str; 3] = ["left", "right", "down"];

pub const KEY_ROWS: [&str; 4] = ["ABCDEFGHIJ", "KLMNOPQRST", "UVWXYZ0123", "456789"];
pub const KEY_W: i32 = 34;
pub const KEY_H: i32 = 22;
pub const KEY_GAP: i32 = 4;
pub const KEYBOARD_X: i32 = 12;
pub const KEYBOARD_Y: i32 = 190;

/// One action per character, so a key's action is its character.
const KEY_CHARS: [&str; 36] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
];

/// On-screen keys (view pixels): one per character, then wide DEL and OK keys
/// on the last row.
pub fn keyboard() -> Vec<Button> {
    let mut keys = Vec::new();
    let mut chars = KEY_CHARS.iter();
    let mut y = KEYBOARD_Y;
    for (r, row) in KEY_ROWS.iter().enumerate() {
        y = KEYBOARD_Y + r as i32 * (KEY_H + KEY_GAP);
        for c in 0..row.len() as i32 {
            let char = chars.next().expect("one action per key");
            keys.push(Button::new(
                char,
                char,
                Rect::new(KEYBOARD_X + c * (KEY_W + KEY_GAP), y, KEY_W, KEY_H),
            ));
        }
    }
    let wide = 2 * KEY_W + KEY_GAP;
    let x = KEYBOARD_X + KEY_ROWS[3].len() as i32 * (KEY_W + KEY_GAP);
    keys.push(Button::new("DEL", "DEL", Rect::new(x, y, wide, KEY_H)));
    keys.push(Button::new(
        "OK",
        "OK",
        Rect::new(x + wide + KEY_GAP, y, wide, KEY_H),
    ));
    keys
}
