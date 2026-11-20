//! TFM2-balanced Karrie kit using the supplied Lost Star sprites and Lightwheel VFX.
//! Visual/playback wiring is prepared in the asset tree; this gameplay draft still
//! requires in-match verification before any behavior is described as tested.
use mod_api_stable::*;

const ULT_BUFF: &str = "mlbb_heroes_karrie_dual_wield";
const MARK_PREFIX: &str = "mlbb_heroes_karrie_lightwheel_mark_";
const MARK_DURATION: usize = 300; // 5s
const ULT_DURATION: usize = 360; // 6s
const MAX_MARKS: usize = 5;
const S1_RANGE: f64 = 95_000.0;
const S1_RADIUS: f64 = 36_000.0;
const S1_ZONE_DURATION: usize = 90; // 1.5s total linger
const S1_ZONE_VISUAL_REFRESH_TICKS: usize = 30; // redraw at impact and each half-second pulse
const S1_ZONE_TICK_DELAYS: [usize; 2] = [30, 60]; // two low-damage pulses at 0.5s and 1.0s
const S1_ZONE_SLOW_DURATION: usize = 35; // refreshes just beyond each 0.5s pulse
const S1_ZONE_DAMAGE_FLAT: usize = 10;
const S1_ZONE_DAMAGE_AD_PERCENT: usize = 10;
const S2_DASH_DISTANCE: f64 = 42_000.0;
const S2_ACQUIRE_RANGE: f64 = 86_000.0;
const PROJECTILE_SPEED: usize = 6_000;
const BASIC_PROJECTILE: &str = "karrie_lightwheel";
const S1_PROJECTILE: &str = "karrie_skill1_wheel";
const S1_IMPACT: &str = "karrie_skill1_impact";
const S1_SLOW_ZONE: &str = "karrie_skill1_slow_zone";
const ULT_PULSE: &str = "karrie_ult_activation_pulse";
const PASSIVE_PROC: &str = "karrie_lightwheel_proc";
const MARK_VIEW_BUFFS: [&str; 6] = [
    "mlbb_heroes_karrie_lightwheel_mark_1",
    "mlbb_heroes_karrie_lightwheel_mark_2",
    "mlbb_heroes_karrie_lightwheel_mark_3",
    "mlbb_heroes_karrie_lightwheel_mark_4",
    "mlbb_heroes_karrie_lightwheel_mark_5",
    "mlbb_heroes_karrie_lightwheel_mark_ready",
];

fn mark_buff_name(caster_id: usize) -> String {
    format!("{MARK_PREFIX}{caster_id}")
}

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

