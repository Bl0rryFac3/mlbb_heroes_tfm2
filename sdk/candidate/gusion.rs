//! Native support for Gusion's split-phase dagger skills and four-rune passive.
use mod_api_stable::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const RUNE: &str = "mlbb_heroes_gusion_rune";
const S1_MARK: &str = "mlbb_heroes_gusion_s1_mark";
const S1_RECAST: &str = "mlbb_heroes_gusion_s1_recast";
const S2_RECAST: &str = "mlbb_heroes_gusion_s2_recast";
const S2_RESET_KEEP: &str = "mlbb_heroes_gusion_s2_reset_keep";
const ULT_RECAST: &str = "mlbb_heroes_gusion_ult_recast";
const MAX_RUNES: usize = 4;
const RUNE_DURATION: usize = 1200;
const S1_MARK_DURATION: usize = 300;
const S1_RECAST_DURATION: usize = 300;
const S2_RECAST_DURATION: usize = 240;
const S2_RECALL_READY_DELAY: usize = 36;
const ULT_RECAST_DURATION: usize = 150;
const S2_RANGE: f64 = 78_000.0;
const TOWER_DASH_BUFFER: f64 = 18_000.0;
const S2_SPREAD_DEGREES: [f64; 5] = [-22.0, -11.0, 0.0, 11.0, 22.0];
const S2_DAGGER: &str = "mlbb_gusion_s2_dagger";
const S2_TRAIL: &str = "mlbb_gusion_s2_trail";
const S1_DAGGER: &str = "mlbb_gusion_s1_dagger";
const PASSIVE_FX: &str = "mlbb_gusion_empowered_hit";
const DASH_FX: &str = "mlbb_gusion_dash_trail";

#[derive(Clone, Copy, Debug)]
struct DaggerVolley {
    origin: (f64, f64),
    direction: (f64, f64),
}

static S2_VOLLEYS: OnceLock<Mutex<HashMap<usize, Vec<DaggerVolley>>>> = OnceLock::new();
static S1_MARK_OWNERS: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
type S1PreflightKey = (u32, u64, u64, usize, usize);
static S1_PREFLIGHT_REJECTIONS: OnceLock<Mutex<HashMap<S1PreflightKey, (bool, bool)>>> =
    OnceLock::new();

fn dagger_volleys() -> &'static Mutex<HashMap<usize, Vec<DaggerVolley>>> {
    S2_VOLLEYS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn mark_owners() -> &'static Mutex<HashMap<usize, usize>> {
    S1_MARK_OWNERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn clear_s1_preflight_rejections(sim: &StableSim<'_>, caster_id: usize) {
    let origin = sim.sim_origin().unwrap_or_default();
    if let Ok(mut previous) = S1_PREFLIGHT_REJECTIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        previous.retain(|(kind, match_id, set_index, owner, _), _| {
            *kind != origin.kind
                || *match_id != origin.match_id
                || *set_index != origin.set_index
                || *owner != caster_id
        });
    }
}

fn clear_s1_preflight_rejection(sim: &StableSim<'_>, caster_id: usize, target_id: usize) {
    let origin = sim.sim_origin().unwrap_or_default();
    if let Ok(mut previous) = S1_PREFLIGHT_REJECTIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        previous.retain(|(kind, match_id, set_index, owner, target), _| {
            *kind != origin.kind
                || *match_id != origin.match_id
                || *set_index != origin.set_index
                || *owner != caster_id
                || *target != target_id
        });
    }
}

/// End a failed S1 recast phase without disturbing a mark owned by another Gusion.
/// A missing target clears this caster's outstanding mark-owner entries.
fn clear_s1_recast_state(sim: &mut StableSim<'_>, caster_id: usize, target_id: Option<usize>) {
    let _ = sim.entity_remove_buff(caster_id, S1_RECAST);
    clear_s1_preflight_rejections(sim, caster_id);
    let targets_to_clear = if let Ok(mut owners) = mark_owners().lock() {
        match target_id {
            Some(target_id) if owners.get(&target_id).copied() == Some(caster_id) => {
                owners.remove(&target_id);
                vec![target_id]
            }
            Some(_) => Vec::new(),
            None => {
                let targets: Vec<usize> = owners
                    .iter()
                    .filter_map(|(target_id, owner)| (*owner == caster_id).then_some(*target_id))
                    .collect();
                owners.retain(|_, owner| *owner != caster_id);
                targets
            }
        }
    } else {
        Vec::new()
    };
    for target_id in targets_to_clear {
        let _ = sim.entity_remove_buff(target_id, S1_MARK);
    }
}

fn log_gusion_event(sim: &StableSim<'_>, caster_id: usize, message: impl AsRef<str>) {
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
    let replay_id = if origin.replay_id == SimOriginV1::NONE {
        "-".to_string()
    } else {
        origin.replay_id.to_string()
    };
    let set_index = if origin.set_index == SimOriginV1::NONE {
        "-".to_string()
    } else {
        origin.set_index.to_string()
    };
    crate::wlog(format!(
        "Gusion [{kind} match={match_id} set={set_index} replay={replay_id} tick={}] caster#{caster_id}: {}",
        sim.tick(),
        message.as_ref()
    ));
}

fn log_s1_preflight_rejection(
    sim: &StableSim<'_>,
    caster: &crate::UnitSnap,
    target: &crate::UnitSnap,
    target_unsafe: bool,
    route_unsafe: bool,
    destination: (f64, f64),
) {
    let origin = sim.sim_origin().unwrap_or_default();
    let key = (
        origin.kind,
        origin.match_id,
        origin.set_index,
        caster.id,
        target.id,
    );
    let state = (target_unsafe, route_unsafe);
    let changed = S1_PREFLIGHT_REJECTIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .is_some_and(|mut previous| {
            if target_unsafe || route_unsafe {
                previous.insert(key, state) != Some(state)
            } else {
                previous.remove(&key);
                false
            }
        });
    if changed {
        log_gusion_event(
            sim,
            caster.id,
            format!(
                "S1 recast preflight blocked target#{} start=({:.0}, {:.0}) target=({:.0}, {:.0}) projected_end=({:.0}, {:.0}) target_tower_unsafe={} route_unsafe={}; no recast animation requested",
                target.id,
                caster.pos.0,
                caster.pos.1,
                target.pos.0,
                target.pos.1,
                destination.0,
                destination.1,
                target_unsafe,
                route_unsafe
            ),
        );
    }
}

fn log_gusion_ai(ctx: &mut StableAiContext<'_>, caster_id: usize, message: impl AsRef<str>) {
    if let Some(sim) = ctx.sim() {
        log_gusion_event(&sim, caster_id, message);
    } else {
        crate::wlog(format!(
            "Gusion AI caster#{caster_id}: {}",
            message.as_ref()
        ));
    }
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

fn has_buff(sim: &StableSim<'_>, entity_id: usize, name: &str) -> bool {
    named_buff_count(sim, entity_id, name) > 0
}

fn gusion_base_stat(level: usize, stack: usize) -> StatV1 {
    let growth = level.saturating_sub(1).min(14);
    StatV1 {
        attack: 58 + 4 * growth,
        magic_power: 78 + 8 * growth,
        hp: 740 + 96 * growth,
        defence: 20 + 3 * growth,
        magic_resistance: 20 + 2 * growth,
        move_speed: 1180 + 5 * growth,
        hp_regen: 6,
        stack: stack.min(MAX_RUNES),
        crit_chance: 0,
    }
}

fn set_rune_counter(sim: &mut StableSim<'_>, entity_id: usize, count: usize) {
    let level = sim
        .get_entity(entity_id)
        .map_or(1, |entity| entity.level().max(1));
    let _ = sim.entity_set_base_stat(entity_id, &gusion_base_stat(level, count));
}

fn add_rune(sim: &mut StableSim<'_>, entity_id: usize) {
    let buff = BuffV1::timed(RUNE, RUNE_DURATION);
    let count = sim.entity_stack_buff(entity_id, &buff, MAX_RUNES, true);
    set_rune_counter(sim, entity_id, count);
}

fn point_for_input(
    sim: &StableSim<'_>,
    caster_id: usize,
    input: InputTargetV1,
) -> Option<(f64, f64)> {
    match input.kind {
        kind if kind == InputTargetKindV1::Target.code() => {
            sim.get_entity(input.target_id).map(|entity| {
                let (x, y) = entity.pos();
                (x as f64, y as f64)
            })
        }
        kind if kind == InputTargetKindV1::Pos.code() => Some((input.x as f64, input.y as f64)),
        kind if kind == InputTargetKindV1::Dir.code() => {
            let (x, y) = sim.get_entity(caster_id)?.pos();
            let (dx, dy) = unit(input.dir_x as f64, input.dir_y as f64);
            Some((x as f64 + dx * S2_RANGE, y as f64 + dy * S2_RANGE))
        }
        _ => None,
    }
}

fn unit(dx: f64, dy: f64) -> (f64, f64) {
    let length = dx.hypot(dy);
    if !length.is_finite() || length < 1.0 {
        (1.0, 0.0)
    } else {
        (dx / length, dy / length)
    }
}

fn gusion_s1_dash_destination(start: (f64, f64), target: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = unit(target.0 - start.0, target.1 - start.1);
    (
        target.0 + dx * TOWER_DASH_BUFFER,
        target.1 + dy * TOWER_DASH_BUFFER,
    )
}

/// Return target-danger and full-route danger separately so both the AI
/// preflight and effect-time revalidation use exactly the same projection.
fn gusion_s1_dash_safety(
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    target: &crate::UnitSnap,
) -> (bool, bool, (f64, f64)) {
    let destination = gusion_s1_dash_destination(caster.pos, target.pos);
    let target_unsafe =
        crate::is_unsafe_under_enemy_tower_with_margin(all, caster, target, TOWER_DASH_BUFFER);
    let route_unsafe = crate::is_unsafe_tower_route(all, caster, destination, TOWER_DASH_BUFFER);
    (target_unsafe, route_unsafe, destination)
}

const TOWER_ESCAPE_CLEARANCE: f64 = TOWER_DASH_BUFFER;
const TOWER_ESCAPE_RELEASE_RADIUS: f64 = crate::TOWER_DANGER_RADIUS + TOWER_ESCAPE_CLEARANCE;
// If the native AI still proposes a route into tower danger at the normal exit,
// move one additional existing dash-safety buffer outward before waiting for a
// safe native route. This avoids releasing into an immediate hold at the edge.
const TOWER_ESCAPE_ROUTE_EXTENSION: f64 = TOWER_DASH_BUFFER;
// Keep the outward waypoint latched across one short safe-route confirmation
// window, so a native route that flips unsafe immediately after release is
// handled by the escape state rather than by the stationary fallback.
const TOWER_ESCAPE_RELEASE_CONFIRM_TICKS: u8 = 8;
const TOWER_ESCAPE_DIRECTION_STEPS: usize = 72;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GusionTowerEscape {
    caster_id: usize,
    tower_id: usize,
    sim_key: (u32, u64, u64, u64),
    waypoint: (u64, u64),
    release_radius: u64,
    used_fallback: bool,
    safe_route_ticks: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GusionTowerEscapeTransition {
    None,
    Entered,
    Retargeted,
    Extended,
    ReleasePending,
    Released,
}

fn gusion_tower_has_allied_tankers(
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    tower: &crate::UnitSnap,
) -> bool {
    all.iter()
        .filter(|unit| {
            unit.alive
                && unit.champion
                && unit.team == caster.team
                && unit.id != caster.id
                && crate::distance(unit.pos, tower.pos) <= crate::TOWER_DANGER_RADIUS * 0.92
        })
        .count()
        >= 2
}

fn gusion_unsupported_tower<'a>(
    all: &'a [crate::UnitSnap],
    caster: &crate::UnitSnap,
) -> Option<&'a crate::UnitSnap> {
    all.iter()
        .filter(|tower| {
            tower.tower
                && tower.alive
                && tower.team != caster.team
                && crate::distance(caster.pos, tower.pos) <= crate::TOWER_DANGER_RADIUS
                && !gusion_tower_has_allied_tankers(all, caster, tower)
        })
        .min_by(|a, b| {
            crate::distance(caster.pos, a.pos).total_cmp(&crate::distance(caster.pos, b.pos))
        })
}

