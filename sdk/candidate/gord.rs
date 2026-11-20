//! Native Gord gameplay — Mystic Favor (passive).
//!
//! Written against the `StableEffectType` trait in mod-api-stable. Every API
//! call here is copied from a verified call site in `alpha.rs`
//! (`record_beta_hit`), `obsidia.rs` or `uranus.rs`; nothing is invented.
//!
//! Registration (in candidate/src/lib.rs, next to `alpha::register`):
//!     mod gord;
//!     gord::register(&mut module);
//!
//! # Why this is the only native in the kit
//!
//! Gord's S2 (a persistent field), his ult (a repeated beam), his stun and all
//! of his damage are expressible in data — see `GORD_SPEC.md` §4. Only the
//! passive is not, and for one reason: **the counter is per target.**
//!
//! > Nothing reads a buff count on the target, so the stacks are
//! > league_kennen's caster ladder b_1 -> b_2.
//! > — `champion-data.md:3672` (TFM2-League-Heroes reference)
//!
//! A caster-side ladder would count hits across *every* enemy Gord touches, so
//! four ticks of his field on three minions would arm the proc for a champion
//! he has never hit. `entity_stack_buff` on the target does it properly, and it
//! is already used in six modules here (uranus, hanabi, lancelot, karrie,
//! alpha, obsidia).
//!
//! # The counting rule
//!
//! Canon (Liquipedia, verbatim): *"After Gord hits an enemy 4 times with skills
//! within a short period of time, the next skill damage will deal an additional
//! 140 (+8\*Hero Level) (+40% Magic Power) True Damage and slow them by 20% for
//! 1s (the slow effect stacks up 2 times)."*
//!
//! Four hits **arm** it; the **fifth** carries the bonus. That is one proc per
//! five hits, not one per four, and `GORD_SPEC.md` §6 prices a full rotation at
//! six procs on that basis (33 skill hits / 5). The step function
//! [`mystic_step`] encodes it and is unit-tested without a sim.
//!
//! Feeding it: S1's splash, every one of S2's sixteen ticks and every ult wave
//! carry a `Native` node `mlbb_heroes:gord_mystic_hit` in their
//! `applied_effects`. One native, three call sites.

use mod_api_stable::*;

// ---------------------------------------------------------------------------
// Buff names
// ---------------------------------------------------------------------------

/// Per-target skill-hit counter. A stack buff rather than state in a passive
/// Charged on **Gord**, not on his victim, so S1, all sixteen S2 ticks and all
/// sixteen ult waves feed one counter no matter which enemy they hit. Was
/// target-scoped until 2026-11-06; see `record_mystic_hit` for why it moved.
const MYSTIC_STACK: &str = "mlbb_gord_mystic_stack";
/// Set on **Gord** when the counter fills. The **next** skill hit consumes it
/// and pays the bonus. Separate from the stack buff because the counter and the
/// armed flag are different states: four stacks means "nearly there", this
/// means "the next one pays". Its view_buffs entry (`z: 3`) renders over Gord's
/// head — that is the point of it being caster-scoped.
const MYSTIC_READY: &str = "mlbb_gord_mystic_ready";
/// The 20% slow the proc leaves. Stacked, so two procs in quick succession
/// reach the canon ceiling of 40%.
const MYSTIC_SLOW: &str = "mlbb_gord_mystic_slow";

// ---------------------------------------------------------------------------
// Timing (60 ticks = 1 s)
// ---------------------------------------------------------------------------

/// Gord balance lever D (2026-11-07).
///
/// Canon is "hits an enemy 4 times with skills" -> proc on the 5th, i.e. one
/// proc per **five** hits. The owner shipped A+B+C in v2.0.7-rc.4 (S2 AP 5->8,
/// ult AP 8->11, ult CD 44->32 s) which took an L15 rotation from 973 to 1149,
/// then reported Gord still reading weak against Gusion (1390) and Uranus
/// (~1263). rc.4 deliberately stopped at 93% of the Alpha v17 reference (1229)
/// and left this lever unspent for exactly that eventuality.
///
/// Note the reference: Alpha is the *weakest* of the three comparators, so
/// capping Gord at 93% of Alpha necessarily leaves him below Gusion and Uranus
/// both. Spending this lever takes L15 1149 -> 1261 (6 -> 8 procs at 56 True
/// Damage each): level with Uranus, 91% of Gusion, 103% of Alpha.
///
/// **Revert:** set back to `4` to restore the pre-rc.5 cadence (one proc per
/// five hits, L15 rotation 1149). Revert `S2_AP`/`ULT_AP`/`ULT_CD` separately
/// if A+B+C also need to come out — they are independent levers.
const MYSTIC_HITS: usize = 3;
/// How long a stack survives. Canon says only "within a short period of time" —
/// no number is printed anywhere. 300 ticks (5 s) is long enough that
/// S1 -> S2 -> the first ult waves chain into one proc, and short enough that
/// poke between fights does not pre-load it. This is the single knob that
/// quietly controls how often the passive fires; see GORD_SPEC.md §7 Q2.
const STACK_DURATION: usize = 300;
/// The armed flag must outlive the gap between two casts, or arming is wasted.
const READY_DURATION: usize = 300;
/// Canon: slow 20% for 1 s.
const SLOW_DURATION: usize = 60;
/// Canon: "the slow effect stacks up 2 times".
const SLOW_STACKS: usize = 2;
const SLOW_PERCENT: i32 = -20;

