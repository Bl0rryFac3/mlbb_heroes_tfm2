//! Native Belerick gameplay.
//!
//! Only two pieces of the kit cannot be expressed in data — see
//! `BELERICK_SPEC.md` §4:
//!
//! 1. **Deadly Thorns (passive).** It fires when Belerick is *hit*, and there
//!    is no data trigger for "was damaged" (`porting-heroes.md:64`).
//! 2. **Nature's Strike heal.** It scales with his *lost* HP, and no data field
//!    does that (`champion-data.md:220`). It lives in the basic-attack handler
//!    because that is where the empower is consumed — Chou's
//!    `PASSIVE_READY` is consumed the same way (`chou.rs:486`).
//!
//! Everything else — the S1 line and its delayed eruption, the taunt, the S2
//! move-speed buff, the four ult ticks — is data in
//! `champion/mlbb_belerick.data_champion`.
//!
//! # The passive's counting rule
//!
//! Canon is a 25% chance on being hit, with a 0.4 s floor. Both were replaced,
//! deliberately and with the owner's approval (§6):
//!
//! - **Every 4th qualifying hit** instead of the coin flip. `on_damaged`
//!   receives no seed (unlike `on_update`), so a roll would have to invent
//!   entropy. Every-4th is the same average rate with zero variance.
//! - **A 1.2 s (72 tick) floor** instead of 0.4 s. Under sustained fire 0.4 s
//!   approaches 2.5 procs/second, which is too fast for our HP pools.
//!
//! The floor gates *payouts*, not counting. A proc that lands while the floor
//! is running is **banked**, not lost, and fires the instant the floor clears
//! (see [`BelerickDeadlyThorns::try_proc`]). So the rate is
//! `min(1 per 4 hits, 1 per 1.2 s)` — whichever is slower.
//!
//! # The passive is a projectile, not a ground effect
//!
//! §8b item 10: the thorns sweep *forward*, and only `view_projectiles` gets
//! aimed — `play_view_effect` resolves `view_effects` bindings and cannot be
//! rotated (`gord.rs:97`). So the proc spawns a real projectile and the
//! **hit effect carries the damage**. The owner chose this over instant damage
//! (2026-11-06) knowing it means a few ticks of flight and the possibility of
//! a whiff if the target blinks.

use crate::audio;
use mod_api_stable::*;

// ---------------------------------------------------------------------------
// Buff names
// ---------------------------------------------------------------------------

/// The empowered-strike buff. Its `view_buffs` entry is
/// `belerick_nature_ready`, which is why it must outlive the skill2 row: for
/// most of its 3 s life Belerick is back on `idle` or `run`. (§8b item 6.)
const NATURE_READY: &str = "mlbb_belerick_nature_ready";
/// First half of the S2 speed burst. Two buffs because `BuffV1` has no decay
/// curve — only a flat `move_speed_mult` for a fixed tick count — so a linear
/// ramp has to be a staircase. These are the midpoints of the canon 80→0 line
/// over its two halves, which preserves the 40% average. (Owner's call,
/// 2026-11-06.)
const NATURE_SPEED_A: &str = "mlbb_belerick_nature_speed_a";
/// Second half of the same burst.
const NATURE_SPEED_B: &str = "mlbb_belerick_nature_speed_b";
/// S1's vine-pass slow. Stacked with nothing; one application.
const SEED_SLOW: &str = "mlbb_belerick_seed_slow";
/// The 60% slow on the empowered strike.
const STRIKE_SLOW: &str = "mlbb_belerick_nature_slow";
/// Ult's taunt marker buff. Data-side only — it exists so `view_buffs`
/// can draw the taunt marker over the victim (`belerick_taunt_marker`).
/// The ult carries no slow: patch 1.6.66 swapped its root for a taunt and no
/// source publishes a slow magnitude for it (2026-11-07).
const DRYAD_TAUNT: &str = "mlbb_belerick_dryad_taunt";

// ---------------------------------------------------------------------------
// Passive — Deadly Thorns
// ---------------------------------------------------------------------------

