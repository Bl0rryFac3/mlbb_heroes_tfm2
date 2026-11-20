# MLBB Heroes — Popol performance/icon hotfix candidate / internal 1.1.140

Owner reports severe lag in BOTH menus and matches; disabling the mod removes it. Compact ban icons and selected-champion art are blank. 1.1.139 is blocked on this regression. No new kit features until performance is confirmed.

## Performance changes
- Popol UI overlay no longer compiled/registered: no tree walks, widget polling or card drawing. Original Julian voice extension registered directly; audio.rs unchanged.
- Global driver indexes only current-scope owners instead of scanning historical states. Simulation-copy isolation retained.
- Duplicate processed ticks skip cosmetics; duplicate visual calls removed. Pose/effect identity caching eliminates repeated buff scans/writes. ServerPresim skips cosmetic work, retaining combat/movement/recovery.
- Wall grids/layout selection cached behind Arc: normal rescore every 120 ticks; incompatible-endpoint early recheck no more than every 15 ticks. AI copies only decision fields instead of cloning route/history. Fanny map/cable behavior unchanged.

## Shared icon asset
A visible first idle frame supplies native static UI portraits/face crops, bans and selected slots. Timed gameplay advances past a one-microsecond presentation frame into a transparent carrier for 24 hours; other action tags are transparent. Existing body layers still draw the match character. No invisibility/banish, targetability or damage-protection changes.
This asset approach replaces scanning overlays. Compiler checks visible portrait/face pixels, unchanged supplied art, valid bounds and transparent timed carrier frames. Actual host icon paths and animation-start behavior still need live verification. Check for an unwanted extra standing Popol while mounted or when pausing at a transition.
Rebuild with tools/compile_popol_shared_preview.py AFTER other carrier rebuilds. compile_popol_probe.py is historical diagnostic tooling.

## Verification
397 active Rust tests pass, 0 fail; one ignored manual performance probe. Removed UI-scanner module/tests are no longer compiled. Combat regressions remain; new checks cover duplicate-tick work budgets, cached poses, headless damage without cosmetics and grid reuse. Windows/package reports included.
Synthetic release-mode fixture: 2000 ticks with 10000 historical states, 81.553 ms before vs 3.168 ms after (~25.7x lower elapsed time); 1000 map selections, 12.447 ms vs 0.042 ms. These are mocked workloads, NOT live FPS claims. No actual TFM2 launch here. Logs and reproducible ignored test included.

## Install and focused test
Close the game, back up the current mod OUTSIDE mods, replace with this ZIP's mlbb_heroes folder, keep only one installed copy, verify 1.1.140.
1. Menus/draft: compare scrolling/responsiveness with mod off/on. Is severe stutter gone?
2. Match/simulation: compare playback and pause/unpause with Popol present; a match without him also helps identify remaining overhead.
3. Check starting-selection art, compact ban icons and selected champion displays. Mounted Popol must be one seated character with no extra standing body/flash.
4. No full kit retest requested; report incidental regressions in spears, Alpha, following or revival.
If lag/missing art remains, send the current mlbb_heroes.log plus affected screen/match. Keep the mod disabled or use the pre-Popol 1.1.134 backup if unusable; 1.1.139 is not recommended as a performance rollback.

Spear-arrival and Alpha S1 area behavior retained; live combat validation pending. Traps/passive/items and balance/audio remain unfinished. Received original sprites/VFX unchanged.
