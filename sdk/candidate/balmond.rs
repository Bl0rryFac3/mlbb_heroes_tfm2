//! Native Balmond gameplay for the separate TFM2 candidate.
use crate::audio;
use mod_api_stable::*;

const UNIT_HEAL_PERCENT: usize = 5;
const HERO_HEAL_PERCENT: usize = 10;
const S1_SLOW: &str = "mlbb_heroes_balmond_s1_slow";
const S1_MAX_DASH_DISTANCE: f64 = 90_000.0;
const S1_PATH_HALF_WIDTH: f64 = 20_000.0;
const S1_ARRIVAL_HIT_RADIUS: f64 = 30_000.0;
const S2_ACTIVE: &str = "mlbb_heroes_balmond_s2_active";
const S2_SPEED: &str = "mlbb_heroes_balmond_s2_speed";
const S2_SHRED_PREFIX: &str = "mlbb_heroes_balmond_s2_shred_";
const S2_DURATION: usize = 180; // 3s at 60 simulation ticks/s
const S2_HITS: usize = 14;
const S2_MAX_SHRED_STACKS: usize = 8;
const S2_RADIUS: f64 = 36_000.0; // TFM2 geometry adaptation; validate in match
const MOVE_LINES: &[&str] = &[
    "mlbb_balmond_voice_move01",
    "mlbb_balmond_voice_move02",
    "mlbb_balmond_voice_move03",
    "mlbb_balmond_voice_move04",
    "mlbb_balmond_voice_move05",
    "mlbb_balmond_voice_move06",
    "mlbb_balmond_voice_move07",
    "mlbb_balmond_voice_move08",
    "mlbb_balmond_voice_move09",
    "mlbb_balmond_voice_move10",
    "mlbb_balmond_voice_move11",
    "mlbb_balmond_voice_move12",
    "mlbb_balmond_voice_move13",
    "mlbb_balmond_voice_move14",
    "mlbb_balmond_voice_move15",
    "mlbb_balmond_voice_move16",
    "mlbb_balmond_voice_move17",
    "mlbb_balmond_voice_move18",
    "mlbb_balmond_voice_move19",
];
const BASIC_LINES: &[&str] = &[
    "mlbb_balmond_voice_basic01",
    "mlbb_balmond_voice_basic02",
    "mlbb_balmond_voice_basic03",
    "mlbb_balmond_voice_basic04",
];
const DEATH_LINES: &[&str] = &["mlbb_balmond_voice_death01", "mlbb_balmond_voice_death02"];

fn percent(value: usize, pct: usize) -> usize {
    value.saturating_mul(pct) / 100
}
fn level_linear(low: usize, high: usize, level: usize) -> usize {
    low + (high - low).saturating_mul(level.saturating_sub(1).min(14)) / 14
}
fn distance(a: (u64, u64), b: (u64, u64)) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
}
fn path_projection(origin: (u64, u64), point: (u64, u64), ux: f64, uy: f64) -> (f64, f64) {
    let px = point.0 as f64 - origin.0 as f64;
    let py = point.1 as f64 - origin.1 as f64;
    (px * ux + py * uy, (px * uy - py * ux).abs())
}
fn bounded_position_path(
    origin: (u64, u64),
    destination: (u64, u64),
    max_distance: f64,
) -> Option<(f64, f64, f64)> {
    let dx = destination.0 as f64 - origin.0 as f64;
    let dy = destination.1 as f64 - origin.1 as f64;
    let length = dx.hypot(dy);
    if length < 1.0 || max_distance <= 0.0 {
        return None;
    }
    Some((dx / length, dy / length, length.min(max_distance)))
}
fn soul_lock_damage(attack: usize, level: usize) -> usize {
    level_linear(150, 275, level).saturating_add(percent(attack, 60))
}
fn log_s2_match_view(sim: &StableSim<'_>, message: String) {
    let is_match_view = sim
        .sim_origin()
        .and_then(|origin| SimOriginKindV1::from_code(origin.kind))
        .is_some_and(|kind| kind == SimOriginKindV1::ClientMatchView);
    if is_match_view {
        crate::wlog(message);
    }
}

