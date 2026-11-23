//! Native support for Lancelot's data-driven champion definition.
//!
//! Puncture's per-target mark check/reset and Soul Cutter damage scaling need
//! runtime logic; geometry, timing and visual assets remain data/atlas driven.

use mod_api_stable::*;

const SOUL_CUTTER_BUFF: &str = "mlbb_heroes_lancelot_soul_cutter";
const SWORD_MARK_BUFF: &str = "mlbb_heroes_lancelot_sword_mark";
const PUNCTURE_DASH_USED: &str = "mlbb_heroes_lancelot_puncture_used";
const PUNCTURE_MARK_VIEW: &str = "mlbb_lancelot_puncture_mark";
const PHANTOM_EXECUTION_VIEW: &str = "mlbb_lancelot_phantom_execution_fx";
const PHANTOM_EXECUTION_VIEW_REVERSE: &str = "mlbb_lancelot_phantom_execution_fx_reverse";
const THORNED_ROSE_SLOW: &str = "mlbb_heroes_lancelot_thorned_rose_slow";
const PUNCTURE_AI_LOCK: &str = "mlbb_heroes_lancelot_puncture_ai_lock";
const SKILL_RESET_EFFECT: &str = "mlbb_heroes:lancelot_puncture_reset";
const SOUL_CUTTER_DURATION: usize = 240; // 4s at 60 ticks/s
const PUNCTURE_DASH_USED_DURATION: usize = 60; // guard the single mark through the dash
const SWORD_MARK_DURATION: usize = 180; // 3s at 60 ticks/s
const THORNED_ROSE_SLOW_DURATION: usize = 30; // 0.5s at 60 ticks/s
const MAX_SOUL_CUTTER_STACKS: usize = 4;

fn named_buff_count(sim: &StableSim<'_>, entity_id: usize, name: &str) -> usize {
    let Some(entity) = sim.get_entity(entity_id) else {
        return 0;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .filter(|buff| buff.name() == name)
        .count()
}

fn has_buff(sim: &StableSim<'_>, entity_id: usize, name: &str) -> bool {
    named_buff_count(sim, entity_id, name) > 0
}

fn soul_stacks(sim: &StableSim<'_>, entity_id: usize) -> usize {
    named_buff_count(sim, entity_id, SOUL_CUTTER_BUFF).min(MAX_SOUL_CUTTER_STACKS)
}

/// Soul Cutter adds exactly 7.5% per stack; use tenths of a percent so the
/// passive's 7.5/15/22.5/30% progression is not rounded to a whole percent.
fn with_soul_cutter(raw: usize, stacks: usize) -> usize {
    raw.saturating_mul(1000 + 75 * stacks.min(MAX_SOUL_CUTTER_STACKS)) / 1000
}

fn level_scaled(base_at_level_one: usize, per_level: usize, level: usize) -> usize {
    base_at_level_one.saturating_add(per_level.saturating_mul(level.saturating_sub(1).min(14)))
}

/// Flat damage growth plus a percentage of the caster's total attack.
fn ad_skill_damage(level: usize, attack: usize, base: usize, growth: usize, ratio: usize) -> usize {
    level_scaled(base, growth, level).saturating_add(attack.saturating_mul(ratio) / 100)
}

pub fn register(module: &mut StableMod) {
    module.add_native_effect("mlbb_heroes:lancelot_soul_cutter_charge", SoulCutterCharge);
    module.add_native_effect("mlbb_heroes:lancelot_puncture_start", PunctureDashStart);
    module.add_native_effect("mlbb_heroes:lancelot_puncture_hit", PunctureHit);
    module.add_native_effect(SKILL_RESET_EFFECT, PunctureCooldownReset);
    module.add_native_effect("mlbb_heroes:lancelot_thorned_rose_hit", ThornedRoseHit);
    module.add_native_effect(
        "mlbb_heroes:lancelot_phantom_execution_hit",
        PhantomExecutionHit,
    );
    module.add_native_passive(
        "mlbb_heroes:lancelot_soul_cutter",
        LancelotSoulCutter::default(),
    );
    module.add_player_input_ai(LancelotCombatAi::default());
}

#[derive(Clone, Copy, Default)]
struct SoulCutterCharge;

impl StableEffectType for SoulCutterCharge {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        if !sim.get_entity(caster_id).is_some_and(|e| e.is_alive()) {
            return;
        }
        add_soul_cutter_stack(sim, caster_id);
    }
}

