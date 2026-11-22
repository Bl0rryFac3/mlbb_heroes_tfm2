//! Chou: native combat mechanics with data-driven actions and sprite timelines.
//! All distances use TFM2 world units; 60 simulation ticks = 1 second.
use mod_api_stable::*;

const PASSIVE_READY: &str = "mlbb_chou_only_fast_ready";
const PASSIVE_SHRED: &str = "mlbb_chou_only_fast_shred";
const S1_STAGE2: &str = "mlbb_chou_s1_stage2";
const S1_STAGE3: &str = "mlbb_chou_s1_stage3";
const ULT_SEQUENCE: &str = "mlbb_chou_ult_sequence";
const ULT_DAMAGE_PREFIX: &str = "mlbb_chou_ult_damage:";
const S2_CC_IMMUNE: &str = "mlbb_chou_shunpo_cc_immune";
const S2_GUARD: &str = "mlbb_chou_shunpo_guard";

const PASSIVE_MOVE_DISTANCE: f64 = 42_000.0;
const S1_RANGE: f64 = 67_000.0;
const S1_HALF_WIDTH: f64 = 30_000.0;
const S1_DASH_DISTANCE: f64 = 20_000.0;
const S1_DASH_SPEED: usize = 2_500;
const S1_RECAST_WINDOW: usize = 360;
const S1_FINISHER_RADIUS: f64 = 30_000.0;
// Conservative damage restoration: about +10% at level 1, up to ~18–20% late.
const S1_DAMAGE_MIN: usize = 100;
const S1_DAMAGE_MAX: usize = 165;
const S1_DAMAGE_AD_RATIO: usize = 45;
const ULT_FIRST_DAMAGE_MIN: usize = 275;
const ULT_FIRST_DAMAGE_MAX: usize = 420;
const ULT_FIRST_EXTRA_AD_RATIO: usize = 110;
const ULT_RECAST_DAMAGE_MIN: usize = 330;
const ULT_RECAST_DAMAGE_MAX: usize = 500;
const ULT_RECAST_EXTRA_AD_RATIO: usize = 132;
const S1_FINISHER_APPROACH_RANGE: f64 = S1_FINISHER_RADIUS + S1_DASH_DISTANCE;
// The finisher slam begins at frame 3 (~11 ticks into the retimed 45-tick strip).
// The effect is queued from action start_timing 2, so wait 9 more ticks.
const S1_FINISHER_IMPACT_DELAY: usize = 9;
const S2_DASH_DISTANCE: f64 = 30_000.0;
const S2_SHIELD_TICKS: usize = 150;
const S2_BACKSTEP_DISTANCE: f64 = 12_000.0;
const ULT_FIRST_RANGE: f64 = 20_000.0;
const ULT_KICK_DELAY: usize = 13;
const ULT_KNOCKBACK_SPEED: usize = 6_000;
const ULT_KNOCKBACK_TICKS: usize = 10;
const ULT_AIRBORNE_DELAY: usize = ULT_KNOCKBACK_TICKS + 1;
const ULT_AIRBORNE_TICKS: u64 = 30;
const ULT_CHASE_AFTER_KICK: usize = 17;
const ULT_CHASE_RANGE: f64 = 90_000.0;
const ULT_CHASE_SPEED: usize = 8_000;
const ULT_STRIKE_RANGE: f64 = 22_000.0;
const ULT_HIT_EFFECTS: [&str; 4] = [
    "mlbb_heroes:chou_ult_hit_1",
    "mlbb_heroes:chou_ult_hit_2",
    "mlbb_heroes:chou_ult_hit_3",
    "mlbb_heroes:chou_ult_hit_4",
];
const ULT_FLURRY_HITS: usize = ULT_HIT_EFFECTS.len();
const ULT_FLURRY_TICKS: u64 = 30;
const ULT_HIT_SPACING: usize = 7;

fn log_chou_event(sim: &StableSim<'_>, caster_id: usize, message: impl AsRef<str>) {
    let origin = sim.sim_origin().unwrap_or_default();
    let kind = match SimOriginKindV1::from_code(origin.kind) {
        Some(SimOriginKindV1::ServerPresim) => "presim",
        Some(SimOriginKindV1::ClientMatchView) => "match",
        Some(SimOriginKindV1::ClientSpectate) => "spectate",
        Some(SimOriginKindV1::ClientReplay) => "replay",
        Some(SimOriginKindV1::Tool) => "tool",
        _ => "unknown",
    };
    let match_id = if origin.match_id == SimOriginV1::NONE {
        "-".to_string()
    } else {
        origin.match_id.to_string()
    };
    crate::wlog(format!(
        "Chou [{kind} match={match_id} tick={}] caster#{caster_id}: {}",
        sim.tick(),
        message.as_ref()
    ));
}

fn clamp_level(level: usize) -> usize {
    level.clamp(1, 15)
}

fn level_value(start: usize, end: usize, level: usize) -> usize {
    let level = clamp_level(level);
    start + (end.saturating_sub(start) * (level - 1)) / 14
}

fn extra_attack(total_attack: usize, level: usize) -> usize {
    let base_attack =
        60usize.saturating_add(clamp_level(level).saturating_sub(1).saturating_mul(5));
    total_attack.saturating_sub(base_attack)
}

fn chou_s1_damage(total_attack: usize, level: usize) -> usize {
    level_value(S1_DAMAGE_MIN, S1_DAMAGE_MAX, level)
        .saturating_add(total_attack.saturating_mul(S1_DAMAGE_AD_RATIO) / 100)
}

fn chou_ult_first_damage(total_attack: usize, level: usize) -> usize {
    level_value(ULT_FIRST_DAMAGE_MIN, ULT_FIRST_DAMAGE_MAX, level).saturating_add(
        extra_attack(total_attack, level).saturating_mul(ULT_FIRST_EXTRA_AD_RATIO) / 100,
    )
}

fn chou_ult_recast_damage(total_attack: usize, level: usize) -> usize {
    level_value(ULT_RECAST_DAMAGE_MIN, ULT_RECAST_DAMAGE_MAX, level).saturating_add(
        extra_attack(total_attack, level).saturating_mul(ULT_RECAST_EXTRA_AD_RATIO) / 100,
    )
}

