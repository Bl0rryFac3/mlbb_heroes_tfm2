//! TFM2-balanced Hanabi implementation.
//!
//! Petal Blade jumps are homing projectiles. A short-lived pending-hit buff
//! carries each projectile's deterministic chain data to its native hit effect,
//! so damage and the next ricochet occur on arrival instead of all at cast time.
use mod_api_stable::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const MARK_BUFF_PREFIX: &str = "h_mark_";
const MARK_VIEW_BUFF: &str = "mlbb_heroes_hanabi_soul_mark";
const EQUINOX_BUFF: &str = "h_equinox_active";
const CC_GUARD_BUFF: &str = "h_shield_cc_guard";
const CHAIN_BUFF_PREFIX: &str = "hch_";
const PENDING_BUFF_PREFIX: &str = "hpend_";
const MARK_DURATION: usize = 300; // 5s
const EQUINOX_DURATION: usize = 300; // 5s
const MAX_PETAL_BOUNCES: usize = 4;
const PETAL_RANGE: f64 = 44_000.0;
const PETAL_PROJECTILE_SPEED: usize = 7_000;
const PETAL_PROJECTILE_RADIUS: usize = 6_000;

// ── Voice ────────────────────────────────────────────────────────────────────
// Owner's rule: voice on S1 / S2 / ult only, never on basic attack.
// The line names are what `speak()` passes; each resolves through
// `sound/sfx/<line>.sound_info` → `plays[].clip` → `<clip>.wav`, and each needs
// BOTH a `<line>` and a `mlbb_heroes_clip_<clip>` entry in mod.override_info or
// the engine never loads the file. (Arlott shipped silent for 3 versions for
// exactly that reason.)
const VOICE_SKILL1: &str = "mlbb_hanabi_skill_cast";
const VOICE_SKILL2: &str = "mlbb_hanabi_skill2_cast";
const VOICE_ULT: &str = "mlbb_hanabi_ult_cast";

/// Only the view contexts speak — the server sim must never emit sound.
fn speak(sim: &mut StableSim<'_>, caster: usize, line: &str) {
    let Some(origin) = sim.sim_origin() else {
        return;
    };
    let renders = SimOriginKindV1::from_code(origin.kind).is_some_and(|kind| {
        matches!(
            kind,
            SimOriginKindV1::ClientMatchView
                | SimOriginKindV1::ClientSpectate
                | SimOriginKindV1::ClientReplay
        )
    });
    if !renders {
        return;
    }
    let _ = crate::audio::play_voice(sim, caster, line);
}
const SOUL_SCROLL_RANGE: f64 = 86_000.0;
const SOUL_SCROLL_SPEED: usize = 6_000;
const ULT_RANGE: f64 = 100_000.0;
const ULT_SPEED: usize = 6_500;
const ULT_BLOOM_DELAY: usize = 48; // 0.8s
const ULT_BLOOM_RADIUS: f64 = 34_000.0;
const ULT_ROOT_TICKS: usize = 48; // 0.8s
const SOUL_SCROLL_SLOW_TICKS: usize = 60; // 1s
const SOUL_SCROLL_SLOW_PERCENT: i32 = 30;
const EQUINOX_SHIELD_BASE: usize = 120;
const EQUINOX_SHIELD_AD_PERCENT: usize = 35;
const EQUINOX_SHIELD_CAP_PERCENT: usize = 50;
const EQUINOX_HERO_CONVERSION_PERCENT: usize = 20;
const EQUINOX_UNIT_CONVERSION_PERCENT: usize = 10;
const EQUINOX_MOVE_SPEED_PERCENT: i32 = 12;
const EQUINOX_ATTACK_SPEED_PERCENT: i32 = 20;

// Petal-only diagnostics are capped independently by simulation origin so
// server pre-simulation cannot consume the live-match trace budget.
static HANABI_TRACE_COUNTS: [AtomicUsize; 5] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

