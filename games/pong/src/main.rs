//! Pong: Rust/bevy port of `src/main.c` from torresnelson/pong.
//!
//! Menu: up/down or w/s and enter (or m), 1 and 2 start an option directly,
//! or click or tap one. Player one moves with w/s (and the arrows when
//! playing alone), player two with the arrows; space pauses, m goes back to
//! the menu when paused or after a win, enter asks for a rematch, esc quits
//! (not in the browser). On a touch screen the left third of the field is
//! player one's controller (top half up, bottom half down), the button at
//! the top centre pauses, and a tap continues. The game runs at 60 steps per
//! second whatever the screen's frame rate.

mod game;

use bevy::prelude::*;
use game::{Event, Game, HEIGHT, Held, Key, Outcome, WIDTH};
use sandbox_ui::{PixelCanvas, PointerId, Pointers, SandboxPlugin};

#[derive(Resource)]
struct Pong {
    game: Game,
    /// Presses since the last step.
    pending: Vec<Event>,
    /// Steps owed, as a fraction of a step.
    carry: f32,
}

const KEYS: [(KeyCode, Key); 13] = [
    (KeyCode::ArrowUp, Key::Up),
    (KeyCode::ArrowDown, Key::Down),
    (KeyCode::KeyW, Key::W),
    (KeyCode::KeyS, Key::S),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::NumpadEnter, Key::Enter),
    (KeyCode::Space, Key::Space),
    (KeyCode::KeyM, Key::M),
    (KeyCode::Digit1, Key::One),
    (KeyCode::Numpad1, Key::One),
    (KeyCode::Digit2, Key::Two),
    (KeyCode::Numpad2, Key::Two),
    (KeyCode::Escape, Key::Escape),
];

fn main() {
    let web = cfg!(target_arch = "wasm32");
    sandbox_ui::game_app("Pong", WIDTH as u32, HEIGHT as u32)
        .add_plugins(SandboxPlugin {
            canvas_size: (WIDTH, HEIGHT),
            scanlines: false,
        })
        .insert_resource(Pong {
            game: Game::new(web, sandbox_ui::clock::seed()),
            pending: Vec::new(),
            carry: 0.0,
        })
        .add_systems(Update, play)
        .run();
}

fn play(
    mut pong: ResMut<Pong>,
    mut pixel: ResMut<PixelCanvas>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    pointers: Res<Pointers>,
    time: Res<Time>,
    mut last_hover: Local<Option<(i32, i32)>>,
) {
    let Pong {
        game,
        pending,
        carry,
    } = &mut *pong;

    pending.extend(keys.get_just_pressed().filter_map(|code| {
        KEYS.iter()
            .find(|(known, _)| known == code)
            .map(|&(_, key)| Event::Key(key))
    }));
    pending.extend(sandbox_ui::hover_moved(&mut last_hover, pointers.hover).map(Event::Hover));
    pending.extend(pointers.pressed.iter().filter_map(|p| match p.id {
        PointerId::Touch(_) => Some(Event::Tap(p.pos)),
        PointerId::MouseLeft => Some(Event::Click(p.pos)),
        PointerId::MouseRight => None,
    }));
    let held = Held {
        up: keys.pressed(KeyCode::ArrowUp),
        down: keys.pressed(KeyCode::ArrowDown),
        w: keys.pressed(KeyCode::KeyW),
        s: keys.pressed(KeyCode::KeyS),
        touches: pointers
            .down
            .iter()
            .filter(|p| p.id.is_touch())
            .map(|p| p.pos)
            .collect(),
    };

    for _ in 0..sandbox_ui::clock::steps_due(carry, time.delta_secs()) {
        let events = std::mem::take(pending);
        if game.step(&events, &held) == Outcome::Quit {
            exit.write(AppExit::Success);
            return;
        }
    }

    game.draw(&mut pixel.canvas);
}
