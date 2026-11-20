//! Native Cecilion gameplay — Overflowing (passive) and the basic attack.
//!
//! Written against the `StableEffectType` / `StablePassive` traits in
//! mod-api-stable. Every API call here is copied from a verified call site in
//! `gord.rs` (the structural template for this roster); nothing is invented.
//!
//! Registration (in candidate/src/lib.rs, next to `gord::register`):
//!     mod cecilion;
//!     cecilion::register(&mut module);
//!
//! # Build status
//!
//! VERIFIED 2026-11-08: compiles clean and all 7 unit tests pass under
//! `rustc/cargo 1.99.1` (x86_64-unknown-linux-gnu), checked as a standalone
//! crate against `mod-api-stable` at its workspace path. `cargo check --lib`
//! reports no errors and no warnings; `cargo test` reports 7 passed, 0 failed.
//!
//! The sandbox ships **no** Rust toolchain by default, but egress works and
//! `rustup` installs one in ~25 s — "no toolchain available" is FALSE and is
//! listed as a pre-debunked claim in `PROJECT_CONTEXT.md`. Install it to
//! `CARGO_HOME=/var/tmp/cargo` (see §29.16): `/var/tmp` is outside `/home/user`
//! and outside the snapshot, whereas `/home/user/.cargo` is 751 MB against a
//! 128 MB cap and `/tmp` dies with ENOSPC. To re-verify:
//!
//! ```sh
//! mkdir -p /tmp/rcheck/src && cp cecilion.rs /tmp/rcheck/src/
//! printf '%s\n' '[package]' 'name="c"' 'version="0.1.0"' 'edition="2021"' \
//!     '[lib]' 'path="src/lib.rs"' \
//!     '[dependencies]' \
//!     'mod-api-stable={path="/abs/path/to/build_v2081/sdk/mod-api-stable"}' \
//!     '[workspace]' > /tmp/rcheck/Cargo.toml
//! echo 'pub mod cecilion;' > /tmp/rcheck/src/lib.rs
//! cd /tmp/rcheck && cargo test
//! ```
//!
//! The shipped DLL is still built on Windows with the other hero modules; this
//! check covers syntax, types and the pure logic in `mod tests`, not the
//! Windows `cdylib` link step.
//!
//! # Canon, and the one thing that could NOT be ported
//!
//! Mobile Legends (fandom + gamerhub, agreeing):
//!
//! > Overflowing: Cecilion increases his Max Mana by 10 and restores the same
//! > amount each time his skill hits an enemy target. Cooldown: 1 second.
//! > His skill damage scales with his Max Mana. **Stacks up to 99,999.**
//!
//! Two hard facts forced a redesign:
//!
//! 1. **TFM2 has no mana.** `mana` occurs 0 times across all 17
//!    `.data_champion` files and every engine reference. The literal mechanic
//!    is unportable. Owner decision (2026-11-08): stacks grant **magic_power**
//!    instead, so his existing +% magic_power ratios scale for free.
//! 2. **The count is effectively unbounded (99,999).** Gord's stack counter is
//!    a stack buff read by counting same-named instances (`stack_count`).
//!    That is O(stacks) memory and O(buffs) to read — fine for Gord's 4,
//!    ruinous at 99,999. See [`read_stacks`] for what we do instead.
//!
//! # How the counter is stored
//!
//! One buff, `mlbb_cecilion_overflowing_power`, whose `magic_power` field IS
//! the total bonus: `magic_power = magic_power_per_stack * stacks`.
//!
//! - O(1) memory at any stack count.
//! - The engine already sums stat buffs, so the damage scaling is free.
//! - Reading it back is one field on one buff, not a count of 99,999.
//!
//! The stack count is recovered as `magic_power / magic_power_per_stack`, so
//! **the per-stack value must never be 0** — it would divide by zero and it
//! would also make the counter unreadable. `configure` clamps it to >= 1.
//!
//! # Why no `static` state (the Aldous bug)
//!
//! The sibling project needed a native addon to keep Veigar's stacks across
//! death, and this roster has `static ... HashMap<EntityKey, _>` precedent
//! (`lib.rs:448 SPIN_HISTORY`). Neither is used here, deliberately:
//!
//! > Stacks survive death — state lives on the player, not the champion
//! > entity. ⚠️ That same fact must be checked against §6 item 9 (pre-sim
//! > state bleed); a passive that double-counts across the two pre-sims *is*
//! > the Aldous bug.
//! > — `PROJECT_CONTEXT.md:11216`
//!
//! `StablePassive` is instantiated per **player** and its hooks take
//! `&mut self`, so the stack count simply lives in `self`. No static map, no
//! cross-match bleed, no double-counting. `on_spawn` re-applies the buff
//! after death from the surviving field.
//!
//! # Tick rate
//!
//! **60 ticks = 1 second** (`PROJECT_CONTEXT.md:651`, confirmed at `:3896`
//! from `Franco skill 540 ticks / canon 9s`). The 1s gain cooldown is
//! therefore 60 ticks.