fn distance_sq(a: (u64, u64), b: (u64, u64)) -> u128 {
    let dx = a.0 as i128 - b.0 as i128;
    let dy = a.1 as i128 - b.1 as i128;
    (dx * dx + dy * dy) as u128
}

fn unit(x: f64, y: f64) -> (f64, f64) {
    let len = (x * x + y * y).sqrt();
    if len < 0.001 {
        (1.0, 0.0)
    } else {
        (x / len, y / len)
    }
}

fn point_behind_target(target: (u64, u64), goal: (u64, u64), distance: f64) -> (u64, u64) {
    let toward_goal = unit(
        goal.0 as f64 - target.0 as f64,
        goal.1 as f64 - target.1 as f64,
    );
    let x = (target.0 as f64 - toward_goal.0 * distance)
        .clamp(0.0, u64::MAX as f64)
        .round() as u64;
    let y = (target.1 as f64 - toward_goal.1 * distance)
        .clamp(0.0, u64::MAX as f64)
        .round() as u64;
    (x, y)
}

fn kick_goal_position(
    sim: &StableSim<'_>,
    caster_id: usize,
    target_id: usize,
    caster_pos: (u64, u64),
    target_pos: (u64, u64),
) -> ((u64, u64), Option<usize>) {
    let Some(caster) = sim.get_entity(caster_id) else {
        return (target_pos, None);
    };
    let team = caster.team();
    let mut best: Option<(u128, usize, (u64, u64))> = None;
    for i in 0..sim.entity_count() {
        let Some(entity) = sim.entity_at(i) else {
            continue;
        };
        if entity.id() == caster_id
            || entity.team() != team
            || !entity.is_alive()
            || !entity.is_targetable()
        {
            continue;
        }
        let is_structure = matches!(
            entity.kind(),
            Some(EntityKindV1::Tower | EntityKindV1::Nexus)
        );
        if !entity.is_champion() && !entity.is_tower() && !is_structure {
            continue;
        }
        let pos = entity.pos();
        let d2 = distance_sq(target_pos, pos);
        if best.is_none_or(|(best_d2, _, _)| d2 < best_d2) {
            best = Some((d2, entity.id(), pos));
        }
    }
    if let Some((_, goal_id, goal_pos)) = best {
        return (goal_pos, Some(goal_id));
    }
    // If no allied champion or structure is available, retain the familiar
    // caster-relative kick direction as a safe fallback.
    let away = unit(
        target_pos.0 as f64 - caster_pos.0 as f64,
        target_pos.1 as f64 - caster_pos.1 as f64,
    );
    let fallback = (
        (target_pos.0 as f64 + away.0 * 100_000.0)
            .clamp(0.0, u64::MAX as f64)
            .round() as u64,
        (target_pos.1 as f64 + away.1 * 100_000.0)
            .clamp(0.0, u64::MAX as f64)
            .round() as u64,
    );
    let _ = target_id;
    (fallback, None)
}

fn is_on_kick_back_side(
    caster_pos: (u64, u64),
    target_pos: (u64, u64),
    goal_pos: (u64, u64),
) -> bool {
    let toward_goal = unit(
        goal_pos.0 as f64 - target_pos.0 as f64,
        goal_pos.1 as f64 - target_pos.1 as f64,
    );
    let kick_line = unit(
        target_pos.0 as f64 - caster_pos.0 as f64,
        target_pos.1 as f64 - caster_pos.1 as f64,
    );
    distance_sq(caster_pos, target_pos) > 0
        && kick_line.0 * toward_goal.0 + kick_line.1 * toward_goal.1 >= 0.65
}

fn finisher_mark(caster: usize) -> String {
    format!("mlbb_chou_s1_hit:{caster}")
}

fn input_direction(
    sim: &StableSim<'_>,
    caster_id: usize,
    input: InputTargetV1,
) -> Option<(f64, f64)> {
    let caster = sim.get_entity(caster_id)?;
    let origin = caster.pos();
    match input.kind {
        kind if kind == InputTargetKindV1::Target.code() => {
            let target = sim.get_entity(input.target_id)?;
            let target_pos = target.pos();
            Some(unit(
                target_pos.0 as f64 - origin.0 as f64,
                target_pos.1 as f64 - origin.1 as f64,
            ))
        }
        kind if kind == InputTargetKindV1::Pos.code() => Some(unit(
            input.x as f64 - origin.0 as f64,
            input.y as f64 - origin.1 as f64,
        )),
        kind if kind == InputTargetKindV1::Dir.code() => {
            Some(unit(input.dir_x as f64, input.dir_y as f64))
        }
        _ => None,
    }
}

fn route_is_safe(sim: &StableSim<'_>, caster: usize, destination: (u64, u64)) -> bool {
    let all = crate::units(sim);
    let to = (destination.0 as f64, destination.1 as f64);
    let Some(actor) = all.iter().find(|unit| unit.id == caster) else {
        return false;
    };
    !crate::is_unsafe_tower_route(&all, actor, to, 0.0)
        && crate::path_is_clear(&crate::current_grid(&all), actor.pos, to)
}

fn is_enemy_nonstructure(sim: &StableSim<'_>, caster_id: usize, target_id: usize) -> bool {
    let Some((caster_team, target_team, alive, targetable, kind)) = sim
        .get_entity(caster_id)
        .zip(sim.get_entity(target_id))
        .map(|(caster, target)| {
            (
                caster.team(),
                target.team(),
                target.is_alive(),
                target.is_targetable(),
                target.kind(),
            )
        })
    else {
        return false;
    };
    caster_team != target_team
        && alive
        && targetable
        && !matches!(kind, Some(EntityKindV1::Tower | EntityKindV1::Nexus))
}

