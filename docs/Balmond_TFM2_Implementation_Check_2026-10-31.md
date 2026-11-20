# Balmond v3 implementation check — 31 October 2026

## Correction and status

The previous v2 source patch did **not** follow the full recommendation: it updated Skill 1 but retained the v1 all-native Ultimate. V3 corrects that by making the fan target selection data-driven and keeping only missing-health damage and caps in the native per-target effect. The v1 and v2 files/archives remain unchanged.

V3 is still a source patch, not a rebuilt runtime. The existing v1 source-with-SDK and runtime ZIPs remain unchanged. The matching full source tree/SDK is not extracted in this workspace, and `cargo`/`rustc` are unavailable; no full build or TFM2 gameplay test has been done.

## Repository and schema research

- Official actions schema distinguishes `Direction` from `Targeting`; official effects schema documents `RushTime`, `RangeEffect`, and nested effect lists. The official enum page documents the `DirDot` shape object but does not define the units of its `range` field. [Actions](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/actions.md) · [Effects](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/effects.md) · [Enums / ProjectileShape](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/enums.md#projectileshape)
- Sion's current champion data uses a Direction action with `RushTime`, an enemy-champion collision filter and `penetrate: false`; that supports the implementation shape for Balmond S1. [Sion data](https://github.com/BoDUD/TFM2-League-Heroes/blob/main/league/champion/league_sion.data_champion)
- Miss Fortune's PR specifies a 40° 12-wave R fan; her R data combines `LineRangeProjectile` with `RangeEffect`/`DirDot` to select targets. That validates the data-driven fan pattern, not Balmond's angle or single-hit behavior. [Miss Fortune PR #17](https://github.com/BoDUD/TFM2-League-Heroes/pull/17) · [Miss Fortune data](https://github.com/BoDUD/TFM2-League-Heroes/blob/main/league/champion/league_missfortune.data_champion)
- Locke's PR describes a real execute implemented in its add-on; generic `Attack.target_hp_ratio` is documented as target maximum HP, not missing HP. [Locke PR #548](https://github.com/BoDUD/TFM2-League-Heroes/pull/548) · [Attack schema](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/effects.md#attack)
- `DirDot.range = 707` was chosen by translating the prior native 45° half-angle dot threshold `0.707106...` into an apparent thousandths-scale value. Repository examples such as 906 for Miss Fortune show that numeric values are in this neighborhood; the docs do not state the conversion. This is an informed approximation and remains an in-game validation gate, not a claim of verified 90° geometry.

## V3 boundary checks

- Ultimate data contains exactly one `RangeEffect` with `DirDot` and a nested `Native` reference for the execute. The native function accepts a target input and no longer reads a direction, performs a cone test, or enumerates entities.
- The per-target native effect checks alive/targetable/opposing-team/non-tower status; reads current and maximum HP; uses fixed damage; and applies the non-hero cap. It still uses the preserved level-band approximation for Ultimate tiers.
- Skill 1 is Direction + `RushTime`; native path damage excludes champions, while the collision effect handles the first hero. It is escape-capable at the action-input level, but no custom AI input rule was added to guarantee retreat use.
- Skill 2 and visuals are carried forward from v2 unchanged; visual assets were visually inspected and the FAnim/PNG bounds audit passed. This inspection is not an in-game visual test.

## Checks performed

`tools/audit_balmond_v3.py`: **73 checks passed, 0 failed**, including JSON parse, effect/registration linkage, data/native split, formula arithmetic, direction geometry, icon decode, and every body/VFX crop's dimensions and positive frame duration. Rust unit-test cases were added for relevant helper math but could not be executed.

The v3 incremental patch was applied to a synthetic v2 file tree: code/data matched the v3 checkpoint and the updated `mod.mod_info` parsed as JSON. It was not applied to the full project/SDK tree because that archive is not local. Before release: apply v1, v2, v3 incrementally; run Rust tests and Windows x86-64 build; run source/runtime ZIP audits; verify the runtime has no source/SDK; then perform TFM2 0.7.0 tests for S1, S2, fan edge/aim/target delivery, missing-health damage, caps, VFX placement and audio.