fn escape_direction_is_outward(start: (f64, f64), end: (f64, f64), tower: (f64, f64)) -> bool {
    let from_tower = (start.0 - tower.0, start.1 - tower.1);
    let move_vec = (end.0 - start.0, end.1 - start.1);
    let start_distance = crate::distance(start, tower);
    let end_distance = crate::distance(end, tower);
    let radial_dot = from_tower.0 * move_vec.0 + from_tower.1 * move_vec.1;
    // Allow at most two world units of initial radial rounding error after
    // converting a floating waypoint to the game's integer coordinates.
    // The endpoint must still make net outward progress, including between
    // overlapping tower zones.
    radial_dot >= -2.0 * start_distance && end_distance > start_distance + 1.0
}

fn gusion_escape_angles(
    caster: &crate::UnitSnap,
    primary_tower: &crate::UnitSnap,
    hostile_towers: &[&crate::UnitSnap],
    fountain: (f64, f64),
) -> Vec<f64> {
    let radial = (
        caster.pos.0 - primary_tower.pos.0,
        caster.pos.1 - primary_tower.pos.1,
    );
    let base_angle = if radial.0.hypot(radial.1) > 1.0 {
        radial.1.atan2(radial.0)
    } else {
        (fountain.1 - primary_tower.pos.1).atan2(fountain.0 - primary_tower.pos.0)
    };
    let mut angles: Vec<f64> = (0..TOWER_ESCAPE_DIRECTION_STEPS)
        .map(|step| {
            base_angle
                + (step as f64) * (2.0 * std::f64::consts::PI / TOWER_ESCAPE_DIRECTION_STEPS as f64)
        })
        .collect();
    // Include exact tangent directions for every tower currently around Gusion.
    // They cover narrow safe wedges that a fixed angular sample can miss.
    for tower in hostile_towers
        .iter()
        .filter(|tower| crate::distance(caster.pos, tower.pos) <= crate::TOWER_DANGER_RADIUS)
    {
        let radial_angle = (caster.pos.1 - tower.pos.1).atan2(caster.pos.0 - tower.pos.0);
        angles.push(radial_angle + std::f64::consts::FRAC_PI_2);
        angles.push(radial_angle - std::f64::consts::FRAC_PI_2);
    }
    angles.push((fountain.1 - caster.pos.1).atan2(fountain.0 - caster.pos.0));
    angles
}

fn gusion_escape_route_valid_at_radius(
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    waypoint: (u64, u64),
    release_radius: f64,
) -> bool {
    let end = (waypoint.0 as f64, waypoint.1 as f64);
    let hostiles = all
        .iter()
        .filter(|unit| unit.tower && unit.alive && unit.team != caster.team);
    for tower in hostiles {
        let start_distance = crate::distance(caster.pos, tower.pos);
        let end_distance = crate::distance(end, tower.pos);
        if end_distance < release_radius - 2.0 {
            return false;
        }
        if start_distance <= crate::TOWER_DANGER_RADIUS {
            if !escape_direction_is_outward(caster.pos, end, tower.pos) {
                return false;
            }
        } else if crate::segment_distance(tower.pos, caster.pos, end) <= crate::TOWER_DANGER_RADIUS
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
fn gusion_escape_waypoint(
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    primary_tower: &crate::UnitSnap,
) -> ((u64, u64), bool) {
    gusion_escape_waypoint_at_radius(all, caster, primary_tower, TOWER_ESCAPE_RELEASE_RADIUS)
}

fn gusion_escape_waypoint_at_radius(
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    primary_tower: &crate::UnitSnap,
    goal_radius: f64,
) -> ((u64, u64), bool) {
    let bounds = crate::MOBA_BOUNDS;
    let fountain = if caster.team == 0 {
        (96_000.0, 864_000.0)
    } else {
        (864_000.0, 96_000.0)
    };
    let hostiles: Vec<&crate::UnitSnap> = all
        .iter()
        .filter(|unit| unit.tower && unit.alive && unit.team != caster.team)
        .collect();
    let angles = gusion_escape_angles(caster, primary_tower, &hostiles, fountain);
    let goal_radius = goal_radius.max(crate::TOWER_DANGER_RADIUS + 2.0);
    let mut candidates: Vec<(f64, f64, usize, (f64, f64))> = Vec::new();

    for (step, angle) in angles.iter().copied().enumerate() {
        let direction = (angle.cos(), angle.sin());
        let mut travel = 0.0_f64;
        let mut direction_valid = true;
        for tower in &hostiles {
            let from_tower = (caster.pos.0 - tower.pos.0, caster.pos.1 - tower.pos.1);
            let start_distance = from_tower.0.hypot(from_tower.1);
            let dot = from_tower.0 * direction.0 + from_tower.1 * direction.1;
            if start_distance <= crate::TOWER_DANGER_RADIUS && dot < -1.0e-4 {
                direction_valid = false;
                break;
            }
            if start_distance < goal_radius {
                let discriminant = (dot * dot + goal_radius * goal_radius
                    - start_distance * start_distance)
                    .max(0.0);
                travel = travel.max(-dot + discriminant.sqrt());
            }
        }
        if !direction_valid || !travel.is_finite() || travel <= 1.0 {
            continue;
        }
        let raw = (
            caster.pos.0 + direction.0 * travel,
            caster.pos.1 + direction.1 * travel,
        );
        let point = (
            raw.0.clamp(bounds.0, bounds.2),
            raw.1.clamp(bounds.1, bounds.3),
        );
        if crate::distance(point, primary_tower.pos) < goal_radius - 2.0 {
            continue;
        }
        if gusion_escape_route_valid_at_radius(
            all,
            caster,
            (pos_u64(point.0), pos_u64(point.1)),
            goal_radius,
        ) {
            candidates.push((travel, crate::distance(point, fountain), step, point));
        }
    }

    if let Some((_, _, _, point)) = candidates.into_iter().min_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.1.total_cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    }) {
        return ((pos_u64(point.0), pos_u64(point.1)), false);
    }

    // Geometrically crowded or map-edge fallback: maximize outward progress
    // from the currently threatened towers, then hostile-tower clearance.
    let mut fallback: Option<(f64, f64, f64, usize, (f64, f64))> = None;
    for (step, angle) in angles.iter().copied().enumerate() {
        let direction = (angle.cos(), angle.sin());
        let mut travel = 0.0_f64;
        let mut minimum_outward_dot = f64::INFINITY;
        for tower in &hostiles {
            let from_tower = (caster.pos.0 - tower.pos.0, caster.pos.1 - tower.pos.1);
            let start_distance = from_tower.0.hypot(from_tower.1);
            let dot = from_tower.0 * direction.0 + from_tower.1 * direction.1;
            if start_distance <= crate::TOWER_DANGER_RADIUS {
                minimum_outward_dot = minimum_outward_dot.min(dot);
            }
            if start_distance < goal_radius {
                let discriminant = (dot * dot + goal_radius * goal_radius
                    - start_distance * start_distance)
                    .max(0.0);
                travel = travel.max(-dot + discriminant.sqrt());
            }
        }
        if !travel.is_finite() || travel <= 1.0 {
            continue;
        }
        let raw = (
            caster.pos.0 + direction.0 * travel,
            caster.pos.1 + direction.1 * travel,
        );
        let point = (
            raw.0.clamp(bounds.0, bounds.2),
            raw.1.clamp(bounds.1, bounds.3),
        );
        let clearance = hostiles
            .iter()
            .map(|tower| crate::distance(point, tower.pos))
            .fold(f64::INFINITY, f64::min);
        let candidate = (minimum_outward_dot, clearance, -travel, step, point);
        let better = fallback.as_ref().is_none_or(|best| {
            candidate.0.total_cmp(&best.0).is_gt()
                || (candidate.0.total_cmp(&best.0).is_eq()
                    && (candidate.1.total_cmp(&best.1).is_gt()
                        || (candidate.1.total_cmp(&best.1).is_eq()
                            && candidate.2.total_cmp(&best.2).is_gt())))
        });
        if better {
            fallback = Some(candidate);
        }
    }
    let point = fallback.map(|entry| entry.4).unwrap_or_else(|| {
        let toward_home = unit(fountain.0 - caster.pos.0, fountain.1 - caster.pos.1);
        (
            (caster.pos.0 + toward_home.0 * goal_radius).clamp(bounds.0, bounds.2),
            (caster.pos.1 + toward_home.1 * goal_radius).clamp(bounds.1, bounds.3),
        )
    });
    ((pos_u64(point.0), pos_u64(point.1)), true)
}

