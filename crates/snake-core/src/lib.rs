//! Snake rules with no engine code: a snake on a walled grid eats food,
//! grows a cell per meal and dies when it runs into a wall or itself.
//! Filling the whole grid wins.

use std::collections::VecDeque;

pub const COLUMNS: i32 = 20;
pub const ROWS: i32 = 20;
pub const START_LENGTH: i32 = 3;
/// Turns queued ahead of the snake, so two quick presses take a corner.
pub const MAX_QUEUED: usize = 2;
/// Step time at the start and the fastest it gets, in milliseconds.
pub const START_STEP_MS: u64 = 160;
pub const FASTEST_STEP_MS: u64 = 70;
/// Each meal takes this much off the step time.
pub const SPEEDUP_MS: u64 = 4;

/// A grid cell as (column, row).
pub type Cell = (i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// (columns, rows) moved by one step.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Running,
    Dead,
    /// The snake fills the grid.
    Won,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snake {
    /// Head first.
    pub body: VecDeque<Cell>,
    pub heading: Direction,
    queued: VecDeque<Direction>,
    pub food: Option<Cell>,
    /// Meals eaten.
    pub score: u32,
    pub status: Status,
    /// Cells still to grow.
    grow: u32,
}

pub fn in_bounds((x, y): Cell) -> bool {
    (0..COLUMNS).contains(&x) && (0..ROWS).contains(&y)
}

/// A random free cell for the food, or `None` when the snake fills the grid.
pub fn place_food(body: &VecDeque<Cell>, rng: &mut fastrand::Rng) -> Option<Cell> {
    let free: Vec<Cell> = (0..ROWS)
        .flat_map(|y| (0..COLUMNS).map(move |x| (x, y)))
        .filter(|cell| !body.contains(cell))
        .collect();
    if free.is_empty() {
        None
    } else {
        Some(free[rng.usize(..free.len())])
    }
}

/// Milliseconds between steps: shorter with every meal, down to a floor.
pub fn step_ms(score: u32) -> u64 {
    START_STEP_MS
        .saturating_sub(u64::from(score) * SPEEDUP_MS)
        .max(FASTEST_STEP_MS)
}

impl Snake {
    /// A short snake in the middle heading right, and the first food.
    pub fn new(rng: &mut fastrand::Rng) -> Self {
        let (x, y) = (COLUMNS / 2, ROWS / 2);
        let body: VecDeque<Cell> = (0..START_LENGTH).map(|i| (x - i, y)).collect();
        let food = place_food(&body, rng);
        Self {
            body,
            heading: Direction::Right,
            queued: VecDeque::new(),
            food,
            score: 0,
            status: Status::Running,
            grow: 0,
        }
    }

    pub fn head(&self) -> Cell {
        self.body[0]
    }

    /// The way the snake will be heading after the queued turns.
    pub fn next_heading(&self) -> Direction {
        self.queued.back().copied().unwrap_or(self.heading)
    }

    /// Queue a turn. Going the same way, turning back into the neck and
    /// turns beyond `MAX_QUEUED` are ignored.
    pub fn turn(&mut self, direction: Direction) {
        let last = self.next_heading();
        if direction == last || direction == last.opposite() || self.queued.len() >= MAX_QUEUED {
            return;
        }
        self.queued.push_back(direction);
    }

    /// Move one cell; true if the snake ate.
    pub fn step(&mut self, rng: &mut fastrand::Rng) -> bool {
        if self.status != Status::Running {
            return false;
        }
        if let Some(direction) = self.queued.pop_front() {
            self.heading = direction;
        }
        let (dx, dy) = self.heading.delta();
        let (x, y) = self.head();
        let next = (x + dx, y + dy);
        if !in_bounds(next) {
            self.status = Status::Dead;
            return false;
        }
        let ate = self.food == Some(next);
        if ate {
            self.grow += 1;
        }
        // The tail leaves its cell this step unless the snake grows, so the
        // head may follow right behind it.
        let tail_leaves = self.grow == 0;
        let solid = self.body.len() - usize::from(tail_leaves);
        if self.body.iter().take(solid).any(|&cell| cell == next) {
            self.status = Status::Dead;
            return false;
        }
        self.body.push_front(next);
        if self.grow > 0 {
            self.grow -= 1;
        } else {
            self.body.pop_back();
        }
        if ate {
            self.score += 1;
            self.food = place_food(&self.body, rng);
            if self.food.is_none() {
                self.status = Status::Won;
            }
        }
        ate
    }
}

#[cfg(test)]
mod tests;
