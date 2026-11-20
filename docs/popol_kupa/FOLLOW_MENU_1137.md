# MLBB Heroes — Popol follow/menu candidate / internal 1.1.137

1.1.136 owner results: in-match animation works; pair movement/facing and dismount work; shield readable; prayer revival and incoming-hit interruption work; Alpha transformation visible. Startup following remains intermittent and the selection card is blank. Possible companion teleport was reported; S2 return and Alpha expiry remain unverified.

## What changed
- Reuses Fanny's map-hook wall grids, but strips Fanny-only synthetic MOBA cable boundaries for Kupa ground movement. Fanny gameplay/maps are unchanged.
- Chooses a stable candidate layout using all living champions and observed owner positions rather than only two moving endpoints. This remains a layout-selection heuristic; the stable SDK provides no simulation game-mode getter.
- Clears stale route waypoints when the selected layout changes. Adds throttled stall/layout diagnostics and recall-start distance logging.
- Large separation while mounted ends the mount rather than warping Kupa to a relocated owner. Ordinary riding still attaches the pair at close range. A new revival still intentionally spawns a fresh companion near Popol.
- Adds a menu-only Popol pixel preview in starting-champion/database cards, retaining transparent gameplay carriers. It draws only in NewGame/DatabaseEdit scenes and delegates every existing Julian audio extension hook unchanged. Live menu-tree discovery/rendering needs owner verification; it is not a static base-sprite replacement.

## Install and focused test
Close TFM2, back up the current installed mod outside mods, replace the folder with this ZIP's mlbb_heroes folder, and keep only one installed copy. Verify version 1.1.137.
1. Open the starting-champion pool; after about a second, is Popol visible in his card? Check a different category and scroll back. There should be no extra sprite over text or another card.
2. Watch several match openings. Does Kupa leave the nexus consistently and rejoin Popol without Popol walking back? Note where any stall occurs.
3. If Popol teleports/returns to base while mounted, Kupa should dismount and travel, rather than jump across the map with him. A fresh spawn after prayer is expected and separate.
4. Observe S2 when Kupa is actually separated: a recall cue should appear while he approaches, and mounting starts on arrival. If already within about 8000 world units, mounting is nearly immediate; no long return is expected.
If any of these fail, return the CURRENT mlbb_heroes.log with the observation. Look for [POPOL navigation], [POPOL recall], and [POPOL menu].

## Validation and limits
379 Rust tests pass, including the outside-cable-boundary follow regression, stable-layout scoring, distant recall/arrival, and mounted relocation. Windows build/package verification is recorded in reports. All tests are automated/mocked; no TFM2 launch here. User art, alpha-adjusted shields and existing 1.1.136 recovery behavior are retained. UI helper tests do not prove the game's actual menu node paths or rendering.

Traps, cooperative stacks, full item inheritance, Alpha area knock-up and final audio remain unfinished. Spear damage remains immediate with a cosmetic projectile. Alpha hit falls back to idle and normal Kupa death art is reused. Keep owner-accepted 1.1.134 as rollback.