// ---------------------------------------------------------------------------
// Damage — GORD_SPEC.md §6
//
// Canon is 140 (+8*Hero Level) (+40% Magic Power) against a 2,380 HP pool.
// This roster is 650-820 HP, so the whole kit is re-expressed as a share of a
// 700 HP bar: 12 (+2/level) (+8% AP). See GORD_SPEC.md §6 for the table and
// for why the ratio compresses ~5x while the shape is kept.
// ---------------------------------------------------------------------------

const PROC_BASE: usize = 12;
const PROC_LEVEL_GAIN: usize = 2;
const PROC_AP_RATIO: usize = 8;

/// The proc's burst. MUST live in `view_effects` — `play_view_effect` resolves
/// view-effect bindings, never `view_projectiles` ones.
const MYSTIC_BURST_FX: &str = "gord_mystic_burst";
const MYSTIC_BURST_TIME: u64 = 30;

// ---------------------------------------------------------------------------
// Basic attack
//
// The data routes Gord's basic through native code for the same reason Karrie,
// Hanabi, Obsidia and Zhask do: a bare `ViewEffect` has no travel, so the hit
// landed the instant the swing started. A `TargetProjectile` would do it with
// no code, but only Estes uses one and his `applied_effects` is empty, so the
// schema for a landing effect on that node is unproven here. `spawn_projectile`
// takes the hit effect as an explicit argument, which is not.
//
// Damage matches the roster's plain `Attack` node exactly — `damage: 0,
// attack_ratio: 100`, i.e. 100% of AD, physical. Gord's own basic attack
// previously carried no damage node at all, only a `ViewEffect`.
// ---------------------------------------------------------------------------

const BASIC_DAMAGE_BASE: usize = 0;
/// 100 = "attack_ratio: 100", the value every non-native hero in this roster
/// uses. Gord is a mage but basic attacks are physical here, as in canon.
const BASIC_ATTACK_RATIO: usize = 100;
/// Range is 62_000, so 6_000/s is ~10 ticks of flight — visible travel without
/// making him feel sluggish. Same speed Obsidia's shard uses.
const BASIC_PROJECTILE_SPEED: u64 = 6_000;
const BASIC_PROJECTILE_RADIUS: u64 = 6_000;
/// MUST live in `view_projectiles`, not `view_effects`.
const BASIC_PROJECTILE: &str = "gord_basic_bolt";
/// The native that runs on arrival; it deals the damage below.
const BASIC_HIT_EFFECT: &str = "mlbb_heroes:gord_basic_hit";
/// The landing spark. MUST live in `view_effects`.
const BASIC_HIT_FX: &str = "gord_basic_hit";

// ---------------------------------------------------------------------------
// Pure state machine — unit-testable without a sim
// ---------------------------------------------------------------------------

/// What one landed skill hit does to the passive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MysticStep {
    /// Not armed yet: put one more stack on the target (the count after).
    Charge(usize),
    /// The counter just filled: clear the stacks and arm the flag.
    Arm,
    /// The flag was up: consume it and pay the bonus.
    Proc,
}

/// The whole counting rule in one function.
///
/// Four hits arm it, the fifth pays — so the proc is one per **five** hits, not
/// one per four. Returns `Proc` only when the armed flag was already up.
fn mystic_step(ready: bool, stacks: usize) -> MysticStep {
    if ready {
        MysticStep::Proc
    } else if stacks.saturating_add(1) >= MYSTIC_HITS {
        MysticStep::Arm
    } else {
        MysticStep::Charge(stacks + 1)
    }
}

/// True Damage the proc deals: `12 + 2*level + 8% AP`.
fn proc_damage(magic_power: usize, level: usize) -> usize {
    PROC_BASE
        .saturating_add(PROC_LEVEL_GAIN.saturating_mul(level))
        .saturating_add(percent(magic_power, PROC_AP_RATIO))
}

// ---------------------------------------------------------------------------
// Helpers (per-module copies, as in alpha.rs / obsidia.rs — these are not
// shared across the crate)
// ---------------------------------------------------------------------------

/// `value` percent of `pct`, rounded to nearest.
///
/// Two conventions exist in this crate: `alpha.rs` truncates
/// (`value * pct / 100`), `obsidia.rs` rounds (`(value * pct + 50) / 100`).
/// Gord uses Obsidia's — the newer module — and because rounding reproduces
/// the L1/L8/L15 column of GORD_SPEC.md §6 (19 / 38 / 56) exactly, while
/// truncation would turn the L8 figure into 37.
fn percent(value: usize, pct: usize) -> usize {
    let scaled = value.saturating_mul(pct);
    // saturating_add, not `+`: `(scaled + 50)` overflows in a debug build and
    // wraps silently in release, so the rounding bias silently disappears at
    // the extreme. Identical for every realistic value of `scaled`.
    scaled.saturating_add(50) / 100
}

/// Towers, inhibitors and the nexus: never worth Mystic stacks.
fn is_tower(sim: &StableSim<'_>, target: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| e.is_tower())
}

