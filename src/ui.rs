//! UI rendering for Minesweeper.
//!
//! Draws menu, game grid, game over screens, and best times.

use crate::app::{AppState, MinesApp};
use crate::minefield::{CellContent, CellState, Difficulty, GameState};

use gam::*;

// Layout constants (ecosystem standard)
const SCREEN_WIDTH: isize = 336;
const HEADER_HEIGHT: isize = 30;
const FOOTER_HEIGHT: isize = 46;
const CONTENT_TOP: isize = HEADER_HEIGHT;
const CONTENT_BOTTOM: isize = 536 - FOOTER_HEIGHT;

// Font metrics
const REGULAR_HEIGHT: isize = 15;
const LINE_GAP: isize = 4;
const LINE_HEIGHT: isize = REGULAR_HEIGHT + LINE_GAP;

// Status bar height (mines remaining + timer)
const STATUS_HEIGHT: isize = 24;

/// Draw the complete screen based on current app state.
pub fn draw(app: &MinesApp, gam: &Gam, canvas: graphics_server::Gid) {
    // Clear screen
    let clear_rect = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, 536,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Light,
            graphics_server::PixelColor::Light,
            0,
        ),
    );
    gam.draw_rectangle(canvas, clear_rect).ok();

    match app.state {
        AppState::Menu => draw_menu(app, gam, canvas),
        AppState::Playing | AppState::Won | AppState::Lost => draw_game(app, gam, canvas),
        AppState::BestTimes => draw_best_times(app, gam, canvas),
    }

    gam.redraw().ok();
}

// ─── Header / Footer helpers ────────────────────────────────────────────────

fn draw_header(gam: &Gam, canvas: graphics_server::Gid, title: &str) {
    let bg = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, HEADER_HEIGHT,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark,
            0,
        ),
    );
    gam.draw_rectangle(canvas, bg).ok();

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, 2, SCREEN_WIDTH - 8, HEADER_HEIGHT - 2,
        )),
    );
    tv.style = GlyphStyle::Bold;
    tv.invert = true;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", title).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_footer(gam: &Gam, canvas: graphics_server::Gid, text: &str) {
    let y = CONTENT_BOTTOM;

    let sep = graphics_server::Line::new_with_style(
        Point::new(0, y),
        Point::new(SCREEN_WIDTH, y),
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark,
            1,
        ),
    );
    gam.draw_line(canvas, sep).ok();

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, y + 4, SCREEN_WIDTH - 8, y + FOOTER_HEIGHT - 4,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", text).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_text(gam: &Gam, canvas: graphics_server::Gid,
    x: isize, y: isize, w: isize, text: &str, style: GlyphStyle) {
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            x, y, x + w, y + LINE_HEIGHT,
        )),
    );
    tv.style = style;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", text).ok();
    gam.post_textview(&mut tv).ok();
}

// ─── Menu Screen ────────────────────────────────────────────────────────────

fn draw_menu(app: &MinesApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Minesweeper");

    let mut y = CONTENT_TOP + 20;

    // Title
    draw_text(gam, canvas, 16, y, 300, "Select Difficulty:", GlyphStyle::Regular);
    y += LINE_HEIGHT + 12;

    for (i, diff) in Difficulty::all().iter().enumerate() {
        let selected = i == app.menu_cursor;
        let (w, h, m) = diff.params();

        if selected {
            let hl = graphics_server::Rectangle::new_coords_with_style(
                8, y - 2, SCREEN_WIDTH - 8, y + LINE_HEIGHT + 2,
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Dark,
                    0,
                ),
            );
            gam.draw_rectangle(canvas, hl).ok();
        }

        let label = alloc::format!("{}. {} ({}x{}, {} mines)",
            i + 1, diff.label(), w, h, m);

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                16, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT,
            )),
        );
        tv.style = GlyphStyle::Regular;
        tv.invert = selected;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", label).ok();
        gam.post_textview(&mut tv).ok();

        y += LINE_HEIGHT + 10;
    }

    // Best times summary
    y += 20;
    draw_text(gam, canvas, 16, y, 300, "Best Times:", GlyphStyle::Bold);
    y += LINE_HEIGHT + 4;

    for diff in Difficulty::all() {
        let time_str = match app.best_time(diff) {
            Some(t) => alloc::format!("{}s", t),
            None => alloc::string::String::from("---"),
        };
        let line = alloc::format!("  {}: {}", diff.label(), time_str);
        draw_text(gam, canvas, 16, y, 300, &line, GlyphStyle::Small);
        y += LINE_HEIGHT;
    }

    draw_footer(gam, canvas, "1-3:Select  Enter:Start  H:Times  Q:Quit");
}

