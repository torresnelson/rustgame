//! Brick breaker rules with no engine code: a paddle at the bottom of a
//! walled field keeps a ball in play, the ball breaks the bricks it hits,
//! and clearing every breakable brick moves on to the next level. All
//! speeds are per 1/60 s step; positions are field pixels, origin top-left.

pub const WIDTH: f32 = 360.0;
pub const HEIGHT: f32 = 480.0;
pub const BRICK_COLUMNS: usize = 10;
pub const BRICK_W: f32 = WIDTH / BRICK_COLUMNS as f32;
pub const BRICK_H: f32 = 14.0;
pub const BRICKS_TOP: f32 = 48.0;
pub const PADDLE_W: f32 = 64.0;
pub const PADDLE_H: f32 = 8.0;
pub const PADDLE_Y: f32 = 440.0;
/// Paddle speed for held keys.
pub const PADDLE_SPEED: f32 = 7.0;
/// The ball is a square this wide.
pub const BALL: f32 = 6.0;
pub const START_LIVES: u32 = 3;
/// The paddle's edge sends the ball off this far from straight up (radians).
pub const MAX_BOUNCE: f32 = 1.05;
/// The ball moves at most this far between collision checks.
const MAX_SUBSTEP: f32 = 3.0;
pub const POINTS_PER_HIT: u32 = 10;

