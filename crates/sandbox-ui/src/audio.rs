//! Sound effects: a game gives [`Audio`] its named sounds (samples from the
//! `sfx` crate) and calls [`Audio::play`]; [`plugin`] plays what was queued
//! at the end of each frame.
//!
//! On the desktop each sound becomes a bevy `AudioSource` (WAV bytes). In
//! the browser bevy's audio is left out of the build: browsers only allow
//! sound after a tap or key press (iOS only inside its handler), so the web
//! page makes an `AudioContext` there (`window.gameAudio`, see
//! `web/game.html`) and the sounds are played into it with Web Audio.

use bevy::prelude::*;

/// The page's `AudioContext`, made on the first tap or key press.
pub const WEB_AUDIO_CONTEXT: &str = "gameAudio";

/// A game's sounds and the ones to play this frame.
#[derive(Resource, Debug, Default)]
pub struct Audio {
    sounds: Vec<(&'static str, Vec<f32>)>,
    queue: Vec<usize>,
    /// Queued sounds are dropped instead of played.
    pub muted: bool,
}

impl Audio {
    /// Sounds by name, as mono samples at `sfx::SAMPLE_RATE`.
    pub fn new(sounds: Vec<(&'static str, Vec<f32>)>) -> Self {
        Self {
            sounds,
            ..default()
        }
    }

    /// Play the sound called `name` at the end of the frame.
    pub fn play(&mut self, name: &str) {
        let index = self.sounds.iter().position(|(known, _)| *known == name);
        debug_assert!(index.is_some(), "no sound called {name:?}");
        self.queue.extend(index);
    }

    /// The queued sounds, emptied; none when muted.
    fn take_queue(&mut self) -> Vec<usize> {
        let queue = std::mem::take(&mut self.queue);
        if self.muted { Vec::new() } else { queue }
    }
}

/// Plays queued sounds; the game inserts its [`Audio`].
pub fn plugin(app: &mut App) {
    app.add_systems(Last, platform::play);
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings};
    use bevy::prelude::*;

    use super::Audio;

    pub fn play(
        mut commands: Commands,
        mut audio: ResMut<Audio>,
        mut sources: ResMut<Assets<AudioSource>>,
        mut handles: Local<Vec<Handle<AudioSource>>>,
    ) {
        let queue = audio.take_queue();
        if queue.is_empty() {
            return;
        }
        if handles.len() != audio.sounds.len() {
            *handles = audio
                .sounds
                .iter()
                .map(|(_, samples)| {
                    sources.add(AudioSource {
                        bytes: sfx::wav_bytes(samples).into(),
                    })
                })
                .collect();
        }
        for index in queue {
            commands.spawn((
                AudioPlayer(handles[index].clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use std::cell::RefCell;

    use bevy::prelude::*;
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{AudioBuffer, AudioContext};

    use super::{Audio, WEB_AUDIO_CONTEXT};

    thread_local! {
        // Web Audio objects are not `Send`; the browser runs the app on one
        // thread.
        static BUFFERS: RefCell<Option<(AudioContext, Vec<AudioBuffer>)>> =
            const { RefCell::new(None) };
    }

    fn context() -> Option<AudioContext> {
        let window = web_sys::window()?;
        let context = js_sys::Reflect::get(&window, &WEB_AUDIO_CONTEXT.into()).ok()?;
        context.dyn_into().ok()
    }

    fn buffers(context: &AudioContext, audio: &Audio) -> Option<Vec<AudioBuffer>> {
        audio
            .sounds
            .iter()
            .map(|(_, samples)| {
                let length = samples.len().max(1) as u32;
                let buffer = context
                    .create_buffer(1, length, sfx::SAMPLE_RATE as f32)
                    .ok()?;
                buffer.copy_to_channel(samples, 0).ok()?;
                Some(buffer)
            })
            .collect()
    }

    fn start(context: &AudioContext, buffer: &AudioBuffer) -> Result<(), JsValue> {
        let source = context.create_buffer_source()?;
        source.set_buffer(Some(buffer));
        source.connect_with_audio_node(&context.destination())?;
        source.start()
    }

    pub fn play(mut audio: ResMut<Audio>) {
        let queue = audio.take_queue();
        if queue.is_empty() {
            return;
        }
        BUFFERS.with_borrow_mut(|state| {
            if state.is_none() {
                // No context before the first tap or key press: the sound is
                // dropped, as the browser would not play it anyway.
                *state = context().and_then(|context| {
                    let buffers = buffers(&context, &audio)?;
                    Some((context, buffers))
                });
            }
            if let Some((context, buffers)) = state {
                for index in queue {
                    if let Err(error) = start(context, &buffers[index]) {
                        warn!("could not play a sound: {error:?}");
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio() -> Audio {
        Audio::new(vec![("ding", vec![0.0; 4]), ("buzz", vec![0.5; 2])])
    }

    #[test]
    fn play_queues_sounds_by_name_in_order() {
        let mut audio = audio();
        audio.play("buzz");
        audio.play("ding");
        audio.play("buzz");
        assert_eq!(audio.take_queue(), [1, 0, 1]);
        assert!(audio.take_queue().is_empty(), "played once");
    }

    #[test]
    fn muted_audio_drops_what_was_queued() {
        let mut audio = audio();
        audio.muted = true;
        audio.play("ding");
        assert!(audio.take_queue().is_empty());
        audio.muted = false;
        audio.play("ding");
        assert_eq!(audio.take_queue(), [0]);
    }
}
