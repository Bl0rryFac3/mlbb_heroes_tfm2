# Chou Ultimate range investigation — v1.1.39

**Date:** 2026-10-26  
**Input:** `/home/user/uploads/mlbb_heroes.log` (22,335 lines; 3,027,404 bytes)  
**Scope:** Explain the far first-phase Ultimate / no-knockback report, preserve the long recast chase, and test the user-requested 100,000-unit first-phase knockback.

## Finding

The report is consistent with the implementation. The v1.1.38 champion data gives the *shared Ultimate action* a 145,000-unit target range so phase 2 can chase a displaced enemy. The first-phase native callback only checks distance later, when its effect starts, and rejects targets beyond 30,000. That creates a bad gap: an out-of-range input/action can look like an Ultimate cast, but the native effect returns before damage, knockback, airborne, and recast setup. A rejected callback cannot knock the target back.

The log contains **126 first-phase range rejections** at distances **31,720–164,905** (median 133,801), against 30,000. The live/client-match-origin trace includes a rejection at tick 33,648 (about 9:21 at 60 ticks/sec) with the target **158,507** away—close to the user's reported ~9:30 window. This is direct evidence of an out-of-range first-phase attempt; the log cannot prove exactly what animation was visible on screen.

The same log also confirms the intended close-kick → long-chase behavior does occur in a presimulation: at tick 35,252 (~9:47), caster/target were about **13,984** apart, the first phase logged `knockback=true`, and 11 ticks later the target position was about **60,000** units from its start. The recast then executed at tick 35,359 while the target was roughly 74,000 away. This is a presimulation trace, not a live-match visual confirmation. It also shows why the first phase and recast need different ranges.

## Range comparison

- Chou's current data basic-attack target range: **15,000**.
- v1.1.38 shared Ultimate data range: **145,000** — about **9.7×** the basic-attack range.
- v1.1.38 first-phase native guard: **30,000** — twice the basic-attack range, but checked only inside the effect callback.
- The prior setting was **6,000 speed × 10 ticks**, and the presim trace observed about 60,000 units of displacement. The user requested testing **10,000 × 10 = 100,000 units**; this is a deliberate +67% experimental increase, not a live-verified result.

The MLBB references describe phase 1 as a roundhouse kick that knocks the target back, followed by a recast that chases that target ([Liquipedia Chou](https://liquipedia.net/mobilelegends/Chou)). Patch history separately records reductions to first-kick displacement—the 1.8.92 notes say the first kick distance was reduced by 20% ([Liquipedia patch notes](https://liquipedia.net/mobilelegends/Patch_1.8.92)); 1.8.86 Advanced Server notes also reduced maximum first-stage knockback from 5 to 4 ([Fandom patch notes](https://mobile-legends.fandom.com/wiki/Patch_1.8.86_(Advanced_Server))). Those are **knockback-distance** changes, not evidence that phase 1 should target at 145,000. I found no authoritative published numeric cast radius in the same TFM2 world-unit scale; using approximately basic-attack reach is the user's intended tuning, not a claim that MLBB documents an identical value.

## Candidate correction in source (v1.1.39)

- Set the shared action's **base range to 20,000**, modestly wider than the 15,000 basic-attack range.
- Set the native first-phase hard guard to the same **20,000**; set the AI first-phase trigger to **18,000**.
- Add an outer, range-only native marker whose `range_adjust` adds **125,000 only while `mlbb_chou_ult_recast` is present**. That makes the calculated total 20,000 + 125,000 = **145,000 during recast only**, retaining the far target chase without exposing phase 1 to that range. TFM2's Stable API documents dynamic range as an additive per-caster/target adjustment read by range checks, including AI checks ([Stable Native Mods docs](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/stable-native-mods.md); [Stable API reference](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/stable-api-reference.md)).
- Preserve all v1.1.38 S1/combo changes and test the requested first-phase knockback at **10,000 × 10 = 100,000**.

## Validation and remaining runtime test

Source checks passed after the 100,000-unit adjustment: `cargo fmt --check`, **36 Rust tests**, Windows x64 release build, Chou integration validator, Windows DLL verification, and ZIP CRC/package-data checks. These verify compilation and the intended range arithmetic, not TFM2's actual target validator. The critical live test is that phase 1 cannot start on a target beyond 20,000, while a recast on the already-kicked target still accepts a target around 60,000–100,000 away and rejects beyond 145,000. Also verify a close first-phase kick still visibly displaces the target and that the recast animates/chases it.

The Windows v1.1.39 ZIP is built after the user authorized cleanup of older, irrelevant archives. To stay conservative, only the four v1.1.27/.28 Linux/Windows ZIPs were removed; their small audit JSONs and the v1.1.35/.36/.37/.38 rollback ZIPs remain. The v1.1.39 archive includes the 100,000-unit knockback experiment; its successful tests validate code and packaging, not TFM2 runtime behavior.