/// Levels, one string per brick row: `.` empty, `1` to `3` a brick that
/// takes that many hits, `#` a brick that never breaks.
pub const LEVELS: [&[&str]; 4] = [
    &[
        "..........",
        "1111111111",
        "1111111111",
        "1111111111",
        "1111111111",
    ],
    &[
        "....22....",
        "...2112...",
        "..211112..",
        ".21111112.",
        "2111111112",
    ],
    &[
        "3333333333",
        "2.2.2.2.2.",
        ".1.1.1.1.1",
        "##..##..##",
        "1111111111",
    ],
    &[
        "3..3333..3",
        "3..2222..3",
        "#..1111..#",
        "2222..2222",
        "1111111111",
        "..........",
        "##......##",
    ],
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl FRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn overlaps(&self, other: &FRect) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brick {
    pub column: usize,
    pub row: usize,
    /// Hits still needed (0 once broken).
    pub hits: u8,
    pub solid: bool,
}

impl Brick {
    pub fn rect(&self) -> FRect {
        FRect::new(
            self.column as f32 * BRICK_W,
            BRICKS_TOP + self.row as f32 * BRICK_H,
            BRICK_W,
            BRICK_H,
        )
    }

    pub fn standing(&self) -> bool {
        self.solid || self.hits > 0
    }
}

/// The bricks of a level drawn as text.
pub fn parse_level(rows: &[&str]) -> Vec<Brick> {
    rows.iter()
        .enumerate()
        .flat_map(|(row, line)| {
            line.chars()
                .take(BRICK_COLUMNS)
                .enumerate()
                .filter_map(move |(column, char)| {
                    let (hits, solid) = match char {
                        '1'..='3' => (char as u8 - b'0', false),
                        '#' => (0, true),
                        _ => return None,
                    };
                    Some(Brick {
                        column,
                        row,
                        hits,
                        solid,
                    })
                })
        })
        .collect()
}

/// Ball speed in pixels per step: faster every round, up to a limit.
pub fn ball_speed(round: u32) -> f32 {
    (4.0 + 0.5 * round as f32).min(8.0)
}

/// The square ball, by its top-left corner and velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

impl Ball {
    pub fn rect(&self) -> FRect {
        FRect::new(self.x, self.y, BALL, BALL)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Nothing,
    /// The ball fell past the paddle; it waits on the paddle again.
    LifeLost,
    /// Every breakable brick is gone; the next level is laid out.
    Cleared,
    /// The last life was lost.
    GameOver,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Levels cleared so far (the level shown is `round % LEVELS.len()`).
    pub round: u32,
    pub bricks: Vec<Brick>,
    /// The paddle's left edge.
    pub paddle_x: f32,
    pub ball: Ball,
    /// The ball rides on the paddle until launched.
    pub stuck: bool,
    pub lives: u32,
    pub score: u32,
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

impl Field {
    pub fn new() -> Self {
        let mut field = Self {
            round: 0,
            bricks: Vec::new(),
            paddle_x: (WIDTH - PADDLE_W) / 2.0,
            ball: Ball {
                x: 0.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
            },
            stuck: true,
            lives: START_LIVES,
            score: 0,
        };
        field.lay_out();
        field
    }

    pub fn level(&self) -> usize {
        self.round as usize % LEVELS.len()
    }

    pub fn paddle(&self) -> FRect {
        FRect::new(self.paddle_x, PADDLE_Y, PADDLE_W, PADDLE_H)
    }

    /// Breakable bricks still standing.
    pub fn bricks_left(&self) -> usize {
        self.bricks
            .iter()
            .filter(|b| !b.solid && b.hits > 0)
            .count()
    }

    fn lay_out(&mut self) {
        self.bricks = parse_level(LEVELS[self.level()]);
        self.paddle_x = (WIDTH - PADDLE_W) / 2.0;
        self.catch_ball();
    }

    /// Put the ball back on the paddle.
    fn catch_ball(&mut self) {
        self.stuck = true;
        self.ball.vx = 0.0;
        self.ball.vy = 0.0;
        self.ride();
    }

    /// The ball sits on the paddle's middle.
    fn ride(&mut self) {
        self.ball.x = self.paddle_x + (PADDLE_W - BALL) / 2.0;
        self.ball.y = PADDLE_Y - BALL;
    }

    /// Move the paddle by `dx`, kept inside the walls.
    pub fn move_paddle(&mut self, dx: f32) {
        self.paddle_x = (self.paddle_x + dx).clamp(0.0, WIDTH - PADDLE_W);
        if self.stuck {
            self.ride();
        }
    }

    /// Centre the paddle on `x` (as far as the walls allow).
    pub fn paddle_to(&mut self, x: f32) {
        self.move_paddle(x - PADDLE_W / 2.0 - self.paddle_x);
    }

    /// Send the ball up from the paddle, a little to the right.
    pub fn launch(&mut self) {
        if !self.stuck {
            return;
        }
        self.stuck = false;
        self.set_heading(0.35);
    }

    /// Ball speed for this round, at `angle` radians from straight up.
    fn set_heading(&mut self, angle: f32) {
        let speed = ball_speed(self.round);
        self.ball.vx = speed * angle.sin();
        self.ball.vy = -speed * angle.cos();
    }

    /// One 1/60 s step.
    pub fn step(&mut self) -> Event {
        if self.stuck {
            self.ride();
            return Event::Nothing;
        }
        let speed = self.ball.vx.hypot(self.ball.vy);
        let parts = (speed / MAX_SUBSTEP).ceil().max(1.0);
        for _ in 0..parts as u32 {
            self.substep(parts);
            if self.ball.y > HEIGHT {
                self.lives -= 1;
                if self.lives == 0 {
                    return Event::GameOver;
                }
                self.catch_ball();
                return Event::LifeLost;
            }
        }
        if self.bricks_left() == 0 {
            self.round += 1;
            self.lay_out();
            return Event::Cleared;
        }
        Event::Nothing
    }

    fn substep(&mut self, parts: f32) {
        let (dx, dy) = (self.ball.vx / parts, self.ball.vy / parts);

        self.ball.x += dx;
        if self.ball.x < 0.0 {
            self.ball.x = -self.ball.x;
            self.ball.vx = self.ball.vx.abs();
        } else if self.ball.x + BALL > WIDTH {
            self.ball.x = 2.0 * (WIDTH - BALL) - self.ball.x;
            self.ball.vx = -self.ball.vx.abs();
        }
        if self.hit_brick() {
            self.ball.x -= dx;
            self.ball.vx = -self.ball.vx;
        }

        self.ball.y += dy;
        if self.ball.y < 0.0 {
            self.ball.y = -self.ball.y;
            self.ball.vy = self.ball.vy.abs();
        }
        if self.hit_brick() {
            self.ball.y -= dy;
            self.ball.vy = -self.ball.vy;
        }

        // The paddle only catches a falling ball that was above it.
        let was_above = self.ball.y + BALL - dy <= PADDLE_Y + 0.01;
        if self.ball.vy > 0.0 && was_above && self.ball.rect().overlaps(&self.paddle()) {
            self.bounce();
        }
    }

    /// Damage the first brick the ball overlaps; true if it hit one.
    fn hit_brick(&mut self) -> bool {
        let ball = self.ball.rect();
        let Some(brick) = self
            .bricks
            .iter_mut()
            .find(|b| b.standing() && b.rect().overlaps(&ball))
        else {
            return false;
        };
        if !brick.solid {
            brick.hits -= 1;
            self.score += POINTS_PER_HIT;
        }
        true
    }

    /// Off the paddle: the further from its middle, the wider the angle.
    fn bounce(&mut self) {
        let ball_middle = self.ball.x + BALL / 2.0;
        let paddle_middle = self.paddle_x + PADDLE_W / 2.0;
        let offset = ((ball_middle - paddle_middle) / ((PADDLE_W + BALL) / 2.0)).clamp(-1.0, 1.0);
        self.set_heading(offset * MAX_BOUNCE);
        self.ball.y = PADDLE_Y - BALL;
    }
}

#[cfg(test)]
mod tests;
