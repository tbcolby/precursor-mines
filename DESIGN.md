# MINESWEEPER
## Design Specification

```
"The only winning move is to not click randomly."
    — Every Minesweeper player, eventually

"But what if random was truly random?"
    — Precursor
```

---

## Problem Statement

Minesweeper is the perfect game for a device with a hardware true random number generator. Every mine placement is quantum noise — not `Math.random()`, not a Mersenne Twister, not a Linear Congruential Generator. The universe places the mines.

On a monochrome display, Minesweeper is actually *better* than on a color screen. The original Windows Minesweeper used colors to distinguish numbers (blue 1, green 2, red 3...). But experienced players don't rely on color — they rely on pattern recognition. A 1-bit display forces clean typography and clear cell boundaries. No visual clutter. Just the grid and your logic.

---

## Precursor Fit

| Constraint | Advantage |
|-----------|-----------|
| Hardware TRNG | Truly random mine placement — provably fair |
| 1-bit display | Clean grid rendering, no color-coding confusion |
| Physical keyboard | Arrow keys for navigation, Space/Enter for reveal, F for flag |
| PDDB encrypted | Save best times — nobody can peek at your stats |
| 336x536 display | Fits 16x16 grid at 18px cells with room for header/status |
| No network | No leaderboard cheating — your times are your own |

---

## Difficulty Levels

| Level | Grid | Mines | Density |
|-------|------|-------|---------|
| Beginner | 9x9 | 10 | 12.3% |
| Intermediate | 16x16 | 40 | 15.6% |
| Expert | 16x20 | 60 | 18.8% |

Expert is 16 columns (max for 336px at 20px cells) by 20 rows (fits in content area).

---

## Cell Display

```
Hidden:     [###]    (filled/dark)
Flagged:    [!F!]    (flag marker)
Revealed 0: [   ]    (empty, blank)
Revealed N: [ N ]    (number 1-8)
Mine (boom): [*X*]   (game over reveal)
Mine (safe): [ * ]   (end-game reveal)
```

---

## Screen Layout

```
+----------------------------------+  0
|  MINESWEEPER  Beginner           |  Header (30px)
+----------------------------------+  30
|                                  |
|  Mines: 10    Time: 042          |  Status bar
|                                  |
|  ### ### ### ### ### ### ### ###  |
|  ### ### ### ### ### ### ### ###  |
|  ###  2   1  ###  1             |  Game grid
|   1   1   1   1   1              |
|                                  |
|              etc.                |
|                                  |
+----------------------------------+  490
|  Arrows:Move  Space:Dig  F:Flag |  Footer
+----------------------------------+  536
```

---

## Game States

```
         ┌──────────┐
         │   Menu   │
         └────┬─────┘
              │
         ┌────▼─────┐
         │  Playing  │◄── New Game
         └────┬─────┘
              │
        ┌─────┴─────┐
        ▼           ▼
   ┌────────┐  ┌────────┐
   │  Won!  │  │  Lost  │
   └────┬───┘  └────┬───┘
        │           │
        └─────┬─────┘
              ▼
         ┌─────────┐
         │ Menu /   │
         │ New Game │
         └─────────┘
```

---

## Keyboard Mapping

### Menu
| Key | Action |
|-----|--------|
| Up/Down | Select difficulty |
| Enter | Start game |
| H | View high scores |
| Q | Quit app |

### Playing
| Key | Action |
|-----|--------|
| Arrows | Move cursor |
| Space/Enter | Reveal cell |
| F | Toggle flag |
| R | New game (same difficulty) |
| Q | Back to menu |

### Game Over / Win
| Key | Action |
|-----|--------|
| Enter/Space | New game |
| Q | Back to menu |

---

## First-Reveal Safety

The first cell revealed is always safe (no mine). If a mine is placed there, it is relocated to the first empty cell. This is standard Minesweeper behavior.

---

## Flood Fill

When a cell with 0 adjacent mines is revealed, all adjacent cells are recursively revealed (flood fill). This continues until cells with non-zero counts are reached (which are revealed but not expanded further).

---

## Timer

- Starts on first reveal
- Stops on win or loss
- Displays seconds elapsed (000-999)
- Best times saved per difficulty level

---

## PDDB Schema

### Dictionary: `mines.state`
| Key | Format | Description |
|-----|--------|-------------|
| `config` | JSON | `{ "difficulty": "beginner" }` |
| `best_times` | JSON | `{ "beginner": 42, "intermediate": 180, "expert": null }` |

---

## Complexity Estimate

| Metric | Estimate |
|--------|----------|
| Total LOC | ~1,800-2,200 |
| Modules | 5 (main, app, minefield, ui, storage) |
| Threading | None |
| PDDB | 1 dictionary, 2 keys |
| New dependency | trng (hardware TRNG) |
| Key challenge | Flood fill performance on 16x20 grid |
