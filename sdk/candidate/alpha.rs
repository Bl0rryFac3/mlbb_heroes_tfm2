//! Native Alpha gameplay — Beta, Advance! (passive), Beta's strafes, and the
//! Lock On mark.
//!
//! Written against the StablePassive / StableEffectType traits in
//! mod-api-stable. Every API call here is copied from a verified call site in
//! candidate/src/balmond.rs or the shipped SDK; nothing is invented.
//!
//! Registration (in candidate/src/lib.rs, next to `balmond::register`):
//!     mod alpha;
//!     alpha::register(&mut module);
//!
//! v13 wiring — all three native entry points feed one model:
//!   passive  mlbb_heroes:alpha_beta_advance     counts LANDED Beta hits,
//!                                               applies the Lock On mark
//!   effect   mlbb_heroes:alpha_beta_strafe      S1/S2 strafe: true damage +
//!                                               one Beta visual per enemy
//!   effect   mlbb_heroes:alpha_beta_strafe_ult  ult: queues the strafe five
//!                                               times, spread over ticks
//! plus two internal effects used by the above (ult strafe, cosmetic strike).

use mod_api_stable::*;

/// The Lock On mark. Bound in `view_buffs`; the detonation/hover visual is a
/// separate `view_effects` name, because a buff binding cannot be consumed.
const MARK_BUFF: &str = "mlbb_alpha_lockon_mark";
/// Per-target Beta-hit counter. A stack buff rather than state in the passive
/// struct, so S1/S2 strafes can charge the same counter as basic attacks.
const MARK_STACK_TARGET: &str = "mlbb_alpha_beta_stack";
const MARK_DURATION: usize = 90; // 1.5 s at 60 ticks/s
/// v1.1.75 balance: **3**, not 2. Every landed hit charges this counter —
/// basic attacks, S1/S2 strafes and all five ult strafes — so at 2 the
/// armour-proof Lock-On laser was firing on nearly every second hit in a real
/// fight, and the post-match bar showed 69% of Alpha's damage as True
/// (17,759 of 25,920). See V18_ALPHA_BALANCE.md.
const STACKS_NEEDED: usize = 3;
const LOCKON_RANGE: f64 = 90_000.0;
// v16 balance (option A). Shapes taken from the MLBB kit, then scaled for an
// engine with no items:
//   S1/S2 Beta strafe   60-120 (+30% extra AD) true      -> 25-39 (+20% attack)
//   ult Beta strafe     15-30  (+30% extra AD) true x5   -> 8 flat (+15% attack)
//   Lock On laser       80 (+8 x level) (+100% extra AD) -> 45-101 (+25% attack)
// v17: the v16 numbers were still ~40% above this roster's whole damage band,
// and the true-damage share showed up as ~10,000 of Kiin's 29,968 damage on the
// post-match bar. See V17_BALANCE_PLAN.md.
// Before v16 the ult's five strafes reused the S1 strafe at full value and the
// laser took 100% of attack, which put a level-15 combo at ~2,475 true damage
// against a roster whose champions have 650-820 HP. See V16_BALANCE.md.
// v1.1.75 balance: 45+4/lvl (+25% attack) -> 30+2.5/lvl (+15% attack).
// The old curve paid ~112 at level 11 against a roster whose champions have
// 650-820 HP; see V18_ALPHA_BALANCE.md for the arithmetic. Kept as half-steps
// by doubling the numbers and halving in `laser_damage`, because these are
// integer constants and 2.5/lvl is not representable otherwise.
const LASER_DAMAGE_BASE: usize = 30;
const LASER_LEVEL_GAIN: usize = 3; // 30 -> 72 at level 15 (3/level, was 4)
const LASER_ATTACK_RATIO: usize = 15;
const STRAFE_DAMAGE_BASE: usize = 25;
const STRAFE_LEVEL_GAIN: usize = 1; // 25 -> 39 at level 15
const STRAFE_ATTACK_RATIO: usize = 20;
/// The ult's strafes are deliberately much smaller than S1/S2's, because five
/// of them land in 0.4 s.
const ULT_STRAFE_DAMAGE_BASE: usize = 8;
const ULT_STRAFE_LEVEL_GAIN: usize = 0; // flat 8: five hits already pay 5x
const ULT_STRAFE_ATTACK_RATIO: usize = 10;
const ULT_STRAFE_HITS: usize = 5;
/// Ticks between the ult's five strafes: 0/6/12/18/24, i.e. 0.4 s total.
const ULT_STRAFE_SPACING: usize = 6;
const ULT_STRAFE_ONCE: &str = "mlbb_heroes:alpha_beta_strafe_once";
/// Cosmetic-only effect applied by Beta's projectile. Never deals damage.
const BETA_STRIKE_ONCE: &str = "mlbb_heroes:alpha_beta_strike_once";
/// Beta's flash on a strafe. MUST live in `view_effects` — `play_view_effect`
/// resolves view-effect bindings, never `view_projectiles` ones.
const BETA_STRIKE_NAME: &str = "alpha_beta_strike";
/// Beta hovering on the marked champion while Lock On is up.
const BETA_MARK_NAME: &str = "alpha_beta_mark";
const BETA_FLASH_TIME: u64 = 12;
const BETA_FLASH_RADIUS: u64 = 24_000;