fn trace_petal(sim: &StableSim<'_>, caster_id: usize, message: impl AsRef<str>) {
    let origin = sim.sim_origin().unwrap_or_default();
    let (bucket, label) = match SimOriginKindV1::from_code(origin.kind) {
        Some(SimOriginKindV1::ServerPresim) => (0, "presim"),
        Some(SimOriginKindV1::ClientMatchView) => (1, "match"),
        Some(SimOriginKindV1::ClientSpectate) => (2, "spectate"),
        Some(SimOriginKindV1::ClientReplay) => (3, "replay"),
        Some(SimOriginKindV1::Tool) => (4, "tool"),
        _ => (4, "unknown"),
    };
    let count = HANABI_TRACE_COUNTS[bucket].fetch_add(1, Ordering::Relaxed) + 1;
    if count > 500 && count % 500 != 0 {
        return;
    }
    let match_id = if origin.match_id == SimOriginV1::NONE {
        "-".to_string()
    } else {
        origin.match_id.to_string()
    };
    let set_index = if origin.set_index == SimOriginV1::NONE {
        "-".to_string()
    } else {
        origin.set_index.to_string()
    };
    super::wlog(format!(
        "[HANABI v5 {label} m={match_id} s={set_index} t={}] #{count} caster#{caster_id} {}",
        sim.tick(),
        message.as_ref()
    ));
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

fn pos_u64(value: f64) -> u64 {
    value.round().clamp(0.0, u64::MAX as f64) as u64
}

fn is_enemy(sim: &StableSim<'_>, caster_id: usize, target_id: usize) -> bool {
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

fn is_petal_target(sim: &StableSim<'_>, caster_id: usize, target_id: usize) -> bool {
    is_enemy(sim, caster_id, target_id)
        && sim
            .get_entity(target_id)
            .map_or(false, |target| !target.is_tower())
}

fn named_buff_count(sim: &StableSim<'_>, entity_id: usize, name: &str) -> usize {
    let Some(entity) = sim.get_entity(entity_id) else {
        return 0;
    };
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .filter(|buff| buff.name() == name)
        .count()
}

fn has_named_buff(sim: &StableSim<'_>, entity_id: usize, name: &str) -> bool {
    named_buff_count(sim, entity_id, name) > 0
}

fn mark_buff_name(caster_id: usize) -> String {
    format!("{MARK_BUFF_PREFIX}{caster_id}")
}

fn chain_buff_name(caster_id: usize, token: u64) -> String {
    format!("{CHAIN_BUFF_PREFIX}{caster_id}_{token:012x}")
}

fn chain_token(seed: u64, tick: usize, caster_id: usize, root_id: usize) -> u64 {
    const TOKEN_MASK: u64 = (1u64 << 46) - 1;
    let mut token = (tick as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ seed.rotate_left(17)
        ^ (caster_id as u64).rotate_left(31)
        ^ (root_id as u64).rotate_left(7);
    token &= TOKEN_MASK;
    if token == 0 {
        1
    } else {
        token
    }
}

// The packed projectile payload reserves 14 bits for raw trigger damage,
// 1 bit for Basic-Attack inheritance, 3 bits for the hop, and 46 bits for the
// unique chain token. TFM2 hero hit values are well inside this bound.
fn pack_pending(token: u64, hop: usize, basic_attack: bool, base_damage: usize) -> u64 {
    let token = token & ((1u64 << 46) - 1);
    (token << 18)
        | (((hop.min(7) as u64) & 7) << 15)
        | ((basic_attack as u64) << 14)
        | (base_damage.min(16_383) as u64)
}

fn unpack_pending(packed: u64) -> (u64, usize, bool, usize) {
    (
        packed >> 18,
        ((packed >> 15) & 7) as usize,
        ((packed >> 14) & 1) != 0,
        (packed & 0x3fff) as usize,
    )
}

fn petal_bounce_damage(base_damage: usize, hop: usize, prior_hits: usize) -> usize {
    if hop == 0 || hop > MAX_PETAL_BOUNCES {
        return 0;
    }
    let mut damage = base_damage.saturating_mul(35) / 100;
    for _ in 1..hop {
        damage = damage.saturating_mul(85) / 100;
    }
    for _ in 0..prior_hits {
        damage = damage.saturating_mul(70) / 100;
    }
    damage
}

fn marked_bounce_bonus(attack: usize) -> usize {
    20 + attack.saturating_mul(50) / 100
}

fn equinox_shield_value(max_hp: usize, attack: usize) -> usize {
    let cap = max_hp.saturating_mul(EQUINOX_SHIELD_CAP_PERCENT) / 100;
    (EQUINOX_SHIELD_BASE + attack.saturating_mul(EQUINOX_SHIELD_AD_PERCENT) / 100).min(cap)
}

fn buff_remaining(sim: &StableSim<'_>, entity_id: usize, name: &str) -> usize {
    let Some(entity) = sim.get_entity(entity_id) else {
        return 0;
    };
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .filter(|buff| buff.name() == name)
        .map(|buff| buff.duration_tick)
        .max()
        .unwrap_or(0)
}

fn hp_and_shield(sim: &StableSim<'_>, entity_id: usize) -> usize {
    sim.get_entity(entity_id)
        .map_or(0, |entity| entity.hp().0.saturating_add(entity.shield()))
}

/// Apply a physical hit, estimate its applied amount from the target's HP plus
/// shield before/after, then add Equinox's conversion shield if active.
fn damage_with_equinox(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    target_id: usize,
    raw_damage: usize,
    attack_type: AttackTypeV1,
) -> usize {
    if raw_damage == 0 || !is_enemy(sim, caster_id, target_id) {
        return 0;
    }
    let before = hp_and_shield(sim, target_id);
    sim.deal_damage(caster_id, target_id, raw_damage, 0, attack_type);
    let after = hp_and_shield(sim, target_id);
    let applied = before.saturating_sub(after);
    let remaining = buff_remaining(sim, caster_id, EQUINOX_BUFF);
    if applied == 0 || remaining == 0 {
        return applied;
    }
    let Some((max_hp, shield_now)) = sim
        .get_entity(caster_id)
        .map(|caster| (caster.hp().1, caster.shield()))
    else {
        return applied;
    };
    let target_is_hero = sim
        .get_entity(target_id)
        .map_or(false, |target| target.is_champion());
    let cap = max_hp.saturating_mul(EQUINOX_SHIELD_CAP_PERCENT) / 100;
    // Conservative layer-safe guard: count every shield source toward the
    // Equinox cap because the stable API does not expose individual shield IDs.
    let room = cap.saturating_sub(shield_now);
    let conversion = if target_is_hero {
        EQUINOX_HERO_CONVERSION_PERCENT
    } else {
        EQUINOX_UNIT_CONVERSION_PERCENT
    };
    let gain = (applied.saturating_mul(conversion) / 100).min(room);
    if gain > 0 {
        let _ = sim.entity_add_shield(caster_id, gain, remaining);
    }
    applied
}

/// All hops are still Hanabi's projectiles; the last hit target supplies only
/// the launch position, never the projectile's team/allegiance.
fn petal_projectile_team(caster_team: usize) -> usize {
    caster_team
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
    casting_target: CastingTargetV1,
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
        casting_target: casting_target.code(),
        target_x: pos_u64(end.0),
        target_y: pos_u64(end.1),
        ..ProjectileSpawnV1::default()
    };
    if let Some(target) = target_id {
        spec.move_kind = ProjectileMoveKindV1::Target.code();
        spec.target_id = target;
    } else {
        spec.move_kind = ProjectileMoveKindV1::Linear.code();
    }
    spec
}

fn aim_direction(
    sim: &StableSim<'_>,
    caster_id: usize,
    input: InputTargetV1,
) -> Option<(f64, f64)> {
    match input.kind {
        k if k == InputTargetKindV1::Target.code() => {
            let start = sim.get_entity(caster_id)?.pos();
            let end = sim.get_entity(input.target_id)?.pos();
            Some((end.0 as f64 - start.0 as f64, end.1 as f64 - start.1 as f64))
        }
        k if k == InputTargetKindV1::Pos.code() => {
            let start = sim.get_entity(caster_id)?.pos();
            Some((
                input.x as f64 - start.0 as f64,
                input.y as f64 - start.1 as f64,
            ))
        }
        k if k == InputTargetKindV1::Dir.code() => Some((input.dir_x as f64, input.dir_y as f64)),
        _ => None,
    }
}

fn add_chain_root(sim: &mut StableSim<'_>, caster_id: usize, root_id: usize, token: u64) {
    let name = chain_buff_name(caster_id, token);
    let buff = BuffV1::timed(&name, 90);
    let _ = sim.entity_stack_buff(root_id, &buff, MAX_PETAL_BOUNCES + 1, true);
}

fn set_mark(sim: &mut StableSim<'_>, caster_id: usize, target_id: usize) {
    let mark_name = mark_buff_name(caster_id);
    let _ = sim.entity_remove_buff(target_id, &mark_name);
    sim.add_buff(target_id, &BuffV1::timed(&mark_name, MARK_DURATION));
    let _ = sim.entity_remove_buff(target_id, MARK_VIEW_BUFF);
    sim.add_buff(target_id, &BuffV1::timed(MARK_VIEW_BUFF, MARK_DURATION));
    let _ = sim.play_view_effect(
        "hanabi_soul_mark_burst",
        caster_id,
        &InputTargetV1::target(target_id),
        0,
        0,
        18,
    );
}

fn consume_mark(sim: &mut StableSim<'_>, caster_id: usize, target_id: usize) -> bool {
    let mark_name = mark_buff_name(caster_id);
    if !has_named_buff(sim, target_id, &mark_name) {
        return false;
    }
    let _ = sim.entity_remove_buff(target_id, &mark_name);
    let _ = sim.entity_remove_buff(target_id, MARK_VIEW_BUFF);
    let _ = sim.play_view_effect(
        "hanabi_soul_mark_consume",
        caster_id,
        &InputTargetV1::target(target_id),
        0,
        0,
        18,
    );
    true
}

fn launch_petal(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    current_id: usize,
    next_id: usize,
    token: u64,
    hop: usize,
    basic_attack: bool,
    base_damage: usize,
) {
    if hop == 0 || hop > MAX_PETAL_BOUNCES || !is_petal_target(sim, caster_id, next_id) {
        return;
    }
    let Some(start) = sim.get_entity(current_id).map(|entity| entity.pos()) else {
        return;
    };
    let Some(caster_team) = sim.get_entity(caster_id).map(|entity| entity.team()) else {
        return;
    };
    let team = petal_projectile_team(caster_team);
    let from_team = sim
        .get_entity(current_id)
        .map_or(usize::MAX, |entity| entity.team());
    let Some(end) = sim.get_entity(next_id).map(|entity| {
        let (x, y) = entity.pos();
        (x as f64, y as f64)
    }) else {
        return;
    };
    let packed = pack_pending(token, hop, basic_attack, base_damage);
    let pending_name = format!("{PENDING_BUFF_PREFIX}{caster_id}_{packed:016x}");
    let _ = sim.entity_remove_buff(next_id, &pending_name);
    sim.add_buff(next_id, &BuffV1::timed(&pending_name, 90));
    let attack_type = if basic_attack {
        AttackTypeV1::BaseAttack
    } else {
        AttackTypeV1::Skill
    };
    let spec = projectile_spec(
        caster_id,
        team,
        start,
        end,
        PETAL_PROJECTILE_SPEED,
        PETAL_PROJECTILE_RADIUS,
        Some(next_id),
        attack_type,
        CastingTargetV1::Enemy,
    );
    let ok = sim.spawn_projectile("hanabi_petal_blade", "mlbb_heroes:hanabi_petal_hit", &spec);
    let target_team = sim
        .get_entity(next_id)
        .map_or(usize::MAX, |entity| entity.team());
    trace_petal(
        sim,
        caster_id,
        format!(
            "PETAL_SPAWN token={token:012x} hop={hop} from=#{current_id} from_team={from_team} to=#{next_id} target_team={target_team} projectile_team={team} distance={:.0} ok={ok}",
            distance(start, (end.0 as u64, end.1 as u64))
        ),
    );
    if !ok {
        let _ = sim.entity_remove_buff(next_id, &pending_name);
    }
}

fn select_petal_target(candidates: &[(usize, f64, bool)], current_id: usize) -> Option<usize> {
    let nearest = |unvisited_only: bool| {
        candidates
            .iter()
            .filter(|(id, _, visited)| *id != current_id && (!unvisited_only || !*visited))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _, _)| *id)
    };
    // First prefer enemies that this chain has not hit yet. If every eligible
    // enemy has been visited, repeats remain valid and retain their damage
    // falloff. Never return `current_id`: that would self-target the projectile.
    nearest(true).or_else(|| nearest(false))
}

