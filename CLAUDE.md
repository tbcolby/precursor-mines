# Minesweeper — Agent Instructions

## App Identity
- **Package**: `mines`
- **SERVER_NAME**: `_Minesweeper_`
- **APP_NAME**: `Mines`
- **manifest context_name**: `Mines`

## Architecture
- **UX Type**: `UxType::Chat` with raw keyboard input
- **State Machine**: 5 states (Menu, Playing, Won, Lost, BestTimes)
- **Threading**: None — flood fill is fast enough on 100MHz
- **PDDB**: 1 dictionary (`mines.state`), 2 keys
- **Timer**: Uses `ticktimer_server::Ticktimer::elapsed_ms()`

## Module Organization
- `minefield.rs` — Core game logic. **Zero GAM dependencies.**
  - Cell grid with Hidden/Flagged/Revealed states
  - Mine placement with first-click safety (radius 1 exclusion)
  - Iterative BFS flood fill (not recursive — no stack overflow risk)
  - Adjacency counting, win/loss detection, flag management
- `rng.rs` — TRNG wrapper (same pattern as Decision Engine)
- `app.rs` — State machine. Handles keys, timer updates, best time tracking.
- `ui.rs` — Grid rendering with dynamic cell sizing. Only GAM user.
- `storage.rs` — PDDB for difficulty preference and best times.

## Key Design Decisions
1. **Iterative flood fill** — BFS with a stack vector, not recursive. A 16x20 Expert grid could recurse hundreds of levels deep; iterative approach is safe.
2. **Dynamic cell sizing** — `cell_size = min(avail_width/cols, avail_height/rows)` with max of 24px. Grid is centered in content area.
3. **First-click safety radius** — Excludes the clicked cell AND all 8 neighbors from mine placement. Guarantees first click opens an area, not just a single cell.
4. **Timer via ticktimer** — `elapsed_ms()` called on each key event and redraw. Start time recorded on first reveal.

## Cell Rendering Strategy
- Hidden: dark filled rectangle (cursor adds thick 2px border)
- Flagged: dark filled + "F" text (inverted)
- Revealed empty: light rectangle with 1px dark border
- Revealed number: same + number text
- Mine: dark bg with "*" if death cell, outline + "*" otherwise
- Text rendering uses `GlyphStyle::Small` to fit in cells as small as 14px

## Build
```bash
cargo build -p mines --target riscv32imac-unknown-xous-elf
cargo xtask renode-image mines
```