fn push_direction(
    sim: &mut StableSim<'_>,
    entity_id: usize,
    direction: (f64, f64),
    distance: f64,
    speed: usize,
) {
    if let Some(entity) = sim.get_entity(entity_id) {
        let origin = entity.pos();
        let dx = direction.0 * distance;
        let dy = direction.1 * distance;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.0 {
            return;
        }
        let ticks = (len / speed.max(1) as f64).ceil().max(1.0) as usize;
        let mut cc = CcV1::of_kind(CcKindV1::ForceMove, ticks as u64);
        cc.dx = dx.round() as i64;
        cc.dy = dy.round() as i64;
        cc.speed = (len / ticks as f64).ceil() as u64;
        sim.apply_cc(entity_id, &cc);
        // Keep the entity's coordinates referenced to its original point. The
        // ForceMove state performs the actual travel over the computed ticks.
        let _ = origin;
    }
}

fn add_passive_shred(sim: &mut StableSim<'_>, caster_id: usize, target_id: usize, level: usize) {
    let _ = sim.entity_remove_buff(target_id, "mlbb_chou_only_fast_slow");
    let mut slow = BuffV1::timed("mlbb_chou_only_fast_slow", 60);
    slow.move_speed_mult = -80;
    sim.add_buff(target_id, &slow);

    let mut shred = BuffV1::timed(PASSIVE_SHRED, 120);
    shred.defence = -10;
    let _ = sim.entity_stack_buff(target_id, &shred, 2, true);
    let _ = level;
    let _ = caster_id;
}

#[derive(Clone, Default)]
struct ChouOnlyFast {
    last_pos: Option<(u64, u64)>,
    distance_since_attack: f64,
    armed: bool,
}