fn update_gusion_tower_escape(
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    previous: Option<GusionTowerEscape>,
    sim_key: (u32, u64, u64, u64),
    unsafe_movement_route: bool,
) -> (Option<GusionTowerEscape>, GusionTowerEscapeTransition) {
    let previous =
        previous.filter(|state| state.caster_id == caster.id && state.sim_key == sim_key);
    let active_tower = gusion_unsupported_tower(all, caster);
    if let Some(active_tower) = active_tower {
        if let Some(state) = previous {
            if state.tower_id == active_tower.id
                && gusion_escape_route_valid_at_radius(
                    all,
                    caster,
                    state.waypoint,
                    state.release_radius as f64,
                )
            {
                return (
                    Some(GusionTowerEscape {
                        safe_route_ticks: 0,
                        ..state
                    }),
                    GusionTowerEscapeTransition::None,
                );
            }
            let previous_tower_still_unsafe = all.iter().find(|tower| {
                tower.id == state.tower_id
                    && tower.tower
                    && tower.alive
                    && tower.team != caster.team
                    && crate::distance(caster.pos, tower.pos) <= crate::TOWER_DANGER_RADIUS
                    && !gusion_tower_has_allied_tankers(all, caster, tower)
            });
            if previous_tower_still_unsafe.is_some()
                && gusion_escape_route_valid_at_radius(
                    all,
                    caster,
                    state.waypoint,
                    state.release_radius as f64,
                )
            {
                return (
                    Some(GusionTowerEscape {
                        safe_route_ticks: 0,
                        ..state
                    }),
                    GusionTowerEscapeTransition::None,
                );
            }
        }
        let release_radius = previous
            .map(|state| state.release_radius)
            .unwrap_or_else(|| pos_u64(TOWER_ESCAPE_RELEASE_RADIUS));
        let (waypoint, used_fallback) =
            gusion_escape_waypoint_at_radius(all, caster, active_tower, release_radius as f64);
        let state = GusionTowerEscape {
            caster_id: caster.id,
            tower_id: active_tower.id,
            sim_key,
            waypoint,
            release_radius,
            used_fallback,
            safe_route_ticks: 0,
        };
        let transition = if previous.is_some() {
            GusionTowerEscapeTransition::Retargeted
        } else {
            GusionTowerEscapeTransition::Entered
        };
        return (Some(state), transition);
    }

    let Some(state) = previous else {
        return (None, GusionTowerEscapeTransition::None);
    };
    let Some(tower) = all.iter().find(|tower| {
        tower.id == state.tower_id && tower.tower && tower.alive && tower.team != caster.team
    }) else {
        return (None, GusionTowerEscapeTransition::Released);
    };
    let current_distance = crate::distance(caster.pos, tower.pos);
    if current_distance < state.release_radius as f64 - 2.0 {
        if gusion_escape_route_valid_at_radius(
            all,
            caster,
            state.waypoint,
            state.release_radius as f64,
        ) {
            return (
                Some(GusionTowerEscape {
                    safe_route_ticks: 0,
                    ..state
                }),
                GusionTowerEscapeTransition::None,
            );
        }
        let (waypoint, used_fallback) =
            gusion_escape_waypoint_at_radius(all, caster, tower, state.release_radius as f64);
        return (
            Some(GusionTowerEscape {
                waypoint,
                used_fallback,
                safe_route_ticks: 0,
                ..state
            }),
            GusionTowerEscapeTransition::Retargeted,
        );
    }

    // Do not release at the ordinary radius if the native AI is already
    // proposing a route back through unsupported tower danger. Extend outward
    // by one established dash-safety buffer, then keep the waypoint latched
    // until the base route becomes safe or the tower disappears.
    if unsafe_movement_route {
        if state.release_radius as f64 <= TOWER_ESCAPE_RELEASE_RADIUS + 1.0 {
            let extended_radius = (state.release_radius as f64 + TOWER_ESCAPE_ROUTE_EXTENSION)
                .max(current_distance + TOWER_ESCAPE_ROUTE_EXTENSION);
            let (waypoint, used_fallback) =
                gusion_escape_waypoint_at_radius(all, caster, tower, extended_radius);
            return (
                Some(GusionTowerEscape {
                    waypoint,
                    release_radius: pos_u64(extended_radius),
                    used_fallback,
                    safe_route_ticks: 0,
                    ..state
                }),
                GusionTowerEscapeTransition::Extended,
            );
        }
        return (
            Some(GusionTowerEscape {
                safe_route_ticks: 0,
                ..state
            }),
            GusionTowerEscapeTransition::None,
        );
    }

    // Require a brief consecutive safe-route window before releasing. If the
    // native route flips unsafe during this window, the escape remains active
    // and the existing outward extension path can take over.
    let next_safe_route_ticks = state.safe_route_ticks.saturating_add(1);
    if next_safe_route_ticks < TOWER_ESCAPE_RELEASE_CONFIRM_TICKS {
        let transition = if state.safe_route_ticks == 0 {
            GusionTowerEscapeTransition::ReleasePending
        } else {
            GusionTowerEscapeTransition::None
        };
        return (
            Some(GusionTowerEscape {
                safe_route_ticks: next_safe_route_ticks,
                ..state
            }),
            transition,
        );
    }

    (None, GusionTowerEscapeTransition::Released)
}

fn pos_u64(value: f64) -> u64 {
    value.round().clamp(0.0, u64::MAX as f64) as u64
}

fn named_input(input: InputTargetV1) -> InputTargetV1 {
    input
}

fn projectile_spec(
    caster_id: usize,
    team: usize,
    start: (f64, f64),
    end: (f64, f64),
    speed: usize,
    radius: usize,
    penetrate: bool,
) -> ProjectileSpawnV1 {
    ProjectileSpawnV1 {
        caster_id,
        team,
        x: pos_u64(start.0),
        y: pos_u64(start.1),
        radius: radius as u64,
        speed: speed as u64,
        move_kind: ProjectileMoveKindV1::Linear.code(),
        target_x: pos_u64(end.0),
        target_y: pos_u64(end.1),
        penetrate,
        attack_type: AttackTypeV1::Skill.code(),
        casting_type: CastingTypeV1::Targeting.code(),
        casting_target: CastingTargetV1::Enemy.code(),
        ..ProjectileSpawnV1::default()
    }
}

pub fn register(module: &mut StableMod) {
    module.add_native_effect("mlbb_heroes:gusion_basic_attack", GusionBasicAttack);
    module.add_native_effect("mlbb_heroes:gusion_s1_throw", GusionS1Throw);
    module.add_native_effect("mlbb_heroes:gusion_s1_hit", GusionS1Hit);
    module.add_native_effect("mlbb_heroes:gusion_s1_recast", GusionS1Recast);
    module.add_native_effect("mlbb_heroes:gusion_s2_throw", GusionS2Throw);
    module.add_native_effect("mlbb_heroes:gusion_s2_hit", GusionS2Hit);
    module.add_native_effect("mlbb_heroes:gusion_s2_recall", GusionS2Recall);
    module.add_native_effect("mlbb_heroes:gusion_s2_recall_hit", GusionS2RecallHit);
    module.add_native_effect("mlbb_heroes:gusion_s2_recall_ready", GusionS2RecallReady);
    module.add_native_effect("mlbb_heroes:gusion_ult_start", GusionUltStart);
    module.add_native_effect("mlbb_heroes:gusion_ult_second", GusionUltSecond);
    module.add_native_effect("mlbb_heroes:gusion_visual_noop", GusionVisualNoop);
    module.add_native_passive("mlbb_heroes:gusion_passive", GusionPassive::default());
    module.add_player_input_ai(GusionComboAi::default());
}

#[derive(Clone, Copy, Default)]
struct GusionBasicAttack;