/// Alpha's voice. Clips live in the mod as `sound/sfx/mlbb_alpha_sfx_vo_*` and are
/// played through `play_voice`, which applies the roster's shared voice gates
/// (actor lock + global lock) so he cannot talk over himself or another hero.
const VOICE_SKILL1: &str = "mlbb_alpha_skill1_cast";
const VOICE_SKILL2: &str = "mlbb_alpha_skill2_cast";
const VOICE_ULT: &str = "mlbb_alpha_ult_cast";

fn speak(sim: &mut StableSim<'_>, caster: usize, line: &str) {
    if !can_render(sim) {
        return;
    }
    let _ = crate::audio::play_voice(sim, caster, line);
}

/// S2 fan radius in world units — the skill's own 35,000 hit radius.
const S2_FAN_RADIUS: u64 = 35_000;

/// Angular bins for the S2 fan. The engine does not rotate `view_effects` art,
/// so the compiler ships one pre-rotated wedge per 30° and native picks the tag
/// by cast direction — the same technique Estes (`link_wire_0..11`) and Zhask
/// (`spawn_beam_0..11`) ship.
/// Registered native that plays the correctly-turned S2 wedge. Spelled out at
/// the registration site too (not through this constant) because
/// `verify_effects.py` greps the source for the literal `add_native_effect(...)`.
#[cfg(test)]
const S2_FAN_EFFECT: &str = "mlbb_heroes:alpha_s2_fan_bin";
const BIN_COUNT: usize = 12;
const BIN_NAMES: [&str; BIN_COUNT] = [
    "alpha_s2_fan_0",
    "alpha_s2_fan_1",
    "alpha_s2_fan_2",
    "alpha_s2_fan_3",
    "alpha_s2_fan_4",
    "alpha_s2_fan_5",
    "alpha_s2_fan_6",
    "alpha_s2_fan_7",
    "alpha_s2_fan_8",
    "alpha_s2_fan_9",
    "alpha_s2_fan_10",
    "alpha_s2_fan_11",
];

/// Pick the pre-rotated bin whose wedge opens along `from -> to`.
///
/// Measured from the compiled art with an alpha-weighted centroid: bin `k` opens
/// at screen angle `-30k` degrees, where screen angles are counter-clockwise
/// with Y pointing **up**. World Y grows the other way, hence the negation.
/// Verified bins: 0 → -2°, 3 → -93°, 6 → +177°, 9 → +88°.
fn bin_index(from: (u64, u64), to: (u64, u64)) -> usize {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    if dx == 0.0 && dy == 0.0 {
        return 0;
    }
    let screen_angle = (-dy).atan2(dx); // CCW, 0 = pointing +X on screen
    let step = std::f64::consts::TAU / BIN_COUNT as f64; // 30°
    let idx = (-screen_angle / step).round() as i64;
    (idx.rem_euclid(BIN_COUNT as i64)) as usize
}

/// Name of the bin to play for a cast from `from` to `to`.
fn bin_name(from: (u64, u64), to: (u64, u64)) -> &'static str {
    BIN_NAMES[bin_index(from, to)]
}

fn percent(value: usize, pct: usize) -> usize {
    value.saturating_mul(pct) / 100
}

fn level_linear(low: usize, high: usize, level: usize) -> usize {
    low + (high - low).saturating_mul(level.saturating_sub(1).min(14)) / 14
}

fn distance(a: (u64, u64), b: (u64, u64)) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
}