fn lancelot_base_stat(level: usize, stack: usize) -> StatV1 {
    let growth = level.saturating_sub(1);
    StatV1 {
        attack: 70 + 6 * growth,
        magic_power: 0,
        hp: 790 + 100 * growth,
        defence: 24 + 4 * growth,
        magic_resistance: 22 + 3 * growth,
        move_speed: 1150 + 5 * growth,
        hp_regen: 7,
        stack,
        crit_chance: 0,
    }
}

fn set_stack_counter(sim: &mut StableSim<'_>, entity_id: usize, stack: usize) {
    let level = sim.get_entity(entity_id).map_or(1, |e| e.level().max(1));
    let stat = lancelot_base_stat(level, stack.min(MAX_SOUL_CUTTER_STACKS));
    let _ = sim.entity_set_base_stat(entity_id, &stat);
}

fn add_soul_cutter_stack(sim: &mut StableSim<'_>, entity_id: usize) {
    let buff = BuffV1::timed(SOUL_CUTTER_BUFF, SOUL_CUTTER_DURATION);
    let stacks = sim.entity_stack_buff(entity_id, &buff, MAX_SOUL_CUTTER_STACKS, true);
    set_stack_counter(sim, entity_id, stacks);
}

#[derive(Clone, Copy, Default)]
struct PunctureDashStart;

impl StableEffectType for PunctureDashStart {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let _ = sim.entity_remove_buff(caster_id, PUNCTURE_DASH_USED);
        sim.add_buff(caster_id, &BuffV1::timed(PUNCTURE_AI_LOCK, 30));
        add_soul_cutter_stack(sim, caster_id);
    }
}

#[derive(Clone, Copy, Default)]
struct PunctureHit;

impl StableEffectType for PunctureHit {
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
        let target_id = input.target_id;
        let Some((level, attack, caster_alive, caster_team)) = sim
            .get_entity(caster_id)
            .map(|e| (e.level().max(1), e.stat().attack, e.is_alive(), e.team()))
        else {
            return;
        };
        let Some((target_alive, targetable, target_team)) = sim
            .get_entity(target_id)
            .map(|e| (e.is_alive(), e.is_targetable(), e.team()))
        else {
            return;
        };
        if !caster_alive || !target_alive || !targetable || caster_team == target_team {
            return;
        }

        let already_marked = has_buff(sim, target_id, SWORD_MARK_BUFF);
        let marked_this_dash = has_buff(sim, caster_id, PUNCTURE_DASH_USED);
        if !already_marked && !marked_this_dash {
            sim.add_buff(
                target_id,
                &BuffV1::timed(SWORD_MARK_BUFF, SWORD_MARK_DURATION),
            );
            sim.add_buff(
                caster_id,
                &BuffV1::timed(PUNCTURE_DASH_USED, PUNCTURE_DASH_USED_DURATION),
            );
            let target_input = InputTargetV1::target(target_id);
            let _ = sim.play_view_effect(PUNCTURE_MARK_VIEW, caster_id, &target_input, 0, 0, 0);
            // The first unmarked target is tagged even if the hit kills it. Defer
            // the reset one tick so the newly started Skill cooldown is present.
            let _ = sim.queue_effect(
                SKILL_RESET_EFFECT,
                AttackTypeV1::Skill,
                caster_id,
                &InputTargetV1::target(caster_id),
                1,
            );
        }

        let raw = ad_skill_damage(level, attack, 80, 7, 40);
        let damage = with_soul_cutter(raw, soul_stacks(sim, caster_id));
        sim.deal_damage(caster_id, target_id, damage, 0, AttackTypeV1::Skill);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (80 + caster_stat.attack * 40 / 100, 0)
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Default)]
struct PunctureCooldownReset;

impl StableEffectType for PunctureCooldownReset {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let _ = sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Skill, 10_000);
    }
}

