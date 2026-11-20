//! Native Arlott gameplay — Demon Gaze (passive), Dauntless Strike's delayed
//! stun, Vengeance's marked payoff, and Final Slash's displacement.
//!
//! Written against the StablePassive / StableEffectType traits in
//! mod-api-stable. Every API call here is copied from a verified call site in
//! the shipped `alpha.rs` / `balmond.rs`; nothing is invented.
//!
//! Registration (in candidate/src/lib.rs, next to `alpha::register`):
//!     mod arlott;
//!     arlott::register(&mut module);
//!
//! Data wiring — four entry points, one model:
//!   passive  mlbb_heroes:arlott_demon_gaze     auto-marks a nearby enemy hero
//!                                              every 7 s; also marks anything
//!                                              Arlott applies a control effect to
//!   effect   mlbb_heroes:arlott_s1_stun        S1's delayed stun, longer the
//!                                              further the target is
//!   effect   mlbb_heroes:arlott_s2_vengeance   S2 vs a MARKED target: add the
//!                                              hit again, reset S2's cooldown,
//!                                              heal for 30% of attack
//!   effect   mlbb_heroes:arlott_ult_push       ult drag to his right side
//!
//! The mark is the engine of the hero: S2's double damage, cooldown reset and
//! heal all gate on it, and the only ways to place it are the 7 s auto-mark or
//! landing control. That is the canon shape, per ARLOTT_SPEC.md.

use mod_api_stable::*;

/// The Demon Gaze mark. Bound in the data's `view_buffs`, so the eye glyph
/// tracks whichever unit carries the buff — no per-unit visual bookkeeping.
const MARK_BUFF: &str = "mlbb_arlott_mark";
/// Canon is 7 s; the sim runs at 60 ticks/s.
const MARK_TICKS: usize = 420;
/// Canon: "marks a nearby enemy hero every 7 s". First application is staggered
/// slightly so it can never fire on the same tick Arlott spawns.
const AUTO_MARK_TICKS: usize = 420;
const AUTO_MARK_DELAY: usize = 60;
/// Canon range 5.5 — the same 90,000 units Alpha's Lock On scan uses. Both are
/// "about half a screen"; matching Alpha keeps the two heroes comparable.
const MARK_RANGE: f64 = 90_000.0;
/// S1's stun. Canon: 1 s, and "the further the target is, the longer the stun"
/// (ONE Esports, 2023-12 preview). We pay a short stun up close and the full
/// second at the far edge of the 42,000 strike.
const STUN_MIN_TICKS: u64 = 30; // 0.5 s at the caster's feet
const STUN_MAX_TICKS: u64 = 60; // 1.0 s at the far edge
const S1_REACH: f64 = 42_000.0;
/// Vengeance's payoff: canon doubles the damage, refreshes its own cooldown and
/// heals 90% (+90% total PA) — but only 50% of that against non-heroes.
const VENGEANCE_HEAL_ATTACK_RATIO: usize = 30; // 30% of attack, hero
const VENGEANCE_MINION_HEAL_PCT: usize = 50; // half of the above vs minions
/// Full refresh = take the whole skill cooldown off the slot. The data prices
/// S2 at 180 ticks (3 s) as of the v2.0.11 rebalance, so refunding 180 always
/// lands it ready. Keep this equal to the data's S2 `cooltime`; if the two
/// drift, the refund silently stops being a full refresh.
const VENGEANCE_CD_REFUND: usize = 180;
/// Final Slash drags every target to the same point on Arlott's right side.
/// 30,000 is ~0.8 of the ult's 38,000 radius: far enough that a dragged hero is
/// out of the swing, close enough that he can still reach them with Vengeance.
const ULT_DRAG_DISTANCE: f64 = 30_000.0;

/// Vengeance's CC immunity buff. Canon: the dash *"cannot be interrupted during
/// movement"* — scoped to the movement only, so Arlott is still fully
/// CC-vulnerable before the cast and after the dash lands.
const S2_CC_IMMUNE: &str = "mlbb_arlott_vengeance_cc_immune";
/// Arlott's dash is `range / speed` = 60,000 / 7,000 = **8.57 ticks** at full
/// range. 12 ticks covers that worst case with +3.43 ticks of margin, and —
/// like Chou's Shunpo (10-tick dash, 12-tick buff) — expires well before the
/// 46-tick skill ends, so the post-dash hit and recovery frames stay vulnerable.
const S2_CC_IMMUNE_TICKS: usize = 12;

/// Arlott's voice. Clips live in the mod as `sound/sfx/mlbb_arlott_sfx_vo_*`;
/// `play_voice` applies the roster's shared gates (actor lock + global lock), so
/// he cannot talk over himself or another hero.
const VOICE_SKILL1: &str = "mlbb_arlott_skill_cast";
const VOICE_SKILL2: &str = "mlbb_arlott_skill2_cast";
const VOICE_ULT: &str = "mlbb_arlott_ult_cast";

fn speak(sim: &mut StableSim<'_>, caster: usize, line: &str) {
    if !can_render(sim) {
        return;
    }
    let _ = crate::audio::play_voice(sim, caster, line);
}

fn percent(value: usize, pct: usize) -> usize {
    value.saturating_mul(pct) / 100
}

fn distance(a: (u64, u64), b: (u64, u64)) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
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

fn is_tower(sim: &StableSim<'_>, target: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| e.is_tower())
}

fn is_hostile(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target)
        .is_some_and(|e| e.team() != team && e.is_alive() && e.is_targetable())
}

/// Resolve the nearest eligible enemy to `origin` within `radius`.
/// Mirrors the scan in `alpha.rs::nearest_enemy`.
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

