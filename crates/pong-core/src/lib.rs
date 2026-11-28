//! Pong rules (port of `src/game.c` and `include/pong.h` from
//! torresnelson/pong), with no engine code.
//!
//! Positions are in screen pixels of the 800x450 field, speeds in pixels per
//! frame at 60 frames per second, as in the C version.

pub mod computer;

pub const SCREEN_WIDTH: f32 = 800.0;
pub const SCREEN_HEIGHT: f32 = 450.0;
pub const PLAYER_HEIGHT: f32 = 100.0;
pub const PLAYER_WIDTH: f32 = 10.0;
/// Ball radius.
pub const RADIUS: f32 = 10.0;
/// Pixels per frame a human paddle moves.
pub const PADDLE_SPEED: f32 = 15.0;
/// Points that win a match.
pub const MAX_SCORE: u32 = 10;
/// Matches that win the game.
pub const TOTAL_MATCHES: u32 = 3;
pub const MAX_SPEED: Vec2 = Vec2::new(15.0, 8.0);
/// Speed of a serve (towards the right; mirrored towards the left).
pub const SERVE_SPEED: Vec2 = Vec2::new(7.2, 3.25);
/// Highest and lowest `y` of a paddle's top edge.
pub const FIELD_TOP: f32 = 0.0;
pub const FIELD_BOTTOM: f32 = SCREEN_HEIGHT - PLAYER_HEIGHT;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A rectangle with float coordinates (raylib's `Rectangle`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

    /// Half-open, like raylib's `CheckCollisionPointRec`.
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x < self.x + self.w
            && point.y >= self.y
            && point.y < self.y + self.h
    }

    /// Whether the rectangles overlap (raylib's `CheckCollisionRecs`).
    pub fn overlaps(&self, other: &FRect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// Whether a circle touches the rectangle (raylib's `CheckCollisionCircleRec`).
    pub fn touches_circle(&self, center: Vec2, radius: f32) -> bool {
        let nearest_x = center.x.clamp(self.x, self.x + self.w);
        let nearest_y = center.y.clamp(self.y, self.y + self.h);
        let (dx, dy) = (center.x - nearest_x, center.y - nearest_y);
        dx * dx + dy * dy <= radius * radius
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    pub bar: FRect,
    /// Points in the current match.
    pub score: u32,
    /// Matches won.
    pub match_score: u32,
}

impl Player {
    pub const fn new(bar: FRect) -> Self {
        Self {
            bar,
            score: 0,
            match_score: 0,
        }
    }

    /// The left paddle, at its starting place.
    pub const fn left() -> Self {
        Self::new(FRect::new(
            0.0,
            SCREEN_HEIGHT / 2.0,
            PLAYER_WIDTH,
            PLAYER_HEIGHT,
        ))
    }

    /// The right paddle, at its starting place.
    pub const fn right() -> Self {
        Self::new(FRect::new(
            SCREEN_WIDTH - PLAYER_WIDTH,
            SCREEN_HEIGHT / 2.0,
            PLAYER_WIDTH,
            PLAYER_HEIGHT,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub position: Vec2,
    pub speed: Vec2,
}

impl Ball {
    pub const fn new(position: Vec2, speed: Vec2) -> Self {
        Self { position, speed }
    }

    /// The first serve, from the middle towards the right.
    pub const fn serve() -> Self {
        Self::new(
            Vec2::new(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0),
            SERVE_SPEED,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Menu,
    Playing,
    Paused,
    MatchOver,
    GameOver,
}

/// Main menu options, in the order they are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuOption {
    OnePlayer,
    TwoPlayers,
}

impl MenuOption {
    pub const ALL: [MenuOption; 2] = [MenuOption::OnePlayer, MenuOption::TwoPlayers];

    pub fn label(self) -> &'static str {
        match self {
            MenuOption::OnePlayer => "ONE PLAYER",
            MenuOption::TwoPlayers => "TWO PLAYERS",
        }
    }
}

/// Move the main menu cursor by `delta` entries, wrapping around at both ends.
pub fn menu_select(option: MenuOption, delta: i32) -> MenuOption {
    let count = MenuOption::ALL.len() as i32;
    MenuOption::ALL[(option as i32 + delta).rem_euclid(count) as usize]
}

/// On-screen touch controls for player 1 (phones and tablets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchControl {
    Up,
    Down,
    Pause,
}

/// Screen area of each touch control: the left third of the field is split
/// into an up and a down half for player 1, and a small pause button sits
/// top centre.
pub fn touch_control_rect(control: TouchControl) -> FRect {
    match control {
        TouchControl::Up => FRect::new(0.0, 0.0, SCREEN_WIDTH / 3.0, SCREEN_HEIGHT / 2.0),
        TouchControl::Down => FRect::new(
            0.0,
            SCREEN_HEIGHT / 2.0,
            SCREEN_WIDTH / 3.0,
            SCREEN_HEIGHT / 2.0,
        ),
        TouchControl::Pause => FRect::new(SCREEN_WIDTH / 2.0 - 35.0, 0.0, 70.0, 60.0),
    }
}

/// Which touch control a point on the screen presses, if any.
pub fn touch_control_at(point: Vec2) -> Option<TouchControl> {
    [TouchControl::Pause, TouchControl::Up, TouchControl::Down]
        .into_iter()
        .find(|&control| touch_control_rect(control).contains(point))
}

/// Both players back to zero points and zero matches.
pub fn reset_game(player1: &mut Player, player2: &mut Player) {
    for player in [player1, player2] {
        player.score = 0;
        player.match_score = 0;
    }
}

/// Serve again from the middle once the ball has left the field, towards the
/// player who just scored against.
pub fn ball_reset(ball: &mut Ball) {
    let center = Vec2::new(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0);
    if ball.position.x > SCREEN_WIDTH + 2.0 * RADIUS {
        ball.speed = Vec2::new(-SERVE_SPEED.x, -SERVE_SPEED.y);
        ball.position = Vec2::new(center.x - ball.speed.x, center.y + ball.speed.y);
    } else if ball.position.x < -2.0 * RADIUS {
        ball.speed = SERVE_SPEED;
        ball.position = Vec2::new(center.x + ball.speed.x, center.y - ball.speed.y);
    }
}

/// Award a point when the ball has left the field, and serve again.
pub fn eye_ball(player1: &mut Player, player2: &mut Player, ball: &mut Ball) {
    if ball.position.x > SCREEN_WIDTH + 2.0 * RADIUS {
        ball_reset(ball);
        player1.score += 1;
    } else if ball.position.x < -2.0 * RADIUS {
        ball_reset(ball);
        player2.score += 1;
    }
}

/// Award a match to whoever reached `max_score` and return the resulting state.
pub fn score_manager(
    player1: &mut Player,
    player2: &mut Player,
    max_score: u32,
    total_matches: u32,
) -> State {
    let winner = if player1.score >= max_score {
        &mut *player1
    } else if player2.score >= max_score {
        &mut *player2
    } else {
        return State::Playing;
    };
    winner.match_score += 1;
    let matches = winner.match_score;
    player1.score = 0;
    player2.score = 0;
    if matches >= total_matches {
        State::GameOver
    } else {
        State::MatchOver
    }
}

/// Keep each speed component within the given maximum magnitude.
pub fn clamp_speed(speed: Vec2, max: Vec2) -> Vec2 {
    Vec2::new(speed.x.clamp(-max.x, max.x), speed.y.clamp(-max.y, max.y))
}

/// Send the ball back with a random pitch, capped at `MAX_SPEED`.
pub fn paddle_bounce(ball: &mut Ball, rng: &mut fastrand::Rng) {
    let pitch = rng.i32(4..=7) as f32 * 0.1 + 0.5;
    ball.speed.x *= -pitch;
    ball.speed.y *= pitch;
    ball.speed = clamp_speed(ball.speed, MAX_SPEED);
}

pub fn ball_movement(ball: &mut Ball) {
    ball.position.x += ball.speed.x;
    ball.position.y += ball.speed.y;
}

/// Bounce the ball off the roof, the floor and the paddles, and keep the
/// paddles inside the field.
pub fn collisions(
    player1: &mut Player,
    player2: &mut Player,
    ball: &mut Ball,
    rng: &mut fastrand::Rng,
) {
    // Bounce off roof and floor, keeping the ball inside the field.
    if ball.position.y >= SCREEN_HEIGHT - RADIUS {
        ball.position.y = SCREEN_HEIGHT - RADIUS;
        ball.speed.y = -ball.speed.y.abs();
    } else if ball.position.y <= RADIUS {
        ball.position.y = RADIUS;
        ball.speed.y = ball.speed.y.abs();
    }

    // Bounce off a paddle only while moving towards it and still in front of
    // it, then push the ball out so it cannot bounce again on the next frame.
    let (left, right) = (player1.bar, player2.bar);
    if ball.speed.x < 0.0
        && ball.position.x > left.x + left.w / 2.0
        && left.touches_circle(ball.position, RADIUS)
    {
        ball.position.x = left.x + left.w + RADIUS;
        paddle_bounce(ball, rng);
    } else if ball.speed.x > 0.0
        && ball.position.x < right.x + right.w / 2.0
        && right.touches_circle(ball.position, RADIUS)
    {
        ball.position.x = right.x - RADIUS;
        paddle_bounce(ball, rng);
    }

    for player in [player1, player2] {
        player.bar.y = player.bar.y.clamp(FIELD_TOP, FIELD_BOTTOM);
    }
}

/// One frame of play: score, bounce, move. Returns the state that follows.
pub fn referee(
    player1: &mut Player,
    player2: &mut Player,
    ball: &mut Ball,
    rng: &mut fastrand::Rng,
) -> State {
    eye_ball(player1, player2, ball);
    let state = score_manager(player1, player2, MAX_SCORE, TOTAL_MATCHES);
    collisions(player1, player2, ball, rng);
    ball_movement(ball);
    state
}

#[cfg(test)]
mod tests;