fn is_enemy_target(sim: &StableSim<'_>, caster_id: usize, target_id: usize) -> bool {
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

fn distance(a: (u64, u64), b: (u64, u64)) -> f64 {
    let dx = a.0 as f64 - b.0 as f64;
    let dy = a.1 as f64 - b.1 as f64;
    dx.hypot(dy)
}

fn unit(dx: f64, dy: f64) -> (f64, f64) {
    let len = dx.hypot(dy);
    if !len.is_finite() || len < 1.0 {
        (1.0, 0.0)
    } else {
        (dx / len, dy / len)
    }
}

fn pos_u64(v: f64) -> u64 {
    v.round().clamp(0.0, u64::MAX as f64) as u64
}

fn aim_point(sim: &StableSim<'_>, caster_id: usize, input: InputTargetV1) -> Option<(f64, f64)> {
    match input.kind {
        k if k == InputTargetKindV1::Target.code() => sim.get_entity(input.target_id).map(|e| {
            let (x, y) = e.pos();
            (x as f64, y as f64)
        }),
        k if k == InputTargetKindV1::Pos.code() => Some((input.x as f64, input.y as f64)),
        k if k == InputTargetKindV1::Dir.code() => {
            let (x, y) = sim.get_entity(caster_id)?.pos();
            let d = unit(input.dir_x as f64, input.dir_y as f64);
            Some((x as f64 + d.0 * S1_RANGE, y as f64 + d.1 * S1_RANGE))
        }
        _ => None,
    }
}

fn projectile_spec(
    caster_id: usize,
    team: usize,
    start: (u64, u64),
    end: (f64, f64),
    speed: usize,
    radius: usize,
    target_id: Option<usize>,
    attack_type: AttackTypeV1,
) -> ProjectileSpawnV1 {
    let mut spec = ProjectileSpawnV1 {
        caster_id,
        team,
        x: start.0,
        y: start.1,
        radius: radius as u64,
        speed: speed as u64,
        attack_type: attack_type.code(),
        casting_type: CastingTypeV1::Targeting.code(),
        casting_target: CastingTargetV1::Enemy.code(),
        target_x: pos_u64(end.0),
        target_y: pos_u64(end.1),
        ..ProjectileSpawnV1::default()
    };
    if let Some(id) = target_id {
        spec.move_kind = ProjectileMoveKindV1::Target.code();
        spec.target_id = id;
    } else {
        spec.move_kind = ProjectileMoveKindV1::Linear.code();
    }
    spec
}

fn set_mark_view(sim: &mut StableSim<'_>, target_id: usize, state: usize, duration: usize) {
    for name in MARK_VIEW_BUFFS {
        let _ = sim.entity_remove_buff(target_id, name);
    }
    if state == 0 {
        return;
    }
    let index = state.saturating_sub(1).min(MARK_VIEW_BUFFS.len() - 1);
    let buff = BuffV1::timed(MARK_VIEW_BUFFS[index], duration);
    sim.add_buff(target_id, &buff);
}

/// One per-caster mark stack on a victim. The fifth hit converts it into a
/// level-scaled max-HP true-damage Lightwheel proc, then clears that caster's mark.
fn lightwheel_proc_ready(stacks: usize) -> bool {
    stacks >= MAX_MARKS
}

fn lightwheel_ratio_percent(level: usize) -> usize {
    // TFM2 levels cap at 12; scale the tank-killer proc from 8% to 12%.
    8 + 4 * level.saturating_sub(1).min(11) / 11
}

fn lightwheel_true_damage(max_hp: usize, level: usize, cap: Option<usize>) -> usize {
    let damage = max_hp.saturating_mul(lightwheel_ratio_percent(level)) / 100;
    cap.map_or(damage, |limit| damage.min(limit))
}

fn apply_lightwheel_mark(sim: &mut StableSim<'_>, caster_id: usize, target_id: usize) {
    if !is_enemy_target(sim, caster_id, target_id) {
        return;
    }
    let name = mark_buff_name(caster_id);
    let buff = BuffV1::timed(&name, MARK_DURATION);
    let stacks = sim
        .entity_stack_buff(target_id, &buff, MAX_MARKS, true)
        .min(MAX_MARKS);
    if stacks == 0 {
        return;
    }
    if !lightwheel_proc_ready(stacks) {
        set_mark_view(sim, target_id, stacks, MARK_DURATION);
        return;
    }
    // Brief full-charge head marker, followed by the proc burst.
    set_mark_view(sim, target_id, 6, 24);
    let max_hp = sim.get_entity(target_id).map_or(0, |e| e.hp().1);
    let level = sim.get_entity(caster_id).map_or(1, |e| e.level().max(1));
    let cap = sim.get_entity(target_id).and_then(|target| {
        if target.is_champion() {
            None
        } else if matches!(
            target.kind(),
            Some(EntityKindV1::Jungle | EntityKindV1::Epic | EntityKindV1::Serpen)
        ) {
            Some(300)
        } else {
            Some(250)
        }
    });
    let true_damage = lightwheel_true_damage(max_hp, level, cap);
    let _ = sim.entity_remove_buff(target_id, &name);
    if true_damage > 0 {
        let _ = sim.deal_damage_typed(
            caster_id,
            target_id,
            true_damage,
            DamageTypeV1::Fixed,
            AttackTypeV1::Skill,
        );
    }
    let _ = sim.play_view_effect(
        PASSIVE_PROC,
        caster_id,
        &InputTargetV1::target(target_id),
        0,
        0,
        24,
    );
}

fn add_slow(sim: &mut StableSim<'_>, target_id: usize, duration: usize) {
    const SLOW_BUFF: &str = "mlbb_heroes_karrie_s1_slow";
    let _ = sim.entity_remove_buff(target_id, SLOW_BUFF);
    let mut buff = BuffV1::timed(SLOW_BUFF, duration);
    buff.move_speed_mult = -35;
    sim.add_buff(target_id, &buff);
}

fn skill1_zone_damage(attack: usize) -> usize {
    S1_ZONE_DAMAGE_FLAT + attack * S1_ZONE_DAMAGE_AD_PERCENT / 100
}

fn nearest_enemy(
    sim: &StableSim<'_>,
    caster_id: usize,
    origin: (u64, u64),
    range: f64,
) -> Option<usize> {
    let team = sim.get_entity(caster_id)?.team();
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| {
            e.is_alive()
                && e.is_targetable()
                && e.team() != team
                && !e.is_tower()
                && !matches!(e.kind(), Some(EntityKindV1::Nexus))
        })
        .map(|e| (e.id(), e.pos()))
        .map(|(id, p)| (id, p, distance(origin, p)))
        .filter(|(_, _, d)| *d <= range)
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(id, _, _)| id)
}