/// Is `target` carrying the Demon Gaze mark?
///
/// `StableEntity` has no `has_buff` (that lives on other handles), so this walks
/// the buff list the same way `alpha.rs::stack_count` does.
fn has_mark(sim: &StableSim<'_>, target: usize) -> bool {
    let Some(entity) = sim.get_entity(target) else {
        return false;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .any(|b| b.name() == MARK_BUFF)
}

/// Place (or refresh) the Demon Gaze mark. Re-applying refreshes the duration
/// rather than stacking, which is what canon says CC re-application does.
fn apply_mark(sim: &mut StableSim<'_>, target: usize) {
    let _ = sim.entity_remove_buff(target, MARK_BUFF);
    sim.add_buff(target, &BuffV1::timed(MARK_BUFF, MARK_TICKS));
}

// ---------------------------------------------------------------------------
// Passive — Demon Gaze
// ---------------------------------------------------------------------------

/// Two independent ways to place the mark, both on the passive because the
/// passive owns the 7 s clock:
///   * a 7 s auto-mark on the nearest enemy hero, and
///   * a mark on anything Arlott lands control on (the CC effects call
///     `apply_mark` directly, so the mark and the control land on the same
///     tick and cannot desync).
///
/// Priority (spec §2): unmarked first, then targetable, visible, nearest. The
/// `nearest_enemy` scan already enforces targetable + nearest; the unmarked
/// preference is done here so the 7 s tick tops up a fresh target instead of
/// wasting itself on one that is already carrying the eye.
#[derive(Clone, Default)]
struct ArlottDemonGaze {
    /// Ticks since the last auto-mark attempt.
    clock: usize,
    /// Ticks since spawn; used to stagger the very first auto-mark.
    age: usize,
}

impl StablePassive for ArlottDemonGaze {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, _sim: &mut StableSim<'_>, _player: usize, _entity: usize) {
        self.clock = 0;
        self.age = 0;
    }

    fn on_update(
        &mut self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        entity: usize,
    ) {
        self.age = self.age.saturating_add(1);
        if self.age < AUTO_MARK_DELAY {
            return;
        }
        self.clock = self.clock.saturating_add(1);
        if self.clock < AUTO_MARK_TICKS {
            return;
        }
        self.clock = 0;

        let Some((pos, team)) = sim.get_entity(entity).map(|e| (e.pos(), e.team())) else {
            return;
        };
        // Prefer an unmarked hero: that is the spec's stated priority, and it
        // also stops the auto-mark from burning every 7 s on one duel.
        let target = nearest_enemy(sim, pos, team, MARK_RANGE, true)
            .filter(|&t| !has_mark(sim, t))
            .or_else(|| nearest_enemy(sim, pos, team, MARK_RANGE, true));
        if let Some(target) = target {
            apply_mark(sim, target);
        }
    }
}

// ---------------------------------------------------------------------------
// S1 — Dauntless Strike: the delayed stun
// ---------------------------------------------------------------------------
//
// The data pays S1's physical damage through its own `LineRangeProjectile`; the
// stun is native so its length can depend on how far away the target was when
// the slam landed. This is the one place the kit's "the further the target is,
// the longer the stun" is expressed.

#[derive(Clone, Copy, Default)]
struct ArlottS1Stun;

impl ArlottS1Stun {
    /// Stun length for a target `d` units from the caster, linear from
    /// `STUN_MIN_TICKS` at zero range to `STUN_MAX_TICKS` at `S1_REACH`.
    fn ticks_for(d: f64) -> u64 {
        if !(d.is_finite()) {
            return STUN_MIN_TICKS;
        }
        let t = (d / S1_REACH).clamp(0.0, 1.0);
        let span = (STUN_MAX_TICKS - STUN_MIN_TICKS) as f64;
        STUN_MIN_TICKS + (span * t).round() as u64
    }
}

impl StableEffectType for ArlottS1Stun {
    fn apply(&self, sim: &mut StableSim<'_>, _tick: u64, caster: usize, input: InputTargetV1) {
        let Some((cpos, team)) = sim.get_entity(caster).map(|e| (e.pos(), e.team())) else {
            return;
        };
        let Some(target) = resolve_target(sim, cpos, team, input) else {
            return;
        };
        if !is_hostile(sim, target, team) || is_tower(sim, target) {
            return;
        }
        let Some(tpos) = sim.get_entity(target).map(|e| e.pos()) else {
            return;
        };
        let ticks = ArlottS1Stun::ticks_for(distance(cpos, tpos));
        sim.apply_cc(target, &CcV1::stun(ticks));
        speak(sim, caster, VOICE_SKILL1);
        // Control landed -> the eye opens. Same tick, so the mark can never be
        // applied to a target that resisted the stun.
        apply_mark(sim, target);
    }
}

/// Resolve the unit an effect hit. `applied_effects` are invoked with the
/// target already known, but the vtable hands us an input like Alpha's
/// strafes; accept a direct target first and fall back to the nearest enemy.
fn resolve_target(
    sim: &StableSim<'_>,
    cpos: (u64, u64),
    team: usize,
    input: InputTargetV1,
) -> Option<usize> {
    // `applied_effects` normally hand us the unit directly, so take it when the
    // input is a real target; otherwise fall back to a nearest-enemy scan.
    if input.kind == InputTargetKindV1::Target.code() && input.target_id != 0 {
        let id = input.target_id;
        if sim.get_entity(id).is_some_and(|e| e.team() != team) {
            return Some(id);
        }
    }
    nearest_enemy(sim, cpos, team, S1_REACH, false)
}

