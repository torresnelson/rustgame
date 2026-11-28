//! The 8-bit look shared by the newer games (snake, minesweeper, 2048,
//! bricks): NES-style boxes and buttons, bevelled tiles and checked text.
//! Tetris keeps its own copies in `games/tetris/src/draw.rs`.

use pixelfont::Rect;
use touch::Button;

use crate::canvas::{BLACK, Canvas, Rgb};

pub const WHITE: Rgb = (252, 252, 252);
pub const GREY: Rgb = (124, 124, 124);
pub const DIM: Rgb = (120, 120, 150);
pub const BACKGROUND: Rgb = (8, 8, 24);
pub const BOX_BLUE: Rgb = (0, 88, 248);
pub const GOLD: Rgb = (248, 184, 0);
pub const RED: Rgb = (228, 0, 88);
pub const GREEN: Rgb = (0, 184, 0);

/// Blend towards white (`amount` > 0) or black (`amount` < 0).
pub fn shade(color: Rgb, amount: f32) -> Rgb {
    let target = if amount > 0.0 { 255.0 } else { 0.0 };
    let blend = |c: u8| (c as f32 + (target - c as f32) * amount.abs()) as u8;
    (blend(color.0), blend(color.1), blend(color.2))
}

pub fn center(rect: Rect) -> (i32, i32) {
    (rect.x + rect.w / 2, rect.y + rect.h / 2)
}

/// Whether `pos` is inside `rect` (right and bottom edges excluded).
pub fn contains(rect: Rect, (x, y): (i32, i32)) -> bool {
    rect.x <= x && x < rect.x + rect.w && rect.y <= y && y < rect.y + rect.h
}

/// A bevelled block: light top-left edge, dark bottom-right edge, glint.
pub fn draw_tile(canvas: &mut Canvas, rect: Rect, color: Rgb) {
    canvas.fill_rect(rect, color);
    let (light, dark) = (shade(color, 0.5), shade(color, -0.55));
    canvas.fill_rect(Rect::new(rect.x, rect.y, rect.w, 1), light);
    canvas.fill_rect(Rect::new(rect.x, rect.y, 1, rect.h), light);
    canvas.fill_rect(Rect::new(rect.x, rect.y + rect.h - 1, rect.w, 1), dark);
    canvas.fill_rect(Rect::new(rect.x + rect.w - 1, rect.y, 1, rect.h), dark);
    if rect.w >= 8 && rect.h >= 8 {
        canvas.fill_rect(Rect::new(rect.x + 2, rect.y + 2, 2, 2), shade(color, 0.85));
    }
}

/// NES-style dialog box: black fill, white outer line, blue inner line, and
/// an optional title cut into the top edge.
pub fn draw_box(canvas: &mut Canvas, rect: Rect, title: Option<&str>) {
    canvas.fill_rect(rect, BLACK);
    canvas.outline(rect, WHITE, 1);
    canvas.outline(
        Rect::new(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4),
        BOX_BLUE,
        1,
    );
    if let Some(title) = title {
        let (width, height) = pixelfont::text_size(title, 1);
        canvas.fill_rect(Rect::new(rect.x + 5, rect.y - 3, width + 4, height), BLACK);
        label(canvas, title, (rect.x + 7, rect.y - 3), WHITE, 1);
    }
}

/// A boxed button with its label centred; big buttons get bigger labels.
pub fn draw_button(canvas: &mut Canvas, button: &Button, color: Rgb) {
    draw_box(canvas, button.rect, None);
    let scale = if button.rect.h >= 60 { 3 } else { 2 };
    centered(canvas, button.label, center(button.rect), color, scale);
}

/// Pixel font text with its top-left at `pos`.
pub fn label(canvas: &mut Canvas, text: &str, pos: (i32, i32), color: Rgb, scale: i32) -> Rect {
    let area = canvas.text(text, pos, color, scale, None, false);
    checked(canvas, text, area);
    area
}

/// Pixel font text with its top-right at `pos`.
pub fn label_right(
    canvas: &mut Canvas,
    text: &str,
    (x, y): (i32, i32),
    color: Rgb,
    scale: i32,
) -> Rect {
    let (width, _) = pixelfont::text_size(text, scale);
    label(canvas, text, (x - width, y), color, scale)
}

/// Pixel font text centred on `pos`.
pub fn centered(canvas: &mut Canvas, text: &str, pos: (i32, i32), color: Rgb, scale: i32) -> Rect {
    let area = canvas.text(text, pos, color, scale, None, true);
    checked(canvas, text, area);
    area
}

/// Centred text with a hard drop shadow, for titles.
pub fn title(canvas: &mut Canvas, text: &str, pos: (i32, i32), color: Rgb, scale: i32) -> Rect {
    let area = canvas.text(text, pos, color, scale, Some(shade(color, -0.7)), true);
    checked(canvas, text, area);
    area
}

/// Every text must be drawable and on screen; the game tests draw every
/// screen in debug builds, so a wrong text fails them.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shade_blends_towards_white_and_black() {
        assert_eq!(shade((100, 100, 100), 0.5), (177, 177, 177));
        assert_eq!(shade((100, 100, 100), -0.5), (50, 50, 50));
        assert_eq!(shade((10, 20, 30), 0.0), (10, 20, 30));
    }

    #[test]
    fn contains_is_half_open() {
        let rect = Rect::new(10, 20, 5, 5);
        assert!(contains(rect, (10, 20)));
        assert!(contains(rect, (14, 24)));
        assert!(!contains(rect, (15, 20)));
        assert!(!contains(rect, (10, 25)));
    }

    #[test]
    fn draw_button_frames_and_labels() {
        let mut canvas = Canvas::new(100, 40);
        let button = Button::new("go", "GO", Rect::new(10, 5, 80, 30));
        draw_button(&mut canvas, &button, GOLD);
        assert_eq!(canvas.get(10, 5), Some(WHITE));
        assert_eq!(canvas.get(12, 7), Some(BOX_BLUE));
        let lit = (0..100).any(|x| (0..40).any(|y| canvas.get(x, y) == Some(GOLD)));
        assert!(lit);
    }

    #[test]
    fn draw_tile_bevels_the_edges() {
        let mut canvas = Canvas::new(10, 10);
        draw_tile(&mut canvas, Rect::new(0, 0, 10, 10), (100, 100, 100));
        assert_eq!(canvas.get(0, 5), Some(shade((100, 100, 100), 0.5)));
        assert_eq!(canvas.get(9, 5), Some(shade((100, 100, 100), -0.55)));
        assert_eq!(canvas.get(5, 5), Some((100, 100, 100)));
    }
}
