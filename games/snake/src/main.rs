//! Snake on a walled 20x20 grid.
//!
//! Arrows (or WASD) turn, enter starts, space or p pauses, esc ends the game
//! (and quits from the ready and game over screens on the desktop). On a
//! touch screen swipe to turn and tap to start or go on; the portrait layout
//! adds thumb buttons under the board. A window (or phone) taller than wide
//! gets the portrait layout; `--portrait` opens one. The best score is kept
//! in the browser's local storage, or in the user's data folder on desktop.

mod game;

use bevy::prelude::*;
use game::{Event, Game, Key, Layout, Outcome, STORAGE_KEY};
use sandbox_ui::{Orientation, PixelCanvas, Pointers, SandboxPlugin};

/// The canvas opens at twice its size.
const SCALE: i32 = 2;

#[derive(Resource)]
struct Snake {
    game: Game,
    landscape: Layout,
    portrait: Layout,
}

const KEYS: [(KeyCode, Key); 13] = [
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
    (KeyCode::Space, Key::Space),
    (KeyCode::KeyP, Key::Pause),
    (KeyCode::Escape, Key::Escape),
];

fn main() {
    let web = cfg!(target_arch = "wasm32");
    let portrait = !web && std::env::args().any(|arg| arg == "--portrait");
    let snake = Snake {
        game: Game::new(
            web,
            sandbox_ui::clock::seed(),
            sandbox_ui::storage::load_best(STORAGE_KEY),
        ),
        landscape: game::landscape(),
        portrait: game::portrait(),
    };
    let (width, height) = if portrait {
        snake.portrait.size
    } else {
        snake.landscape.size
    };
    sandbox_ui::game_app("Snake", (width * SCALE) as u32, (height * SCALE) as u32)
        .add_plugins(SandboxPlugin {
            canvas_size: (width, height),
            scanlines: true,
        })
        .insert_resource(snake)
        .add_systems(Update, play)
        .run();
}

fn play(
    mut snake: ResMut<Snake>,
    mut pixel: ResMut<PixelCanvas>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    pointers: Res<Pointers>,
    orientation: Res<Orientation>,
) {
    let Snake {
        game,
        landscape,
        portrait,
    } = &mut *snake;
    let layout = match *orientation {
        Orientation::Landscape => &*landscape,
        Orientation::Portrait => &*portrait,
    };

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

    let mut outcomes = vec![game.update(pointers.now_ms)];
    outcomes.extend(events.into_iter().map(|event| game.handle(event, layout)));
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