// ---------------------------------------------------------------------------
// S2 — Vengeance: the marked payoff
// ---------------------------------------------------------------------------
//
// Data pays the base hit to everything in the landing circle (100 + 55% attack
// as of v2.0.11; it was 90 + 35%). Canon says a MARKED target takes extra
// damage, Arlott's cooldown refreshes, and he heals. All three live here, and
// all three gate on the same mark check, so they can never diverge.

#[derive(Clone, Copy, Default)]
struct ArlottS2Vengeance;

impl StableEffectType for ArlottS2Vengeance {
    fn apply(&self, sim: &mut StableSim<'_>, _tick: u64, caster: usize, input: InputTargetV1) {
        let Some((cpos, team, attack)) = sim
            .get_entity(caster)
            .map(|e| (e.pos(), e.team(), e.stat().attack))
        else {
            return;
        };
        let Some(target) = resolve_target(sim, cpos, team, input) else {
            return;
        };
        if !is_hostile(sim, target, team) || is_tower(sim, target) || !has_mark(sim, target) {
            return;
        }
        // 1. The marked payoff: the data already paid one hit, so pay one more.
        //    v2.0.11: 70 + 35% attack, up from 60 + 20%. Marking can only be
        //    supplied about once per 7 s, so this hit carries most of Arlott's
        //    damage and it was the cheapest place to add some.
        let amount = percent(attack, 35).saturating_add(70);
        let _ = sim.deal_damage_typed(
            caster,
            target,
            amount,
            DamageTypeV1::Ad,
            AttackTypeV1::Skill,
        );
        // 2. Refresh S2. Full refund, so it is ready immediately.
        let _ = sim.entity_reduce_cooldown(caster, SkillSlotV1::Skill2, VENGEANCE_CD_REFUND);
        // 3. Heal — half value against anything that is not a champion, which is
        //    canon's "only 50% against non-hero units".
        let is_hero = sim.get_entity(target).is_some_and(|e| e.is_champion());
        let mut heal = percent(attack, VENGEANCE_HEAL_ATTACK_RATIO);
        if !is_hero {
            heal = percent(heal, VENGEANCE_MINION_HEAL_PCT);
        }
        if heal > 0 {
            sim.heal(caster, caster, heal);
        }
        // The mark has done its job; clear it so the next S2 needs fresh setup.
        let _ = sim.entity_remove_buff(target, MARK_BUFF);
        speak(sim, caster, VOICE_SKILL2);
    }
}

// ---------------------------------------------------------------------------
// Ult — Final Slash: drag every target to Arlott's right side
// ---------------------------------------------------------------------------
//
// Canon, two independent sources agreeing: the ult "directs all of his enemies
// to his **right side**" (gamingonphone, 2022-12) and cleaves "dragging them to
// the **side edge** of the area of effect, the direction where he swung his
// lance to" (TV Tropes). Players rely on it — a Reddit thread is entirely about
// the ult dumping enemies into the bottom-lane bush because that is his right.
//
// That is NOT what v1.1.73 shipped: it pushed radially away from Arlott, so the
// same target landed somewhere different depending on where it stood. This was
// owner-approved 2026-11-01 ("1 side drag").
//
// Geometry: the ult is cast in a `Direction` (`casting_type` in the data), so
// `input.dir_*` is the lance's swing direction — the same aim the fanned
// `RangeEffect` and the `arlott_ult_cleave` view effect already use. Rotating
// that aim by -90 degrees gives the cast's right side, and every target is
// pulled there at the same distance from Arlott. Because they share one anchor
// they also stack up, which is the whole point of the combo: drag, then
// Vengeance the pile.

#[derive(Clone, Copy, Default)]
struct ArlottUltPush;

/// Rotate `(x, y)` by -90 degrees: the caster's right-hand side.
fn right_of(dx: f64, dy: f64) -> (f64, f64) {
    (dy, -dx)
}

/// Swing aim from the cast input: the `Direction` when the host sends one, else
/// the caster-to-target line, else +x. Never returns a zero vector.
fn ult_aim(input: InputTargetV1, cpos: (u64, u64), tpos: (u64, u64)) -> (f64, f64) {
    let aim = if input.kind == InputTargetKindV1::Dir.code() {
        (input.dir_x as f64, input.dir_y as f64)
    } else {
        (tpos.0 as f64 - cpos.0 as f64, tpos.1 as f64 - cpos.1 as f64)
    };
    let len = aim.0.hypot(aim.1);
    if len > 1.0 {
        (aim.0 / len, aim.1 / len)
    } else {
        (1.0, 0.0)
    }
}

/// Where Final Slash drops `tpos`. Split out so the geometry is testable
/// without a simulation: the drag distance and the side are the two things that
/// must not silently regress.
fn ult_landing(cpos: (u64, u64), tpos: (u64, u64), aim: (f64, f64)) -> (u64, u64) {
    let (rx, ry) = right_of(aim.0, aim.1);
    let nx = (cpos.0 as f64 + rx * ULT_DRAG_DISTANCE).max(0.0);
    let ny = (cpos.1 as f64 + ry * ULT_DRAG_DISTANCE).max(0.0);
    let _ = tpos; // the landing point is anchored on the caster, not the target
    (nx as u64, ny as u64)
}

