# TFM2 — authoritative effect types

Source: https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/data-champion-schema/effects.md
Fetched 2026-11-09. This is the OFFICIAL modding guide for our engine.

## Rules for using this file

1. **This list is the authority on what EXISTS.** The shipped roster is only
   evidence of what we have USED. Absence from the roster does not make a
   type invalid -- `RemoveCasterBuff` and `Invisible` are both documented
   here and neither was used by any shipped hero.
2. **`tfm2_league_ref` (League) is a DIFFERENT MOD.** Its vocabulary is not
   evidence about ours. `CasterInvisible` came from there and does not exist.
3. Absence from this list is not proof of absence either: `Taunt` is used by
   a shipped hero and is not documented here. When the two disagree, prefer
   shipped-and-working, then documented, and never neither.

## The 55 documented types

| Type | Documented behaviour |
|---|---|
| `AddBuff` | Adds a stat/status `buff_state` to the selected target. |
| `AddCasted` | Adds a periodic over-time effect to the selected target. Every `period` ticks until `duration`, each nested effect is applied to that target. |
| `AddCasterBuff` | Adds a stat/status `buff_state` to the caster. If `only_to_enemy` is true, the buff is added only when the original target is an enemy. |
| `Airborne` | Lifts/disables the target for `duration` ticks. Use it for knock-up style hard CC. |
| `ApAttack` | Deals magic-power-based damage. Use this for AP-style skill damage. |
| `ApplyInProjectile` | Creates an invisible projectile/area that waits `tick` ticks and then applies its effects to targets in `shape`. If `follow_caster` is true, the area follows the caster until it applies. |
| `Attack` | Deals physical attack damage. Use this for AD-style skill damage or basic-attack-like damage. |
| `AutoTargetProjectile` | Automatically chooses an enemy target in `range`, preferring the caster's recently attacked target when valid, then fires a target-following projectile. |
| `BackToCasterLinearProjectile` | Spawns a straight projectile from a position back to the caster. This is mainly useful as an `end_effect` of another projectile because projectile end effects pass a position input. |
| `Banish` | Temporarily removes/locks the target with optional visual effect names for the lock and ending moments. |
| `Bind` | Roots or movement-locks the target for `duration` ticks. |
| `BlockAttack` | Prevents basic attacks for `tick` ticks. |
| `BlockMoveSkill` | Prevents movement skills for `tick` ticks. |
| `BlockSkill` | Prevents skill use for `tick` ticks. |
| `CasterAnimation` | Adds a named caster animation state for `tick` ticks. |
| `CasterViewEffect` | Plays a named visual effect at the caster position. |
| `Charm` | Forces the target into charm behavior for `tick` ticks. |
| `Combine` | Runs multiple effects immediately with the same input. |
| `Delayed` | Queues nested effects to run after `tick` ticks. |
| `DirTeleport` | Instantly moves the caster `moved` distance along a direction input. |
| `Fear` | Forces the target into fear behavior for `tick` ticks. |
| `FixedAttack` | Deals fixed damage through the fixed-damage path. Use it when the effect should not behave like normal AD or AP damage. |
| `Grab` | Pulls the target toward the caster. `tick` is optional; if omitted, the engine uses the grab effect's internal timing behavior. |
| `Heal` | Restores HP. Use `heal_type` to decide whether the effect heals the caster, the selected target, an ally only, or nearby allies. |
| `Invisible` | Makes the target invisible for `tick` ticks. |
| `Knockback` | Pushes the target away with the given `speed` for `tick` ticks. |
| `LineRangeProjectile` | Creates a delayed line-shaped area from the caster toward the input direction/target/position. |
| `LinearProjectile` | Spawns a projectile that travels in a straight line from the caster toward a target, position, or direction. It applies `applied_effects` on hit and runs `end_effects` when it ends. |
| `MoveBack` | Moves the caster away from the target's position for `tick` ticks at `speed`. |
| `MoveTo` | Starts a movement state toward a target, position, or direction. When the movement ends, `end_effects` fire. |
| `MoveToTarget` | Starts a movement state that tracks a target entity. If the target is gone when the effect resolves, `end_effects` run immediately against the original input. |
| `ParabolicProjectile` | Spawns a projectile that travels in a fixed arc to a target or position. On arrival it applies hit effects and then `end_effects`. `range_effect_name` can show a preview/impact range visual while it travels. |
| `Pull` | Pulls the target toward the effect point/caster for a fixed number of ticks. |
| `RandomTarget` | Chooses a random target in `range` that matches `casting_target`, then applies nested effects to it. |
| `RangeEffect` | Immediately applies nested effects to targets inside `shape`. The area can be centered around the caster or placed forward from the caster. |
| `RangePeriodProjectile` | Creates a stationary periodic area at a position input. It applies `applied_effects` every `period` ticks until `tick` expires, then applies `end_effects` to entities still in the area. |
| `RangeProjectile` | Creates a delayed area at a position input. After `delay` ticks, it applies effects for `apply` ticks to targets in `shape`. |
| `RemoveCasterAnimation` | Removes a named caster animation state. |
| `RemoveCasterBuff` | Removes a named normal buff from the caster. Use it for temporary self-state cleanup, such as ending a mode buff. |
| `Rush` | Moves the caster toward a position input. While rushing, the caster can apply nested effects to entities that match `casting_target`. |
| `RushMoveToBack` | Rushes behind the target and triggers `applied_effects` after the travel delay. Use this for dash-behind or backstab-style skills. |
| `RushTime` | Moves the caster in the input direction for a fixed number of ticks instead of moving to a clicked destination. |
| `Sfx` | Plays a named sound effect at the caster position. |
| `Shield` | Adds a temporary shield to the target. |
| `ShrinkingBarrier` | Creates a circle around a target that follows that target and shrinks over time. Effects are applied on the barrier edge according to the engine barrier behavior. |
| `Stun` | Prevents the target from acting for `duration` ticks. |
| `SwitchByBuff` | Chooses one of two effects depending on whether the caster currently has `buff_name`. |
| `TargetProjectile` | Spawns a projectile that follows the selected target and applies effects on hit. |
| `TargetProjectileFromProjectile` | Spawns a target-following projectile from the current projectile position. This only works inside an effect chain that has projectile optional info, such as another projectile's `applied_effects` or `end_effects`. |
| `TargetSfx` | Plays a named sound effect at the target, position input, or current projectile position. |
| `TargetSplashProjectile` | Spawns a projectile that follows a target, then can jump/splash to another target within `range`. |
| `Taunt` | Taunts the target for `duration` ticks. |
| `Teleport` | Instantly moves the caster to a position input or target position. |
| `ViewEffect` | Plays a named visual effect at the effect target/input position. |
| `WithSelf` | Runs nested effects with the caster as the target/self context. |