fn thorned_rose_line_target_count(
    sim: &StableSim<'_>,
    caster_id: usize,
    target_id: usize,
    caster_team: usize,
) -> usize {
    let Some(caster) = sim.get_entity(caster_id) else {
        return 1;
    };
    let Some(target) = sim.get_entity(target_id) else {
        return 1;
    };
    let (cx, cy) = caster.pos();
    let (tx, ty) = target.pos();
    let dx = tx as f64 - cx as f64;
    let dy = ty as f64 - cy as f64;
    let length = (dx * dx + dy * dy).sqrt();
    if length < 1.0 {
        return 1;
    }
    let ux = dx / length;
    let uy = dy / length;
    let mut count = 0usize;
    for index in 0..sim.entity_count() {
        let Some(entity) = sim.entity_at(index) else {
            continue;
        };
        if entity.team() == caster_team
            || !entity.is_alive()
            || !entity.is_targetable()
            || entity.is_tower()
        {
            continue;
        }
        let (ex, ey) = entity.pos();
        let rx = ex as f64 - cx as f64;
        let ry = ey as f64 - cy as f64;
        let along = rx * ux + ry * uy;
        let across = (rx * uy - ry * ux).abs();
        let padding = entity.radius() as f64;
        if along >= -padding && along <= 64_000.0 + padding && across <= 21_000.0 + padding {
            count += 1;
        }
    }
    count.max(1)
}

#[derive(Clone, Copy, Default)]
struct ThornedRoseHit;

impl StableEffectType for ThornedRoseHit {
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
        let target_id = input.target_id;
        let Some((level, attack, alive, team)) = sim
            .get_entity(caster_id)
            .map(|e| (e.level().max(1), e.stat().attack, e.is_alive(), e.team()))
        else {
            return;
        };
        let Some((target_alive, targetable, target_team)) = sim
            .get_entity(target_id)
            .map(|e| (e.is_alive(), e.is_targetable(), e.team()))
        else {
            return;
        };
        if !alive || !target_alive || !targetable || team == target_team {
            return;
        }
        // Three delayed line waves are scheduled in .data_champion. TFM2 stats
        // do not split "extra physical attack" from base attack, so this uses
        // a conservative total-attack ratio. The crowd-hit penalty is estimated
        // against the same 42k-wide/64k-long line used by the data effect.
        let raw = ad_skill_damage(level, attack, 85, 5, 30);
        let hit_count = thorned_rose_line_target_count(sim, caster_id, target_id, team);
        let area_scaled = if hit_count > 1 {
            raw.saturating_mul(30) / 100
        } else {
            raw
        };
        let damage = with_soul_cutter(area_scaled, soul_stacks(sim, caster_id));
        sim.deal_damage(caster_id, target_id, damage, 0, AttackTypeV1::Skill);
        let mut slow = BuffV1::timed(THORNED_ROSE_SLOW, THORNED_ROSE_SLOW_DURATION);
        slow.move_speed_mult = -20;
        sim.add_buff(target_id, &slow);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (85 + caster_stat.attack * 30 / 100, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct PhantomExecutionHit;

impl StableEffectType for PhantomExecutionHit {
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
        let target_id = input.target_id;
        let Some((level, attack, alive, team)) = sim
            .get_entity(caster_id)
            .map(|e| (e.level().max(1), e.stat().attack, e.is_alive(), e.team()))
        else {
            return;
        };
        let Some((target_alive, targetable, target_team)) = sim
            .get_entity(target_id)
            .map(|e| (e.is_alive(), e.is_targetable(), e.team()))
        else {
            return;
        };
        if !alive || !target_alive || !targetable || team == target_team {
            return;
        }
        let raw = ad_skill_damage(level, attack, 100, 4, 100);
        let damage = with_soul_cutter(raw, soul_stacks(sim, caster_id));

        // Draw the directional slash at the reached target's world position, not
        // on the target's facing. Select a mirrored atlas from the caster-target axis
        // so the spear tip follows the dash direction on either side of the arena.
        if let (Some((cx, _)), Some((tx, ty))) = (
            sim.get_entity(caster_id).map(|entity| entity.pos()),
            sim.get_entity(target_id).map(|entity| entity.pos()),
        ) {
            let view_name = if tx >= cx {
                PHANTOM_EXECUTION_VIEW
            } else {
                PHANTOM_EXECUTION_VIEW_REVERSE
            };
            let position = InputTargetV1::pos(tx, ty);
            let _ = sim.play_view_effect(view_name, caster_id, &position, 0, 0, 0);
        }
        sim.deal_damage(caster_id, target_id, damage, 0, AttackTypeV1::Skill);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (100 + caster_stat.attack, 0)
    }
}

#[derive(Clone, Default)]
struct LancelotCombatAi {
    last_log_tick: Option<usize>,
    last_status_tick: Option<usize>,
}

impl LancelotCombatAi {
    fn log_cast(&mut self, tick: usize, action: &str, target_id: usize, prevalidated: bool) {
        if self
            .last_log_tick
            .is_none_or(|last| tick >= last.saturating_add(120))
        {
            self.last_log_tick = Some(tick);
            crate::wlog(format!(
                "Lancelot combat AI: issued {action} on target #{target_id}; is_valid_input={prevalidated} (local gates passed)"
            ));
        }
    }