impl StableEffectType for GusionBasicAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target_id = input.target_id;
        let Some(caster) = sim.get_entity(caster_id) else {
            return;
        };
        let attack = caster.stat().attack;
        let magic_power = caster.stat().magic_power;
        let stacks = named_buff_count(sim, caster_id, RUNE).min(MAX_RUNES);
        let Some(target) = sim.get_entity(target_id) else {
            return;
        };
        if !target.is_alive() || !target.is_targetable() || target.team() == caster.team() {
            return;
        }
        let empowered = stacks >= MAX_RUNES;
        let target_max_hp = target.hp().1;
        let bonus_magic = if empowered {
            target_max_hp.saturating_mul(3 * stacks) / 100
        } else {
            0
        };
        sim.deal_damage(
            caster_id,
            target_id,
            attack,
            bonus_magic,
            AttackTypeV1::BaseAttack,
        );
        if empowered {
            let heal = stacks.saturating_mul(50 + magic_power.saturating_mul(25) / 100);
            sim.heal(caster_id, caster_id, heal);
            let _ = sim.entity_remove_buff(caster_id, RUNE);
            set_rune_counter(sim, caster_id, 0);
            let _ = sim.play_view_effect(PASSIVE_FX, caster_id, &named_input(input), 0, 0, 30);
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (stat.attack, 0)
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS1Throw;

impl StableEffectType for GusionS1Throw {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let Some((x, y, team)) = sim.get_entity(caster_id).map(|entity| {
            let (x, y) = entity.pos();
            (x, y, entity.team())
        }) else {
            return;
        };
        let Some(point) = point_for_input(sim, caster_id, input) else {
            return;
        };
        add_rune(sim, caster_id);
        let mut spec = projectile_spec(
            caster_id,
            team,
            (x as f64, y as f64),
            point,
            9_200,
            7_000,
            false,
        );
        if input.kind == InputTargetKindV1::Target.code() {
            spec.move_kind = ProjectileMoveKindV1::Target.code();
            spec.target_id = input.target_id;
        }
        let _ = sim.spawn_projectile(S1_DAGGER, "mlbb_heroes:gusion_s1_hit", &spec);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, 200 + stat.magic_power * 50 / 100)
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS1Hit;

impl StableEffectType for GusionS1Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target_id = input.target_id;
        let Some((ap, team, alive)) = sim
            .get_entity(caster_id)
            .map(|entity| (entity.stat().magic_power, entity.team(), entity.is_alive()))
        else {
            return;
        };
        let Some(target) = sim.get_entity(target_id) else {
            return;
        };
        if !alive || !target.is_alive() || !target.is_targetable() || target.team() == team {
            return;
        }
        sim.deal_damage(
            caster_id,
            target_id,
            0,
            200 + ap * 50 / 100,
            AttackTypeV1::Skill,
        );
        let _ = sim.entity_remove_buff(target_id, S1_MARK);
        sim.add_buff(target_id, &BuffV1::timed(S1_MARK, S1_MARK_DURATION));
        if let Ok(mut owners) = mark_owners().lock() {
            owners.insert(target_id, caster_id);
        }
        let _ = sim.entity_remove_buff(caster_id, S1_RECAST);
        sim.add_buff(caster_id, &BuffV1::timed(S1_RECAST, S1_RECAST_DURATION));
        let _ = sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Skill, 9_999);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, 200 + stat.magic_power * 50 / 100)
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS1Recast;

impl StableEffectType for GusionS1Recast {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            clear_s1_recast_state(sim, caster_id, None);
            log_gusion_event(
                sim,
                caster_id,
                "S1 recast effect rejected: invalid input; cleared pending mark/recast state",
            );
            return;
        }
        let target_id = input.target_id;
        let marked_by_caster = mark_owners()
            .lock()
            .ok()
            .and_then(|owners| owners.get(&target_id).copied())
            == Some(caster_id);
        if !marked_by_caster || !has_buff(sim, target_id, S1_MARK) {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            log_gusion_event(
                sim,
                caster_id,
                format!(
                    "S1 recast effect rejected: target#{target_id} lacks this caster's live mark; cleared pending recast state"
                ),
            );
            return;
        }
        let Some((start, team, ap)) = sim
            .get_entity(caster_id)
            .map(|caster| (caster.pos(), caster.team(), caster.stat().magic_power))
        else {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            return;
        };
        let Some((end, target_valid)) = sim.get_entity(target_id).map(|target| {
            (
                target.pos(),
                target.is_alive() && target.is_targetable() && target.team() != team,
            )
        }) else {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            log_gusion_event(
                sim,
                caster_id,
                format!(
                    "S1 recast effect rejected: target#{target_id} disappeared; cleared pending recast state"
                ),
            );
            return;
        };
        if !target_valid {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            log_gusion_event(
                sim,
                caster_id,
                format!(
                    "S1 recast effect rejected: target#{target_id} is dead, untargetable, or allied; cleared pending recast state"
                ),
            );
            return;
        }
        let all = crate::units(sim);
        let Some(caster_snap) = all.iter().find(|unit| unit.id == caster_id) else {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            return;
        };
        let Some(target_snap) = all.iter().find(|unit| unit.id == target_id) else {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            return;
        };
        let (target_unsafe, route_unsafe, destination) =
            gusion_s1_dash_safety(&all, caster_snap, target_snap);
        if target_unsafe || route_unsafe {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            log_gusion_event(
                sim,
                caster_id,
                format!(
                    "S1 recast effect rejected after cast input: target#{target_id} start=({}, {}) target=({}, {}) projected_end=({:.0}, {:.0}) target_tower_unsafe={target_unsafe} route_unsafe={route_unsafe}; cleared pending mark/recast state",
                    start.0, start.1, end.0, end.1, destination.0, destination.1
                ),
            );
            return;
        }
        let moved = sim.entity_set_pos(caster_id, pos_u64(destination.0), pos_u64(destination.1));
        if !moved {
            clear_s1_recast_state(sim, caster_id, Some(target_id));
            log_gusion_event(
                sim,
                caster_id,
                format!(
                    "S1 recast effect failed: safe route but teleport position update failed for target#{target_id}; cleared pending state"
                ),
            );
            return;
        }
        add_rune(sim, caster_id);
        sim.deal_damage(caster_id, target_id, 0, 200 + ap, AttackTypeV1::Skill);
        clear_s1_recast_state(sim, caster_id, Some(target_id));
        let _ = sim.play_view_effect(DASH_FX, caster_id, &named_input(input), 0, 0, 24);
        log_gusion_event(
            sim,
            caster_id,
            format!(
                "S1 recast succeeded target#{target_id} projected_end=({:.0}, {:.0}) dash_visual_ticks=24",
                destination.0, destination.1
            ),
        );
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, 200 + stat.magic_power)
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS2Throw;

impl StableEffectType for GusionS2Throw {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let Some((cx, cy, team)) = sim.get_entity(caster_id).map(|entity| {
            let (x, y) = entity.pos();
            (x as f64, y as f64, entity.team())
        }) else {
            return;
        };
        let Some(aim) = point_for_input(sim, caster_id, input) else {
            return;
        };
        let direction = unit(aim.0 - cx, aim.1 - cy);
        add_rune(sim, caster_id);
        let preserve_first_volley = has_buff(sim, caster_id, S2_RESET_KEEP);
        let _ = sim.entity_remove_buff(caster_id, S2_RESET_KEEP);
        if let Ok(mut volleys) = dagger_volleys().lock() {
            let stored = volleys.entry(caster_id).or_default();
            if !preserve_first_volley {
                stored.clear();
            }
            stored.push(DaggerVolley {
                origin: (cx, cy),
                direction,
            });
        }
        let _ = sim.entity_remove_buff(caster_id, S2_RECAST);
        sim.add_buff(caster_id, &BuffV1::timed(S2_RECAST, S2_RECAST_DURATION));
        let _ = sim.queue_effect(
            "mlbb_heroes:gusion_s2_recall_ready",
            AttackTypeV1::Skill,
            caster_id,
            &InputTargetV1::NONE,
            S2_RECALL_READY_DELAY,
        );
        for degrees in S2_SPREAD_DEGREES {
            let radians = degrees.to_radians();
            let d = (
                direction.0 * radians.cos() - direction.1 * radians.sin(),
                direction.0 * radians.sin() + direction.1 * radians.cos(),
            );
            let end = (cx + d.0 * S2_RANGE, cy + d.1 * S2_RANGE);
            let dagger = projectile_spec(caster_id, team, (cx, cy), end, 2_300, 6_500, false);
            let trail = projectile_spec(caster_id, team, (cx, cy), end, 1_700, 1_000, true);
            let _ = sim.spawn_projectile(S2_DAGGER, "mlbb_heroes:gusion_s2_hit", &dagger);
            let _ = sim.spawn_projectile(S2_TRAIL, "mlbb_heroes:gusion_visual_noop", &trail);
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, 5 * (65 + stat.magic_power * 40 / 100))
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS2Hit;

impl StableEffectType for GusionS2Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target_id = input.target_id;
        let Some((ap, team, alive)) = sim
            .get_entity(caster_id)
            .map(|entity| (entity.stat().magic_power, entity.team(), entity.is_alive()))
        else {
            return;
        };
        let Some(target) = sim.get_entity(target_id) else {
            return;
        };
        if !alive || !target.is_alive() || !target.is_targetable() || target.team() == team {
            return;
        }
        sim.deal_damage(
            caster_id,
            target_id,
            0,
            65 + ap * 40 / 100,
            AttackTypeV1::Skill,
        );
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, 65 + stat.magic_power * 40 / 100)
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS2Recall;