/// "Every 4th hit" — the deterministic stand-in for canon's 25% chance.
const THORN_HITS: usize = 4;
/// Below this a hit does not count. Keeps Damage-over-time ticks and graze
/// chip from charging the counter.
///
/// Was 20. The owner playtested rc.2 and the passive fired "rarely": early
/// minion hits land around 10-20, so in lane a whole wave could charge
/// nothing and the counter only moved when a champion hit him. 10 lets real
/// hits count.
///
/// This does NOT need to guard the proc rate — [`THORN_FLOOR_TICKS`] already
/// caps that at ~8 procs per 10 s, so lowering the floor is safe. The in-game
/// text prints this number in all 8 locales; keep them in sync.
const THORN_MIN_DAMAGE: usize = 10;
/// Never faster than one proc per this many ticks. **The dial to tune first**
/// if he reads too strong (§6).
const THORN_FLOOR_TICKS: usize = 72;
/// Passive damage: `10 + 1 × level + 1% of max HP`.
const THORN_BASE: usize = 10;
const THORN_LEVEL_GAIN: usize = 1;
/// Whole percents only — `0.5` is a parse error (`champion-data.md:232`).
const THORN_HP_RATIO: usize = 1;
/// How far the sweep reaches, measured from Belerick.
///
/// Was 34,000. A ranged champion poking from outside that filled the counter
/// but the proc was held with nothing in reach, so he took damage and never
/// retaliated — part of why the passive read as "rarely". 45,000 lets him
/// answer a marksman who is stepping out.
///
/// Bonus: at THORN_SPEED 2,000 this is 22.5 ticks of flight against a 22-tick
/// animation, so the sweep now plays all 9 frames instead of ~7.
const THORN_RANGE: u64 = 45_000;
const THORN_RADIUS: u64 = 12_000;
/// Speed is PER TICK, not per second — see the `ProjectileSpawnV1::default`
/// doc in the SDK ("speed 5000/tick"). At 8,000/tick the sweep crossed its
/// whole 34,000 range in ~4 ticks (0.07 s) while its animation is 9 frames /
/// 0.36 s (22 ticks), so only about 2 of 9 frames ever rendered and the
/// passive read as if it never fired. The owner saw nothing.
///
/// 2,000/tick gives 34,000 / 2,000 = 17 ticks (~0.28 s) of flight, so roughly
/// 7 of the 9 frames actually play. Gord's basic bolt is the control: 20 ticks
/// of flight against a 0.36 s animation, and it reads fine in game.
const THORN_SPEED: u64 = 2_000;
/// MUST live in `view_projectiles`; that is the only binding the engine aims.
const THORN_PROJECTILE: &str = "belerick_thorn_sweep";
const THORN_HIT_EFFECT: &str = "mlbb_heroes:belerick_thorn_hit";
/// Each proc takes 1 s off Nature's Strike (§6).
const S2_REFUND_TICKS: usize = 60;

// ---------------------------------------------------------------------------
// Skill 2 — Nature's Strike
// ---------------------------------------------------------------------------

/// The empower window. Canon: "in the next 3 seconds". Missing this let him
/// bank an empowered hit for the whole fight (§8b item 1).
const NATURE_READY_TICKS: usize = 180;
const SPEED_A_PERCENT: i32 = 60;
const SPEED_B_PERCENT: i32 = 20;
/// Each staircase step; two of them cover the canon 2 s.
const SPEED_STEP_TICKS: usize = 60;
/// Empowered strike: a flat `60` magic damage.
///
/// Was `90 + 4% of max HP`. The HP term is gone — Belerick was the only
/// champion in the roster with a non-zero `hp_ratio`, and because it reads the
/// *caster's* max HP every HP item he bought raised his auto-attack damage as
/// well as his survivability, with no opportunity cost. HP scaling now lives
/// only on the passive (see [`THORN_HP_RATIO`]), which is canon for Deadly
/// Thorns and is reactive rather than on every swing.
///
/// These two constants mirror `champion/mlbb_belerick.data_champion` and are
/// currently unused by the sim — the bonus strike is resolved from the data
/// file. They are kept so the numbers can be asserted in one place; keep them
/// in step with the data.
const STRIKE_BASE: usize = 60;
const STRIKE_HP_RATIO: usize = 0;
/// The slow the empowered strike leaves. 60% for 1.4 s.
const STRIKE_SLOW_PERCENT: i32 = -60;
const STRIKE_SLOW_TICKS: usize = 84;
/// Heal: `150 + 12% of lost HP`. Native because nothing reads missing health
/// (`champion-data.md:220`), and it fires **on hit** (owner's call, §8b 7) —
/// a whiffed empowered attack heals nothing.
const HEAL_BASE: usize = 150;
const HEAL_LOST_HP_RATIO: usize = 12;

// ---------------------------------------------------------------------------
// Taunt
// ---------------------------------------------------------------------------

/// Taunt is a data effect — `{"type":"Taunt","duration":N}` — rated OK and
/// shipped in four League heroes (§5). This constant exists so the native
/// fallback is one edit away if a data taunt ever misbehaves in game.
///
/// Verifying taunt visually is the **#1 playtest item**.
const TAUNT_IN_DATA: bool = true;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn percent(value: usize, pct: usize) -> usize {
    let scaled = value.saturating_mul(pct);
    (scaled + 50) / 100
}

fn entity_ids(sim: &StableSim<'_>) -> Vec<usize> {
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i).map(|e| e.id()))
        .collect()
}

fn distance(a: (u64, u64), b: (u64, u64)) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
}