fn has_buff(sim: &StableSim<'_>, id: usize, name: &str) -> bool {
    sim.get_entity(id).is_some_and(|e| {
        (0..e.buff_count())
            .filter_map(|i| e.buff_at(i))
            .any(|b| b.name() == name)
    })
}
fn apply_slow(sim: &mut StableSim<'_>, target: usize, name: &str, pct: i32, ticks: usize) {
    let _ = sim.entity_remove_buff(target, name);
    let mut buff = BuffV1::timed(name, ticks);
    buff.move_speed_mult = -pct;
    sim.add_buff(target, &buff);
}
fn cyclone_hit_damage(attack: usize, max_hp: usize, level: usize) -> usize {
    level_linear(24, 44, level)
        .saturating_add(percent(attack, 25))
        .saturating_add(max_hp.saturating_mul(13) / 1000)
}
fn missing_hp_damage(max_hp: usize, current_hp: usize, ratio_tenths_pct: usize) -> usize {
    max_hp
        .saturating_sub(current_hp)
        .saturating_mul(ratio_tenths_pct)
        / 1000
}
fn ult_ratio_tenths_pct(level: usize) -> usize {
    300 + 75 * ((level.saturating_sub(1) / 5).min(2))
}
fn ult_nonhero_cap(level: usize) -> usize {
    1000 + 500 * ((level.saturating_sub(1) / 5).min(2))
}
fn lethal_counter_damage(
    attack: usize,
    max_hp: usize,
    current_hp: usize,
    level: usize,
    is_champion: bool,
) -> usize {
    let base = 150usize.saturating_add(percent(attack, 70));
    let amount = base.saturating_add(missing_hp_damage(
        max_hp,
        current_hp,
        ult_ratio_tenths_pct(level),
    ));
    if is_champion {
        amount
    } else {
        amount.min(ult_nonhero_cap(level))
    }
}

