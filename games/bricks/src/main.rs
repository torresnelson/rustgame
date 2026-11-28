//! Bricks: keep the ball in play with the paddle and break every brick.
//!
//! Left/right (or A/D) or the mouse move the paddle; space, enter, up or a
//! click launch the ball, p pauses, esc quits on the desktop. On a touch
//! screen drag anywhere to move the paddle and tap to launch; the portrait
//! layout adds a strip under the field to drag from. A window (or phone)
//! taller than wide gets the portrait layout; `--portrait` opens one. The
//! game runs at 60 steps per second whatever the screen's frame rate. The
//! best score is kept in the browser's local storage, or in the user's data
//! folder on desktop.

mod game;

use bevy::prelude::*;
use game::{Event, Game, Held, Key, Layout, Outcome, STORAGE_KEY};
use sandbox_ui::{Orientation, PixelCanvas, Pointers, SandboxPlugin};

/// The desktop window is this many times the canvas size (as a fraction).
const SCALE: (i32, i32) = (3, 2);

#[derive(Resource)]
struct Bricks {
    game: Game,
    landscape: Layout,
    portrait: Layout,
    /// Steps owed, as a fraction of a step.
    carry: f32,
}

const KEYS: [(KeyCode, Key); 7] = [
    (KeyCode::Space, Key::Launch),
    (KeyCode::Enter, Key::Launch),
    (KeyCode::NumpadEnter, Key::Launch),
    (KeyCode::ArrowUp, Key::Launch),
    (KeyCode::KeyW, Key::Launch),
    (KeyCode::KeyP, Key::Pause),
    (KeyCode::Escape, Key::Escape),
];

fn main() {
    let web = cfg!(target_arch = "wasm32");
    let portrait = !web && std::env::args().any(|arg| arg == "--portrait");
    let bricks = Bricks {
        game: Game::new(web, sandbox_ui::storage::load_best(STORAGE_KEY)),
        landscape: game::landscape(),
        portrait: game::portrait(),
        carry: 0.0,
    };
    let (width, height) = if portrait {
        bricks.portrait.size
    } else {
        bricks.landscape.size
    };
    let window = |size: i32| (size * SCALE.0 / SCALE.1) as u32;
    sandbox_ui::game_app("Bricks", window(width), window(height))
        .add_plugins(SandboxPlugin {
            canvas_size: (width, height),
            scanlines: true,
        })
        .insert_resource(bricks)
        .add_systems(Update, play)
        .run();
}

#[allow(clippy::too_many_arguments)]
fn play(
    mut bricks: ResMut<Bricks>,
    mut pixel: ResMut<PixelCanvas>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    pointers: Res<Pointers>,
    orientation: Res<Orientation>,
    time: Res<Time>,
    mut last_hover: Local<Option<(i32, i32)>>,
) {
    let Bricks {
        game,
        landscape,
        portrait,
        carry,
    } = &mut *bricks;
    let layout = match *orientation {
        Orientation::Landscape => &*landscape,
        Orientation::Portrait => &*portrait,
    };

    let mut events: Vec<Event> = keys
        .get_just_pressed()
        .filter_map(|code| KEYS.iter().find(|(known, _)| known == code))
        .map(|&(_, key)| Event::Key(key))
        .collect();
    events.extend(sandbox_ui::hover_moved(&mut last_hover, pointers.hover).map(Event::Hover));
    events.extend(pointers.pressed.iter().map(|p| Event::Press {
        id: p.id,
        pos: p.pos,
    }));
    events.extend(pointers.down.iter().map(|p| Event::Move {
        id: p.id,
        pos: p.pos,
    }));
    events.extend(pointers.released.iter().map(|p| Event::Release {
        id: p.id,
        pos: p.pos,
    }));
    let held = Held {
        left: keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]),
        right: keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]),
    };

    let mut outcomes: Vec<Outcome> = events
        .into_iter()
        .map(|event| game.handle(event, layout))
        .collect();
    for _ in 0..sandbox_ui::clock::steps_due(carry, time.delta_secs()) {
        outcomes.push(game.step(held));
    }
    for outcome in outcomes {
        match outcome {
            Outcome::Continue => {}
            Outcome::Quit => {
                exit.write(AppExit::Success);
            }
            Outcome::Save => {
                if let Err(error) = sandbox_ui::storage::save_best(STORAGE_KEY, game.best) {
                    warn!("could not save the best score: {error}");
                }
            }
        }
    }

    if pixel.canvas.size() != layout.size {
        pixel.canvas.resize(layout.size.0, layout.size.1);
    }
    game.draw(&mut pixel.canvas, layout);
}
