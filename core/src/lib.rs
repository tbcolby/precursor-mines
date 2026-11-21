//! Minesweeper rules with no Xous dependencies.
//!
//! * [`field`]: grid, first-click-safe mine placement (unbiased, fail-closed),
//!   reveal, flood fill, flags, win/loss.
//! * [`clock`]: game time that only runs while the app is in front.
//! * [`draw`]: cell geometry as layered rectangles, so the shell can batch a board
//!   into a handful of GAM calls instead of one TextView per number.
//! * [`saved`]: best times and last difficulty.

pub use precursor_appkit_core::entropy::{Entropy, EntropyError};

pub mod clock;
pub mod draw;
pub mod field;
pub mod saved;

pub use field::{CellState, Difficulty, GameState, Minefield};
