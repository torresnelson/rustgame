//! Shows the `PixelCanvas` on screen, scaled with hard pixel edges,
//! letterboxed to fit the window, and optionally overlaid with scanlines (the
//! tetris look).
//!
//! On the desktop the canvas is uploaded to a texture each frame and drawn by
//! bevy's renderer ([`gpu`]). In the browser it is drawn with the page
//! canvas's 2D context instead (`web_display`): on Firefox for Android,
//! bevy's WebGL2 draws silently produced nothing (see
//! `plans/claude/mobile-black-screen.md`), and one scaled image needs no GPU
//! pipeline.

use bevy::prelude::*;

use crate::canvas::Canvas;

/// Darkening of every other screen row, as alpha of a black line:
/// 1 - 190/255, the factor the Python tetris multiplies by.
const SCANLINE_ALPHA: u8 = 65;

/// The canvas games draw on each frame.
#[derive(Resource, Debug)]
pub struct PixelCanvas {
    pub canvas: Canvas,
    pub scanlines: bool,
}

/// Two screen rows per canvas row: clear, then a translucent black line.
/// RGBA pixels of a 1 x (2 * `canvas_height`) image.
pub fn scanline_pixels(canvas_height: i32) -> Vec<u8> {
    (0..canvas_height * 2)
        .flat_map(|row| {
            if row % 2 == 1 {
                [0, 0, 0, SCANLINE_ALPHA]
            } else {
                [0, 0, 0, 0]
            }
        })
        .collect()
}

pub(crate) fn plugin(app: &mut App) {
    #[cfg(target_arch = "wasm32")]
    app.add_plugins(crate::web_display::plugin);
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(gpu::plugin);
}

/// The desktop display: one texture and two sprites drawn by bevy.
#[cfg(not(target_arch = "wasm32"))]
mod gpu {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::ImageSampler;
    use bevy::prelude::*;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy::window::PrimaryWindow;

    use super::{PixelCanvas, scanline_pixels};
    use crate::canvas::Canvas;

    #[derive(Resource)]
    struct DisplayImages {
        canvas: Handle<Image>,
        scanlines: Handle<Image>,
    }

    #[derive(Component)]
    struct CanvasSprite;

    #[derive(Component)]
    struct ScanlineSprite;

    pub(super) fn plugin(app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(PostUpdate, present);
    }

    fn texture(width: i32, height: i32, pixels: Vec<u8>) -> Image {
        let mut image = Image::new(
            Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        image
    }

    pub(super) fn canvas_image(canvas: &Canvas) -> Image {
        let (width, height) = canvas.size();
        texture(width, height, canvas.pixels().to_vec())
    }

    fn scanline_image(canvas_height: i32) -> Image {
        texture(1, canvas_height * 2, scanline_pixels(canvas_height))
    }

    fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, canvas: Res<PixelCanvas>) {
        let handles = DisplayImages {
            canvas: images.add(canvas_image(&canvas.canvas)),
            scanlines: images.add(scanline_image(canvas.canvas.height())),
        };
        commands.spawn((Camera2d, Msaa::Off));
        commands.spawn((CanvasSprite, Sprite::from_image(handles.canvas.clone())));
        commands.spawn((
            ScanlineSprite,
            Sprite::from_image(handles.scanlines.clone()),
            Transform::from_xyz(0.0, 0.0, 1.0),
        ));
        commands.insert_resource(handles);
    }

    fn present(
        canvas: Res<PixelCanvas>,
        mut handles: ResMut<DisplayImages>,
        mut images: ResMut<Assets<Image>>,
        window: Single<&Window, With<PrimaryWindow>>,
        mut canvas_sprite: Single<&mut Sprite, (With<CanvasSprite>, Without<ScanlineSprite>)>,
        mut scanline_sprite: Single<(&mut Sprite, &mut Visibility), With<ScanlineSprite>>,
    ) {
        let (width, height) = canvas.canvas.size();
        let (sprite, visibility) = &mut *scanline_sprite;
        if canvas.is_changed() {
            let same_size = images
                .get(&handles.canvas)
                .is_some_and(|image| image.size() == UVec2::new(width as u32, height as u32));
            if same_size {
                if let Some(mut image) = images.get_mut(&handles.canvas) {
                    image.data = Some(canvas.canvas.pixels().to_vec());
                }
            } else {
                // A new size (e.g. the phone was turned): new textures, old ones dropped.
                let old = std::mem::replace(
                    &mut *handles,
                    DisplayImages {
                        canvas: images.add(canvas_image(&canvas.canvas)),
                        scanlines: images.add(scanline_image(height)),
                    },
                );
                images.remove(&old.canvas);
                images.remove(&old.scanlines);
                canvas_sprite.image = handles.canvas.clone();
                sprite.image = handles.scanlines.clone();
            }
        }

        let size = window.size();
        let (_, _, box_w, box_h) = touch::fit_box(size.x, size.y, width as f32, height as f32);
        let fitted = Some(Vec2::new(box_w, box_h));
        if canvas_sprite.custom_size != fitted {
            canvas_sprite.custom_size = fitted;
        }
        if sprite.custom_size != fitted {
            sprite.custom_size = fitted;
        }
        visibility.set_if_neq(if canvas.scanlines {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanlines_darken_every_second_row() {
        let pixels = scanline_pixels(2);
        let alphas: Vec<u8> = pixels.chunks(4).map(|p| p[3]).collect();
        assert_eq!(alphas, [0, SCANLINE_ALPHA, 0, SCANLINE_ALPHA]);
    }

    #[test]
    fn canvas_image_matches_the_canvas() {
        let image = gpu::canvas_image(&Canvas::new(4, 3));
        assert_eq!(image.size(), UVec2::new(4, 3));
        assert_eq!(image.data.as_deref(), Some(Canvas::new(4, 3).pixels()));
    }
}
