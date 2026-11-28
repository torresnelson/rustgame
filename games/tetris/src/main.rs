//! Tetris: Rust/bevy port of `src/tetris/tetris.py`.
//!
//! Menu: up/down + enter, or a click or tap. In game: arrows move/rotate,
//! space hard drops, `p` pauses, `esc` ends; on a touch screen tap rotates,
//! drag moves, drag down soft drops, flick down hard drops. A window (or
//! phone) taller than wide gets the portrait layout with thumb buttons;
//! `--portrait` opens one. High scores are kept in the browser's local
//! storage, or in the user's data folder on desktop.

mod draw;
mod game;
mod layout;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key as Logical, KeyboardInput};
use bevy::prelude::*;
use game::{Event, Game, Key, Outcome};
use layout::Layout;
use sandbox_ui::{Orientation, PixelCanvas, Pointer, PointerId, Pointers, SandboxPlugin};
use tetris_core::highscores;

/// The canvas opens at twice its size, the 2x pixels of `tetris.py`.
const SCALE: i32 = 2;

#[derive(Resource)]
struct Tetris {
    game: Game,
    landscape: Layout,
    portrait: Layout,
}

const KEYS: [(KeyCode, Key); 10] = [
    (KeyCode::ArrowUp, Key::Up),
    (KeyCode::ArrowDown, Key::Down),
    (KeyCode::ArrowLeft, Key::Left),
    (KeyCode::ArrowRight, Key::Right),
    (KeyCode::Space, Key::Space),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::NumpadEnter, Key::Enter),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::KeyP, Key::Pause),
    (KeyCode::Backspace, Key::Backspace),
];

fn main() {
    let web = cfg!(target_arch = "wasm32");
    let portrait = !web && std::env::args().any(|arg| arg == "--portrait");
    let (landscape, portrait_layout) = (layout::landscape(), layout::portrait());
    let start = if portrait {
        &portrait_layout
    } else {
        &landscape
    };
    let saved = sandbox_ui::storage::load_text(highscores::STORAGE_KEY);
    let game = Game::new(
        web,
        sandbox_ui::clock::seed(),
        highscores::from_json(saved.as_deref()),
        sandbox_ui::clock::today(),
        start,
    );
    let (width, height) = start.size;
    sandbox_ui::game_app("Tetris", (width * SCALE) as u32, (height * SCALE) as u32)
        .add_plugins(SandboxPlugin {
            canvas_size: (width, height),
            scanlines: true,
        })
        .insert_resource(Tetris {
            game,
            landscape,
            portrait: portrait_layout,
        })
        .add_systems(Update, play)
        .run();
}

#[allow(clippy::too_many_arguments)]
fn play(
    mut tetris: ResMut<Tetris>,
    mut pixel: ResMut<PixelCanvas>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    pointers: Res<Pointers>,
    orientation: Res<Orientation>,
    mut last_hover: Local<Option<(i32, i32)>>,
) {
    let Tetris {
        game,
        landscape,
        portrait,
    } = &mut *tetris;
    let layout = match *orientation {
        Orientation::Landscape => &*landscape,
        Orientation::Portrait => &*portrait,
    };
    game.update(pointers.now_ms, layout);

    let mut events: Vec<Event> = keys
        .get_just_pressed()
        .map(|code| {
            let key = KEYS.iter().find(|(known, _)| known == code);
            Event::Key(key.map_or(Key::Other, |&(_, key)| key))
        })
        .collect();
    for input in typed.read() {
        if input.state != ButtonState::Pressed || input.repeat {
            continue;
        }
        match &input.logical_key {
            Logical::Character(text) => events.extend(text.chars().map(Event::Char)),
            Logical::Space => events.push(Event::Char(' ')),
            _ => {}
        }
    }
    events.extend(sandbox_ui::hover_moved(&mut last_hover, pointers.hover).map(Event::Hover));
    let left_or_touch = |p: &&Pointer| p.id != PointerId::MouseRight;
    let pressed = pointers.pressed.iter().filter(left_or_touch);
    events.extend(pressed.map(|p| Event::Press {
        id: p.id,
        pos: p.pos,
    }));
    let down = pointers.down.iter().filter(left_or_touch);
    events.extend(down.map(|p| Event::Move {
        id: p.id,
        pos: p.pos,
    }));
    let released = pointers.released.iter().filter(left_or_touch);
    events.extend(released.map(|p| Event::Release {
        id: p.id,
        pos: p.pos,
    }));

    for event in events {
        match game.handle(event, layout) {
            Outcome::Continue => {}
            Outcome::Quit => {
                exit.write(AppExit::Success);
            }
            Outcome::Save => {
                let text = highscores::to_json(&game.entries);
                if let Err(error) = sandbox_ui::storage::save_text(highscores::STORAGE_KEY, &text) {
                    warn!("could not save the high scores: {error}");
                }
            }
        }
    }

    if pixel.canvas.size() != layout.size {
        pixel.canvas.resize(layout.size.0, layout.size.1);
    }
    game.draw(&mut pixel.canvas, layout);
}
