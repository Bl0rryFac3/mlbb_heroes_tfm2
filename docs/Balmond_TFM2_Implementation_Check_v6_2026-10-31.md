# Balmond v6 implementation check — 31 October 2026

## Changes

- S2 forced `CasterAnimation("skill2")` duration: 180 → 12 ticks. Action duration and native 180-tick cyclone window remain 180; no native damage formula, hit count, radius, shred, or move-speed behavior changed. This addresses the known TFM2 behavior where a long `CasterAnimation` holds the unit stationary.
- Add low-volume live-match-only diagnostics around S2 start, passive activation and each of 14 pulse attempts. `targets=0` indicates no eligible enemy within the native 36,000-unit radius at that pulse; missing start/passive lines help isolate whether the data effect or registered passive failed to run. The logs are diagnostic, not proof of damage; check target HP/damage numbers in-game.
- Ult damage `RangeEffect` and native Lethal Counter are unchanged. The old caster-anchored `CasterViewEffect` is removed; an `ViewEffect` is nested in the target fan's effects, following the selected enemy briefly. The single-frame slash atlas is resized to 161x98 (80% of 201x122); FAnim duration remains 0.5.

## Miss Fortune precedent applied

BoDUD Miss Fortune's R uses separate geometry and visual layers: a `DirDot` RangeEffect handles the cone hits, a separate line projectile supplies the visible wave, and a target-position `ViewEffect` marks each hit. For Balmond, the target-impact portion is adapted while retaining the already-working Balmond cone and one-shot execute. Miss Fortune's 40° cone and 12-wave channel are **not** copied.

## Validation and limits

75 Rust tests passed; the Windows x86-64 release DLL cross-build succeeded; v6 static and runtime package audits passed. No TFM2 launch or match was run. This is a test candidate, not a verified gameplay fix.