#[derive(Clone, Default)]
struct BalmondBloodthirst {
    cyclone_started: Option<usize>,
    next_s2_hit: usize,
    basic_voice_cursor: usize,
    voice: audio::VoiceLifecycle,
}
impl BalmondBloodthirst {
    fn heal_owner(&self, sim: &mut StableSim<'_>, owner: usize, pct: usize) {
        if let Some(me) = sim.get_entity(owner) {
            if me.is_alive() {
                let (_, max) = me.hp();
                sim.heal(owner, owner, percent(max, pct));
            }
        }
    }
    fn cyclone_hit(&self, sim: &mut StableSim<'_>, owner: usize) -> usize {
        let Some(me) = sim.get_entity(owner) else {
            return 0;
        };
        let pos = me.pos();
        let team = me.team();
        let stat = me.stat();
        let (_, max_hp) = me.hp();
        let level = me.level().max(1);
        let damage = cyclone_hit_damage(stat.attack, max_hp, level);
        let ids: Vec<usize> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i).map(|e| e.id()))
            .collect();
        let mut hit_targets = 0usize;
        for id in ids {
            let Some(target) = sim.get_entity(id) else {
                continue;
            };
            if target.team() == team
                || !target.is_alive()
                || !target.is_targetable()
                || target.is_tower()
                || distance(pos, target.pos()) > S2_RADIUS
            {
                continue;
            }
            let mut stacks = 0usize;
            for n in 1..=S2_MAX_SHRED_STACKS {
                if has_buff(sim, id, &format!("{S2_SHRED_PREFIX}{owner}_{n}")) {
                    stacks = n;
                }
            }
            let next = (stacks + 1).min(S2_MAX_SHRED_STACKS);
            for n in 1..=S2_MAX_SHRED_STACKS {
                let _ = sim.entity_remove_buff(id, &format!("{S2_SHRED_PREFIX}{owner}_{n}"));
            }
            let mut shred = BuffV1::timed(&format!("{S2_SHRED_PREFIX}{owner}_{next}"), S2_DURATION);
            shred.defence_mult = -(5 * next as i32);
            sim.add_buff(id, &shred);
            sim.deal_damage(owner, id, damage, 0, AttackTypeV1::Skill);
            hit_targets += 1;
        }
        hit_targets
    }
}
impl StablePassive for BalmondBloodthirst {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _: usize, entity: usize) {
        self.cyclone_started = None;
        self.next_s2_hit = 0;
        self.basic_voice_cursor = 0;
        self.voice
            .on_spawn(sim, entity, Some("mlbb_balmond_voice_select"));
    }
    fn on_kill(&mut self, sim: &mut StableSim<'_>, _: usize, owner: usize, victim: usize) {
        if sim.get_entity(victim).is_some_and(|v| v.is_champion()) {
            self.heal_owner(sim, owner, HERO_HEAL_PERCENT);
        }
    }
    fn on_assist(&mut self, sim: &mut StableSim<'_>, _: usize, owner: usize) {
        self.heal_owner(sim, owner, HERO_HEAL_PERCENT);
    }
    fn on_unit_kill(&mut self, sim: &mut StableSim<'_>, _: usize, owner: usize, _: usize) {
        self.heal_owner(sim, owner, UNIT_HEAL_PERCENT);
    }
    fn on_base_attack(&mut self, sim: &mut StableSim<'_>, _: u64, _: usize, owner: usize) {
        let line = BASIC_LINES[self.basic_voice_cursor % BASIC_LINES.len()];
        self.basic_voice_cursor = self.basic_voice_cursor.wrapping_add(1);
        let _ = audio::play_voice(sim, owner, line);
    }
    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        self.voice.on_dead(sim, player, DEATH_LINES);
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, _: usize, owner: usize) {
        self.voice.on_update(sim, owner, &[], MOVE_LINES);
        if !has_buff(sim, owner, S2_ACTIVE) {
            self.cyclone_started = None;
            self.next_s2_hit = 0;
            let _ = sim.entity_remove_buff(owner, S2_SPEED);
            return;
        }
        let now = sim.tick();
        let first_observed_tick = self.cyclone_started.is_none();
        let start = *self.cyclone_started.get_or_insert(now);
        if first_observed_tick {
            log_s2_match_view(
                sim,
                format!("[BALMOND_S2] passive_active caster={owner} tick={now}"),
            );
        }
        while self.next_s2_hit < S2_HITS
            && now >= start + (self.next_s2_hit * S2_DURATION / S2_HITS)
        {
            let pulse = self.next_s2_hit + 1;
            let hit_targets = self.cyclone_hit(sim, owner);
            log_s2_match_view(sim, format!("[BALMOND_S2] pulse={pulse}/{S2_HITS} caster={owner} tick={now} targets={hit_targets} radius={S2_RADIUS:.0}"));
            self.next_s2_hit += 1;
        }
    }
}

