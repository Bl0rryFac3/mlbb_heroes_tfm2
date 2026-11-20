//! Uranus integration for the MLBB Heroes pack.
//!
//! Gameplay is deliberately expressed in TFM2-scale values. The source-game
//! timings and mark rules are preserved where the stable API allows them;
//! shield-layer break detection is limited to the aggregate shield value.
use mod_api_stable::*;

const MARK_PREFIX: &str = "mlbb_heroes_uranus_ionic_edge_mark_";
const MARK_VIEW_BUFFS: [&str; 4] = [
    "mlbb_heroes_uranus_ionic_edge_mark_1_2",
    "mlbb_heroes_uranus_ionic_edge_mark_3_4",
    "mlbb_heroes_uranus_ionic_edge_mark_5_6",
    "mlbb_heroes_uranus_ionic_edge_mark_7_8",
];
const MARK_TICKS: usize = 360; // 6 seconds; each stack expires independently.
const MAX_MARKS: usize = 8;
const URANUS_DAMAGE_SCALE_PERCENT: usize = 60; // S1/S2 raw damage only; not shields or sustain.
const S1_SLOW: &str = "mlbb_heroes_uranus_s1_slow";
const S2_SHIELD: &str = "mlbb_heroes_uranus_s2_shield";
const S2_SLOW: &str = "mlbb_heroes_uranus_s2_slow";
const ULT_BUFF: &str = "mlbb_heroes_uranus_ult";
const ULT_SPEED: &str = "mlbb_heroes_uranus_ult_speed";
const RADIANCE: &str = "mlbb_heroes_uranus_radiance";
const MAX_RADIANCE: usize = 12;
const RADIANCE_TICKS: usize = 600; // 10 seconds per stack.
const RADIANCE_HEAL_TICKS: usize = 48; // 0.8 seconds.
/// Minimum gap between Radiance stack grants.
///
/// Without this, `on_damaged` granted a stack for *every* instance of damage,
/// so a damage-over-time kit (Gord's field, Hanabi's bounces) filled Uranus to
/// the cap in a fraction of a second and he was unkillable against exactly the
/// comps that should counter him. One stack per second means he ramps into a
/// fight instead of being armed before it starts.
///
/// Note this only bites because stacks expire: at 60 ticks the sustainable
/// count is `RADIANCE_TICKS / RADIANCE_GAIN_TICKS` = 10, below the cap of 12,
/// so the cooldown — not the cap — is what sets his steady-state healing.
const RADIANCE_GAIN_TICKS: usize = 60; // 1 second.
/// The stack count `radiance_heal_at_twenty` is calibrated against.
///
/// Deliberately NOT [`MAX_RADIANCE`]. The two were the same constant, which
/// meant lowering the cap also shrank the divisor and *raised* the heal per
/// stack — capping at 12 left the top-end heal at exactly the old 224/0.8 s.
/// The heal curve is authored against a 20-stack reference, so the divisor has
/// to stay at 20 for the cap to actually reduce anything.
const RADIANCE_HEAL_DIVISOR: usize = 20;
const S2_SHIELD_TICKS: usize = 240; // 4 seconds.
const ULT_TICKS: usize = 480; // 8 seconds.
const ULT_SPEED_START: usize = 60;
const S2_BURST_RADIUS: f64 = 42_000.0;
const S2_DASH_MAX_RANGE: f64 = 90_000.0;

fn mark_buff_name(caster_id: usize) -> String {
    format!("{MARK_PREFIX}{caster_id}")
}

fn s1_cast_mark_name(caster_id: usize) -> String {
    format!("mlbb_heroes_uranus_s1_marked_this_cast_{caster_id}")
}

fn s2_path_hit_name(caster_id: usize) -> String {
    format!("mlbb_heroes_uranus_s2_path_hit_{caster_id}")
}