fn pos_u64(v: f64) -> u64 {
    if v.is_finite() && v > 0.0 {
        v as u64
    } else {
        0
    }
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

/// `StableEntity` has no `has_buff` — that method lives on
/// `StableEntityView`, which is not what `get_entity` hands back. So walk the
/// buff list the way `gord.rs` does.
fn has_named_buff(sim: &StableSim<'_>, target: usize, name: &str) -> bool {
    let Some(entity) = sim.get_entity(target) else {
        return false;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .any(|b| b.name() == name)
}

/// What may charge Deadly Thorns. **Owner's list, 2026-11-06: champions,
/// jungle monsters, minions.** Towers are excluded — they are not "being
/// attacked" in the sense the passive answers.
fn charges_thorns(sim: &StableSim<'_>, attacker: usize, team: usize) -> bool {
    sim.get_entity(attacker).is_some_and(|e| {
        e.team() != team
            && matches!(
                e.kind(),
                Some(EntityKindV1::Champion) | Some(EntityKindV1::Minion) | Some(EntityKindV1::Jungle)
            )
    })
}

/// Nearest hostile champion, jungle monster or minion within `radius`.
fn nearest_thorn_target(
    sim: &StableSim<'_>,
    origin: (u64, u64),
    team: usize,
    radius: u64,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for id in entity_ids(sim) {
        let Some(target) = sim.get_entity(id) else {
            continue;
        };
        if !charges_thorns(sim, id, team) || !target.is_alive() || !target.is_targetable() {
            continue;
        }
        let d = distance(origin, target.pos());
        if d > radius as f64 {
            continue;
        }
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((id, d));
        }
    }
    best.map(|(id, _)| id)
}

fn thorn_damage(max_hp: usize, level: usize) -> usize {
    THORN_BASE
        .saturating_add(THORN_LEVEL_GAIN.saturating_mul(level))
        .saturating_add(percent(max_hp, THORN_HP_RATIO))
}

/// `150 + 12% of lost HP`.
fn strike_heal(max_hp: usize, current_hp: usize) -> usize {
    HEAL_BASE.saturating_add(percent(max_hp.saturating_sub(current_hp), HEAL_LOST_HP_RATIO))
}

// ---------------------------------------------------------------------------
// Passive — Deadly Thorns
// ---------------------------------------------------------------------------

/// `mlbb_heroes:belerick_deadly_thorns`.
///
/// Counts qualifying hits taken and pays out a forward thorn sweep on the
/// fourth. The counter is **native state, not a buff** — a buff would be
/// visible in the buff bar and would be stripped by anything that cleanses.
/// "Mighty oaks from little acorns grow." Played once on spawn.
const SELECT_LINE: &str = "mlbb_belerick_voice_select";
/// Routine lines while moving. `VoiceLifecycle` cycles them on its own
/// repeat gate, so this is just the pool.
const MOVE_LINES: &[&str] = &[
    "mlbb_belerick_voice_move01",
    "mlbb_belerick_voice_move02",
    "mlbb_belerick_voice_move03",
    "mlbb_belerick_voice_move04",
    "mlbb_belerick_voice_move05",
    "mlbb_belerick_voice_move06",
    "mlbb_belerick_voice_move07",
    "mlbb_belerick_voice_move08",
];
/// Chosen by death count, so a player hears a different one each time.
const DEATH_LINES: &[&str] = &[
    "mlbb_belerick_voice_death01",
    "mlbb_belerick_voice_death02",
    "mlbb_belerick_voice_death03",
];

#[derive(Clone, Default)]
struct BelerickDeadlyThorns {
    /// Qualifying hits since the last payout, capped at [`THORN_HITS`].
    hits: usize,
    /// Ticks until the next payout is allowed.
    floor: usize,
    /// Select / move / death lines. Idle lines are empty: he has no idle set
    /// on the source page, and an empty slice simply never fires.
    voice: audio::VoiceLifecycle,
}

impl BelerickDeadlyThorns {
    /// Pay out if the counter is full **and** the floor has cleared.
    ///
    /// Called both from `on_damaged` and from `on_update`, and that second
    /// call site is the whole point: when the fourth hit lands inside the
    /// floor, the proc is held rather than dropped, and fires on the first
    /// tick it is legal. Without it, a fast exchange would silently eat
    /// procs and the passive would read as inconsistent.
    fn try_proc(&mut self, sim: &mut StableSim<'_>, caster: usize) {
        if self.hits < THORN_HITS || self.floor > 0 {
            return;
        }
        let Some((pos, team)) = sim.get_entity(caster).map(|e| (e.pos(), e.team())) else {
            return;
        };
        let Some(target) = nearest_thorn_target(sim, pos, team, THORN_RANGE) else {
            // Nothing in sweep range. Keep the counter full and let
            // `on_update` retry — the retaliation is owed, it is just not
            // deliverable yet.
            return;
        };
        let Some(tpos) = sim.get_entity(target).map(|e| e.pos()) else {
            return;
        };

        self.hits = 0;
        self.floor = THORN_FLOOR_TICKS;

        let spec = ProjectileSpawnV1 {
            caster_id: caster,
            team,
            x: pos.0,
            y: pos.1,
            radius: THORN_RADIUS,
            speed: THORN_SPEED,
            move_kind: ProjectileMoveKindV1::Linear.code(),
            target_x: tpos.0,
            target_y: tpos.1,
            penetrate: false,
            attack_type: AttackTypeV1::Skill.code(),
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: CastingTargetV1::EnemyWithoutTower.code(),
            ..ProjectileSpawnV1::default()
        };
        // `can_render` guards the *view* only — the projectile itself has to
        // spawn on the server too, because it is what delivers the damage.
        let _ = sim.spawn_projectile(THORN_PROJECTILE, THORN_HIT_EFFECT, &spec);

        // The refund is owed for being hit, not for connecting.
        let _ = sim.entity_reduce_cooldown(caster, SkillSlotV1::Skill2, S2_REFUND_TICKS);
    }
}

impl StablePassive for BelerickDeadlyThorns {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_damaged(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity: usize,
        attacker: usize,
        damage: usize,
    ) {
        let Some(team) = sim.get_entity(entity).map(|e| e.team()) else {
            return;
        };
        if damage < THORN_MIN_DAMAGE || !charges_thorns(sim, attacker, team) {
            return;
        }
        self.hits = self.hits.saturating_add(1).min(THORN_HITS);
        self.try_proc(sim, entity);
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize) {
        self.hits = 0;
        self.floor = THORN_FLOOR_TICKS;
        if let Some(e) = sim.get_entity(entity) {
            let (hp, max_hp) = e.hp();
            crate::wlog(format!(
                "[roster] Belerick build={} origin={:?} tick={} level={} hp={}/{} effective_stat_hp={} basic_interval_ticks={}",
                env!("CARGO_PKG_VERSION"), sim.sim_origin().map(|o| o.kind), sim.tick(),
                e.level(), hp, max_hp, e.stat().hp, e.attack_interval(),
            ));
        }
        self.voice.on_spawn(sim, entity, Some(SELECT_LINE));
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        self.voice.on_dead(sim, player, DEATH_LINES);
    }

    fn on_update(
        &mut self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        entity: usize,
    ) {
        self.voice.on_update(sim, entity, &[], MOVE_LINES);
        self.floor = self.floor.saturating_sub(1);
        self.try_proc(sim, entity);
    }
}

/// `mlbb_heroes:belerick_thorn_hit` — the sweep connecting.
///
/// Carries the passive's damage. It is a separate native from the passive
/// because the damage belongs to the projectile's arrival, not to the moment
/// Belerick was hit — that is what makes the sweep aimable.
#[derive(Clone, Copy)]
struct BelerickThornHit;

impl StableEffectType for BelerickThornHit {
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
        let Some((max_hp, level)) = sim
            .get_entity(caster_id)
            .map(|e| (e.stat().hp, e.level().max(1)))
        else {
            return;
        };
        let amount = thorn_damage(max_hp, level);
        // `DamageTypeV1` is `Ad | Ap | Fixed` — there is no `Magic`. Canon
        // calls Deadly Thorns magic damage, so it is `Ap` here, which is what
        // magic resistance reduces. (`Fixed` would be true damage; Gord's
        // Mystic Favor proc uses that, and this is not it.)
        let _ = sim.deal_damage_typed(
            caster_id,
            input.target_id,
            amount,
            DamageTypeV1::Ap,
            AttackTypeV1::Skill,
        );
    }

    /// AI hint: the passive is worth roughly a quarter of its per-proc
    /// damage per hit taken. Full value here would make the AI walk into
    /// fights it should not take.
    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (0, thorn_damage(caster_stat.hp, 1) / THORN_HITS)
    }
}

