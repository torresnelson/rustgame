//! Merge rules with no engine code: tiles on a 4x4 board slide as far as they
//! go in one direction, two equal tiles meeting merge into their sum (once
//! per move), and every move that changes the board adds a 2 (or a 4).
//! Tiles are stored as exponents: 0 is empty, 1 is 2, 11 is 2048.

pub const SIZE: usize = 4;
/// The 2048 tile.
pub const GOAL: u8 = 11;
/// One new tile in ten is a 4.
pub const FOUR_CHANCE: f32 = 0.1;

pub type Board = [[u8; SIZE]; SIZE];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];
}

/// The number on a tile (0 for an empty cell).
pub fn value(exponent: u8) -> u32 {
    if exponent == 0 { 0 } else { 1 << exponent }
}

/// Slide one line towards its start, merging equal neighbours once each;
/// returns the new line and the points (the merged values).
pub fn slide_line(line: [u8; SIZE]) -> ([u8; SIZE], u32) {
    let mut out = [0; SIZE];
    let mut points = 0;
    let mut next = 0;
    let mut can_merge = false;
    for tile in line.into_iter().filter(|&t| t != 0) {
        if can_merge && out[next - 1] == tile {
            out[next - 1] += 1;
            points += value(tile + 1);
            can_merge = false;
        } else {
            out[next] = tile;
            next += 1;
            can_merge = true;
        }
    }
    (out, points)
}

/// The board positions of line `i` for `direction`, first the cell tiles
/// slide towards.
fn line_cells(direction: Direction, i: usize) -> [(usize, usize); SIZE] {
    std::array::from_fn(|k| match direction {
        Direction::Left => (i, k),
        Direction::Right => (i, SIZE - 1 - k),
        Direction::Up => (k, i),
        Direction::Down => (SIZE - 1 - k, i),
    })
}

/// Slide the whole board; returns the new board and the points scored.
pub fn slide(board: &Board, direction: Direction) -> (Board, u32) {
    let mut out = *board;
    let mut points = 0;
    for i in 0..SIZE {
        let cells = line_cells(direction, i);
        let line = cells.map(|(r, c)| board[r][c]);
        let (slid, gained) = slide_line(line);
        points += gained;
        for (k, (r, c)) in cells.into_iter().enumerate() {
            out[r][c] = slid[k];
        }
    }
    (out, points)
}

pub fn empty_cells(board: &Board) -> Vec<(usize, usize)> {
    (0..SIZE)
        .flat_map(|r| (0..SIZE).map(move |c| (r, c)))
        .filter(|&(r, c)| board[r][c] == 0)
        .collect()
}

/// Put a 2 (or sometimes a 4) on a random empty cell; where, if anywhere.
pub fn spawn(board: &mut Board, rng: &mut fastrand::Rng) -> Option<(usize, usize)> {
    let empty = empty_cells(board);
    if empty.is_empty() {
        return None;
    }
    let (r, c) = empty[rng.usize(..empty.len())];
    board[r][c] = if rng.f32() < FOUR_CHANCE { 2 } else { 1 };
    Some((r, c))
}

pub fn can_move(board: &Board) -> bool {
    Direction::ALL
        .into_iter()
        .any(|direction| slide(board, direction).0 != *board)
}

pub fn highest(board: &Board) -> u8 {
    board.iter().flatten().copied().max().unwrap_or(0)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Puzzle {
    pub board: Board,
    pub score: u32,
    /// The last tile added, for a pop-in effect.
    pub spawned: Option<(usize, usize)>,
}

impl Puzzle {
    /// An empty board with two starting tiles.
    pub fn new(rng: &mut fastrand::Rng) -> Self {
        let mut board = [[0; SIZE]; SIZE];
        spawn(&mut board, rng);
        let spawned = spawn(&mut board, rng);
        Self {
            board,
            score: 0,
            spawned,
        }
    }

    /// Slide; when the board changed, score and add a tile. True if it moved.
    pub fn play(&mut self, direction: Direction, rng: &mut fastrand::Rng) -> bool {
        let (board, points) = slide(&self.board, direction);
        if board == self.board {
            return false;
        }
        self.board = board;
        self.score += points;
        self.spawned = spawn(&mut self.board, rng);
        true
    }

    pub fn over(&self) -> bool {
        !can_move(&self.board)
    }

    pub fn reached_goal(&self) -> bool {
        highest(&self.board) >= GOAL
    }
}

#[cfg(test)]
mod tests;