impl StablePassive for ChouOnlyFast {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        self.last_pos = sim.get_entity(entity_id).map(|entity| entity.pos());
        self.distance_since_attack = 0.0;
        self.armed = false;
        // A respawn always starts a fresh combo; stale stage/recast buffs must
        // never cause the first visible cast to resolve as a later phase.
        let _ = sim.entity_remove_buff(entity_id, S1_STAGE2);
        let _ = sim.entity_remove_buff(entity_id, S1_STAGE3);
        let _ = sim.entity_remove_buff(entity_id, ULT_SEQUENCE);
        let _ = sim.entity_remove_buff(entity_id, PASSIVE_READY);
        let _ = sim.entity_remove_buff(entity_id, PASSIVE_SHRED);
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _seed: u64, _player: usize, entity_id: usize) {
        let Some(entity) = sim.get_entity(entity_id) else {
            return;
        };
        if !entity.is_alive() {
            return;
        }
        let pos = entity.pos();
        if let Some(last) = self.last_pos {
            self.distance_since_attack += (distance_sq(pos, last) as f64).sqrt();
        }
        self.last_pos = Some(pos);
        if !self.armed && self.distance_since_attack >= PASSIVE_MOVE_DISTANCE {
            self.armed = true;
            sim.add_buff(entity_id, &BuffV1::named(PASSIVE_READY));
        }
    }

    fn on_base_attack(
        &mut self,
        _sim: &mut StableSim<'_>,
        _seed: u64,
        _player: usize,
        _entity_id: usize,
    ) {
        if self.armed {
            self.armed = false;
            self.distance_since_attack = 0.0;
        }
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        self.last_pos = None;
        self.distance_since_attack = 0.0;
        self.armed = false;
        if let Some(id) = sim
            .get_player(player)
            .and_then(|p| p.champion())
            .map(|e| e.id())
        {
            let _ = sim.entity_remove_buff(id, ULT_SEQUENCE);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ChouEffect {
    kind: ChouEffectKind,
}

#[derive(Clone, Copy, Default)]
enum ChouEffectKind {
    #[default]
    BasicAttack,
    S1First,
    S1Second,
    S1Third,
    S1ThirdImpact,
    S2Start,
    S2End,
    UltFirst,
    UltKick,
    UltAirborne,
    UltChase,
    UltArrive,
    UltHit(usize),
}

impl StableEffectType for ChouEffect {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        match self.kind {
            ChouEffectKind::BasicAttack => basic_attack(sim, caster_id, input),
            ChouEffectKind::S1First => skill1(sim, caster_id, input, 1),
            ChouEffectKind::S1Second => skill1(sim, caster_id, input, 2),
            ChouEffectKind::S1Third => skill1(sim, caster_id, input, 3),
            ChouEffectKind::S1ThirdImpact => skill1_third_impact(sim, caster_id),
            ChouEffectKind::S2Start => shunpo_start(sim, caster_id),
            ChouEffectKind::S2End => shunpo_end(sim, caster_id),
            ChouEffectKind::UltFirst => ultimate_first(sim, caster_id, input),
            ChouEffectKind::UltKick => ultimate_kick(sim, caster_id, input),
            ChouEffectKind::UltAirborne => ultimate_airborne(sim, caster_id, input),
            ChouEffectKind::UltChase => ultimate_chase(sim, caster_id, input),
            ChouEffectKind::UltArrive => ultimate_arrive(sim, caster_id, input),
            ChouEffectKind::UltHit(hit) => ultimate_hit(sim, caster_id, input, hit),
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        match self.kind {
            ChouEffectKind::BasicAttack => (stat.attack, 0),
            ChouEffectKind::S1First | ChouEffectKind::S1Second | ChouEffectKind::S1Third => {
                (S1_DAMAGE_MIN + stat.attack * S1_DAMAGE_AD_RATIO / 100, 0)
            }
            ChouEffectKind::UltFirst => (
                ULT_FIRST_DAMAGE_MIN + stat.attack * ULT_FIRST_EXTRA_AD_RATIO / 100,
                0,
            ),
            _ => (0, 0),
        }
    }

    fn expected_shield(&self, stat: &StatV1) -> usize {
        if matches!(self.kind, ChouEffectKind::S2End) {
            250 + stat.attack
        } else {
            0
        }
    }
}

fn basic_attack(sim: &mut StableSim<'_>, caster_id: usize, input: InputTargetV1) {
    if input.kind != InputTargetKindV1::Target.code()
        || !is_enemy_nonstructure(sim, caster_id, input.target_id)
    {
        return;
    }
    let target_id = input.target_id;
    let Some((attack, crit_chance, level)) = sim.get_entity(caster_id).map(|caster| {
        let stat = caster.stat();
        (stat.attack, stat.crit_chance, caster.level())
    }) else {
        return;
    };

    let empowered = crate::has_named_buff(sim, caster_id, PASSIVE_READY);
    if empowered {
        let _ = sim.entity_remove_buff(caster_id, PASSIVE_READY);
        let suppress = BuffV1::timed("mlbb_chou_passive_crit_suppress", 2);
        let mut suppress = suppress;
        suppress.crit_chance = -(crit_chance.min(i32::MAX as usize) as i32);
        sim.add_buff(caster_id, &suppress);
    }
    let raw_damage = if empowered {
        attack.saturating_mul(180) / 100
    } else {
        attack
    };
    sim.deal_damage(
        caster_id,
        target_id,
        raw_damage,
        0,
        AttackTypeV1::BaseAttack,
    );
    if empowered {
        let _ = sim.entity_remove_buff(caster_id, "mlbb_chou_passive_crit_suppress");
        add_passive_shred(sim, caster_id, target_id, level);
        let _ = sim.entity_remove_buff(caster_id, PASSIVE_READY);
    }
}

fn skill1(sim: &mut StableSim<'_>, caster_id: usize, input: InputTargetV1, stage: usize) {
    let Some(direction) = input_direction(sim, caster_id, input) else {
        log_chou_event(
            sim,
            caster_id,
            format!("S1 stage {stage} ignored: invalid aim kind={}", input.kind),
        );
        return;
    };
    log_chou_event(
        sim,
        caster_id,
        format!("S1 stage {stage} effect started; aim_kind={}", input.kind),
    );
    if stage == 3 {
        // The data effect schedules the damage helper and RangeEffect airborne
        // together at the baked shockwave's impact frame.
        log_chou_event(
            sim,
            caster_id,
            format!(
                "S1 stage 3 impact scheduled by data effect; delay={} ticks radius={}",
                S1_FINISHER_IMPACT_DELAY, S1_FINISHER_RADIUS
            ),
        );
        push_direction(sim, caster_id, direction, S1_DASH_DISTANCE, S1_DASH_SPEED);
        let _ = sim.entity_remove_buff(caster_id, S1_STAGE2);
        let _ = sim.entity_remove_buff(caster_id, S1_STAGE3);
        return;
    }

    let Some((origin, team, attack, level)) = sim.get_entity(caster_id).map(|caster| {
        (
            caster.pos(),
            caster.team(),
            caster.stat().attack,
            caster.level(),
        )
    }) else {
        return;
    };
    let damage = chou_s1_damage(attack, level);
    log_chou_event(
        sim,
        caster_id,
        format!("S1 stage {stage} raw_damage_per_target={damage}"),
    );
    let count = sim.entity_count();
    for index in 0..count {
        let Some((target_id, target_team, alive, targetable, kind, pos, radius)) =
            sim.entity_at(index).map(|target| {
                (
                    target.id(),
                    target.team(),
                    target.is_alive(),
                    target.is_targetable(),
                    target.kind(),
                    target.pos(),
                    target.radius(),
                )
            })
        else {
            continue;
        };
        if target_id == caster_id || target_team == team || !alive || !targetable {
            continue;
        }
        if matches!(kind, Some(EntityKindV1::Tower | EntityKindV1::Nexus)) {
            continue;
        }
        let vx = pos.0 as f64 - origin.0 as f64;
        let vy = pos.1 as f64 - origin.1 as f64;
        let along = vx * direction.0 + vy * direction.1;
        let across = (vx * direction.1 - vy * direction.0).abs();
        let target_radius = radius as f64;
        if along < 0.0 || along > S1_RANGE + target_radius || across > S1_HALF_WIDTH + target_radius
        {
            continue;
        }
        sim.deal_damage(caster_id, target_id, damage, 0, AttackTypeV1::Skill);
    }

    push_direction(sim, caster_id, direction, S1_DASH_DISTANCE, S1_DASH_SPEED);
    match stage {
        1 => {
            let _ = sim.entity_remove_buff(caster_id, S1_STAGE3);
            sim.add_buff(caster_id, &BuffV1::timed(S1_STAGE2, S1_RECAST_WINDOW));
        }
        2 => {
            let _ = sim.entity_remove_buff(caster_id, S1_STAGE2);
            sim.add_buff(caster_id, &BuffV1::timed(S1_STAGE3, S1_RECAST_WINDOW));
        }
        _ => unreachable!("S1 finisher handled above"),
    }
}

fn skill1_third_impact(sim: &mut StableSim<'_>, caster_id: usize) {
    let Some((origin, team, attack, level)) = sim.get_entity(caster_id).map(|caster| {
        (
            caster.pos(),
            caster.team(),
            caster.stat().attack,
            caster.level(),
        )
    }) else {
        return;
    };
    let damage = chou_s1_damage(attack, level);
    let radius_sq = (S1_FINISHER_RADIUS * S1_FINISHER_RADIUS) as u128;
    let mut hit_champion = false;
    let mut hit_count = 0usize;
    let count = sim.entity_count();
    for index in 0..count {
        let Some((target_id, target_team, alive, targetable, kind, pos, is_champion)) =
            sim.entity_at(index).map(|target| {
                (
                    target.id(),
                    target.team(),
                    target.is_alive(),
                    target.is_targetable(),
                    target.kind(),
                    target.pos(),
                    target.is_champion(),
                )
            })
        else {
            continue;
        };
        if target_id == caster_id || target_team == team || !alive || !targetable {
            continue;
        }
        if matches!(kind, Some(EntityKindV1::Tower | EntityKindV1::Nexus))
            || distance_sq(origin, pos) > radius_sq
        {
            continue;
        }
        sim.deal_damage(caster_id, target_id, damage, 0, AttackTypeV1::Skill);
        hit_count += 1;
        if is_champion {
            hit_champion = true;
            if sim
                .get_entity(target_id)
                .is_some_and(|target| target.is_alive() && target.hp().0 > 0)
            {
                sim.add_buff(target_id, &BuffV1::timed(&finisher_mark(caster_id), 30));
            }
        }
    }
    let skill2_reset =
        hit_champion && sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Skill2, 9_999);
    log_chou_event(
        sim,
        caster_id,
        format!(
            "S1 finisher damage impact at ({},{}) radius={} raw_damage_per_target={damage} hit_targets={hit_count} hit_champion={hit_champion} airborne=delayed_data_effect skill2_reset={skill2_reset} combo_hit_marker={hit_champion}",
            origin.0, origin.1, S1_FINISHER_RADIUS
        ),
    );
}

fn shunpo_start(sim: &mut StableSim<'_>, caster_id: usize) {
    let mut buff = BuffV1::timed(S2_CC_IMMUNE, 12);
    buff.cc_immune = true;
    buff.move_speed_mult = 10;
    sim.add_buff(caster_id, &buff);
}

fn shunpo_end(sim: &mut StableSim<'_>, caster_id: usize) {
    let Some((attack, level)) = sim
        .get_entity(caster_id)
        .map(|caster| (caster.stat().attack, caster.level()))
    else {
        return;
    };
    let base = level_value(200, 300, level);
    let shield = base.saturating_add(extra_attack(attack, level));
    let _ = sim.entity_add_shield(caster_id, shield, S2_SHIELD_TICKS);
    let mut pen = BuffV1::timed(S2_GUARD, S2_SHIELD_TICKS);
    pen.defence_penetration = 15;
    sim.add_buff(caster_id, &pen);
}

fn live_champion_target(sim: &StableSim<'_>, caster: usize, target: usize) -> bool {
    is_enemy_nonstructure(sim, caster, target)
        && sim
            .get_entity(caster)
            .is_some_and(|e| e.is_alive() && e.hp().0 > 0)
        && sim
            .get_entity(target)
            .is_some_and(|e| e.is_champion() && e.hp().0 > 0)
}

fn ultimate_target(
    sim: &StableSim<'_>,
    caster: usize,
    input: InputTargetV1,
) -> Option<(usize, (u64, u64), (u64, u64))> {
    let target = input.target_id;
    if input.kind != InputTargetKindV1::Target.code()
        || !crate::has_named_buff(sim, caster, ULT_SEQUENCE)
        || !live_champion_target(sim, caster, target)
    {
        return None;
    }
    Some((
        target,
        sim.get_entity(caster)?.pos(),
        sim.get_entity(target)?.pos(),
    ))
}

fn ultimate_first(sim: &mut StableSim<'_>, caster: usize, input: InputTargetV1) {
    if input.kind != InputTargetKindV1::Target.code()
        || !live_champion_target(sim, caster, input.target_id)
    {
        return;
    }
    let Some((from, target)) = sim
        .get_entity(caster)
        .zip(sim.get_entity(input.target_id))
        .map(|(caster, target)| (caster.pos(), target.pos()))
    else {
        return;
    };
    if distance_sq(from, target) > (ULT_FIRST_RANGE as u128).pow(2) {
        return;
    }
    sim.add_buff(caster, &BuffV1::timed(ULT_SEQUENCE, 100));
    let queued = sim.queue_effect(
        "mlbb_heroes:chou_ult_kick",
        AttackTypeV1::Skill,
        caster,
        &input,
        ULT_KICK_DELAY,
    );
    log_chou_event(
        sim,
        caster,
        format!("Ultimate target#{} kick_queued={queued}", input.target_id),
    );
}

fn ultimate_kick(sim: &mut StableSim<'_>, caster: usize, input: InputTargetV1) {
    let Some((target, from, target_pos)) = ultimate_target(sim, caster, input) else {
        return;
    };
    if distance_sq(from, target_pos) > (ULT_STRIKE_RANGE as u128).pow(2)
        || crate::hanzo::blocked(sim, caster, false)
    {
        return;
    }
    let Some(amount) = sim
        .get_entity(caster)
        .map(|e| chou_ult_first_damage(e.stat().attack, e.level()))
    else {
        return;
    };
    sim.deal_damage(caster, target, amount, 0, AttackTypeV1::Skill);
    if !live_champion_target(sim, caster, target) {
        return;
    }
    let kicked = sim.entity_knockback(caster, target, ULT_KNOCKBACK_SPEED, ULT_KNOCKBACK_TICKS);
    log_chou_event(
        sim,
        caster,
        format!("Ultimate first kick target#{target} knockback={kicked}"),
    );
    if kicked {
        sim.queue_effect(
            "mlbb_heroes:chou_ult_airborne",
            AttackTypeV1::Skill,
            caster,
            &input,
            ULT_AIRBORNE_DELAY,
        );
        sim.queue_effect(
            "mlbb_heroes:chou_ult_chase",
            AttackTypeV1::Skill,
            caster,
            &input,
            ULT_CHASE_AFTER_KICK,
        );
    }
}

fn ultimate_airborne(sim: &mut StableSim<'_>, caster: usize, input: InputTargetV1) {
    if let Some((target, _, _)) = ultimate_target(sim, caster, input) {
        sim.apply_cc(
            target,
            &CcV1::of_kind(CcKindV1::Airborne, ULT_AIRBORNE_TICKS),
        );
    }
}

fn chase_destination(from: (u64, u64), to: (u64, u64)) -> Option<((u64, u64), usize)> {
    let gap = (distance_sq(from, to) as f64).sqrt();
    if gap > ULT_CHASE_RANGE {
        return None;
    }
    let travel = (gap - 10_000.0).max(0.0);
    let dir = unit(to.0 as f64 - from.0 as f64, to.1 as f64 - from.1 as f64);
    let destination = (
        (from.0 as f64 + dir.0 * travel).round().max(0.0) as u64,
        (from.1 as f64 + dir.1 * travel).round().max(0.0) as u64,
    );
    Some((
        destination,
        (travel / ULT_CHASE_SPEED as f64).ceil() as usize,
    ))
}

fn ultimate_chase(sim: &mut StableSim<'_>, caster: usize, input: InputTargetV1) {
    let Some((target, from, target_pos)) = ultimate_target(sim, caster, input) else {
        return;
    };
    let Some((destination, ticks)) = chase_destination(from, target_pos) else {
        return;
    };
    let Some(team) = sim.get_entity(caster).map(|e| e.team()) else {
        return;
    };
    if crate::hanzo::blocked(sim, caster, true)
        || !route_is_safe(sim, caster, destination)
        || !sim.is_visible(team, target)
    {
        log_chou_event(
            sim,
            caster,
            format!("Ultimate follow-up skipped: unsafe route or crowd control, target#{target}"),
        );
        return;
    }
    if ticks == 0 {
        ultimate_arrive(sim, caster, input);
        return;
    }
    if sim.queue_effect(
        "mlbb_heroes:chou_ult_arrive",
        AttackTypeV1::Skill,
        caster,
        &input,
        ticks + 1,
    ) {
        let dx = destination.0 as i64 - from.0 as i64;
        let dy = destination.1 as i64 - from.1 as i64;
        let mut dash = CcV1::of_kind(CcKindV1::ForceMove, ticks as u64);
        dash.dx = dx;
        dash.dy = dy;
        dash.speed = ((dx as f64).hypot(dy as f64) / ticks as f64).ceil() as u64;
        sim.apply_cc(caster, &dash);
    }
}

fn ultimate_arrive(sim: &mut StableSim<'_>, caster: usize, input: InputTargetV1) {
    let Some((target, from, target_pos)) = ultimate_target(sim, caster, input) else {
        return;
    };
    if distance_sq(from, target_pos) > (ULT_STRIKE_RANGE as u128).pow(2)
        || !route_is_safe(sim, caster, from)
        || crate::hanzo::blocked(sim, caster, false)
    {
        log_chou_event(
            sim,
            caster,
            format!("Ultimate follow-up skipped: no safe arrival on target#{target}"),
        );
        return;
    }
    let Some(total) = sim
        .get_entity(caster)
        .map(|e| chou_ult_recast_damage(e.stat().attack, e.level()))
    else {
        return;
    };
    let mut flurry = CcV1::of_kind(CcKindV1::Animation, ULT_FLURRY_TICKS);
    flurry.set_name("ult_recast");
    sim.apply_cc(caster, &flurry);
    sim.apply_cc(target, &CcV1::of_kind(CcKindV1::Airborne, ULT_FLURRY_TICKS));
    // Target inputs are marshalled by the host. Keep the damage snapshot on
    // the caster and encode the hit index in the registered effect.
    sim.add_buff(
        caster,
        &BuffV1::timed(
            &format!("{ULT_DAMAGE_PREFIX}{total}"),
            ULT_FLURRY_TICKS as usize,
        ),
    );
    log_chou_event(
        sim,
        caster,
        format!("Ultimate follow-up arrived on target#{target}, raw total={total}"),
    );
    for (hit, name) in ULT_HIT_EFFECTS.iter().enumerate() {
        sim.queue_effect(
            name,
            AttackTypeV1::Skill,
            caster,
            &input,
            1 + hit * ULT_HIT_SPACING,
        );
    }
}

fn flurry_damage(total: usize, hit: usize) -> usize {
    total / ULT_FLURRY_HITS + usize::from(hit < total % ULT_FLURRY_HITS)
}

fn ultimate_hit(sim: &mut StableSim<'_>, caster: usize, input: InputTargetV1, hit: usize) {
    let Some((target, from, target_pos)) = ultimate_target(sim, caster, input) else {
        let _ = sim.entity_remove_buff(caster, ULT_SEQUENCE);
        return;
    };
    if distance_sq(from, target_pos) > (ULT_STRIKE_RANGE as u128).pow(2)
        || crate::hanzo::blocked(sim, caster, false)
    {
        let _ = sim.entity_remove_buff(caster, ULT_SEQUENCE);
        return;
    }
    let Some(total) = sim.get_entity(caster).and_then(|e| {
        (0..e.buff_count())
            .filter_map(|i| e.buff_at(i))
            .find_map(|b| {
                b.name()
                    .strip_prefix(ULT_DAMAGE_PREFIX)?
                    .parse::<usize>()
                    .ok()
            })
    }) else {
        return;
    };
    sim.deal_damage(
        caster,
        target,
        flurry_damage(total, hit),
        0,
        AttackTypeV1::Skill,
    );
    if hit + 1 == ULT_FLURRY_HITS {
        let _ = sim.entity_remove_buff(caster, ULT_SEQUENCE);
    }
}

#[derive(Clone, Default)]
struct ChouAi;

impl StablePlayerAi for ChouAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(self.clone())
    }

    fn id(&self) -> String {
        "mlbb_chou_combo_ai".to_string()
    }

    fn priority(&self) -> i32 {
        150
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name == "mlbb_chou"
    }

    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        let base = base?;
        if base.kind != InputKindV1::Attack.code()
            || base.target.kind != InputTargetKindV1::Target.code()
        {
            return None;
        }
        let player = ctx.player_id();
        let proposed = {
            let sim = ctx.sim()?;
            let e = sim.get_player(player)?.champion()?;
            let id = e.id();
            let target = base.target.target_id;
            if !live_champion_target(&sim, id, target)
                || !sim.is_visible(e.team(), target)
                || crate::has_named_buff(&sim, id, ULT_SEQUENCE)
                || crate::hanzo::blocked(&sim, id, false)
            {
                return None;
            }
            let from = e.pos();
            let to = sim.get_entity(target)?.pos();
            let gap = (distance_sq(from, to) as f64).sqrt();
            let ready = |slot| {
                e.cooldown(slot)
                    .is_some_and(|c| c.unlocked && c.charges > 0)
            };
            let shunpo = ready(SkillSlotV1::Skill2) && !crate::hanzo::blocked(&sim, id, true);
            let ult = ready(SkillSlotV1::Ult);
            let stage3 = crate::has_named_buff(&sim, id, S1_STAGE3);
            let stage2 = crate::has_named_buff(&sim, id, S1_STAGE2);
            if ult && crate::has_named_buff(&sim, target, &finisher_mark(id)) {
                let (goal, _) = kick_goal_position(&sim, id, target, from, to);
                if !crate::has_named_buff(&sim, id, S2_CC_IMMUNE)
                    && gap <= ULT_FIRST_RANGE
                    && is_on_kick_back_side(from, to, goal)
                    && route_is_safe(&sim, id, to)
                {
                    InputV1::action(InputKindV1::Ult, InputTargetV1::target(target))
                } else {
                    let behind = point_behind_target(to, goal, S2_BACKSTEP_DISTANCE);
                    if !shunpo
                        || distance_sq(from, behind) > (S2_DASH_DISTANCE as u128).pow(2)
                        || !is_on_kick_back_side(behind, to, goal)
                        || !route_is_safe(&sim, id, behind)
                    {
                        return None;
                    }
                    InputV1::action(InputKindV1::Skill2, InputTargetV1::pos(behind.0, behind.1))
                }
            } else if (stage2 || stage3)
                && ready(SkillSlotV1::Skill)
                && !crate::hanzo::blocked(&sim, id, true)
                && gap > 0.0
            {
                let max_range = if stage3 {
                    S1_FINISHER_APPROACH_RANGE
                } else {
                    S1_RANGE
                };
                let (kind, travel) = if gap <= max_range {
                    (InputKindV1::Skill, S1_DASH_DISTANCE)
                } else if stage3 && ult && shunpo && gap <= max_range + S2_DASH_DISTANCE {
                    (InputKindV1::Skill2, S2_DASH_DISTANCE)
                } else {
                    return None;
                };
                let dir = unit(to.0 as f64 - from.0 as f64, to.1 as f64 - from.1 as f64);
                let landing = (
                    (from.0 as f64 + travel * dir.0).max(0.0) as u64,
                    (from.1 as f64 + travel * dir.1).max(0.0) as u64,
                );
                if !route_is_safe(&sim, id, landing) {
                    return None;
                }
                InputV1::action(kind, InputTargetV1::pos(to.0, to.1))
            } else {
                return None;
            }
        };
        ctx.is_valid_input(&proposed).then_some(proposed)
    }
}