    fn log_status(
        &mut self,
        tick: usize,
        level: usize,
        skill_cd: usize,
        skill2_cd: usize,
        ult_cd: usize,
        skill_target: Option<usize>,
        skill2_target: Option<usize>,
        ult_target: Option<usize>,
        skill_locked: bool,
    ) {
        if self
            .last_status_tick
            .is_none_or(|last| tick >= last.saturating_add(300))
        {
            self.last_status_tick = Some(tick);
            crate::wlog(format!(
                "Lancelot AI status: level={level} cd(skill={skill_cd}, skill2={skill2_cd}, ult={ult_cd}) targets(skill={skill_target:?}, skill2={skill2_target:?}, ult={ult_target:?}) puncture_lock={skill_locked}"
            ));
        }
    }
}

fn lancelot_ai_target(
    sim: &StableSim<'_>,
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    range: f64,
    champions_only: bool,
    preferred: Option<usize>,
) -> Option<usize> {
    let valid = |unit: &&crate::UnitSnap| {
        unit.id != caster.id
            && unit.team != caster.team
            && unit.alive
            && unit.targetable
            && !unit.tower
            && (!champions_only || unit.champion)
            && crate::distance(caster.pos, unit.pos) <= range
            && crate::is_unit_visible(sim, caster.team, unit)
    };
    if let Some(id) = preferred {
        if all.iter().find(|u| u.id == id).is_some_and(|u| valid(&u)) {
            return Some(id);
        }
    }
    all.iter()
        .filter(valid)
        .min_by(|a, b| {
            b.champion.cmp(&a.champion).then_with(|| {
                crate::distance(caster.pos, a.pos).total_cmp(&crate::distance(caster.pos, b.pos))
            })
        })
        .map(|unit| unit.id)
}

impl StablePlayerAi for LancelotCombatAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(self.clone())
    }

    fn id(&self) -> String {
        "mlbb_lancelot_targeted_combat_ai".to_string()
    }

    fn priority(&self) -> i32 {
        150
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name.to_ascii_lowercase().contains("lancelot")
    }

    fn think(
        &mut self,
        ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        let tick = ctx.tick();
        let player_id = ctx.player_id();
        let athlete_id = ctx.athlete_id();
        let preferred = base_input
            .filter(|input| input.target.kind == InputTargetKindV1::Target.code())
            .map(|input| input.target.target_id);
        let (
            level,
            skill_cd,
            skill2_cd,
            ult_cd,
            skill_target,
            skill2_target,
            ult_target,
            skill_locked,
        ) = {
            let sim = ctx.sim()?;
            let player = sim.get_player(player_id)?;
            let caster_id = player
                .champion()
                .map(|champ| champ.id())
                .unwrap_or(athlete_id);
            let cooldowns = player.cooldowns()?;
            let caster = sim.get_entity(caster_id)?;
            let level = (player.level() as usize).max(caster.level());
            let all = crate::units(&sim);
            let caster_snap = all.iter().find(|unit| unit.id == caster_id)?;
            let skill_locked = has_buff(&sim, caster_id, PUNCTURE_AI_LOCK);
            (
                level,
                cooldowns.1,
                cooldowns.2,
                cooldowns.3,
                lancelot_ai_target(&sim, &all, caster_snap, 90_000.0, false, preferred),
                lancelot_ai_target(&sim, &all, caster_snap, 64_000.0, false, preferred),
                lancelot_ai_target(&sim, &all, caster_snap, 120_000.0, true, preferred),
                skill_locked,
            )
        };
        self.log_status(
            tick,
            level,
            skill_cd,
            skill2_cd,
            ult_cd,
            skill_target,
            skill2_target,
            ult_target,
            skill_locked,
        );

        // Explicit target inputs keep the built-in AI from choosing an empty dash lane.
        if level >= 5 && ult_cd == 0 {
            if let Some(target_id) = ult_target {
                let input = InputV1::action(InputKindV1::Ult, InputTargetV1::target(target_id));
                let prevalidated = ctx.is_valid_input(&input);
                self.log_cast(tick, "Phantom Execution", target_id, prevalidated);
                // Like the tested Fanny hook, do not reject a locally-valid action just
                // because the host prevalidation says false; 0.6.x can under-report
                // mod-champion inputs. The game still validates the returned input.
                return Some(input);
            }
        }
        // Prioritize Puncture over Skill 2 when both are ready, so the level-1
        // dash is not starved by the lower-cooldown triple-wave skill.
        if !skill_locked && skill_cd == 0 {
            if let Some(target_id) = skill_target {
                let input = InputV1::action(InputKindV1::Skill, InputTargetV1::target(target_id));
                let prevalidated = ctx.is_valid_input(&input);
                self.log_cast(tick, "Puncture", target_id, prevalidated);
                return Some(input);
            }
        }
        if level >= 3 && skill2_cd == 0 {
            if let Some(target_id) = skill2_target {
                let input = InputV1::action(InputKindV1::Skill2, InputTargetV1::target(target_id));
                let prevalidated = ctx.is_valid_input(&input);
                self.log_cast(tick, "Thorned Rose", target_id, prevalidated);
                return Some(input);
            }
        }
        None
    }
}