fn named_buffs(sim: &StableSim<'_>, entity_id: usize, name: &str) -> Vec<BuffV1> {
    let Some(entity) = sim.get_entity(entity_id) else {
        return Vec::new();
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .filter(|buff| buff.name() == name)
        .collect()
}

fn buff_count(sim: &StableSim<'_>, entity_id: usize, name: &str) -> usize {
    named_buffs(sim, entity_id, name).len()
}

fn has_buff(sim: &StableSim<'_>, entity_id: usize, name: &str) -> bool {
    buff_count(sim, entity_id, name) > 0
}

fn remove_buff(sim: &mut StableSim<'_>, entity_id: usize, name: &str) {
    let _ = sim.entity_remove_buff(entity_id, name);
}

fn enemy_target(sim: &StableSim<'_>, caster_id: usize, target_id: usize) -> bool {
    let Some(caster) = sim.get_entity(caster_id) else {
        return false;
    };
    let Some(target) = sim.get_entity(target_id) else {
        return false;
    };
    target.is_alive()
        && target.is_targetable()
        && target.team() != caster.team()
        && !matches!(target.kind(), Some(EntityKindV1::Nexus))
}

const TFM2_MAX_LEVEL: usize = 12;

/// Linearly maps a Fandom min/max value onto TFM2 levels 1–12, rounded to nearest.
fn level_lerp(start: usize, end: usize, level: usize) -> usize {
    let step = level.clamp(1, TFM2_MAX_LEVEL) - 1;
    let span = TFM2_MAX_LEVEL - 1;
    (start * (span - step) + end * step + span / 2) / span
}

fn s1_damage(level: usize, magic_power: usize, marks: usize) -> usize {
    // S1/S2 direct damage is tuned to 60% of the source formula; each Ionic Mark remains +20%.
    let raw =
        (level_lerp(133, 219, level) + magic_power * 30 / 100) * URANUS_DAMAGE_SCALE_PERCENT / 100;
    raw * (100 + 20 * marks.min(MAX_MARKS)) / 100
}

fn s1_ai_damage(magic_power: usize) -> usize {
    2 * s1_damage(6, magic_power, 0)
}

fn ult_ai_value(stat: &StatV1) -> usize {
    // Positive AI value proxy for the source-scaled heal, movement burst, and Radiance grant.
    200 + stat.hp / 20 + stat.magic_power / 5
}

fn s2_path_damage(level: usize, magic_power: usize) -> usize {
    (level_lerp(80, 200, level) + magic_power * 20 / 100) * URANUS_DAMAGE_SCALE_PERCENT / 100
}

fn s2_arrival_damage(level: usize, magic_power: usize) -> usize {
    (level_lerp(160, 400, level) + magic_power * 80 / 100) * URANUS_DAMAGE_SCALE_PERCENT / 100
}

fn s2_burst_damage(level: usize, magic_power: usize) -> usize {
    (level_lerp(260, 650, level) + magic_power * 130 / 100) * URANUS_DAMAGE_SCALE_PERCENT / 100
}

fn s2_shield_amount(level: usize, magic_power: usize) -> usize {
    level_lerp(300, 750, level) + magic_power * 200 / 100
}

fn ult_heal(level: usize) -> usize {
    level_lerp(200, 400, level)
}

fn ult_radiance_stacks(level: usize) -> usize {
    level_lerp(5, 15, level)
}

fn radiance_heal_at_twenty(level: usize) -> usize {
    level_lerp(48, 224, level)
}

fn mark_view_state(sim: &StableSim<'_>, target_id: usize) -> usize {
    MARK_VIEW_BUFFS
        .iter()
        .position(|name| has_buff(sim, target_id, name))
        .map_or(0, |index| index + 1)
}

fn set_mark_view(sim: &mut StableSim<'_>, target_id: usize, state: usize, duration: usize) {
    for name in MARK_VIEW_BUFFS {
        remove_buff(sim, target_id, name);
    }
    if state > 0 && duration > 0 {
        let index = state.saturating_sub(1).min(MARK_VIEW_BUFFS.len() - 1);
        sim.add_buff(target_id, &BuffV1::timed(MARK_VIEW_BUFFS[index], duration));
    }
}

fn sync_mark_views(sim: &mut StableSim<'_>, caster_id: usize) {
    let name = mark_buff_name(caster_id);
    let target_ids: Vec<usize> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i).map(|entity| entity.id()))
        .collect();
    for target_id in target_ids {
        let stacks = named_buffs(sim, target_id, &name);
        if stacks.is_empty() {
            if mark_view_state(sim, target_id) != 0 {
                set_mark_view(sim, target_id, 0, 0);
            }
            continue;
        }
        let count = stacks.len().min(MAX_MARKS);
        let state = (count + 1) / 2; // one supplied visual state per pair of marks.
        let duration = stacks
            .iter()
            .map(|buff| buff.duration_tick)
            .max()
            .unwrap_or(1);
        if mark_view_state(sim, target_id) != state {
            set_mark_view(sim, target_id, state, duration.max(1));
        }
    }
}