pub(crate) fn register(module: &mut StableMod) {
    crate::wlog("Chou trace enabled: S1 stages/impact and Ultimate phase events will be recorded.");
    module.add_native_passive("mlbb_heroes:chou_only_fast", ChouOnlyFast::default());
    for (name, kind) in [
        ("chou_basic_attack", ChouEffectKind::BasicAttack),
        ("chou_s1_first", ChouEffectKind::S1First),
        ("chou_s1_second", ChouEffectKind::S1Second),
        ("chou_s1_third", ChouEffectKind::S1Third),
        ("chou_s1_third_impact", ChouEffectKind::S1ThirdImpact),
        ("chou_s2_start", ChouEffectKind::S2Start),
        ("chou_s2_end", ChouEffectKind::S2End),
        ("chou_ult_first", ChouEffectKind::UltFirst),
        ("chou_ult_kick", ChouEffectKind::UltKick),
        ("chou_ult_airborne", ChouEffectKind::UltAirborne),
        ("chou_ult_chase", ChouEffectKind::UltChase),
        ("chou_ult_arrive", ChouEffectKind::UltArrive),
    ] {
        module.add_native_effect(format!("mlbb_heroes:{name}"), ChouEffect { kind });
    }
    for (hit, name) in ULT_HIT_EFFECTS.iter().enumerate() {
        module.add_native_effect(
            *name,
            ChouEffect {
                kind: ChouEffectKind::UltHit(hit),
            },
        );
    }
    module.add_player_input_ai(ChouAi);
}

