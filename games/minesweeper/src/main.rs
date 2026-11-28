//! Minesweeper: easy, medium and hard boards; the first opening is always
//! safe.
//!
//! Menu: up/down + enter, 1 2 3, or a click or tap. Board: click or tap to
//! dig, right click or hold to flag; DIG/FLAG swaps the two for taps. The
//! arrows move a cursor, enter or space digs, f flags, n starts a new board
//! and esc goes back to the menu (and quits from it on the desktop). A
//! window (or phone) taller than wide gets the portrait layout;
//! `--portrait` opens one. Best times are kept in the browser's local
//! storage, or in the user's data folder on desktop.

mod game;

use bevy::prelude::*;
use game::{Event, Game, Key, Layout, Outcome, STORAGE_KEY};
use sandbox_ui::{Orientation, PixelCanvas, Pointers, SandboxPlugin};

/// The canvas opens at twice its size.
const SCALE: i32 = 2;

#[derive(Resource)]
struct Minesweeper {
    game: Game,
    landscape: Layout,
    portrait: Layout,
}

const KEYS: [(KeyCode, Key); 20] = [
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
    (KeyCode::KeyF, Key::Flag),
    (KeyCode::KeyN, Key::New),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::Digit1, Key::One),
    (KeyCode::Digit2, Key::Two),
    (KeyCode::Digit3, Key::Three),
    (KeyCode::Numpad1, Key::One),
    (KeyCode::Numpad2, Key::Two),
    (KeyCode::Numpad3, Key::Three),
];

fn main() {
    let web = cfg!(target_arch = "wasm32");
    let portrait = !web && std::env::args().any(|arg| arg == "--portrait");
    let saved = sandbox_ui::storage::load_text(STORAGE_KEY);
    let minesweeper = Minesweeper {
        game: Game::new(
            web,
            sandbox_ui::clock::seed(),
            game::parse_bests(saved.as_deref()),
        ),
        landscape: game::landscape(),
        portrait: game::portrait(),
    };
    let (width, height) = if portrait {
        minesweeper.portrait.size
    } else {
        minesweeper.landscape.size
    };
    sandbox_ui::game_app(
        "Minesweeper",
        (width * SCALE) as u32,
        (height * SCALE) as u32,
    )
    .add_plugins(SandboxPlugin {
        canvas_size: (width, height),
        scanlines: true,
    })
    .insert_resource(minesweeper)
    .add_systems(Update, play)
    .run();
}

fn play(
    mut minesweeper: ResMut<Minesweeper>,
    mut pixel: ResMut<PixelCanvas>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    pointers: Res<Pointers>,
    orientation: Res<Orientation>,
    mut last_hover: Local<Option<(i32, i32)>>,
) {
    let Minesweeper {
        game,
        landscape,
        portrait,
    } = &mut *minesweeper;
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

    let mut outcomes = vec![game.update(pointers.now_ms, layout)];
    outcomes.extend(events.into_iter().map(|event| game.handle(event, layout)));
    for outcome in outcomes {
        match outcome {
            Outcome::Continue => {}
            Outcome::Quit => {
                exit.write(AppExit::Success);
            }
            Outcome::Save => {
                if let Err(error) = sandbox_ui::storage::save_text(STORAGE_KEY, &game.best_text()) {
                    warn!("could not save the best times: {error}");
                }
            }
        }
    }

    if pixel.canvas.size() != layout.size {
        pixel.canvas.resize(layout.size.0, layout.size.1);
    }
    game.draw(&mut pixel.canvas, layout);
}
