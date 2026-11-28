//! Merge: slide number tiles on a 4x4 board, equal tiles merge, aim for 2048.
//!
//! Arrows (or WASD) slide, n or NEW starts over, enter goes on after the
//! goal or retries after the end, esc quits on the desktop. On a touch
//! screen swipe anywhere to slide. A window (or phone) taller than wide gets
//! the portrait layout; `--portrait` opens one. The best score is kept in
//! the browser's local storage, or in the user's data folder on desktop.

mod game;

use bevy::prelude::*;
use game::{Event, Game, Key, Layout, Outcome, STORAGE_KEY};
use sandbox_ui::{Orientation, PixelCanvas, Pointers, SandboxPlugin};

/// The canvas opens at twice its size.
const SCALE: i32 = 2;

#[derive(Resource)]
struct Merge {
    game: Game,
    landscape: Layout,
    portrait: Layout,
}

const KEYS: [(KeyCode, Key); 12] = [
    (KeyCode::ArrowUp, Key::Up),
    (KeyCode::KeyW, Key::Up),
    (KeyCode::ArrowDown, Key::Down),
    (KeyCode::KeyS, Key::Down),
    (KeyCode::ArrowLeft, Key::Left),
    (KeyCode::KeyA, Key::Left),
    (KeyCode::ArrowRight, Key::Right),
    (KeyCode::KeyD, Key::Right),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::NumpadEnter, Key::Enter),
    (KeyCode::KeyN, Key::New),
    (KeyCode::Escape, Key::Escape),
];

fn main() {
    let web = cfg!(target_arch = "wasm32");
    let portrait = !web && std::env::args().any(|arg| arg == "--portrait");
    let merge = Merge {
        game: Game::new(
            web,
            sandbox_ui::clock::seed(),
            sandbox_ui::storage::load_best(STORAGE_KEY),
        ),
        landscape: game::landscape(),
        portrait: game::portrait(),
    };
    let (width, height) = if portrait {
        merge.portrait.size
    } else {
        merge.landscape.size
    };
    sandbox_ui::game_app("Merge", (width * SCALE) as u32, (height * SCALE) as u32)
        .add_plugins(SandboxPlugin {
            canvas_size: (width, height),
            scanlines: true,
        })
        .insert_resource(merge)
        .add_systems(Update, play)
        .run();
}

fn play(
    mut merge: ResMut<Merge>,
    mut pixel: ResMut<PixelCanvas>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    pointers: Res<Pointers>,
    orientation: Res<Orientation>,
) {
    let Merge {
        game,
        landscape,
        portrait,
    } = &mut *merge;
    let layout = match *orientation {
        Orientation::Landscape => &*landscape,
        Orientation::Portrait => &*portrait,
    };
    game.update(pointers.now_ms);

    let mut events: Vec<Event> = keys
        .get_just_pressed()
        .filter_map(|code| KEYS.iter().find(|(known, _)| known == code))
        .map(|&(_, key)| Event::Key(key))
        .collect();
    events.extend(pointers.pressed.iter().map(|p| Event::Press {
        id: p.id,
        pos: p.pos,
    }));
    events.extend(pointers.down.iter().map(|p| Event::Move {
        id: p.id,
        pos: p.pos,
    }));
    events.extend(
        pointers
            .released
            .iter()
            .map(|p| Event::Release { id: p.id }),
    );

    for event in events {
        match game.handle(event, layout) {
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