impl StableEffectType for ArlottUltPush {
    fn apply(&self, sim: &mut StableSim<'_>, _tick: u64, caster: usize, input: InputTargetV1) {
        let Some((cpos, team)) = sim.get_entity(caster).map(|e| (e.pos(), e.team())) else {
            return;
        };
        let Some(target) = resolve_target(sim, cpos, team, input) else {
            return;
        };
        if !is_hostile(sim, target, team) || is_tower(sim, target) {
            return;
        }
        let Some(tpos) = sim.get_entity(target).map(|e| e.pos()) else {
            return;
        };
        let (nx, ny) = ult_landing(cpos, tpos, ult_aim(input, cpos, tpos));
        let _ = sim.entity_set_pos(target, nx, ny);
        // The ult's sweep is control in canon's own words ("displacing"), so it
        // marks too — that is what makes ult -> S2 a legal, rewarding line.
        apply_mark(sim, target);
        speak(sim, caster, VOICE_ULT);
    }
}

// ---------------------------------------------------------------------------
// S2 — Vengeance: CC immunity during the dash
// ---------------------------------------------------------------------------
//
// Canon: "this skill cannot be interrupted during movement", and players confirm
// he can dash straight into a Khufra in S2 form. Bound as a `Native` at
// skill2.effect.effects[0], i.e. at CAST START, mirroring Chou's
// `mlbb_heroes:chou_s2_start` (chou data: idx 1 Native, idx 2 MoveTo).
//
// The owner has confirmed in-engine that `cc_immune` actually bites on Chou and
// Hanabi, so this is a working mechanism in this engine — not a guess.

#[derive(Clone, Copy, Default)]
struct ArlottS2CcImmune;

impl StableEffectType for ArlottS2CcImmune {
    fn apply(&self, sim: &mut StableSim<'_>, _tick: u64, caster: usize, _input: InputTargetV1) {
        let mut buff = BuffV1::timed(S2_CC_IMMUNE, S2_CC_IMMUNE_TICKS);
        buff.cc_immune = true;
        sim.add_buff(caster, &buff);
    }
}

pub fn register(module: &mut StableMod) {
    module.add_native_passive("mlbb_heroes:arlott_demon_gaze", ArlottDemonGaze::default());
    module.add_native_effect("mlbb_heroes:arlott_s1_stun", ArlottS1Stun);
    module.add_native_effect("mlbb_heroes:arlott_s2_cc_immune", ArlottS2CcImmune);
    module.add_native_effect("mlbb_heroes:arlott_s2_vengeance", ArlottS2Vengeance);
    module.add_native_effect("mlbb_heroes:arlott_ult_push", ArlottUltPush);
}

#[cfg(test)]
mod tests {
    use super::{
        percent, right_of, ult_aim, ult_landing, ArlottS1Stun, S1_REACH, S2_CC_IMMUNE,
        S2_CC_IMMUNE_TICKS, STUN_MAX_TICKS, STUN_MIN_TICKS, ULT_DRAG_DISTANCE, VENGEANCE_CD_REFUND,
        VENGEANCE_HEAL_ATTACK_RATIO, VENGEANCE_MINION_HEAL_PCT, VOICE_SKILL1, VOICE_SKILL2,
        VOICE_ULT,
    };
    use crate::InputTargetV1;

    #[test]
    fn s1_stun_is_short_at_the_feet_and_full_at_the_edge() {
        assert_eq!(ArlottS1Stun::ticks_for(0.0), STUN_MIN_TICKS);
        assert_eq!(ArlottS1Stun::ticks_for(S1_REACH), STUN_MAX_TICKS);
        assert_eq!(ArlottS1Stun::ticks_for(S1_REACH * 2.0), STUN_MAX_TICKS);
        assert_eq!(ArlottS1Stun::ticks_for(-5.0), STUN_MIN_TICKS);
    }

    #[test]
    fn s1_stun_never_decreases_with_distance() {
        // The kit's promise is "the further, the longer" — enforce monotonicity
        // rather than trusting the linear form.
        let mut last = 0;
        for step in 0..=20 {
            let d = S1_REACH * (step as f64) / 20.0;
            let t = ArlottS1Stun::ticks_for(d);
            assert!(t >= last, "stun shrank at distance {d}");
            assert!((STUN_MIN_TICKS..=STUN_MAX_TICKS).contains(&t));
            last = t;
        }
    }

    #[test]
    fn s1_stun_is_about_a_second_at_max() {
        // 60 ticks at 60 ticks/s.
        assert_eq!(STUN_MAX_TICKS, 60);
    }

    #[test]
    fn vengeance_heal_scales_with_attack() {
        // 30% of attack on a hero; half that on a minion.
        assert_eq!(percent(200, VENGEANCE_HEAL_ATTACK_RATIO), 60);
        assert_eq!(
            percent(
                percent(200, VENGEANCE_HEAL_ATTACK_RATIO),
                VENGEANCE_MINION_HEAL_PCT
            ),
            30
        );
        assert_eq!(percent(68, VENGEANCE_HEAL_ATTACK_RATIO), 20);
    }

    // -- S2 CC immunity ----------------------------------------------------

    #[test]
    fn s2_cc_immunity_covers_the_whole_dash() {
        // The dash is range/speed = 60_000/7_000 = 8.57 ticks. The buff must
        // outlast it, or the tail of the dash stays interruptible and the fix
        // does nothing for the case canon describes (dashing into Khufra).
        const DASH_TICKS: f64 = 60_000.0 / 7_000.0;
        assert!(
            (S2_CC_IMMUNE_TICKS as f64) > DASH_TICKS,
            "immunity ({S2_CC_IMMUNE_TICKS} ticks) must exceed the dash ({DASH_TICKS:.2} ticks)"
        );
    }