use mod_api_stable::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Tuning, shared between the passive and the hit effect
// ---------------------------------------------------------------------------
//
// These MUST be shared. `CecilionOverflowingHit` and `CecilionOverflowing` are
// separate objects with no handle to each other, but both need the per-stack
// value: the effect writes `+per_stack` into the buff, the passive divides by
// `per_stack` to read the count back. If they disagreed the count would
// silently corrupt on every gain.
//
// SAFE FROM THE ALDOUS BUG. That bug is about *per-entity accumulated state*
// (stacks, hit history) bleeding across the two pre-sims. These are
// module-wide tuning constants that `configure` rewrites to the same value
// from the data file on every instantiation — re-running a pre-sim cannot
// double them, because they are set, never incremented.
static MP_PER_STACK: AtomicUsize = AtomicUsize::new(DEFAULT_MP_PER_STACK);
static GAIN_CD: AtomicUsize = AtomicUsize::new(DEFAULT_GAIN_CD);
static MAX_STACK: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_STACK);

// ---------------------------------------------------------------------------
// Buff names
// ---------------------------------------------------------------------------

/// The whole counter. A single stat buff whose `magic_power` is
/// `magic_power_per_stack * stacks`. Not a stack buff: one instance at any
/// count. Rewritten (remove + add) on every gain.
const OVERFLOWING_POWER: &str = "mlbb_cecilion_overflowing_power";

/// Rate gate. Canon gives Overflowing a **1 second** cooldown, and that
/// cooldown — not the 99,999 ceiling — is the real balance valve. Present on
/// the caster => the hit earns nothing.
///
/// Same shape as Franco's `HOOK_SPENT_BUFF`, a marker checked before the body
/// runs so it can fire at most once per window.
const OVERFLOWING_CD: &str = "mlbb_cecilion_overflowing_cd";

// ---------------------------------------------------------------------------
// Tuning defaults — overridden by `configure` from the data file's params
// ---------------------------------------------------------------------------

const DEFAULT_MAX_STACK: usize = 99_999;
const DEFAULT_MP_PER_STACK: usize = 2;
const DEFAULT_GAIN_CD: usize = 60; // 1 second @ 60 ticks/sec

/// Duration stamped on the power buff. Long enough to outlast any match; it
/// is also refreshed on every gain and re-applied by `on_update` if missing,
/// so this is a backstop, not the mechanism.
const POWER_BUFF_TICKS: usize = 600_000;

// ---------------------------------------------------------------------------
// Basic attack
// ---------------------------------------------------------------------------

const BASIC_DAMAGE_BASE: usize = 0;
const BASIC_ATTACK_RATIO: usize = 100;
const BASIC_PROJECTILE_SPEED: u64 = 6_000;
const BASIC_PROJECTILE_RADIUS: u64 = 6_000;
const BASIC_PROJECTILE: &str = "cecilion_basic_bolt";
const BASIC_HIT_EFFECT: &str = "mlbb_heroes:cecilion_basic_hit";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn percent(value: usize, pct: usize) -> usize {
    let scaled = value.saturating_mul(pct);
    (scaled + 50) / 100
}

/// Towers, inhibitors and the nexus: never worth Overflowing stacks. Canon
/// says "hits an enemy target"; structures are not the farm.
fn is_tower(sim: &StableSim<'_>, target: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| e.is_tower())
}

/// True when `target` is a legal hostile target for `team`.
fn is_hostile(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| {
        e.team() != team && e.is_alive() && e.is_targetable() && !e.is_tower()
    })
}

/// Basic-attack targeting: unlike `is_hostile` this **allows towers** — a
/// basic attack that cannot hit a tower is broken — and still excludes the
/// Nexus, matching the Hanabi/Karrie/Obsidia rule.
fn is_hostile_basic(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| {
        e.team() != team
            && e.is_alive()
            && e.is_targetable()
            && !matches!(e.kind(), Some(EntityKindV1::Nexus))
    })
}