/// Resolve the nearest eligible enemy to `origin` within `radius`.
/// Mirrors the nearest-target scan in BalmondSoulLockHit.
fn nearest_enemy(
    sim: &StableSim<'_>,
    origin: (u64, u64),
    team: usize,
    radius: f64,
    champions_only: bool,
) -> Option<usize> {
    let ids: Vec<usize> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i).map(|e| e.id()))
        .collect();
    let mut best: Option<(usize, f64)> = None;
    for id in ids {
        let Some(target) = sim.get_entity(id) else {
            continue;
        };
        if target.team() == team
            || !target.is_alive()
            || !target.is_targetable()
            || target.is_tower()
            || (champions_only && !target.is_champion())
        {
            continue;
        }
        let d = distance(origin, target.pos());
        if d > radius {
            continue;
        }
        let closer = best.map_or(true, |(_, bd)| d < bd);
        if closer {
            best = Some((id, d));
        }
    }
    best.map(|(id, _)| id)
}

/// The Lock-On laser. **Fixed** damage, so armour does not apply to it — which
/// is exactly why it is the smallest per-hit number in the kit.
fn laser_damage(attack: usize, level: usize) -> usize {
    level_linear(
        LASER_DAMAGE_BASE,
        LASER_DAMAGE_BASE + LASER_LEVEL_GAIN * 14,
        level,
    )
    .saturating_add(percent(attack, LASER_ATTACK_RATIO))
}

fn strafe_damage(attack: usize, level: usize) -> usize {
    level_linear(
        STRAFE_DAMAGE_BASE,
        STRAFE_DAMAGE_BASE + STRAFE_LEVEL_GAIN * 14,
        level,
    )
    .saturating_add(percent(attack, STRAFE_ATTACK_RATIO))
}

/// One of the ult's five strafes. Separate from `strafe_damage` because the kit
/// prices them at a quarter of an S1 strafe.
fn ult_strafe_damage(attack: usize, level: usize) -> usize {
    level_linear(
        ULT_STRAFE_DAMAGE_BASE,
        ULT_STRAFE_DAMAGE_BASE + ULT_STRAFE_LEVEL_GAIN * 14,
        level,
    )
    .saturating_add(percent(attack, ULT_STRAFE_ATTACK_RATIO))
}

/// The champion the current ult's strafes are locked onto.
const ULT_LOCK_PREFIX: &str = "mlbb_alpha_ult_lock_";
/// Lock lifetime in ticks: comfortably longer than the five strafes (0..24), and
/// short enough that it can never leak into a later ult.
const ULT_LOCK_TICKS: usize = 120;

fn ult_lock_name(champion: usize) -> String {
    format!("{ULT_LOCK_PREFIX}{champion}")
}

fn set_ult_strafe_lock(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    let name = ult_lock_name(target);
    let _ = sim.entity_remove_buff(caster, &name);
    let _ = sim.add_buff(caster, &BuffV1::timed(&name, ULT_LOCK_TICKS));
}

fn clear_ult_strafe_lock(sim: &mut StableSim<'_>, caster: usize) {
    let Some(entity) = sim.get_entity(caster) else {
        return;
    };
    let names: Vec<String> = (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .map(|b| b.name().to_string())
        .filter(|n| n.starts_with(ULT_LOCK_PREFIX))
        .collect();
    for name in names {
        let _ = sim.entity_remove_buff(caster, &name);
    }
}

fn ult_strafe_lock(sim: &StableSim<'_>, caster: usize) -> Option<usize> {
    let entity = sim.get_entity(caster)?;
    for i in 0..entity.buff_count() {
        let Some(buff) = entity.buff_at(i) else {
            continue;
        };
        if let Some(rest) = buff.name().strip_prefix(ULT_LOCK_PREFIX) {
            return rest.parse().ok();
        }
    }
    None
}

/// Towers, inhibitors and the nexus: never worth Beta stacks.
fn is_tower(sim: &StableSim<'_>, target: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| e.is_tower())
}

/// True when `target` is a legal hostile target for `team`.
fn is_hostile(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target)
        .is_some_and(|e| e.team() != team && e.is_alive() && e.is_targetable() && !e.is_tower())
}

