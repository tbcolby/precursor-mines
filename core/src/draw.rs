//! Cell geometry as layered rectangles.
//!
//! Each cell is drawn from a list of `(layer, rect)`. Layers go light, dark, light, dark
//! (`layer % 2 == 0` is light), so the shell can gather the rectangles of every changed
//! cell and draw each layer with one batched call. Digits are a 3x5 pixel font scaled up,
//! so a board needs no TextViews (0.1 posted one per number, up to ~250 on Expert).
//! Rectangles are inclusive at the bottom-right, like GAM's.

use crate::field::{CellContent, CellState, GameState, Minefield};

/// `(x0, y0, x1, y1)` inclusive, relative to the cell's top-left.
pub type Rect = (i32, i32, i32, i32);

pub const LIGHT_BODY: u8 = 0;
pub const DARK_BODY: u8 = 1;
pub const LIGHT_DETAIL: u8 = 2;
pub const DARK_DETAIL: u8 = 3;
pub const LAYERS: u8 = 4;

/// What a cell looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visual {
    Hidden,
    Flag,
    /// Revealed with this many adjacent mines (0 draws blank).
    Number(u8),
    /// A mine shown after a loss.
    Mine,
    /// The mine that was stepped on.
    Boom,
    /// A flag on a safe cell, shown after a loss.
    WrongFlag,
}

/// The visual of one cell of `f`.
pub fn visual(f: &Minefield, x: usize, y: usize) -> Visual {
    let c = f.cell(x, y);
    match (c.state, c.content) {
        (CellState::Hidden, _) => Visual::Hidden,
        (CellState::Flagged, _) if f.is_wrong_flag(x, y) => Visual::WrongFlag,
        (CellState::Flagged, _) => Visual::Flag,
        (CellState::Revealed, CellContent::Mine) => {
            if f.death_cell == Some((x, y)) {
                Visual::Boom
            } else {
                Visual::Mine
            }
        }
        (CellState::Revealed, CellContent::Empty(n)) => Visual::Number(n),
    }
}

/// A code that changes whenever the picture of the cell would change (visual + cursor).
pub fn signature(v: Visual, cursor: bool) -> u8 {
    let base = match v {
        Visual::Hidden => 0,
        Visual::Flag => 1,
        Visual::Number(n) => 2 + n.min(8),
        Visual::Mine => 11,
        Visual::Boom => 12,
        Visual::WrongFlag => 13,
    };
    base | if cursor { 0x80 } else { 0 }
}

/// Largest square cell size that fits `cols x rows` in `w x h`, at least 4 px.
pub fn cell_size(w: i32, h: i32, cols: usize, rows: usize) -> i32 {
    ((w / cols.max(1) as i32).min(h / rows.max(1) as i32)).max(4)
}

// 3x5 digit font, one row per entry, bit 2 = left column.
const DIGITS: [[u8; 5]; 9] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0 (unused, blank is drawn instead)
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
];

/// Rectangles of digit `n` (1-8) centred in a `cs` cell: horizontal runs per font row.
pub fn digit_rects(n: u8, cs: i32) -> Vec<Rect> {
    let Some(rows) = DIGITS.get(n as usize).filter(|_| (1..=8).contains(&n)) else { return Vec::new() };
    let s = ((cs - 4) / 5).max(1);
    let (x0, y0) = ((cs - 3 * s) / 2, (cs - 5 * s) / 2);
    let mut out = Vec::new();
    for (r, bits) in rows.iter().enumerate() {
        let mut c = 0;
        while c < 3 {
            if bits & (0b100 >> c) != 0 {
                let start = c;
                while c < 3 && bits & (0b100 >> c) != 0 {
                    c += 1;
                }
                out.push((x0 + start * s, y0 + r as i32 * s, x0 + c * s - 1, y0 + (r as i32 + 1) * s - 1));
            } else {
                c += 1;
            }
        }
    }
    out
}

fn flag_rects(cs: i32) -> Vec<Rect> {
    let u = (cs / 8).max(1);
    let (px, top, bot) = (cs / 2 - u / 2, cs / 5, cs * 3 / 4);
    vec![
        (px, top, px + u - 1, bot),                          // pole
        (px + u, top, px + u + (cs / 4).max(2) - 1, top + (cs / 4).max(2) - 1), // cloth
        (px - u * 2, bot + 1, px + u * 3 - 1, bot + u),      // base
    ]
}

