# Precursor Minesweeper

Minesweeper for [Precursor](https://precursor.dev). Mines are placed from the hardware TRNG with an unbiased selection (every legal layout is equally likely), and the first click is always safe.

Version 0.2. Not yet run on Renode or hardware: `capture.py` is written but unrun.

## Features

- Three difficulties: Beginner 9x9 / 10 mines, Intermediate 16x16 / 40, Expert 16x20 / 60.
- First-click safety: the clicked cell and its eight neighbours never hold a mine, so the first dig opens an area.
- Flood fill on empty cells (iterative; flagged cells stay closed), flags, mine counter.
- A clock that counts only while the game is in front and stops exactly at the winning or losing move. It ticks once a second from a pump that stops when the app is in the background.
- Best time per difficulty, saved.
- After a loss: all mines are shown, the mine you hit is marked, and flags on safe cells are crossed out.
- Abandoning a game in progress (Q, Backspace, F4 or R) asks for a second press first.
- The board is drawn from rectangles (digits are a pixel font), batched into a few GAM calls per move, and only changed cells are redrawn.

## Keys

Every screen leaves with Backspace or F4 (the `∴` menu key belongs to the OS).

| Screen | Keys |
|--------|------|
| Menu | Up/Down, Enter or Space start, 1-3 pick a difficulty, H best times, Backspace/F4 leave the app |
| Game | Arrows move, Enter or Space dig, F flag, R new game, Q / Backspace / F4 back to the menu |
| Game over | Enter, Space or R new game; Q / Backspace / F4 menu |
| Best times | any key back |

## Storage

Dictionary `mines.state`, one key `state`: a versioned JSON record with the best times and last difficulty. It is written only when it changes (a new best, a different difficulty, losing focus), by replacing the key, and reloaded when the set of open PDDB bases changes. A record that can't be read or is from a newer version is left alone and the app doesn't save over it. A game in progress is not saved.

## Layout

```
core/          mines-core: field, clock, draw (cell rectangles), saved
src/main.rs    registration, 1 Hz pump, message routing
src/app.rs     keys -> core, saving, drawing
```

Building: `bin/integrate --xous-root <xous-core> --grade <this repo>` from [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit). Host tests: `cd core && cargo test` (28 tests). Manifest: `manifest-fragment.json`, key `mines` (GAM app name `gam::APP_NAME_MINES`).

## Fixed from the 0.1 audit

- The flood-fill borrow error (E0502/E0506) that stopped the crate compiling, and its dead re-check.
- Best time understated: the clock is now stopped at the winning move instead of reading the previous key press's time.
- Clock froze while thinking (no timer): 1 Hz pump, stopped on Background; background time no longer counts.
- Clock started on a key press that dug nothing (a flagged cell); it now starts on the first cell actually opened.
- Whole-board redraw per key (one IPC per cell plus one TextView per number): now changed cells only, batched.
- TRNG failure no longer becomes `0`; it shows an error and no board is made.
- `UxType::Chat` and a hard-coded 336x536 layout: framebuffer app sized from the canvas.
- Best-times JSON corrupted by shorter rewrites (no `set_len`): the key is replaced whole.
- Quit terminated the process; it now leaves via `relinquish_focus`.

## Limitations

- No chording (revealing around a satisfied number) and no game save/resume.
- The cell size is chosen to fit the canvas, so Expert cells are about 21 px; digits are drawn with a 3x5 pixel font.
- "True random" means the Precursor TRNG as exposed by the OS; this app doesn't test it.

## Author

Made by Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Apache-2.0, see [LICENSE](LICENSE).
