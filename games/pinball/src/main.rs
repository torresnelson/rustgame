//! Pinball: keep the ball on the table with the flippers and score off the
//! bumpers, slingshots and rollover lanes.
//!
//! Left/A/Z and right/D/M (or /) work the flippers, as do the left and
//! right mouse buttons; hold space or down to pull the plunger and let go to
//! launch; enter starts, p pauses, s turns the sound on or off, esc quits
//! on the desktop. On a touch screen hold the left or right half of the
//! screen for that flipper and hold the LAUNCH button to pull the plunger. A
//! window (or phone) taller than wide gets the portrait layout; `--portrait`
//! opens one. The game runs at 60 steps per second whatever the screen's
//! frame rate. The best score is kept in the browser's local storage, or in
//! the user's data folder on desktop. Sound effects are made in code
//! (`sounds`).

mod game;
mod sounds;

use bevy::prelude::*;
use game::{Event, Game, Key, Layout, Outcome, STORAGE_KEY};
use pinball_core::Held;
use sandbox_ui::audio::Audio;
use sandbox_ui::{Orientation, PixelCanvas, Pointers, SandboxPlugin};

/// The desktop window is this many times the canvas size (as a fraction).
const SCALE: (i32, i32) = (3, 2);

const LEFT_KEYS: [KeyCode; 3] = [KeyCode::ArrowLeft, KeyCode::KeyA, KeyCode::KeyZ];
const RIGHT_KEYS: [KeyCode; 4] = [
    KeyCode::ArrowRight,
    KeyCode::KeyD,
    KeyCode::KeyM,
    KeyCode::Slash,
];
const PLUNGER_KEYS: [KeyCode; 2] = [KeyCode::Space, KeyCode::ArrowDown];

#[derive(Resource)]
struct Pinball {
    game: Game,
    landscape: Layout,
    portrait: Layout,
    /// Steps owed, as a fraction of a step.
    carry: f32,
}

/// The game key a pressed key stands for, if any.
fn key_for(code: KeyCode) -> Option<Key> {
    if LEFT_KEYS.contains(&code) {
        return Some(Key::Left);
    }
    if RIGHT_KEYS.contains(&code) {
        return Some(Key::Right);
    }
    if PLUNGER_KEYS.contains(&code) {
        return Some(Key::Plunger);
    }
    match code {
        KeyCode::Enter | KeyCode::NumpadEnter => Some(Key::Start),
        KeyCode::KeyP => Some(Key::Pause),
        KeyCode::KeyS => Some(Key::Sound),
        KeyCode::Escape => Some(Key::Escape),
        _ => None,
    }
}

fn main() {
    let web = cfg!(target_arch = "wasm32");
    let portrait = !web && std::env::args().any(|arg| arg == "--portrait");
    let pinball = Pinball {
        game: Game::new(web, sandbox_ui::storage::load_best(STORAGE_KEY)),
        landscape: game::landscape(),
        portrait: game::portrait(),
        carry: 0.0,
    };
    let (width, height) = if portrait {
        pinball.portrait.size
    } else {
        pinball.landscape.size
    };
    let window = |size: i32| (size * SCALE.0 / SCALE.1) as u32;
    sandbox_ui::game_app("Pinball", window(width), window(height))
        .add_plugins(SandboxPlugin {
            canvas_size: (width, height),
            scanlines: true,
        })
        .add_plugins(sandbox_ui::audio::plugin)
        .insert_resource(Audio::new(sounds::sounds()))
        .insert_resource(pinball)
        .add_systems(Update, play)
        .run();
}

#[allow(clippy::too_many_arguments)]
fn play(
    mut pinball: ResMut<Pinball>,
    mut audio: ResMut<Audio>,
    mut pixel: ResMut<PixelCanvas>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    pointers: Res<Pointers>,
    orientation: Res<Orientation>,
    time: Res<Time>,
) {
    let Pinball {
        game,
        landscape,
        portrait,
        carry,
    } = &mut *pinball;
    let layout = match *orientation {
        Orientation::Landscape => &*landscape,
        Orientation::Portrait => &*portrait,
    };

    let mut events: Vec<Event> = keys
        .get_just_pressed()
        .filter_map(|&code| key_for(code))
        .map(Event::Key)
        .collect();
    events.extend(pointers.pressed.iter().map(|p| Event::Press {
        id: p.id,
        pos: p.pos,
    }));
    events.extend(
        pointers
            .released
            .iter()
            .map(|p| Event::Release { id: p.id }),
    );
    let held = Held {
        left: keys.any_pressed(LEFT_KEYS),
        right: keys.any_pressed(RIGHT_KEYS),
        plunger: keys.any_pressed(PLUNGER_KEYS),
    };

    let mut outcomes: Vec<Outcome> = events
        .into_iter()
        .map(|event| game.handle(event, layout))
        .collect();
    for _ in 0..sandbox_ui::clock::steps_due(carry, time.delta_secs()) {
        outcomes.push(game.step(held));
    }
    audio.muted = !game.sound_on;
    for name in game.take_sounds() {
        audio.play(name);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_flippers_plunger_and_screens() {
        assert_eq!(key_for(KeyCode::KeyZ), Some(Key::Left));
        assert_eq!(key_for(KeyCode::Slash), Some(Key::Right));
        assert_eq!(key_for(KeyCode::Space), Some(Key::Plunger));
        assert_eq!(key_for(KeyCode::Enter), Some(Key::Start));
        assert_eq!(key_for(KeyCode::KeyS), Some(Key::Sound));
        assert_eq!(key_for(KeyCode::KeyQ), None);
    }
}
