//! The browser display: draws the `PixelCanvas` into the page's `<canvas>`
//! with its 2D context, scaled with hard pixel edges, letterboxed, and
//! optionally overlaid with scanlines.
//!
//! bevy's renderer is not built for the browser (see `sandbox-ui`'s
//! `Cargo.toml`), so the page canvas has no WebGL context and its 2D context
//! is free to use. The
//! frame goes through an off-page canvas at the game's own size, which the
//! browser then scales to the page canvas.

use std::cell::RefCell;

use bevy::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement, ImageData};

use crate::display::{PixelCanvas, scanline_pixels};

/// The page canvas and the off-page canvases the frame and the scanlines are
/// put on before scaling.
struct Surfaces {
    page: HtmlCanvasElement,
    page_context: CanvasRenderingContext2d,
    frame: HtmlCanvasElement,
    frame_context: CanvasRenderingContext2d,
    scanlines: HtmlCanvasElement,
    scanlines_context: CanvasRenderingContext2d,
    /// Canvas height the scanline image was made for (0: none yet).
    scanlines_for: i32,
}

thread_local! {
    // DOM handles are not `Send`, so they live here rather than in a resource;
    // the browser runs the app on one thread.
    static SURFACES: RefCell<Option<Surfaces>> = const { RefCell::new(None) };
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, present);
}

fn context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas.get_context("2d").ok()??.dyn_into().ok()
}

fn off_page_canvas(document: &Document) -> Option<(HtmlCanvasElement, CanvasRenderingContext2d)> {
    let canvas: HtmlCanvasElement = document.create_element("canvas").ok()?.dyn_into().ok()?;
    let context = context(&canvas)?;
    Some((canvas, context))
}

impl Surfaces {
    fn find() -> Option<Self> {
        let document = web_sys::window()?.document()?;
        let page: HtmlCanvasElement = document
            .query_selector(crate::CANVAS_SELECTOR)
            .ok()??
            .dyn_into()
            .ok()?;
        let page_context = context(&page)?;
        let (frame, frame_context) = off_page_canvas(&document)?;
        let (scanlines, scanlines_context) = off_page_canvas(&document)?;
        info!("display: 2D canvas");
        Some(Self {
            page,
            page_context,
            frame,
            frame_context,
            scanlines,
            scanlines_context,
            scanlines_for: 0,
        })
    }

    /// Put `pixels` (RGBA, `width` x `height`) on `canvas`, resizing it first.
    fn put(
        canvas: &HtmlCanvasElement,
        context: &CanvasRenderingContext2d,
        pixels: &[u8],
        width: i32,
        height: i32,
    ) {
        let (width, height) = (width as u32, height as u32);
        if canvas.width() != width || canvas.height() != height {
            canvas.set_width(width);
            canvas.set_height(height);
        }
        if let Ok(image) =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(pixels), width, height)
        {
            let _ = context.put_image_data(&image, 0.0, 0.0);
        }
    }

    /// Show the frame; `changed`: the game touched the canvas since the last
    /// call. An unchanged canvas on an unchanged page is not drawn again.
    fn draw(&mut self, pixel: &PixelCanvas, changed: bool) {
        // Without bevy's renderer nothing sizes the page canvas's backing store
        // (it stays at the default 300x150), so match it to its size on the
        // page in device pixels, as the renderer's surface did.
        let ratio = web_sys::window().map_or(1.0, |window| window.device_pixel_ratio());
        let backing = (
            (self.page.client_width() as f64 * ratio).round() as u32,
            (self.page.client_height() as f64 * ratio).round() as u32,
        );
        let resized = backing.0 > 0 && (self.page.width(), self.page.height()) != backing;
        if resized {
            self.page.set_width(backing.0);
            self.page.set_height(backing.1);
        }
        if !changed && !resized {
            return;
        }

        let (width, height) = pixel.canvas.size();
        if changed {
            Self::put(
                &self.frame,
                &self.frame_context,
                pixel.canvas.pixels(),
                width,
                height,
            );
        }
        let (page_w, page_h) = (self.page.width() as f32, self.page.height() as f32);
        let (x, y, w, h) = touch::fit_box(page_w, page_h, width as f32, height as f32);
        let (x, y, w, h) = (x as f64, y as f64, w as f64, h as f64);
        let page = &self.page_context;
        // Resizing the page canvas resets its state, so set this every frame.
        page.set_image_smoothing_enabled(false);
        page.set_fill_style_str("#000");
        page.fill_rect(0.0, 0.0, page_w as f64, page_h as f64);
        let _ = page.draw_image_with_html_canvas_element_and_dw_and_dh(&self.frame, x, y, w, h);

        if pixel.scanlines {
            if self.scanlines_for != height {
                let pixels = scanline_pixels(height);
                Self::put(
                    &self.scanlines,
                    &self.scanlines_context,
                    &pixels,
                    1,
                    height * 2,
                );
                self.scanlines_for = height;
            }
            let _ =
                page.draw_image_with_html_canvas_element_and_dw_and_dh(&self.scanlines, x, y, w, h);
        }
    }
}

fn present(canvas: Res<PixelCanvas>) {
    SURFACES.with_borrow_mut(|surfaces| {
        let found_now = surfaces.is_none();
        if found_now {
            *surfaces = Surfaces::find();
        }
        if let Some(surfaces) = surfaces {
            surfaces.draw(&canvas, canvas.is_changed() || found_now);
        }
    });
}
