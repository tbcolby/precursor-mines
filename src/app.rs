//! Key handling, saving and drawing for Minesweeper. Rules, the game clock, cell
//! geometry and the saved record live in `mines-core`.

use gam::menu::{PixelColor, Point};
use gam::GlyphStyle;
use mines_core::clock::Clock;
use mines_core::draw::{cell_ops, cell_size, changed_cells, current_signature, visual, LAYERS};
use mines_core::field::{Difficulty, GameState, Minefield};
use mines_core::saved::{LoadError, Saved};
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::rng::Trng;
use precursor_appkit::screen::Screen;
use precursor_appkit::store::Store;

pub const DICT: &str = "mines.state";
const KEY_STATE: &str = "state";

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Menu,
    Game,
    BestTimes,
}

/// Where an abandon-game confirmation leads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Abandon {
    ToMenu,
    NewGame,
}

pub struct App {
    screen: Screen,
    store: Store,
    trng: Trng,
    tt: ticktimer_server::Ticktimer,
    view: View,
    menu: ListState,
    field: Minefield,
    clock: Clock,
    saved: Saved,
    /// Signature of what is on screen per cell (`0xFF` = nothing drawn yet).
    drawn: Vec<u8>,
    full_redraw: bool,
    last_secs: u32,
    pending: Option<Abandon>,
    dirty: bool,
    /// Set when saved data couldn't be used; blocks saves so it isn't overwritten.
    read_only: bool,
    status: String,
}

impl App {
    pub fn new(screen: Screen, store: Store, trng: Trng) -> Self {
        let mut app = Self {
            screen,
            store,
            trng,
            tt: ticktimer_server::Ticktimer::new().expect("mines: no ticktimer"),
            view: View::Menu,
            menu: ListState::default(),
            field: Minefield::new(Difficulty::Beginner),
            clock: Clock::default(),
            saved: Saved::default(),
            drawn: Vec::new(),
            full_redraw: true,
            last_secs: 0,
            pending: None,
            dirty: false,
            read_only: false,
            status: String::new(),
        };
        app.reload();
        app
    }

    pub fn screen(&self) -> &Screen { &self.screen }

    fn now(&self) -> u64 { self.tt.elapsed_ms() }

    // ---------------------------------------------------------------- storage

