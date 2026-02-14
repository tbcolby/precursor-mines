# Precursor Minesweeper

**The universe places the mines.**

```
"Every tile you uncover is a question
the universe has already answered.
You're just reading the results."
```

---

## What This Is

Minesweeper for Precursor, powered by the hardware true random number generator. Every mine placement comes from quantum noise — not seeded algorithms, not deterministic PRNGs. The minefield is as random as physics allows.

Three difficulty levels, first-click safety, flood-fill reveal, flag placement, timer with personal best tracking, all rendered on a 1-bit monochrome display with keyboard navigation.

---

## Why Precursor

**Hardware TRNG.** Mine placement uses Precursor's true random number generator with rejection sampling for uniform distribution. The mines are placed by quantum noise. No seed, no replay, no pattern. Every game is cosmically unique.

**1-bit display.** Minesweeper was originally designed for monochrome displays (Windows 3.1). The 1-bit screen is not a limitation — it's historically authentic. Hidden cells are dark blocks. Revealed cells show clean numbers. The visual contrast between hidden and revealed regions is immediate and unambiguous.

**Physical keyboard.** Arrow keys for cursor movement, Space to dig, F to flag. Faster than mouse-based Minesweeper once you develop the muscle memory. No accidental clicks — every reveal is intentional.

**PDDB storage.** Best times are saved in hardware-encrypted flash. Your records persist across reboots and are private by design.

---

## Features

- **Three difficulties**: Beginner (9x9, 10 mines), Intermediate (16x16, 40 mines), Expert (16x20, 60 mines)
- **First-click safety**: The first cell you reveal is always safe (mine relocated if necessary)
- **Flood fill**: Revealing an empty cell (0 adjacent mines) auto-reveals all connected empty cells
- **Flag placement**: Mark suspected mines with F key
- **Mine counter**: Shows remaining mines (total minus flags placed)
- **Timer**: Starts on first reveal, stops on win/loss, displays seconds (000-999)
- **Best times**: Personal records saved per difficulty level
- **Cursor navigation**: Arrow keys move cursor, visible highlight on current cell
- **Instant restart**: R key starts a new game at the same difficulty

---

## Keyboard Reference

### Menu
| Key | Action |
|-----|--------|
| Up/Down | Select difficulty |
| Enter | Start game |
| 1-3 | Quick-select difficulty |
| H | View best times |
| Q | Quit app |

### Playing
| Key | Action |
|-----|--------|
| Arrow keys | Move cursor |
| Space/Enter | Reveal cell |
| F | Toggle flag |
| R | New game (same difficulty) |
| Q | Back to menu |

### Game Over
| Key | Action |
|-----|--------|
| Enter/Space/R | New game |
| Q | Back to menu |

---

## Cell Display

| State | Appearance |
|-------|-----------|
| Hidden | Dark filled block |
| Flagged | Dark block with "F" |
| Revealed (empty) | Light block, thin border |
| Revealed (1-8) | Light block with number |
| Mine (death) | Dark block with "*" |
| Mine (revealed) | Light block with "*" |
| Cursor | Thick border highlight |

---

## Architecture

```
src/
  main.rs       — Xous event loop, GAM + TRNG + ticktimer init
  app.rs        — State machine (Menu, Playing, Won, Lost, BestTimes)
  minefield.rs  — Core game logic (grid, mines, flood fill, win/loss)
  rng.rs        — TRNG wrapper (rejection sampling, Fisher-Yates)
  ui.rs         — All rendering (menu, grid, cells, overlays)
  storage.rs    — PDDB persistence (difficulty, best times)
```

### Module Independence
- `minefield.rs` depends only on `rng.rs` — zero Xous graphics dependencies
- `rng.rs` depends only on `trng` crate — extractable as library
- `ui.rs` is the only module that uses GAM
- `storage.rs` is the only module that uses PDDB

### First-Click Safety
On the first reveal, `place_mines()` is called with the clicked position. All cells within a radius of 1 from the click are excluded from mine placement candidates. This guarantees the first click opens a safe area (usually triggering a flood fill).

### Grid Sizing
Cell size is calculated dynamically to fit the grid in available screen space while maintaining square cells. Beginner uses larger cells, Expert uses smaller ones.

---

## Build

```bash
cargo build -p mines --target riscv32imac-unknown-xous-elf
cargo xtask renode-image mines
```

---

## PDDB Schema

| Dictionary | Key | Format | Content |
|-----------|-----|--------|---------|
| mines.state | config | JSON | Last difficulty |
| mines.state | best_times | JSON | Best time per difficulty (seconds) |

---

## Complexity

| Metric | Value |
|--------|-------|
| Source files | 6 |
| Estimated LOC | ~1,900 |
| App states | 5 |
| PDDB dictionary | 1 |
| PDDB keys | 2 |
| Hardware dependency | TRNG + Ticktimer |

---

## License

Apache 2.0 — see LICENSE file.
