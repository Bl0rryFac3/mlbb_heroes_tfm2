//! Chou: native combat mechanics with data-driven actions and sprite timelines.
//! All distances use TFM2 world units; 60 simulation ticks = 1 second.
use mod_api_stable::*;

const PASSIVE_READY: &str = "mlbb_chou_only_fast_ready";
const PASSIVE_SHRED: &str = "mlbb_chou_only_fast_shred";
const S1_STAGE2: &str = "mlbb_chou_s1_stage2";
const S1_STAGE3: &str = "mlbb_chou_s1_stage3";
const S1_FINISHER_HIT: &str = "mlbb_chou_s1_finisher_hit";
const ULT_RECAST: &str = "mlbb_chou_ult_recast";
const S2_CC_IMMUNE: &str = "mlbb_chou_shunpo_cc_immune";
const S2_GUARD: &str = "mlbb_chou_shunpo_guard";

const PASSIVE_MOVE_DISTANCE: f64 = 42_000.0;
const S1_RANGE: f64 = 67_000.0;
const S1_HALF_WIDTH: f64 = 30_000.0;
const S1_DASH_DISTANCE: f64 = 20_000.0;
const S1_DASH_SPEED: usize = 2_500;
const S1_ACTION_LOCK: usize = 45;
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
const S2_DASH_SPEED: usize = 3_000;
const S2_SHIELD_TICKS: usize = 150;
const S2_ACTION_LOCK: usize = 52; // 50-tick action plus the existing 2-tick AI guard.
const S2_BACKSTEP_DISTANCE: f64 = 20_000.0;
const ULT_WINDOW: usize = 240;
const ULT_READY_DELAY: usize = 72;
const ULT_FIRST_RANGE: f64 = 20_000.0; // Slightly wider than Chou's 15,000 basic-attack range.
const ULT_FIRST_AI_RANGE: f64 = 18_000.0; // Close-range trigger with room for the cast-start tick.
const ULT_TARGET_RANGE: f64 = 145_000.0;
const ULT_RECAST_RANGE_BONUS: u64 = 125_000; // 20,000 first-cast base + bonus = 145,000 recast reach.
const ULT_KNOCKBACK_SPEED: usize = 10_000;
const ULT_KNOCKBACK_TICKS: usize = 10;
const ULT_AIRBORNE_DELAY: usize = ULT_KNOCKBACK_TICKS + 1;
const ULT_AIRBORNE_TICKS: u64 = 30;

fn ult_recast_range_bonus(recast_active: bool) -> u64 {
    if recast_active {
        ULT_RECAST_RANGE_BONUS
    } else {
        0
    }
}

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