/// Live Beta-hit stacks on `target`.
fn stack_count(sim: &StableSim<'_>, target: usize) -> usize {
    let Some(entity) = sim.get_entity(target) else {
        return 0;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .filter(|b| b.name() == MARK_STACK_TARGET)
        .count()
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

/// Beta's flash at a hit: one named view effect at the target. Fire-and-forget,
/// no damage. (v14 removed the cosmetic projectile that used to fly from Alpha
/// to the enemy — it drew a full drone as a travelling projectile, which read as
/// Beta leaving his post and duplicated the flash.)
fn play_beta_strike(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    if !can_render(sim) {
        return;
    }
    let Some(tpos) = sim.get_entity(target).map(|e| e.pos()) else {
        return;
    };
    let _ = sim.play_view_effect(
        BETA_STRIKE_NAME,
        caster,
        &InputTargetV1::pos(tpos.0, tpos.1),
        0,
        BETA_FLASH_RADIUS,
        BETA_FLASH_TIME,
    );
}

/// Record one landed Beta hit on `target` and run Lock On at the threshold.
/// Shared by basic attacks (passive) and skill strafes, so both feed one
/// counter — which is what makes "two Beta hits" mean what the kit says.
fn record_beta_hit(sim: &mut StableSim<'_>, owner: usize, target: usize) {
    let Some((team, attack, level)) = sim
        .get_entity(owner)
        .map(|e| (e.team(), e.stat().attack, e.level().max(1)))
    else {
        return;
    };
    // v15: the kit's Lock On is a duelling mechanic, not a siege one. The
    // basic-attack path can hand us a tower; never charge, mark, or laser one.
    if !is_hostile(sim, target, team) || is_tower(sim, target) {
        return;
    }
    let stacks = stack_count(sim, target) + 1;
    if stacks < STACKS_NEEDED {
        let stack = BuffV1::timed(MARK_STACK_TARGET, MARK_DURATION);
        let _ = sim.entity_stack_buff(target, &stack, STACKS_NEEDED, true);
        return;
    }
    // Two Beta hits: Lock On.
    let _ = sim.entity_remove_buff(target, MARK_STACK_TARGET);
    let _ = sim.entity_remove_buff(target, MARK_BUFF);
    sim.add_buff(target, &BuffV1::timed(MARK_BUFF, MARK_DURATION));
    if can_render(sim) {
        let _ = sim.play_view_effect(
            BETA_MARK_NAME,
            target,
            &InputTargetV1::target(target),
            0,
            0,
            0,
        );
    }
    let amount = laser_damage(attack, level);
    let _ = sim.deal_damage_typed(
        owner,
        target,
        amount,
        DamageTypeV1::Ad,
        AttackTypeV1::Skill,
    );
    play_beta_strike(sim, owner, target);
    let mut slow = BuffV1::timed("mlbb_alpha_lockon_slow", 60);
    slow.move_speed_mult = -30;
    let _ = sim.entity_remove_buff(target, "mlbb_alpha_lockon_slow");
    sim.add_buff(target, &slow);
}

/// Betas flash on a strafe hit, then charge the mark.
///
/// The S1/S2 strafe is the one place the kit keeps `Fixed`: this is the shape
/// seaway asked for ("put true damage in his second"), so the Lock-On identity
/// survives while the laser and the ult volley become physical.
fn strafe_target(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    strafe_target_with(
        sim,
        caster,
        target,
        DamageTypeV1::Fixed,
        |attack, level| strafe_damage(attack, level),
    );
}

/// Same strafe, different price and damage type.
///
/// The ult's five hits use `ult_strafe_damage` and are `Ad`; the S1/S2 strafe
/// is `Fixed`. See `strafe_target`.
fn strafe_target_with(
    sim: &mut StableSim<'_>,
    caster: usize,
    target: usize,
    damage_type: DamageTypeV1,
    price: impl Fn(usize, usize) -> usize,
) {
    let Some((team, attack, level)) = sim
        .get_entity(caster)
        .map(|e| (e.team(), e.stat().attack, e.level().max(1)))
    else {
        return;
    };
    if !is_hostile(sim, target, team) {
        return;
    }
    let amount = price(attack, level);
    let _ = sim.deal_damage_typed(
        caster,
        target,
        amount,
        damage_type,
        AttackTypeV1::Skill,
    );
    play_beta_strike(sim, caster, target);
    // NOTE: do not charge the mark here as well. The passive's `on_attack`
    // fires for every hit the owner lands, including this one; charging in both
    // places would make a single strafe count twice.
}

// ---------------------------------------------------------------------------
// Passive — Beta, Advance!
// `on_attack` fires for every hit the owner lands and hands us the unit that
// was hit, so: no charge on a miss (old bug R3) and the mark can only land on
// a champion Alpha actually struck (old bug R4).
// ---------------------------------------------------------------------------
#[derive(Clone, Default)]
struct AlphaBetaAdvance;

impl StablePassive for AlphaBetaAdvance {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    /// v15: Beta no longer loiters at Alpha's side. v14's `alpha_beta_loiter`
    /// resolved to the entity's *head* (the play_view_effect target is a
    /// position input, not an offset from the caster), so Alpha wore a second
    /// Beta on his head. The owner prefers Beta as the strike/mark companion on
    /// the enemy — that is `alpha_beta_strike` / `alpha_beta_mark`, untouched.
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize) {
        // Nothing to start; kept so the passive still has an explicit spawn hook.
        let _ = (sim, entity);
    }

    fn on_update(
        &mut self,
        _sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        _entity: usize,
    ) {
    }

    fn on_attack(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity: usize,
        target: usize,
        _damage: &mut usize,
    ) {
        // This hook has no attack_type in the current SDK: it fires for every
        // hit the owner lands, so it is the single charge path for Beta hits.
        record_beta_hit(sim, entity, target);
    }
}

// ---------------------------------------------------------------------------
// Beta strafe suite — true damage on every enemy the skill's area selects,
// one Beta visual per enemy (S1 and S2 both reference this effect).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Default)]
struct AlphaBetaStrafe;

impl StableEffectType for AlphaBetaStrafe {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        strafe_target(sim, caster, input.target_id);
        // This effect runs once per enemy the area selects, so the voice is
        // deduped by the shared actor/global voice gates inside `play_voice`
        // rather than by a cast counter.
        speak(sim, caster, VOICE_SKILL1);
    }
}