    pub fn reload(&mut self) {
        self.read_only = false;
        self.saved = match self.store.read_bytes(KEY_STATE) {
            Ok(None) => Saved::default(),
            Ok(Some(bytes)) => match Saved::from_bytes(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("saved state unusable: {:?}", e);
                    self.read_only = true;
                    self.status = match e {
                        LoadError::Newer(_) => "Saved data is from a newer version; not saving".into(),
                        LoadError::Parse => "Saved data is damaged; not saving".into(),
                    };
                    Saved::default()
                }
            },
            Err(e) => {
                log::error!("couldn't read saved state: {:?}", e.kind());
                self.read_only = true;
                self.status = "Couldn't read saved data; not saving".into();
                Saved::default()
            }
        };
        self.menu.selected = self.saved.difficulty.index();
        self.dirty = false;
        self.draw();
    }

    pub fn save(&mut self) {
        if !self.dirty || self.read_only {
            return;
        }
        match self.store.write_bytes(KEY_STATE, &self.saved.to_bytes()) {
            Ok(()) => self.dirty = false,
            Err(e) => {
                log::error!("save failed: {:?}", e.kind());
                self.status = "Save failed".into();
            }
        }
    }

    // ------------------------------------------------------------------ input

    pub fn on_key(&mut self, key: Key) -> Action {
        let action = match self.view {
            View::Menu => self.menu_key(key),
            View::Game => {
                self.game_key(key);
                Action::Stay
            }
            View::BestTimes => {
                self.view = View::Menu;
                Action::Stay
            }
        };
        if action == Action::Stay {
            self.draw();
        }
        action
    }

    fn menu_key(&mut self, key: Key) -> Action {
        match key {
            k if k.is_back() => return Action::Leave,
            Key::Up => self.menu.step(-1, 3, 3),
            Key::Down => self.menu.step(1, 3, 3),
            Key::Enter | Key::Char(' ') => self.start_game(Difficulty::ALL[self.menu.selected]),
            Key::Char(c @ '1'..='3') => self.start_game(Difficulty::ALL[c as usize - '1' as usize]),
            Key::Char('h') | Key::Char('H') => self.view = View::BestTimes,
            _ => {}
        }
        Action::Stay
    }

    fn start_game(&mut self, d: Difficulty) {
        self.field = Minefield::new(d);
        self.clock.reset();
        self.pending = None;
        self.status.clear();
        self.drawn.clear();
        self.full_redraw = true;
        self.view = View::Game;
        self.menu.selected = d.index();
        if self.saved.difficulty != d {
            self.saved.difficulty = d;
            self.dirty = true;
            self.save();
        }
    }

    fn in_progress(&self) -> bool { self.field.game_state == GameState::Playing && self.field.cells_revealed > 0 }

    /// Leave a game in progress only after a second press, so a stray key doesn't lose it.
    fn abandon(&mut self, to: Abandon) {
        if self.in_progress() && self.pending != Some(to) {
            self.pending = Some(to);
            self.status = "Game in progress: press again to abandon it".into();
            return;
        }
        self.pending = None;
        self.status.clear();
        match to {
            Abandon::ToMenu => {
                self.view = View::Menu;
                self.clock.reset();
            }
            Abandon::NewGame => self.start_game(self.field.difficulty),
        }
    }

    fn game_key(&mut self, key: Key) {
        let over = matches!(self.field.game_state, GameState::Won | GameState::Lost);
        let abandon_keys = key.is_back() || key == Key::Char('q') || key == Key::Char('Q');
        if abandon_keys {
            return self.abandon(Abandon::ToMenu);
        }
        if matches!(key, Key::Char('r') | Key::Char('R')) || (over && matches!(key, Key::Enter | Key::Char(' '))) {
            return self.abandon(Abandon::NewGame);
        }
        // any other key cancels a pending confirmation
        self.pending = None;
        if over {
            return;
        }
        self.status.clear();
        match key {
            Key::Up => self.field.move_cursor(0, -1),
            Key::Down => self.field.move_cursor(0, 1),
            Key::Left => self.field.move_cursor(-1, 0),
            Key::Right => self.field.move_cursor(1, 0),
            Key::Char('f') | Key::Char('F') => {
                let (x, y) = (self.field.cursor_x, self.field.cursor_y);
                self.field.toggle_flag(x, y);
            }
            Key::Enter | Key::Char(' ') => self.dig(),
            _ => {}
        }
    }

    fn dig(&mut self) {
        let (x, y) = (self.field.cursor_x, self.field.cursor_y);
        let was_ready = self.field.game_state == GameState::Ready;
        let now = self.now();
        match self.field.reveal(x, y, &mut self.trng) {
            Ok(true) => {
                // The clock starts on the first cell actually opened, not on a refused key press.
                if was_ready {
                    self.clock.start(now);
                }
            }
            Ok(false) => {}
            Err(_) => {
                self.status = "TRNG error: no board generated".into();
                return;
            }
        }
        match self.field.game_state {
            GameState::Won => {
                self.clock.pause(now);
                let secs = self.clock.secs(now);
                let best = self.saved.record(self.field.difficulty, secs);
                if best {
                    self.dirty = true;
                    self.save();
                }
                self.status = format!("You win! {secs}s{}", if best { " (best)" } else { "" });
            }
            GameState::Lost => {
                self.clock.pause(now);
                self.status = "Boom. Enter or R for a new game".into();
            }
            _ => {}
        }
    }

    // --------------------------------------------------------------- lifecycle

    pub fn on_foreground(&mut self) {
        self.screen.refresh_size();
        if self.view == View::Game && self.field.game_state == GameState::Playing {
            let now = self.now();
            self.clock.resume(now);
        }
        self.full_redraw = true;
        self.draw();
    }

    pub fn on_background(&mut self) {
        let now = self.now();
        self.clock.pause(now);
        self.save();
    }

    pub fn on_tick(&mut self) {
        if self.view == View::Game && self.clock.is_running() {
            let secs = self.clock.secs(self.now());
            if secs != self.last_secs {
                self.draw_status();
                self.screen.flush();
            }
        }
    }

    pub fn redraw(&mut self) {
        self.full_redraw = true;
        self.draw();
    }

    // ---------------------------------------------------------------- drawing

    pub fn draw(&mut self) {
        match self.view {
            View::Menu => self.draw_menu(),
            View::BestTimes => self.draw_best_times(),
            View::Game => self.draw_game(),
        }
        self.screen.flush();
    }

    fn draw_menu(&mut self) {
        let s = &self.screen;
        s.clear();
        let top = s.header("Minesweeper");
        let bottom = s.footer("Enter start · 1-3 · H best times · ⌫ exit");
        let items: Vec<String> = Difficulty::ALL
            .iter()
            .map(|d| {
                let (w, h, m) = d.params();
                format!("{}  {}x{}, {} mines", d.label(), w, h, m)
            })
            .collect();
        s.list(top, bottom, &items, &self.menu);
        if !self.status.is_empty() {
            s.para(bottom - 30, bottom, GlyphStyle::Small, &self.status);
        }
        self.full_redraw = true;
    }

    fn draw_best_times(&mut self) {
        let s = &self.screen;
        s.clear();
        let top = s.header("Best times");
        let bottom = s.footer("Any key: back");
        let mut y = top;
        for d in Difficulty::ALL {
            let t = match self.saved.best(d) {
                Some(secs) => format!("{secs} s"),
                None => "-".to_string(),
            };
            y = s.para(y, bottom, GlyphStyle::Regular, &format!("{}: {}", d.label(), t));
        }
        self.full_redraw = true;
    }

    /// Board geometry: `(cell px, origin x, origin y)`.
    fn geometry(&self) -> (i32, isize, isize) {
        let size = self.screen.size();
        let top = self.screen.line_height(GlyphStyle::Bold) + 4;
        let bottom = size.y - self.screen.line_height(GlyphStyle::Small) - 4;
        let cs = cell_size(size.x as i32 - 2, (bottom - top) as i32, self.field.width, self.field.height);
        let ox = (size.x - (self.field.width as i32 * cs) as isize) / 2;
        (cs, ox.max(0), top)
    }

    fn draw_status(&mut self) {
        let s = &self.screen;
        let size = s.size();
        let h = s.line_height(GlyphStyle::Bold);
        s.fill(Point::new(0, 0), Point::new(size.x - 1, h + 2), PixelColor::Light);
        let secs = self.clock.secs(self.tt.elapsed_ms());
        self.last_secs = secs;
        let line = if self.status.is_empty() {
            format!("Mines {:03}   {}   {:03}", self.field.mines_remaining().max(0), self.field.difficulty.label(), secs)
        } else {
            self.status.clone()
        };
        s.text_in(Point::new(4, 2), Point::new(size.x - 4, h + 2), GlyphStyle::Bold, &line);
        let hint_h = s.line_height(GlyphStyle::Small);
        s.fill(Point::new(0, size.y - hint_h - 3), Point::new(size.x - 1, size.y - 1), PixelColor::Light);
        s.text_in(
            Point::new(4, size.y - hint_h - 2),
            Point::new(size.x - 4, size.y - 1),
            GlyphStyle::Small,
            "Enter dig · F flag · R new · ⌫ menu",
        );
    }

    fn draw_game(&mut self) {
        let (cs, ox, oy) = self.geometry();
        let cells = self.field.width * self.field.height;
        if self.full_redraw || self.drawn.len() != cells {
            self.screen.clear();
            self.drawn = vec![0xFF; cells];
            self.full_redraw = false;
        }
        self.draw_status();

        // Only cells whose picture changed, batched into one call per layer.
        let changed = changed_cells(&self.field, &self.drawn);
        let mut layers: Vec<Vec<(Point, Point)>> = vec![Vec::new(); LAYERS as usize];
        for &i in &changed {
            let (cx, cy) = (i % self.field.width, i / self.field.width);
            let (bx, by) = (ox + cx as isize * cs as isize, oy + cy as isize * cs as isize);
            let cursor = (cx, cy) == (self.field.cursor_x, self.field.cursor_y) && self.field.game_state != GameState::Lost;
            for (layer, (x0, y0, x1, y1)) in cell_ops(visual(&self.field, cx, cy), cs, cursor) {
                layers[layer as usize].push((
                    Point::new(bx + x0 as isize, by + y0 as isize),
                    Point::new(bx + x1 as isize, by + y1 as isize),
                ));
            }
            self.drawn[i] = current_signature(&self.field, i);
        }
        for (n, rects) in layers.into_iter().enumerate() {
            if !rects.is_empty() {
                let color = if n % 2 == 0 { PixelColor::Light } else { PixelColor::Dark };
                self.screen.fill_rects(rects, color);
            }
        }
    }
}
