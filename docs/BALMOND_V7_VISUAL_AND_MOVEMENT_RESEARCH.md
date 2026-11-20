# Balmond v7 visual and movement research — 2026-10-31

## Why the v6 Ultimate repeated

V6 put `ViewEffect(mlbb_balmond_ult_slash)` inside Balmond's directional `RangeEffect` target list. A `RangeEffect` runs its children for each selected target; therefore each enemy hit received its own slash. This matches the screenshot, but not the requested one-shot, line-wide presentation. It also used a target-position effect, which is not the right binding for a caster-to-direction line.

The test_mod Sion direction skill (`sion.data_champion`, Drive file `1svhyLlNVx9JGjdhvxZjoXTGaI5eF4egD`) demonstrates the relevant pattern: `casting_type: Direction` plus one `LineRangeProjectile` (`width: 25000`, `length: 75000`) whose `name` is registered in `view_projectiles` as an `Animated` binding to `sion_skill`. The official effects schema describes `LineRangeProjectile` as a delayed line area from the caster toward the input direction/target/position. This is a single directional visual binding, not a child effect duplicated for every enemy in a fan.

References:
- [Official effects schema — LineRangeProjectile](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/effects.md#linerangeprojectile)
- [Official visual binding schema — projectile visuals](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/visual-bindings.md#projectile-visuals)
- [test_mod Sion data](https://drive.google.com/file/d/1svhyLlNVx9JGjdhvxZjoXTGaI5eF4egD/view)
- [test_mod Sion skill effect sheet](https://drive.google.com/file/d/1pQdl4-HFWjtBz9NJDhys69z-yGYjRT_V/view)

## Why the Garen examples move while animating

The Garen examples do not establish that `can_use_with_move` lets a unit freely walk during a forced `CasterAnimation`:

- BoDUD's current Garen spin action has `can_use_with_move: true`, but the action also explicitly uses repeated `RandomTarget` → `MoveToTarget` effects during the spin. Its short per-pulse movement effects are what drive repositioning. The related [BoDUD PR #3 notes](https://github.com/BoDUD/TFM2-League-Heroes/pull/3) state that `can_use_with_move` permits casting without stopping, but does not override a long caster animation; movement must be driven by `MoveToTarget`/`MoveTo`/`RushTime` in the effect tree.
- test_mod Garen's spin uses the same repeated `RandomTarget` → `MoveToTarget` pattern, despite `can_use_with_move: false`. That directly shows the movement is effect-driven rather than caused by the flag.

References:
- [BoDUD Garen data](https://github.com/BoDUD/TFM2-League-Heroes/blob/main/league/champion/league_garen.data_champion)
- [test_mod Garen data](https://drive.google.com/file/d/1h9fuGsk58HZrUUgwAhJtW2HA_51FN8m4/view)
- [Official actions schema](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/actions.md)

## v7 design scope

- Keep the existing Balmond `DirDot` hit fan, range, native lethal-counter damage/execute, and ultimate audio unchanged.
- Remove the enemy-by-enemy slash `ViewEffect`.
- Add one visual-only `LineRangeProjectile` using the cast direction, `length: 110000`, `width: 60000`, `delay: 0`, and `apply: 30`. Its `applied_effects` list is empty; the original fan remains the only damage/execute path.
- Bind that one projectile to Balmond's original 201×122 fire slash art (restored from the preserved v5 asset). The artwork has its pointed end at the caster side and broad flame toward the far end; the projectile direction supplies alignment. The in-game orientation/scale still needs confirmation.
- Reduce only the visible S2 spin art by 10% inside each existing animation frame. The sheet dimensions, crop rectangles, frame durations, native spin duration, pulses, damage, and movement timing stay unchanged.
- Preserve the v6 12-tick entry animation and S2 diagnostics. S1 remains untouched.

## Verification boundary

Schema/data inspection and offline tests can verify names, direction fields, and preserved damage-tree structure. They cannot prove the actual client renders the line effect with the intended origin, orientation, scale, or range. No TFM2 in-game test was performed for v7; the ultimate visual and S2 scale remain pending that test.