/// True when `target` is a legal hostile target for `team`.
fn is_hostile(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target)
        .is_some_and(|e| e.team() != team && e.is_alive() && e.is_targetable() && !e.is_tower())
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

/// Live Mystic stacks on `target`.
fn stack_count(sim: &StableSim<'_>, target: usize) -> usize {
    let Some(entity) = sim.get_entity(target) else {
        return 0;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .filter(|b| b.name() == MYSTIC_STACK)
        .count()
}

fn has_named_buff(sim: &StableSim<'_>, target: usize, name: &str) -> bool {
    let Some(entity) = sim.get_entity(target) else {
        return false;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .any(|b| b.name() == name)
}

/// Only draw in view contexts; the server sim must not emit visuals.
fn can_render(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|origin| SimOriginKindV1::from_code(origin.kind))
        .is_some_and(|kind| {
            matches!(
                kind,
                SimOriginKindV1::ClientMatchView
                    | SimOriginKindV1::ClientSpectate
                    | SimOriginKindV1::ClientReplay
            )
        })
}

/// The proc's burst at the target. Fire-and-forget, no damage of its own.
fn play_mystic_burst(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    if !can_render(sim) {
        return;
    }
    let Some(tpos) = sim.get_entity(target).map(|e| e.pos()) else {
        return;
    };
    let _ = sim.play_view_effect(
        MYSTIC_BURST_FX,
        caster,
        &InputTargetV1::pos(tpos.0, tpos.1),
        0,
        0,
        MYSTIC_BURST_TIME,
    );
}

// ---------------------------------------------------------------------------
// The one native
// ---------------------------------------------------------------------------

/// Record one landed **skill** hit on `target` and pay Mystic Favor when it is
/// armed. Called from S1's splash, every S2 tick and every ult wave.
///
/// Basic attacks must not call this — canon counts "hits an enemy with
/// skills", and Gord's field and beam land 32 ticks a rotation, which is
/// already plenty.
fn record_mystic_hit(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    let Some((team, magic_power, level)) = sim
        .get_entity(caster)
        .map(|e| (e.team(), e.stat().magic_power, e.level().max(1)))
    else {
        return;
    };
    if !is_hostile(sim, target, team) || is_tower(sim, target) {
        return;
    }

    // THE COUNTER AND THE ARMED FLAG LIVE ON GORD, NOT ON THE TARGET.
    //
    // They used to be on `target`. That put the `gord_mystic_ready` marker on
    // the ENEMY, stacked on top of `gord_mystic_slow` — two 72x72 markers (each
    // the full size of Gord's own sprite) overlapping on one victim for 0.54 s
    // apiece. The owner reported the clutter and expected the "armed" marker
    // over Gord, which is what the z: 3 on its view_buffs entry implies.
    //
    // Moving the marker alone is not coherent: a marker over Gord has to
    // describe a state Gord owns. So the whole counter is caster-scoped. Skill
    // hits on ANY enemy charge it, and the next skill hit pays — instead of the
    // counter scattering across targets when the ult re-aims (it re-picks a
    // target every one of its 16 waves), which made procs rare precisely when
    // several enemies were in range.
    //
    // `MYSTIC_SLOW` deliberately stays on the target: the slow is something
    // that happens to the victim, not to Gord.
    match mystic_step(
        has_named_buff(sim, caster, MYSTIC_READY),
        stack_count(sim, caster),
    ) {
        MysticStep::Charge(_) => {
            let stack = BuffV1::timed(MYSTIC_STACK, STACK_DURATION);
            let _ = sim.entity_stack_buff(caster, &stack, MYSTIC_HITS, true);
        }
        MysticStep::Arm => {
            // Four hits: clear the counter (entity_remove_buff takes every
            // buff of that name, so this drops all of them at once) and arm.
            let _ = sim.entity_remove_buff(caster, MYSTIC_STACK);
            sim.add_buff(caster, &BuffV1::timed(MYSTIC_READY, READY_DURATION));
        }
        MysticStep::Proc => {
            let _ = sim.entity_remove_buff(caster, MYSTIC_READY);
            let amount = proc_damage(magic_power, level);
            // **Fixed**: ignores magic resistance, which is what "True Damage"
            // means. Same call Alpha's Lock-On laser uses.
            let _ = sim.deal_damage_typed(
                caster,
                target,
                amount,
                DamageTypeV1::Fixed,
                AttackTypeV1::Skill,
            );
            let mut slow = BuffV1::timed(MYSTIC_SLOW, SLOW_DURATION);
            slow.move_speed_mult = SLOW_PERCENT;
            // Stacked, not refreshed: two procs reach the canon 40% ceiling.
            let _ = sim.entity_stack_buff(target, &slow, SLOW_STACKS, true);
            play_mystic_burst(sim, caster, target);
        }
    }
}

/// `mlbb_heroes:gord_mystic_hit` — one landed skill hit.
///
/// Placed in the `applied_effects` of every damaging node in the kit, next to
/// the `ApAttack` that deals the hit's own damage. It never deals the hit's
/// damage, only the passive's bonus.
#[derive(Clone, Copy)]
struct GordMysticHit;

impl StableEffectType for GordMysticHit {
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
        record_mystic_hit(sim, caster_id, input.target_id);
    }

    /// AI hint: raw (AD, AP) damage. The proc fires on roughly one hit in five,
    /// so the honest per-hit expected value is a fifth of its AP term. Full
    /// value would make the AI over-committed to a champion whose damage is
    /// mostly in the field and the beam.
    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (
            0,
            percent(caster_stat.magic_power, PROC_AP_RATIO) / (MYSTIC_HITS + 1),
        )
    }
}

