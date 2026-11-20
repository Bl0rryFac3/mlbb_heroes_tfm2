# Balmond v4 implementation check — 31 October 2026

## Changes made

- **S1 trail:** changed the action trigger from `ViewEffect` to `CasterViewEffect`; the existing binding already uses `is_follow: true`. Dash mechanics are unchanged.
- **S2:** changed the action target from `AllyOnlySelf`/range 0 to `EnemyWithoutTower`/range 35,000, following the confirmed-working `test_mod` Garen precedent. Added `CasterAnimation("skill2", tick: 180)` and retained the native CycloneStart effect. The existing buff-bound spin VFX remains separate. This action filter can still include jungle units; it does not guarantee hero/minion priority.
- **Ultimate:** added `CasterAnimation("ult", tick: 30)` based on the timing map's approximate 0.5-second cycle; 30 ticks is an approximation, not a recovered exact source duration. Changed the slash trigger to `CasterViewEffect`. Damage, target fan, cooldowns, and Rust code are unchanged.

## Validation and remaining gates

`tools/audit_balmond_v4.py` passes **78 static checks, 0 failures**. The v4 patch applies cleanly to a v3 tree. These checks do not prove TFM2 loader, visual, or AI behavior. No full project build or in-game match test was run.

The S1 trail asset was intentionally not regenerated or rescaled because no calibrated scale factor is available; its reported size still needs an in-game adjustment. Ultimate fan orientation at Balmond's position needs a match check. S2 may still select jungle units; the current target enum does not implement a hero/minion-only preference.

## Deliverables

- Source/data patch archive (recommended): [MLBB_Heroes_Balmond_TFM2_0.7.0_v4r1_source_patch.zip](https://drive.google.com/file/d/1CnqP3GCe_3P8qHv8UkM1NRJ-7BPYp_M_/view?usp=drivesdk)
- Updated champion data: [mlbb_balmond_v4.data_champion](https://drive.google.com/file/d/1jwIY57VPnFhjatTX-2Z2wj0vlbPhh7pM/view?usp=drivesdk)

The archive applies after v3, contains the modified champion data, the unchanged Balmond Rust module for context, the v4 audit and notes, and no SDK/runtime DLL. No runtime ZIP was produced; the v3 and older runtime files remain unchanged and must not be relabelled v4.