#[cfg(test)]
mod tests {
    use super::{
        chou_s1_damage, chou_ult_first_damage, chou_ult_recast_damage, extra_attack, level_value,
        unit,
    };

    #[test]
    fn revised_damage_and_shield_scaling_stays_bounded() {
        assert_eq!(level_value(90, 140, 1), 90);
        assert_eq!(level_value(90, 140, 15), 140);
        assert_eq!(level_value(250, 350, 99), 350);
        assert_eq!(level_value(300, 420, 1), 300);
        assert_eq!(level_value(300, 420, 15), 420);
        assert_eq!(level_value(200, 300, 15), 300);
        assert_eq!(extra_attack(130, 15), 0);
        assert_eq!(extra_attack(230, 15), 100);
    }

    #[test]
    fn new_chou_ranges_and_slam_timing_are_coherent() {
        assert_eq!(super::S1_DASH_DISTANCE, 20_000.0);
        assert_eq!(super::S1_FINISHER_RADIUS, 30_000.0);
        assert_eq!(super::S1_FINISHER_APPROACH_RANGE, 50_000.0);
        assert_eq!(super::S1_FINISHER_IMPACT_DELAY + 2, 11);
        assert!(super::S2_BACKSTEP_DISTANCE < super::ULT_FIRST_RANGE);
    }

