//! The 8-bit drawing primitives of `tetris.py`: bevelled tiles, NES-style
//! boxes, the rainbow title, on any [`Paint`] surface.

use pixelfont::Rect;
use sandbox_ui::{Canvas, Rgb};
use tetris_core::SHAPE_COLORS;

use crate::layout::WIDTH;

pub const BLACK: Rgb = (0, 0, 0);
pub const WHITE: Rgb = (252, 252, 252);
pub const GREY: Rgb = (124, 124, 124);
pub const DIM: Rgb = (120, 120, 150);
pub const BOARD_BG: Rgb = (8, 8, 24);
pub const GRID_DOT: Rgb = (36, 36, 64);
pub const BOX_BLUE: Rgb = (0, 88, 248);
pub const GOLD: Rgb = (248, 184, 0);
pub const SILVER: Rgb = (188, 188, 188);
pub const BRONZE: Rgb = (200, 108, 40);
pub const RANK_COLORS: [Rgb; 3] = [GOLD, SILVER, BRONZE];

/// Something to draw on: the canvas, or a view of it shifted down (the
/// centred menu area in portrait).
pub trait Paint {
    fn fill_rect(&mut self, rect: Rect, color: Rgb);
    fn outline(&mut self, rect: Rect, color: Rgb, width: i32);
    fn text(
        &mut self,
        text: &str,
        pos: (i32, i32),
        color: Rgb,
        scale: i32,
        shadow: Option<Rgb>,
        center: bool,
    ) -> Rect;
}

impl Paint for Canvas {
    fn fill_rect(&mut self, rect: Rect, color: Rgb) {
        Canvas::fill_rect(self, rect, color);
    }

    fn outline(&mut self, rect: Rect, color: Rgb, width: i32) {
        Canvas::outline(self, rect, color, width);
    }

    fn text(
        &mut self,
        text: &str,
        pos: (i32, i32),
        color: Rgb,
        scale: i32,
        shadow: Option<Rgb>,
        center: bool,
    ) -> Rect {
        Canvas::text(self, text, pos, color, scale, shadow, center)
    }
}

/// The canvas seen `dy` pixels lower: drawing at (x, y) lands at (x, y + dy).
pub struct View<'a> {
    pub canvas: &'a mut Canvas,
    pub dy: i32,
}

impl Paint for View<'_> {
    fn fill_rect(&mut self, rect: Rect, color: Rgb) {
        self.canvas
            .fill_rect(Rect::new(rect.x, rect.y + self.dy, rect.w, rect.h), color);
    }

    fn outline(&mut self, rect: Rect, color: Rgb, width: i32) {
        self.canvas.outline(
            Rect::new(rect.x, rect.y + self.dy, rect.w, rect.h),
            color,
            width,
        );
    }

    fn text(
        &mut self,
        text: &str,
        pos: (i32, i32),
        color: Rgb,
        scale: i32,
        shadow: Option<Rgb>,
        center: bool,
    ) -> Rect {
        let area = self
            .canvas
            .text(text, (pos.0, pos.1 + self.dy), color, scale, shadow, center);
        Rect::new(area.x, area.y - self.dy, area.w, area.h)
    }
}

/// Blend towards white (`amount` > 0) or black (`amount` < 0).
pub fn shade(color: Rgb, amount: f32) -> Rgb {
    let target = if amount > 0.0 { 255.0 } else { 0.0 };
    let blend = |c: u8| (c as f32 + (target - c as f32) * amount.abs()) as u8;
    (blend(color.0), blend(color.1), blend(color.2))
}

/// A bevelled 8-bit block: light top-left edge, dark bottom-right edge, glint.
pub fn draw_tile(surface: &mut impl Paint, color: Rgb, x: i32, y: i32, size: i32) {
    surface.fill_rect(Rect::new(x, y, size, size), color);
    let (light, dark) = (shade(color, 0.5), shade(color, -0.55));
    surface.fill_rect(Rect::new(x, y, size, 1), light);
    surface.fill_rect(Rect::new(x, y, 1, size), light);
    surface.fill_rect(Rect::new(x, y + size - 1, size, 1), dark);
    surface.fill_rect(Rect::new(x + size - 1, y, 1, size), dark);
    if size >= 8 {
        surface.fill_rect(Rect::new(x + 2, y + 2, 2, 2), shade(color, 0.85));
    }
}

/// NES-style dialog box: black fill, white outer line, blue inner line, and
/// an optional title cut into the top edge.
pub fn draw_box(surface: &mut impl Paint, rect: Rect, title: Option<&str>) {
    surface.fill_rect(rect, BLACK);
    surface.outline(rect, WHITE, 1);
    surface.outline(
        Rect::new(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4),
        BOX_BLUE,
        1,
    );
    if let Some(title) = title {
        let (width, height) = pixelfont::text_size(title, 1);
        surface.fill_rect(Rect::new(rect.x + 5, rect.y - 3, width + 4, height), BLACK);
        surface.text(title, (rect.x + 7, rect.y - 3), WHITE, 1, None, false);
    }
}

/// "TETRIS" with each letter in a different piece colour and a hard shadow.
pub fn draw_title(surface: &mut impl Paint, center_y: i32, scale: i32) {
    let word = "TETRIS";
    let (width, height) = pixelfont::text_size(word, scale);
    let x = (WIDTH - width).div_euclid(2);
    let advance = (pixelfont::GLYPH_WIDTH + pixelfont::SPACING) * scale;
    for (i, letter) in word.chars().enumerate() {
        let color = SHAPE_COLORS[i % SHAPE_COLORS.len()];
        let pos = (x + i as i32 * advance, center_y - height.div_euclid(2));
        surface.text(
            &letter.to_string(),
            pos,
            color,
            scale,
            Some(shade(color, -0.7)),
            false,
        );
    }
}

pub fn center(rect: Rect) -> (i32, i32) {
    (rect.x + rect.w / 2, rect.y + rect.h / 2)
}

/// A boxed button; big buttons get bigger labels.
pub fn draw_button(surface: &mut impl Paint, rect: Rect, label: &str, color: Rgb) {
    draw_box(surface, rect, None);
    let scale = if rect.h >= 60 { 3 } else { 2 };
    surface.text(label, center(rect), color, scale, None, true);
}