fn apply_slow(
    sim: &mut StableSim<'_>,
    target_id: usize,
    name: &str,
    percent: i32,
    duration: usize,
) {
    remove_buff(sim, target_id, name);
    let mut slow = BuffV1::timed(name, duration);
    slow.move_speed_mult = -percent;
    sim.add_buff(target_id, &slow);
}

fn ionic_mark_kind_allowed(kind: Option<EntityKindV1>) -> bool {
    !matches!(kind, Some(EntityKindV1::Tower | EntityKindV1::Nexus))
}

fn apply_ionic_mark(sim: &mut StableSim<'_>, caster_id: usize, target_id: usize) {
    let cast_mark = s1_cast_mark_name(caster_id);
    let kind_allowed = sim
        .get_entity(target_id)
        .is_some_and(|target| ionic_mark_kind_allowed(target.kind()));
    if !enemy_target(sim, caster_id, target_id)
        || !kind_allowed
        || has_buff(sim, target_id, &cast_mark)
    {
        return;
    }
    let name = mark_buff_name(caster_id);
    let mark = BuffV1::timed(&name, MARK_TICKS);
    let count = sim
        .entity_stack_buff(target_id, &mark, MAX_MARKS, false)
        .min(MAX_MARKS);
    if count > 0 {
        sim.add_buff(target_id, &BuffV1::timed(&cast_mark, 30));
        let duration = named_buffs(sim, target_id, &name)
            .iter()
            .map(|buff| buff.duration_tick)
            .max()
            .unwrap_or(MARK_TICKS);
        set_mark_view(sim, target_id, (count + 1) / 2, duration.max(1));
    }
}

fn detonate_skill2_shield(sim: &mut StableSim<'_>, caster_id: usize) {
    let Some((team, center, level, magic_power)) = sim.get_entity(caster_id).map(|caster| {
        let (x, y) = caster.pos();
        (
            caster.team(),
            (x, y),
            caster.level().max(1),
            caster.stat().magic_power,
        )
    }) else {
        return;
    };
    let ids: Vec<usize> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i).map(|target| target.id()))
        .collect();
    let radius_sq = (S2_BURST_RADIUS * S2_BURST_RADIUS) as u128;
    for target_id in ids {
        let Some(target) = sim.get_entity(target_id) else {
            continue;
        };
        let (x, y) = target.pos();
        let dx = center.0 as i128 - x as i128;
        let dy = center.1 as i128 - y as i128;
        let valid = target.is_alive()
            && target.is_targetable()
            && target.team() != team
            && !matches!(target.kind(), Some(EntityKindV1::Nexus))
            && (dx * dx + dy * dy) as u128 <= radius_sq;
        drop(target);
        if valid {
            sim.deal_damage(
                caster_id,
                target_id,
                0,
                s2_burst_damage(level, magic_power),
                AttackTypeV1::Skill,
            );
        }
    }
    let burst_input = InputTargetV1::pos(center.0, center.1);
    let _ = sim.play_view_effect(
        "uranus_skill2_shield_burst",
        caster_id,
        &burst_input,
        0,
        0,
        24,
    );
}

#[derive(Clone, Copy, Default)]
struct UranusBasicAttack;
impl StableEffectType for UranusBasicAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !enemy_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let attack = sim
            .get_entity(caster_id)
            .map_or(0, |caster| caster.stat().attack);
        // Uranus's basic is a physical claw/arm swipe, never an Ionic Mark source.
        sim.deal_damage(
            caster_id,
            input.target_id,
            attack,
            0,
            AttackTypeV1::BaseAttack,
        );
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack, 0)
    }
}