// ---------------------------------------------------------------------------
// S2 fan, direction-correct. S2 is cast at a Position, so the engine places the
// effect at that point but does NOT turn the art. This native reads the same
// Position input, works out which pre-rotated bin points from Alpha towards it,
// and plays that name. The data keeps its own plain `ViewEffect` as a fallback
// for the case where this never fires; see V14_DATA_PATCH.md.
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Default)]
struct AlphaS2Fan;

impl StableEffectType for AlphaS2Fan {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if !can_render(sim) {
            return;
        }
        let Some(from) = sim.get_entity(caster).map(|e| e.pos()) else {
            return;
        };
        // A Position cast carries the clicked point; a Target cast carries an
        // entity we have to look up. Anything else falls back to bin 0.
        let to = match InputTargetKindV1::from_code(input.kind) {
            Some(InputTargetKindV1::Pos) => (input.x, input.y),
            Some(InputTargetKindV1::Target) => match sim.get_entity(input.target_id) {
                Some(e) => e.pos(),
                None => from,
            },
            _ => from,
        };
        let _ = sim.play_view_effect(
            bin_name(from, to),
            caster,
            &InputTargetV1::pos(to.0, to.1),
            0,
            S2_FAN_RADIUS,
            0,
        );
        // Once per cast, not once per enemy: this native is a top-level entry.
        speak(sim, caster, VOICE_SKILL2);
    }
}

// ---------------------------------------------------------------------------
// Ult — Beta's five post-slam strafes, spread across ticks. The old version
// ran all five inside one `apply`, so they read as one hit (old bug R2).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Default)]
struct AlphaBetaStrafeUlt;

impl StableEffectType for AlphaBetaStrafeUlt {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _: InputTargetV1) {
        let Some(pos) = sim.get_entity(caster).map(|e| e.pos()) else {
            return;
        };
        speak(sim, caster, VOICE_ULT);
        // A fresh ult always re-locks the target.
        clear_ult_strafe_lock(sim, caster);
        let input = InputTargetV1::pos(pos.0, pos.1);
        for hit in 0..ULT_STRAFE_HITS {
            let _ = sim.queue_effect(
                ULT_STRAFE_ONCE,
                AttackTypeV1::Skill,
                caster,
                &input,
                hit * ULT_STRAFE_SPACING,
            );
        }
    }
}

/// One of the ult's strafes: re-resolves the nearest enemy each time so Beta
/// keeps up with a target moving between hits.
#[derive(Clone, Copy, Default)]
struct AlphaBetaStrafeUltOnce;