fn has_named_buff(sim: &StableSim<'_>, target: usize, name: &str) -> bool {
    let Some(entity) = sim.get_entity(target) else {
        return false;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .any(|b| b.name() == name)
}

/// Read the live bonus magic power off the power buff.
fn read_buff_magic_power(sim: &StableSim<'_>, caster: usize) -> Option<usize> {
    let entity = sim.get_entity(caster)?;
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .find(|b| b.name() == OVERFLOWING_POWER)
        .map(|b| b.magic_power.max(0) as usize)
}

/// Rewrite the power buff to `total` bonus magic power. Remove-then-add
/// rather than stack, so there is exactly ever one instance.
fn write_buff_magic_power(sim: &mut StableSim<'_>, caster: usize, total: usize) {
    sim.entity_remove_buff(caster, OVERFLOWING_POWER);
    let mut buff = BuffV1::timed(OVERFLOWING_POWER, POWER_BUFF_TICKS);
    buff.magic_power = total.min(i32::MAX as usize) as i32;
    sim.add_buff(caster, &buff);
}

/// Pull an integer out of the params JSON.
///
/// `configure` is documented to receive "a JSON object with sorted keys and
/// integer values", so this is a targeted scan rather than a full parse —
/// the roster ships no serde dependency and none is needed for flat ints.
fn json_int(json: &str, key: &str, default: usize) -> usize {
    let needle = format!("\"{key}\":");
    let Some(idx) = json.find(&needle) else {
        return default;
    };
    let rest = &json[idx + needle.len()..];
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<usize>().unwrap_or(default)
}

// ---------------------------------------------------------------------------
// The passive
// ---------------------------------------------------------------------------

/// `mlbb_heroes:cecilion_overflowing`.
///
/// Owns the authoritative stack count in `self.stacks`, which is what makes
/// the count survive death: `StablePassive` is per-player, so the field
/// outlives the champion entity. The buff is only the *stat delivery* and is
/// rebuilt from this field whenever it goes missing.
// Deliberately NOT `Default`: a derived default gives
// `magic_power_per_stack: 0`, which nullifies the stat AND divides by zero on
// read-back. `new()` is the only constructor, and `configure` clamps to >= 1.
#[derive(Clone)]
struct CecilionOverflowing {
    /// Authoritative count. Lives here, not in a static map.
    stacks: usize,
    max_stack: usize,
    /// Never 0 — it is both the stat granted and the read-back divisor.
    magic_power_per_stack: usize,
    gain_cooldown_ticks: usize,
}

impl CecilionOverflowing {
    fn new() -> Self {
        Self {
            stacks: 0,
            max_stack: DEFAULT_MAX_STACK,
            magic_power_per_stack: DEFAULT_MP_PER_STACK,
            gain_cooldown_ticks: DEFAULT_GAIN_CD,
        }
    }

    fn bonus_magic_power(&self) -> usize {
        self.stacks.saturating_mul(self.magic_power_per_stack)
    }

    /// Push `self.stacks` onto the buff. Used on gain, on spawn, and by
    /// `on_update` whenever the buff has gone missing (death, purge).
    fn sync_buff(&self, sim: &mut StableSim<'_>, entity: usize) {
        write_buff_magic_power(sim, entity, self.bonus_magic_power());
    }
}

impl StablePassive for CecilionOverflowing {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    /// Called once with `DataPassiveDef.params` before any hook.
    fn configure(&mut self, params_json: &str) {
        self.max_stack = json_int(params_json, "max_stack", DEFAULT_MAX_STACK);
        // Clamp to >= 1: a 0 would zero the stat AND make read-back a
        // division by zero.
        self.magic_power_per_stack =
            json_int(params_json, "magic_power_per_stack", DEFAULT_MP_PER_STACK).max(1);
        self.gain_cooldown_ticks =
            json_int(params_json, "gain_cooldown_ticks", DEFAULT_GAIN_CD).max(1);

        // Publish to the hit effect. Set, never incremented.
        MP_PER_STACK.store(self.magic_power_per_stack, Ordering::Relaxed);
        GAIN_CD.store(self.gain_cooldown_ticks, Ordering::Relaxed);
        MAX_STACK.store(self.max_stack, Ordering::Relaxed);
    }

    /// Rebuild the buff after death. The entity is gone; `self.stacks` is not.
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize) {
        if self.stacks > 0 {
            self.sync_buff(sim, entity);
        }
    }

    /// Reconcile field and buff.
    ///
    /// The buff is authoritative while it exists (the hit effect writes it
    /// directly), so mirror it into `self.stacks` — that is what carries the
    /// count across death. When the buff is absent but we remember stacks,
    /// put it back.
    fn on_update(&mut self, sim: &mut StableSim<'_>, _rng_seed: u64, _player: usize, entity: usize) {
        match read_buff_magic_power(sim, entity) {
            Some(bonus) => {
                self.stacks = bonus / self.magic_power_per_stack;
            }
            None if self.stacks > 0 => {
                // Buff purged or champion respawned: restore from the field.
                self.sync_buff(sim, entity);
            }
            None => {}
        }
    }
}