/// Explicit AI estimate for an area skill whose actual hit effects are delayed.
#[derive(Clone, Copy, Default)]
struct UranusSkill1AiHint;
impl StableEffectType for UranusSkill1AiHint {
    fn apply(
        &self,
        _sim: &mut StableSim<'_>,
        _seed: u64,
        _caster_id: usize,
        _input: InputTargetV1,
    ) {
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, s1_ai_damage(stat.magic_power))
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Default)]
struct UranusSkill1Hit;
impl StableEffectType for UranusSkill1Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !enemy_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let target_id = input.target_id;
        let mark_name = mark_buff_name(caster_id);
        let Some((level, magic_power)) = sim
            .get_entity(caster_id)
            .map(|caster| (caster.level().max(1), caster.stat().magic_power))
        else {
            return;
        };
        let existing_marks = buff_count(sim, target_id, &mark_name).min(MAX_MARKS);
        // Both blades in one cast use the mark count from before that cast.
        let cast_mark = s1_cast_mark_name(caster_id);
        let same_cast_mark = has_buff(sim, target_id, &cast_mark);
        let damage_marks = existing_marks.saturating_sub(usize::from(same_cast_mark));
        sim.deal_damage(
            caster_id,
            target_id,
            0,
            s1_damage(level, magic_power, damage_marks),
            AttackTypeV1::Skill,
        );
        apply_slow(sim, target_id, S1_SLOW, 30, 120);
        apply_ionic_mark(sim, caster_id, target_id);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, s1_damage(1, stat.magic_power, 0))
    }
}

#[derive(Clone, Copy, Default)]
struct UranusSkill2Start;
impl StableEffectType for UranusSkill2Start {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let Some((start, level, magic_power)) = sim
            .get_entity(caster_id)
            .filter(|caster| caster.is_alive())
            .map(|caster| {
                let (x, y) = caster.pos();
                (
                    (x as f64, y as f64),
                    caster.level().max(1),
                    caster.stat().magic_power,
                )
            })
        else {
            return;
        };
        let mut shield = s2_shield_amount(level, magic_power);
        if has_buff(sim, caster_id, ULT_BUFF) {
            shield = shield * 120 / 100;
        }
        remove_buff(sim, caster_id, S2_SHIELD);
        let _ = sim.entity_add_shield(caster_id, shield.max(1), S2_SHIELD_TICKS);
        sim.add_buff(caster_id, &BuffV1::timed(S2_SHIELD, S2_SHIELD_TICKS));

        // Stamp a faint, short-lived trail along the selected dash path. These
        // are world effects, not a flying projectile and not a follow-caster dome.
        let aim = match input.kind {
            kind if kind == InputTargetKindV1::Target.code() => {
                sim.get_entity(input.target_id).map(|target| {
                    let (x, y) = target.pos();
                    (x as f64, y as f64)
                })
            }
            kind if kind == InputTargetKindV1::Pos.code() => Some((input.x as f64, input.y as f64)),
            kind if kind == InputTargetKindV1::Dir.code() => {
                let dx = input.dir_x as f64;
                let dy = input.dir_y as f64;
                let length = dx.hypot(dy);
                (length > 0.0).then(|| {
                    (
                        start.0 + dx / length * S2_DASH_MAX_RANGE,
                        start.1 + dy / length * S2_DASH_MAX_RANGE,
                    )
                })
            }
            _ => None,
        };
        if let Some(aim) = aim {
            let dx = aim.0 - start.0;
            let dy = aim.1 - start.1;
            let length = dx.hypot(dy);
            if length > 1.0 {
                let travel = length.min(S2_DASH_MAX_RANGE);
                let end = (
                    start.0 + dx / length * travel,
                    start.1 + dy / length * travel,
                );
                for fraction in [0.15, 0.35, 0.55, 0.75, 0.95] {
                    let at = InputTargetV1::pos(
                        (start.0 + (end.0 - start.0) * fraction).round().max(0.0) as u64,
                        (start.1 + (end.1 - start.1) * fraction).round().max(0.0) as u64,
                    );
                    let _ =
                        sim.play_view_effect("uranus_skill2_dash_trail", caster_id, &at, 0, 0, 16);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct UranusSkill2PathHit;
impl StableEffectType for UranusSkill2PathHit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let path_hit = s2_path_hit_name(caster_id);
        if input.kind != InputTargetKindV1::Target.code()
            || !enemy_target(sim, caster_id, input.target_id)
            || has_buff(sim, input.target_id, &path_hit)
        {
            return;
        }
        let target_id = input.target_id;
        let Some((level, magic_power)) = sim
            .get_entity(caster_id)
            .map(|caster| (caster.level().max(1), caster.stat().magic_power))
        else {
            return;
        };
        sim.add_buff(target_id, &BuffV1::timed(&path_hit, 24));
        sim.deal_damage(
            caster_id,
            target_id,
            0,
            s2_path_damage(level, magic_power),
            AttackTypeV1::Skill,
        );
        apply_slow(sim, target_id, S2_SLOW, 25, 60);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, s2_path_damage(1, stat.magic_power))
    }
}

#[derive(Clone, Copy, Default)]
struct UranusSkill2ArrivalHit;
impl StableEffectType for UranusSkill2ArrivalHit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !enemy_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let Some((level, magic_power)) = sim
            .get_entity(caster_id)
            .map(|caster| (caster.level().max(1), caster.stat().magic_power))
        else {
            return;
        };
        sim.deal_damage(
            caster_id,
            input.target_id,
            0,
            s2_arrival_damage(level, magic_power),
            AttackTypeV1::Skill,
        );
        apply_slow(sim, input.target_id, S2_SLOW, 25, 60);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, s2_arrival_damage(1, stat.magic_power))
    }
}

