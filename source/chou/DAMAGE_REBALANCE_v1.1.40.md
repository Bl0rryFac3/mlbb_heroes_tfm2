# Chou damage test — v1.1.40

## Decision

This is an intentionally cautious first step after Chou's damage reductions proved too severe. It does **not** restore the much larger published MLBB coefficients because TFM2's attack scale and live match behavior are not yet calibrated. The goal is a modest lift that can be compared in-game, not a final balance claim.

## Formula changes from v1.1.39

| Ability phase | v1.1.39 | v1.1.40 test | Expected increase in the current stat model |
|---|---:|---:|---:|
| Jeet Kune Do, each hit | 90–140 + 40% total physical attack | **100–165 + 45% total physical attack** | +11.4% at level 1 (60 attack); +16.1% at level 15 (130 attack), no items |
| Ultimate, roundhouse | 250–350 + 100% extra physical attack | **275–420 + 110% extra physical attack** | Base +10% early to +20% late; extra-attack coefficient +10% relative |
| Ultimate, recast kick flurry | 300–420 + 120% extra physical attack | **330–500 + 132% extra physical attack** | Base +10% early to +19% late; extra-attack coefficient +10% relative |

At level 15 with 100 extra attack, the new Ultimate phases produce 530 and 632 raw damage (before defense), versus 450 and 540 under v1.1.39. These estimates concern the formula only; actual kill pressure also depends on defenses, items, combo timing, and TFM2 runtime behavior.

Shunpo has no direct-damage buff. Its dash, control immunity, shield, and penetration are unchanged. The recast remains one engine damage event during the aerial kick-flurry animation to avoid unvalidated per-kick proc/CC behavior.

## Evidence and logging

The latest uploaded log contains 10 Ultimate recast raw-damage events: 342, 368, 368, 366, 336, 362, 371, 362, 380, and 397 (median 367; range 336–397). Only two of those are match-origin events (362 and 371); the remaining entries are not equivalent match evidence. The trace did not record numeric S1 damage. Therefore, the values above are a conservative experiment, not a balance conclusion.

v1.1.40 adds event-level `raw_damage_per_target` for S1 stages and the delayed finisher, and `raw_damage` for the first Ultimate phase. Recast already logs its single event amount. Logging occurs at ability impact/callback, not every frame.

## Test protocol

1. Compare match-origin logs at similar Chou levels and item/attack totals; keep presim out of the main comparison.
2. Check S1 values per hit and the target's actual HP loss; raw formulas are logged before the engine damage/defense pipeline.
3. Record first-phase and recast damage separately, then evaluate the full S1 → Ultimate sequence against target HP and defenses.
4. If the combo is too lethal, roll back to v1.1.39 or reduce the relevant phase only. If it remains too weak, adjust in another small step. Do not jump directly to the full published MLBB values without matched TFM2 evidence.

## Preserved scope

The v1.1.39 20,000 first-cast Ultimate range, recast-only +125,000 dynamic range (145,000 total), 100,000 knockback, combo flow, 45-tick S1 timing, S1/S2 behavior, and all other non-damage values are unchanged. The v1.1.35–.39 Windows archives remain preserved rollback baselines. Rust/source/package checks cannot verify match balance or visual behavior.