/// `mlbb_heroes:gord_mystic_favor` — the passive's data-side handle.
///
/// Mystic Favor is entirely **reactive**: the counter lives in the native
/// *effect* (`GordMysticHit`), fed from every damaging node in the kit. This
/// passive therefore hooks nothing.
///
/// It exists because every champion in this roster carries a
/// `passive.passive_ref`, and the game binds the passive's tooltip and its
/// icon to that reference. Gord is the first hero here whose passive needs no
/// `on_*` hook at all — Gusion's, by contrast, drives its whole kit from
/// `on_attack`.
#[derive(Clone, Default)]
struct GordMysticFavor;

impl StablePassive for GordMysticFavor {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Basic attack — travel + impact
// ---------------------------------------------------------------------------

/// `mlbb_heroes:gord_basic_attack` — launches the bolt. Deals no damage
/// itself; `GordBasicHit` pays it out on arrival.
///
/// Deliberately does **not** call `record_mystic_hit`. Canon counts "hits an
/// enemy with **skills**" for Mystic Favor, and Gord's field and beam already
/// land 32 ticks a rotation. Letting basics charge the passive would make it
/// fire almost continuously and would also misrepresent the kit.
#[derive(Clone, Copy, Default)]
struct GordBasicAttack;

impl StableEffectType for GordBasicAttack {
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

/// `mlbb_heroes:gord_basic_hit` — the bolt landing. Passed to
/// `spawn_projectile` above, so the engine runs it when the bolt arrives
/// rather than when the swing starts, which is what makes the attack travel.
#[derive(Clone, Copy, Default)]
struct GordBasicHit;

impl StableEffectType for GordBasicHit {
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
        if can_render(sim) {
            let _ = sim.play_view_effect(
                BASIC_HIT_FX,
                caster,
                &InputTargetV1::target(target),
                0,
                0,
                0,
            );
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            BASIC_DAMAGE_BASE + percent(stat.attack, BASIC_ATTACK_RATIO),
            0,
        )
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register(module: &mut StableMod) {
    module.add_native_passive(
        "mlbb_heroes:gord_mystic_favor",
        GordMysticFavor::default(),
    );
    // Spelled out literally, not through a constant: verify_effects.py greps
    // the source for `add_native_effect("mlbb_heroes:...")` and a constant here
    // would make the gate blind to it. (Same rule as alpha.rs.)
    module.add_native_effect("mlbb_heroes:gord_mystic_hit", GordMysticHit);
    module.add_native_effect("mlbb_heroes:gord_basic_attack", GordBasicAttack);
    module.add_native_effect("mlbb_heroes:gord_basic_hit", GordBasicHit);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the counting rule ----

    /// Walk the state machine the way a real cast sequence would, and collect
    /// which hit numbers actually paid out.
    fn procs_over(hits: usize) -> Vec<usize> {
        let (mut stacks, mut ready) = (0usize, false);
        let mut out = Vec::new();
        for hit in 1..=hits {
            match mystic_step(ready, stacks) {
                MysticStep::Charge(n) => stacks = n,
                MysticStep::Arm => {
                    stacks = 0;
                    ready = true;
                }
                MysticStep::Proc => {
                    ready = false;
                    out.push(hit);
                }
            }
        }
        out
    }

    #[test]
    fn hits_arm_and_the_next_one_pays() {
        // The proc pays on the hit *after* the arming threshold, so the cadence
        // is one proc per (MYSTIC_HITS + 1) skill hits -- never one per
        // MYSTIC_HITS. Derived from the constant rather than hardcoded, so this
        // guard tracks lever D instead of rotting the moment it is spent.
        let cadence = MYSTIC_HITS + 1;
        assert_eq!(procs_over(cadence), vec![cadence]);
        assert!(
            procs_over(MYSTIC_HITS).is_empty(),
            "the {MYSTIC_HITS}th hit must not pay out"
        );
    }

    #[test]
    fn proc_cadence_over_a_full_rotation() {
        // GORD_SPEC.md §6 prices a rotation at 33 skill hits (S1 + 16 field
        // ticks + 16 beam ticks). Cadence is MYSTIC_HITS + 1 per proc:
        //   MYSTIC_HITS = 4 (pre-rc.5) -> 33 / 5 = 6 procs -> L15 total 1149
        //   MYSTIC_HITS = 3 (lever D)  -> 33 / 4 = 8 procs -> L15 total 1261
        let cadence = MYSTIC_HITS + 1;
        let procs = procs_over(33);
        let expected: Vec<usize> = (1..=33).filter(|h| h % cadence == 0).collect();
        assert_eq!(procs, expected, "procs at {procs:?}");

        // The shape above follows the constant; this line pins the number the
        // balance decision actually chose, so silently moving MYSTIC_HITS fails
        // here rather than shipping a number nobody signed off on.
        assert_eq!(
            procs.len(),
            8,
            "lever D: 8 procs per rotation (L15 1261). MYSTIC_HITS={MYSTIC_HITS}"
        );
    }

    #[test]
    fn steps_before_the_first_proc() {
        // Derived from MYSTIC_HITS rather than spelled out, so this covers the
        // cadence at any threshold instead of pinning the pre-rc.5 value of 4:
        // the counter charges 1..=MYSTIC_HITS-1 and arms at stacks MYSTIC_HITS-1,
        // because the gate is `stacks + 1 >= MYSTIC_HITS`.
        for n in 1..MYSTIC_HITS {
            assert_eq!(
                mystic_step(false, n - 1),
                MysticStep::Charge(n),
                "hit {n} should charge"
            );
        }
        assert_eq!(
            mystic_step(false, MYSTIC_HITS - 1),
            MysticStep::Arm,
            "stacks {} should arm (MYSTIC_HITS={MYSTIC_HITS})",
            MYSTIC_HITS - 1
        );
        assert_eq!(mystic_step(true, 0), MysticStep::Proc);
    }

    #[test]
    fn an_armed_flag_always_pays_regardless_of_stacks() {
        // Once armed, the next hit pays even if a stray stack is still on the
        // target: the flag is the state that matters.
        assert_eq!(mystic_step(true, 3), MysticStep::Proc);
    }

    /// The passive is CASTER-scoped: counter and armed flag live on Gord, the
    /// slow lives on the victim. Guards the 2026-11-06 fix.
    ///
    /// It was target-scoped, which put the `gord_mystic_ready` marker on the
    /// enemy stacked under `gord_mystic_slow` — two 72x72 markers (each the full
    /// size of Gord's sprite) on one victim. The owner reported the clutter.
    /// Moving only the marker would have been incoherent: a marker over Gord has
    /// to describe a state Gord owns, so the counter moved with it.
    #[test]
    fn passive_counter_and_ready_flag_live_on_gord_not_the_victim() {
        let src = body_of(&own_source(), "fn record_mystic_hit", 4200);
        // The read + all three mutations of the counter/flag must name `caster`.
        for call in [
            "has_named_buff(sim, caster, MYSTIC_READY)",
            "stack_count(sim, caster)",
            "entity_stack_buff(caster, &stack",
            "entity_remove_buff(caster, MYSTIC_STACK)",
            "add_buff(caster, &BuffV1::timed(MYSTIC_READY",
            "entity_remove_buff(caster, MYSTIC_READY)",
        ] {
            assert!(
                src.contains(call),
                "missing caster-scoped call `{call}` — the passive drifted back \
                 onto the target, which puts the ready marker on the enemy"
            );
        }
        // ...and they must NOT name `target`.
        for call in [
            "has_named_buff(sim, target, MYSTIC_READY)",
            "stack_count(sim, target)",
            "entity_stack_buff(target, &stack",
            "add_buff(target, &BuffV1::timed(MYSTIC_READY",
        ] {
            assert!(
                !src.contains(call),
                "found target-scoped `{call}` — the passive counter must stay \
                 on Gord"
            );
        }
        // The slow is the exception: it belongs to the victim.
        assert!(
            src.contains("entity_stack_buff(target, &slow"),
            "the slow must stay on the target — it is something that happens \
             to the victim, not to Gord"
        );
    }

    // ---- the damage curve ----

    #[test]
    fn proc_curve_matches_the_spec_table() {
        // GORD_SPEC.md §6: 19 at L1, 38 at L8, 56 at L15 (AP 64 + 8/level).
        let ap_at = |level: usize| 64 + 8 * (level - 1);
        assert_eq!(proc_damage(ap_at(1), 1), 19);
        assert_eq!(proc_damage(ap_at(8), 8), 38);
        assert_eq!(proc_damage(ap_at(15), 15), 56);
    }

    #[test]
    fn proc_never_overflows() {
        assert!(proc_damage(usize::MAX, usize::MAX) > 0);
    }

    // ---- naming and wiring ----

    #[test]
    fn buff_names_are_namespaced() {
        for name in [MYSTIC_STACK, MYSTIC_READY, MYSTIC_SLOW] {
            assert!(
                name.starts_with("mlbb_gord_"),
                "buff name is not namespaced: {name}"
            );
            assert!(
                name.len() < BUFF_NAME_CAP,
                "buff name is too long for BuffV1: {name}"
            );
        }
    }

    #[test]
    fn native_effect_registration_is_visible_to_verify_effects() {
        // verify_effects.py greps for the literal `add_native_effect("...")`,
        // so the name must appear spelled out in the source.
        let src = include_str!("gord.rs");
        assert!(src.contains("add_native_effect(\"mlbb_heroes:gord_mystic_hit\", GordMysticHit)"));
        assert!(src.contains("fn is_tower("), "is_tower helper is missing");
    }

    #[test]
    fn tower_guard_is_wired_into_the_hit_path() {
        // The data selects EnemyWithoutTower, but the native is called from
        // applied_effects and must refuse a tower regardless. Guarded by
        // reading the source so the check cannot silently rot.
        let src = include_str!("gord.rs");
        assert!(
            src.contains("|| is_tower(sim, target)"),
            "record_mystic_hit must reject towers before charging"
        );
    }

    #[test]
    fn proc_damage_is_true_damage() {
        // "True Damage" = Fixed, so magic resistance does not apply. Same call
        // Alpha's Lock-On laser uses; guarded so a future edit cannot swap it
        // for a resisted type by accident.
        let src = include_str!("gord.rs");
        assert!(src.contains("DamageTypeV1::Fixed"));
    }

    #[test]
    fn slow_reaches_the_canon_ceiling_and_no_further() {
        assert_eq!(SLOW_PERCENT * SLOW_STACKS as i32, -40);
        assert_eq!(SLOW_DURATION, 60, "canon is 1 s");
    }

    // ---- voice (Conqueror set) ----

    /// The three voice cues, and the clip each `sound_info` must name.
    ///
    /// Gord's skills are data-driven, so the wiring lives in build_gord.py —
    /// `VOICE(...)` sits at the head of each skill's `Combine`, and audio.rs
    /// turns the suffix into `mlbb_heroes:audio_voice_gord_<slot>`. A missing
    /// piece fails silently in game, so every link is guarded here.
    const VOICE_LINES: &[(&str, &str)] = &[
        ("mlbb_gord_skill1_cast", "mlbb_gord_sfx_vo_skill1"),
        ("mlbb_gord_skill2_cast", "mlbb_gord_sfx_vo_skill2"),
        ("mlbb_gord_ult_cast", "mlbb_gord_sfx_vo_ult"),
    ];

    #[test]
    fn voice_clips_and_sound_info_are_installed() {
        // Same shape as arlott.rs: search upward for `sound/sfx` so the guard
        // does not depend on one checkout layout.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("sound").join("sfx"));
            if !cur.pop() {
                break;
            }
        }
        let dir = roots
            .iter()
            .find(|d| d.is_dir())
            .unwrap_or_else(|| panic!("no sound dir found; looked in {roots:?}"))
            .clone();

        for (line, clip) in VOICE_LINES {
            let info = dir.join(format!("{line}.sound_info"));
            assert!(
                info.is_file(),
                "missing {info:?} — the cue would silently do nothing"
            );
            let body = std::fs::read_to_string(&info).expect("read sound_info");
            assert!(
                body.contains(clip),
                "{line}.sound_info must reference clip {clip}, got: {body}"
            );
            let wav = dir.join(format!("{clip}.wav"));
            assert!(wav.is_file(), "missing clip asset {wav:?}");
            let len = std::fs::metadata(&wav).expect("stat wav").len();
            assert!(len > 1000, "{wav:?} looks empty ({len} B)");
        }
    }

