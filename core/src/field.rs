//! The minefield.

use crate::{Entropy, EntropyError};
use precursor_appkit_core::entropy::below;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Hidden,
    Flagged,
    Revealed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellContent {
    /// Adjacent mine count, 0 to 8.
    Empty(u8),
    Mine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub state: CellState,
    pub content: CellContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Expert,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Beginner, Difficulty::Intermediate, Difficulty::Expert];

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Beginner => "Beginner",
            Difficulty::Intermediate => "Intermediate",
            Difficulty::Expert => "Expert",
        }
    }

    /// `(width, height, mines)`.
    pub fn params(self) -> (usize, usize, usize) {
        match self {
            Difficulty::Beginner => (9, 9, 10),
            Difficulty::Intermediate => (16, 16, 40),
            Difficulty::Expert => (16, 20, 60),
        }
    }

    pub fn index(self) -> usize {
        match self {
            Difficulty::Beginner => 0,
            Difficulty::Intermediate => 1,
            Difficulty::Expert => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// Mines are placed on the first reveal, so the first click is always safe.
    Ready,
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Clone)]
pub struct Minefield {
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    pub cells: Vec<Cell>,
    pub game_state: GameState,
    pub flags_placed: usize,
    pub cells_revealed: usize,
    pub difficulty: Difficulty,
    pub cursor_x: usize,
    pub cursor_y: usize,
    /// The mine that was stepped on.
    pub death_cell: Option<(usize, usize)>,
}

impl Minefield {
    pub fn new(difficulty: Difficulty) -> Self {
        let (w, h, mines) = difficulty.params();
        Self::with_size(w, h, mines, difficulty)
    }