fn next_petal_target(
    sim: &StableSim<'_>,
    caster_id: usize,
    current_id: usize,
    token: u64,
) -> Option<usize> {
    let caster_team = sim.get_entity(caster_id)?.team();
    let origin = sim.get_entity(current_id)?.pos();
    let chain_name = chain_buff_name(caster_id, token);
    let candidates: Vec<_> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|target| {
            target.id() != current_id
                && target.is_alive()
                && target.is_targetable()
                && target.team() != caster_team
                && !target.is_tower()
                && !matches!(target.kind(), Some(EntityKindV1::Nexus))
                && distance(origin, target.pos()) <= PETAL_RANGE
        })
        .map(|target| {
            (
                target.id(),
                distance(origin, target.pos()),
                named_buff_count(sim, target.id(), &chain_name) > 0,
            )
        })
        .collect();
    select_petal_target(&candidates, current_id)
}

fn bloom_impact_input(target_id: usize, position: (u64, u64)) -> InputTargetV1 {
    InputTargetV1 {
        kind: InputTargetKindV1::Pos.code(),
        target_id,
        x: position.0,
        y: position.1,
        ..InputTargetV1::NONE
    }
}

fn start_petal_chain(
    sim: &mut StableSim<'_>,
    seed: u64,
    caster_id: usize,
    root_id: usize,
    base_damage: usize,
    basic_attack: bool,
) {
    if base_damage == 0 || !is_petal_target(sim, caster_id, root_id) {
        return;
    }
    let token = chain_token(seed, sim.tick(), caster_id, root_id);
    add_chain_root(sim, caster_id, root_id, token);
    let Some(next_id) = next_petal_target(sim, caster_id, root_id, token) else {
        return;
    };
    launch_petal(
        sim,
        caster_id,
        root_id,
        next_id,
        token,
        1,
        basic_attack,
        base_damage,
    );
}

