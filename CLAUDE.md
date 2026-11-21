# Minesweeper: agent notes

- Package `mines`; GAM app name `gam::APP_NAME_MINES` (manifest key `mines`, context name "Mines"); xous-names server `_Mines_`.
- `core/` (`mines-core`) holds the field, first-click-safe placement, game clock, cell geometry and the saved record, with no Xous dependencies. Run `cd core && cargo test`.
- Mine placement uses `precursor_appkit_core::entropy::Entropy` (fallible, unbiased). A TRNG failure leaves the board unplaced.
- `src/` is the shell: `UxType::Framebuffer`, raw keys, a 1 Hz pump that stops on Background, `Store` (dictionary `mines.state`, key `state`), board drawn with `Screen::fill_rects` from `mines_core::draw` layers.
- Gate: `xous-dev-toolkit/bin/integrate --xous-root <tree> --grade <repo>`.