fn mine_rects(cs: i32) -> Vec<Rect> {
    let m = cs / 2;
    let r = (cs / 6).max(1); // body half size
    let arm = (cs / 3).max(2);
    vec![
        (m - r, m - r, m + r, m + r),
        (m - arm, m, m + arm, m),
        (m, m - arm, m, m + arm),
    ]
}

fn cross_rects(cs: i32) -> Vec<Rect> {
    let (a, b) = (2, cs - 3);
    (0..=(b - a)).flat_map(|i| [(a + i, a + i, a + i, a + i), (b - i, a + i, b - i, a + i)]).collect()
}

/// The layered rectangles of a cell. The cursor is a 1 px frame: light on a dark cell, dark on a light one.
pub fn cell_ops(v: Visual, cs: i32, cursor: bool) -> Vec<(u8, Rect)> {
    let last = cs - 1;
    let mut ops: Vec<(u8, Rect)> = vec![(LIGHT_BODY, (0, 0, last, last))];
    let dark_cell = matches!(v, Visual::Hidden | Visual::Flag | Visual::Boom);
    if dark_cell {
        // 1 px gap on the right and bottom keeps cells distinct
        ops.push((DARK_BODY, (0, 0, last - 1, last - 1)));
    }
    match v {
        Visual::Hidden => {}
        Visual::Flag => ops.extend(flag_rects(cs).into_iter().map(|r| (LIGHT_DETAIL, r))),
        Visual::Number(n) => ops.extend(digit_rects(n, cs).into_iter().map(|r| (DARK_DETAIL, r))),
        Visual::Mine => ops.extend(mine_rects(cs).into_iter().map(|r| (DARK_DETAIL, r))),
        Visual::Boom => ops.extend(mine_rects(cs).into_iter().map(|r| (LIGHT_DETAIL, r))),
        Visual::WrongFlag => ops.extend(cross_rects(cs).into_iter().map(|r| (DARK_DETAIL, r))),
    }
    if cursor {
        let (layer, e) = if dark_cell { (LIGHT_DETAIL, last - 1) } else { (DARK_DETAIL, last) };
        let o = if dark_cell { 1 } else { 0 };
        let e = e - if dark_cell { 1 } else { 0 };
        ops.extend([
            (layer, (o, o, e, o)),
            (layer, (o, e, e, e)),
            (layer, (o, o + 1, o, e - 1)),
            (layer, (e, o + 1, e, e - 1)),
        ]);
    }
    ops
}

/// Which cells need redrawing, given what was drawn last (`drawn[i]` is a signature, `0xFF` = nothing yet).
pub fn changed_cells(f: &Minefield, drawn: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    for y in 0..f.height {
        for x in 0..f.width {
            let i = y * f.width + x;
            let sig = signature(visual(f, x, y), (x, y) == (f.cursor_x, f.cursor_y) && f.game_state != GameState::Lost);
            if drawn.get(i) != Some(&sig) {
                out.push(i);
            }
        }
    }
    out
}