fn parse_pending_name(name: &str, caster_id: usize) -> Option<u64> {
    let prefix = format!("{PENDING_BUFF_PREFIX}{caster_id}_");
    name.strip_prefix(&prefix)
        .and_then(|packed| u64::from_str_radix(packed, 16).ok())
}

fn consume_first_pending(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    target_id: usize,
) -> Option<u64> {
    let (name, packed) = {
        let target = sim.get_entity(target_id)?;
        (0..target.buff_count())
            .filter_map(|index| target.buff_at(index))
            .filter_map(|buff| {
                parse_pending_name(buff.name(), caster_id)
                    .map(|packed| (buff.name().to_string(), packed))
            })
            .min_by_key(|(_, packed)| *packed)?
    };
    let _ = sim.entity_remove_buff(target_id, &name);
    Some(packed)
}

#[derive(Clone, Copy, Default)]
struct HanabiBasicAttack;
impl StableEffectType for HanabiBasicAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_enemy(sim, caster_id, input.target_id)
        {
            return;
        }
        let Some((start, team, alive)) = sim
            .get_entity(caster_id)
            .map(|entity| (entity.pos(), entity.team(), entity.is_alive()))
        else {
            return;
        };
        let Some(end) = sim.get_entity(input.target_id).map(|entity| {
            let (x, y) = entity.pos();
            (x as f64, y as f64)
        }) else {
            return;
        };
        if !alive {
            return;
        }
        let spec = projectile_spec(
            caster_id,
            team,
            start,
            end,
            7_000,
            6_000,
            Some(input.target_id),
            AttackTypeV1::BaseAttack,
            CastingTargetV1::Enemy,
        );
        let _ = sim.spawn_projectile(
            "hanabi_basic_higanbana",
            "mlbb_heroes:hanabi_basic_hit",
            &spec,
        );
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiBasicHit;
impl StableEffectType for HanabiBasicHit {
    fn apply(&self, sim: &mut StableSim<'_>, seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_enemy(sim, caster_id, input.target_id)
        {
            return;
        }
        let target_id = input.target_id;
        let attack = sim
            .get_entity(caster_id)
            .map_or(0, |entity| entity.stat().attack);
        let _ = damage_with_equinox(sim, caster_id, target_id, attack, AttackTypeV1::BaseAttack);
        start_petal_chain(sim, seed, caster_id, target_id, attack, true);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiPetalHit;
impl StableEffectType for HanabiPetalHit {
    fn apply(&self, sim: &mut StableSim<'_>, seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            trace_petal(
                sim,
                caster_id,
                format!(
                    "PETAL_HIT_REJECT reason=input_kind input_kind={}",
                    input.kind
                ),
            );
            return;
        }
        let target_id = input.target_id;
        if !is_petal_target(sim, caster_id, target_id) {
            trace_petal(
                sim,
                caster_id,
                format!("PETAL_HIT_REJECT reason=invalid_target target=#{target_id}"),
            );
            return;
        }
        let Some(packed) = consume_first_pending(sim, caster_id, target_id) else {
            trace_petal(
                sim,
                caster_id,
                format!("PETAL_PENDING_MISSING target=#{target_id}"),
            );
            return;
        };
        let (token, hop, basic_attack, base_damage) = unpack_pending(packed);
        if hop == 0 || hop > MAX_PETAL_BOUNCES {
            trace_petal(
                sim,
                caster_id,
                format!("PETAL_HIT_REJECT reason=invalid_hop token={token:012x} hop={hop}"),
            );
            return;
        }
        let chain_name = chain_buff_name(caster_id, token);
        let repeats = named_buff_count(sim, target_id, &chain_name);
        let mut damage = petal_bounce_damage(base_damage, hop, repeats);
        if basic_attack && consume_mark(sim, caster_id, target_id) {
            let attack = sim
                .get_entity(caster_id)
                .map_or(0, |entity| entity.stat().attack);
            damage = damage.saturating_add(marked_bounce_bonus(attack));
        }
        let attack_type = if basic_attack {
            AttackTypeV1::BaseAttack
        } else {
            AttackTypeV1::Skill
        };
        let applied = damage_with_equinox(sim, caster_id, target_id, damage, attack_type);
        let target_team = sim
            .get_entity(target_id)
            .map_or(usize::MAX, |entity| entity.team());
        let alive_after = sim
            .get_entity(target_id)
            .map_or(false, |entity| entity.is_alive());
        trace_petal(
            sim,
            caster_id,
            format!(
                "PETAL_HIT token={token:012x} hop={hop} target=#{target_id} target_team={target_team} basic={basic_attack} base={base_damage} repeats={repeats} raw={damage} applied_immediate={applied} alive_after={alive_after}"
            ),
        );
        if !alive_after {
            trace_petal(
                sim,
                caster_id,
                format!("CHAIN_STOP token={token:012x} reason=target_dead hop={hop}"),
            );
            let _ = seed;
            return;
        }
        let buff = BuffV1::timed(&chain_name, 90);
        let _ = sim.entity_stack_buff(target_id, &buff, MAX_PETAL_BOUNCES + 1, true);
        if hop >= MAX_PETAL_BOUNCES {
            trace_petal(
                sim,
                caster_id,
                format!("CHAIN_STOP token={token:012x} reason=max_hops hop={hop}"),
            );
        } else if let Some(next_id) = next_petal_target(sim, caster_id, target_id, token) {
            trace_petal(
                sim,
                caster_id,
                format!(
                    "CHAIN_NEXT token={token:012x} next_hop={} from=#{target_id} to=#{next_id}",
                    hop + 1
                ),
            );
            launch_petal(
                sim,
                caster_id,
                target_id,
                next_id,
                token,
                hop + 1,
                basic_attack,
                base_damage,
            );
        } else {
            trace_petal(
                sim,
                caster_id,
                format!(
                    "CHAIN_NO_TARGET token={token:012x} root=#{target_id} range={PETAL_RANGE:.0}"
                ),
            );
        }
        let _ = seed;
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack * 35 / 100, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiEquinox;
impl StableEffectType for HanabiEquinox {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, _input: InputTargetV1) {
        speak(sim, caster_id, VOICE_SKILL1);
        let Some((max_hp, attack, pos, alive)) = sim.get_entity(caster_id).map(|entity| {
            (
                entity.hp().1,
                entity.stat().attack,
                entity.pos(),
                entity.is_alive(),
            )
        }) else {
            return;
        };
        if !alive {
            return;
        }
        let _ = sim.entity_remove_buff(caster_id, EQUINOX_BUFF);
        let mut active = BuffV1::timed(EQUINOX_BUFF, EQUINOX_DURATION);
        active.move_speed_mult = EQUINOX_MOVE_SPEED_PERCENT;
        active.attack_speed_mult = EQUINOX_ATTACK_SPEED_PERCENT;
        sim.add_buff(caster_id, &active);
        let shield = equinox_shield_value(max_hp, attack);
        if shield > 0 {
            let _ = sim.entity_add_shield(caster_id, shield, EQUINOX_DURATION);
        }
        let input = InputTargetV1::target(caster_id);
        let _ = sim.play_view_effect(
            "hanabi_equinox_shield",
            caster_id,
            &input,
            0,
            0,
            EQUINOX_DURATION as u64,
        );
        let _ = pos;
    }
    fn expected_shield(&self, stat: &StatV1) -> usize {
        equinox_shield_value(stat.hp, stat.attack)
    }
    fn expected_buff(&self, _stat: &StatV1) -> Option<BuffV1> {
        let mut buff = BuffV1::timed(EQUINOX_BUFF, EQUINOX_DURATION);
        buff.move_speed_mult = EQUINOX_MOVE_SPEED_PERCENT;
        buff.attack_speed_mult = EQUINOX_ATTACK_SPEED_PERCENT;
        Some(buff)
    }
    fn on_caster(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiSoulScroll;
impl StableEffectType for HanabiSoulScroll {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        speak(sim, caster_id, VOICE_SKILL2);
        let Some((start, team, attack, alive)) = sim.get_entity(caster_id).map(|entity| {
            (
                entity.pos(),
                entity.team(),
                entity.stat().attack,
                entity.is_alive(),
            )
        }) else {
            return;
        };
        if !alive {
            return;
        }
        let Some(aim) = aim_direction(sim, caster_id, input) else {
            return;
        };
        let direction = unit(aim.0, aim.1);
        let end = (
            start.0 as f64 + direction.0 * SOUL_SCROLL_RANGE,
            start.1 as f64 + direction.1 * SOUL_SCROLL_RANGE,
        );
        let mut spec = projectile_spec(
            caster_id,
            team,
            start,
            end,
            SOUL_SCROLL_SPEED,
            8_000,
            None,
            AttackTypeV1::Skill,
            CastingTargetV1::EnemyWithoutTower,
        );
        spec.move_kind = ProjectileMoveKindV1::Linear.code();
        spec.target_x = pos_u64(end.0);
        spec.target_y = pos_u64(end.1);
        spec.penetrate = true;
        let _ = attack;
        let _ = sim.spawn_projectile(
            "hanabi_soul_scroll",
            "mlbb_heroes:hanabi_soul_scroll_hit",
            &spec,
        );
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (65 + stat.attack * 75 / 100, 0)
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiSoulScrollHit;
impl StableEffectType for HanabiSoulScrollHit {
    fn apply(&self, sim: &mut StableSim<'_>, seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_petal_target(sim, caster_id, input.target_id)
        {
            return;
        }
        let target_id = input.target_id;
        let Some((attack, damage)) = sim.get_entity(caster_id).map(|entity| {
            let attack = entity.stat().attack;
            (attack, 65 + attack * 75 / 100)
        }) else {
            return;
        };
        let _ = damage_with_equinox(sim, caster_id, target_id, damage, AttackTypeV1::Skill);
        let _ = sim.entity_remove_buff(target_id, "h_soul_scroll_slow");
        let mut slow = BuffV1::timed("h_soul_scroll_slow", SOUL_SCROLL_SLOW_TICKS);
        slow.move_speed_mult = -SOUL_SCROLL_SLOW_PERCENT;
        sim.add_buff(target_id, &slow);
        set_mark(sim, caster_id, target_id);
        start_petal_chain(sim, seed, caster_id, target_id, damage, false);
        let _ = attack;
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (65 + stat.attack * 75 / 100, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiUltimate;
impl StableEffectType for HanabiUltimate {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        speak(sim, caster_id, VOICE_ULT);
        let Some((start, team, alive)) = sim
            .get_entity(caster_id)
            .map(|entity| (entity.pos(), entity.team(), entity.is_alive()))
        else {
            return;
        };
        if !alive {
            return;
        }
        let Some(aim) = aim_direction(sim, caster_id, input) else {
            return;
        };
        let direction = unit(aim.0, aim.1);
        let end = (
            start.0 as f64 + direction.0 * ULT_RANGE,
            start.1 as f64 + direction.1 * ULT_RANGE,
        );
        let mut spec = projectile_spec(
            caster_id,
            team,
            start,
            end,
            ULT_SPEED,
            10_000,
            None,
            AttackTypeV1::Skill,
            CastingTargetV1::EnemyChampion,
        );
        spec.move_kind = ProjectileMoveKindV1::Linear.code();
        spec.penetrate = false;
        let _ = sim.spawn_projectile("hanabi_ult_higanbana", "mlbb_heroes:hanabi_ult_hit", &spec);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (150 + stat.attack * 80 / 100, 0)
    }
    fn can_move(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiUltimateHit;
impl StableEffectType for HanabiUltimateHit {
    fn apply(&self, sim: &mut StableSim<'_>, seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code()
            || !is_enemy(sim, caster_id, input.target_id)
        {
            return;
        }
        let target_id = input.target_id;
        let Some(impact_position) = sim.get_entity(target_id).map(|entity| entity.pos()) else {
            return;
        };
        let Some((attack, damage)) = sim.get_entity(caster_id).map(|entity| {
            let attack = entity.stat().attack;
            (attack, 150 + attack * 80 / 100)
        }) else {
            return;
        };
        let _ = damage_with_equinox(sim, caster_id, target_id, damage, AttackTypeV1::Skill);
        sim.apply_cc(
            target_id,
            &CcV1::of_kind(CcKindV1::Bind, ULT_ROOT_TICKS as u64),
        );
        start_petal_chain(sim, seed, caster_id, target_id, damage, false);
        let _ = sim.queue_effect(
            "mlbb_heroes:hanabi_ult_bloom_hit",
            AttackTypeV1::Skill,
            caster_id,
            &bloom_impact_input(target_id, impact_position),
            ULT_BLOOM_DELAY,
        );
        let _ = attack;
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (150 + stat.attack * 80 / 100, 0)
    }
    fn expected_cc_time(&self) -> Option<usize> {
        Some(ULT_ROOT_TICKS)
    }
}

#[derive(Clone, Copy, Default)]
struct HanabiUltimateBloom;
impl StableEffectType for HanabiUltimateBloom {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Pos.code() {
            return;
        }
        // The queued position is captured on projectile impact, so a moving
        // target cannot drag the delayed bloom away from the hit location.
        let center = (input.x, input.y);
        let primary_target_id = input.target_id;
        let Some((team, attack)) = sim
            .get_entity(caster_id)
            .map(|entity| (entity.team(), entity.stat().attack))
        else {
            return;
        };
        let center_input = InputTargetV1::pos(center.0, center.1);
        let _ = sim.play_view_effect(
            "hanabi_ult_bloom",
            caster_id,
            &center_input,
            0,
            ULT_BLOOM_RADIUS as u64,
            30,
        );
        for index in 0..sim.entity_count() {
            let Some(target) = sim.entity_at(index) else {
                continue;
            };
            let target_id = target.id();
            let valid = target.is_alive()
                && target.is_targetable()
                && target.team() != team
                && !target.is_tower()
                && !matches!(target.kind(), Some(EntityKindV1::Nexus))
                && distance(center, target.pos()) <= ULT_BLOOM_RADIUS;
            drop(target);
            if !valid {
                continue;
            }
            let damage = 105 + attack * 55 / 100;
            let _ = damage_with_equinox(sim, caster_id, target_id, damage, AttackTypeV1::Skill);
            if target_id != primary_target_id {
                let _ = sim.entity_remove_buff(target_id, "h_higanbana_bloom_slow");
                let mut slow = BuffV1::timed("h_higanbana_bloom_slow", 36);
                slow.move_speed_mult = -20;
                sim.add_buff(target_id, &slow);
            }
        }
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (105 + stat.attack * 55 / 100, 0)
    }
}

#[derive(Clone, Default)]
struct HanabiPassive;
impl StablePassive for HanabiPassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        let _ = sim.entity_remove_buff(entity_id, EQUINOX_BUFF);
        let _ = sim.entity_remove_buff(entity_id, CC_GUARD_BUFF);
        let mark = mark_buff_name(entity_id);
        let ids: Vec<usize> = (0..sim.entity_count())
            .filter_map(|index| sim.entity_at(index).map(|e| e.id()))
            .collect();
        for id in ids {
            let _ = sim.entity_remove_buff(id, &mark);
            let _ = sim.entity_remove_buff(id, MARK_VIEW_BUFF);
        }
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _seed: u64, _player: usize, entity_id: usize) {
        let shield = sim
            .get_entity(entity_id)
            .map_or(0, |entity| entity.shield());
        let _ = sim.entity_remove_buff(entity_id, CC_GUARD_BUFF);
        if shield > 0 {
            let mut guard = BuffV1::timed(CC_GUARD_BUFF, 2);
            guard.cc_immune = true;
            sim.add_buff(entity_id, &guard);
        }
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, _player: usize) {
        // Shield layers expire through the engine; the short passive guard is
        // removed on the next update once the dead body no longer reports shield.
        let _ = sim;
    }
}

pub fn register(module: &mut StableMod) {
    module.add_native_effect("mlbb_heroes:hanabi_basic_attack", HanabiBasicAttack);
    module.add_native_effect("mlbb_heroes:hanabi_basic_hit", HanabiBasicHit);
    module.add_native_effect("mlbb_heroes:hanabi_petal_hit", HanabiPetalHit);
    module.add_native_effect("mlbb_heroes:hanabi_equinox", HanabiEquinox);
    module.add_native_effect("mlbb_heroes:hanabi_soul_scroll", HanabiSoulScroll);
    module.add_native_effect("mlbb_heroes:hanabi_soul_scroll_hit", HanabiSoulScrollHit);
    module.add_native_effect("mlbb_heroes:hanabi_ultimate", HanabiUltimate);
    module.add_native_effect("mlbb_heroes:hanabi_ult_hit", HanabiUltimateHit);
    module.add_native_effect("mlbb_heroes:hanabi_ult_bloom_hit", HanabiUltimateBloom);
    module.add_native_passive("mlbb_heroes:hanabi_passive", HanabiPassive);
}

#[cfg(test)]
mod tests {
    use super::{
        bloom_impact_input, equinox_shield_value, marked_bounce_bonus, pack_pending,
        petal_bounce_damage, petal_projectile_team, select_petal_target, unpack_pending,
        InputTargetKindV1, MAX_PETAL_BOUNCES, VOICE_SKILL1, VOICE_SKILL2, VOICE_ULT,
    };

    #[test]
    fn every_voice_line_has_a_clip_on_disk_AND_both_override_entries() {
        // Hanabi shipped with NO voice at all — not even a speak() call — while
        // Arlott shipped with one and was still inaudible, because a sound asset
        // merely present in the tree is never loaded. Both halves are checked
        // here: the files, and the registration that makes the engine find them.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.clone());
            if !cur.pop() {
                break;
            }
        }
        let sfx = roots
            .iter()
            .map(|r| r.join("sound").join("sfx"))
            .find(|d| d.is_dir())
            .or_else(|| {
                roots
                    .iter()
                    .map(|r| r.join("franco_work").join("sfx"))
                    .find(|d| d.is_dir())
            })
            .unwrap_or_else(|| panic!("no sound dir found"));
        let overrides = roots
            .iter()
            .map(|r| r.join("mod.override_info"))
            .find(|p| p.is_file());

        for (line, clip) in [
            (VOICE_SKILL1, "mlbb_hanabi_sfx_vo_skill1"),
            (VOICE_SKILL2, "mlbb_hanabi_sfx_vo_skill2"),
            (VOICE_ULT, "mlbb_hanabi_sfx_vo_ult"),
        ] {
            let info = sfx.join(format!("{line}.sound_info"));
            assert!(info.is_file(), "{info:?} missing — speak() would be silent");
            let body = std::fs::read_to_string(&info).expect("read sound_info");
            assert!(body.contains(clip), "{line}.sound_info must name {clip}");
            let wav = sfx.join(format!("{clip}.wav"));
            assert!(wav.is_file(), "{wav:?} missing");
            assert!(
                std::fs::metadata(&wav).unwrap().len() > 1000,
                "{wav:?} empty"
            );

            if let Some(ov) = &overrides {
                let text = std::fs::read_to_string(ov).expect("read override_info");
                assert!(
                    text.contains(&format!("asset/base/sound/sfx/{line}")),
                    "{line} not in mod.override_info — Hanabi will be silent like Arlott was"
                );
                assert!(
                    text.contains(&format!("mlbb_heroes_clip_{clip}")),
                    "clip twin for {clip} not in mod.override_info"
                );
            }
        }
    }

    #[test]
    fn voice_is_on_skills_only_never_on_basic_attack() {
        // Owner's standing rule: S1 / S2 / ult, no hit SFX, no basic-attack voice.
        assert_eq!(VOICE_SKILL1, "mlbb_hanabi_skill_cast");
        assert_eq!(VOICE_SKILL2, "mlbb_hanabi_skill2_cast");
        assert_eq!(VOICE_ULT, "mlbb_hanabi_ult_cast");
    }

    #[test]
    fn petal_projectiles_keep_hanabis_team_even_when_launched_from_an_enemy() {
        assert_eq!(petal_projectile_team(0), 0);
        assert_eq!(petal_projectile_team(1), 1);
    }

    #[test]
    fn petal_bounce_damage_decays_by_hop_and_repeated_target() {
        assert_eq!(petal_bounce_damage(100, 1, 0), 35);
        assert_eq!(petal_bounce_damage(100, 2, 0), 29);
        assert_eq!(petal_bounce_damage(100, 3, 0), 24);
        assert_eq!(petal_bounce_damage(100, 1, 1), 24);
        assert_eq!(petal_bounce_damage(100, 1, 2), 16);
        assert_eq!(petal_bounce_damage(100, 5, 0), 0);
    }

    #[test]
    fn petal_targeting_prefers_unvisited_then_allows_falloff_repeats_without_self_targeting() {
        let candidates = [(2, 1.0, true), (3, 3.0, false), (4, 2.0, false)];
        assert_eq!(select_petal_target(&candidates, 1), Some(4));

        let all_visited = [(3, 0.0, true), (2, 2.0, true), (4, 1.0, true)];
        assert_eq!(select_petal_target(&all_visited, 3), Some(4));
        assert_eq!(select_petal_target(&[(9, 1.0, true)], 9), None);
    }

    #[test]
    fn delayed_bloom_payload_keeps_hit_position_and_primary_target() {
        let input = bloom_impact_input(17, (12_345, 67_890));
        assert_eq!(input.kind, InputTargetKindV1::Pos.code());
        assert_eq!(input.target_id, 17);
        assert_eq!((input.x, input.y), (12_345, 67_890));
    }

    #[test]
    fn marked_basic_bounce_bonus_is_separate_from_falloff() {
        assert_eq!(marked_bounce_bonus(54), 47);
        assert_eq!(marked_bounce_bonus(100), 70);
    }

    #[test]
    fn equinox_shield_uses_tfm2_values_and_caps_at_half_max_hp() {
        assert_eq!(equinox_shield_value(700, 53), 138);
        assert_eq!(equinox_shield_value(200, 100), 100);
        assert_eq!(equinox_shield_value(4_000, 100), 155);
    }

    #[test]
    fn projectile_payload_round_trips_chain_state() {
        let token = (1u64 << 45) + 123_456;
        for hop in 1..=MAX_PETAL_BOUNCES {
            let packed = pack_pending(token, hop, hop % 2 == 0, 900 + hop);
            let (decoded_token, decoded_hop, basic, base_damage) = unpack_pending(packed);
            assert_eq!(decoded_token, token);
            assert_eq!(decoded_hop, hop);
            assert_eq!(basic, hop % 2 == 0);
            assert_eq!(base_damage, 900 + hop);
        }
    }
}