    #[test]
    fn damage_restoration_is_modest_early_and_larger_late() {
        // Compared with the previous v1.1.39 formulas: approximately +10% early,
        // rising to roughly +16–20% late, rather than importing full MLBB values.
        assert_eq!(chou_s1_damage(60, 1), 127); // old: 114
        assert_eq!(chou_s1_damage(130, 15), 223); // old: 192
        assert_eq!(chou_ult_first_damage(60, 1), 275); // old: 250
        assert_eq!(chou_ult_first_damage(130, 15), 420); // old: 350
        assert_eq!(chou_ult_recast_damage(60, 1), 330); // old: 300
        assert_eq!(chou_ult_recast_damage(130, 15), 500); // old: 420
        assert_eq!(chou_ult_first_damage(230, 15), 530); // includes +10% extra-attack ratio
        assert_eq!(chou_ult_recast_damage(230, 15), 632); // includes +10% extra-attack ratio
    }

    #[test]
    fn insec_position_is_on_the_side_opposite_the_kick_goal() {
        let target = (50_000, 50_000);
        let goal = (0, 50_000);
        let back = super::point_behind_target(target, goal, 20_000.0);
        assert_eq!(back, (70_000, 50_000));
        assert!(super::is_on_kick_back_side(back, target, goal));
        assert!(!super::is_on_kick_back_side((40_000, 50_000), target, goal));
    }