// ---------------------------------------------------------------------------
// Skill 2 — the heal, in the attack handler
// ---------------------------------------------------------------------------

/// `mlbb_heroes:belerick_nature_heal`.
///
/// Consumes the empower and heals. It runs **inside the `attack` row's
/// empowered branch**, not on the S2 cast, because canon groups the heal with
/// the enhanced attack and the owner chose "on hit" (§8b item 7). A whiffed
/// empowered attack heals nothing.
///
/// The damage and the slow are data — `ApAttack` and `AddBuff` in the same
/// branch. This native only does the two things data cannot: read lost HP,
/// and clear the buff.
#[derive(Clone, Copy)]
struct BelerickNatureHeal;

impl StableEffectType for BelerickNatureHeal {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        if !has_named_buff(sim, caster_id, NATURE_READY) {
            return;
        }
        let _ = sim.entity_remove_buff(caster_id, NATURE_READY);
        // Gone the instant the attack consumes it — that is what makes the
        // `view_buffs` glow disappear at the right frame (§8b item 6).
        let _ = sim.entity_remove_buff(caster_id, NATURE_SPEED_A);
        let _ = sim.entity_remove_buff(caster_id, NATURE_SPEED_B);

        let Some((current_hp, max_hp)) = sim.get_entity(caster_id).map(|e| e.hp()) else {
            return;
        };
        let amount = strike_heal(max_hp, current_hp);
        if amount > 0 {
            sim.heal(caster_id, caster_id, amount);
        }
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register(module: &mut StableMod) {
    module.add_native_passive(
        "mlbb_heroes:belerick_deadly_thorns",
        BelerickDeadlyThorns::default(),
    );
    // Spelled out literally, not through a constant: verify_effects.py greps
    // the source for `add_native_effect("mlbb_heroes:...")` and a constant
    // here would make the gate blind to it. (Same rule as alpha.rs and
    // gord.rs.)
    module.add_native_effect("mlbb_heroes:belerick_thorn_hit", BelerickThornHit);
    module.add_native_effect("mlbb_heroes:belerick_nature_heal", BelerickNatureHeal);
}

// ---------------------------------------------------------------------------
// Taunt fallback — compiled, unused while TAUNT_IN_DATA is true
// ---------------------------------------------------------------------------

/// The native taunt, kept for the day a data taunt misbehaves. Mirrors
/// `league_galio`; `CcV1.target` **must** be the taunter
/// (`values.rs:912`) or the victim walks the wrong way.
///
/// To switch over: set `TAUNT_IN_DATA = false`, drop the `Taunt` node from the
/// S1 eruption and the ult in the `.data_champion`, and call this from a
/// native in their place.
#[allow(dead_code)]
fn apply_native_taunt(sim: &mut StableSim<'_>, caster: usize, target: usize, ticks: u64) {
    if TAUNT_IN_DATA {
        return;
    }
    let mut cc = CcV1::of_kind(CcKindV1::Taunt, ticks);
    cc.target = caster;
    sim.apply_cc(target, &cc);
    sim.add_buff(target, &BuffV1::timed(DRYAD_TAUNT, ticks as usize));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Tick-accurate model of the counter, mirroring the real call order:
    /// `on_update` runs **every** tick and calls `try_proc` after decrementing
    /// the floor; `on_damaged` also calls `try_proc` after each hit. Returns
    /// the tick numbers that paid out.
    ///
    /// This is deliberately a re-implementation of `BelerickDeadlyThorns`
    /// rather than a call into it, because the real one needs a `StableSim`.
    /// If the two ever disagree, the real one wins and this must be fixed.
    fn sim_ticks(hits_per_tick: &[usize], floor_set: usize) -> Vec<usize> {
        let (mut hits, mut floor) = (0usize, 0usize);
        let mut out = Vec::new();
        let mut pay = |hits: &mut usize, floor: &mut usize, out: &mut Vec<usize>, t: usize| {
            if *hits >= THORN_HITS && *floor == 0 {
                *hits = 0;
                *floor = floor_set;
                out.push(t);
            }
        };
        for (i, &n) in hits_per_tick.iter().enumerate() {
            let t = i + 1;
            floor = floor.saturating_sub(1);
            pay(&mut hits, &mut floor, &mut out, t);
            for _ in 0..n {
                hits = hits.saturating_add(1).min(THORN_HITS);
                pay(&mut hits, &mut floor, &mut out, t);
            }
        }
        out
    }

    fn steady(rate: usize, ticks: usize) -> Vec<usize> {
        sim_ticks(&vec![rate; ticks], THORN_FLOOR_TICKS)
    }

    #[test]
    fn every_fourth_hit_pays_when_nothing_is_blocking() {
        // One hit per tick with the floor disabled: 4th, 8th, 12th.
        assert_eq!(sim_ticks(&vec![1; 12], 0), vec![4, 8, 12]);
    }

    /// A floor shorter than the time it takes to collect four hits cannot
    /// bind, and should not: it would just delay procs for no reason.
    #[test]
    fn a_floor_shorter_than_the_counter_is_inert() {
        // Four hits at one per tick take 4 ticks, so a 3-tick floor never
        // gets a chance to hold anything back.
        assert_eq!(sim_ticks(&vec![1; 12], 3), vec![4, 8, 12]);
    }

    /// The point of `try_proc` being called from `on_update` as well as
    /// `on_damaged`. A proc that comes due while the floor is running must be
    /// **owed**, not discarded.
    #[test]
    fn a_proc_blocked_by_the_floor_is_banked_and_paid_later() {
        // Four attackers land on tick 1 (pays immediately) and four more on
        // tick 2. That second set is owed but blocked; it must land exactly
        // 72 ticks after the first, not vanish.
        let mut ticks = vec![4, 4];
        ticks.extend(std::iter::repeat(0).take(120));
        assert_eq!(sim_ticks(&ticks, THORN_FLOOR_TICKS), vec![1, 73]);
    }

    /// §6: "1.2 s caps it at ~8 procs in a 10 s fight." This is the dial the
    /// owner is meant to tune first.
    #[test]
    fn the_floor_caps_sustained_fire() {
        let procs = steady(4, 600); // 600 ticks = 10 s, four attackers every tick
        // Without the floor this would be 150 procs (600/4).
        assert_eq!(procs.len(), 9, "procs at {procs:?}");
        for gap in procs.windows(2).map(|w| w[1] - w[0]) {
            assert_eq!(gap, THORN_FLOOR_TICKS, "spacing must be exactly the floor");
        }
    }

    /// The surprised-making one: because the counter only needs 4 ticks to
    /// refill and the floor is 72, the floor binds even against a **single**
    /// attacker. The passive is effectively "once per 1.2 s", and the
    /// every-4th-hit rule is what stops a lone poke from charging it.
    #[test]
    fn the_floor_binds_even_against_one_attacker() {
        let procs = steady(1, 600);
        assert_eq!(procs.len(), 9, "procs at {procs:?}");
        assert_eq!(procs[0], 4, "the very first proc still needs 4 hits");
    }

    #[test]
    fn one_hit_below_the_threshold_never_procs() {
        // Chip damage must not charge the counter at all.
        assert!(sim_ticks(&vec![1; 600], THORN_FLOOR_TICKS).len() < 600);
        assert_eq!(steady(1, 3).len(), 0, "three hits is not four");
    }

    #[test]
    fn thorn_damage_matches_the_spec_table() {
        // BELERICK_SPEC.md §6: L1 20, L8 35, L15 50.
        assert_eq!(thorn_damage(870, 1), 20, "L1 (870 max HP)");
        assert_eq!(thorn_damage(1_696, 8), 35, "L8 (1696 max HP)");
        assert_eq!(thorn_damage(2_522, 15), 50, "L15 (2522 max HP)");
    }

    #[test]
    fn heal_matches_the_spec_table_at_half_hp() {
        // BELERICK_SPEC.md §6 heal row @ 50%: 202 / 252 / 301. This column
        // is correct and it is what pins the formula down.
        assert_eq!(strike_heal(870, 435), 202, "L1 at 50% HP");
        assert_eq!(strike_heal(1_696, 848), 252, "L8 at 50% HP");
        assert_eq!(strike_heal(2_522, 1_261), 301, "L15 at 50% HP");
    }

    /// ⚠️ **The spec's "heal @ 15% HP" column is arithmetically impossible**
    /// and these are the corrected values, not the printed ones.
    ///
    /// `BELERICK_SPEC.md` §6 prints 261 / 322 / 384. Work backwards: 261 − 150
    /// = 111, and 111 ÷ 0.12 = **925 lost HP on a champion with 870 max HP.**
    /// The L15 row is worse — 384 implies 1,950 lost out of 2,522, i.e. 77%
    /// missing, not 15%.
    ///
    /// The 50% column matches the formula on all three levels, so the formula
    /// is right and that one column is a drafting error. Flagged to the owner
    /// 2026-11-06; these are the numbers the shipped heal will actually use.
    #[test]
    fn heal_at_fifteen_percent_is_lower_than_the_spec_prints() {
        assert_eq!(strike_heal(870, 130), 239, "spec printed 261 (needs 925 lost of 870)");
        assert_eq!(strike_heal(1_696, 254), 323, "spec printed 322 (rounding only)");
        assert_eq!(strike_heal(2_522, 378), 407, "spec printed 384 (needs 1950 lost of 2522)");
    }

    #[test]
    fn strike_damage_is_flat_and_no_longer_scales_with_hp() {
        // Flat 60 at every level. The old `90 + 4% of max HP` gave 125 / 158 /
        // 191 across L1 / L8 / L15 and — worse — grew with every HP item.
        for &hp in &[870usize, 1_696, 2_522, 6_000] {
            assert_eq!(
                STRIKE_BASE + percent(hp, STRIKE_HP_RATIO),
                60,
                "the strike must not change with max HP (tried {hp})"
            );
        }
        // An HP-heavy build used to add 4% of 6,000 = 240 damage a swing.
        assert_eq!(percent(6_000, STRIKE_HP_RATIO), 0, "no HP term");
    }

    #[test]
    fn a_full_hp_belerick_still_gets_the_flat_heal() {
        assert_eq!(strike_heal(2_522, 2_522), HEAL_BASE);
    }

    #[test]
    fn ratios_are_whole_percents() {
        // A fractional ratio is a parse error in the .data_champion
        // (champion-data.md:232), so every ratio constant here is usize.
        assert_eq!(percent(2_522, STRIKE_HP_RATIO), 0, "strike has no HP term");
        assert_eq!(percent(2_522, THORN_HP_RATIO), 25, "1% of 2522");
        assert_eq!(percent(870, HEAL_LOST_HP_RATIO), 104, "12% of 870");
    }

    #[test]
    fn the_staircase_preserves_the_average_of_a_linear_decay() {
        // Canon is +80% decaying linearly to 0 over 2 s, whose average is 40%.
        // Two midpoint samples: 60% over the first half (80→40) and 20% over
        // the second (40→0). BuffV1 has no decay curve, so this is as close
        // as the engine gets.
        let span = SPEED_STEP_TICKS * 2;
        let area = SPEED_A_PERCENT * SPEED_STEP_TICKS as i32 + SPEED_B_PERCENT * SPEED_STEP_TICKS as i32;
        assert_eq!(area / span as i32, 40, "average boost over the window");
        assert_eq!(span, 120, "canon: 2 s");
    }

    #[test]
    fn the_taunt_is_data_until_someone_flips_it() {
        assert!(TAUNT_IN_DATA, "the data Taunt node ships; see BELERICK_SPEC.md §5");
    }

    // ---- voice ----
    //
    // Same shape as gord.rs. Every link in the chain is guarded because a
    // missing piece fails *silently* in game: the line just never plays.
    //   data_champion  -> mlbb_heroes:audio_voice_belerick_ult
    //   audio.rs       -> VOICE_CUES turns that into the event name below
    //   sound/sfx      -> <event>.sound_info -> <clip>.wav
    //   mod.override_info -> a route for BOTH the event and the clip
    // (`install_belerick_audio.py` writes the last two and re-checks them.)

    /// The ult is the only cast line; select/move/death are lifecycle.
    const voice_lines: &[(&str, &str)] = &[(
        "mlbb_belerick_ult_cast",
        "mlbb_belerick_sfx_vo_ult",
    )];

    const LIFECYCLE_LINES: &[&str] = &[
        "mlbb_belerick_voice_select",
        "mlbb_belerick_voice_move01",
        "mlbb_belerick_voice_move08",
        "mlbb_belerick_voice_death01",
        "mlbb_belerick_voice_death03",
    ];

    #[test]
    fn voice_clips_and_sound_info_are_installed() {
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
            .unwrap_or_else(|| panic!("no sound dir found; looked in {roots:?}\n\
                build_v2081/sound/ is 20 MB and is deliberately not persisted — \
                restore it from the source archive (PROJECT_CONTEXT.md §23ac)"))
            .clone();

        for (line, clip) in voice_lines {
            let info = dir.join(format!("{line}.sound_info"));
            assert!(info.is_file(), "missing {info:?} — the cue would silently do nothing");
            let body = std::fs::read_to_string(&info).expect("read sound_info");
            assert!(body.contains(clip), "{line}.sound_info must reference clip {clip}, got: {body}");
            let wav = dir.join(format!("{clip}.wav"));
            assert!(wav.is_file(), "missing clip asset {wav:?}");
            assert!(std::fs::metadata(&wav).expect("stat wav").len() > 1000, "{wav:?} looks empty");
        }
        // Lifecycle lines: the clip carries the mlbb_heroes_clip_ prefix,
        // because build_release_zip.py rejects a clip that shares the event's
        // own name — "TFM2 can fail to play same-ID event/clip routes".
        for line in LIFECYCLE_LINES {
            let info = dir.join(format!("{line}.sound_info"));
            let body = std::fs::read_to_string(&info).expect("read sound_info");
            let clip = body
                .split_once("\"clip\"")
                .and_then(|(_, value)| value.split('"').nth(1))
                .expect("voice clip asset ID");
            assert!(clip.starts_with("mlbb_heroes_clip_mlbb_belerick_voice_"), "{line} -> {clip}");
            assert!(dir.join(format!("{clip}.wav")).is_file(), "missing {clip}.wav");
        }
    }

    #[test]
    fn voice_cues_are_registered_in_audio_rs() {
        let src = include_str!("audio.rs");
        for (line, _clip) in voice_lines {
            assert!(src.contains(&format!("\"{line}\"")), "VOICE_CUES is missing {line}");
        }
        // The data refers to audio_voice_belerick_ult; without this suffix the
        // Native resolves to nothing and the ult line never plays.
        assert!(src.contains("\"belerick_ult\""), "VOICE_CUES is missing suffix belerick_ult");
        for line in LIFECYCLE_LINES {
            assert!(src.contains(&format!("\"{line}\"")), "VOICE_CUES is missing {line}");
        }
    }

    #[test]
    fn the_ult_cast_line_is_in_the_champion_data() {
        // The other half of the chain: audio.rs can register the cue all day,
        // but nothing fires it unless the ult's Combine asks for it.
        let data = include_str!("../champion/mlbb_belerick.data_champion");
        assert!(
            data.contains("mlbb_heroes:audio_voice_belerick_ult"),
            "the ult has no VOICE node — the cast line would never fire"
        );
    }

    // ---- visibility ----
    //
    // The owner playtested v2.0.7-rc.1 and could not see the ultimate or the
    // passive. Both were wired correctly; both were unreadable. These guard the
    // three causes so they cannot silently come back.

    /// Ticks per second the sim runs at (60 ticks = 1 s).
    const TICKS_PER_SEC: f64 = 60.0;

    /// Whitespace-stripped copy, so these assertions do not care whether a
    /// `.fanim` was written compact or indented — the roster ships both.
    fn collapsed(json: &str) -> String {
        json.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// Read a file from the build root, searching upward from the crate the way
    /// Arlott's sound test does. `include_str!` cannot take a runtime path, and
    /// a hard-coded relative path breaks the moment the crate moves.
    fn read_upward(rel: &str) -> String {
        use std::path::{Path, PathBuf};
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            let p: PathBuf = cur.join(rel);
            if p.is_file() {
                return std::fs::read_to_string(&p).expect("read asset");
            }
            if !cur.pop() {
                break;
            }
        }
        panic!("could not find {rel} in any parent of {}", env!("CARGO_MANIFEST_DIR"));
    }

    /// Total animation length in ticks, read from the asset's own `.fanim`, so
    /// the assertion can never drift from what is on disk. Every frame in a
    /// given anim carries the same duration, so count x first value is exact.
    fn anim_ticks(fanim: &str) -> u64 {
        let c = collapsed(fanim);
        let count = c.matches("\"duration\":").count();
        assert!(count > 0, "no frames in fanim");
        let after = c.split("\"duration\":").nth(1).expect("duration value");
        let num: String = after
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
            .collect();
        let d: f64 = num.parse().expect("duration parses");
        (d * count as f64 * TICKS_PER_SEC).round() as u64
    }

    #[test]
    fn the_thorn_sweep_lives_long_enough_to_be_seen() {
        // THORN_SPEED is per TICK, not per second. At 8,000 it crossed the whole
        // range in ~4 ticks while the animation runs 22, so only ~2 of 9 frames
        // ever rendered and the passive looked like it never fired. Gord's basic
        // bolt is the control: 20 ticks of flight against 22 ticks of animation.
        let flight = THORN_RANGE / THORN_SPEED;
        let anim = anim_ticks(&read_upward("effects/belerick_thorn_sweep#anim.fanim"));
        assert!(
            flight * 4 >= anim * 3,
            "sweep despawns after {flight} ticks but its animation needs {anim}; \
             most of it would never render. THORN_SPEED is per tick, not per second."
        );
    }

    #[test]
    fn the_ult_plays_a_caster_view_effect_not_a_bare_one() {
        // THE bug that hid the ultimate for two builds. The ult is
        // `casting_type: None`, so it takes no position input and a plain
        // `ViewEffect` resolves to the caster's own feet — and the engine does
        // not draw those. champion-data.md: "a `ViewEffect` on a point where
        // the caster himself stands did not show in game ... Play such a
        // picture as a `CasterViewEffect`".
        //
        // I first misdiagnosed this as z-order and moved z -1 -> 2. z was
        // never the cause; that change is harmless but this is the actual fix.
        let data = collapsed(&read_upward("champion/mlbb_belerick.data_champion"));
        let ult = data.find("\"ult\":").expect("no ult action");
        let next = data[ult + 6..]
            .find("\"passive\"")
            .expect("passive follows the actions");
        let body = &data[ult..ult + 6 + next];
        assert!(
            body.contains("\"type\":\"CasterViewEffect\",\"name\":\"belerick_dryad_wrath\""),
            "the ult must play dryad_wrath as a CasterViewEffect. A bare ViewEffect \
             resolves to Belerick's own feet with a `casting_type: None` cast, and the \
             engine does not draw it — the ult renders invisible."
        );
    }

    #[test]
    fn skill2_plays_enough_of_its_animation_to_be_seen() {
        // The owner: "i dont see his skill 2 animation sprite". The skill2 tag
        // is 13 frames / 0.55 s, but the CasterAnimation only held it for 6
        // ticks (0.1 s) — about one frame, which reads as no animation.
        //
        // 6-8 ticks is the roster's usual "tell" and is fine for a punchy
        // wind-up, but Belerick's skill2 motion is subtle (mean frame delta
        // 3.3 vs the ult's 9.7), so a short tell is invisible on him.
        let data = collapsed(&read_upward("champion/mlbb_belerick.data_champion"));
        let s2 = data.find("\"skill2\":").expect("no skill2 action");
        let ult = data[s2..].find("\"ult\":").expect("ult follows skill2") + s2;
        let body = &data[s2..ult];
        let at = body
            .find("\"type\":\"CasterAnimation\",\"name\":\"skill2\",\"tick\":")
            .expect("skill2 must force its sprite animation");
        let num: String = body[at + "\"type\":\"CasterAnimation\",\"name\":\"skill2\",\"tick\":".len()..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let tick: u64 = num.parse().expect("tick parses");
        assert!(
            tick >= 24,
            "skill2 holds its animation for only {tick} ticks (0.1 s of a 0.55 s \
             animation) — it reads as no animation at all. Raise it to ~30."
        );
    }

    #[test]
    fn the_taunt_marker_is_anchored_above_the_head() {
        // `view_buffs` carries no offset field anywhere in the roster, so a
        // marker can only be lifted above the head by giving the asset a TALL
        // canvas with anchor_y 1.0 and the art drawn near the top — which is
        // what arlott_mark, gusion_s1_mark and karrie's head marks all do.
        let c = collapsed(&read_upward("effects/belerick_taunt_marker#anim.fanim"));
        assert!(
            c.contains("\"anchor_y\":1.0"),
            "taunt marker must be bottom-anchored (anchor_y 1.0) to sit above the head; \
             anchor_y 0.5 centres it on his chest"
        );
        // Tall canvas: read the h of the first frame.
        let at = c.find("\"h\":").expect("frame height");
        let h: String = c[at + 4..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        let h: u64 = h.parse().expect("h parses");
        assert!(
            h > 96,
            "canvas is only {h} tall; a head marker needs a tall canvas so the art \
             can sit in the top of it (arlott_mark uses 142)"
        );
    }
}