impl StableEffectType for GusionS2Recall {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let Some((cx, cy, team)) = sim.get_entity(caster_id).map(|entity| {
            let (x, y) = entity.pos();
            (x as f64, y as f64, entity.team())
        }) else {
            return;
        };
        let volleys = dagger_volleys()
            .lock()
            .ok()
            .and_then(|mut stored| stored.remove(&caster_id))
            .unwrap_or_default();
        let volleys = if volleys.is_empty() {
            let direction = point_for_input(sim, caster_id, input)
                .map(|aim| unit(aim.0 - cx, aim.1 - cy))
                .unwrap_or((1.0, 0.0));
            vec![DaggerVolley {
                origin: (cx, cy),
                direction,
            }]
        } else {
            volleys
        };
        let _ = sim.entity_remove_buff(caster_id, S2_RECAST);
        add_rune(sim, caster_id);
        for volley in volleys {
            for degrees in S2_SPREAD_DEGREES {
                let radians = degrees.to_radians();
                let d = (
                    volley.direction.0 * radians.cos() - volley.direction.1 * radians.sin(),
                    volley.direction.0 * radians.sin() + volley.direction.1 * radians.cos(),
                );
                let start = (
                    volley.origin.0 + d.0 * S2_RANGE,
                    volley.origin.1 + d.1 * S2_RANGE,
                );
                let dagger = projectile_spec(caster_id, team, start, (cx, cy), 5_500, 6_500, false);
                let trail = projectile_spec(caster_id, team, start, (cx, cy), 4_000, 1_000, true);
                let _ =
                    sim.spawn_projectile(S2_DAGGER, "mlbb_heroes:gusion_s2_recall_hit", &dagger);
                let _ = sim.spawn_projectile(S2_TRAIL, "mlbb_heroes:gusion_visual_noop", &trail);
            }
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, 5 * (65 + stat.magic_power * 40 / 100))
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS2RecallHit;

impl StableEffectType for GusionS2RecallHit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target_id = input.target_id;
        let Some((ap, team, alive)) = sim
            .get_entity(caster_id)
            .map(|entity| (entity.stat().magic_power, entity.team(), entity.is_alive()))
        else {
            return;
        };
        let Some(target) = sim.get_entity(target_id) else {
            return;
        };
        if !alive || !target.is_alive() || !target.is_targetable() || target.team() == team {
            return;
        }
        sim.deal_damage(
            caster_id,
            target_id,
            0,
            65 + ap * 40 / 100,
            AttackTypeV1::Skill,
        );
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, 65 + stat.magic_power * 40 / 100)
    }
}

#[derive(Clone, Copy, Default)]
struct GusionS2RecallReady;

impl StableEffectType for GusionS2RecallReady {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, _input: InputTargetV1) {
        if has_buff(sim, caster_id, S2_RECAST) {
            let _ = sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Skill2, 9_999);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct GusionUltStart;

impl StableEffectType for GusionUltStart {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, _input: InputTargetV1) {
        add_rune(sim, caster_id);
        let _ = sim.entity_remove_buff(caster_id, ULT_RECAST);
        sim.add_buff(caster_id, &BuffV1::timed(ULT_RECAST, ULT_RECAST_DURATION));
        // Incandescence reopens S2's throw phase but preserves an active first
        // volley; the next S2 throw adds a second stored volley before recall.
        let keep_first_volley = has_buff(sim, caster_id, S2_RECAST);
        let _ = sim.entity_remove_buff(caster_id, S2_RECAST);
        let _ = sim.entity_remove_buff(caster_id, S2_RESET_KEEP);
        if keep_first_volley {
            sim.add_buff(
                caster_id,
                &BuffV1::timed(S2_RESET_KEEP, ULT_RECAST_DURATION),
            );
        }
        // A pre-Ultimate S1 mark/phase is invalidated; the AI must throw and land
        // a fresh S1 dagger before using its teleport phase again.
        let _ = sim.entity_remove_buff(caster_id, S1_RECAST);
        let marked_targets: Vec<usize> = mark_owners()
            .lock()
            .ok()
            .map(|owners| {
                owners
                    .iter()
                    .filter_map(|(target_id, owner)| (*owner == caster_id).then_some(*target_id))
                    .collect()
            })
            .unwrap_or_default();
        for target_id in marked_targets {
            let _ = sim.entity_remove_buff(target_id, S1_MARK);
        }
        if let Ok(mut owners) = mark_owners().lock() {
            owners.retain(|_, owner| *owner != caster_id);
        }
        let _ = sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Skill, 9_999);
        let _ = sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Skill2, 9_999);
        let _ = sim.entity_reduce_cooldown(caster_id, SkillSlotV1::Ult, 9_999);
    }
}

#[derive(Clone, Copy, Default)]
struct GusionUltSecond;

impl StableEffectType for GusionUltSecond {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, _input: InputTargetV1) {
        add_rune(sim, caster_id);
    }
}

fn gusion_ai_target_valid(
    sim: &StableSim<'_>,
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    unit: &crate::UnitSnap,
    range: f64,
    champions_only: bool,
) -> bool {
    unit.id != caster.id
        && unit.team != caster.team
        && unit.alive
        && unit.targetable
        && !unit.tower
        && !matches!(
            sim.get_entity(unit.id).and_then(|entity| entity.kind()),
            Some(EntityKindV1::Nexus)
        )
        && !crate::is_unsafe_under_enemy_tower_with_margin(all, caster, unit, TOWER_DASH_BUFFER)
        && (!champions_only || unit.champion)
        && crate::distance(caster.pos, unit.pos) <= range
        && crate::is_unit_visible(sim, caster.team, unit)
}