#[derive(Clone, Copy, Default)]
struct UranusUltimate;
impl StableEffectType for UranusUltimate {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, _input: InputTargetV1) {
        let Some((max_hp, level, alive)) = sim
            .get_entity(caster_id)
            .map(|caster| (caster.hp().1, caster.level().max(1), caster.is_alive()))
        else {
            return;
        };
        if !alive {
            return;
        }
        // Current stable API clears all CC, not slow-only; this is the user-approved translation.
        let _ = sim.entity_clear_cc(caster_id);
        let _ = sim.heal(caster_id, caster_id, ult_heal(level).min(max_hp).max(1));
        let stacks = ult_radiance_stacks(level);
        for _ in 0..stacks {
            let _ = sim.entity_stack_buff(
                caster_id,
                &BuffV1::timed(RADIANCE, RADIANCE_TICKS),
                MAX_RADIANCE,
                false,
            );
        }
        remove_buff(sim, caster_id, ULT_BUFF);
        let hp_regen = sim
            .get_entity(caster_id)
            .map_or(0, |caster| caster.stat().hp_regen * 20 / 100);
        let mut ult_buff = BuffV1::timed(ULT_BUFF, ULT_TICKS);
        ult_buff.hp_regen = hp_regen as i32;
        sim.add_buff(caster_id, &ult_buff);
        let _ = sim.play_view_effect(
            "uranus_ultimate_flare",
            caster_id,
            &InputTargetV1::target(caster_id),
            0,
            0,
            30,
        );
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, ult_ai_value(stat))
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Default)]
struct UranusPassive {
    voice: crate::audio::VoiceLifecycle,
    next_radiance_heal: usize,
    next_radiance_stack: usize,
    shield_armed: bool,
}
impl StablePassive for UranusPassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        self.voice.on_spawn(sim, entity_id, None);
        self.next_radiance_heal = sim.tick().saturating_add(RADIANCE_HEAL_TICKS);
        self.next_radiance_stack = sim.tick();
        self.shield_armed = false;
        let own_mark = mark_buff_name(entity_id);
        let ids: Vec<usize> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i).map(|entity| entity.id()))
            .collect();
        for id in ids {
            remove_buff(sim, id, &own_mark);
            for view_name in MARK_VIEW_BUFFS {
                remove_buff(sim, id, view_name);
            }
        }
        for name in [RADIANCE, S2_SHIELD, ULT_BUFF, ULT_SPEED] {
            remove_buff(sim, entity_id, name);
        }
    }

    fn on_damaged(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity_id: usize,
        _attacker: usize,
        damage: usize,
    ) {
        if damage == 0
            || !sim
                .get_entity(entity_id)
                .is_some_and(|entity| entity.is_alive())
        {
            return;
        }
        // Rate-limited: at most one stack per RADIANCE_GAIN_TICKS no matter how
        // many separate damage instances land in that window.
        let now = sim.tick();
        if now < self.next_radiance_stack {
            return;
        }
        self.next_radiance_stack = now.saturating_add(RADIANCE_GAIN_TICKS);
        let _ = sim.entity_stack_buff(
            entity_id,
            &BuffV1::timed(RADIANCE, RADIANCE_TICKS),
            MAX_RADIANCE,
            false,
        );
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _seed: u64, _player: usize, entity_id: usize) {
        let now = sim.tick();
        let Some((alive, shield)) = sim
            .get_entity(entity_id)
            .map(|entity| (entity.is_alive(), entity.shield()))
        else {
            return;
        };
        if !alive {
            return;
        }

        let radiance = buff_count(sim, entity_id, RADIANCE).min(MAX_RADIANCE);
        if now >= self.next_radiance_heal {
            self.next_radiance_heal = now.saturating_add(RADIANCE_HEAL_TICKS);
            if radiance > 0 {
                let max_stack_heal = radiance_heal_at_twenty(
                    sim.get_entity(entity_id)
                        .map_or(1, |entity| entity.level().max(1)),
                );
                let mut heal = max_stack_heal.saturating_mul(radiance) / RADIANCE_HEAL_DIVISOR;
                if has_buff(sim, entity_id, ULT_BUFF) {
                    heal = heal * 120 / 100;
                }
                if heal > 0 {
                    let _ = sim.heal(entity_id, entity_id, heal);
                }
            }
        }

        sync_mark_views(sim, entity_id);

        if has_buff(sim, entity_id, S2_SHIELD) {
            self.shield_armed = true;
            if shield == 0 {
                remove_buff(sim, entity_id, S2_SHIELD);
                self.shield_armed = false;
                detonate_skill2_shield(sim, entity_id);
            }
        } else if self.shield_armed {
            self.shield_armed = false;
            detonate_skill2_shield(sim, entity_id);
        }

        if let Some(remaining) = named_buffs(sim, entity_id, ULT_BUFF)
            .iter()
            .map(|buff| buff.duration_tick)
            .max()
        {
            let speed = ULT_SPEED_START.saturating_mul(remaining.min(ULT_TICKS)) / ULT_TICKS;
            remove_buff(sim, entity_id, ULT_SPEED);
            if speed > 0 {
                let mut speed_buff = BuffV1::timed(ULT_SPEED, 2);
                speed_buff.move_speed_mult = speed as i32;
                sim.add_buff(entity_id, &speed_buff);
            }
        } else {
            remove_buff(sim, entity_id, ULT_SPEED);
        }
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        self.voice.on_dead(
            sim,
            player,
            &[
                "mlbb_uranus_voice_death01",
                "mlbb_uranus_voice_death02",
                "mlbb_uranus_voice_death03",
            ],
        );
        self.shield_armed = false;
    }
}