    #[test]
    fn voice_cues_are_registered_in_audio_rs() {
        // The champion data refers to `mlbb_heroes:audio_voice_gord_<slot>`;
        // audio.rs builds those Native names out of VOICE_CUES. Without an
        // entry the Native resolves to nothing and the line never plays.
        let src = include_str!("audio.rs");
        for (line, _clip) in VOICE_LINES {
            assert!(
                src.contains(&format!("\"{line}\"")),
                "VOICE_CUES is missing {line}"
            );
        }
        for slot in ["gord_skill1", "gord_skill2", "gord_ult"] {
            assert!(src.contains(slot), "VOICE_CUES is missing suffix {slot}");
        }
    }

    #[test]
    fn voice_cue_priorities_follow_the_roster() {
        // Ult is high-priority (queued when a line is already playing) the way
        // every other hero's ult is; S1/S2 are droppable chatter.
        let src = include_str!("audio.rs");
        assert!(
            src.contains("(\"gord_skill1\", \"mlbb_gord_skill1_cast\", false, false)"),
            "S1 cue is missing or has the wrong priority"
        );
        assert!(
            src.contains("(\"gord_skill2\", \"mlbb_gord_skill2_cast\", false, false)"),
            "S2 cue is missing or has the wrong priority"
        );
        assert!(
            src.contains("(\"gord_ult\", \"mlbb_gord_ult_cast\", true, false)"),
            "ult cue must be high-priority"
        );
    }

