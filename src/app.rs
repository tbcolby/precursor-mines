//! Application state machine for Minesweeper.

extern crate alloc;
use alloc::string::String;

use crate::minefield::{Difficulty, GameState, Minefield};
use crate::rng::Rng;
use crate::storage::Storage;

// Standard key codes (ecosystem standard)
const KEY_UP: char = '\u{2191}';
const KEY_DOWN: char = '\u{2193}';
const KEY_LEFT: char = '\u{2190}';
const KEY_RIGHT: char = '\u{2192}';
const KEY_ENTER: char = '\r';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_MENU: char = '\u{2234}';

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Menu,
    Playing,
    Won,
    Lost,
    BestTimes,
}

pub struct MinesApp {
    pub state: AppState,
    pub needs_redraw: bool,

    // Menu
    pub menu_cursor: usize,

    // Game
    pub field: Option<Minefield>,
    pub difficulty: Difficulty,
    pub elapsed_seconds: u32,
    pub game_start_ms: Option<u64>,

    // Best times
    pub best_beginner: Option<u32>,
    pub best_intermediate: Option<u32>,
    pub best_expert: Option<u32>,

    // Storage
    pub storage: Option<Storage>,
}

impl MinesApp {
    pub fn new() -> Self {
        Self {
            state: AppState::Menu,
            needs_redraw: true,
            menu_cursor: 0,
            field: None,
            difficulty: Difficulty::Beginner,
            elapsed_seconds: 0,
            game_start_ms: None,
            best_beginner: None,
            best_intermediate: None,
            best_expert: None,
            storage: None,
        }
    }

    pub fn init_storage(&mut self) {
        if let Ok(mut stor) = Storage::new() {
            if let Some(diff) = stor.load_difficulty() {
                self.difficulty = diff;
                self.menu_cursor = match diff {
                    Difficulty::Beginner => 0,
                    Difficulty::Intermediate => 1,
                    Difficulty::Expert => 2,
                };
            }
            self.best_beginner = stor.load_best_time(&Difficulty::Beginner);
            self.best_intermediate = stor.load_best_time(&Difficulty::Intermediate);
            self.best_expert = stor.load_best_time(&Difficulty::Expert);
            self.storage = Some(stor);
        }
    }

    pub fn save_state(&mut self) {
        if let Some(ref mut stor) = self.storage {
            stor.save_difficulty(&self.difficulty);
        }
    }

    pub fn best_time(&self, diff: &Difficulty) -> Option<u32> {
        match diff {
            Difficulty::Beginner => self.best_beginner,
            Difficulty::Intermediate => self.best_intermediate,
            Difficulty::Expert => self.best_expert,
        }
    }

    fn set_best_time(&mut self, diff: Difficulty, time: u32) {
        match diff {
            Difficulty::Beginner => self.best_beginner = Some(time),
            Difficulty::Intermediate => self.best_intermediate = Some(time),
            Difficulty::Expert => self.best_expert = Some(time),
        }
    }

    /// Update timer. Call this from the main loop with current ticktimer ms.
    pub fn update_timer(&mut self, now_ms: u64) {
        if let Some(start) = self.game_start_ms {
            if self.state == AppState::Playing {
                let new_elapsed = ((now_ms - start) / 1000) as u32;
                if new_elapsed != self.elapsed_seconds {
                    self.elapsed_seconds = new_elapsed.min(999);
                    self.needs_redraw = true;
                }
            }
        }
    }

    /// Handle a key press. Returns false to quit.
    pub fn handle_key(&mut self, key: char, rng: &Rng, now_ms: u64) -> bool {
        self.needs_redraw = true;

        match self.state {
            AppState::Menu => self.handle_menu_key(key),
            AppState::Playing => self.handle_playing_key(key, rng, now_ms),
            AppState::Won | AppState::Lost => self.handle_gameover_key(key),
            AppState::BestTimes => self.handle_besttimes_key(key),
        }
    }

