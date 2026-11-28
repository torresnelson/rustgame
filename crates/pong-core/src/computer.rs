//! Computer opponent (port of `src/computer.c`). It plays the right paddle
//! and gets a little better as the game goes on: it reacts sooner, moves
//! faster, aims better and, from `PREDICT_LEVEL` on, predicts where the ball
//! will land instead of chasing it.

use crate::{Ball, Player, RADIUS, SCREEN_HEIGHT, SCREEN_WIDTH, Vec2};

pub const MAX_LEVEL: u32 = 10;
pub const PREDICT_LEVEL: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Computer {
    pub level: u32,
    /// Where it aims, relative to the ball, for the current approach.
    pub aim_offset: f32,
    /// Whether it is following the ball on the current approach.
    pub tracking: bool,
}

impl Default for Computer {
    fn default() -> Self {
        Self {
            level: 1,
            aim_offset: 0.0,
            tracking: false,
        }
    }
}

/// Two levels per match already played plus one every four points of the
/// current match.
pub fn computer_level(human: &Player, cpu: &Player) -> u32 {
    let matches = human.match_score + cpu.match_score;
    let points = human.score + cpu.score;
    (1 + 2 * matches + points / 4).min(MAX_LEVEL)
}

/// Paddle speed in pixels per frame, always slower than the human (15).
pub fn computer_speed(level: u32) -> f32 {
    3.0 + 0.8 * level as f32
}

/// Ball `x` position from which the computer starts to follow the ball.
pub fn computer_reaction_x(level: u32) -> f32 {
    SCREEN_WIDTH * (0.75 - 0.05 * level as f32)
}

/// Maximum distance in pixels between where the computer aims and the ball.
pub fn computer_aim_error(level: u32) -> i32 {
    (MAX_LEVEL.saturating_sub(level) * 12) as i32
}

/// `y` of the ball when it reaches `target_x`, folding in roof and floor
/// bounces.
pub fn predict_ball_y(position: Vec2, speed: Vec2, target_x: f32, radius: f32) -> f32 {
    if speed.x <= 0.0 {
        return position.y;
    }
    let low = radius;
    let span = SCREEN_HEIGHT - 2.0 * radius;
    let mut y = (position.y + speed.y * (target_x - position.x) / speed.x - low) % (2.0 * span);
    if y < 0.0 {
        y += 2.0 * span;
    }
    if y > span {
        y = 2.0 * span - y;
    }
    low + y
}

/// Move from `from` towards `to` by at most `max_step`.
pub fn step_toward(from: f32, to: f32, max_step: f32) -> f32 {
    from + (to - from).clamp(-max_step, max_step)
}

/// Move the computer's paddle (`cpu`) for one frame.
pub fn computer_update(
    computer: &mut Computer,
    cpu: &mut Player,
    human: &Player,
    ball: &Ball,
    rng: &mut fastrand::Rng,
) {
    computer.level = computer_level(human, cpu);
    let center = cpu.bar.y + cpu.bar.h / 2.0;
    let mut target = center;

    if ball.speed.x <= 0.0 {
        // Ball going away: relax back to the middle of the field.
        computer.tracking = false;
        target = SCREEN_HEIGHT / 2.0;
    } else if ball.position.x >= computer_reaction_x(computer.level) {
        if !computer.tracking {
            // Pick a new aim error once per approach so the paddle does not jitter.
            let error = computer_aim_error(computer.level);
            computer.aim_offset = rng.i32(-error..=error) as f32;
            computer.tracking = true;
        }
        let y = if computer.level >= PREDICT_LEVEL {
            predict_ball_y(ball.position, ball.speed, cpu.bar.x - RADIUS, RADIUS)
        } else {
            ball.position.y
        };
        target = y + computer.aim_offset;
    }

    cpu.bar.y += step_toward(center, target, computer_speed(computer.level)) - center;
}