// ─── Game Screen ────────────────────────────────────────────────────────────

fn draw_game(app: &MinesApp, gam: &Gam, canvas: graphics_server::Gid) {
    let field = match &app.field {
        Some(f) => f,
        None => return,
    };

    // Header with difficulty
    let title = alloc::format!("Minesweeper - {}", app.difficulty.label());
    draw_header(gam, canvas, &title);

    // Status bar: mines remaining + timer
    let status_y = CONTENT_TOP + 4;
    let mines_text = alloc::format!("Mines: {:>3}", field.mines_remaining());
    let time_text = alloc::format!("Time: {:03}", app.elapsed_seconds);

    draw_text(gam, canvas, 16, status_y, 150, &mines_text, GlyphStyle::Bold);
    draw_text(gam, canvas, 200, status_y, 120, &time_text, GlyphStyle::Bold);

    // Calculate cell size to fit grid in available space
    let grid_top = CONTENT_TOP + STATUS_HEIGHT + 4;
    let grid_height = CONTENT_BOTTOM - grid_top - 4;
    let grid_width = SCREEN_WIDTH - 16; // 8px margin each side

    let cell_w = (grid_width / field.width as isize).min(24);
    let cell_h = (grid_height / field.height as isize).min(24);
    let cell_size = cell_w.min(cell_h); // keep square

    // Center the grid
    let total_grid_w = cell_size * field.width as isize;
    let total_grid_h = cell_size * field.height as isize;
    let grid_x = (SCREEN_WIDTH - total_grid_w) / 2;
    let grid_y = grid_top + (grid_height - total_grid_h) / 2;

    // Draw cells
    for cy in 0..field.height {
        for cx in 0..field.width {
            let px = grid_x + (cx as isize) * cell_size;
            let py = grid_y + (cy as isize) * cell_size;
            let is_cursor = cx == field.cursor_x && cy == field.cursor_y
                && app.state == AppState::Playing;
            let is_death = field.death_cell == Some((cx, cy));

            draw_cell(gam, canvas, px, py, cell_size,
                field.cell(cx, cy), is_cursor, is_death);
        }
    }

    // Footer / overlay for game states
    match app.state {
        AppState::Won => {
            // Victory overlay
            let overlay_y = grid_y + total_grid_h / 2 - 15;
            let bg = graphics_server::Rectangle::new_coords_with_style(
                grid_x, overlay_y,
                grid_x + total_grid_w, overlay_y + 30,
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Light,
                    graphics_server::PixelColor::Light,
                    0,
                ),
            );
            gam.draw_rectangle(canvas, bg).ok();

            let border = graphics_server::Rectangle::new_coords_with_style(
                grid_x + 2, overlay_y + 2,
                grid_x + total_grid_w - 2, overlay_y + 28,
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Light,
                    2,
                ),
            );
            gam.draw_rectangle(canvas, border).ok();

            let win_msg = alloc::format!("YOU WIN! {}s", app.elapsed_seconds);
            draw_text(gam, canvas, grid_x + 10, overlay_y + 6,
                total_grid_w - 20, &win_msg, GlyphStyle::Bold);

            draw_footer(gam, canvas, "Enter:New game  Q:Menu");
        }
        AppState::Lost => {
            draw_footer(gam, canvas, "BOOM! Enter:New game  Q:Menu");
        }
        _ => {
            draw_footer(gam, canvas, "Arrows:Move  Space:Dig  F:Flag  R:New  Q:Menu");
        }
    }
}

