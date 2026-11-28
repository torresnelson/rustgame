//! Minesweeper rules with no engine code: a grid of hidden cells, some
//! mined; opening a cell shows how many mines touch it, opening a zero opens
//! its neighbours too, and flags mark suspected mines. The mines are laid
//! after the first opening, away from it, so the first move is always safe.

pub type Cell = (i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Difficulty {
    pub name: &'static str,
    pub columns: i32,
    pub rows: i32,
    pub mines: usize,
}

pub const DIFFICULTIES: [Difficulty; 3] = [
    Difficulty {
        name: "EASY",
        columns: 9,
        rows: 9,
        mines: 10,
    },
    Difficulty {
        name: "MEDIUM",
        columns: 16,
        rows: 16,
        mines: 40,
    },
    Difficulty {
        name: "HARD",
        columns: 16,
        rows: 20,
        mines: 60,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cover {
    Hidden,
    Flagged,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    Won,
    Lost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub columns: i32,
    pub rows: i32,
    pub mines: usize,
    mined: Vec<bool>,
    covers: Vec<Cover>,
    counts: Vec<u8>,
    /// Whether the mines are laid yet (they are on the first opening).
    pub laid: bool,
    pub status: Status,
    /// The mine that went off.
    pub exploded: Option<Cell>,
}

impl Field {
    pub fn new(difficulty: &Difficulty) -> Self {
        let size = (difficulty.columns * difficulty.rows) as usize;
        Self {
            columns: difficulty.columns,
            rows: difficulty.rows,
            mines: difficulty.mines,
            mined: vec![false; size],
            covers: vec![Cover::Hidden; size],
            counts: vec![0; size],
            laid: false,
            status: Status::Playing,
            exploded: None,
        }
    }

    pub fn in_bounds(&self, (x, y): Cell) -> bool {
        (0..self.columns).contains(&x) && (0..self.rows).contains(&y)
    }

    fn index(&self, (x, y): Cell) -> usize {
        (y * self.columns + x) as usize
    }

    pub fn cells(&self) -> impl Iterator<Item = Cell> + use<> {
        let (columns, rows) = (self.columns, self.rows);
        (0..rows).flat_map(move |y| (0..columns).map(move |x| (x, y)))
    }

    /// The up to eight cells around `cell`.
    pub fn neighbours(&self, (x, y): Cell) -> Vec<Cell> {
        (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| (x + dx, y + dy)))
            .filter(|&cell| cell != (x, y) && self.in_bounds(cell))
            .collect()
    }

    pub fn cover(&self, cell: Cell) -> Cover {
        self.covers[self.index(cell)]
    }

    pub fn is_mine(&self, cell: Cell) -> bool {
        self.mined[self.index(cell)]
    }

    /// Mines touching `cell`.
    pub fn count(&self, cell: Cell) -> u8 {
        self.counts[self.index(cell)]
    }

    pub fn flags(&self) -> usize {
        self.covers.iter().filter(|&&c| c == Cover::Flagged).count()
    }

    /// Mines not yet flagged (negative with too many flags).
    pub fn mines_left(&self) -> i32 {
        self.mines as i32 - self.flags() as i32
    }

    /// Lay the mines at random, never on `safe` or next to it (when the
    /// grid has room for that).
    pub fn lay_mines(&mut self, safe: Cell, rng: &mut fastrand::Rng) {
        let near = |cell: Cell| (cell.0 - safe.0).abs() <= 1 && (cell.1 - safe.1).abs() <= 1;
        let mut spots: Vec<Cell> = self.cells().filter(|&cell| !near(cell)).collect();
        if spots.len() < self.mines {
            spots = self.cells().filter(|&cell| cell != safe).collect();
        }
        rng.shuffle(&mut spots);
        spots.truncate(self.mines);
        self.lay_mines_at(&spots);
    }

    /// Lay mines on exactly these cells.
    pub fn lay_mines_at(&mut self, cells: &[Cell]) {
        self.mined.fill(false);
        for &cell in cells {
            let index = self.index(cell);
            self.mined[index] = true;
        }
        self.mines = cells.len();
        for cell in self.cells() {
            let count = self
                .neighbours(cell)
                .into_iter()
                .filter(|&n| self.is_mine(n))
                .count();
            let index = self.index(cell);
            self.counts[index] = count as u8;
        }
        self.laid = true;
    }

    /// Flag or unflag a hidden cell; true if it changed.
    pub fn toggle_flag(&mut self, cell: Cell) -> bool {
        if self.status != Status::Playing || !self.in_bounds(cell) {
            return false;
        }
        let index = self.index(cell);
        self.covers[index] = match self.covers[index] {
            Cover::Hidden => Cover::Flagged,
            Cover::Flagged => Cover::Hidden,
            Cover::Open => return false,
        };
        true
    }

    /// Open `cell`: lays the mines on the first opening, floods zeros, and
    /// on an already open number opens its unflagged neighbours when the
    /// flags around it match the number (a chord). True if anything opened.
    pub fn open(&mut self, cell: Cell, rng: &mut fastrand::Rng) -> bool {
        if self.status != Status::Playing || !self.in_bounds(cell) {
            return false;
        }
        if !self.laid {
            self.lay_mines(cell, rng);
        }
        match self.cover(cell) {
            Cover::Flagged => false,
            Cover::Open => self.chord(cell),
            Cover::Hidden => {
                self.uncover(cell);
                self.settle();
                true
            }
        }
    }

    fn chord(&mut self, cell: Cell) -> bool {
        let around = self.neighbours(cell);
        let flagged = around
            .iter()
            .filter(|&&n| self.cover(n) == Cover::Flagged)
            .count();
        if self.count(cell) == 0 || flagged != usize::from(self.count(cell)) {
            return false;
        }
        let mut opened = false;
        for n in around {
            if self.cover(n) == Cover::Hidden {
                self.uncover(n);
                opened = true;
            }
        }
        self.settle();
        opened
    }

    /// Open `start`, and every cell reachable through zeros.
    fn uncover(&mut self, start: Cell) {
        if self.is_mine(start) {
            let index = self.index(start);
            self.covers[index] = Cover::Open;
            if self.exploded.is_none() {
                self.exploded = Some(start);
            }
            return;
        }
        let mut stack = vec![start];
        while let Some(cell) = stack.pop() {
            let index = self.index(cell);
            if self.covers[index] != Cover::Hidden {
                continue;
            }
            self.covers[index] = Cover::Open;
            if self.counts[index] == 0 {
                stack.extend(
                    self.neighbours(cell)
                        .into_iter()
                        .filter(|&n| self.cover(n) == Cover::Hidden),
                );
            }
        }
    }

    /// Lost after a mine went off; won once every safe cell is open (the
    /// remaining mines are then flagged).
    fn settle(&mut self) {
        if self.exploded.is_some() {
            self.status = Status::Lost;
            return;
        }
        let safe_hidden =
            (0..self.covers.len()).any(|i| !self.mined[i] && self.covers[i] != Cover::Open);
        if !safe_hidden {
            self.status = Status::Won;
            for i in 0..self.covers.len() {
                if self.mined[i] {
                    self.covers[i] = Cover::Flagged;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