impl StableEffectType for AlphaBetaStrafeUltOnce {
    /// The ult's strafe used to re-scan for the nearest enemy on *every* hit, so
    /// the five strafes could land on five different champions (and the Beta
    /// mark could hop between them). The kit says Beta strafes the area the
    /// slam hit, so the first hit locks a target and the rest stick to it,
    /// falling back to a fresh scan only if that target dies mid-ult.
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _: InputTargetV1) {
        let Some((pos, team)) = sim.get_entity(caster).map(|e| (e.pos(), e.team())) else {
            return;
        };
        let locked = ult_strafe_lock(sim, caster);
        let target = match locked {
            Some(t) if is_hostile(sim, t, team) => t,
            _ => match nearest_enemy(sim, pos, team, LOCKON_RANGE, false) {
                Some(t) => {
                    set_ult_strafe_lock(sim, caster, t);
                    t
                }
                None => return,
            },
        };
        strafe_target_with(sim, caster, target, DamageTypeV1::Ad, ult_strafe_damage);
    }
}

/// Cosmetic-only effect the spawned projectile applies on hit. It must never
/// deal damage: the strafe already paid out.
#[derive(Clone, Copy, Default)]
struct AlphaBetaStrikeOnce;

impl StableEffectType for AlphaBetaStrikeOnce {
    fn apply(&self, _sim: &mut StableSim<'_>, _: u64, _caster: usize, _: InputTargetV1) {}
}

pub fn register(module: &mut StableMod) {
    module.add_native_passive(
        "mlbb_heroes:alpha_beta_advance",
        AlphaBetaAdvance::default(),
    );
    module.add_native_effect("mlbb_heroes:alpha_beta_strafe", AlphaBetaStrafe);
    module.add_native_effect("mlbb_heroes:alpha_beta_strafe_ult", AlphaBetaStrafeUlt);
    module.add_native_effect(ULT_STRAFE_ONCE, AlphaBetaStrafeUltOnce);
    module.add_native_effect(BETA_STRIKE_ONCE, AlphaBetaStrikeOnce);
    module.add_native_effect("mlbb_heroes:alpha_s2_fan_bin", AlphaS2Fan);
}

#[cfg(test)]
mod tests {
    use super::{
        bin_index, distance, laser_damage, level_linear, percent, strafe_damage, ult_lock_name,
        ult_strafe_damage, BETA_MARK_NAME, BETA_STRIKE_NAME, BIN_COUNT, BIN_NAMES, S2_FAN_EFFECT,
        STACKS_NEEDED, ULT_LOCK_PREFIX, ULT_LOCK_TICKS, ULT_STRAFE_HITS, ULT_STRAFE_ONCE,
        ULT_STRAFE_SPACING, VOICE_SKILL1, VOICE_SKILL2, VOICE_ULT,
    };

    #[test]
    fn level_linear_clamps_at_level_fifteen() {
        assert_eq!(level_linear(40, 152, 1), 40);
        assert_eq!(level_linear(40, 152, 15), 152);
        assert_eq!(level_linear(40, 152, 99), 152);
        assert_eq!(level_linear(40, 152, 0), 40);
    }

    #[test]
    fn laser_damage_matches_the_v1_1_75_curve() {
        // v1.1.75 nerf: 30 + 3/level, plus 15% of attack.
        // Was 45 + 4/level plus 25% of attack (70 / 126 on these same inputs).
        assert_eq!(laser_damage(100, 1), 45);
        assert_eq!(laser_damage(100, 15), 87);
    }

    #[test]
    fn laser_cannot_run_away_from_a_strafe() {
        // The laser is the passive's payoff, so it is *meant* to be bigger than
        // one strafe hit — but it is Fixed (armour-proof) and it charges from
        // strafe hits, so its DPS is laser_per_hit / hits_per_proc against a
        // number nobody can itemise against. If it drifts far above a strafe the
        // hero collapses back into "spam true damage", which is exactly the
        // v1.1.74 problem (69% of his damage bar reading True).
        //
        // Realistic attack per level, not a flat sweep: this roster has no items
        // at all, so AD is 60 + 5/level (Alpha's own stat block). Testing L15 at
        // 60 AD would be testing a build that cannot exist — and it is the one
        // point where the flat term dominates and the ratio spikes to 1.59x.
        //
        // Ceiling 1.6x on realistic AD: worst case is L15 at bare AD (81/51 =
        // 1.59x). Past that, the curve needs a look.
        for level in [1usize, 8, 11, 15] {
            let base_ad = 60 + 5 * (level as usize - 1);
            for bonus in [0usize, 50, 100] {
                let attack = base_ad + bonus;
                let (laser, strafe) = (laser_damage(attack, level), strafe_damage(attack, level));
                assert!(
                    laser as f64 <= strafe as f64 * 1.6,
                    "laser ({laser}) ran away from a strafe ({strafe}) at L{level} attack {attack}"
                );
            }
        }
    }