pub fn register(module: &mut StableMod) {
    module.add_native_effect("mlbb_heroes:uranus_basic_attack", UranusBasicAttack);
    module.add_native_effect("mlbb_heroes:uranus_skill1_ai_hint", UranusSkill1AiHint);
    module.add_native_effect("mlbb_heroes:uranus_skill1_hit", UranusSkill1Hit);
    module.add_native_effect("mlbb_heroes:uranus_skill2_start", UranusSkill2Start);
    module.add_native_effect("mlbb_heroes:uranus_skill2_path_hit", UranusSkill2PathHit);
    module.add_native_effect(
        "mlbb_heroes:uranus_skill2_arrival_hit",
        UranusSkill2ArrivalHit,
    );
    module.add_native_effect("mlbb_heroes:uranus_ultimate", UranusUltimate);
    module.add_native_passive("mlbb_heroes:uranus_radiance", UranusPassive::default());
}

#[cfg(test)]
mod tests {
    use super::{
        ionic_mark_kind_allowed, level_lerp, radiance_heal_at_twenty, s1_ai_damage, s1_damage,
        s2_arrival_damage, s2_burst_damage, s2_path_damage, s2_shield_amount, ult_ai_value,
        ult_heal, ult_radiance_stacks, MAX_RADIANCE, RADIANCE_GAIN_TICKS, RADIANCE_HEAL_DIVISOR,
        RADIANCE_TICKS,
    };
    use mod_api_stable::StatV1;

    #[test]
    fn ionic_edge_uses_tuned_damage_and_caps_mark_bonus() {
        assert_eq!(s1_damage(1, 0, 0), 79);
        assert_eq!(s1_damage(12, 0, 0), 131);
        assert_eq!(s1_damage(1, 0, 1), 94);
        assert_eq!(s1_damage(1, 0, 8), 205);
        assert_eq!(s1_damage(1, 0, 99), 205);
        assert_eq!(s1_damage(99, 100, 0), 149);
    }