#[derive(Clone, Copy, Default)]
struct BalmondSoulLockPath;
impl StableEffectType for BalmondSoulLockPath {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Pos.code() {
            return;
        }
        let Some((start, team, attack, level)) = sim
            .get_entity(caster)
            .map(|e| (e.pos(), e.team(), e.stat().attack, e.level().max(1)))
        else {
            return;
        };
        let Some((ux, uy, path_limit)) =
            bounded_position_path(start, (input.x, input.y), S1_MAX_DASH_DISTANCE)
        else {
            return;
        };
        let damage = soul_lock_damage(attack, level);
        let ids: Vec<usize> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i).map(|e| e.id()))
            .collect();

        // Sample minion damage only along the selected, capped route. To retain
        // Balmond's prior path behavior, do not damage units beyond the first
        // enemy champion on that route; the champion is hit at MoveTo completion.
        let mut stop_at = path_limit;
        for id in &ids {
            let Some(target) = sim.get_entity(*id) else {
                continue;
            };
            if target.team() == team
                || !target.is_alive()
                || !target.is_targetable()
                || !target.is_champion()
                || target.is_tower()
            {
                continue;
            }
            let (along, across) = path_projection(start, target.pos(), ux, uy);
            if along >= 0.0 && along <= stop_at && across <= S1_PATH_HALF_WIDTH {
                stop_at = along;
            }
        }

        for id in ids {
            let Some(target) = sim.get_entity(id) else {
                continue;
            };
            if target.team() == team
                || !target.is_alive()
                || !target.is_targetable()
                || target.is_champion()
                || target.is_tower()
            {
                continue;
            }
            let (along, across) = path_projection(start, target.pos(), ux, uy);
            if along < 0.0 || along > stop_at || across > S1_PATH_HALF_WIDTH {
                continue;
            }
            sim.deal_damage(caster, id, damage, 0, AttackTypeV1::Skill);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct BalmondSoulLockHit;
impl StableEffectType for BalmondSoulLockHit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _: InputTargetV1) {
        let Some((caster_pos, caster_team, attack, level)) = sim
            .get_entity(caster)
            .map(|e| (e.pos(), e.team(), e.stat().attack, e.level().max(1)))
        else {
            return;
        };
        // MoveTo end effects are location-oriented, so resolve one nearest
        // eligible enemy champion around Balmond's actual arrival position.
        let target = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i).map(|e| e.id()))
            .filter_map(|id| {
                let target = sim.get_entity(id)?;
                if target.team() == caster_team
                    || !target.is_alive()
                    || !target.is_targetable()
                    || !target.is_champion()
                    || target.is_tower()
                {
                    return None;
                }
                let d = distance(caster_pos, target.pos());
                (d <= S1_ARRIVAL_HIT_RADIUS).then_some((id, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id);
        let Some(target) = target else {
            return;
        };
        let damage = soul_lock_damage(attack, level);
        sim.deal_damage(caster, target, damage, 0, AttackTypeV1::Skill);
        apply_slow(sim, target, S1_SLOW, 40, 180);
        // Retain the existing small impact bump on the struck hero.
        let _ = sim.entity_knockback(caster, target, 1_000, 3);
    }
}

#[derive(Clone, Copy, Default)]
struct BalmondCycloneStart;
impl StableEffectType for BalmondCycloneStart {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _: InputTargetV1) {
        let _ = sim.entity_remove_buff(caster, S2_ACTIVE);
        let _ = sim.entity_remove_buff(caster, S2_SPEED);
        log_s2_match_view(
            sim,
            format!(
                "[BALMOND_S2] native_start caster={caster} tick={} duration={S2_DURATION}",
                sim.tick()
            ),
        );
        let active = BuffV1::timed(S2_ACTIVE, S2_DURATION);
        sim.add_buff(caster, &active);
        let mut speed = BuffV1::timed(S2_SPEED, S2_DURATION);
        speed.move_speed_mult = 15;
        sim.add_buff(caster, &speed);
    }
}