fn log_chou_ai(ctx: &mut StableAiContext<'_>, caster_id: usize, message: impl AsRef<str>) {
    if let Some(sim) = ctx.sim() {
        log_chou_event(&sim, caster_id, message);
    } else {
        crate::wlog(format!("Chou AI caster#{caster_id}: {}", message.as_ref()));
    }
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
    let target_to_caster = (
        caster_pos.0 as f64 - target_pos.0 as f64,
        caster_pos.1 as f64 - target_pos.1 as f64,
    );
    target_to_caster.0 * toward_goal.0 + target_to_caster.1 * toward_goal.1 < 0.0
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

fn chou_tower_target_is_safe(sim: &StableSim<'_>, caster_id: usize, target_id: usize) -> bool {
    let all = crate::units(sim);
    let Some(caster) = all.iter().find(|unit| unit.id == caster_id) else {
        return false;
    };
    let Some(target) = all.iter().find(|unit| unit.id == target_id) else {
        return false;
    };
    !crate::is_unsafe_under_enemy_tower(&all, caster, target)
        && !crate::is_unsafe_tower_route(&all, caster, target.pos, 0.0)
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
        let _ = sim.entity_remove_buff(entity_id, S1_FINISHER_HIT);
        let _ = sim.entity_remove_buff(entity_id, ULT_RECAST);
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
        let _ = player;
        self.last_pos = None;
        self.distance_since_attack = 0.0;
        self.armed = false;
        // The passive callback only exposes player on death, not the entity id;
        // the next on_spawn clears the visual-ready state.
        let _ = sim;
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
    UltAirborne,
    UltRecastReady,
    UltRecast,
    UltRange,
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
            ChouEffectKind::UltAirborne => ultimate_airborne(sim, caster_id, input),
            ChouEffectKind::UltRecastReady => ult_recast_ready(sim, caster_id),
            ChouEffectKind::UltRecast => ultimate_recast(sim, caster_id, input),
            ChouEffectKind::UltRange => {} // Range-only marker; dynamic adjustment is below.
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
            ChouEffectKind::UltRecast => (
                ULT_RECAST_DAMAGE_MIN + stat.attack * ULT_RECAST_EXTRA_AD_RATIO / 100,
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

    fn may_adjust_range(&self) -> bool {
        matches!(self.kind, ChouEffectKind::UltRange)
    }

    fn range_adjust(&self, caster: &StableEntityView<'_>, _target: &StableEntityView<'_>) -> u64 {
        if matches!(self.kind, ChouEffectKind::UltRange) {
            ult_recast_range_bonus(caster.has_buff(ULT_RECAST))
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
    if stage == 1 {
        let _ = sim.entity_remove_buff(caster_id, S1_FINISHER_HIT);
    }
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
        }
    }
    let skill2_reset =
        hit_champion && sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Skill2, 9_999);
    if hit_champion {
        sim.add_buff(caster_id, &BuffV1::timed(S1_FINISHER_HIT, 120));
    }
    log_chou_event(
        sim,
        caster_id,
        format!(
            "S1 finisher damage impact at ({},{}) radius={} raw_damage_per_target={damage} hit_targets={hit_count} hit_champion={hit_champion} airborne=delayed_data_effect skill2_reset={skill2_reset} combo_hit_marker={hit_champion}",
            origin.0, origin.1, S1_FINISHER_RADIUS
        ),
    );
}

fn ult_recast_ready(sim: &mut StableSim<'_>, caster_id: usize) {
    let has_recast = crate::has_named_buff(sim, caster_id, ULT_RECAST);
    let reset = has_recast && sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Ult, 99_999);
    log_chou_event(
        sim,
        caster_id,
        format!("Ultimate recast-ready effect: buff_present={has_recast} cooldown_reset={reset}"),
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

fn ultimate_first(sim: &mut StableSim<'_>, caster_id: usize, input: InputTargetV1) {
    if input.kind != InputTargetKindV1::Target.code()
        || !is_enemy_nonstructure(sim, caster_id, input.target_id)
    {
        log_chou_event(
            sim,
            caster_id,
            format!(
                "Ultimate first phase rejected input kind={} target#{}",
                input.kind, input.target_id
            ),
        );
        return;
    }
    let target_id = input.target_id;
    let Some((caster_pos, target_pos, attack, level)) = sim
        .get_entity(caster_id)
        .zip(sim.get_entity(target_id))
        .map(|(caster, target)| {
            (
                caster.pos(),
                target.pos(),
                caster.stat().attack,
                caster.level(),
            )
        })
    else {
        return;
    };
    let first_range_sq = (ULT_FIRST_RANGE as u128) * (ULT_FIRST_RANGE as u128);
    let target_distance_sq = distance_sq(caster_pos, target_pos);
    if target_distance_sq > first_range_sq {
        log_chou_event(
            sim,
            caster_id,
            format!(
                "Ultimate first phase rejected: target#{target_id} distance={:.0} exceeds close range {}",
                (target_distance_sq as f64).sqrt(), ULT_FIRST_RANGE
            ),
        );
        return;
    }
    let damage = chou_ult_first_damage(attack, level);
    sim.deal_damage(caster_id, target_id, damage, 0, AttackTypeV1::Skill);
    // Experimental long roundhouse: 10,000 units/tick for 10 ticks = 100,000 units. Airborne is applied
    // by a delayed effect after the 10-tick knockback has completed, avoiding
    // an immediate CC state replacing the movement state.
    let knockback = sim.entity_knockback(
        caster_id,
        target_id,
        ULT_KNOCKBACK_SPEED,
        ULT_KNOCKBACK_TICKS,
    );
    sim.add_buff(caster_id, &BuffV1::timed(ULT_RECAST, ULT_WINDOW));
    let target_input = InputTargetV1::target(target_id);
    let airborne_queued = sim.queue_effect(
        "mlbb_heroes:chou_ult_airborne",
        AttackTypeV1::Skill,
        caster_id,
        &target_input,
        ULT_AIRBORNE_DELAY,
    );
    let recast_ready_queued = sim.queue_effect(
        "mlbb_heroes:chou_ult_recast_ready",
        AttackTypeV1::Skill,
        caster_id,
        &InputTargetV1::NONE,
        ULT_READY_DELAY,
    );
    log_chou_event(
        sim,
        caster_id,
        format!(
            "Ultimate first phase target#{target_id}: close_range={} raw_damage={damage} knockback={knockback} knockback_speed={} knockback_ticks={} caster_start=({},{}) target_start=({},{}) airborne_queued={airborne_queued} airborne_delay={ULT_AIRBORNE_DELAY} recast_ready_queued={recast_ready_queued} ready_delay={ULT_READY_DELAY}",
            ULT_FIRST_RANGE, ULT_KNOCKBACK_SPEED, ULT_KNOCKBACK_TICKS,
            caster_pos.0, caster_pos.1, target_pos.0, target_pos.1
        ),
    );
}

fn ultimate_airborne(sim: &mut StableSim<'_>, caster_id: usize, input: InputTargetV1) {
    if input.kind != InputTargetKindV1::Target.code()
        || !is_enemy_nonstructure(sim, caster_id, input.target_id)
    {
        log_chou_event(
            sim,
            caster_id,
            format!(
                "Ultimate airborne skipped: invalid/non-targetable target#{} kind={}",
                input.target_id, input.kind
            ),
        );
        return;
    }
    let target_pos = sim.get_entity(input.target_id).map(|target| target.pos());
    let airborne = CcV1::of_kind(CcKindV1::Airborne, ULT_AIRBORNE_TICKS);
    sim.apply_cc(input.target_id, &airborne);
    log_chou_event(
        sim,
        caster_id,
        format!(
            "Ultimate airborne applied to target#{} for {} ticks at_position={:?}",
            input.target_id, ULT_AIRBORNE_TICKS, target_pos
        ),
    );
}

fn ultimate_recast(sim: &mut StableSim<'_>, caster_id: usize, input: InputTargetV1) {
    if input.kind != InputTargetKindV1::Target.code()
        || !is_enemy_nonstructure(sim, caster_id, input.target_id)
    {
        log_chou_event(
            sim,
            caster_id,
            format!(
                "Ultimate recast rejected input kind={} target#{}",
                input.kind, input.target_id
            ),
        );
        return;
    }
    let target_id = input.target_id;
    let Some((caster_pos, target_pos, attack, level)) = sim
        .get_entity(caster_id)
        .zip(sim.get_entity(target_id))
        .map(|(caster, target)| {
            (
                caster.pos(),
                target.pos(),
                caster.stat().attack,
                caster.level(),
            )
        })
    else {
        return;
    };
    let total_damage = chou_ult_recast_damage(attack, level);
    // Conservative one-hit damage fallback: the recast flurry is fully shown
    // in the sprite animation, while damage/proc accounting remains a single
    // engine event until live TFM2 testing confirms safe multi-hit procs.
    sim.deal_damage(caster_id, target_id, total_damage, 0, AttackTypeV1::Skill);
    let toward = unit(
        target_pos.0 as f64 - caster_pos.0 as f64,
        target_pos.1 as f64 - caster_pos.1 as f64,
    );
    push_direction(sim, caster_id, toward, S2_DASH_DISTANCE, S2_DASH_SPEED);
    let _ = sim.entity_remove_buff(caster_id, ULT_RECAST);
    log_chou_event(
        sim,
        caster_id,
        format!("Ultimate recast executed on target#{target_id}; one damage event={total_damage}; dash issued; data airborne duration=60 ticks"),
    );
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ChouComboPhase {
    #[default]
    None,
    S1Chain,
    AwaitFinisher,
    AwaitPosition,
}

#[derive(Clone, Default)]
struct ChouAi {
    ult_target: Option<usize>,
    ult_expires: usize,
    s1_lock_until: usize,
    combo_lock_until: usize,
    combo_target: Option<usize>,
    combo_phase: ChouComboPhase,
    approach_s2_used: bool,
    failed_kick_target: Option<usize>,
    combo_retry_count: u8,
    combo_wait_count: u8,
}

impl ChouAi {
    fn clear_combo(&mut self) {
        self.combo_target = None;
        self.combo_phase = ChouComboPhase::None;
        self.approach_s2_used = false;
        self.combo_lock_until = 0;
        self.combo_retry_count = 0;
        self.combo_wait_count = 0;
    }
}

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
        init.champion_name.to_ascii_lowercase().contains("chou")
    }

    fn think(
        &mut self,
        ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        let player_id = ctx.player_id();
        let athlete_id = ctx.athlete_id();
        let tick = ctx.tick();
        if tick < self.s1_lock_until || tick < self.combo_lock_until {
            return None;
        }
        let (
            caster_id,
            team,
            pos,
            stage2,
            stage3,
            ult_recast,
            unsafe_base_input,
            retreat_from_tower,
            hold_position,
            skill_unlocked,
            skill2_unlocked,
            ult_unlocked,
            best_target,
        ) = {
            let sim = ctx.sim()?;
            let player = sim.get_player(player_id)?;
            let caster_id = player
                .champion()
                .map(|champion| champion.id())
                .unwrap_or(athlete_id);
            let entity = sim.get_entity(caster_id)?;
            let pos = entity.pos();
            let team = entity.team();
            let stage2 = crate::has_named_buff(&sim, caster_id, S1_STAGE2);
            let stage3 = crate::has_named_buff(&sim, caster_id, S1_STAGE3);
            let ult_recast = crate::has_named_buff(&sim, caster_id, ULT_RECAST);
            let all = crate::units(&sim);
            let Some(caster_snap) = all.iter().find(|unit| unit.id == caster_id) else {
                return None;
            };
            let unsafe_base_input = base_input.is_some_and(|input| {
                if input.kind == InputKindV1::Move.code() {
                    crate::is_unsafe_tower_route(
                        &all,
                        caster_snap,
                        (input.x as f64, input.y as f64),
                        0.0,
                    )
                } else if input.target.kind == InputTargetKindV1::Target.code() {
                    all.iter()
                        .find(|unit| unit.id == input.target.target_id)
                        .is_some_and(|target| {
                            target.team != team
                                && (crate::is_unsafe_under_enemy_tower(&all, caster_snap, target)
                                    || crate::is_unsafe_tower_route(
                                        &all,
                                        caster_snap,
                                        target.pos,
                                        0.0,
                                    ))
                        })
                } else if input.target.kind == InputTargetKindV1::Pos.code() {
                    crate::is_unsafe_tower_route(
                        &all,
                        caster_snap,
                        (input.target.x as f64, input.target.y as f64),
                        0.0,
                    )
                } else {
                    false
                }
            });
            let retreat_from_tower =
                crate::is_unsafe_tower_route(&all, caster_snap, caster_snap.pos, 0.0);
            let hold_position = InputV1::move_to(
                caster_snap.pos.0.max(0.0) as u64,
                caster_snap.pos.1.max(0.0) as u64,
            );
            let preferred = base_input
                .filter(|input| input.target.kind == InputTargetKindV1::Target.code())
                .map(|input| input.target.target_id);
            let mut best: Option<(usize, u128, bool)> = None;
            for i in 0..sim.entity_count() {
                let Some(enemy) = sim.entity_at(i) else {
                    continue;
                };
                if enemy.team() == team || !enemy.is_alive() || !enemy.is_targetable() {
                    continue;
                }
                if matches!(
                    enemy.kind(),
                    Some(EntityKindV1::Tower | EntityKindV1::Nexus)
                ) {
                    continue;
                }
                if all
                    .iter()
                    .find(|unit| unit.id == enemy.id())
                    .is_some_and(|target| {
                        crate::is_unsafe_under_enemy_tower(&all, caster_snap, target)
                            || crate::is_unsafe_tower_route(&all, caster_snap, target.pos, 0.0)
                    })
                {
                    continue;
                }
                let candidate = (
                    enemy.id(),
                    distance_sq(pos, enemy.pos()),
                    enemy.is_champion(),
                );
                if let Some((current_id, current_d2, current_champ)) = best {
                    if preferred == Some(candidate.0)
                        || (preferred != Some(current_id)
                            && (candidate.2 && !current_champ
                                || candidate.2 == current_champ && candidate.1 < current_d2))
                    {
                        best = Some(candidate);
                    }
                } else {
                    best = Some(candidate);
                }
            }
            (
                caster_id,
                team,
                pos,
                stage2,
                stage3,
                ult_recast,
                unsafe_base_input,
                retreat_from_tower,
                hold_position,
                entity
                    .cooldown(SkillSlotV1::Skill)
                    .is_some_and(|state| state.unlocked && state.charges > 0),
                entity
                    .cooldown(SkillSlotV1::Skill2)
                    .is_some_and(|state| state.unlocked && state.charges > 0),
                entity
                    .cooldown(SkillSlotV1::Ult)
                    .is_some_and(|state| state.unlocked && state.charges > 0),
                best.map(|v| v.0),
            )
        };
        // Do not pursue a target or movement order through an unsupported tower zone.
        if retreat_from_tower {
            return Some(InputV1::return_home());
        }
        if unsafe_base_input {
            return Some(hold_position);
        }

        // Recast the saved target only after the queued cooldown reset has run.
        if ult_recast && tick <= self.ult_expires && ult_unlocked {
            let max_range_sq = (ULT_TARGET_RANGE as u128) * (ULT_TARGET_RANGE as u128);
            if let Some(saved) = self.ult_target.filter(|id| {
                ctx.sim().is_some_and(|sim| {
                    sim.get_entity(*id).is_some_and(|entity| {
                        entity.is_alive()
                            && distance_sq(pos, entity.pos()) <= max_range_sq
                            && chou_tower_target_is_safe(&sim, caster_id, *id)
                    })
                })
            }) {
                crate::wlog(format!(
                    "Chou AI tick={tick}: issued Ultimate recast on saved target#{saved}"
                ));
                return Some(InputV1::action(
                    InputKindV1::Ult,
                    InputTargetV1::target(saved),
                ));
            }
        }
        // Hold the selected hero throughout an active combo instead of retargeting mid-sequence.
        let target_id = if let Some(locked) = self.combo_target {
            let valid = ctx.sim().is_some_and(|sim| {
                sim.get_entity(locked).is_some_and(|target| {
                    target.team() != team
                        && target.is_alive()
                        && target.is_targetable()
                        && target.is_champion()
                        && chou_tower_target_is_safe(&sim, caster_id, locked)
                })
            });
            if valid {
                Some(locked)
            } else {
                self.clear_combo();
                best_target
            }
        } else {
            best_target
        };
        let Some(target_id) = target_id else {
            return None;
        };
        let Some((target_pos, target_distance, target_is_champion)) =
            ctx.sim()?.get_entity(target_id).map(|target| {
                (
                    target.pos(),
                    (distance_sq(pos, target.pos()) as f64).sqrt(),
                    target.is_champion(),
                )
            })
        else {
            self.clear_combo();
            return None;
        };

        // Combo phase 1: finish the three S1 casts, optionally using the first S2
        // as a gap-close before the airborne finisher.
        if self.combo_phase == ChouComboPhase::S1Chain {
            if !ult_unlocked || !target_is_champion {
                self.clear_combo();
                return None;
            }
            if stage3 && !self.approach_s2_used && target_distance > 32_000.0 && skill2_unlocked {
                self.approach_s2_used = true;
                self.combo_lock_until = tick.saturating_add(S2_ACTION_LOCK);
                log_chou_ai(
                    ctx,
                    caster_id,
                    format!("combo S2 approach target#{target_id} distance={target_distance:.0}"),
                );
                return Some(InputV1::action(
                    InputKindV1::Skill2,
                    InputTargetV1::pos(target_pos.0, target_pos.1),
                ));
            }
            if stage3 {
                self.approach_s2_used = true;
            }
            let s1_recast_range = if stage3 {
                S1_FINISHER_APPROACH_RANGE
            } else {
                S1_RANGE
            };
            if (stage2 || stage3) && skill_unlocked && target_distance <= s1_recast_range {
                if stage3 {
                    self.combo_phase = ChouComboPhase::AwaitFinisher;
                }
                log_chou_ai(
                    ctx,
                    caster_id,
                    format!(
                        "combo S1 recast stage={} target#{target_id} distance={target_distance:.0}",
                        if stage3 { 3 } else { 2 }
                    ),
                );
                self.s1_lock_until = tick.saturating_add(S1_ACTION_LOCK + 2);
                return Some(InputV1::action(
                    InputKindV1::Skill,
                    InputTargetV1::pos(target_pos.0, target_pos.1),
                ));
            }
            if (stage2 || stage3) && !skill_unlocked && self.combo_wait_count < 5 {
                self.combo_wait_count += 1;
                self.combo_lock_until = tick.saturating_add(3);
                log_chou_ai(
                    ctx,
                    caster_id,
                    format!(
                        "combo waiting for S1 charge target#{target_id} stage2={stage2} stage3={stage3} wait={}",
                        self.combo_wait_count
                    ),
                );
                return None;
            }
            if !stage2 && !stage3 && skill_unlocked && self.combo_retry_count == 0 {
                self.combo_retry_count = 1;
                self.combo_wait_count = 0;
                self.s1_lock_until = tick.saturating_add(S1_ACTION_LOCK + 2);
                log_chou_ai(
                    ctx,
                    caster_id,
                    format!("combo retrying initial S1 target#{target_id} distance={target_distance:.0}"),
                );
                return Some(InputV1::action(
                    InputKindV1::Skill,
                    InputTargetV1::pos(target_pos.0, target_pos.1),
                ));
            }
            log_chou_ai(
                ctx,
                caster_id,
                format!(
                    "combo aborted before S1 finisher target#{target_id} distance={target_distance:.0} stage2={stage2} stage3={stage3} skill_ready={skill_unlocked} retries={}",
                    self.combo_retry_count
                ),
            );
            self.failed_kick_target = Some(target_id);
            self.clear_combo();
            return None;
        }

        // The impact helper marks a confirmed hero hit and resets Shunpo. Only then
        // spend the refreshed S2 to move to the far side of the chosen kick line.
        if self.combo_phase == ChouComboPhase::AwaitFinisher {
            let confirmed_hit = ctx
                .sim()
                .is_some_and(|sim| crate::has_named_buff(&sim, caster_id, S1_FINISHER_HIT));
            if !confirmed_hit || !skill2_unlocked {
                if (!confirmed_hit || !skill2_unlocked) && self.combo_wait_count < 5 {
                    self.combo_wait_count += 1;
                    self.combo_lock_until = tick.saturating_add(3);
                    log_chou_ai(
                        ctx,
                        caster_id,
                        format!(
                            "combo waiting after finisher target#{target_id} confirmed_hit={confirmed_hit} s2_ready={skill2_unlocked} wait={}",
                            self.combo_wait_count
                        ),
                    );
                    return None;
                }
                log_chou_ai(
                    ctx,
                    caster_id,
                    format!(
                        "combo aborted after S1 finisher target#{target_id} confirmed_hit={confirmed_hit} s2_ready={skill2_unlocked}"
                    ),
                );
                self.failed_kick_target = Some(target_id);
                self.clear_combo();
                return None;
            }
            let (goal_pos, goal_id) = ctx
                .sim()
                .map(|sim| kick_goal_position(&sim, caster_id, target_id, pos, target_pos))
                .unwrap_or((target_pos, None));
            let back_pos = point_behind_target(target_pos, goal_pos, S2_BACKSTEP_DISTANCE);
            log_chou_ai(
                ctx,
                caster_id,
                format!(
                    "combo S2 behind target#{target_id} goal_entity={goal_id:?} goal=({},{}) s2_aim=({},{})",
                    goal_pos.0, goal_pos.1, back_pos.0, back_pos.1
                ),
            );
            self.combo_phase = ChouComboPhase::AwaitPosition;
            self.combo_lock_until = tick.saturating_add(S2_ACTION_LOCK);
            return Some(InputV1::action(
                InputKindV1::Skill2,
                InputTargetV1::pos(back_pos.0, back_pos.1),
            ));
        }

        // Confirm the S2 actually put Chou on the correct side before kicking.
        if self.combo_phase == ChouComboPhase::AwaitPosition {
            let (goal_pos, goal_id) = ctx
                .sim()
                .map(|sim| kick_goal_position(&sim, caster_id, target_id, pos, target_pos))
                .unwrap_or((target_pos, None));
            let behind = is_on_kick_back_side(pos, target_pos, goal_pos);
            if !behind || target_distance > ULT_FIRST_AI_RANGE || !ult_unlocked {
                log_chou_ai(
                    ctx,
                    caster_id,
                    format!(
                        "combo kick cancelled target#{target_id}; behind={behind} distance={target_distance:.0} ai_range={ULT_FIRST_AI_RANGE:.0} ult_ready={ult_unlocked} goal_entity={goal_id:?} caster=({},{}) target=({},{})",
                        pos.0, pos.1, target_pos.0, target_pos.1
                    ),
                );
                self.failed_kick_target = Some(target_id);
                self.clear_combo();
                return None;
            }
            log_chou_ai(
                ctx,
                caster_id,
                format!(
                    "combo kick target#{target_id} toward goal_entity={goal_id:?} distance={target_distance:.0}"
                ),
            );
            self.ult_target = Some(target_id);
            self.ult_expires = tick.saturating_add(ULT_WINDOW);
            self.clear_combo();
            return Some(InputV1::action(
                InputKindV1::Ult,
                InputTargetV1::target(target_id),
            ));
        }

        // Preserve the existing staged S1 behavior when this isn't an Ultimate combo.
        let s1_recast_range = if stage3 {
            S1_FINISHER_APPROACH_RANGE
        } else {
            S1_RANGE
        };
        if (stage2 || stage3) && skill_unlocked && target_distance <= s1_recast_range {
            crate::wlog(format!(
                "Chou AI tick={tick}: issued S1 recast stage={} target#{target_id} distance={target_distance:.0}",
                if stage3 { 3 } else { 2 }
            ));
            self.s1_lock_until = tick.saturating_add(S1_ACTION_LOCK + 2);
            return Some(InputV1::action(
                InputKindV1::Skill,
                InputTargetV1::pos(target_pos.0, target_pos.1),
            ));
        }
        if ult_unlocked
            && !ult_recast
            && !stage2
            && !stage3
            && target_distance <= ULT_FIRST_AI_RANGE
            && self.failed_kick_target != Some(target_id)
            && !(target_is_champion && skill_unlocked)
        {
            crate::wlog(format!(
                "Chou AI tick={tick}: issued first Ultimate on target#{target_id} distance={target_distance:.0}"
            ));
            self.ult_target = Some(target_id);
            self.ult_expires = tick.saturating_add(ULT_WINDOW);
            return Some(InputV1::action(
                InputKindV1::Ult,
                InputTargetV1::target(target_id),
            ));
        }
        if !stage2 && !stage3 && skill_unlocked && target_distance <= S1_RANGE {
            if ult_unlocked && target_is_champion {
                self.combo_target = Some(target_id);
                self.combo_phase = ChouComboPhase::S1Chain;
                self.approach_s2_used = false;
                self.failed_kick_target = None;
                self.combo_retry_count = 0;
                self.combo_wait_count = 0;
                log_chou_ai(
                    ctx,
                    caster_id,
                    format!("started Insec combo target#{target_id} distance={target_distance:.0}"),
                );
            } else {
                crate::wlog(format!(
                    "Chou AI tick={tick}: issued S1 first cast target#{target_id} distance={target_distance:.0}"
                ));
            }
            self.s1_lock_until = tick.saturating_add(S1_ACTION_LOCK + 2);
            return Some(InputV1::action(
                InputKindV1::Skill,
                InputTargetV1::pos(target_pos.0, target_pos.1),
            ));
        }
        if skill2_unlocked && target_distance > 32_000.0 && target_distance <= 90_000.0 {
            return Some(InputV1::action(
                InputKindV1::Skill2,
                InputTargetV1::pos(target_pos.0, target_pos.1),
            ));
        }
        None
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
        ("chou_ult_airborne", ChouEffectKind::UltAirborne),
        ("chou_ult_recast_ready", ChouEffectKind::UltRecastReady),
        ("chou_ult_recast", ChouEffectKind::UltRecast),
        ("chou_ult_range", ChouEffectKind::UltRange),
    ] {
        module.add_native_effect(format!("mlbb_heroes:{name}"), ChouEffect { kind });
    }
    module.add_player_input_ai(ChouAi::default());
}

#[cfg(test)]
mod tests {
    use super::{
        chou_s1_damage, chou_ult_first_damage, chou_ult_recast_damage, extra_attack, level_value,
        ult_recast_range_bonus, unit,
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
        assert_eq!(super::S1_ACTION_LOCK, 45);
        assert_eq!(super::S1_FINISHER_IMPACT_DELAY + 2, 11);
        assert_eq!(super::ULT_FIRST_RANGE, 20_000.0);
        assert_eq!(super::ULT_FIRST_AI_RANGE, 18_000.0);
        assert_eq!(super::ULT_RECAST_RANGE_BONUS, 125_000);
        assert_eq!(ult_recast_range_bonus(false), 0);
        assert_eq!(ult_recast_range_bonus(true), 125_000);
        assert_eq!(
            super::ULT_FIRST_RANGE as u64 + ult_recast_range_bonus(true),
            145_000
        );
        assert_eq!(super::ULT_TARGET_RANGE, 145_000.0);
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
    fn experimental_ultimate_first_kick_displacement_is_100k() {
        assert_eq!(super::ULT_KNOCKBACK_SPEED, 10_000);
        assert_eq!(super::ULT_KNOCKBACK_TICKS, 10);
        assert_eq!(
            super::ULT_KNOCKBACK_SPEED * super::ULT_KNOCKBACK_TICKS,
            100_000
        );
        assert_eq!(super::S2_BACKSTEP_DISTANCE, 20_000.0);
    }

    #[test]
    fn direction_is_normalized() {
        let d = unit(3.0, 4.0);
        assert!((d.0 - 0.6).abs() < 1e-9);
        assert!((d.1 - 0.8).abs() < 1e-9);
    }
}