fn gusion_ai_target(
    sim: &StableSim<'_>,
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    range: f64,
    champions_only: bool,
    preferred: Option<usize>,
) -> Option<usize> {
    let valid = |unit: &&crate::UnitSnap| {
        gusion_ai_target_valid(sim, all, caster, unit, range, champions_only)
    };
    if let Some(id) = preferred {
        if all
            .iter()
            .find(|unit| unit.id == id)
            .is_some_and(|unit| valid(&unit))
        {
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

fn gusion_marked_target(
    sim: &StableSim<'_>,
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    preferred: Option<usize>,
) -> Option<usize> {
    let mut targets: Vec<usize> = mark_owners()
        .lock()
        .ok()?
        .iter()
        .filter_map(|(target, owner)| (*owner == caster.id).then_some(*target))
        .collect();
    targets.sort_unstable();
    if let Some(preferred) = preferred {
        targets.sort_by_key(|target| usize::from(*target != preferred));
    }
    targets.into_iter().find(|target_id| {
        if !has_buff(sim, *target_id, S1_MARK) {
            clear_s1_preflight_rejection(sim, caster.id, *target_id);
            return false;
        }
        let Some(target) = all.iter().find(|unit| unit.id == *target_id) else {
            clear_s1_preflight_rejection(sim, caster.id, *target_id);
            return false;
        };
        let (target_unsafe, route_unsafe, destination) = gusion_s1_dash_safety(all, caster, target);
        log_s1_preflight_rejection(
            sim,
            caster,
            target,
            target_unsafe,
            route_unsafe,
            destination,
        );
        if target_unsafe || route_unsafe {
            return false;
        }
        gusion_ai_target_valid(sim, all, caster, target, 95_000.0, false)
    })
}

fn gusion_base_input_unsafe_reason(
    all: &[crate::UnitSnap],
    caster: &crate::UnitSnap,
    input: InputV1,
    s1_recast: bool,
) -> Option<&'static str> {
    if input.kind == InputKindV1::Move.code() {
        return crate::is_unsafe_tower_route(
            all,
            caster,
            (input.x as f64, input.y as f64),
            TOWER_DASH_BUFFER,
        )
        .then_some("movement route enters unsupported enemy-tower danger");
    }

    if input.target.kind != InputTargetKindV1::Target.code() {
        return None;
    }

    let target = all.iter().find(|unit| unit.id == input.target.target_id)?;
    // The custom preflight also rejects unsafe S1 dash routes, but its None
    // result alone does not suppress a native/base AI input. Recheck targeted
    // S1 recasts here so a safe target with an unsafe dash path cannot still
    // start the non-cancelable recast animation through the base-input path.
    if s1_recast && input.kind == InputKindV1::Skill.code() {
        let (target_unsafe, route_unsafe, _) = gusion_s1_dash_safety(all, caster, target);
        if target_unsafe || route_unsafe {
            return Some("S1 recast target or dash route enters unsupported enemy-tower danger");
        }
    }
    // The champion-dive rule applies to enemy champions only. Enemy towers are
    // valid push targets and must not be mistaken for champions standing under
    // their own tower. Route and active-escape checks remain separate.
    (target.team != caster.team
        && target.champion
        && !target.tower
        && crate::is_unsafe_under_enemy_tower_with_margin(all, caster, target, TOWER_DASH_BUFFER))
    .then_some("enemy champion target is under unsupported tower danger")
}

/// A state-aware combo driver: build a dagger volley, use Ultimate as the
/// reset when available, then mark/dash and recall every outstanding volley.
#[derive(Clone, Copy, Default)]
struct GusionComboAi {
    tower_escape: Option<GusionTowerEscape>,
    base_input_blocked: bool,
}

impl StablePlayerAi for GusionComboAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(*self)
    }

    fn id(&self) -> String {
        "mlbb_gusion_combo_ai".to_string()
    }

    fn priority(&self) -> i32 {
        150
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name.to_ascii_lowercase().contains("gusion")
    }

    fn think(
        &mut self,
        ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        let player_id = ctx.player_id();
        let athlete_id = ctx.athlete_id();
        let preferred = base_input
            .filter(|input| input.target.kind == InputTargetKindV1::Target.code())
            .map(|input| input.target.target_id);
        let (
            caster_id,
            skill_cd,
            skill2_cd,
            ult_cd,
            s1_recast,
            s2_recast,
            ult_recast,
            s2_unlocked,
            ult_unlocked,
            volley_count,
            marked_target,
            s1_target,
            s2_target,
            ult_target,
            unsafe_base_input_reason,
            tower_escape,
            escape_transition,
            hold_position,
        ) = {
            let sim = ctx.sim()?;
            let player = sim.get_player(player_id)?;
            let caster_id = player
                .champion()
                .map(|champion| champion.id())
                .unwrap_or(athlete_id);
            let cooldowns = player.cooldowns()?;
            let caster = sim.get_entity(caster_id)?;
            let all = crate::units(&sim);
            let caster_snap = all.iter().find(|unit| unit.id == caster_id)?;
            let s1_recast = has_buff(&sim, caster_id, S1_RECAST);
            let unsafe_base_input_reason = base_input.and_then(|input| {
                gusion_base_input_unsafe_reason(&all, caster_snap, input, s1_recast)
            });
            let origin = sim.sim_origin().unwrap_or_default();
            let sim_key = (
                origin.kind,
                origin.match_id,
                origin.replay_id,
                origin.set_index,
            );
            let unsafe_movement_route = unsafe_base_input_reason
                == Some("movement route enters unsupported enemy-tower danger");
            let (tower_escape, escape_transition) = update_gusion_tower_escape(
                &all,
                caster_snap,
                self.tower_escape,
                sim_key,
                unsafe_movement_route,
            );
            let hold_position = InputV1::move_to(
                caster_snap.pos.0.max(0.0) as u64,
                caster_snap.pos.1.max(0.0) as u64,
            );
            let volley_count = dagger_volleys()
                .lock()
                .ok()
                .and_then(|volleys| volleys.get(&caster_id).map(Vec::len))
                .unwrap_or(0);
            (
                caster_id,
                cooldowns.1,
                cooldowns.2,
                cooldowns.3,
                s1_recast,
                has_buff(&sim, caster_id, S2_RECAST),
                has_buff(&sim, caster_id, ULT_RECAST),
                caster
                    .cooldown(SkillSlotV1::Skill2)
                    .is_some_and(|state| state.unlocked),
                caster
                    .cooldown(SkillSlotV1::Ult)
                    .is_some_and(|state| state.unlocked),
                volley_count,
                gusion_marked_target(&sim, &all, caster_snap, preferred),
                gusion_ai_target(&sim, &all, caster_snap, 95_000.0, false, preferred),
                gusion_ai_target(&sim, &all, caster_snap, 78_000.0, false, preferred),
                gusion_ai_target(&sim, &all, caster_snap, 90_000.0, true, preferred),
                unsafe_base_input_reason,
                tower_escape,
                escape_transition,
                hold_position,
            )
        };

        // Escape locally from unsupported tower danger; do not issue Return while
        // the tower can still attack. Keep one waypoint until the release radius
        // is cleared so the decision cannot oscillate with ordinary AI orders.
        self.tower_escape = tower_escape;
        match (escape_transition, tower_escape) {
            (GusionTowerEscapeTransition::Entered, Some(state)) => log_gusion_ai(
                ctx,
                state.caster_id,
                format!(
                    "tower escape entered tower#{} waypoint=({}, {}) fallback={}",
                    state.tower_id, state.waypoint.0, state.waypoint.1, state.used_fallback
                ),
            ),
            (GusionTowerEscapeTransition::Retargeted, Some(state)) => log_gusion_ai(
                ctx,
                state.caster_id,
                format!(
                    "tower escape retargeted tower#{} waypoint=({}, {}) release_radius={} fallback={}",
                    state.tower_id,
                    state.waypoint.0,
                    state.waypoint.1,
                    state.release_radius,
                    state.used_fallback
                ),
            ),
            (GusionTowerEscapeTransition::Extended, Some(state)) => log_gusion_ai(
                ctx,
                state.caster_id,
                format!(
                    "tower escape extended because native route remains unsafe tower#{} waypoint=({}, {}) release_radius={} fallback={}",
                    state.tower_id,
                    state.waypoint.0,
                    state.waypoint.1,
                    state.release_radius,
                    state.used_fallback
                ),
            ),
            (GusionTowerEscapeTransition::ReleasePending, Some(state)) => log_gusion_ai(
                ctx,
                state.caster_id,
                format!(
                    "tower escape awaiting {TOWER_ESCAPE_RELEASE_CONFIRM_TICKS}-tick safe-route confirmation tower#{}",
                    state.tower_id
                ),
            ),
            (GusionTowerEscapeTransition::Released, None) => {
                log_gusion_ai(
                    ctx,
                    caster_id,
                    "tower escape released; danger radius cleared",
                );
            }
            _ => {}
        }
        if let Some(state) = tower_escape {
            self.base_input_blocked = false;
            return Some(InputV1::move_to(state.waypoint.0, state.waypoint.1));
        }

        // With the first dagger fan still out, reset and reopen S2's throw
        // phase. A second fan is then stored alongside the first for one recall.
        if s2_recast && volley_count == 1 && ult_unlocked && ult_cd == 0 && !ult_recast {
            if let Some(target_id) = ult_target {
                self.base_input_blocked = false;
                return Some(InputV1::action(
                    InputKindV1::Ult,
                    InputTargetV1::target(target_id),
                ));
            }
        }

        // Finish the mark phase before recalling the daggers, so the dash puts
        // Gusion across the target's return path.
        if s1_recast {
            if let Some(target_id) = marked_target {
                log_gusion_ai(
                    ctx,
                    caster_id,
                    format!(
                        "AI issued S1 recast for preflight-safe marked target#{target_id}; effect-time safety recheck remains active"
                    ),
                );
                self.base_input_blocked = false;
                return Some(InputV1::action(
                    InputKindV1::Skill,
                    InputTargetV1::target(target_id),
                ));
            }
        }

        // Start with S2 when possible, then use S1 to mark and dash.
        if s2_unlocked && !s2_recast && skill2_cd == 0 {
            if let Some(target_id) = s2_target {
                self.base_input_blocked = false;
                return Some(InputV1::action(
                    InputKindV1::Skill2,
                    InputTargetV1::target(target_id),
                ));
            }
        }
        if !s1_recast && skill_cd == 0 {
            if let Some(target_id) = s1_target {
                self.base_input_blocked = false;
                return Some(InputV1::action(
                    InputKindV1::Skill,
                    InputTargetV1::target(target_id),
                ));
            }
        }
        if s2_unlocked && s2_recast && skill2_cd == 0 {
            if let Some(target_id) = s2_target.or(s1_target) {
                self.base_input_blocked = false;
                return Some(InputV1::action(
                    InputKindV1::Skill2,
                    InputTargetV1::target(target_id),
                ));
            }
        }
        if let Some(reason) = unsafe_base_input_reason {
            if !self.base_input_blocked {
                log_gusion_ai(
                    ctx,
                    caster_id,
                    format!("base AI input held by tower safety: {reason}"),
                );
            }
            self.base_input_blocked = true;
            return Some(hold_position);
        }
        self.base_input_blocked = false;
        None
    }
}

#[derive(Clone, Copy, Default)]
struct GusionVisualNoop;

impl StableEffectType for GusionVisualNoop {
    fn apply(
        &self,
        _sim: &mut StableSim<'_>,
        _seed: u64,
        _caster_id: usize,
        _input: InputTargetV1,
    ) {
    }
}

#[derive(Clone, Default)]
struct GusionPassive;

impl StablePassive for GusionPassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        let _ = sim.entity_remove_buff(entity_id, RUNE);
        let _ = sim.entity_remove_buff(entity_id, S1_RECAST);
        let _ = sim.entity_remove_buff(entity_id, S2_RECAST);
        let _ = sim.entity_remove_buff(entity_id, S2_RESET_KEEP);
        let _ = sim.entity_remove_buff(entity_id, ULT_RECAST);
        set_rune_counter(sim, entity_id, 0);
        if let Ok(mut volleys) = dagger_volleys().lock() {
            volleys.remove(&entity_id);
        }
        if let Ok(mut owners) = mark_owners().lock() {
            owners.retain(|_, owner| *owner != entity_id);
        }
        clear_s1_preflight_rejections(sim, entity_id);
    }
}

#[cfg(test)]
mod tower_safety_tests {
    use super::TOWER_DASH_BUFFER;
    use crate::{is_unsafe_tower_route, is_unsafe_under_enemy_tower_with_margin, UnitSnap};
    use mod_api_stable::InputV1;

    fn unit(
        id: usize,
        pos: (f64, f64),
        team: usize,
        tower: bool,
        hp: usize,
        max_hp: usize,
    ) -> UnitSnap {
        UnitSnap {
            id,
            pos,
            team,
            alive: true,
            targetable: true,
            champion: !tower,
            tower,
            monster: false,
            level: 1,
            radius: 10_000.0,
            hp,
            max_hp,
            stack: 0,
        }
    }

    #[test]
    fn normal_attack_targeting_enemy_tower_is_not_mistaken_for_a_champion_dive() {
        use super::gusion_base_input_unsafe_reason;
        use mod_api_stable::{InputKindV1, InputTargetV1};

        let caster = unit(1, (350_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1_000, 1_000);
        let all = vec![caster.clone(), tower];
        let input = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));