#[derive(Clone, Copy, Default)]
struct BalmondLethalCounter;
impl StableEffectType for BalmondLethalCounter {
    // The data-driven RangeEffect selects the directional fan targets. This
    // native effect is invoked once per selected target and owns only the live
    // missing-HP formula, fixed-damage route, and non-hero cap.
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target_id = input.target_id;
        let Some((team, attack, level)) = sim
            .get_entity(caster)
            .map(|e| (e.team(), e.stat().attack, e.level().max(1)))
        else {
            return;
        };
        let Some((target_team, alive, targetable, is_tower, is_champion, hp, max_hp)) =
            sim.get_entity(target_id).map(|e| {
                let (hp, max_hp) = e.hp();
                (
                    e.team(),
                    e.is_alive(),
                    e.is_targetable(),
                    e.is_tower(),
                    e.is_champion(),
                    hp,
                    max_hp,
                )
            })
        else {
            return;
        };
        if target_team == team || !alive || !targetable || is_tower {
            return;
        }
        let amount = lethal_counter_damage(attack, max_hp, hp, level, is_champion);
        let _ = sim.deal_damage_typed(
            caster,
            target_id,
            amount,
            DamageTypeV1::Fixed,
            AttackTypeV1::Skill,
        );
    }
}

pub fn register(module: &mut StableMod) {
    module.add_native_passive(
        "mlbb_heroes:balmond_bloodthirst",
        BalmondBloodthirst::default(),
    );
    module.add_native_effect("mlbb_heroes:balmond_soul_lock_path", BalmondSoulLockPath);
    module.add_native_effect("mlbb_heroes:balmond_soul_lock_hit", BalmondSoulLockHit);
    module.add_native_effect("mlbb_heroes:balmond_cyclone_start", BalmondCycloneStart);
    module.add_native_effect("mlbb_heroes:balmond_lethal_counter", BalmondLethalCounter);
}

#[cfg(test)]
mod tests {
    use super::{
        bounded_position_path, cyclone_hit_damage, distance, lethal_counter_damage, missing_hp_damage,
        path_projection, percent, soul_lock_damage, ult_nonhero_cap, ult_ratio_tenths_pct,
    };
    #[test]
    fn bloodthirst_amounts_match_current_reference() {
        assert_eq!(percent(2000, 5), 100);
        assert_eq!(percent(2000, 10), 200);
    }
    #[test]
    fn cyclone_hit_scales_with_level_attack_and_health() {
        assert_eq!(cyclone_hit_damage(100, 1000, 1), 62);
        assert_eq!(cyclone_hit_damage(100, 1000, 8), 72);
        assert_eq!(cyclone_hit_damage(100, 1000, 15), 82);
    }
    #[test]
    fn soul_lock_damage_and_position_geometry_are_stable() {
        assert_eq!(soul_lock_damage(100, 1), 210);
        assert_eq!(soul_lock_damage(100, 15), 335);
        assert_eq!(
            path_projection((0, 0), (30_000, 4_000), 1.0, 0.0),
            (30_000.0, 4_000.0)
        );
        assert_eq!(
            path_projection((0, 0), (30_000, 4_000), 0.0, 1.0),
            (4_000.0, 30_000.0)
        );
        assert_eq!(
            bounded_position_path((0, 0), (30_000, 40_000), 90_000.0),
            Some((0.6, 0.8, 50_000.0))
        );
        assert_eq!(
            bounded_position_path((0, 0), (120_000, 0), 90_000.0),
            Some((1.0, 0.0, 90_000.0))
        );
        assert_eq!(bounded_position_path((0, 0), (0, 0), 90_000.0), None);
    }
    #[test]
    fn ultimate_uses_missing_hp_and_three_level_brackets() {
        assert_eq!(missing_hp_damage(2000, 1000, 300), 300);
        assert_eq!(ult_ratio_tenths_pct(1), 300);
        assert_eq!(ult_ratio_tenths_pct(6), 375);
        assert_eq!(ult_ratio_tenths_pct(11), 450);
        assert_eq!(ult_nonhero_cap(11), 2000);
        assert_eq!(lethal_counter_damage(100, 2000, 1000, 1, true), 520);
        assert_eq!(lethal_counter_damage(1000, 10000, 0, 11, false), 2000);
        assert_eq!(lethal_counter_damage(100, 2000, 2000, 1, true), 220);
    }
    #[test]
    fn radius_math_is_stable() {
        assert_eq!(distance((0, 0), (3000, 4000)), 5000.0);
    }
}
