# Balmond v8 research follow-up — 2026-10-31

## S1: why the dash looked short on a champion

The S1 action is directional. `RushTime` moves at 7,000 units/tick for 13 ticks, capped at 90,000, with `penetrate: false`; its native hit handler applies damage and a 40% slow for 180 ticks. The source also used `entity_knockback(caster, target, 2500, 6)`. In the stable API, this moves the target away from the caster, so it pushed the enemy away from Balmond by up to roughly 15,000 units after a successful collision. On an unobstructed escape, no champion collision stops the dash, so the full path is available.

V8 reduces only that enemy impact push to speed 1,000 for 3 ticks (about 3,000 units maximum). The direction, dash speed/ticks/range, minion path, damage, and slow are unchanged.

## Ult: why V7 showed only the cast animation

V7's visual-only line was configured `delay: 0`, `apply: 30`. BoDUD PR #13 reports an SDK probe finding that for `LineRangeProjectile`, `apply - 1` is the hit/trigger tick and `delay - 1` is the lifetime; it notes the public skill documentation had the order reversed and warns when `apply` is beyond `delay`. Under that measured behavior, V7's line had no positive lifetime and its trigger was scheduled beyond it, consistent with the user's report that the cast animation appeared without the line.

A no-effects line used only for visuals is an established pattern in BoDUD Lux R and Vi E. V8 therefore uses `apply: 1`, `delay: 30` for an immediate line with a brief lifetime. It keeps the same 110,000 × 60,000 dimensions and matching projectile visual binding. The existing Balmond DirDot damage fan/native execute remain unchanged; no Sion airborne effect or charge delay is added.

References:
- [BoDUD PR #13: measured projectile timing](https://github.com/BoDUD/TFM2-League-Heroes/pull/13)
- [BoDUD PR #5: separate damage-free visual line for Lux R](https://github.com/BoDUD/TFM2-League-Heroes/pull/5)
- [BoDUD PR #77: visual-only line for Vi E](https://github.com/BoDUD/TFM2-League-Heroes/pull/77)
- [Stable API reference: entity_knockback](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/stable-api-reference.md)
- [Effects schema: RushTime and LineRangeProjectile](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/effects.md)

## Boundary

These changes are source/data-based and statically audited. The modified impact spacing and corrected Ult line timing still need an in-game test. No behavior is claimed as fixed in-client until that test.