    #[test]
    fn chase_reaches_the_kicked_target_and_preserves_total_damage() {
        let from = (100_000, 200_000);
        let target = (180_000, 200_000);
        let (landing, ticks) = super::chase_destination(from, target).unwrap();
        assert_eq!(landing, (170_000, 200_000));
        assert_eq!(ticks, 9);
        assert!(super::distance_sq(landing, target) < (super::ULT_STRIKE_RANGE as u128).pow(2));
        assert!(super::chase_destination(from, (200_000, 200_000)).is_none());
        for total in [330, 500, 562, 631] {
            let sum: usize = (0..super::ULT_FLURRY_HITS)
                .map(|hit| super::flurry_damage(total, hit))
                .sum();
            assert_eq!(sum, total);
        }
    }

    #[test]
    fn follow_up_keeps_target_and_pays_only_after_arrival() {
        use super::*;
        use crate::hanzo::tests::Host;

        let mut h = Host::new();
        h.knockback_allowed = true;
        h.record_queued_effects = true;
        // Select a clear 80k stretch in the host's map fixture.
        let origin = h
            .sim(|sim| {
                (10..70)
                    .flat_map(|x| (10..70).map(move |y| (x * 12_000, y * 12_000)))
                    .find(|&(x, y)| {
                        sim.entity_set_pos(1, x, y);
                        sim.entity_set_pos(2, x + 15_000, y);
                        route_is_safe(sim, 1, (x + 65_000, y))
                    })
            })
            .expect("open route");
        let (x, y) = origin;
        let target = InputTargetV1::target(2);
        h.set_unit_max_hp(2, 1500);
        h.respawn_unit(2, (x + 15_000, y));
        let hp = |host: &mut Host| host.sim(|sim| sim.get_entity(2).unwrap().hp().0);

        assert!(h
            .think_with(&mut ChouAi, Some(InputV1::return_home()))
            .is_none());
        assert!(h
            .think_with(&mut ChouAi, Some(InputV1::move_to(x, y)))
            .is_none());
        h.sim(|sim| ultimate_first(sim, 1, target));
        assert_eq!(hp(&mut h), 1500);
        assert_eq!(
            h.queued.remove(0),
            (
                "mlbb_heroes:chou_ult_kick".into(),
                1,
                target,
                ULT_KICK_DELAY
            )
        );
        h.sim(|sim| ultimate_kick(sim, 1, target));
        assert_eq!(h.queued.len(), 2);
        assert!(h.queued.iter().all(|(_, _, input, _)| *input == target));
        h.queued.clear();

        // In the host fixture we advance the knockback and ForceMove positions.
        h.set_unit_position(2, (x + 75_000, y));
        let kicked_hp = hp(&mut h);
        h.sim(|sim| ultimate_chase(sim, 1, target));
        assert_eq!(hp(&mut h), kicked_hp);
        assert_eq!(h.queued.remove(0).0, "mlbb_heroes:chou_ult_arrive");
        h.sim(|sim| ultimate_arrive(sim, 1, target));
        assert_eq!(hp(&mut h), kicked_hp);
        assert!(h.queued.is_empty());

        h.set_unit_position(1, (x + 65_000, y));
        let total = h.sim(|sim| {
            let e = sim.get_entity(1).unwrap();
            chou_ult_recast_damage(e.stat().attack, e.level())
        });
        h.sim(|sim| ultimate_arrive(sim, 1, target));
        for (hit, (name, caster, input, delay)) in
            std::mem::take(&mut h.queued).into_iter().enumerate()
        {
            assert_eq!(
                (name.as_str(), caster, input, delay),
                (ULT_HIT_EFFECTS[hit], 1, target, 1 + hit * ULT_HIT_SPACING)
            );
            h.sim(|sim| ultimate_hit(sim, caster, input, hit));
        }
        assert_eq!(hp(&mut h), kicked_hp - total);
        assert!(!h.sim(|sim| crate::has_named_buff(sim, 1, ULT_SEQUENCE)));
    }

    #[test]
    fn native_combat_target_gets_a_reachable_shunpo_and_in_range_kick() {
        use super::*;
        use crate::hanzo::tests::{add_unit, Host};
        let mut h = Host::new();
        add_unit(&mut h, 3, EntityKindV1::Champion, 0, (200_000, 200_000));
        let (x, y) = h
            .sim(|sim| {
                (10..70)
                    .flat_map(|x| (10..70).map(move |y| (x * 12_000, y * 12_000)))
                    .find(|&(x, y)| {
                        sim.entity_set_pos(1, x + 28_000, y);
                        sim.entity_set_pos(2, x + 15_000, y);
                        sim.entity_set_pos(3, x + 60_000, y);
                        route_is_safe(sim, 1, (x + 3_000, y))
                    })
            })
            .expect("open Shunpo route");
        let attack = Some(InputV1::action(
            InputKindV1::Attack,
            InputTargetV1::target(2),
        ));
        assert!(
            h.think_with(&mut ChouAi, attack).is_none(),
            "ordinary native attack"
        );
        h.sim(|sim| sim.add_buff(2, &BuffV1::timed(&finisher_mark(1), 30)));
        let dash = h.think_with(&mut ChouAi, attack).expect("Shunpo");
        assert_eq!(dash.kind, InputKindV1::Skill2.code());
        assert_eq!((dash.target.x, dash.target.y), (x + 3_000, y));
        h.set_unit_position(1, (dash.target.x, dash.target.y));
        let kick = h
            .think_with(&mut ChouAi, attack)
            .expect("kick from the actual landing");
        assert_eq!(
            kick,
            InputV1::action(InputKindV1::Ult, InputTargetV1::target(2))
        );
        assert!(h
            .think_with(&mut ChouAi, Some(InputV1::return_home()))
            .is_none());
    }

    #[test]
    fn direction_is_normalized() {
        let d = unit(3.0, 4.0);
        assert!((d.0 - 0.6).abs() < 1e-9);
        assert!((d.1 - 0.8).abs() < 1e-9);
    }
}