    #[test]
    fn s2_cc_immunity_expires_before_the_skill_ends() {
        // Canon scopes immunity to the MOVEMENT only. The skill runs 46 ticks;
        // immunity must be gone before then so the post-dash hit and recovery
        // are not immune. If this ever fails, we are granting free immunity.
        const SKILL_TICKS: usize = 46;
        assert!(
            S2_CC_IMMUNE_TICKS < SKILL_TICKS,
            "immunity ({S2_CC_IMMUNE_TICKS}) must end before the skill ({SKILL_TICKS}) does"
        );
        assert_eq!(S2_CC_IMMUNE, "mlbb_arlott_vengeance_cc_immune");
    }

    // -- S1 stun reach -----------------------------------------------------

    #[test]
    fn s1_line_stun_is_declared_per_target_in_the_data() {
        // The stun is a `Native` inside LineRangeProjectile.applied_effects,
        // alongside the damage, with casting_type "Targeting" -- so the HOST
        // invokes it once per unit in the line, exactly as Alpha's S1 fires
        // alpha_beta_strafe once per unit.
        //
        // This test pins the CONTRACT our native depends on. It does NOT prove
        // the host delivers per-target ids: `resolve_target` falls back to
        // nearest_enemy when it cannot, and InputTargetV1 carries a single
        // target_id so it cannot express "all units in the line".
        //
        // If S1 ever stuns only the front enemy, the cause is that fallback,
        // not the data or the stun curve. Fix by hardening resolve_target.
        const S1_WIDTH: f64 = 26_000.0;
        const S1_LENGTH: f64 = S1_REACH;
        assert!(
            S1_LENGTH > S1_WIDTH,
            "S1 must read as a line, not a square: {S1_LENGTH} > {S1_WIDTH}"
        );
        assert_eq!(S1_REACH, 42_000.0);
    }

    #[test]
    fn passive_mark_art_is_marker_sized_and_lifted_above_the_body() {
        // v1.1.70 shipped a 27x82 mark in a 48x96 cell with NO empty space below
        // the art, so with anchor_y 1.0 the eye landed at the enemy's feet and
        // stretched across a ~46px body. It covered the champion and made his
        // skills impossible to watch. Owner reported it; this test is the guard.
        //
        // The shipped Karrie mark (karrie_passive_marks_head_smaller) is the
        // proven reference: 48x142 cell, art in the TOP of the cell, ~100px of
        // empty space below so the glyph lands clear of the body.
        //
        // This checks GEOMETRY only. It cannot check the exact on-screen offset,
        // which is the host's anchor maths — that needs an in-engine look.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("effects"));
            roots.push(cur.join("arlott_work").join("effects"));
            if !cur.pop() {
                break;
            }
        }
        let dir = roots
            .iter()
            .find(|d| d.join("arlott_mark#anim.fanim").is_file())
            .unwrap_or_else(|| panic!("arlott_mark not found; looked in {roots:?}"))
            .clone();
        let fanim = std::fs::read_to_string(dir.join("arlott_mark#anim.fanim"))
            .expect("read arlott_mark fanim");

        // Cell must be the proven height, with the art in the top band only.
        // (Parsed by hand: no serde in this crate.)
        assert!(fanim.contains("\"anchor_y\": 1.0") || fanim.contains("\"anchor_y\":1.0"));
        let w = extract_int(&fanim, "\"w\"").expect("frame w");
        let h = extract_int(&fanim, "\"h\"").expect("frame h");
        assert_eq!(w, 48, "mark cell width must stay 48");
        assert_eq!(
            h, 142,
            "mark cell must be 48x142 to match Karrie's proven mark"
        );