pub fn register(module: &mut StableMod) {
    module.add_native_effect("mlbb_heroes:karrie_basic_attack", KarrieBasicAttack);
    module.add_native_effect("mlbb_heroes:karrie_basic_hit", KarrieBasicHit);
    module.add_native_effect("mlbb_heroes:karrie_skill1_cast", KarrieSkill1Cast);
    module.add_native_effect("mlbb_heroes:karrie_skill1_hit", KarrieSkill1Hit);
    module.add_native_effect("mlbb_heroes:karrie_skill1_zone_tick", KarrieSkill1ZoneTick);
    module.add_native_effect("mlbb_heroes:karrie_skill2", KarrieSkill2);
    module.add_native_effect("mlbb_heroes:karrie_skill2_hit", KarrieSkill2Hit);
    module.add_native_effect("mlbb_heroes:karrie_ultimate", KarrieUltimate);
    module.add_native_passive(
        "mlbb_heroes:karrie_lightwheel_mark",
        KarriePassive::default(),
    );
    module.add_player_input_ai(KarriePositionAi);
}

/// Keeps Skill 2 position-castable for the TFM2 AI while leaving other base-AI
/// decisions alone. If the base AI supplies an enemy target, convert that to a
/// world-space aim point; when it has no action, opportunistically dash toward
/// a visible enemy inside the skill's cast range.
#[derive(Clone, Copy, Default)]
struct KarriePositionAi;