/// The signature of cell `i` right now (to store after drawing it).
pub fn current_signature(f: &Minefield, i: usize) -> u8 {
    let (x, y) = (i % f.width, i / f.width);
    signature(visual(f, x, y), (x, y) == (f.cursor_x, f.cursor_y) && f.game_state != GameState::Lost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Difficulty;

    const ALL: [Visual; 14] = [
        Visual::Hidden, Visual::Flag, Visual::Number(0), Visual::Number(1), Visual::Number(2), Visual::Number(3),
        Visual::Number(4), Visual::Number(5), Visual::Number(6), Visual::Number(7), Visual::Number(8),
        Visual::Mine, Visual::Boom, Visual::WrongFlag,
    ];

    #[test]
    fn every_rect_stays_inside_the_cell_at_every_size() {
        for cs in 6..=48 {
            for v in ALL {
                for cursor in [false, true] {
                    for (layer, (x0, y0, x1, y1)) in cell_ops(v, cs, cursor) {
                        assert!(layer < LAYERS);
                        assert!(x0 >= 0 && y0 >= 0 && x1 <= cs - 1 && y1 <= cs - 1, "cs={cs} {v:?} cursor={cursor}: {x0},{y0},{x1},{y1}");
                        assert!(x0 <= x1 && y0 <= y1, "cs={cs} {v:?}: empty rect {x0},{y0},{x1},{y1}");
                    }
                }
            }
        }
    }

    #[test]
    fn digits_are_distinct_nonempty_and_centred() {
        let cs = 21;
        let mut seen = Vec::new();
        for n in 1..=8u8 {
            let r = digit_rects(n, cs);
            assert!(!r.is_empty());
            assert!(!seen.contains(&r), "digit {n} duplicates another");
            let (min_x, max_x) = (r.iter().map(|r| r.0).min().unwrap(), r.iter().map(|r| r.2).max().unwrap());
            assert!(((min_x) - (cs - 1 - max_x)).abs() <= 1, "digit {n} off-centre: {min_x}..{max_x}");
            seen.push(r);
        }
        assert!(digit_rects(0, cs).is_empty() && digit_rects(9, cs).is_empty());
    }

    #[test]
    fn digit_pixel_count_matches_the_font() {
        // scale 1 (cs 9): count pixels, compare with the bit patterns
        for n in 1..=8u8 {
            let px: i32 = digit_rects(n, 9).iter().map(|r| (r.2 - r.0 + 1) * (r.3 - r.1 + 1)).sum();
            let want: u32 = DIGITS[n as usize].iter().map(|b| b.count_ones()).sum();
            assert_eq!(px as u32, want, "digit {n}");
        }
        // an 8 has 13 lit pixels, a 1 has 8
        assert_eq!(DIGITS[8].iter().map(|b| b.count_ones()).sum::<u32>(), 13);
    }

    #[test]
    fn dark_body_leaves_a_gap_and_light_cells_have_no_dark_body() {
        let hidden = cell_ops(Visual::Hidden, 10, false);
        assert!(hidden.contains(&(DARK_BODY, (0, 0, 8, 8))));
        assert!(!cell_ops(Visual::Number(3), 10, false).iter().any(|o| o.0 == DARK_BODY));
        assert!(!cell_ops(Visual::Mine, 10, false).iter().any(|o| o.0 == DARK_BODY));
    }

    #[test]
    fn cursor_frame_is_visible_on_both_backgrounds() {
        let on_dark = cell_ops(Visual::Hidden, 12, true);
        assert!(on_dark.iter().any(|o| o.0 == LIGHT_DETAIL));
        let on_light = cell_ops(Visual::Number(2), 12, true);
        assert!(on_light.iter().filter(|o| o.0 == DARK_DETAIL).count() > digit_rects(2, 12).len());
    }

    #[test]
    fn signatures_distinguish_pictures() {
        let mut sigs: Vec<u8> = ALL.iter().flat_map(|&v| [signature(v, false), signature(v, true)]).collect();
        sigs.sort_unstable();
        // Number(0) through Number(8) and others are all different pictures
        sigs.dedup();
        assert_eq!(sigs.len(), ALL.len() * 2);
    }

    #[test]
    fn cell_size_fits() {
        assert_eq!(cell_size(336, 440, 16, 20), 21);
        assert_eq!(cell_size(336, 440, 9, 9), 37);
        assert_eq!(cell_size(10, 10, 16, 20), 4, "never below 4");
    }

    #[test]
    fn changed_cells_tracks_cursor_and_state() {
        let mut f = Minefield::with_size(4, 4, 0, Difficulty::Beginner);
        f.game_state = GameState::Playing;
        let mut drawn = vec![0xFFu8; 16];
        assert_eq!(changed_cells(&f, &drawn).len(), 16, "first draw is everything");
        for i in 0..16 {
            drawn[i] = current_signature(&f, i);
        }
        assert!(changed_cells(&f, &drawn).is_empty());
        let before = (f.cursor_x, f.cursor_y);
        f.move_cursor(1, 0);
        let ch = changed_cells(&f, &drawn);
        assert_eq!(ch.len(), 2, "old and new cursor cells: {before:?}");
        f.toggle_flag(0, 0);
        assert!(changed_cells(&f, &drawn).contains(&0));
    }

    #[test]
    fn visuals_follow_state() {
        let mut f = Minefield::with_size(4, 4, 0, Difficulty::Beginner);
        f.game_state = GameState::Playing;
        assert_eq!(visual(&f, 0, 0), Visual::Hidden);
        f.toggle_flag(0, 0);
        assert_eq!(visual(&f, 0, 0), Visual::Flag);
    }
}
