//! Minefield — core game logic for Minesweeper.
//!
//! Grid of cells with mine placement, adjacency counting,
//! flood-fill reveal, and win/loss detection.

extern crate alloc;
use alloc::vec::Vec;

/// Cell state as seen by the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellState {
    Hidden,
    Flagged,
    Revealed,
}

/// What's actually in the cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellContent {
    Empty(u8),   // adjacent mine count (0-8)
    Mine,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub state: CellState,
    pub content: CellContent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Expert,
}

impl Difficulty {
    pub fn label(&self) -> &'static str {
        match self {
            Difficulty::Beginner => "Beginner",
            Difficulty::Intermediate => "Intermediate",
            Difficulty::Expert => "Expert",
        }
    }

    pub fn params(&self) -> (usize, usize, usize) {
        // (width, height, mines)
        match self {
            Difficulty::Beginner => (9, 9, 10),
            Difficulty::Intermediate => (16, 16, 40),
            Difficulty::Expert => (16, 20, 60),
        }
    }

    pub fn all() -> &'static [Difficulty] {
        &[Difficulty::Beginner, Difficulty::Intermediate, Difficulty::Expert]
    }

    pub fn key(&self) -> &'static str {
        match self {
            Difficulty::Beginner => "beginner",
            Difficulty::Intermediate => "intermediate",
            Difficulty::Expert => "expert",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameState {
    Ready,      // mines not yet placed (first click safety)
    Playing,
    Won,
    Lost,
}

pub struct Minefield {
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    pub cells: Vec<Cell>,
    pub game_state: GameState,
    pub flags_placed: usize,
    pub cells_revealed: usize,
    pub difficulty: Difficulty,
    // Cursor position
    pub cursor_x: usize,
    pub cursor_y: usize,
    // The cell that killed you (for highlighting)
    pub death_cell: Option<(usize, usize)>,
}

impl Minefield {
    /// Create a new empty minefield (mines placed on first reveal).
    pub fn new(difficulty: Difficulty) -> Self {
        let (w, h, mines) = difficulty.params();
        let cells = vec![Cell {
            state: CellState::Hidden,
            content: CellContent::Empty(0),
        }; w * h];

        Self {
            width: w,
            height: h,
            mine_count: mines,
            cells,
            game_state: GameState::Ready,
            flags_placed: 0,
            cells_revealed: 0,
            difficulty,
            cursor_x: w / 2,
            cursor_y: h / 2,
            death_cell: None,
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn cell(&self, x: usize, y: usize) -> &Cell {
        &self.cells[self.idx(x, y)]
    }

    fn cell_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        let i = self.idx(x, y);
        &mut self.cells[i]
    }

    /// Place mines randomly, ensuring (safe_x, safe_y) and its neighbors are safe.
    pub fn place_mines(&mut self, safe_x: usize, safe_y: usize, rng: &crate::rng::Rng) {
        // Collect all valid positions (not the safe cell or its neighbors)
        let mut positions: Vec<(usize, usize)> = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = (x as isize - safe_x as isize).abs();
                let dy = (y as isize - safe_y as isize).abs();
                if dx > 1 || dy > 1 {
                    positions.push((x, y));
                }
            }
        }

        // Shuffle and take first mine_count positions
        rng.shuffle(&mut positions);
        let count = self.mine_count.min(positions.len());

        for i in 0..count {
            let (mx, my) = positions[i];
            self.cell_mut(mx, my).content = CellContent::Mine;
        }

        // Calculate adjacency counts for non-mine cells
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cell(x, y).content == CellContent::Mine {
                    continue;
                }
                let count = self.count_adjacent_mines(x, y);
                self.cell_mut(x, y).content = CellContent::Empty(count);
            }
        }

        self.game_state = GameState::Playing;
    }

    fn count_adjacent_mines(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        for (nx, ny) in self.neighbors(x, y) {
            if self.cell(nx, ny).content == CellContent::Mine {
                count += 1;
            }
        }
        count
    }

    fn neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < self.width as i32
                    && ny >= 0 && ny < self.height as i32
                {
                    result.push((nx as usize, ny as usize));
                }
            }
        }
        result
    }

    /// Reveal a cell. Returns true if the action was valid.
    pub fn reveal(&mut self, x: usize, y: usize, rng: &crate::rng::Rng) -> bool {
        // First reveal — place mines
        if self.game_state == GameState::Ready {
            self.place_mines(x, y, rng);
        }

        if self.game_state != GameState::Playing {
            return false;
        }

        let cell = self.cell(x, y);
        if cell.state != CellState::Hidden {
            return false;
        }

        match cell.content {
            CellContent::Mine => {
                // BOOM
                self.cell_mut(x, y).state = CellState::Revealed;
                self.death_cell = Some((x, y));
                self.game_state = GameState::Lost;
                self.reveal_all_mines();
                true
            }
            CellContent::Empty(0) => {
                // Flood fill
                self.flood_reveal(x, y);
                self.check_win();
                true
            }
            CellContent::Empty(_) => {
                self.cell_mut(x, y).state = CellState::Revealed;
                self.cells_revealed += 1;
                self.check_win();
                true
            }
        }
    }

    /// Flood-fill reveal for cells with 0 adjacent mines.
    fn flood_reveal(&mut self, x: usize, y: usize) {
        // Iterative BFS to avoid stack overflow on large grids
        let mut stack: Vec<(usize, usize)> = Vec::new();
        stack.push((x, y));

        while let Some((cx, cy)) = stack.pop() {
            let cell = self.cell(cx, cy);
            if cell.state != CellState::Hidden {
                continue;
            }

            self.cell_mut(cx, cy).state = CellState::Revealed;
            self.cells_revealed += 1;

            // If this cell is flagged, unflag it
            if cell.state == CellState::Flagged {
                self.flags_placed -= 1;
            }

            if let CellContent::Empty(0) = self.cell(cx, cy).content {
                for (nx, ny) in self.neighbors(cx, cy) {
                    if self.cell(nx, ny).state == CellState::Hidden {
                        stack.push((nx, ny));
                    }
                }
            }
        }
    }

    /// Toggle flag on a hidden cell.
    pub fn toggle_flag(&mut self, x: usize, y: usize) -> bool {
        if self.game_state != GameState::Playing && self.game_state != GameState::Ready {
            return false;
        }

        let cell = self.cell(x, y);
        match cell.state {
            CellState::Hidden => {
                self.cell_mut(x, y).state = CellState::Flagged;
                self.flags_placed += 1;
                true
            }
            CellState::Flagged => {
                self.cell_mut(x, y).state = CellState::Hidden;
                self.flags_placed -= 1;
                true
            }
            CellState::Revealed => false,
        }
    }

    /// Check if all non-mine cells are revealed.
    fn check_win(&mut self) {
        let total_safe = self.width * self.height - self.mine_count;
        if self.cells_revealed >= total_safe {
            self.game_state = GameState::Won;
            // Auto-flag all remaining mines
            for y in 0..self.height {
                for x in 0..self.width {
                    if self.cell(x, y).content == CellContent::Mine
                        && self.cell(x, y).state == CellState::Hidden
                    {
                        self.cell_mut(x, y).state = CellState::Flagged;
                    }
                }
            }
        }
    }

    /// Reveal all mines (on loss).
    fn reveal_all_mines(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cell(x, y).content == CellContent::Mine
                    && self.cell(x, y).state == CellState::Hidden
                {
                    self.cell_mut(x, y).state = CellState::Revealed;
                }
            }
        }
    }

    /// Move cursor with bounds checking.
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let nx = self.cursor_x as i32 + dx;
        let ny = self.cursor_y as i32 + dy;
        if nx >= 0 && nx < self.width as i32 {
            self.cursor_x = nx as usize;
        }
        if ny >= 0 && ny < self.height as i32 {
            self.cursor_y = ny as usize;
        }
    }

    /// Remaining mine count (mines - flags).
    pub fn mines_remaining(&self) -> i32 {
        self.mine_count as i32 - self.flags_placed as i32
    }
}