    // ─── Menu ───────────────────────────────────────────────────────────────

    fn handle_menu_key(&mut self, key: char) -> bool {
        match key {
            KEY_UP => {
                if self.menu_cursor > 0 {
                    self.menu_cursor -= 1;
                }
            }
            KEY_DOWN => {
                if self.menu_cursor < 2 {
                    self.menu_cursor += 1;
                }
            }
            KEY_ENTER | ' ' => {
                self.difficulty = Difficulty::all()[self.menu_cursor];
                self.start_game();
            }
            '1' => { self.difficulty = Difficulty::Beginner; self.start_game(); }
            '2' => { self.difficulty = Difficulty::Intermediate; self.start_game(); }
            '3' => { self.difficulty = Difficulty::Expert; self.start_game(); }
            'h' | 'H' => {
                self.state = AppState::BestTimes;
            }
            'q' | 'Q' | KEY_MENU => return false,
            _ => {}
        }
        true
    }

    fn start_game(&mut self) {
        self.field = Some(Minefield::new(self.difficulty));
        self.elapsed_seconds = 0;
        self.game_start_ms = None;
        self.state = AppState::Playing;
        self.save_state();
    }

    // ─── Playing ────────────────────────────────────────────────────────────

    fn handle_playing_key(&mut self, key: char, rng: &Rng, now_ms: u64) -> bool {
        match key {
            KEY_UP => {
                if let Some(ref mut field) = self.field {
                    field.move_cursor(0, -1);
                }
            }
            KEY_DOWN => {
                if let Some(ref mut field) = self.field {
                    field.move_cursor(0, 1);
                }
            }
            KEY_LEFT => {
                if let Some(ref mut field) = self.field {
                    field.move_cursor(-1, 0);
                }
            }
            KEY_RIGHT => {
                if let Some(ref mut field) = self.field {
                    field.move_cursor(1, 0);
                }
            }
            KEY_ENTER | ' ' => {
                if let Some(ref mut field) = self.field {
                    let x = field.cursor_x;
                    let y = field.cursor_y;

                    // Start timer on first reveal
                    if field.game_state == GameState::Ready {
                        self.game_start_ms = Some(now_ms);
                    }

                    field.reveal(x, y, rng);

                    match field.game_state {
                        GameState::Won => {
                            self.state = AppState::Won;
                            self.check_best_time();
                        }
                        GameState::Lost => {
                            self.state = AppState::Lost;
                        }
                        _ => {}
                    }
                }
            }
            'f' | 'F' => {
                if let Some(ref mut field) = self.field {
                    let x = field.cursor_x;
                    let y = field.cursor_y;
                    field.toggle_flag(x, y);
                }
            }
            'r' | 'R' => {
                self.start_game();
            }
            'q' | 'Q' | KEY_MENU | KEY_BACKSPACE => {
                self.state = AppState::Menu;
            }
            _ => {}
        }
        true
    }

    fn check_best_time(&mut self) {
        let time = self.elapsed_seconds;
        let current_best = self.best_time(&self.difficulty);

        if current_best.is_none() || time < current_best.unwrap() {
            self.set_best_time(self.difficulty, time);
            if let Some(ref mut stor) = self.storage {
                stor.save_best_time(&self.difficulty, time);
            }
        }
    }

    // ─── Game Over ──────────────────────────────────────────────────────────

    fn handle_gameover_key(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER | ' ' | 'r' | 'R' => {
                self.start_game();
            }
            'q' | 'Q' | KEY_MENU | KEY_BACKSPACE => {
                self.state = AppState::Menu;
            }
            _ => {}
        }
        true
    }

    // ─── Best Times ─────────────────────────────────────────────────────────

    fn handle_besttimes_key(&mut self, key: char) -> bool {
        match key {
            _ => {
                self.state = AppState::Menu;
            }
        }
        true
    }
}