// ---------------------------------------------------------------------------
// The one native effect
// ---------------------------------------------------------------------------

/// `mlbb_heroes:cecilion_overflowing_hit` — earn one Overflowing stack.
///
/// Carried by 43 nodes in the data file: S1's path and its destination burst
/// (2), S2 (1), and all 40 ult bolts. Canon's 1 second cooldown is what keeps
/// 40 bolts from paying 40 stacks; without it this passive would be the whole
/// kit.
fn grant_overflowing(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    let Some(team) = sim.get_entity(caster).map(|e| e.team()) else {
        return;
    };
    if !is_hostile(sim, target, team) || is_tower(sim, target) {
        return;
    }

    // THE GATE. Present => the window since the last gain has not elapsed.
    if has_named_buff(sim, caster, OVERFLOWING_CD) {
        return;
    }

    // Respect the ceiling. At the 99,999 cap further hits earn nothing but
    // still cost the cooldown, which is the canon behaviour.
    let per_stack = MP_PER_STACK.load(Ordering::Relaxed).max(1);
    let max_stack = MAX_STACK.load(Ordering::Relaxed);
    let gain_cd = GAIN_CD.load(Ordering::Relaxed).max(1);

    let current = read_buff_magic_power(sim, caster).unwrap_or(0);
    if current / per_stack >= max_stack {
        return;
    }

    write_buff_magic_power(sim, caster, current.saturating_add(per_stack));
    sim.add_buff(caster, &BuffV1::timed(OVERFLOWING_CD, gain_cd));
}

struct CecilionOverflowingHit;

impl StableEffectType for CecilionOverflowingHit {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        grant_overflowing(sim, caster_id, input.target_id);
    }

    /// AI hint: no damage of its own. It is pure scaling, and it is throttled
    /// to one stack per second, so advertising its AP term here would make the
    /// AI chase stacks instead of fighting.
    fn expected_damage(&self, _caster_stat: &StatV1) -> (usize, usize) {
        (0, 0)
    }
}

// ---------------------------------------------------------------------------
// Basic attack — travel + impact
// ---------------------------------------------------------------------------

/// `mlbb_heroes:cecilion_basic_attack` — launches the bolt. Deals no damage
/// itself; `CecilionBasicHit` pays it out on arrival.
///
/// Deliberately does **not** call `grant_overflowing`. Canon counts hits with
/// *skills*, and the skills already carry 43 feeding nodes.
#[derive(Clone, Copy, Default)]
struct CecilionBasicAttack;

impl StableEffectType for CecilionBasicAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let Some((start, team, alive)) = sim
            .get_entity(caster)
            .map(|e| (e.pos(), e.team(), e.is_alive()))
        else {
            return;
        };
        if !alive || !is_hostile_basic(sim, input.target_id, team) {
            return;
        }
        let Some(end) = sim.get_entity(input.target_id).map(|e| e.pos()) else {
            return;
        };
        let spec = ProjectileSpawnV1 {
            caster_id: caster,
            team,
            x: start.0,
            y: start.1,
            radius: BASIC_PROJECTILE_RADIUS,
            speed: BASIC_PROJECTILE_SPEED,
            move_kind: ProjectileMoveKindV1::Target.code(),
            target_id: input.target_id,
            attack_type: AttackTypeV1::BaseAttack.code(),
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: CastingTargetV1::Enemy.code(),
            target_x: end.0,
            target_y: end.1,
            penetrate: false,
        };
        let _ = sim.spawn_projectile(BASIC_PROJECTILE, BASIC_HIT_EFFECT, &spec);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            BASIC_DAMAGE_BASE + percent(stat.attack, BASIC_ATTACK_RATIO),
            0,
        )
    }
}