    /// The heal curve is authored against a TWENTY-stack reference, so the
    /// divisor must stay at 20 no matter what the cap is.
    ///
    /// Regression: cap and divisor were the same constant. Lowering the cap
    /// 20 -> 12 therefore shrank the divisor too, and the top-end heal stayed
    /// at the old 224/0.8 s — the nerf silently did nothing.
    #[test]
    fn radiance_heal_divisor_is_independent_of_the_cap() {
        assert_eq!(
            RADIANCE_HEAL_DIVISOR, 20,
            "the heal curve is calibrated at 20 stacks; changing this rescales every heal"
        );
        assert!(
            RADIANCE_HEAL_DIVISOR >= MAX_RADIANCE,
            "divisor {RADIANCE_HEAL_DIVISOR} must be >= cap {MAX_RADIANCE} or the cap stops being reachable"
        );
        // Capping must actually reduce the peak heal.
        let at_cap = radiance_heal_at_twenty(15) * MAX_RADIANCE / RADIANCE_HEAL_DIVISOR;
        let at_twenty = radiance_heal_at_twenty(15);
        assert!(
            at_cap < at_twenty,
            "heal at the cap ({at_cap}) must be below the 20-stack reference ({at_twenty})"
        );
        // Sustainable stacks are set by the gain cooldown, not the cap:
        // RADIANCE_TICKS / RADIANCE_GAIN_TICKS = 600/60 = 10.
        let sustainable = (RADIANCE_TICKS / RADIANCE_GAIN_TICKS).min(MAX_RADIANCE);
        assert_eq!(sustainable, 10);
        let per_tick = radiance_heal_at_twenty(15) * sustainable / RADIANCE_HEAL_DIVISOR;
        assert_eq!(per_tick, 112, "sustained heal per 0.8 s tick");
        assert_eq!(per_tick * 60 / 48, 140, "sustained HP/sec");
    }

    #[test]
    fn ionic_marks_skip_towers_and_nexus_but_keep_other_enemy_kinds() {
        use mod_api_stable::EntityKindV1;
        assert!(!ionic_mark_kind_allowed(Some(EntityKindV1::Tower)));
        assert!(!ionic_mark_kind_allowed(Some(EntityKindV1::Nexus)));
        assert!(ionic_mark_kind_allowed(Some(EntityKindV1::Champion)));
        assert!(ionic_mark_kind_allowed(Some(EntityKindV1::Minion)));
    }

    #[test]
    fn ward_damage_is_tuned_while_shield_values_stay_unchanged() {
        assert_eq!(s2_path_damage(1, 25), 51);
        assert_eq!(s2_path_damage(12, 25), 123);
        assert_eq!(s2_arrival_damage(1, 25), 108);
        assert_eq!(s2_arrival_damage(12, 25), 252);
        assert_eq!(s2_burst_damage(1, 25), 175);
        assert_eq!(s2_burst_damage(12, 25), 409);
        assert_eq!(s2_shield_amount(1, 25), 350);
        assert_eq!(s2_shield_amount(12, 25), 800);
    }

    #[test]
    fn radiance_and_consecration_interpolate_fandom_endpoints() {
        assert_eq!(radiance_heal_at_twenty(1), 48);
        assert_eq!(radiance_heal_at_twenty(12), 224);
        assert_eq!(ult_heal(1), 200);
        assert_eq!(ult_heal(12), 400);
        assert_eq!(ult_radiance_stacks(1), 5);
        assert_eq!(ult_radiance_stacks(6), 10);
        assert_eq!(ult_radiance_stacks(12), 15);
    }

    #[test]
    fn level_interpolation_clamps_at_tfm2_endpoints() {
        assert_eq!(level_lerp(100, 200, 0), 100);
        assert_eq!(level_lerp(100, 200, 99), 200);
    }

    #[test]
    fn target_skill_ai_hints_expose_positive_value() {
        assert_eq!(s1_ai_damage(25), 214);
        let stat = StatV1 {
            hp: 1_000,
            magic_power: 50,
            ..StatV1::default()
        };
        assert_eq!(ult_ai_value(&stat), 260);
    }
}