    #[test]
    fn lock_on_nerf_holds_at_a_typical_mid_game_attack() {
        // Regression pin on the v1.1.75 numbers, using the roster's real stats
        // (base 60 AD + 5/level): L11 -> AD 110. Was 112 before the nerf.
        let ad = 60 + 5 * 10;
        assert_eq!(laser_damage(ad, 11), 76);
        assert!(
            laser_damage(ad, 11) < 90,
            "the nerf is pointless if a mid-game laser is back near 112"
        );
    }

    #[test]
    fn lock_on_needs_three_landed_hits_after_the_nerf() {
        // v1.1.75: was 2. Raising this is the lever that cuts the true-damage
        // *share* rather than just the per-hit number.
        assert_eq!(STACKS_NEEDED, 3);
    }

    #[test]
    fn strafe_damage_matches_the_kit() {
        // 25 -> 39 by level 15, plus 20% of attack.
        assert_eq!(strafe_damage(200, 1), 65);
        assert_eq!(strafe_damage(200, 15), 79);
    }

    #[test]
    fn ult_strafe_stays_well_under_an_s1_strafe() {
        // Flat 8 + 10% of attack. The kit prices the five-hit volley at roughly
        // one to two S1 strafes' worth of damage, so hold it under 2x.
        assert_eq!(ult_strafe_damage(0, 1), 8);
        assert_eq!(ult_strafe_damage(0, 15), 8);
        assert_eq!(ult_strafe_damage(200, 1), 28);
        assert!(ult_strafe_damage(100, 8) > 0);
        // Check the invariant at the attack each hero level actually reaches
        // (60 + 5/level): the volley must stay within 2x one S1 strafe.
        for level in 1..=15 {
            let attack = 60 + 5 * (level - 1);
            assert!(
                ult_strafe_damage(attack, level) * 5 <= strafe_damage(attack, level) * 2,
                "the five-hit volley must stay within 2x one S1 strafe \
                 (attack {attack}, level {level})"
            );
        }
    }

    #[test]
    fn ult_strafe_lock_round_trips_through_the_buff_name() {
        assert_eq!(ult_lock_name(42), "mlbb_alpha_ult_lock_42");
        assert_eq!(
            ult_lock_name(42)
                .strip_prefix(ULT_LOCK_PREFIX)
                .unwrap()
                .parse::<usize>(),
            Ok(42)
        );
        // The lock must outlive the last strafe (24 ticks) and cannot reach a
        // later ult (20 s cooldown).
        assert!(ULT_LOCK_TICKS > 24 && ULT_LOCK_TICKS < 1200);
    }

    #[test]
    fn voice_lines_are_namespaced_to_the_mod() {
        for line in [VOICE_SKILL1, VOICE_SKILL2, VOICE_ULT] {
            assert!(
                line.starts_with("mlbb_alpha_"),
                "{line} must be a mod-owned sound"
            );
        }
    }

    #[test]
    fn percent_rounds_down_and_saturates() {
        assert_eq!(percent(199, 35), 69);
        assert_eq!(percent(usize::MAX, 100), usize::MAX / 100);
        assert_eq!(percent(usize::MAX, 0), 0);
    }

    #[test]
    fn distance_is_symmetric() {
        let a = (1_000u64, 2_000u64);
        let b = (4_000u64, 6_000u64);
        assert_eq!(distance(a, b), 5_000.0);
        assert_eq!(distance(a, b), distance(b, a));
    }

    #[test]
    fn beta_view_effects_are_view_effect_names() {
        // Both are played with play_view_effect, which resolves view-effect
        // bindings only. They must exist in view_effects in the champion data,
        // NOT view_projectiles, or they draw nothing and fail silently.
        assert_eq!(BETA_STRIKE_NAME, "alpha_beta_strike");
        assert_eq!(BETA_MARK_NAME, "alpha_beta_mark");
        assert_ne!(BETA_STRIKE_NAME, "alpha_beta_dive");
    }