    /// Any size (tests use small boards). `mines` is capped so a first click always has a safe 3x3.
    pub fn with_size(w: usize, h: usize, mines: usize, difficulty: Difficulty) -> Self {
        let mine_count = mines.min((w * h).saturating_sub(9));
        Self {
            width: w,
            height: h,
            mine_count,
            cells: vec![Cell { state: CellState::Hidden, content: CellContent::Empty(0) }; w * h],
            game_state: GameState::Ready,
            flags_placed: 0,
            cells_revealed: 0,
            difficulty,
            cursor_x: w / 2,
            cursor_y: h / 2,
            death_cell: None,
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize { y * self.width + x }

    pub fn cell(&self, x: usize, y: usize) -> &Cell { &self.cells[self.idx(x, y)] }

    fn cell_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        let i = self.idx(x, y);
        &mut self.cells[i]
    }

    pub fn is_mine(&self, x: usize, y: usize) -> bool { self.cell(x, y).content == CellContent::Mine }

    /// Place mines, keeping `(safe_x, safe_y)` and its neighbours free. Every legal layout is
    /// equally likely (partial Fisher-Yates with unbiased draws). If the entropy source fails
    /// nothing is placed and the field stays in `Ready`.
    pub fn place_mines<E: Entropy + ?Sized>(&mut self, safe_x: usize, safe_y: usize, e: &mut E) -> Result<(), EntropyError> {
        let mut pool: Vec<usize> = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if x.abs_diff(safe_x) > 1 || y.abs_diff(safe_y) > 1 {
                    pool.push(self.idx(x, y));
                }
            }
        }
        let k = self.mine_count.min(pool.len());
        // choose k of the pool: after step i, pool[..=i] is a uniform sample
        for i in 0..k {
            let j = i + below(e, (pool.len() - i) as u32)? as usize;
            pool.swap(i, j);
        }
        for c in &mut self.cells {
            c.content = CellContent::Empty(0);
        }
        for &i in &pool[..k] {
            self.cells[i].content = CellContent::Mine;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.is_mine(x, y) {
                    let n = self.neighbors(x, y).filter(|&(nx, ny)| self.is_mine(nx, ny)).count() as u8;
                    self.cell_mut(x, y).content = CellContent::Empty(n);
                }
            }
        }
        self.game_state = GameState::Playing;
        Ok(())
    }

    pub fn neighbors(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
        let (w, h) = (self.width as isize, self.height as isize);
        (-1isize..=1).flat_map(move |dy| (-1isize..=1).map(move |dx| (dx, dy))).filter_map(move |(dx, dy)| {
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            ((dx, dy) != (0, 0) && nx >= 0 && ny >= 0 && nx < w && ny < h).then_some((nx as usize, ny as usize))
        })
    }

    /// Reveal a cell. `Ok(true)` if something was revealed, `Ok(false)` if the action did nothing
    /// (flagged or already open cell, or the game is over): callers start the clock only on `true`.
    pub fn reveal<E: Entropy + ?Sized>(&mut self, x: usize, y: usize, e: &mut E) -> Result<bool, EntropyError> {
        if self.game_state == GameState::Ready {
            if self.cell(x, y).state != CellState::Hidden {
                return Ok(false); // a flagged first click must not place mines or start play
            }
            self.place_mines(x, y, e)?;
        }
        if self.game_state != GameState::Playing || self.cell(x, y).state != CellState::Hidden {
            return Ok(false);
        }
        match self.cell(x, y).content {
            CellContent::Mine => {
                self.cell_mut(x, y).state = CellState::Revealed;
                self.death_cell = Some((x, y));
                self.game_state = GameState::Lost;
                self.reveal_all_mines();
            }
            CellContent::Empty(_) => {
                self.flood_reveal(x, y);
                self.check_win();
            }
        }
        Ok(true)
    }

    /// Open `(x, y)` and, for zeros, everything connected. Iterative, so big boards can't overflow
    /// the stack. Flagged cells stay closed. (0.1 held a borrow of the cell across the mutation
    /// and did not compile.)
    fn flood_reveal(&mut self, x: usize, y: usize) {
        let mut stack = vec![(x, y)];
        while let Some((cx, cy)) = stack.pop() {
            if self.cell(cx, cy).state != CellState::Hidden {
                continue;
            }
            self.cell_mut(cx, cy).state = CellState::Revealed;
            self.cells_revealed += 1;
            if self.cell(cx, cy).content == CellContent::Empty(0) {
                let next: Vec<_> = self.neighbors(cx, cy).filter(|&(nx, ny)| self.cell(nx, ny).state == CellState::Hidden).collect();
                stack.extend(next);
            }
        }
    }

    pub fn toggle_flag(&mut self, x: usize, y: usize) -> bool {
        if !matches!(self.game_state, GameState::Playing | GameState::Ready) {
            return false;
        }
        match self.cell(x, y).state {
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

    fn check_win(&mut self) {
        let safe = self.width * self.height - self.mine_count;
        if self.game_state == GameState::Playing && self.cells_revealed >= safe {
            self.game_state = GameState::Won;
            for i in 0..self.cells.len() {
                if self.cells[i].content == CellContent::Mine && self.cells[i].state == CellState::Hidden {
                    self.cells[i].state = CellState::Flagged;
                    self.flags_placed += 1;
                }
            }
        }
    }

    fn reveal_all_mines(&mut self) {
        for c in &mut self.cells {
            if c.content == CellContent::Mine && c.state == CellState::Hidden {
                c.state = CellState::Revealed;
            }
        }
    }

    /// A flag on a cell that is not a mine, shown after a loss.
    pub fn is_wrong_flag(&self, x: usize, y: usize) -> bool {
        self.game_state == GameState::Lost && self.cell(x, y).state == CellState::Flagged && !self.is_mine(x, y)
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        self.cursor_x = (self.cursor_x as i32 + dx).clamp(0, self.width as i32 - 1) as usize;
        self.cursor_y = (self.cursor_y as i32 + dy).clamp(0, self.height as i32 - 1) as usize;
    }

    pub fn mines_remaining(&self) -> i32 { self.mine_count as i32 - self.flags_placed as i32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use precursor_appkit_core::entropy::testing::*;

    fn mines_of(f: &Minefield) -> Vec<(usize, usize)> {
        (0..f.height).flat_map(|y| (0..f.width).map(move |x| (x, y))).filter(|&(x, y)| f.is_mine(x, y)).collect()
    }

    /// Build a board with mines at fixed places (bypassing placement).
    fn board(w: usize, h: usize, mines: &[(usize, usize)]) -> Minefield {
        let mut f = Minefield::with_size(w, h, mines.len(), Difficulty::Beginner);
        for &(x, y) in mines {
            f.cell_mut(x, y).content = CellContent::Mine;
        }
        for y in 0..h {
            for x in 0..w {
                if !f.is_mine(x, y) {
                    let n = f.neighbors(x, y).filter(|&(a, b)| f.is_mine(a, b)).count() as u8;
                    f.cell_mut(x, y).content = CellContent::Empty(n);
                }
            }
        }
        f.game_state = GameState::Playing;
        f
    }

    #[test]
    fn first_click_is_safe_for_every_cell_and_difficulty() {
        let mut e = XorShift(2024);
        for d in Difficulty::ALL {
            let (w, h, m) = d.params();
            for (sx, sy) in [(0, 0), (w - 1, h - 1), (w / 2, h / 2), (0, h - 1), (w - 1, 0)] {
                for _ in 0..20 {
                    let mut f = Minefield::new(d);
                    f.place_mines(sx, sy, &mut e).unwrap();
                    assert_eq!(mines_of(&f).len(), m);
                    for (nx, ny) in f.neighbors(sx, sy).chain([(sx, sy)]) {
                        assert!(!f.is_mine(nx, ny), "{d:?} click ({sx},{sy}) mine at ({nx},{ny})");
                    }
                    assert_eq!(f.game_state, GameState::Playing);
                }
            }
        }
    }

    #[test]
    fn first_reveal_never_loses_and_opens_an_area() {
        let mut e = XorShift(7);
        for _ in 0..200 {
            let mut f = Minefield::new(Difficulty::Expert);
            assert_eq!(f.reveal(8, 10, &mut e), Ok(true));
            assert_ne!(f.game_state, GameState::Lost);
            assert!(f.cells_revealed >= 1);
            assert_eq!(f.cell(8, 10).content, CellContent::Empty(0), "first click is a zero cell, so it floods");
            assert!(f.cells_revealed > 1);
        }
    }

    #[test]
    fn placement_is_uniform_over_the_pool() {
        // 5x5 board, 3 mines, click in the corner: pool = 25 - 4 = 21 cells. Each should get 3/21.
        let mut e = XorShift(99);
        let mut hits = vec![0u32; 25];
        let n = 21_000;
        for _ in 0..n {
            let mut f = Minefield::with_size(5, 5, 3, Difficulty::Beginner);
            f.place_mines(0, 0, &mut e).unwrap();
            for (x, y) in mines_of(&f) {
                hits[y * 5 + x] += 1;
            }
        }
        let expect = n as f64 * 3.0 / 21.0; // 3000
        for (i, &h) in hits.iter().enumerate() {
            let (x, y) = (i % 5, i / 5);
            if x <= 1 && y <= 1 {
                assert_eq!(h, 0, "safe zone ({x},{y})");
            } else {
                assert!((h as f64 - expect).abs() < expect * 0.1, "cell ({x},{y}): {h} vs {expect}");
            }
        }
    }

    #[test]
    fn placement_fails_closed() {
        let mut f = Minefield::new(Difficulty::Beginner);
        assert!(f.reveal(4, 4, &mut Broken).is_err());
        assert_eq!(f.game_state, GameState::Ready);
        assert!(mines_of(&f).is_empty());
        let mut g = Minefield::new(Difficulty::Expert);
        assert!(g.place_mines(0, 0, &mut FailAfter(XorShift(1), 10)).is_err());
        assert_eq!(g.game_state, GameState::Ready);
        assert!(mines_of(&g).is_empty(), "no partial layout");
        // a stuck source can't give a layout that looks random either
        assert!(Minefield::new(Difficulty::Beginner).place_mines(4, 4, &mut Stuck(0)).is_err());
    }

    #[test]
    fn adjacency_counts_are_right() {
        let f = board(4, 4, &[(0, 0), (1, 0), (3, 3)]);
        assert_eq!(f.cell(0, 1).content, CellContent::Empty(2));
        assert_eq!(f.cell(1, 1).content, CellContent::Empty(2));
        assert_eq!(f.cell(2, 1).content, CellContent::Empty(1));
        assert_eq!(f.cell(2, 2).content, CellContent::Empty(1));
        assert_eq!(f.cell(3, 0).content, CellContent::Empty(0));
    }

    #[test]
    fn flood_fill_opens_the_zero_region_and_its_border_only() {
        // single mine in the far corner of a 6x6 board: everything but the mine and ... opens
        let mut f = board(6, 6, &[(5, 5)]);
        assert_eq!(f.reveal(0, 0, &mut Broken), Ok(true));
        assert_eq!(f.cells_revealed, 35);
        assert_eq!(f.game_state, GameState::Won);
        assert_eq!(f.cell(5, 5).state, CellState::Flagged, "remaining mine auto-flagged on win");
        assert_eq!(f.mines_remaining(), 0);

        // a wall of mines stops the fill
        let mine_col: Vec<_> = (0..6).map(|y| (3, y)).collect();
        let mut f = board(6, 6, &mine_col);
        f.reveal(0, 0, &mut Broken).unwrap();
        for y in 0..6 {
            for x in 0..6 {
                let open = f.cell(x, y).state == CellState::Revealed;
                assert_eq!(open, x <= 2, "({x},{y})");
            }
        }
        assert_eq!(f.cells_revealed, 18);
        assert_eq!(f.game_state, GameState::Playing);
    }

    #[test]
    fn flood_fill_skips_flags_and_handles_a_big_empty_board() {
        let mut f = board(6, 6, &[(5, 5)]);
        f.toggle_flag(2, 2);
        f.reveal(0, 0, &mut Broken).unwrap();
        assert_eq!(f.cell(2, 2).state, CellState::Flagged);
        assert_eq!(f.cells_revealed, 34);
        assert_eq!(f.flags_placed, 1);
        // 200x200 with one mine: iterative fill must not overflow the stack
        let mut big = Minefield::with_size(200, 200, 0, Difficulty::Beginner);
        big.mine_count = 0;
        big.game_state = GameState::Playing;
        big.reveal(0, 0, &mut Broken).unwrap();
        assert_eq!(big.cells_revealed, 40_000);
    }

    #[test]
    fn mine_loses_and_marks_the_death_cell_and_wrong_flags() {
        let mut f = board(4, 4, &[(0, 0), (3, 3)]);
        f.toggle_flag(1, 1); // wrong
        f.toggle_flag(3, 3); // right
        assert_eq!(f.reveal(0, 0, &mut Broken), Ok(true));
        assert_eq!(f.game_state, GameState::Lost);
        assert_eq!(f.death_cell, Some((0, 0)));
        assert!(f.is_wrong_flag(1, 1) && !f.is_wrong_flag(3, 3));
        assert_eq!(f.cell(3, 3).state, CellState::Flagged, "correct flag stays");
        assert_eq!(f.reveal(2, 2, &mut Broken), Ok(false), "no moves after the game");
        assert!(!f.toggle_flag(2, 2));
    }

    #[test]
    fn invalid_reveals_return_false() {
        let mut f = board(4, 4, &[(0, 0)]);
        f.toggle_flag(2, 2);
        assert_eq!(f.reveal(2, 2, &mut Broken), Ok(false), "flagged");
        f.reveal(3, 3, &mut Broken).unwrap();
        assert_eq!(f.reveal(3, 3, &mut Broken), Ok(false), "already open");
        // flagged first click doesn't place mines
        let mut g = Minefield::new(Difficulty::Beginner);
        g.toggle_flag(4, 4);
        assert_eq!(g.reveal(4, 4, &mut Broken), Ok(false));
        assert_eq!(g.game_state, GameState::Ready);
    }

    #[test]
    fn numbered_cell_reveals_just_itself() {
        let mut f = board(4, 4, &[(0, 0), (1, 0), (3, 3)]);
        f.reveal(0, 1, &mut Broken).unwrap();
        assert_eq!(f.cells_revealed, 1);
        assert_eq!(f.game_state, GameState::Playing);
    }

    #[test]
    fn win_requires_every_safe_cell_and_flags_balance() {
        let mut f = board(4, 4, &[(1, 1)]);
        for y in 0..4 {
            for x in 0..4 {
                if (x, y) != (1, 1) && f.game_state == GameState::Playing {
                    f.reveal(x, y, &mut Broken).unwrap();
                }
            }
        }
        assert_eq!(f.game_state, GameState::Won);
        assert_eq!(f.cells_revealed, 15);
        assert_eq!(f.flags_placed, 1);
        assert!(!f.toggle_flag(1, 1));
    }

    #[test]
    fn flags_toggle_and_cursor_clamps() {
        let mut f = Minefield::new(Difficulty::Beginner);
        assert!(f.toggle_flag(0, 0));
        assert_eq!(f.mines_remaining(), 9);
        assert!(f.toggle_flag(0, 0));
        assert_eq!(f.flags_placed, 0);
        f.move_cursor(-100, -100);
        assert_eq!((f.cursor_x, f.cursor_y), (0, 0));
        f.move_cursor(100, 100);
        assert_eq!((f.cursor_x, f.cursor_y), (8, 8));
    }

    #[test]
    fn mine_count_is_capped_so_the_first_click_stays_safe() {
        let mut f = Minefield::with_size(3, 3, 50, Difficulty::Beginner);
        assert_eq!(f.mine_count, 0);
        f.place_mines(1, 1, &mut XorShift(1)).unwrap();
        let mut f = Minefield::with_size(4, 4, 50, Difficulty::Beginner);
        assert_eq!(f.mine_count, 7);
        f.place_mines(0, 0, &mut XorShift(1)).unwrap();
        assert_eq!(mines_of(&f).len(), 7);
    }
}
