//! Pinball rules with no engine code: a ball under gravity on a walled
//! table, two flippers, a plunger lane, pop bumpers, slingshots and three
//! rollover lanes. All speeds are per 1/60 s step; positions are table
//! pixels, origin top-left, `y` growing downwards.
//!
//! Walls are line segments and the ball a circle; it moves in substeps
//! short enough that it cannot pass through a wall or a swinging flipper.

use std::f32::consts::PI;
use std::ops::{Add, AddAssign, Mul, Sub};

pub const WIDTH: f32 = 300.0;
pub const HEIGHT: f32 = 480.0;
pub const BALL_RADIUS: f32 = 5.0;
pub const GRAVITY: f32 = 0.2;
pub const MAX_SPEED: f32 = 15.0;
/// Speed kept each step (air and rolling friction).
const DRAG: f32 = 0.999;
/// The ball and the flippers move at most this far between contact checks.
const MAX_SUBSTEP: f32 = 2.0;
const WALL_BOUNCE: f32 = 0.45;
const FLIPPER_BOUNCE: f32 = 0.3;
/// Slower contacts than this do not bounce, so the ball can rest and roll.
const REST_SPEED: f32 = 0.6;

pub const FLIPPER_LENGTH: f32 = 46.0;
pub const FLIPPER_RADIUS: f32 = 4.0;
/// Resting angle below horizontal, and raised angle above it (radians).
pub const FLIPPER_REST: f32 = 0.5;
pub const FLIPPER_UP: f32 = 0.45;
/// Radians a flipper turns per step.
pub const FLIPPER_SPEED: f32 = 0.3;
pub const LEFT_PIVOT: (f32, f32) = (80.0, 428.0);
pub const RIGHT_PIVOT: (f32, f32) = (196.0, 428.0);

/// The plunger lane runs between this wall and the right edge.
pub const LANE_WALL_X: f32 = 276.0;
pub const PLUNGER_Y: f32 = 460.0;
pub const PLUNGER_X: f32 = 288.0;
/// Steps to pull the plunger all the way back.
pub const PLUNGER_STEPS: f32 = 45.0;
/// Launch speed with the plunger barely and fully pulled.
pub const LAUNCH_MIN: f32 = 10.0;
pub const LAUNCH_MAX: f32 = 15.0;

pub const BUMPER_RADIUS: f32 = 14.0;
pub const BUMPERS: [(f32, f32); 3] = [(88.0, 150.0), (188.0, 150.0), (138.0, 205.0)];
pub const BUMPER_KICK: f32 = 6.0;
pub const SLING_KICK: f32 = 4.0;
/// Rollover lane centres, and the band they sense the ball in.
pub const LANES: [f32; 3] = [78.0, 138.0, 198.0];
pub const LANE_TOP: f32 = 30.0;
pub const LANE_BOTTOM: f32 = 66.0;
pub const LANE_HALF_WIDTH: f32 = 12.0;
/// Steps a bumper or slingshot stays lit after a hit.
pub const FLASH_STEPS: u32 = 8;

pub const START_BALLS: u32 = 3;
pub const MAX_MULTIPLIER: u32 = 5;
pub const BUMPER_POINTS: u32 = 100;
pub const SLING_POINTS: u32 = 10;
pub const LANE_POINTS: u32 = 50;
pub const LANES_BONUS: u32 = 1000;

pub type Line = ((f32, f32), (f32, f32));