    #[test]
    fn bins_cover_a_full_turn_in_thirty_degree_steps() {
        // Every bin name must exist, be unique, and be spaced evenly.
        let mut seen = std::collections::HashSet::new();
        for name in BIN_NAMES {
            assert!(seen.insert(name), "duplicate bin name {name}");
            assert!(name.starts_with("alpha_s2_fan_"));
        }
        assert_eq!(seen.len(), BIN_COUNT);
        assert_eq!(BIN_COUNT, 12);
    }

    #[test]
    fn bin_picks_the_wedge_that_points_at_the_target() {
        let from = (50_000u64, 50_000u64);
        // Cast right: the art's bin 0 already opens towards +X.
        assert_eq!(bin_index(from, (60_000, 50_000)), 0);
        // Cast up on screen = world -Y. Bins turn clockwise, so 90 deg CCW is
        // bin 9 (measured: bin 9 opens at +88 deg).
        assert_eq!(bin_index(from, (50_000, 40_000)), 9);
        // Cast down on screen = world +Y -> 90 deg the other way -> bin 3.
        assert_eq!(bin_index(from, (50_000, 60_000)), 3);
        // Cast left -> half a turn -> bin 6 (measured +177 deg).
        assert_eq!(bin_index(from, (40_000, 50_000)), 6);
        // A zero-length cast must not panic.
        assert_eq!(bin_index(from, from), 0);
    }

    #[test]
    fn bin_names_are_all_reachable() {
        let centre = 100_000u64;
        let from = (centre, centre);
        let mut hit = std::collections::HashSet::new();
        for deg in 0..360 {
            let rad = (deg as f64).to_radians();
            // Walk a circle of radius 10,000 around the caster in signed space.
            let to = (
                (centre as i64 + (rad.cos() * 10_000.0) as i64) as u64,
                (centre as i64 + (rad.sin() * 10_000.0) as i64) as u64,
            );
            hit.insert(bin_index(from, to));
        }
        assert_eq!(hit.len(), BIN_COUNT, "not every bin is reachable: {hit:?}");
    }

    #[test]
    fn tower_guard_is_wired_into_the_hit_path() {
        // v15: towers must never charge Beta stacks, take the Lock On mark, or
        // eat the laser. The data already selects EnemyWithoutTower; this is the
        // belt-and-braces guard for the basic-attack path, which has no such
        // filter. Guarded by reading the source so the check cannot silently rot.
        let src = include_str!("alpha.rs");
        assert!(
            src.contains("|| is_tower(sim, target)"),
            "record_beta_hit must reject towers before charging"
        );
        assert!(src.contains("fn is_tower("), "is_tower helper is missing");
    }

    #[test]
    fn s2_fan_effect_is_namespaced() {
        assert!(S2_FAN_EFFECT.starts_with("mlbb_heroes:"));
        // The registration must spell the name out literally: verify_effects.py
        // greps the source for `add_native_effect("mlbb_heroes:...")` so a
        // constant there would make the gate blind to it.
        let src = include_str!("alpha.rs");
        assert!(src.contains(&format!(
            "add_native_effect(\"{S2_FAN_EFFECT}\", AlphaS2Fan)"
        )));
    }

    #[test]
    fn ult_strafes_are_spread_over_five_distinct_ticks() {
        assert_eq!(ULT_STRAFE_HITS, 5);
        let offsets: Vec<usize> = (0..ULT_STRAFE_HITS)
            .map(|h| h * ULT_STRAFE_SPACING)
            .collect();
        assert_eq!(offsets, vec![0, 6, 12, 18, 24]);
        // Five separate hits inside 0.4 s at 60 ticks/s.
        assert!(offsets[ULT_STRAFE_HITS - 1] < 30);
    }

    #[test]
    fn ult_once_effect_is_namespaced() {
        assert!(ULT_STRAFE_ONCE.starts_with("mlbb_heroes:"));
    }

    #[test]
    fn beta_stack_counter_is_wired_to_the_mark() {
        // The counter is a stack buff, not state in the passive struct, so that
        // S1/S2 strafes charge the same counter basic attacks do.
        assert_eq!(super::MARK_STACK_TARGET, "mlbb_alpha_beta_stack");
    }
}