impl StablePlayerAi for KarriePositionAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(*self)
    }

    fn id(&self) -> String {
        "mlbb_karrie_position_dash_ai".to_string()
    }

    fn priority(&self) -> i32 {
        150
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name.to_ascii_lowercase().contains("karrie")
    }

    fn think(
        &mut self,
        ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        if let Some(input) = base_input {
            if input.kind != InputKindV1::Skill2.code() {
                return None;
            }
            let player_id = ctx.player_id();
            let athlete_id = ctx.athlete_id();
            let sim = ctx.sim()?;
            let player = sim.get_player(player_id)?;
            let caster_id = player
                .champion()
                .map(|champion| champion.id())
                .unwrap_or(athlete_id);
            let target = match input.target.kind {
                kind if kind == InputTargetKindV1::Target.code() => {
                    let (x, y) = sim.get_entity(input.target.target_id)?.pos();
                    InputTargetV1::pos(x, y)
                }
                kind if kind == InputTargetKindV1::Pos.code() => input.target,
                kind if kind == InputTargetKindV1::Dir.code() => {
                    let (x, y) = sim.get_entity(caster_id)?.pos();
                    let direction = unit(input.target.dir_x as f64, input.target.dir_y as f64);
                    InputTargetV1::pos(
                        pos_u64(x as f64 + direction.0 * 65_000.0),
                        pos_u64(y as f64 + direction.1 * 65_000.0),
                    )
                }
                _ => return Some(input),
            };
            return Some(InputV1::action(InputKindV1::Skill2, target));
        }

        let player_id = ctx.player_id();
        let athlete_id = ctx.athlete_id();
        let aim = {
            let sim = ctx.sim()?;
            let player = sim.get_player(player_id)?;
            let caster_id = player
                .champion()
                .map(|champion| champion.id())
                .unwrap_or(athlete_id);
            let cooldown = player.cooldowns()?.2;
            if cooldown > 0 {
                return None;
            }
            let all = crate::units(&sim);
            let caster = all.iter().find(|unit| unit.id == caster_id)?;
            let target = all
                .iter()
                .filter(|unit| {
                    unit.id != caster_id
                        && unit.team != caster.team
                        && unit.alive
                        && unit.targetable
                        && !unit.tower
                        && !matches!(
                            sim.get_entity(unit.id).and_then(|entity| entity.kind()),
                            Some(EntityKindV1::Nexus)
                        )
                        && crate::distance(caster.pos, unit.pos) <= 65_000.0
                        && crate::is_unit_visible(&sim, caster.team, unit)
                })
                .min_by(|a, b| {
                    b.champion.cmp(&a.champion).then_with(|| {
                        crate::distance(caster.pos, a.pos)
                            .total_cmp(&crate::distance(caster.pos, b.pos))
                    })
                })?;
            (pos_u64(target.pos.0), pos_u64(target.pos.1))
        };
        Some(InputV1::action(
            InputKindV1::Skill2,
            InputTargetV1::pos(aim.0, aim.1),
        ))
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieBasicAttack;
impl StableEffectType for KarrieBasicAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_enemy_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let Some((start, team, alive)) = sim
            .get_entity(caster_id)
            .map(|e| (e.pos(), e.team(), e.is_alive()))
        else {
            return;
        };
        let target_id = input.target_id;
        let Some(end) = sim.get_entity(target_id).map(|e| {
            let (x, y) = e.pos();
            (x as f64, y as f64)
        }) else {
            return;
        };
        // The data action already range-gates attacks. Avoid a second strict check
        // here: at the attack start/hit timing a moving target may step just outside
        // the radius, which previously canceled the wheel spawn entirely.
        if !alive {
            return;
        }
        // The Ultimate buffs attack speed instead of changing the projectile art.
        let sprite = BASIC_PROJECTILE;
        let hit = "mlbb_heroes:karrie_basic_hit";
        let spec = projectile_spec(
            caster_id,
            team,
            start,
            end,
            PROJECTILE_SPEED,
            6_000,
            Some(target_id),
            AttackTypeV1::BaseAttack,
        );
        let _ = sim.spawn_projectile(sprite, hit, &spec);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieBasicHit;
impl StableEffectType for KarrieBasicHit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_enemy_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let target_id = input.target_id;
        let attack = sim.get_entity(caster_id).map_or(0, |e| e.stat().attack);
        sim.deal_damage(caster_id, target_id, attack, 0, AttackTypeV1::BaseAttack);
        apply_lightwheel_mark(sim, caster_id, target_id);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieSkill1Cast;
impl StableEffectType for KarrieSkill1Cast {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let Some((start, team, alive)) = sim
            .get_entity(caster_id)
            .map(|e| (e.pos(), e.team(), e.is_alive()))
        else {
            return;
        };
        if !alive {
            return;
        }
        let Some(end) = aim_point(sim, caster_id, input) else {
            return;
        };
        let direction = unit(end.0 - start.0 as f64, end.1 - start.1 as f64);
        let destination = if input.kind == InputTargetKindV1::Target.code() {
            end
        } else {
            (
                start.0 as f64 + direction.0 * S1_RANGE,
                start.1 as f64 + direction.1 * S1_RANGE,
            )
        };
        let target = if input.kind == InputTargetKindV1::Target.code() {
            Some(input.target_id)
        } else {
            None
        };
        let spec = projectile_spec(
            caster_id,
            team,
            start,
            destination,
            5_200,
            6_000,
            target,
            AttackTypeV1::Skill,
        );
        let _ = sim.spawn_projectile(S1_PROJECTILE, "mlbb_heroes:karrie_skill1_hit", &spec);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (60 + stat.attack * 80 / 100, 0)
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieSkill1Hit;
impl StableEffectType for KarrieSkill1Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_enemy_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let primary = input.target_id;
        let Some(caster) = sim.get_entity(caster_id) else {
            return;
        };
        let team = caster.team();
        let attack = caster.stat().attack;
        drop(caster);
        let Some(center) = sim.get_entity(primary).map(|t| t.pos()) else {
            return;
        };
        for i in 0..sim.entity_count() {
            let Some(target) = sim.entity_at(i) else {
                continue;
            };
            let id = target.id();
            let valid = target.is_alive()
                && target.is_targetable()
                && target.team() != team
                && !target.is_tower()
                && distance(center, target.pos()) <= S1_RADIUS;
            drop(target);
            if !valid {
                continue;
            }
            let ratio = if id == primary { 80 } else { 35 };
            let flat = if id == primary { 60 } else { 20 };
            sim.deal_damage(
                caster_id,
                id,
                flat + attack * ratio / 100,
                0,
                AttackTypeV1::Skill,
            );
            add_slow(sim, id, 60);
            apply_lightwheel_mark(sim, caster_id, id);
        }
        let _ = sim.play_view_effect(
            S1_IMPACT,
            caster_id,
            &InputTargetV1::target(primary),
            0,
            0,
            20,
        );
        let zone_input = InputTargetV1::pos(center.0, center.1);
        let _ = sim.play_view_effect(
            S1_SLOW_ZONE,
            caster_id,
            &zone_input,
            0,
            0,
            S1_ZONE_VISUAL_REFRESH_TICKS as u64,
        );
        for delay in S1_ZONE_TICK_DELAYS {
            if delay >= S1_ZONE_DURATION {
                continue;
            }
            let _ = sim.queue_effect(
                "mlbb_heroes:karrie_skill1_zone_tick",
                AttackTypeV1::Skill,
                caster_id,
                &zone_input,
                delay,
            );
        }
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (60 + stat.attack * 80 / 100, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieSkill1ZoneTick;
impl StableEffectType for KarrieSkill1ZoneTick {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Pos.code() {
            return;
        }
        let Some((team, attack)) = sim
            .get_entity(caster_id)
            .map(|e| (e.team(), e.stat().attack))
        else {
            return;
        };
        let center = (input.x, input.y);
        let _ = sim.play_view_effect(
            S1_SLOW_ZONE,
            caster_id,
            &input,
            0,
            0,
            S1_ZONE_VISUAL_REFRESH_TICKS as u64,
        );
        for i in 0..sim.entity_count() {
            let Some(target) = sim.entity_at(i) else {
                continue;
            };
            let id = target.id();
            let valid = target.is_alive()
                && target.is_targetable()
                && target.team() != team
                && !target.is_tower()
                && !matches!(target.kind(), Some(EntityKindV1::Nexus))
                && distance(center, target.pos()) <= S1_RADIUS;
            drop(target);
            if !valid {
                continue;
            }
            sim.deal_damage(
                caster_id,
                id,
                skill1_zone_damage(attack),
                0,
                AttackTypeV1::Skill,
            );
            // Zone pulses refresh the slow but do not add extra Lightwheel Marks.
            add_slow(sim, id, S1_ZONE_SLOW_DURATION);
        }
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (skill1_zone_damage(stat.attack), 0)
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieSkill2;
impl StableEffectType for KarrieSkill2 {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let Some((start, team, alive)) = sim
            .get_entity(caster_id)
            .map(|e| (e.pos(), e.team(), e.is_alive()))
        else {
            return;
        };
        if !alive {
            return;
        }
        let Some(aim) = aim_point(sim, caster_id, input) else {
            return;
        };
        let dir = unit(aim.0 - start.0 as f64, aim.1 - start.1 as f64);
        let dest = (
            pos_u64(start.0 as f64 + dir.0 * S2_DASH_DISTANCE),
            pos_u64(start.1 as f64 + dir.1 * S2_DASH_DISTANCE),
        );
        let _ = sim.entity_set_pos(caster_id, dest.0, dest.1);
        let Some(target_id) = nearest_enemy(sim, caster_id, dest, S2_ACQUIRE_RANGE) else {
            return;
        };
        let Some(end) = sim.get_entity(target_id).map(|e| {
            let (x, y) = e.pos();
            (x as f64, y as f64)
        }) else {
            return;
        };
        let sprite = BASIC_PROJECTILE;
        let spec = projectile_spec(
            caster_id,
            team,
            dest,
            end,
            PROJECTILE_SPEED,
            6_000,
            Some(target_id),
            AttackTypeV1::Skill,
        );
        let _ = sim.spawn_projectile(sprite, "mlbb_heroes:karrie_skill2_hit", &spec);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (75 + stat.attack * 70 / 100, 0)
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieSkill2Hit;
impl StableEffectType for KarrieSkill2Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_enemy_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let target = input.target_id;
        let attack = sim.get_entity(caster_id).map_or(0, |e| e.stat().attack);
        let ratio = if has_buff(sim, caster_id, ULT_BUFF) {
            100
        } else {
            70
        };
        sim.deal_damage(
            caster_id,
            target,
            75 + attack * ratio / 100,
            0,
            AttackTypeV1::Skill,
        );
        apply_lightwheel_mark(sim, caster_id, target);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (75 + stat.attack * 70 / 100, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct KarrieUltimate;
impl StableEffectType for KarrieUltimate {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, _input: InputTargetV1) {
        if !sim.get_entity(caster_id).is_some_and(|e| e.is_alive()) {
            return;
        }
        let _ = sim.entity_remove_buff(caster_id, ULT_BUFF);
        let mut buff = BuffV1::timed(ULT_BUFF, ULT_DURATION);
        buff.move_speed_mult = 20;
        buff.attack_speed_mult = 35;
        sim.add_buff(caster_id, &buff);
        let _ = sim.play_view_effect(
            ULT_PULSE,
            caster_id,
            &InputTargetV1::target(caster_id),
            0,
            0,
            24,
        );
    }
    // Give the game AI a positive offensive value for this self-buff, which now
    // converts attack speed into extra basic attacks rather than paired projectiles.
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack * 35 / 100, 0)
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Default)]
struct KarriePassive {
    voice: crate::audio::VoiceLifecycle,
}
impl StablePassive for KarriePassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        self.voice
            .on_spawn(sim, entity_id, Some("mlbb_karrie_voice_select"));
        let name = mark_buff_name(entity_id);
        let ids: Vec<usize> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i).map(|e| e.id()))
            .collect();
        for id in ids {
            let _ = sim.entity_remove_buff(id, &name);
            for view_name in MARK_VIEW_BUFFS {
                let _ = sim.entity_remove_buff(id, view_name);
            }
        }
        let _ = sim.entity_remove_buff(entity_id, ULT_BUFF);
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _seed: u64, _player: usize, entity_id: usize) {
        self.voice.on_update(
            sim,
            entity_id,
            &["mlbb_karrie_voice_select"],
            &[
                "mlbb_karrie_voice_move01",
                "mlbb_karrie_voice_move02",
                "mlbb_karrie_voice_move03",
                "mlbb_karrie_voice_move04",
                "mlbb_karrie_voice_move05",
                "mlbb_karrie_voice_move06",
                "mlbb_karrie_voice_move07",
                "mlbb_karrie_voice_move08",
            ],
        );
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        self.voice.on_dead(
            sim,
            player,
            &["mlbb_karrie_voice_death01", "mlbb_karrie_voice_death02"],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        lightwheel_proc_ready, lightwheel_ratio_percent, lightwheel_true_damage,
        skill1_zone_damage, S1_ZONE_DURATION, S1_ZONE_TICK_DELAYS, S1_ZONE_VISUAL_REFRESH_TICKS,
    };

    #[test]
    fn skill1_zone_visual_refreshes_cover_the_full_linger() {
        assert_eq!(S1_ZONE_TICK_DELAYS[0], 30);
        assert_eq!(
            S1_ZONE_TICK_DELAYS[1] + S1_ZONE_VISUAL_REFRESH_TICKS,
            S1_ZONE_DURATION
        );
    }

    #[test]
    fn skill1_zone_pulses_have_small_physical_damage() {
        assert_eq!(skill1_zone_damage(54), 15);
        assert_eq!(2 * skill1_zone_damage(54), 30);
    }

    #[test]
    fn passive_procs_only_at_five_stacks() {
        assert!(!lightwheel_proc_ready(0));
        assert!(!lightwheel_proc_ready(4));
        assert!(lightwheel_proc_ready(5));
    }

    #[test]
    fn passive_damage_scales_from_eight_to_twelve_percent_of_max_hp() {
        assert_eq!(lightwheel_ratio_percent(1), 8);
        assert_eq!(lightwheel_ratio_percent(6), 9);
        assert_eq!(lightwheel_ratio_percent(12), 12);
        assert_eq!(lightwheel_ratio_percent(99), 12);
        assert_eq!(lightwheel_true_damage(1_100, 1, None), 88);
        assert_eq!(lightwheel_true_damage(2_420, 12, None), 290);
    }

    #[test]
    fn passive_keeps_caps_only_for_non_hero_targets() {
        assert_eq!(lightwheel_true_damage(10_000, 12, Some(250)), 250);
        assert_eq!(lightwheel_true_damage(10_000, 12, Some(300)), 300);
        assert_eq!(lightwheel_true_damage(10_000, 12, None), 1_200);
    }
}