/// The table's fixed walls.
pub const WALLS: [Line; 13] = [
    ((50.0, 0.0), (240.0, 0.0)),      // top
    ((0.0, 50.0), (50.0, 0.0)),       // top-left corner
    ((240.0, 0.0), (300.0, 60.0)),    // top-right corner: turns a launch in
    ((0.0, 50.0), (0.0, 350.0)),      // left side
    ((0.0, 350.0), (76.0, 422.0)),    // left inlane, down to the flipper
    ((300.0, 60.0), (300.0, 460.0)),  // right side
    ((276.0, 120.0), (276.0, 460.0)), // plunger lane wall
    ((276.0, 350.0), (200.0, 422.0)), // right inlane
    ((276.0, 460.0), (300.0, 460.0)), // plunger
    ((48.0, 26.0), (48.0, 70.0)),     // posts between the rollover lanes
    ((108.0, 26.0), (108.0, 70.0)),
    ((168.0, 26.0), (168.0, 70.0)),
    ((228.0, 26.0), (228.0, 70.0)),
];
/// Closes the plunger lane once the ball has left it.
pub const GATE: Line = ((276.0, 36.0), (276.0, 120.0));
/// The kicking faces above the flippers.
pub const SLINGS: [Line; 2] = [
    ((34.0, 318.0), (64.0, 388.0)),
    ((242.0, 318.0), (212.0, 388.0)),
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// Turned a quarter turn: the velocity of a point at this offset from a
    /// pivot turning at one radian per step.
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from((x, y): (f32, f32)) -> Self {
        Vec2::new(x, y)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        *self = *self + other;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f32) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

/// The point of segment `a`-`b` nearest to `p`.
pub fn closest_point(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let length2 = ab.dot(ab);
    if length2 == 0.0 {
        return a;
    }
    let t = ((p - a).dot(ab) / length2).clamp(0.0, 1.0);
    a + ab * t
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flipper {
    pub pivot: Vec2,
    pub rest: f32,
    pub up: f32,
    /// Current angle from the pivot to the tip (radians, `y` down).
    pub angle: f32,
    pub raised: bool,
}

impl Flipper {
    pub fn left() -> Self {
        Self::new(LEFT_PIVOT, FLIPPER_REST, -FLIPPER_UP)
    }

    pub fn right() -> Self {
        Self::new(RIGHT_PIVOT, PI - FLIPPER_REST, PI + FLIPPER_UP)
    }

    fn new(pivot: (f32, f32), rest: f32, up: f32) -> Self {
        Self {
            pivot: pivot.into(),
            rest,
            up,
            angle: rest,
            raised: false,
        }
    }

    pub fn tip(&self) -> Vec2 {
        self.pivot + Vec2::new(self.angle.cos(), self.angle.sin()) * FLIPPER_LENGTH
    }

    fn target(&self) -> f32 {
        if self.raised { self.up } else { self.rest }
    }

    /// Radians still to turn this step, at most `FLIPPER_SPEED`.
    fn swing(&self) -> f32 {
        (self.target() - self.angle).clamp(-FLIPPER_SPEED, FLIPPER_SPEED)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl Ball {
    /// A ball at rest on the plunger.
    pub fn on_plunger() -> Self {
        Self {
            pos: Vec2::new(PLUNGER_X, PLUNGER_Y - BALL_RADIUS),
            vel: Vec2::default(),
        }
    }
}

/// What the controls hold during a step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub left: bool,
    pub right: bool,
    pub plunger: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Nothing,
    /// All three lanes lit: the multiplier went up.
    Bonus,
    /// The ball drained; the next one waits on the plunger.
    Drained,
    /// The last ball drained.
    GameOver,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub ball: Ball,
    /// The ball is in the plunger lane (its gate is open).
    pub in_lane: bool,
    /// How far the plunger is pulled, 0 to 1.
    pub plunger: f32,
    pub flippers: [Flipper; 2],
    /// Lit rollover lanes.
    pub lanes: [bool; 3],
    pub multiplier: u32,
    pub score: u32,
    /// Balls left, the one in play included.
    pub balls: u32,
    /// Steps each bumper and slingshot stays lit.
    pub bumper_flash: [u32; 3],
    pub sling_flash: [u32; 2],
    /// The lane the ball is passing through, so each pass counts once.
    lane_inside: Option<usize>,
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

impl Field {
    pub fn new() -> Self {
        Self {
            ball: Ball::on_plunger(),
            in_lane: true,
            plunger: 0.0,
            flippers: [Flipper::left(), Flipper::right()],
            lanes: [false; 3],
            multiplier: 1,
            score: 0,
            balls: START_BALLS,
            bumper_flash: [0; 3],
            sling_flash: [0; 2],
            lane_inside: None,
        }
    }

    /// The ball sits still on the plunger, ready to launch.
    pub fn ball_ready(&self) -> bool {
        self.in_lane
            && self.ball.pos.y >= PLUNGER_Y - BALL_RADIUS - 1.0
            && self.ball.vel.length() < 1.0
    }

    /// Shift the lit lanes one place left (`-1`) or right (`1`), as the
    /// flipper buttons do on real tables.
    pub fn rotate_lanes(&mut self, direction: i32) {
        if direction < 0 {
            self.lanes.rotate_left(1);
        } else {
            self.lanes.rotate_right(1);
        }
    }

    fn add(&mut self, points: u32) {
        self.score += points * self.multiplier;
    }

    /// One 1/60 s step.
    pub fn step(&mut self, held: Held) -> Event {
        if self.balls == 0 {
            return Event::GameOver;
        }
        self.flippers[0].raised = held.left;
        self.flippers[1].raised = held.right;
        for flash in self.bumper_flash.iter_mut().chain(&mut self.sling_flash) {
            *flash = flash.saturating_sub(1);
        }

        if self.ball_ready() {
            if held.plunger {
                self.plunger = (self.plunger + 1.0 / PLUNGER_STEPS).min(1.0);
                self.ball = Ball::on_plunger();
                self.swing_flippers();
                return Event::Nothing;
            }
            if self.plunger > 0.0 {
                let speed = LAUNCH_MIN + (LAUNCH_MAX - LAUNCH_MIN) * self.plunger;
                self.ball.vel = Vec2::new(0.0, -speed);
                self.plunger = 0.0;
            }
        } else if !held.plunger {
            self.plunger = 0.0;
        }

        self.ball.vel = self.ball.vel * DRAG + Vec2::new(0.0, GRAVITY);
        self.limit_speed();
        let travel = self
            .flippers
            .iter()
            .map(|f| f.swing().abs() * FLIPPER_LENGTH)
            .fold(self.ball.vel.length(), f32::max);
        let parts = (travel / MAX_SUBSTEP).ceil().max(1.0);
        let swings = self.flippers.map(|f| f.swing());
        let mut event = Event::Nothing;
        for _ in 0..parts as u32 {
            if let Some(happened) = self.substep(parts, swings) {
                event = happened;
                if event != Event::Bonus {
                    return event;
                }
            }
        }
        self.limit_speed();
        if !self.ball.pos.is_finite() {
            // Never expected; keep the game going rather than lose the ball.
            self.ball = Ball::on_plunger();
            self.in_lane = true;
        }
        event
    }

    /// Turn the flippers a whole step without moving the ball.
    fn swing_flippers(&mut self) {
        for flipper in &mut self.flippers {
            flipper.angle += flipper.swing();
        }
    }

    fn limit_speed(&mut self) {
        let speed = self.ball.vel.length();
        if speed > MAX_SPEED {
            self.ball.vel = self.ball.vel * (MAX_SPEED / speed);
        }
    }

    fn substep(&mut self, parts: f32, swings: [f32; 2]) -> Option<Event> {
        for (flipper, swing) in self.flippers.iter_mut().zip(swings) {
            flipper.angle += swing / parts;
        }
        self.ball.pos += self.ball.vel * (1.0 / parts);

        for &wall in &WALLS {
            self.hit_wall(wall, WALL_BOUNCE);
        }
        if !self.in_lane {
            self.hit_wall(GATE, WALL_BOUNCE);
        }
        for (i, &sling) in SLINGS.iter().enumerate() {
            if self.hit_wall(sling, WALL_BOUNCE) {
                self.kick_off(sling, SLING_KICK);
                self.sling_flash[i] = FLASH_STEPS;
                self.add(SLING_POINTS);
            }
        }
        for i in 0..BUMPERS.len() {
            if self.hit_bumper(i) {
                self.bumper_flash[i] = FLASH_STEPS;
                self.add(BUMPER_POINTS);
            }
        }
        for (i, swing) in swings.into_iter().enumerate() {
            self.hit_flipper(i, swing);
        }

        if self.in_lane && self.ball.pos.x < LANE_WALL_X - BALL_RADIUS {
            self.in_lane = false;
        }
        if self.ball.pos.y > HEIGHT + BALL_RADIUS {
            return Some(self.drain());
        }
        self.roll_over_lanes().then_some(Event::Bonus)
    }

    /// Push the ball out of a wall and bounce it; true if it was moving into
    /// the wall (a real hit, not resting contact).
    fn hit_wall(&mut self, (a, b): Line, bounce: f32) -> bool {
        let contact = closest_point(self.ball.pos, a.into(), b.into());
        self.contact(contact, BALL_RADIUS, Vec2::default(), bounce)
    }

    /// Resolve a contact with a surface point moving at `surface` velocity,
    /// the ball's centre to be kept `distance` away from it.
    fn contact(&mut self, point: Vec2, distance: f32, surface: Vec2, bounce: f32) -> bool {
        let offset = self.ball.pos - point;
        let gap = offset.length();
        if gap >= distance || gap == 0.0 {
            return false;
        }
        let normal = offset * (1.0 / gap);
        self.ball.pos = point + normal * distance;
        let relative = self.ball.vel - surface;
        let into = relative.dot(normal);
        if into >= 0.0 {
            return false;
        }
        let bounce = if -into < REST_SPEED { 0.0 } else { bounce };
        self.ball.vel = relative - normal * ((1.0 + bounce) * into) + surface;
        true
    }

    /// Send the ball off a slingshot's face at `speed` at least.
    fn kick_off(&mut self, (a, b): Line, speed: f32) {
        let contact = closest_point(self.ball.pos, a.into(), b.into());
        let offset = self.ball.pos - contact;
        let normal = offset * (1.0 / offset.length().max(f32::EPSILON));
        let away = self.ball.vel.dot(normal);
        if away < speed {
            self.ball.vel += normal * (speed - away);
        }
    }

    fn hit_bumper(&mut self, i: usize) -> bool {
        let center = Vec2::from(BUMPERS[i]);
        let reach = BALL_RADIUS + BUMPER_RADIUS;
        if (self.ball.pos - center).length() >= reach {
            return false;
        }
        self.contact(center, reach, Vec2::default(), WALL_BOUNCE);
        let normal = (self.ball.pos - center) * (1.0 / reach);
        let away = self.ball.vel.dot(normal);
        if away < BUMPER_KICK {
            self.ball.vel += normal * (BUMPER_KICK - away);
        }
        true
    }

    /// Flippers are capsules; a swinging one carries the ball with it.
    fn hit_flipper(&mut self, i: usize, swing: f32) {
        let flipper = self.flippers[i];
        let contact = closest_point(self.ball.pos, flipper.pivot, flipper.tip());
        let surface = (contact - flipper.pivot).perp() * swing;
        self.contact(
            contact,
            BALL_RADIUS + FLIPPER_RADIUS,
            surface,
            FLIPPER_BOUNCE,
        );
    }

    /// Light the lane the ball has just entered; true when that lit all three.
    fn roll_over_lanes(&mut self) -> bool {
        let pos = self.ball.pos;
        let inside = LANES.iter().position(|&x| {
            (pos.x - x).abs() < LANE_HALF_WIDTH && LANE_TOP < pos.y && pos.y < LANE_BOTTOM
        });
        let entered = inside.filter(|_| inside != self.lane_inside);
        self.lane_inside = inside;
        let Some(lane) = entered else {
            return false;
        };
        self.lanes[lane] = true;
        self.add(LANE_POINTS);
        if self.lanes.iter().all(|&lit| lit) {
            self.lanes = [false; 3];
            self.multiplier = (self.multiplier + 1).min(MAX_MULTIPLIER);
            self.add(LANES_BONUS);
            return true;
        }
        false
    }

    fn drain(&mut self) -> Event {
        self.balls -= 1;
        self.multiplier = 1;
        if self.balls == 0 {
            return Event::GameOver;
        }
        self.ball = Ball::on_plunger();
        self.in_lane = true;
        self.plunger = 0.0;
        Event::Drained
    }
}

#[cfg(test)]
mod tests;