    // ---- localisation ----

    fn object_body<'a>(text: &'a str, from: usize) -> &'a str {
        let open = from + text[from..].find('{').expect("object brace");
        let bytes = text.as_bytes();
        let mut depth = 0usize;
        for i in open..bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &text[open + 1..i];
                    }
                }
                _ => {}
            }
        }
        panic!("unbalanced braces from {open}");
    }

    #[test]
    fn every_locale_describes_gord_completely() {
        // Same guard as arlott.rs, and for the same reason: the champion data
        // points at `text/champion?description.mlbb_gord.<key>` for all four
        // abilities, but nothing enforces that champion.i18n actually carries a
        // `mlbb_gord` object. Without one the panel renders blank and every
        // other test still passes — which is exactly how Gord shipped textless
        // until this was written.
        //
        // Checks STRUCTURE, not prose: each locale's object must carry all six
        // description keys and all four skill_name keys. All eight locales are
        // covered (arlott.rs only checks five).
        //
        // Scope matters — a bare `text.contains("skill2")` would pass no matter
        // what, since every hero has one. The check has to close in on Gord's
        // own object, so it walks braces.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("text").join("champion.i18n"));
            roots.push(cur.join("gord_work").join("text").join("champion.i18n"));
            if !cur.pop() {
                break;
            }
        }
        let path = roots
            .iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("champion.i18n not found; looked in {roots:?}"));
        let text = std::fs::read_to_string(path).expect("read champion.i18n");

        let locales = [
            "\"en\"", "\"zh-hans\"", "\"zh-hant\"", "\"ko\"", "\"ja\"", "\"fr\"", "\"ru\"", "\"vi\"",
        ];
        for (i, loc) in locales.iter().enumerate() {
            let at = text
                .find(loc)
                .unwrap_or_else(|| panic!("locale {loc} missing from champion.i18n"));
            let end = locales
                .get(i + 1)
                .and_then(|next| text[at + loc.len()..].find(next))
                .map(|o| at + loc.len() + o)
                .unwrap_or(text.len());
            let block = &text[at..end];

            // In a locale block the first `mlbb_gord` is the description node
            // and the second is skill_name.
            let first = block
                .find("\"mlbb_gord\"")
                .unwrap_or_else(|| panic!("locale {loc}: no Gord description object"));
            let desc = object_body(block, first);
            for key in ["name", "attack", "passive", "skill", "skill2", "ult"] {
                let needle = format!("\"{key}\"");
                assert!(
                    desc.contains(&needle),
                    "locale {loc}: Gord's description has no {needle}"
                );
            }

            let second = block[first + 1..]
                .find("\"mlbb_gord\"")
                .map(|o| first + 1 + o)
                .unwrap_or_else(|| panic!("locale {loc}: no Gord skill_name object"));
            let names = object_body(block, second);
            for key in ["passive", "skill1", "skill2", "ult"] {
                let needle = format!("\"{key}\"");
                assert!(
                    names.contains(&needle),
                    "locale {loc}: Gord's skill_name has no {needle}"
                );
            }
        }
    }

    #[test]
    fn voice_lock_ticks_match_the_installed_clips() {
        // The global gate should release as each line ends rather than holding
        // a flat 3 s, so the table carries the measured durations (60 ticks/s).
        let src = include_str!("audio.rs");
        for (line, ticks) in [
            ("mlbb_gord_skill1_cast", 89),
            ("mlbb_gord_skill2_cast", 140),
            ("mlbb_gord_ult_cast", 148),
        ] {
            assert!(
                src.contains(&format!("\"{line}\" => {ticks}")),
                "voice_lock_ticks has no {ticks}-tick entry for {line}"
            );
        }
    }

    // ---- the basic attack ----

    /// Read this file's own source, for the guards below.
    fn own_source() -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("gord.rs");
        std::fs::read_to_string(path).expect("read gord.rs")
    }

    /// Slice the body of a named item, so a guard can talk about one function
    /// rather than the whole file (a bare `contains` on the file would pass on
    /// an unrelated mention).
    fn body_of(src: &str, needle: &str, len: usize) -> String {
        let at = src
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} is missing from gord.rs"));
        src[at..(at + len).min(src.len())].to_string()
    }

    #[test]
    fn basic_attack_reaches_towers_but_never_the_nexus() {
        // The kit-wide `is_hostile` filters towers out, which is right for
        // Mystic Favor but wrong here: a basic attack that cannot hit a tower
        // is broken. Hanabi, Karrie and Obsidia all keep the Nexus excluded.
        let src = own_source();
        let body = body_of(&src, "fn is_hostile_basic(", 500);
        assert!(
            !body.contains("is_tower"),
            "is_hostile_basic excludes towers; Gord cannot basic-attack them"
        );
        assert!(
            body.contains("EntityKindV1::Nexus"),
            "is_hostile_basic must still exclude the Nexus"
        );

        // Both basic call sites have to use it, not the tower-filtering one.
        for site in ["struct GordBasicAttack", "struct GordBasicHit"] {
            let b = body_of(&src, site, 2500);
            assert!(
                b.contains("is_hostile_basic("),
                "{site} gates on the wrong predicate"
            );
        }
    }

    #[test]
    fn basic_attack_does_not_charge_mystic_favor() {
        // Canon counts "hits an enemy with skills". Gord's field and beam land
        // 32 ticks a rotation already, so letting basics charge the passive
        // would make it fire near-continuously.
        let body = body_of(&own_source(), "struct GordBasicAttack", 2500);
        assert!(
            !body.contains("record_mystic_hit"),
            "the basic attack charges Mystic Favor; canon is skills only"
        );
        assert!(
            !body.contains("gord_mystic_hit"),
            "the basic attack pulls in the passive native"
        );
    }

    #[test]
    fn basic_attack_pays_full_ad_like_the_roster_attack_node() {
        // Every non-native hero here uses `Attack { damage: 0, attack_ratio:
        // 100 }`, so Gord's native must land on the same number or he will be
        // quietly weaker or stronger than the roster.
        assert_eq!(BASIC_DAMAGE_BASE, 0);
        assert_eq!(BASIC_ATTACK_RATIO, 100);
        // percent() rounds half-up, so 100% must be exact at every scale.
        for ad in [0usize, 1, 47, 48, 133, 999] {
            assert_eq!(
                percent(ad, BASIC_ATTACK_RATIO),
                ad,
                "100% AD is not identity at AD={ad}"
            );
        }
    }

    #[test]
    fn basic_bolt_projectile_is_bound_in_view_projectiles() {
        // A name in view_effects instead of view_projectiles (or a typo) draws
        // nothing and fails silently, which is exactly how this bug shipped.
        // The verifier checks the built data; this pins the constant that feeds
        // it, so the two cannot drift apart.
        let body = body_of(&own_source(), "struct GordBasicAttack", 2500);
        assert!(
            body.contains("spawn_projectile(BASIC_PROJECTILE"),
            "the basic attack no longer spawns a projectile"
        );
        assert!(
            body.contains("BASIC_HIT_EFFECT"),
            "damage must ride the landing effect, not the launch"
        );
    }

    // ---- the ult beam, and the two impacts that were removed ----
    //
    // Both guards below fail on the shipped v2.0.6-rc.1 behaviour and pass on
    // the fixed data.

    /// The raw data file, embedded so the guards travel with the crate.
    const GORD_DATA: &str = include_str!("../champion/mlbb_gord.data_champion");

    /// Reads an integer field that appears after `anchor` in the raw data text.
    /// Deliberately dependency-free — the crate has no serde, and pulling one
    /// in for a guard is not worth it. (Same helper as franco.rs.)
    fn field_after(text: &str, anchor: &str, field: &str) -> usize {
        let start = text.find(anchor).expect("anchor missing from data file");
        let rest = &text[start..];
        let key = format!("\"{field}\"");
        let at = rest.find(&key).expect("field missing after anchor");
        let tail = &rest[at + key.len()..];
        let colon = tail.find(':').expect("malformed field");
        let digits: String = tail[colon + 1..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse().expect("field is not an integer")
    }

    #[test]
    fn beam_outlives_one_frame_of_art() {
        // The owner reported the ult "still not appearing" across two builds.
        // Shipped was delay 1, apply 1 -> a ZERO-tick life, so the beam was
        // created and deleted in the same tick. Per the project's own recorded
        // SDK probe: a projectile is REMOVED at `delay - 1` ticks.
        let delay = field_after(GORD_DATA, "\"name\": \"gord_gush_wave\"", "delay");
        let apply = field_after(GORD_DATA, "\"name\": \"gord_gush_wave\"", "apply");
        assert!(
            apply < delay,
            "apply {apply} must never reach delay {delay} — the beam would be \
             removed before it can hit"
        );
        // 18 is what every other visible beam in this roster ships
        // (arlott_s1_slam, franco_hook, obsidia_needle, obsidia_hunt_throw,
        // alpha_s1_wave). Franco carried this exact bug and the owner confirmed
        // the fix in game.
        let lifetime = delay - 1;
        assert!(
            lifetime >= 12,
            "beam lives {lifetime} ticks ({} ms) — too short to see; \
             the art alone runs 18 ticks",
            lifetime * 1000 / 60
        );
    }

    #[test]
    fn s2_and_ult_carry_no_per_hit_impact() {
        // These two were removed 2026-11-06 at the owner's call. Their art was
        // a byte-for-byte copy of gord_basic_hit's first four frames, so S2 and
        // the ult were drawing the BASIC ATTACK's impact on every tick. The
        // passive markers (mystic_ready / mystic_slow) carry the feedback.
        //
        // The check is on the whole file, not per-slot: the failure mode was a
        // name reused across slots, so any reappearance anywhere is the bug.
        for name in ["gord_injunction_hit", "gord_gush_hit"] {
            assert!(
                !GORD_DATA.contains(name),
                "{name} is back — its art was the basic attack's impact. \
                 Do not re-add it without art of its own."
            );
        }
    }

    #[test]
    fn the_basic_attack_impact_is_still_the_only_one_of_its_kind() {
        // Guard the other half of the same bug: gord_basic_hit must stay
        // referenced by the basic attack only, so it cannot leak onto a skill.
        let body = body_of(&own_source(), "struct GordBasicHit", 2000);
        assert!(
            body.contains("BASIC_HIT_FX"),
            "the basic attack's landing effect must stay on the basic attack"
        );
        // And it must not leak onto a skill. The binding legitimately mentions
        // it twice — once as `name`, once in the `anim` path — so we cannot
        // just count occurrences. Instead: gord_basic_hit may appear ONLY
        // inside the view_effects block.
        //
        // (Bounding by action slot does not work: `ult` is followed by
        // view_projectiles / view_effects / view_buffs before `passive`, so an
        // "ult" slice would swallow the binding we are trying to exempt.)
        let ve = GORD_DATA
            .find("\"view_effects\"")
            .expect("data file has no view_effects block");
        let vb = GORD_DATA
            .find("\"view_buffs\"")
            .expect("data file has no view_buffs block");
        assert!(ve < vb, "view_effects must precede view_buffs");
        assert!(
            !GORD_DATA[..ve].contains("gord_basic_hit")
                && !GORD_DATA[vb..].contains("gord_basic_hit"),
            "gord_basic_hit escaped the view_effects block — that is the bug \
             that was just removed (S2 and the ult drawing the basic attack's \
             impact on every tick)"
        );
    }
}