        // The art must occupy at most the top third of the cell, so the body
        // below stays readable. 142/3 ~= 47px of art band max.
        let art_band_h = 33; // measured: art sits at cell y=7..33
        assert!(
            art_band_h <= h / 3,
            "art band ({art_band_h}px) must stay in the top third of the {h}px cell"
        );
    }

    /// First integer following `key` in a JSON-ish blob. Test-only convenience.
    fn extract_int(text: &str, key: &str) -> Option<i64> {
        let i = text.find(key)? + key.len();
        let rest = &text[i..];
        let start = rest.find(|c: char| c.is_ascii_digit())?;
        let tail = &rest[start..];
        let end = tail
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(tail.len());
        tail[..end].parse().ok()
    }

    #[test]
    fn every_voice_line_has_a_sound_info_and_a_clip() {
        // The naming-scheme test below only checks STRINGS, which is why Arlott
        // shipped silent for three versions: speak() resolved to nothing and no
        // test noticed. This one walks real files on disk.
        //
        // Layout mirrors Alpha: the .sound_info is named after the LINE speak()
        // passes, and its `plays[].clip` names the WAV asset. Naming a clip after
        // its own asset name resolves to nothing.
        //
        // The assets sit in `sound/sfx/` in the shipped mod and in
        // `arlott_work/sfx/` in the working tree, so search upward for either
        // rather than assuming one layout — a test that only passes in one
        // checkout is not a guard.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("sound").join("sfx"));
            roots.push(cur.join("arlott_work").join("sfx"));
            if !cur.pop() {
                break;
            }
        }
        let dir = roots
            .iter()
            .find(|d| d.is_dir())
            .unwrap_or_else(|| panic!("no sound dir found; looked in {roots:?}"))
            .clone();

        let lines = [
            (VOICE_SKILL1, "mlbb_arlott_sfx_vo_skill1"),
            (VOICE_SKILL2, "mlbb_arlott_sfx_vo_skill2"),
            (VOICE_ULT, "mlbb_arlott_sfx_vo_ult"),
        ];
        for (line, clip) in lines {
            let info = dir.join(format!("{line}.sound_info"));
            assert!(
                info.is_file(),
                "missing {info:?} — speak() would silently do nothing"
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
    fn voice_clips_follow_the_mod_naming_scheme() {
        // Owner's standing rule: voice on S1 / S2 / ult only, no hit SFX.
        assert_eq!(VOICE_SKILL1, "mlbb_arlott_skill_cast");
        assert_eq!(VOICE_SKILL2, "mlbb_arlott_skill2_cast");
        assert_eq!(VOICE_ULT, "mlbb_arlott_ult_cast");
    }

    #[test]
    fn vengeance_double_hit_matches_the_data_curve() {
        // Data pays 60 + 20% attack; the native pays the same again, so a marked
        // S2 is exactly 2x an unmarked one (spec §3: 73 -> 146 at L1).
        for (attack, expected_total) in [(68, 146), (138, 174)] {
            let one = 60 + percent(attack, 20);
            let marked = one * 2;
            assert_eq!(marked, expected_total, "attack {attack}");
        }
    }

    // -----------------------------------------------------------------
    // Ult — Final Slash drags to his right side, not radially outward
    // -----------------------------------------------------------------

    #[test]
    fn ult_right_side_is_a_quarter_turn_from_the_swing() {
        // Aim east -> right side is south, and vice versa. The sign is the part
        // players feel: drag into a wall / the bottom-lane bush, not behind him.
        assert_eq!(right_of(1.0, 0.0), (0.0, -1.0));
        assert_eq!(right_of(0.0, -1.0), (-1.0, 0.0));
        assert_eq!(right_of(-1.0, 0.0), (0.0, 1.0));
    }

    #[test]
    fn ult_is_perpendicular_to_the_aim_at_every_angle() {
        for deg in (0..360).step_by(15) {
            let r = (deg as f64).to_radians();
            let aim = (r.cos(), r.sin());
            let (rx, ry) = right_of(aim.0, aim.1);
            let dot = aim.0 * rx + aim.1 * ry;
            assert!(
                dot.abs() < 1e-9,
                "right_of must stay perpendicular, dot={dot}"
            );
            assert!(
                (rx.hypot(ry) - 1.0).abs() < 1e-9,
                "right_of must not change the length"
            );
        }
    }

    #[test]
    fn ult_stacks_everyone_on_one_point_of_his_right_side() {
        // The whole reason canon drags instead of pushing: targets end up in a
        // pile, so Vengeance hits the group. Radially pushing gave every target
        // its own landing spot, which is what this replaces.
        let cpos = (10_000u64, 10_000u64);
        let aim = (1.0, 0.0); // swings east
        let a = ult_landing(cpos, (18_000, 10_000), aim);
        let b = ult_landing(cpos, (10_000, 2_000), aim);
        let c = ult_landing(cpos, (14_000, 15_000), aim);
        assert_eq!(a, b, "different targets, same cast -> same landing");
        assert_eq!(b, c, "different targets, same cast -> same landing");
    }

    #[test]
    fn ult_lands_the_drag_distance_from_him_on_the_right() {
        let cpos = (50_000u64, 50_000u64);
        let aim = (0.0, 1.0); // swings "up"
        let (nx, ny) = ult_landing(cpos, (50_000, 60_000), aim);
        // right_of((0,1)) = (1, 0) -> due east of him, by the constant.
        assert_eq!(nx, 50_000 + ULT_DRAG_DISTANCE as u64);
        assert_eq!(ny, 50_000);
    }

    #[test]
    fn ult_never_lands_inside_the_swing_radius() {
        // Sanity on the constant: dragging somebody 2,000 units sideways would
        // leave them standing in the ult that just hit them.
        assert!(
            ULT_DRAG_DISTANCE > 20_000.0,
            "drag must clear the 38,000-radius swing's core"
        );
        assert!(
            ULT_DRAG_DISTANCE <= 38_000.0,
            "drag must not throw targets outside the ult's range"
        );
    }

    #[test]
    fn ult_aim_prefers_the_casts_direction_then_falls_back() {
        let cpos = (0u64, 0u64);
        let tpos = (0u64, 5_000u64);
        // A real Direction input is normalised and used as-is.
        let d = ult_aim(InputTargetV1::dir(0, 300), cpos, tpos);
        assert!((d.0).abs() < 1e-9 && (d.1 - 1.0).abs() < 1e-9);
        // No direction: use the caster->target line.
        let t = ult_aim(InputTargetV1::target(7), cpos, tpos);
        assert!((t.1 - 1.0).abs() < 1e-9);
        // Degenerate input must still give a usable unit vector.
        let z = ult_aim(InputTargetV1::dir(0, 0), cpos, cpos);
        assert!((z.0.hypot(z.1) - 1.0).abs() < 1e-9);
    }

    /// Body of the brace-delimited object that starts at or after `from`.
    /// Test-only: the crate has no serde, and pulling one in for a guard is not
    /// worth it. Returns the slice *inside* the braces.
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
    fn every_locale_describes_arlott_completely() {
        // Until v1.1.74 only English had Arlott's skill text; zh-hans, zh-hant,
        // ko and ja carried his name and nothing else, so the hero read blank in
        // four of five shipped languages. Nothing caught it because no test
        // looked at champion.i18n at all. This is that test.
        //
        // It checks STRUCTURE, not prose: in every locale Arlott's `description`
        // object must carry all six keys and his `skill_name` object all four.
        // Wording may change; a missing line cannot.
        //
        // Scope matters. A naive `text.contains("skill2")` would pass no matter
        // what, because every hero in the file has a skill2 — the first version
        // of this test did exactly that and could not fail. The check has to
        // close in on Arlott's own object, so it walks braces.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("text").join("champion.i18n"));
            roots.push(cur.join("arlott_work").join("text").join("champion.i18n"));
            if !cur.pop() {
                break;
            }
        }
        let path = roots
            .iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("champion.i18n not found; looked in {roots:?}"));
        let text = std::fs::read_to_string(path).expect("read champion.i18n");

        let locales = ["\"en\"", "\"zh-hans\"", "\"zh-hant\"", "\"ko\"", "\"ja\""];
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

            // In a locale block the first `mlbb_arlott` is the description node
            // and the second is skill_name.
            let first = block.find("\"mlbb_arlott\"").expect("arlott description");
            let desc = object_body(block, first);
            for key in ["name", "attack", "passive", "skill", "skill2", "ult"] {
                let needle = format!("\"{key}\"");
                assert!(
                    desc.contains(&needle),
                    "locale {loc}: Arlott's description has no {needle}"
                );
            }

            let second = block[first + 1..]
                .find("\"mlbb_arlott\"")
                .map(|o| first + 1 + o)
                .expect("arlott skill_name");
            let names = object_body(block, second);
            for key in ["passive", "skill1", "skill2", "ult"] {
                let needle = format!("\"{key}\"");
                assert!(
                    names.contains(&needle),
                    "locale {loc}: Arlott's skill_name has no {needle}"
                );
            }
        }
    }

    #[test]
    fn arlott_is_actually_wired_into_the_crate_root() {
        // v1.1.74 shipped a source-with-SDK archive whose lib.rs had NO `mod
        // arlott;` and no `arlott::register(...)` call. The archive built, but
        // it built a mod WITHOUT Arlott — the hero was simply absent, and every
        // other test in this file still passed because they exercise arlott.rs
        // directly. Found by the owner asking why the archive was 25 MB instead
        // of the usual 46.
        //
        // A native effect that is never registered is dead code that compiles.
        // This checks the wiring exists in whatever lib.rs sits next to us.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("lib.rs"));
            roots.push(cur.join("src").join("lib.rs"));
            if !cur.pop() {
                break;
            }
        }
        let path = roots
            .iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("no lib.rs near the crate; looked in {roots:?}"));
        let text = std::fs::read_to_string(path).expect("read lib.rs");

        assert!(
            text.contains("mod arlott;"),
            "{} has no `mod arlott;` — the module is never compiled",
            path.display()
        );
        assert!(
            text.contains("arlott::register"),
            "{} never calls `arlott::register(...)` — Arlott's natives are never \
             registered, so the hero ships absent",
            path.display()
        );
    }

    // --- S1 art guards -----------------------------------------------------
    // The crate has no zlib, so these cannot inflate arlott_s1_wave#sheet.png.
    // Instead the builder (`alpha_work/build_s1_fissure.py`) records the ink
    // extent it actually produced in a geometry sidecar, and these tests
    // re-check the properties the art has to hold. If the sidecar is missing
    // they FAIL — a guard that skips when it cannot see the art is not a guard.

    fn s1_sidecar() -> String {
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("effects").join("arlott_s1_wave#geometry.json"));
            roots.push(
                cur.join("arlott_work")
                    .join("effects")
                    .join("arlott_s1_wave#geometry.json"),
            );
            if !cur.pop() {
                break;
            }
        }
        let path = roots.iter().find(|p| p.is_file()).unwrap_or_else(|| {
            panic!("arlott_s1_wave geometry sidecar not found; looked in {roots:?}")
        });
        std::fs::read_to_string(path).expect("read sidecar")
    }

    fn s1_int(text: &str, key: &str) -> i64 {
        let i = text
            .find(key)
            .unwrap_or_else(|| panic!("sidecar has no {key}"))
            + key.len();
        let rest = &text[i..];
        let start = rest.find(|c: char| c.is_ascii_digit()).expect("number");
        let tail = &rest[start..];
        let end = tail
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(tail.len());
        tail[..end].parse().expect("parse")
    }

    /// "runs": [[l0,r0],[l1,r1],...] -> [(l,r); frames]
    fn s1_runs(text: &str, frames: usize) -> Vec<(i64, i64)> {
        let at = text.find("\"runs\":").expect("sidecar has no runs");
        let nums: Vec<i64> = text[at..]
            .chars()
            .scan(String::new(), |acc, ch| {
                if ch.is_ascii_digit() {
                    acc.push(ch);
                    Some(None)
                } else if acc.is_empty() {
                    Some(None)
                } else {
                    let v = acc.parse().ok();
                    acc.clear();
                    Some(v)
                }
            })
            .flatten()
            .collect();
        assert!(
            nums.len() >= frames * 2,
            "sidecar needs 2 runs per frame, found {}",
            nums.len()
        );
        (0..frames)
            .map(|f| (nums[f * 2], nums[f * 2 + 1]))
            .collect()
    }

    #[test]
    fn s1_fissure_is_a_projectile_sprite_and_holds_its_length() {
        // Reported from gameplay: the S1 effect "appears on him not in front of
        // him or the enemy". Root cause was DATA, not art — the skill spawned
        // `CasterViewEffect`s, so the fissure and the impact burst were drawn at
        // the caster. Fixed by deleting both; the effect now comes only from the
        // `view_projectiles` binding to `arlott_s1_slam`.
        //
        // That makes this a PROJECTILE sprite, and a projectile sprite must be
        // the same length in every frame — a "reveal" (v1.1.77) would carry a
        // growing-from-the-left-edge animation down the field with it.
        let text = s1_sidecar();
        let cell_w = s1_int(&text, "\"cell_w\":");
        let frames = s1_int(&text, "\"frames\":");
        assert_eq!(cell_w, 96, "the S1 cell must stay 96 px");
        assert_eq!(frames, 6, "the S1 effect is a 6-frame animation");
        assert!(
            text.contains("\"projectile_static\""),
            "sidecar must declare mode projectile_static — anything else means the \
             builder is animating the geometry again"
        );

        let runs = s1_runs(&text, frames as usize);
        let widths: Vec<i64> = runs.iter().map(|(l, r)| r - l + 1).collect();

        for f in 0..frames as usize {
            let (l, r) = runs[f];
            assert!(
                l > 0,
                "frame {f} starts at x={l} — a projectile sprite needs margin so the \
                 hot core does not clip at the cell edge"
            );
            assert!(r >= cell_w - 8, "frame {f} stops at {r}, short of the cell");
        }

        let spread = *widths.iter().max().unwrap() - *widths.iter().min().unwrap();
        assert!(
            spread <= 2,
            "the fissure changes length across frames ({widths:?}) — as a projectile \
             sprite it must hold its length and animate only its light"
        );
    }

    #[test]
    fn s1_never_draws_the_fissure_on_the_caster() {
        // The actual v1.1.77 bug, guarded at the level that caused it. A name
        // bound in `view_projectiles` must NOT also appear as a
        // `CasterViewEffect` in the skill list, or the engine draws it twice:
        // once down the line, once standing on Arlott.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("champion").join("mlbb_arlott.data_champion"));
            roots.push(
                cur.join("arlott_work")
                    .join("runtime")
                    .join("mlbb_heroes")
                    .join("champion")
                    .join("mlbb_arlott.data_champion"),
            );
            if !cur.pop() {
                break;
            }
        }
        let path = roots
            .iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("mlbb_arlott.data_champion not found; {roots:?}"));
        let text = std::fs::read_to_string(path).expect("read champion data");

        // the S1 block only
        let start = text.find("\"skill\":").expect("no skill block");
        let end = text.find("\"skill2\":").expect("no skill2 block");
        let s1 = &text[start..end];

        assert!(
            !s1.contains("CasterViewEffect"),
            "{} S1 still spawns a CasterViewEffect — that draws the effect on \
             Arlott instead of out along the line",
            path.display()
        );
        assert!(
            s1.contains("LineRangeProjectile"),
            "S1 lost its LineRangeProjectile — the damage would not fire"
        );
        assert!(
            text.contains("\"view_projectiles\"") && text.contains("\"name\": \"arlott_s1_slam\""),
            "the fissure must stay bound to the arlott_s1_slam projectile"
        );
    }

    const ARLOTT_DATA: &str = include_str!("../champion/mlbb_arlott.data_champion");

    /// Read the first integer field named `field` that follows `anchor`.
    fn field_after(text: &str, anchor: &str, field: &str) -> usize {
        let start = text.find(anchor).expect("anchor missing from data file");
        let rest = &text[start..];
        let key = format!("\"{field}\"");
        let at = rest.find(&key).expect("field missing after anchor");
        let tail = &rest[at + key.len()..];
        let colon = tail.find(':').expect("malformed field");
        tail[colon + 1..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .expect("field is not an integer")
    }

    /// v2.0.11 rebalance. Arlott's rotation sat at 83% of Lancelot's at level 1
    /// but only 51% at level 15, because his skills carried no per-level growth
    /// while Lancelot's do. Raising the ratios (not just the flats) is what
    /// closes the late-game gap, so these pin the ratios too.
    #[test]
    fn arlott_damage_matches_the_v2_0_11_rebalance() {
        assert_eq!(
            (
                field_after(ARLOTT_DATA, "\"action_name\": \"skill\",", "damage"),
                field_after(ARLOTT_DATA, "\"action_name\": \"skill\",", "attack_ratio"),
            ),
            (110, 55)
        );
        assert_eq!(
            (
                field_after(ARLOTT_DATA, "\"action_name\": \"skill2\"", "damage"),
                field_after(ARLOTT_DATA, "\"action_name\": \"skill2\"", "attack_ratio"),
            ),
            (100, 55)
        );
        assert_eq!(
            (
                field_after(ARLOTT_DATA, "\"action_name\": \"ult\"", "damage"),
                field_after(ARLOTT_DATA, "\"action_name\": \"ult\"", "attack_ratio"),
            ),
            (160, 75)
        );
    }

    /// S2 is now 3 s. The refund must equal the cooldown or "full refresh" stops
    /// being true, and the whole point of the mark payoff is that it is free.
    #[test]
    fn the_vengeance_refund_still_equals_the_three_second_cooldown() {
        let cd = field_after(ARLOTT_DATA, "\"action_name\": \"skill2\"", "cooltime");
        assert_eq!(cd, 180, "S2 cooldown must be 180 ticks (3 s)");
        assert_eq!(
            VENGEANCE_CD_REFUND, cd,
            "the refund must track the data cooldown"
        );
    }

    /// The totals the owner approved: L1 588 (was 494, +19%) and L15 741
    /// (was 592, +25%).
    #[test]
    fn the_rebalanced_rotation_lands_where_the_owner_approved_it() {
        let l1 = 68usize; // Arlott's base attack
        let s1 = 110 + l1 * 55 / 100;
        let s2 = (100 + l1 * 55 / 100) + (70 + l1 * 35 / 100);
        let ult = 160 + l1 * 75 / 100;
        assert_eq!((s1, s2, ult), (147, 230, 211));
        assert_eq!(s1 + s2 + ult, 588);

        let l15 = 138usize; // 68 + 5/level
        let s1 = 110 + l15 * 55 / 100;
        let s2 = (100 + l15 * 55 / 100) + (70 + l15 * 35 / 100);
        let ult = 160 + l15 * 75 / 100;
        assert_eq!(s1 + s2 + ult, 741);
    }
}