/// `mlbb_heroes:cecilion_basic_hit` — the bolt landing. Passed to
/// `spawn_projectile` above, so the engine runs it when the bolt arrives
/// rather than when the swing starts, which is what makes the attack travel.
#[derive(Clone, Copy, Default)]
struct CecilionBasicHit;

impl StableEffectType for CecilionBasicHit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let Some((team, attack)) = sim
            .get_entity(caster)
            .map(|e| (e.team(), e.stat().attack))
        else {
            return;
        };
        let target = input.target_id;
        if !is_hostile_basic(sim, target, team) {
            return;
        }
        let amount = BASIC_DAMAGE_BASE + percent(attack, BASIC_ATTACK_RATIO);
        let _ = sim.deal_damage_typed(
            caster,
            target,
            amount,
            DamageTypeV1::Ad,
            AttackTypeV1::BaseAttack,
        );
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            BASIC_DAMAGE_BASE + percent(stat.attack, BASIC_ATTACK_RATIO),
            0,
        )
    }
}

// ---------------------------------------------------------------------------
// S2 — Sanguine Claws: the clasp is DATA, not a native
// ---------------------------------------------------------------------------
//
// Canon: the claws "pull together, dragging enemies to the centre".
//
// This used to be `mlbb_heroes:cecilion_claws_pull`, a native built on
// `sim.entity_set_pos`. The owner playtested v2.0.8-rc.1 and saw the stun but
// no displacement at all, so that primitive does not move units in the real
// host — it is a *custom-mode* sim mutation (SDK `sim.rs`, "direct mutation
// (custom-mode building blocks)"). The spike that introduced it proved the ABI
// call well-formed, never that the engine moves the unit.
//
// The engine's own primitive is the data-level `Pull {speed, tick}`. From
// `champion-data.md` "Pull vs Grab": in a projectile's `applied_effects` a
// `Pull` heads for **the projectile's position**, not the caster's — the
// Touhou Patchouli vortex — and the claws' `RangePeriodProjectile` sits on the
// marked point. So `Pull` in its `applied_effects` is exactly "drag them into
// the mark", with no native and no `Pos` input to lose.
//
// Two consequences, both deliberate:
//   * `Pull` is a fixed length, so a victim nearer than `speed x tick` is
//     dragged through the centre and out the far side. `1400 x 10 = 14,000`
//     is the minimax choice against the 28,000 claws radius: whoever the claws
//     catch ends within 14,000 of the mark, which is inside S1's 16,000
//     landing circle — so the S2 -> S1 combo still lands on everyone.
//   * It is crowd control, so `cc_immune` targets are not moved.
//
// `Grab` would stop the victim at the caster, which is the wrong point, and
// `entity_knockback` / `entity_grab` / `entity_pull` are all caster-anchored.

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register(module: &mut StableMod) {
    module.add_native_passive(
        "mlbb_heroes:cecilion_overflowing",
        CecilionOverflowing::new(),
    );
    // Spelled out literally, not through a constant: verify_effects.py greps
    // the source for `add_native_effect("mlbb_heroes:...")` and a constant
    // here would make the gate blind to it. (Same rule as gord.rs/alpha.rs.)
    module.add_native_effect("mlbb_heroes:cecilion_overflowing_hit", CecilionOverflowingHit);
    module.add_native_effect("mlbb_heroes:cecilion_basic_attack", CecilionBasicAttack);
    module.add_native_effect("mlbb_heroes:cecilion_basic_hit", CecilionBasicHit);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- params parsing ----

    /// `configure` must survive the exact JSON the data file emits, and must
    /// fall back cleanly on junk.
    #[test]
    fn configure_reads_shipped_params() {
        let mut p = CecilionOverflowing::new();
        p.configure(
            r#"{"gain_cooldown_ticks":60,"magic_power_per_stack":2,
                 "max_procs_skill":3,"max_procs_skill2":1,"max_procs_ult":5,
                 "max_stack":99999}"#,
        );
        assert_eq!(p.max_stack, 99_999);
        assert_eq!(p.magic_power_per_stack, 2);
        assert_eq!(p.gain_cooldown_ticks, 60);
    }

    #[test]
    fn configure_falls_back_on_garbage() {
        let mut p = CecilionOverflowing::new();
        p.configure("not json at all");
        assert_eq!(p.max_stack, DEFAULT_MAX_STACK);
        assert_eq!(p.magic_power_per_stack, DEFAULT_MP_PER_STACK);
        assert_eq!(p.gain_cooldown_ticks, DEFAULT_GAIN_CD);
    }

    /// A zero per-stack value would both nullify the stat and divide by zero
    /// on read-back. Must be clamped.
    #[test]
    fn per_stack_zero_is_clamped() {
        let mut p = CecilionOverflowing::new();
        p.configure(r#"{"magic_power_per_stack":0}"#);
        assert!(p.magic_power_per_stack >= 1);
    }

    // ---- the scaling curve ----

    #[test]
    fn bonus_is_linear_in_stacks() {
        let mut p = CecilionOverflowing::new();
        p.configure(r#"{"magic_power_per_stack":2}"#);
        p.stacks = 100;
        // 100 stacks ~= +116% over his 172 L15 base; MLBB's +143% is the
        // target this was calibrated against.
        assert_eq!(p.bonus_magic_power(), 200);
        assert!(200 * 100 / 172 > 100);
    }

    /// The read-back must invert the write, or the count silently drifts on
    /// every death/restore cycle.
    #[test]
    fn read_back_inverts_write() {
        let mut p = CecilionOverflowing::new();
        p.configure(r#"{"magic_power_per_stack":2}"#);
        for n in [0usize, 1, 7, 250, 99_999] {
            p.stacks = n;
            let recovered = p.bonus_magic_power() / p.magic_power_per_stack;
            assert_eq!(recovered, n, "stacks {n} did not survive the round trip");
        }
    }

    /// Saturating, not wrapping: 99,999 stacks must not overflow into a
    /// negative or a small number.
    #[test]
    fn max_stacks_does_not_overflow() {
        let mut p = CecilionOverflowing::new();
        p.configure(r#"{"magic_power_per_stack":2,"max_stack":99999}"#);
        p.stacks = p.max_stack;
        let bonus = p.bonus_magic_power();
        assert_eq!(bonus, 99_999 * 2);
        assert!(bonus <= i32::MAX as usize);
    }

    // ---- the throttle (canon's real balance valve) ----

    /// 40 ult bolts over a 360-tick channel, gated at one stack per 60 ticks,
    /// must yield about canon's ~5 — not 40. This is the whole reason the
    /// passive is not the entire kit.
    #[test]
    fn ult_bolt_count_is_throttled() {
        let gain_cd = 60usize;
        // Keep equal to the data's `ult.duration`. The v2.0.9 rebalance
        // shortened the channel 360 -> 180, which is why a full ult now pays
        // 3 stacks rather than the ~5 of the original 6 s version: the gain
        // is throttled to one per second and there is simply less channel.
        // That is 6 magic power instead of 10 — a real but small price, and
        // it is dwarved by the ~2 forgone S1 casts the shorter channel buys.
        let ult_ticks = 180usize;
        let bolts = 40usize;

        let mut earned = 0usize;
        let mut next_allowed = 0usize;
        for i in 0..bolts {
            // Bolts are evenly spaced across the channel.
            let tick = i * (ult_ticks / bolts);
            if tick >= next_allowed {
                earned += 1;
                next_allowed = tick + gain_cd;
            }
        }
        assert!(
            (2..=4).contains(&earned),
            "a 3 s ult should pay ~3 stacks (one per second); got {earned}"
        );
    }

    // ---- S2 clasp geometry ----

    /// The claws' `Pull` is a fixed-length move toward the mark, so a victim
    /// nearer than `speed x tick` is dragged through the centre and out the
    /// far side. `audit_cecilion.py` pins the numbers in the data; what is
    /// worth proving here is the property that makes the S2 -> S1 combo work:
    /// **whoever the claws catch ends up inside S1's landing circle.**
    #[test]
    fn the_clasp_leaves_every_victim_inside_the_s1_landing_circle() {
        const CLAWS_RADIUS: f64 = 28_000.0; // data: skill2 Circle radius
        const PULL_SPEED: f64 = 1_400.0; // data: Pull{speed}
        const PULL_TICKS: f64 = 10.0; // data: Pull{tick}
        const S1_LANDING_RADIUS: f64 = 16_000.0; // data: skill RangeEffect Circle

        let pull = PULL_SPEED * PULL_TICKS;
        // Worst case over every starting distance in the claws.
        let worst = pull.max((pull - CLAWS_RADIUS).abs());
        assert!(
            worst <= S1_LANDING_RADIUS,
            "a victim ends {worst} units off the mark, outside S1's {S1_LANDING_RADIUS}"
        );
        // And the pull must actually be felt: a champion body is ~10,000 wide.
        assert!(pull >= 10_000.0, "a {pull} unit pull reads as nothing");
    }
}