#[derive(Clone, Default)]
struct LancelotSoulCutter {
    voice: crate::audio::VoiceLifecycle,
}

impl StablePassive for LancelotSoulCutter {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        self.voice
            .on_spawn(sim, entity_id, Some("mlbb_lancelot_voice_idle"));
        // Soul Cutter is temporary; clear it and its visible stack counter on respawn.
        let _ = sim.entity_remove_buff(entity_id, SOUL_CUTTER_BUFF);
        set_stack_counter(sim, entity_id, 0);
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _seed: u64, _player: usize, entity_id: usize) {
        self.voice.on_update(
            sim,
            entity_id,
            &["mlbb_lancelot_voice_idle"],
            &["mlbb_lancelot_voice_run"],
        );
    }

    fn on_attack(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity_id: usize,
        _target_id: usize,
        damage: &mut usize,
    ) {
        *damage = with_soul_cutter(*damage, soul_stacks(sim, entity_id));
    }
}

#[cfg(test)]
mod tests {
    use super::{ad_skill_damage, level_scaled, with_soul_cutter, MAX_SOUL_CUTTER_STACKS};

    #[test]
    fn soul_cutter_has_four_exact_tenths_percent_steps_and_a_cap() {
        assert_eq!(with_soul_cutter(1000, 0), 1000);
        assert_eq!(with_soul_cutter(1000, 1), 1075);
        assert_eq!(with_soul_cutter(1000, 2), 1150);
        assert_eq!(with_soul_cutter(1000, 3), 1225);
        assert_eq!(with_soul_cutter(1000, 4), 1300);
        assert_eq!(with_soul_cutter(1000, 99), 1300);
        assert_eq!(MAX_SOUL_CUTTER_STACKS, 4);
    }

    #[test]
    fn level_scaling_stays_inside_the_documented_hero_level_range() {
        assert_eq!(level_scaled(100, 10, 1), 100);
        assert_eq!(level_scaled(100, 10, 15), 240);
        assert_eq!(level_scaled(100, 10, 99), 240);
    }

    #[test]
    fn tuned_damage_coefficients_match_the_fanny_burst_benchmark() {
        // Level 1 and 15 Lancelot values use the project champion's current AD
        // progression (70 + 6/level). One Puncture and the three Thorned Rose
        // waves plus a two-stack Ultimate total about 1,260 raw damage at level 15.
        assert_eq!(ad_skill_damage(1, 70, 80, 7, 40), 108);
        assert_eq!(ad_skill_damage(15, 154, 80, 7, 40), 239);
        assert_eq!(ad_skill_damage(1, 70, 85, 5, 30), 106);
        assert_eq!(ad_skill_damage(15, 154, 85, 5, 30), 201);
        assert_eq!(ad_skill_damage(1, 70, 100, 4, 100), 170);
        assert_eq!(ad_skill_damage(15, 154, 100, 4, 100), 310);

        let puncture = with_soul_cutter(239, 1);
        let thorned_rose = with_soul_cutter(201 * 3, 1);
        let phantom_execution = with_soul_cutter(310, 2);
        assert_eq!(puncture + thorned_rose + phantom_execution, 1_260);
        // At level 15, Fanny's Cut Throat with two Prey Marks is 357 raw
        // (100 + 195% of 132 AD); Lancelot's two-stack Ultimate is within 1 point.
        let fanny_cut_throat = 100 + 132 * 195 / 100;
        assert_eq!(phantom_execution, 356);
        assert_eq!(fanny_cut_throat, 357);
        assert_eq!(fanny_cut_throat - phantom_execution, 1);
    }
}