        assert_eq!(
            gusion_base_input_unsafe_reason(&all, &caster, input, false),
            None
        );
    }

    #[test]
    fn native_s1_recast_with_safe_target_but_unsafe_dash_route_is_blocked() {
        use super::{gusion_base_input_unsafe_reason, gusion_s1_dash_safety};
        use mod_api_stable::{InputKindV1, InputTargetV1};

        let caster = unit(1, (200_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let target = unit(3, (400_000.0, 300_000.0), 1, false, 1_000, 1_000);
        // Target clearance is just outside the existing 52k + 18k margin,
        // while the projected endpoint and dash route cross the tower buffer.
        let tower = unit(2, (471_000.0, 300_000.0), 1, true, 1_000, 1_000);
        let all = vec![caster.clone(), tower, target.clone()];
        let (target_unsafe, route_unsafe, _) = gusion_s1_dash_safety(&all, &caster, &target);
        assert!(!target_unsafe, "the test target itself must remain safe");
        assert!(route_unsafe, "the projected S1 dash route must be unsafe");
        let input = InputV1::action(InputKindV1::Skill, InputTargetV1::target(target.id));
        assert_eq!(
            gusion_base_input_unsafe_reason(&all, &caster, input, true),
            Some("S1 recast target or dash route enters unsupported enemy-tower danger")
        );
        assert_eq!(
            gusion_base_input_unsafe_reason(&all, &caster, input, false),
            None,
            "the initial S1 mark cast is not a dash and remains distinct"
        );
    }

    #[test]
    fn unsafe_route_returning_during_release_confirmation_keeps_escape_latched() {
        use super::{
            update_gusion_tower_escape, GusionTowerEscapeTransition, TOWER_ESCAPE_RELEASE_RADIUS,
        };

        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let caster_inside = unit(1, (310_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let all_inside = vec![caster_inside.clone(), tower.clone()];
        let sim_key = (1, 1, 1, 1);
        let (state, transition) =
            update_gusion_tower_escape(&all_inside, &caster_inside, None, sim_key, false);
        assert_eq!(transition, GusionTowerEscapeTransition::Entered);
        let state = state.expect("danger should latch the escape");

        let caster_at_exit = unit(1, (371_000.0, 300_000.0), 0, false, 1_000, 1_000);
        assert!(
            crate::distance(caster_at_exit.pos, tower.pos) >= TOWER_ESCAPE_RELEASE_RADIUS as f64
        );
        let all_at_exit = vec![caster_at_exit.clone(), tower.clone()];
        let (pending, transition) =
            update_gusion_tower_escape(&all_at_exit, &caster_at_exit, Some(state), sim_key, false);
        assert_eq!(transition, GusionTowerEscapeTransition::ReleasePending);
        let pending = pending.expect("one safe route sample must not release the escape");

        // The failure in the supplied trace: the native route turns unsafe a
        // few ticks after the apparent release. It must extend, not clear.
        let (extended, transition) =
            update_gusion_tower_escape(&all_at_exit, &caster_at_exit, Some(pending), sim_key, true);
        assert_eq!(transition, GusionTowerEscapeTransition::Extended);
        let extended = extended.expect("unsafe route returned during confirmation");
        assert!(extended.release_radius > TOWER_ESCAPE_RELEASE_RADIUS as u64);
    }

    #[test]
    fn unsupported_movement_route_stays_guarded_and_two_ally_support_is_retained() {
        use super::gusion_base_input_unsafe_reason;

        let caster = unit(1, (450_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1_000, 1_000);
        let route = InputV1::move_to(300_000, 300_000);
        let unsupported = vec![caster.clone(), tower.clone()];
        assert_eq!(
            gusion_base_input_unsafe_reason(&unsupported, &caster, route, false),
            Some("movement route enters unsupported enemy-tower danger")
        );

        let ally_a = unit(3, (310_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let ally_b = unit(4, (320_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let supported = vec![caster.clone(), tower, ally_a, ally_b];
        assert_eq!(
            gusion_base_input_unsafe_reason(&supported, &caster, route, false),
            None
        );
    }

    #[test]
    fn unsupported_enemy_champion_dive_remains_blocked() {
        use super::gusion_base_input_unsafe_reason;
        use mod_api_stable::{InputKindV1, InputTargetV1};

        let caster = unit(1, (350_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1_000, 1_000);
        let target = unit(3, (310_000.0, 300_000.0), 1, false, 1_000, 1_000);
        let all = vec![caster.clone(), tower, target];
        let input = InputV1::action(InputKindV1::Attack, InputTargetV1::target(3));

        assert_eq!(
            gusion_base_input_unsafe_reason(&all, &caster, input, false),
            Some("enemy champion target is under unsupported tower danger")
        );
    }

    #[test]
    fn ordinary_non_champion_target_in_tower_zone_is_not_subject_to_champion_dive_rule() {
        use super::gusion_base_input_unsafe_reason;
        use mod_api_stable::{InputKindV1, InputTargetV1};

        let caster = unit(1, (350_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1_000, 1_000);
        let mut minion = unit(3, (310_000.0, 300_000.0), 1, false, 1_000, 1_000);
        minion.champion = false;
        let all = vec![caster.clone(), tower, minion];
        let input = InputV1::action(InputKindV1::Attack, InputTargetV1::target(3));

        assert_eq!(
            gusion_base_input_unsafe_reason(&all, &caster, input, false),
            None
        );
    }

    #[test]
    fn s1_recast_preflight_rejects_a_route_through_tower_danger_before_cast() {
        use super::gusion_s1_dash_safety;

        let caster = unit(1, (-100_000.0, 0.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (0.0, 0.0), 1, true, 1, 1);
        let target = unit(3, (100_000.0, 0.0), 1, false, 1_000, 1_000);
        let all = vec![caster.clone(), tower, target.clone()];
        let (target_unsafe, route_unsafe, destination) =
            gusion_s1_dash_safety(&all, &caster, &target);

        assert!(!target_unsafe, "target itself is outside tower danger");
        assert!(route_unsafe, "full dash crosses the tower's danger zone");
        assert_eq!(destination, (118_000.0, 0.0));
    }

    #[test]
    fn s1_recast_preflight_allows_a_clear_route_and_projects_behind_target() {
        use super::gusion_s1_dash_safety;

        let caster = unit(1, (150_000.0, 100_000.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (0.0, 0.0), 1, true, 1, 1);
        let target = unit(3, (150_000.0, 200_000.0), 1, false, 1_000, 1_000);
        let all = vec![caster.clone(), tower, target.clone()];
        let (target_unsafe, route_unsafe, destination) =
            gusion_s1_dash_safety(&all, &caster, &target);

        assert!(!target_unsafe);
        assert!(!route_unsafe);
        assert_eq!(destination, (150_000.0, 218_000.0));
    }

    #[test]
    fn gusion_rejects_unsupported_targets_in_tower_range_and_dash_buffer() {
        let caster = unit(1, (150_000.0, 0.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (0.0, 0.0), 1, true, 1, 1);
        let target = unit(3, (60_000.0, 0.0), 1, false, 1_000, 1_000);
        let all = vec![caster.clone(), tower, target.clone()];

        assert!(!is_unsafe_under_enemy_tower_with_margin(
            &all, &caster, &target, 0.0
        ));
        assert!(is_unsafe_under_enemy_tower_with_margin(
            &all,
            &caster,
            &target,
            TOWER_DASH_BUFFER
        ));
    }

    #[test]
    fn allied_minions_do_not_count_as_tower_tankers() {
        let caster = unit(1, (150_000.0, 0.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (0.0, 0.0), 1, true, 1, 1);
        let target = unit(3, (30_000.0, 0.0), 1, false, 1_000, 1_000);
        let mut minion_a = unit(4, (10_000.0, 0.0), 0, false, 1_000, 1_000);
        let mut minion_b = unit(5, (20_000.0, 0.0), 0, false, 1_000, 1_000);
        minion_a.champion = false;
        minion_b.champion = false;
        let all = vec![caster.clone(), tower, target.clone(), minion_a, minion_b];
        assert!(is_unsafe_under_enemy_tower_with_margin(
            &all,
            &caster,
            &target,
            TOWER_DASH_BUFFER
        ));
    }

    #[test]
    fn tower_dive_is_allowed_with_allied_tankers_or_a_safe_execute() {
        let caster = unit(1, (150_000.0, 0.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (0.0, 0.0), 1, true, 1, 1);
        let target = unit(3, (30_000.0, 0.0), 1, false, 1_000, 1_000);
        let ally_a = unit(4, (10_000.0, 0.0), 0, false, 1_000, 1_000);
        let ally_b = unit(5, (20_000.0, 0.0), 0, false, 1_000, 1_000);
        let supported = vec![
            caster.clone(),
            tower.clone(),
            target.clone(),
            ally_a,
            ally_b,
        ];

        assert!(!is_unsafe_under_enemy_tower_with_margin(
            &supported,
            &caster,
            &target,
            TOWER_DASH_BUFFER
        ));
        assert!(!is_unsafe_tower_route(
            &supported,
            &caster,
            target.pos,
            TOWER_DASH_BUFFER
        ));

        let execute = unit(6, target.pos, 1, false, 250, 1_000);
        let unsupported = vec![caster.clone(), tower, execute.clone()];
        assert!(!is_unsafe_under_enemy_tower_with_margin(
            &unsupported,
            &caster,
            &execute,
            TOWER_DASH_BUFFER
        ));
    }
    #[test]
    fn tower_escape_waypoint_is_outward_and_past_release_radius() {
        use super::{gusion_escape_waypoint, TOWER_ESCAPE_RELEASE_RADIUS};
        let caster = unit(1, (310_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let all = vec![caster.clone(), tower.clone()];
        let (waypoint, fallback) = gusion_escape_waypoint(&all, &caster, &tower);
        let end = (waypoint.0 as f64, waypoint.1 as f64);
        assert!(
            !fallback,
            "single-tower exit should use the validated route"
        );
        assert!(crate::distance(end, tower.pos) >= TOWER_ESCAPE_RELEASE_RADIUS - 2.0);
        assert!(super::escape_direction_is_outward(
            caster.pos, end, tower.pos
        ));
    }

    #[test]
    fn tower_escape_handles_overlapping_tower_zones_without_recall() {
        use super::{gusion_escape_waypoint, TOWER_ESCAPE_RELEASE_RADIUS};
        let caster = unit(1, (340_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let tower_a = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let tower_b = unit(3, (370_000.0, 300_000.0), 1, true, 1, 1);
        let all = vec![caster.clone(), tower_a.clone(), tower_b.clone()];
        let (waypoint, fallback) = gusion_escape_waypoint(&all, &caster, &tower_a);
        let end = (waypoint.0 as f64, waypoint.1 as f64);
        assert!(
            !fallback,
            "a perpendicular outward route should clear both zones"
        );
        for tower in [&tower_a, &tower_b] {
            assert!(crate::distance(end, tower.pos) >= TOWER_ESCAPE_RELEASE_RADIUS - 2.0);
            assert!(super::escape_direction_is_outward(
                caster.pos, end, tower.pos
            ));
        }
    }

    #[test]
    fn tower_escape_replans_after_a_displacement_makes_the_latched_route_unsafe() {
        use super::{
            update_gusion_tower_escape, GusionTowerEscapeTransition, TOWER_ESCAPE_RELEASE_RADIUS,
        };
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let caster = unit(1, (310_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let initial_all = vec![caster.clone(), tower.clone()];
        let sim_key = (1, 1, 1, 1);
        let (state, transition) =
            update_gusion_tower_escape(&initial_all, &caster, None, sim_key, false);
        assert_eq!(transition, GusionTowerEscapeTransition::Entered);
        let state = state.expect("unsupported tower danger should latch an escape");

        // A knockback to the opposite side of the same tower makes the saved
        // waypoint cross the danger zone; the AI must compute a new outward route.
        let displaced = unit(1, (295_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let displaced_all = vec![displaced.clone(), tower.clone()];
        let (replanned, transition) =
            update_gusion_tower_escape(&displaced_all, &displaced, Some(state), sim_key, false);
        assert_eq!(transition, GusionTowerEscapeTransition::Retargeted);
        let replanned = replanned.expect("unsafe saved route should be replaced");
        assert!(
            crate::distance(
                (replanned.waypoint.0 as f64, replanned.waypoint.1 as f64),
                tower.pos
            ) >= TOWER_ESCAPE_RELEASE_RADIUS - 2.0
        );
        assert!(super::escape_direction_is_outward(
            displaced.pos,
            (replanned.waypoint.0 as f64, replanned.waypoint.1 as f64),
            tower.pos
        ));
    }

    #[test]
    fn tower_escape_latches_until_release_radius_then_clears() {
        use super::{
            update_gusion_tower_escape, GusionTowerEscapeTransition, TOWER_ESCAPE_RELEASE_RADIUS,
        };
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let caster_inside = unit(1, (310_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let all_inside = vec![caster_inside.clone(), tower.clone()];
        let sim_key = (1, 1, 1, 1);
        let (state, transition) =
            update_gusion_tower_escape(&all_inside, &caster_inside, None, sim_key, false);
        assert_eq!(transition, GusionTowerEscapeTransition::Entered);
        let state = state.expect("danger should start an escape");

        let caster_between_radii = unit(1, (360_000.0, 300_000.0), 0, false, 1_000, 1_000);
        assert!(crate::distance(caster_between_radii.pos, tower.pos) < TOWER_ESCAPE_RELEASE_RADIUS);
        let all_between = vec![caster_between_radii.clone(), tower.clone()];
        let (still_escaping, transition) = update_gusion_tower_escape(
            &all_between,
            &caster_between_radii,
            Some(state),
            sim_key,
            false,
        );
        assert_eq!(transition, GusionTowerEscapeTransition::None);
        assert_eq!(still_escaping, Some(state));

        let (new_context_state, new_context_transition) = update_gusion_tower_escape(
            &all_between,
            &caster_between_radii,
            Some(state),
            (1, 2, 2, 1),
            false,
        );
        assert_eq!(new_context_transition, GusionTowerEscapeTransition::None);
        assert_eq!(new_context_state, None);

        let caster_clear = unit(1, (371_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let all_clear = vec![caster_clear.clone(), tower];
        let mut pending = Some(state);
        for tick in 0..super::TOWER_ESCAPE_RELEASE_CONFIRM_TICKS {
            let (next, transition) =
                update_gusion_tower_escape(&all_clear, &caster_clear, pending, sim_key, false);
            pending = next;
            if tick + 1 < super::TOWER_ESCAPE_RELEASE_CONFIRM_TICKS {
                assert!(
                    pending.is_some(),
                    "escape should remain latched through safe confirmation"
                );
                assert_ne!(transition, GusionTowerEscapeTransition::Released);
            } else {
                assert_eq!(pending, None);
                assert_eq!(transition, GusionTowerEscapeTransition::Released);
            }
        }
    }

    #[test]
    fn tower_escape_extends_and_stays_latched_if_native_route_is_still_unsafe_at_exit() {
        use super::{
            update_gusion_tower_escape, GusionTowerEscapeTransition, TOWER_ESCAPE_RELEASE_RADIUS,
            TOWER_ESCAPE_ROUTE_EXTENSION,
        };
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let caster_inside = unit(1, (310_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let all_inside = vec![caster_inside.clone(), tower.clone()];
        let sim_key = (1, 1, 1, 1);
        let (state, transition) =
            update_gusion_tower_escape(&all_inside, &caster_inside, None, sim_key, false);
        assert_eq!(transition, GusionTowerEscapeTransition::Entered);
        let state = state.expect("unsupported tower danger should start an escape");
        assert_eq!(state.release_radius, TOWER_ESCAPE_RELEASE_RADIUS as u64);

        // At the normal exit point, a native route still points back through
        // unsupported tower danger. Extend by the existing dash-safety buffer.
        let caster_at_exit = unit(1, (371_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let all_at_exit = vec![caster_at_exit.clone(), tower.clone()];
        let (extended, transition) =
            update_gusion_tower_escape(&all_at_exit, &caster_at_exit, Some(state), sim_key, true);
        assert_eq!(transition, GusionTowerEscapeTransition::Extended);
        let extended = extended.expect("unsafe native route must not release the escape");
        assert!(
            extended.release_radius as f64
                >= TOWER_ESCAPE_RELEASE_RADIUS + TOWER_ESCAPE_ROUTE_EXTENSION
        );
        let endpoint = (extended.waypoint.0 as f64, extended.waypoint.1 as f64);
        assert!(crate::distance(endpoint, tower.pos) >= extended.release_radius as f64 - 2.0);
        assert!(super::escape_direction_is_outward(
            caster_at_exit.pos,
            endpoint,
            tower.pos
        ));

        // The saved outward waypoint is stable while the same dangerous native
        // route persists; it must neither snap back to home nor oscillate.
        let (still_escaping, transition) = update_gusion_tower_escape(
            &all_at_exit,
            &caster_at_exit,
            Some(extended),
            sim_key,
            true,
        );
        assert_eq!(transition, GusionTowerEscapeTransition::None);
        assert_eq!(still_escaping, Some(extended));

        // Even beyond the extended distance, don't re-enter the bad base route.
        let caster_beyond = unit(1, (390_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let all_beyond = vec![caster_beyond.clone(), tower.clone()];
        let (latched, transition) =
            update_gusion_tower_escape(&all_beyond, &caster_beyond, Some(extended), sim_key, true);
        assert_eq!(transition, GusionTowerEscapeTransition::None);
        assert_eq!(latched, Some(extended));

        let mut pending = Some(extended);
        for tick in 0..super::TOWER_ESCAPE_RELEASE_CONFIRM_TICKS {
            let (next, transition) =
                update_gusion_tower_escape(&all_beyond, &caster_beyond, pending, sim_key, false);
            pending = next;
            if tick + 1 < super::TOWER_ESCAPE_RELEASE_CONFIRM_TICKS {
                assert!(pending.is_some());
                assert_ne!(transition, GusionTowerEscapeTransition::Released);
            } else {
                assert_eq!(pending, None);
                assert_eq!(transition, GusionTowerEscapeTransition::Released);
            }
        }
    }

    #[test]
    fn route_danger_without_an_active_escape_does_not_change_normal_ai_state() {
        use super::{update_gusion_tower_escape, GusionTowerEscapeTransition};
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let caster = unit(1, (450_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let all = vec![caster.clone(), tower];
        let (state, transition) =
            update_gusion_tower_escape(&all, &caster, None, (1, 1, 1, 1), true);
        assert_eq!(state, None);
        assert_eq!(transition, GusionTowerEscapeTransition::None);
    }

    #[test]
    fn tower_escape_respects_existing_two_champion_tank_support() {
        use super::{update_gusion_tower_escape, GusionTowerEscapeTransition};
        let caster = unit(1, (310_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let tower = unit(2, (300_000.0, 300_000.0), 1, true, 1, 1);
        let ally_a = unit(3, (305_000.0, 300_000.0), 0, false, 1_000, 1_000);
        let ally_b = unit(4, (300_000.0, 305_000.0), 0, false, 1_000, 1_000);
        let all = vec![caster.clone(), tower, ally_a, ally_b];
        let (state, transition) =
            update_gusion_tower_escape(&all, &caster, None, (1, 1, 1, 1), false);
        assert_eq!(state, None);
        assert_eq!(transition, GusionTowerEscapeTransition::None);
    }
}