fn draw_cell(gam: &Gam, canvas: graphics_server::Gid,
    x: isize, y: isize, size: isize,
    cell: &crate::minefield::Cell, is_cursor: bool, is_death: bool)
{
    let dark = graphics_server::DrawStyle::new(
        graphics_server::PixelColor::Dark,
        graphics_server::PixelColor::Dark,
        0,
    );
    let light = graphics_server::DrawStyle::new(
        graphics_server::PixelColor::Light,
        graphics_server::PixelColor::Light,
        0,
    );
    let outline = graphics_server::DrawStyle::new(
        graphics_server::PixelColor::Dark,
        graphics_server::PixelColor::Light,
        1,
    );

    match cell.state {
        CellState::Hidden => {
            // Filled dark cell with light border
            let style = if is_cursor {
                // Cursor: checkerboard effect — draw outline with thick border
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Light,
                    2,
                )
            } else {
                dark
            };
            let rect = graphics_server::Rectangle::new_coords_with_style(
                x, y, x + size, y + size, style);
            gam.draw_rectangle(canvas, rect).ok();
        }
        CellState::Flagged => {
            // Dark cell with flag marker
            let bg_style = if is_cursor {
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Light,
                    2,
                )
            } else {
                dark
            };
            let rect = graphics_server::Rectangle::new_coords_with_style(
                x, y, x + size, y + size, bg_style);
            gam.draw_rectangle(canvas, rect).ok();

            // Draw "F" in center (inverted)
            if size >= 14 {
                let mut tv = TextView::new(
                    canvas,
                    TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                        x + 2, y + 2, x + size - 2, y + size - 2,
                    )),
                );
                tv.style = GlyphStyle::Small;
                tv.invert = true;
                tv.draw_border = false;
                tv.margin = Point::new(0, 0);
                write!(tv, "F").ok();
                gam.post_textview(&mut tv).ok();
            }
        }
        CellState::Revealed => {
            match cell.content {
                CellContent::Mine => {
                    // Mine — dark background if death cell, outline otherwise
                    let bg_style = if is_death {
                        // Invert: light on dark for the killing mine
                        dark
                    } else {
                        outline
                    };
                    let rect = graphics_server::Rectangle::new_coords_with_style(
                        x, y, x + size, y + size, bg_style);
                    gam.draw_rectangle(canvas, rect).ok();

                    // Draw "*" in center
                    if size >= 14 {
                        let mut tv = TextView::new(
                            canvas,
                            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                                x + 2, y + 2, x + size - 2, y + size - 2,
                            )),
                        );
                        tv.style = GlyphStyle::Small;
                        tv.invert = is_death;
                        tv.draw_border = false;
                        tv.margin = Point::new(0, 0);
                        write!(tv, "*").ok();
                        gam.post_textview(&mut tv).ok();
                    }
                }
                CellContent::Empty(0) => {
                    // Empty revealed cell — light with thin border
                    let rect = graphics_server::Rectangle::new_coords_with_style(
                        x, y, x + size, y + size, outline);
                    gam.draw_rectangle(canvas, rect).ok();
                }
                CellContent::Empty(n) => {
                    // Number cell — light background with number
                    let rect = graphics_server::Rectangle::new_coords_with_style(
                        x, y, x + size, y + size, outline);
                    gam.draw_rectangle(canvas, rect).ok();

                    if size >= 14 {
                        let mut tv = TextView::new(
                            canvas,
                            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                                x + 2, y + 2, x + size - 2, y + size - 2,
                            )),
                        );
                        tv.style = GlyphStyle::Small;
                        tv.draw_border = false;
                        tv.margin = Point::new(0, 0);
                        write!(tv, "{}", n).ok();
                        gam.post_textview(&mut tv).ok();
                    }
                }
            }
        }
    }
}

// ─── Best Times Screen ──────────────────────────────────────────────────────

fn draw_best_times(app: &MinesApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Best Times");

    let mut y = CONTENT_TOP + 30;

    for diff in Difficulty::all() {
        let (w, h, m) = diff.params();
        let time_str = match app.best_time(diff) {
            Some(t) => {
                let mins = t / 60;
                let secs = t % 60;
                if mins > 0 {
                    alloc::format!("{}:{:02}", mins, secs)
                } else {
                    alloc::format!("{}s", secs)
                }
            }
            None => alloc::string::String::from("No record"),
        };

        let label = alloc::format!("{} ({}x{}, {} mines)", diff.label(), w, h, m);
        draw_text(gam, canvas, 16, y, 300, &label, GlyphStyle::Bold);
        y += LINE_HEIGHT + 2;

        let time_line = alloc::format!("  Best: {}", time_str);
        draw_text(gam, canvas, 16, y, 300, &time_line, GlyphStyle::Regular);
        y += LINE_HEIGHT + 16;
    }

    draw_footer(gam, canvas, "Any key: Back to menu");
}
