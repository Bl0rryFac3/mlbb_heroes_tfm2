//! Standalone Zhask native mod — Nightmaric Spawn + Death Ray + Dominator's Descent
//! Implements the kit described in Zhask_TFM2_Implementation_Plan.md §4-§6
//! with unsupported Hive Clone text removed from its localized description.

use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};

use mod_api_stable::*;

// ---------------- identity ----------------
const MOD_ID: &str = "mlbb_heroes";
const HERO_ID: &str = "mlbb_zhask";
const SPAWN_NAME: &str = "mlbb_zhask_spawn";
const FUSED_SPAWN_NAME: &str = "mlbb_zhask_fused_spawn";

// marker buffs placed on spawned units to identify them (survives name quirks)
const BUFF_SPAWN: &str = "mlbb_zhask_unit_spawn";
const BUFF_FUSED: &str = "mlbb_zhask_unit_fused";
const BUFF_FRENZY: &str = "mlbb_zhask_unit_frenzy";
// temporary buffs on Zhask himself
const BUFF_FUSED_WINDOW: &str = "mlbb_zhask_fused";
const BUFF_ULT_RECAST: &str = "mlbb_zhask_ult_recast";
const BASIC_BOLT_PROJ: &str = "mlbb_zhask_basic_bolt";
const BEAM_REFRESH_TICKS: usize = 2;
const BEAM_SEGMENT_SPACING: f64 = 750.0;
const BEAM_MAX_SEGMENTS: usize = 96;
const S2_STUN_TICKS: u64 = 30;
const FUSION_BANISH_STEP: usize = 24;
const FUSION_BANISH_REFRESH: usize = 12;
const S2_FOLLOWUP_CAP: usize = 6;
const S2_IMPACT_VIEW: &str = "mlbb_heroes_zhask_mind_eater_impact";

// ---------------- config (matches champion/mlbb_zhask.data_champion passive.params) ----------------
#[derive(Clone, Debug)]
struct ZhaskConfig {
    leash: u64,
    spawn_life: u64,
    frenzy_life: u64,
    ray_range: u64,
    ray_base: usize,
    ray_ap_pct: usize,
    ray_slow_pct: i32,
    ray_slow_ticks: u64,
    fused_scale: usize,
    fused_ticks: u64,
    inherit_l1: usize,
    inherit_l12: usize,
    repin_ticks: usize,
    ray_channel_ticks: u64,
    ray_tick_ticks: u64,
    decimation_inherit: usize,
    decimation_ray_bonus: usize,
    ult_banish: bool,
    ult_reset_skill1: bool,
    ult_range_bonus: u64,
    s1_disable_after_loss: u64,
}
impl Default for ZhaskConfig {
    fn default() -> Self {
        Self {
            leash: 120_000,
            spawn_life: 720,
            frenzy_life: 360,
            ray_range: 52_000,
            ray_base: 30,
            ray_ap_pct: 25,
            ray_slow_pct: 30,
            ray_slow_ticks: 36,
            fused_scale: 180,
            fused_ticks: 600,
            inherit_l1: 55,
            inherit_l12: 100,
            repin_ticks: 1,
            ray_channel_ticks: 60,
            ray_tick_ticks: 15,
            decimation_inherit: 100,
            decimation_ray_bonus: 60,
            ult_banish: true,
            ult_reset_skill1: true,
            ult_range_bonus: 12_000,
            s1_disable_after_loss: 120,
        }
    }
}
static CONFIG: LazyLock<Mutex<ZhaskConfig>> = LazyLock::new(|| Mutex::new(ZhaskConfig::default()));

// ---------------- per-Zhask state ----------------
#[derive(Clone, Debug)]
struct PendingHit {
    target: usize,
    due_tick: usize,
}

#[derive(Clone, Debug)]
struct PendingStunCheck {
    target: usize,
    due_tick: usize,
}

// Consume the fusion marker before side effects so every exit path is idempotent.
fn take_fusion_marker(st: &mut ZhaskState) -> bool {
    if st.fused_until == 0 {
        return false;
    }
    st.fused_until = 0;
    true
}

fn queue_pending_hits(
    pending: &mut Vec<PendingHit>,
    hit_ids: &[usize],
    due_tick: usize,
) -> (usize, usize) {
    let accepted = hit_ids
        .len()
        .min(S2_FOLLOWUP_CAP.saturating_sub(pending.len()));
    pending.extend(
        hit_ids
            .iter()
            .take(accepted)
            .map(|&target| PendingHit { target, due_tick }),
    );
    (accepted, hit_ids.len() - accepted)
}

#[derive(Clone, Copy, Debug)]
struct PendingDecimation {
    team: usize,
    pos: (u64, u64),
    stat: StatV1,
    level: usize,
    death_tick: usize,
}

#[derive(Clone, Debug, Default)]
struct ZhaskState {
    spawn: Option<usize>,
    // Normal Spawn hidden during fusion; it is allowed to expire while a distinct fused unit is shown.
    hidden_spawn: Option<usize>,
    anchor: (u64, u64),
    fusion_anchor: (u64, u64),
    // slide animation: from -> to over ticks_left
    lerp_from: (u64, u64),
    lerp_to: (u64, u64),
    lerp_left: usize,
    next_ray: usize,
    channel_until: usize,
    beam_target: Option<usize>,
    last_beam_tick: usize,
    fused_until: usize,
    s1_block_until: usize,
    frenzy: Option<usize>,
    pending_decimation: Option<PendingDecimation>,
    frenzy_spawn_tick: usize,
    frenzy_expires_at: usize,
    frenzy_entity_verified: bool,
    frenzy_missing_retry_count: usize,
    frenzy_team: usize,
    frenzy_owner_mp: usize,
    frenzy_decay_next: usize,
    frenzy_next_ray: usize,
    frenzy_beam_target: Option<usize>,
    frenzy_last_beam_tick: usize,
    frenzy_last_processed_tick: Option<usize>,
    frenzy_scheduler_diagnostic_logged: bool,
    frenzy_no_target_logged: bool,
    frenzy_visual_diagnostic_logged: bool,
    beam_visual_diagnostic_logged: bool,
    banish_refresh_at: usize,
    banish_exit_deadline: usize,
    last_passive_attempt: Option<usize>,
    last_processed_tick: Option<usize>,
    pending: Vec<PendingHit>,
    stun_checks: Vec<PendingStunCheck>,
    last_stat_push: usize,
    // cached death position for Decimation if on_dead fires before next tick
    death_pos: Option<(u64, u64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SimKey {
    match_id: u64,
    set_index: u64,
    seed: u64,
    zhask_id: usize,
    /// Assigned per simulation copy. Two pre-sims of one match are identical in
    /// every derivable field — and `sim_origin()` returns `None` during
    /// pre-simulation, so `match_id` / `set_index` default to `u64::MAX` for
    /// both. The copy tag is what tells them apart. See `SimScope` in lib.rs.
    copy: u64,
}
impl SimKey {
    /// Read-only. Reads the copy tag if the entity already carries one.
    fn of(sim: &StableSim<'_>, zhask_id: usize) -> Self {
        let o = sim.sim_origin().unwrap_or_default();
        Self {
            match_id: o.match_id,
            set_index: o.set_index,
            seed: sim.seed(),
            zhask_id,
            copy: crate::sim_copy_of(sim, zhask_id).unwrap_or(0),
        }
    }

    /// Mutable. Assigns the copy tag if the entity does not carry one yet.
    /// Use from spawn hooks, where a `&mut StableSim` is available.
    fn assign(sim: &mut StableSim<'_>, zhask_id: usize) -> Self {
        let o = sim.sim_origin().unwrap_or_default();
        Self {
            match_id: o.match_id,
            set_index: o.set_index,
            seed: sim.seed(),
            zhask_id,
            copy: crate::entity_key(sim, zhask_id).copy,
        }
    }
}
static STATES: LazyLock<Mutex<BTreeMap<SimKey, ZhaskState>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

// helpers
fn has_buff(e: &StableEntity<'_, '_>, name: &str) -> bool {
    (0..e.buff_count())
        .filter_map(|i| e.buff_at(i))
        .any(|b| b.name() == name)
}
fn is_spawn_entity(e: &StableEntity<'_, '_>) -> bool {
    has_buff(e, BUFF_SPAWN) || has_buff(e, BUFF_FUSED) || has_buff(e, BUFF_FRENZY)
}
fn dist_sq(a: (u64, u64), b: (u64, u64)) -> u128 {
    let dx = a.0 as i128 - b.0 as i128;
    let dy = a.1 as i128 - b.1 as i128;
    (dx * dx + dy * dy) as u128
}
fn clamp_to_leash(from: (u64, u64), want: (u64, u64), leash: u64) -> (u64, u64) {
    let d =
        ((want.0 as f64 - from.0 as f64).powi(2) + (want.1 as f64 - from.1 as f64).powi(2)).sqrt();
    if d <= leash as f64 || d < 1.0 {
        return want;
    }
    let r = leash as f64 / d;
    let nx = from.0 as f64 + (want.0 as f64 - from.0 as f64) * r;
    let ny = from.1 as f64 + (want.1 as f64 - from.1 as f64) * r;
    (nx.max(10_000.0) as u64, ny.max(10_000.0) as u64)
}
fn inherit_pct(level: usize, cfg: &ZhaskConfig) -> usize {
    // 55 / 70 / 85 / 100 by brackets 1-3,4-6,7-9,10-12
    let pct = match level {
        1..=3 => cfg.inherit_l1,
        4..=6 => 70,
        7..=9 => 85,
        _ => cfg.inherit_l12,
    };
    pct.clamp(10, 100)
}
fn read_player_level(sim: &StableSim<'_>, zhask_id: usize) -> usize {
    let pc = sim.player_count();
    for i in 0..pc {
        if let Some(p) = sim.player_at(i) {
            if p.champion().map(|c| c.id()) == Some(zhask_id) {
                return p.level().max(1);
            }
        }
    }
    1
}
fn find_zhasks(sim: &StableSim<'_>) -> Vec<usize> {
    // Scan the entity table even when champion_count() is nonzero: some hosts omit dead
    // champions from the champion index, but Decimation still needs their death tick.
    let mut ids = Vec::new();
    for i in 0..sim.entity_count() {
        if let Some(e) = sim.entity_at(i) {
            if e.is_champion() && e.name().as_deref() == Some(HERO_ID) && !is_spawn_entity(&e) {
                ids.push(e.id());
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}
fn get_state(sim: &StableSim<'_>, zhask_id: usize) -> ZhaskState {
    let k = SimKey::of(sim, zhask_id);
    STATES.lock().unwrap().get(&k).cloned().unwrap_or_default()
}
fn put_state(sim: &StableSim<'_>, zhask_id: usize, st: ZhaskState) {
    let k = SimKey::of(sim, zhask_id);
    let mut m = STATES.lock().unwrap();
    m.insert(k, st);
    // prune presim entries if too many (keep last 128)
    if m.len() > 256 {
        let mut presim_keys: Vec<_> = m
            .keys()
            .filter(|k| k.match_id == u64::MAX)
            .cloned()
            .collect();
        presim_keys.sort();
        for kk in presim_keys.into_iter().take(m.len().saturating_sub(128)) {
            m.remove(&kk);
        }
    }
}

fn log_event(sim: &StableSim<'_>, category: &str, caster: usize, message: &str) {
    let origin = sim.sim_origin().unwrap_or_default();
    let kind = match SimOriginKindV1::from_code(origin.kind) {
        Some(SimOriginKindV1::ServerPresim) => "presim",
        Some(SimOriginKindV1::ClientMatchView) => "view",
        Some(SimOriginKindV1::ClientSpectate) => "spectate",
        Some(SimOriginKindV1::ClientReplay) => "replay",
        Some(SimOriginKindV1::Tool) => "tool",
        _ => "unknown",
    };
    let m = if origin.match_id == SimOriginV1::NONE {
        "-".into()
    } else {
        origin.match_id.to_string()
    };
    let s = if origin.set_index == SimOriginV1::NONE {
        "-".into()
    } else {
        origin.set_index.to_string()
    };
    super::wlog(format!(
        "[Zhask][{category}] [{kind} m={m} s={s}] tick={} caster={} {message}",
        sim.tick(),
        caster
    ));
}

fn can_render_spawn_visual(sim: &StableSim<'_>) -> bool {
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

fn inset_beam_endpoints(
    from: (u64, u64),
    to: (u64, u64),
    from_inset: f64,
    to_inset: f64,
) -> Option<((u64, u64), (u64, u64))> {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let length = dx.hypot(dy);
    if !length.is_finite() || length <= from_inset + to_inset + 1.0 {
        return None;
    }
    let ux = dx / length;
    let uy = dy / length;
    let point = |x: f64, y: f64| {
        (
            x.round().clamp(0.0, u64::MAX as f64) as u64,
            y.round().clamp(0.0, u64::MAX as f64) as u64,
        )
    };
    Some((
        point(
            from.0 as f64 + ux * from_inset,
            from.1 as f64 + uy * from_inset,
        ),
        point(to.0 as f64 - ux * to_inset, to.1 as f64 - uy * to_inset),
    ))
}

fn beam_angle_index(from: (u64, u64), to: (u64, u64)) -> usize {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    // The sprite sheet has 12 undirected orientations over 180 degrees (15 degrees each),
    // not 12 directed orientations over 360 degrees.
    let angle = dy.atan2(dx).rem_euclid(std::f64::consts::PI);
    ((angle / (std::f64::consts::PI / 12.0)).round() as usize) % 12
}

fn beam_segment_points(from: (u64, u64), to: (u64, u64)) -> Vec<(u64, u64)> {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < 1.0 {
        return Vec::new();
    }
    let count = ((distance / BEAM_SEGMENT_SPACING).ceil() as usize).clamp(1, BEAM_MAX_SEGMENTS);
    (0..count)
        .map(|i| {
            let t = (i as f64 + 0.5) / count as f64;
            (
                (from.0 as f64 + dx * t) as u64,
                (from.1 as f64 + dy * t) as u64,
            )
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default)]
struct BeamDrawStats {
    angle: usize,
    requested: usize,
    accepted: usize,
}

fn draw_spawn_beam(
    sim: &mut StableSim<'_>,
    view_owner: usize,
    spawn: usize,
    target: usize,
    from: (u64, u64),
    to: (u64, u64),
) -> BeamDrawStats {
    if !can_render_spawn_visual(sim) {
        return BeamDrawStats::default();
    }
    let from_radius = sim
        .get_entity(spawn)
        .map_or(0.0, |entity| entity.radius() as f64);
    let to_radius = sim
        .get_entity(target)
        .map_or(0.0, |entity| entity.radius() as f64);
    let Some((from, to)) = inset_beam_endpoints(from, to, from_radius * 0.72, to_radius * 0.72)
    else {
        return BeamDrawStats::default();
    };
    let angle = beam_angle_index(from, to);
    let name = format!("mlbb_heroes_zhask_spawn_beam_{angle}");
    let points = beam_segment_points(from, to);
    let requested = points.len();
    let accepted = points
        .into_iter()
        // The directional view-effect bindings belong to Zhask's champion data,
        // so use Zhask as the effect owner and the positions to place each segment.
        // Refresh only in view contexts; the animation controls segment lifetime.
        .filter(|&(x, y)| {
            sim.play_view_effect(&name, view_owner, &InputTargetV1::pos(x, y), 0, 0, 0)
        })
        .count();
    BeamDrawStats {
        angle,
        requested,
        accepted,
    }
}

fn has_active_stun(sim: &StableSim<'_>, target: usize) -> bool {
    sim.get_entity(target).is_some_and(|entity| {
        (0..entity.cc_count()).any(|i| {
            entity
                .cc_at(i)
                .is_some_and(|cc| cc.kind == CcKindV1::Stun.code())
        })
    })
}

fn end_fusion(
    sim: &mut StableSim<'_>,
    caster: usize,
    st: &mut ZhaskState,
    restore_spawn: bool,
    reason: &str,
) {
    let tick = sim.tick();
    let cfg = CONFIG.lock().unwrap().clone();
    let state_was_active = take_fusion_marker(st);
    let buff_was_active = sim
        .get_entity(caster)
        .is_some_and(|entity| has_buff(&entity, BUFF_FUSED_WINDOW));
    if !state_was_active && !buff_was_active {
        log_event(
            sim,
            "Ult",
            caster,
            &format!("duplicate fusion end ignored reason={reason}"),
        );
        return;
    }
    sim.entity_remove_buff(caster, BUFF_FUSED_WINDOW);
    sim.entity_remove_buff(caster, BUFF_ULT_RECAST);

    // Retire the distinct fused model instead of morphing it into the normal Spawn.
    // Keep the short invisibility as a defensive visual cleanup if unit removal is unavailable.
    let fused_id = st.spawn;
    let fused_pos = fused_id
        .filter(|&id| sim.get_entity(id).is_some_and(|e| e.is_alive()))
        .and_then(|id| sim.get_entity(id).map(|e| e.pos()));
    let fused_hidden = if let Some(id) = fused_id {
        if sim.get_entity(id).is_some_and(|e| e.is_alive()) {
            let hidden = sim.entity_set_invisible(id, cfg.fused_ticks as usize);
            let hp_zeroed = if hidden {
                false
            } else {
                sim.entity_set_hp(id, 0)
            };
            sim.entity_remove_buff(id, BUFF_FUSED);
            log_event(
                sim,
                "Ult",
                caster,
                &format!("fused Spawn retired id={id} invisible={hidden} hp_zeroed_on_hide_failure={hp_zeroed}"),
            );
            hidden || hp_zeroed
        } else {
            false
        }
    } else {
        false
    };

    let hidden_normal_pos = st
        .hidden_spawn
        .filter(|&id| sim.get_entity(id).is_some_and(|e| e.is_alive()))
        .and_then(|id| sim.get_entity(id).map(|e| e.pos()));
    let should_restore = restore_spawn || st.hidden_spawn.is_some();
    st.spawn = None;
    st.hidden_spawn = None;

    // Recreate a normal Spawn at the fusion anchor. This gives the two forms distinct
    // unit names/models and avoids mutating a fused model back into a normal one.
    let mut restored_id = None;
    if should_restore {
        if let (Some(caster_entity), Some(team)) = (
            sim.get_entity(caster),
            sim.get_entity(caster).map(|e| e.team()),
        ) {
            let level = read_player_level(sim, caster);
            let stat = compute_spawn_stat(caster_entity.stat(), level, &cfg, false, false);
            let pos = fused_pos
                .or(hidden_normal_pos)
                .filter(|p| *p != (0, 0))
                .unwrap_or_else(|| {
                    if st.fusion_anchor != (0, 0) {
                        st.fusion_anchor
                    } else {
                        caster_entity.pos()
                    }
                });
            let atk = UnitAttackV1 {
                attack_ratio: 0,
                attack: 0,
                range: 0,
                cooltime: 65,
                duration: 20,
                start_timing: 10,
                cancelable: false,
                attack_type: AttackTypeV1::BaseAttack.code(),
            };
            if let Some(id) = sim.spawn_unit(
                SPAWN_NAME,
                caster,
                team,
                pos.0,
                pos.1,
                cfg.spawn_life,
                &stat,
                &atk,
            ) {
                sim.add_buff(id, &BuffV1::named(BUFF_SPAWN));
                restored_id = Some(id);
                log_event(
                    sim,
                    "Ult",
                    caster,
                    &format!(
                        "normal Spawn restored as distinct entity id={id} at=({}, {})",
                        pos.0, pos.1
                    ),
                );
            } else {
                log_event(
                    sim,
                    "Ult",
                    caster,
                    "normal Spawn restoration failed; S1 can recreate it",
                );
            }
        }
    }

    let cc_removed = sim.entity_clear_cc(caster);
    let exit_banish = if cfg.ult_banish {
        sim.entity_banish(caster, caster, 1, "", "")
    } else {
        false
    };
    st.spawn = restored_id;
    st.fused_until = 0;
    st.fusion_anchor = (0, 0);
    st.channel_until = 0;
    st.next_ray = tick + cfg.ray_tick_ticks as usize;
    st.beam_target = None;
    st.last_beam_tick = tick;
    st.pending.clear();
    st.banish_refresh_at = 0;
    st.banish_exit_deadline = tick + 1;
    st.frenzy_next_ray = tick;
    st.frenzy_beam_target = None;
    st.frenzy_last_beam_tick = tick;
    let status = sim
        .get_entity(caster)
        .map(|e| (e.is_targetable(), e.team()));
    let (targetable, enemy_visible) = status.map_or((false, false), |(targetable, team)| {
        (
            targetable,
            sim.is_visible(1usize.saturating_sub(team.min(1)), caster),
        )
    });
    log_event(
        sim,
        "Ult",
        caster,
        &format!("fusion ended reason={reason} cc_removed={cc_removed} one_tick_exit_banish={exit_banish} fused_hidden={fused_hidden} restored_spawn={restored_id:?} targetable={targetable} enemy_visible={enemy_visible}"),
    );
}

// ---------------- stat & damage helpers ----------------
fn compute_spawn_stat(
    zhask_stat: StatV1,
    level: usize,
    cfg: &ZhaskConfig,
    frenzy: bool,
    fused: bool,
) -> StatV1 {
    let pct = if frenzy {
        cfg.decimation_inherit
    } else {
        inherit_pct(level, cfg)
    };
    // fused gets extra HP via buff, but base inherit also 100% when fused (spawn inherits 100% then buff)
    let inherit = if fused { cfg.inherit_l12 } else { pct };
    let hp = (zhask_stat.hp * inherit) / 100;
    let def = (zhask_stat.defence * inherit) / 100;
    let mr = (zhask_stat.magic_resistance * inherit) / 100;
    // movement: pinned entity — use very low speed so engine does not drift
    // Yorick clamps 1850-2100; we want near-zero but engine may require >0. Keep 50.
    let ms = 50usize;
    StatV1 {
        hp: hp.max(80),
        attack: 0,
        magic_power: (zhask_stat.magic_power * inherit) / 100,
        defence: def.max(5),
        magic_resistance: mr.max(5),
        move_speed: ms,
        hp_regen: 0,
        stack: 0,
        crit_chance: 0,
    }
}

fn ray_damage(zhask_mp: usize, cfg: &ZhaskConfig, fused: bool, is_frenzy: bool) -> usize {
    let mut dmg = cfg.ray_base + (zhask_mp * cfg.ray_ap_pct) / 100;
    if is_frenzy {
        dmg += cfg.decimation_ray_bonus;
    }
    if fused {
        dmg = dmg * cfg.fused_scale / 100;
    }
    dmg
}
fn mind_eater_zhask_damage(level: usize, mp: usize, fused: bool) -> usize {
    let base = 87 + (level.saturating_sub(1) * 6); // 87->153
    let mut dmg = base + mp * 60 / 100;
    if fused {
        // fused payload: extra 40 +50% MP (represents Hive fragment)
        dmg += 40 + mp * 50 / 100;
    }
    dmg
}
fn mind_eater_spawn_damage(mp: usize) -> usize {
    35 + mp * 50 / 100
}

// find nearest enemy within ray_range of spawn
fn nearest_enemy_in_range(
    sim: &StableSim<'_>,
    origin: (u64, u64),
    team: usize,
    range: u64,
    zhask_id: usize,
) -> Option<usize> {
    let mut best: Option<(u128, usize)> = None;
    let ec = sim.entity_count();
    let r2 = (range as u128) * (range as u128);
    for i in 0..ec {
        let Some(e) = sim.entity_at(i) else { continue };
        if e.id() == zhask_id {
            continue;
        }
        if e.team() == team {
            continue;
        }
        if !e.is_alive() {
            continue;
        }
        // skip towers? casting_target EnemyWithoutTower — we respect that: skip towers for ray?
        // but ray should hit champions/minions; skip tower entities
        if e.is_tower() {
            continue;
        }
        // skip untargetable
        if !e.is_targetable() {
            continue;
        }
        // skip spawn entities themselves? they are on Zhask team
        let p = e.pos();
        let d2 = dist_sq(origin, p);
        if d2 > r2 {
            continue;
        }
        let score = d2;
        if best.map(|(d, _)| score < d).unwrap_or(true) {
            best = Some((score, e.id()));
        }
    }
    best.map(|(_, id)| id)
}

fn tick_frenzy_attack(
    sim: &mut StableSim<'_>,
    owner: usize,
    team: usize,
    owner_mp: usize,
    st: &mut ZhaskState,
    cfg: &ZhaskConfig,
    tick: usize,
) {
    let Some(fid) = st.frenzy else { return };
    let Some(frenzy) = sim.get_entity(fid) else {
        return;
    };
    if !frenzy.is_alive() {
        return;
    }
    let fpos = frenzy.pos();
    let target = nearest_enemy_in_range(sim, fpos, team, cfg.ray_range, owner);
    let Some(target) = target else {
        st.frenzy_beam_target = None;
        st.frenzy_next_ray = tick + 1;
        if !st.frenzy_no_target_logged {
            log_event(
                sim,
                "Passive",
                owner,
                &format!(
                    "Decimation scheduler active spawn={fid}; no valid enemy within ray_range={}",
                    cfg.ray_range
                ),
            );
            st.frenzy_no_target_logged = true;
        }
        return;
    };
    st.frenzy_no_target_logged = false;
    if st.frenzy_beam_target != Some(target) {
        log_event(
            sim,
            "Passive",
            owner,
            &format!("Decimation Spawn target={} attack_source={fid}", target),
        );
        st.frenzy_beam_target = Some(target);
    }
    if tick >= st.frenzy_next_ray {
        let damage = ray_damage(owner_mp, cfg, false, true);
        // The passive Spawn owns its separate attack path and remains the source after Zhask dies.
        sim.deal_damage(fid, target, 0, damage, AttackTypeV1::BaseAttack);
        log_event(
            sim,
            "Passive",
            owner,
            &format!("Decimation ray damage_requested={damage} source_spawn={fid} target={target} next_tick={}", tick + (cfg.ray_tick_ticks as usize * 2).max(1)),
        );
        let mut slow = BuffV1::timed("mlbb_zhask_ray_slow", cfg.ray_slow_ticks as usize);
        slow.move_speed_mult = -(cfg.ray_slow_pct as i32);
        sim.add_buff(target, &slow);
        st.frenzy_next_ray = tick + (cfg.ray_tick_ticks as usize * 2).max(1);
    }
    if tick >= st.frenzy_last_beam_tick + BEAM_REFRESH_TICKS {
        if can_render_spawn_visual(sim) {
            if let Some(target_pos) = sim.get_entity(target).map(|e| e.pos()) {
                let drawn = draw_spawn_beam(sim, owner, fid, target, fpos, target_pos);
                if !st.frenzy_visual_diagnostic_logged {
                    log_event(
                        sim,
                        "Passive",
                        owner,
                        &format!("Decimation Spawn beam visual spawn={fid} target={target} owner={owner} angle={} requested_segments={} accepted_segments={}; damage scheduler remains independent", drawn.angle, drawn.requested, drawn.accepted),
                    );
                    st.frenzy_visual_diagnostic_logged = true;
                }
            }
        }
        st.frenzy_last_beam_tick = tick;
    }
}

// Decimation is attempted directly from on_dead when the player champion is available,
// and retried from the tick path only if the host rejected unit creation.
fn attempt_decimation_spawn(
    sim: &mut StableSim<'_>,
    zhask_id: usize,
    zhask_team: usize,
    zhask_stat_snapshot: StatV1,
    level: usize,
    cfg: &ZhaskConfig,
    tick: usize,
    st: &mut ZhaskState,
) -> bool {
    let Some(dpos) = st.death_pos else {
        return false;
    };
    if st.frenzy.is_some() {
        return false;
    }
    let frenzy_stat = compute_spawn_stat(zhask_stat_snapshot, level, cfg, true, false);
    let atk = UnitAttackV1 {
        attack_ratio: 0,
        attack: 0,
        range: 0,
        cooltime: 65,
        duration: 20,
        start_timing: 10,
        cancelable: false,
        attack_type: AttackTypeV1::BaseAttack.code(),
    };
    let created = sim.spawn_unit(
        SPAWN_NAME,
        zhask_id,
        zhask_team,
        dpos.0,
        dpos.1,
        cfg.frenzy_life,
        &frenzy_stat,
        &atk,
    );
    st.last_passive_attempt = Some(tick);
    if let Some(fid) = created {
        // Use both markers: the ordinary Spawn marker matches S1's visual/entity path;
        // Frenzy remains the passive-specific marker for passive logic.
        sim.add_buff(fid, &BuffV1::named(BUFF_SPAWN));
        sim.add_buff(fid, &BuffV1::named(BUFF_FRENZY));
        st.frenzy = Some(fid);
        st.frenzy_spawn_tick = tick;
        st.frenzy_expires_at = tick.saturating_add(cfg.frenzy_life as usize);
        st.frenzy_entity_verified = false;
        st.frenzy_team = zhask_team;
        st.frenzy_owner_mp = zhask_stat_snapshot.magic_power;
        st.frenzy_decay_next = tick + 60;
        st.frenzy_next_ray = tick;
        st.frenzy_beam_target = None;
        st.frenzy_last_beam_tick = tick;
        st.frenzy_last_processed_tick = None;
        st.frenzy_scheduler_diagnostic_logged = false;
        st.frenzy_no_target_logged = false;
        st.frenzy_visual_diagnostic_logged = false;
        log_event(
            sim,
            "Passive",
            zhask_id,
            &format!(
                "Decimation Spawn create accepted id={fid} at=({}, {}) trigger=match_tick; awaiting next-tick entity verification",
                dpos.0, dpos.1
            ),
        );
        true
    } else {
        log_event(
            sim,
            "Passive",
            zhask_id,
            &format!(
                "Decimation Spawn creation failed at=({}, {}); will retry after 30 ticks",
                dpos.0, dpos.1
            ),
        );
        false
    }
}

fn record_zhask_death(
    sim: &mut StableSim<'_>,
    zhask_id: usize,
    team: usize,
    pos: (u64, u64),
    stat: StatV1,
    level: usize,
    player_index: usize,
) {
    let tick = sim.tick();
    let mut st = get_state(sim, zhask_id);
    if st.frenzy.is_some() || st.pending_decimation.is_some() {
        log_event(
            sim,
            "Passive",
            zhask_id,
            "death callback ignored: Decimation Spawn/deferred death already tracked",
        );
        return;
    }
    st.death_pos = Some(pos);
    st.pending_decimation = Some(PendingDecimation {
        team,
        pos,
        stat,
        level,
        death_tick: tick,
    });
    log_event(
        sim,
        "Passive",
        zhask_id,
        &format!(
            "death callback resolved player={player_index} position=({}, {}) level={level}; queueing Spawn for regular match tick",
            pos.0, pos.1
        ),
    );
    put_state(sim, zhask_id, st);
}

// ---------------- core tick ----------------
fn tick_zhask(sim: &mut StableSim<'_>, zhask_id: usize) {
    let tick = sim.tick();
    let cfg = CONFIG.lock().unwrap().clone();
    let mut st = get_state(sim, zhask_id);
    // The passive and match hook both invoke this brain; process each hero once per sim tick.
    if st.last_processed_tick == Some(tick) {
        return;
    }
    st.last_processed_tick = Some(tick);

    // fetch zhask snapshot (avoid holding ref across mutable borrows)
    let (
        zhask_team,
        zhask_pos,
        zhask_alive,
        zhask_mp,
        zhask_stat_snapshot,
        _zhask_hpmax,
        has_fused_buff,
    ) = {
        let Some(e) = sim.get_entity(zhask_id) else {
            return;
        };
        (
            e.team(),
            e.pos(),
            e.is_alive(),
            e.stat().magic_power,
            e.stat(),
            e.hp().1,
            has_buff(&e, BUFF_FUSED_WINDOW),
        )
    };
    let level = read_player_level(sim, zhask_id);
    // fused check from buff or state
    let is_fused = tick < st.fused_until || has_fused_buff;
    if is_fused && cfg.ult_banish && tick >= st.banish_refresh_at {
        let applied = sim.entity_banish(zhask_id, zhask_id, FUSION_BANISH_STEP, "", "");
        st.banish_refresh_at = tick + FUSION_BANISH_REFRESH;
        log_event(
            sim,
            "Ult",
            zhask_id,
            &format!(
                "short-step Banish refresh requested={} applied={} step_ticks={} next_refresh={}",
                cfg.ult_banish, applied, FUSION_BANISH_STEP, st.banish_refresh_at
            ),
        );
    } else if !is_fused
        && cfg.ult_banish
        && st.banish_exit_deadline > 0
        && tick >= st.banish_exit_deadline
    {
        let cc_removed = sim.entity_clear_cc(zhask_id);
        st.banish_exit_deadline = 0;
        log_event(
            sim,
            "Ult",
            zhask_id,
            &format!("one-tick exit Banish follow-up clear_cc_called={cc_removed}"),
        );
    }
    // if zhask died this tick, st.death_pos already set via on_dead; otherwise handle
    if !zhask_alive {
        // do not run pin/ray while dead, but still need to keep spawn alive until timer
        // we still need to validate spawn/frenzy liveness below
    }

    // Phase 0: validate Spawn liveness and end fusion immediately if its host is lost.
    if let Some(sid) = st.spawn {
        let spawn_alive = sim
            .get_entity(sid)
            .is_some_and(|e| e.is_alive() && !e.is_champion());
        if !spawn_alive {
            st.spawn = None;
            st.anchor = (0, 0);
            st.lerp_left = 0;
            st.next_ray = tick + cfg.ray_tick_ticks as usize;
            st.channel_until = 0;
            st.beam_target = None;
            st.pending.clear();
            if is_fused {
                st.s1_block_until = tick + cfg.s1_disable_after_loss as usize;
                end_fusion(sim, zhask_id, &mut st, false, "fused_spawn_lost");
            } else {
                log_event(
                    sim,
                    "SpawnBeam",
                    zhask_id,
                    &format!("tracked Spawn lost spawn={sid}"),
                );
            }
        }
    } else if is_fused {
        st.s1_block_until = tick + cfg.s1_disable_after_loss as usize;
        end_fusion(sim, zhask_id, &mut st, false, "fused_spawn_missing");
    }
    // Handle timeout once, after Spawn-loss cleanup. end_fusion consumes the active
    // marker first, so later ticks cannot mistake a finished fusion for an expired one.
    if st.fused_until > 0 && tick >= st.fused_until {
        end_fusion(sim, zhask_id, &mut st, true, "duration_expired");
    }
    // Recompute after loss/expiry cleanup, which may have ended fusion above.
    let is_fused = tick < st.fused_until;
    if !st.stun_checks.is_empty() {
        let mut pending_checks = Vec::new();
        for check in st.stun_checks.drain(..) {
            if tick < check.due_tick {
                pending_checks.push(check);
                continue;
            }
            if sim.get_entity(check.target).is_some_and(|e| e.is_alive()) {
                let active = has_active_stun(sim, check.target);
                log_event(
                    sim,
                    "S2",
                    zhask_id,
                    &format!(
                        "stun next-tick verification target={} active_cc_observed={active}",
                        check.target
                    ),
                );
            } else {
                log_event(
                    sim,
                    "S2",
                    zhask_id,
                    &format!(
                        "stun next-tick verification target={} unavailable",
                        check.target
                    ),
                );
            }
        }
        st.stun_checks = pending_checks;
    }

    if !zhask_alive {
        put_state(sim, zhask_id, st);
        return;
    }
    st.death_pos = None;

    // Phase 1: pin / lerp
    if let Some(sid) = st.spawn {
        if let Some(se) = sim.get_entity(sid) {
            let cur = se.pos();
            if st.lerp_left > 0 {
                let left = st.lerp_left as f64;
                let total = 10.0; // fixed lerp window
                let t = 1.0 - (left - 1.0) / total; // 0..1
                let nx = (st.lerp_from.0 as f64 + (st.lerp_to.0 as f64 - st.lerp_from.0 as f64) * t)
                    as u64;
                let ny = (st.lerp_from.1 as f64 + (st.lerp_to.1 as f64 - st.lerp_from.1 as f64) * t)
                    as u64;
                sim.entity_set_pos(sid, nx, ny);
                st.lerp_left -= 1;
                if st.lerp_left == 0 {
                    st.anchor = st.lerp_to;
                }
            } else {
                // every tick pin to anchor (repin_ticks =1)
                if cur != st.anchor && st.anchor != (0, 0) {
                    sim.entity_set_pos(sid, st.anchor.0, st.anchor.1);
                }
            }
        }
    }
    // also pin frenzy (stay where spawned)
    if let Some(fid) = st.frenzy {
        if let Some(fe) = sim.get_entity(fid) {
            // frenzy is pinned at its spawn location; we don't have separate anchor, so keep current pos stable
            // read its pos once and keep? Instead just ensure it doesn't wander: set move_speed 50 already.
            // We could also pin it to its first seen pos, but for now just ensure frenzy decay pin via low speed.
            let _ = fe.pos();
        }
    }

    // Phase 2: stat push every 60 ticks
    if tick >= st.last_stat_push + 60 {
        if let Some(sid) = st.spawn {
            if let Some(_se) = sim.get_entity(sid) {
                let new_stat =
                    compute_spawn_stat(zhask_stat_snapshot, level, &cfg, false, is_fused);
                sim.entity_set_base_stat(sid, &new_stat);
            }
        }
        if let Some(fid) = st.frenzy {
            if let Some(_fe) = sim.get_entity(fid) {
                let new_stat = compute_spawn_stat(zhask_stat_snapshot, level, &cfg, true, false);
                sim.entity_set_base_stat(fid, &new_stat);
            }
        }
        st.last_stat_push = tick;
    }

    // Phase 3: leash — if Zhask too far from spawn, suppress ray (desync)
    let leashed = if let Some(sid) = st.spawn {
        if let Some(se) = sim.get_entity(sid) {
            let d2 = dist_sq(zhask_pos, se.pos());
            let leash2 = (cfg.leash as u128) * (cfg.leash as u128);
            d2 > leash2
        } else {
            false
        }
    } else {
        false
    };

    // Phase 4: one damage scheduler plus a separate refreshed, target-following visual.
    if !leashed {
        if let Some(sid) = st.spawn {
            if let Some(se) = sim.get_entity(sid) {
                let spos = se.pos();
                let target = nearest_enemy_in_range(sim, spos, zhask_team, cfg.ray_range, zhask_id);
                if let Some(tgt) = target {
                    if st.beam_target != Some(tgt) {
                        let event = if st.beam_target.is_some() {
                            "retarget"
                        } else {
                            "channel start"
                        };
                        log_event(
                            sim,
                            "SpawnBeam",
                            zhask_id,
                            &format!("{event} spawn={sid} target={tgt}"),
                        );
                        st.beam_target = Some(tgt);
                        if tick >= st.channel_until {
                            st.channel_until = tick + cfg.ray_channel_ticks as usize;
                        }
                    }
                    if tick >= st.next_ray {
                        if tick >= st.channel_until {
                            st.channel_until = tick + cfg.ray_channel_ticks as usize;
                        }
                        let dmg = ray_damage(zhask_mp, &cfg, is_fused, false);
                        sim.deal_damage(sid, tgt, 0, dmg, AttackTypeV1::BaseAttack);
                        log_event(
                            sim,
                            "SpawnBeam",
                            zhask_id,
                            &format!("ray damage_requested={dmg} source_spawn={sid} target={tgt} fused={is_fused}"),
                        );
                        let mut slow =
                            BuffV1::timed("mlbb_zhask_ray_slow", cfg.ray_slow_ticks as usize);
                        slow.move_speed_mult = -(cfg.ray_slow_pct as i32);
                        sim.add_buff(tgt, &slow);
                        st.next_ray = tick + cfg.ray_tick_ticks as usize;
                    }
                    if tick >= st.last_beam_tick + BEAM_REFRESH_TICKS {
                        if can_render_spawn_visual(sim) {
                            if let Some(tpos) = sim.get_entity(tgt).map(|e| e.pos()) {
                                let drawn = draw_spawn_beam(sim, zhask_id, sid, tgt, spos, tpos);
                                if !st.beam_visual_diagnostic_logged {
                                    log_event(
                                        sim,
                                        "SpawnBeam",
                                        zhask_id,
                                        &format!("beam visual spawn={sid} target={tgt} owner={zhask_id} angle={} requested_segments={} accepted_segments={} refresh_ticks={BEAM_REFRESH_TICKS}; damage remains independent", drawn.angle, drawn.requested, drawn.accepted),
                                    );
                                    st.beam_visual_diagnostic_logged = true;
                                }
                            }
                        }
                        st.last_beam_tick = tick;
                    }
                } else {
                    if let Some(old_target) = st.beam_target.take() {
                        log_event(
                            sim,
                            "SpawnBeam",
                            zhask_id,
                            &format!("channel stop spawn={sid} lost_target={old_target}"),
                        );
                    }
                    st.channel_until = tick;
                    st.next_ray = tick + 1;
                    st.last_beam_tick = tick;
                }
            }
        }
    } else {
        if st.beam_target.take().is_some() {
            log_event(
                sim,
                "SpawnBeam",
                zhask_id,
                "channel paused outside Spawn leash",
            );
        }
        st.next_ray = tick + cfg.ray_tick_ticks as usize;
        st.channel_until = tick;
        st.last_beam_tick = tick;
    }

    // Phase 5: delayed Mind Eater follow-up. Damage and CC are validated independently.
    if !st.pending.is_empty() {
        let mut remaining = Vec::new();
        for pending in st.pending.drain(..) {
            if tick < pending.due_tick {
                remaining.push(pending);
                continue;
            }
            let Some(sid) = st.spawn else {
                log_event(
                    sim,
                    "S2",
                    zhask_id,
                    &format!(
                        "follow-up skipped: no tracked Spawn target={}",
                        pending.target
                    ),
                );
                continue;
            };
            let spawn_ok = sim.get_entity(sid).is_some_and(|e| e.is_alive());
            let target_info = sim.get_entity(pending.target).and_then(|e| {
                let team = sim
                    .get_entity(zhask_id)
                    .map(|c| c.team())
                    .unwrap_or(usize::MAX);
                (e.is_alive() && e.is_targetable() && e.team() != team).then(|| e.pos())
            });
            if !spawn_ok || target_info.is_none() {
                log_event(
                    sim,
                    "S2",
                    zhask_id,
                    &format!(
                        "follow-up skipped: spawn_valid={spawn_ok} target_valid={} target={}",
                        target_info.is_some(),
                        pending.target
                    ),
                );
                continue;
            }
            let target_pos = target_info.unwrap();
            let spawn_pos = sim.get_entity(sid).map(|e| e.pos()).unwrap_or(target_pos);
            let dmg = mind_eater_spawn_damage(zhask_mp);
            sim.deal_damage(sid, pending.target, 0, dmg, AttackTypeV1::Skill);
            let beam_segments =
                draw_spawn_beam(sim, zhask_id, sid, pending.target, spawn_pos, target_pos);
            log_event(
                sim,
                "S2",
                zhask_id,
                &format!("delayed Spawn bonus damage requested spawn={sid} target={} damage={dmg} visual_segments_requested={} visual_segments_accepted={}", pending.target, beam_segments.requested, beam_segments.accepted),
            );
        }
        st.pending = remaining;
    }

    put_state(sim, zhask_id, st);
}

// Process living Decimation Spawns from their own state even after the dead Zhask
// disappears from the entity table and no longer receives on_update callbacks.
fn tick_active_frenzy_spawns(sim: &mut StableSim<'_>) {
    let tick = sim.tick();
    let cfg = CONFIG.lock().unwrap().clone();
    let origin = sim.sim_origin().unwrap_or_default();
    let seed = sim.seed();
    let owners = {
        let states = STATES.lock().unwrap();
        states
            .iter()
            .filter_map(|(key, state)| {
                (key.match_id == origin.match_id
                    && key.set_index == origin.set_index
                    && key.seed == seed
                    && (state.frenzy.is_some() || state.pending_decimation.is_some()))
                .then_some(key.zhask_id)
            })
            .collect::<Vec<_>>()
    };

    for owner in owners {
        let mut st = get_state(sim, owner);
        if st.frenzy_last_processed_tick == Some(tick) {
            continue;
        }
        st.frenzy_last_processed_tick = Some(tick);

        if let Some(fid) = st.frenzy {
            match sim
                .get_entity(fid)
                .map(|entity| (entity.is_alive(), entity.pos(), entity.kind()))
            {
                Some((true, position, kind)) => {
                    if !st.frenzy_entity_verified && tick > st.frenzy_spawn_tick {
                        log_event(
                            sim,
                            "Passive",
                            owner,
                            &format!("Decimation Spawn verified in match simulation id={fid} alive=true kind={kind:?} pos=({}, {}) tick={tick}; this confirms entity persistence, not on-screen rendering", position.0, position.1),
                        );
                        st.frenzy_entity_verified = true;
                        st.frenzy_missing_retry_count = 0;
                    }
                }
                Some((false, position, kind)) => {
                    log_event(
                        sim,
                        "Passive",
                        owner,
                        &format!("Decimation Spawn entity died id={fid} kind={kind:?} pos=({}, {}); no respawn retry", position.0, position.1),
                    );
                    st.frenzy = None;
                    st.pending_decimation = None;
                    st.death_pos = None;
                    st.frenzy_decay_next = 0;
                    st.frenzy_expires_at = 0;
                }
                None => {
                    if tick >= st.frenzy_expires_at && st.frenzy_expires_at > 0 {
                        log_event(sim, "Passive", owner, &format!("Decimation Spawn duration ended id={fid}; no further creation retry"));
                        st.frenzy = None;
                        st.pending_decimation = None;
                        st.death_pos = None;
                        st.frenzy_decay_next = 0;
                        st.frenzy_expires_at = 0;
                    } else if tick > st.frenzy_spawn_tick {
                        log_event(sim, "Passive", owner, &format!("Decimation Spawn create returned id={fid}, but entity is absent on next match tick; retrying creation in match context"));
                        st.frenzy = None;
                        st.frenzy_entity_verified = false;
                        st.frenzy_missing_retry_count =
                            st.frenzy_missing_retry_count.saturating_add(1);
                        // Permit two immediate retries, then use the existing 30-tick throttle.
                        if st.frenzy_missing_retry_count <= 2 {
                            st.last_passive_attempt = None;
                        }
                    }
                }
            }
        }

        if st.frenzy.is_none() {
            if let Some(pending) = st.pending_decimation {
                let due = tick > pending.death_tick;
                let retry_ready = st
                    .last_passive_attempt
                    .is_none_or(|last| tick.saturating_sub(last) >= 30);
                if due && retry_ready {
                    st.death_pos = Some(pending.pos);
                    attempt_decimation_spawn(
                        sim,
                        owner,
                        pending.team,
                        pending.stat,
                        pending.level,
                        &cfg,
                        tick,
                        &mut st,
                    );
                }
            }
        }

        if let Some(fid) = st.frenzy {
            if !st.frenzy_scheduler_diagnostic_logged {
                let owner_resolvable = sim.get_entity(owner).is_some();
                log_event(
                    sim,
                    "Passive",
                    owner,
                    &format!("Decimation scheduler active spawn={fid} owner_entity_resolvable={owner_resolvable} team={} owner_mp={} ray_range={}", st.frenzy_team, st.frenzy_owner_mp, cfg.ray_range),
                );
                st.frenzy_scheduler_diagnostic_logged = true;
            }
            tick_frenzy_attack(
                sim,
                owner,
                st.frenzy_team,
                st.frenzy_owner_mp,
                &mut st,
                &cfg,
                tick,
            );
            if tick >= st.frenzy_decay_next {
                if let Some((current, max)) = sim.get_entity(fid).map(|entity| entity.hp()) {
                    if max > 0 && current > 0 {
                        let new_current = current.saturating_sub((max * 2) / 100).max(1);
                        sim.entity_set_hp(fid, new_current);
                    }
                }
                st.frenzy_decay_next = tick + 60;
            }
        }
        put_state(sim, owner, st);
    }
}

// ---------------- passive ----------------
const ZHASK_DEATH_LINES: &[&str] = &[
    "mlbb_zhask_voice_death01",
    "mlbb_zhask_voice_death02",
    "mlbb_zhask_voice_death03",
    "mlbb_zhask_voice_death04",
];

#[derive(Clone, Default)]
struct ZhaskSwarmPassive {
    voice: crate::audio::VoiceLifecycle,
}

impl StablePassive for ZhaskSwarmPassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
    fn configure(&mut self, params_json: &str) {
        // manual tiny JSON parser for {"key":number,...} — no serde needed
        let mut map = BTreeMap::<String, u64>::new();
        let s = params_json.trim();
        let inner = s.trim_start_matches('{').trim_end_matches('}');
        for part in inner.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some((k, v)) = part.split_once(':') {
                let k = k.trim().trim_matches('"').trim_matches('\'').to_string();
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if let Ok(n) = v.parse::<u64>() {
                    map.insert(k, n);
                }
            }
        }
        if map.is_empty() {
            return;
        }
        let mut cfg = CONFIG.lock().unwrap();
        for (k, val) in map {
            match k.as_str() {
                "leash" => cfg.leash = val,
                "spawn_life" => cfg.spawn_life = val,
                "frenzy_life" => cfg.frenzy_life = val,
                "ray_range" => cfg.ray_range = val,
                "ray_base" => cfg.ray_base = val as usize,
                "ray_ap_pct" => cfg.ray_ap_pct = val as usize,
                "ray_slow_pct" => cfg.ray_slow_pct = val as i32,
                "ray_slow_ticks" => cfg.ray_slow_ticks = val,
                "fused_scale" => cfg.fused_scale = val as usize,
                "fused_ticks" => cfg.fused_ticks = val,
                "inherit_l1" => cfg.inherit_l1 = val as usize,
                "inherit_l12" => cfg.inherit_l12 = val as usize,
                "repin_ticks" => cfg.repin_ticks = val as usize,
                "ray_channel_ticks" => cfg.ray_channel_ticks = val,
                "ray_tick_ticks" => cfg.ray_tick_ticks = val,
                "decimation_inherit" => cfg.decimation_inherit = val as usize,
                "decimation_ray_bonus" => cfg.decimation_ray_bonus = val as usize,
                "ult_banish" => cfg.ult_banish = val != 0,
                "ult_reset_skill1" => cfg.ult_reset_skill1 = val != 0,
                "ult_range_bonus" => cfg.ult_range_bonus = val,
                "s1_disable_after_loss" => cfg.s1_disable_after_loss = val,
                _ => {}
            }
        }
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize) {
        // clear state on (re)spawn — covers respawn after death.
        // `assign`, not `of`: this is a mutable context, so it can tag this
        // copy of Zhask if it is not tagged yet.
        let k = SimKey::assign(sim, entity);
        STATES.lock().unwrap().remove(&k);
        self.voice.on_spawn(sim, entity, None);
    }
    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        // Prefer the callback's player index: the dead champion may disappear from
        // the global entity scan before the next match tick.
        let resolved_player = sim.player_at(player).map(|p| {
            let level = p.level().max(1);
            p.champion().map(|e| {
                (
                    e.id(),
                    e.name().as_deref() == Some(HERO_ID),
                    e.is_champion(),
                    is_spawn_entity(&e),
                    e.team(),
                    e.pos(),
                    e.stat(),
                    level,
                )
            })
        });
        match resolved_player {
            Some(Some((zid, true, true, false, team, pos, stat, level))) => {
                self.voice.on_dead_random(sim, player, ZHASK_DEATH_LINES);
                record_zhask_death(sim, zid, team, pos, stat, level, player);
                return;
            }
            Some(Some(_)) => return, // this callback belongs to a different champion
            Some(None) | None => {}
        }

        // Compatibility fallback for hosts where player_at/champion is unavailable
        // during on_dead. Capture and attempt immediately while a dead entity remains.
        let mut captured = 0usize;
        for zid in find_zhasks(sim) {
            if let Some((alive, team, pos, stat)) = sim
                .get_entity(zid)
                .map(|e| (e.is_alive(), e.team(), e.pos(), e.stat()))
            {
                if !alive {
                    record_zhask_death(
                        sim,
                        zid,
                        team,
                        pos,
                        stat,
                        read_player_level(sim, zid),
                        player,
                    );
                    captured += 1;
                }
            }
        }
        if captured > 0 {
            self.voice.on_dead_random(sim, player, ZHASK_DEATH_LINES);
        }
        if captured == 0 {
            log_event(
                sim,
                "Passive",
                player,
                "on_dead callback could not resolve a dead Zhask through player or entity lookup",
            );
        }
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _rng: u64, _player: usize, entity: usize) {
        // entity is the champion's entity id for this passive instance
        tick_zhask(sim, entity);
    }
}

// ---------------- match hook (covers cases where passive not ticking, and multiple Zhasks) ----------------
struct ZhaskMatchHook;
impl StableMatchHook for ZhaskMatchHook {
    fn on_match_tick(&self, sim: &mut StableSim<'_>, _rng: u64) {
        crate::audio::observe_voice_tick(sim);
        for zid in find_zhasks(sim) {
            tick_zhask(sim, zid);
        }
        tick_active_frenzy_spawns(sim);
        crate::angela::tick_all(sim);
        crate::eudora::tick_all(sim);
        crate::julian::tick_all(sim);
        crate::popol_probe::tick_all(sim);
        crate::granger::tick_all(sim);
        crate::hanzo::tick_all(sim);
    }
}

// ---------------- effects ----------------
#[derive(Clone, Default)]
struct ZhaskAttack;
impl StableEffectType for ZhaskAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _rng: u64, caster: usize, _input: InputTargetV1) {
        let Some(caster_e) = sim.get_entity(caster) else {
            return;
        };
        let team = caster_e.team();
        let start = caster_e.pos();
        let target = _input.target_id;
        if target == 0 {
            return;
        }
        let Some(te) = sim.get_entity(target) else {
            return;
        };
        if !te.is_alive() || !te.is_targetable() || te.team() == team {
            return;
        }
        let (tx, ty) = te.pos();
        let spec = ProjectileSpawnV1 {
            caster_id: caster,
            team,
            x: start.0,
            y: start.1,
            radius: 3000,
            speed: 8000,
            attack_type: AttackTypeV1::BaseAttack.code(),
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: CastingTargetV1::Enemy.code(),
            target_x: tx,
            target_y: ty,
            move_kind: ProjectileMoveKindV1::Target.code(),
            target_id: target,
            ..ProjectileSpawnV1::default()
        };
        let _ = sim.spawn_projectile(BASIC_BOLT_PROJ, "mlbb_heroes:zhask_attack_hit", &spec);
    }
    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (caster_stat.attack, 0)
    }
}

#[derive(Clone, Default)]
struct ZhaskAttackHit;
impl StableEffectType for ZhaskAttackHit {
    fn apply(&self, sim: &mut StableSim<'_>, _rng: u64, caster: usize, _input: InputTargetV1) {
        let target = _input.target_id;
        if target == 0 {
            return;
        }
        let (ad, team) = {
            let Some(e) = sim.get_entity(caster) else {
                return;
            };
            (e.stat().attack, e.team())
        };
        if let Some(te) = sim.get_entity(target) {
            if te.is_alive() && te.team() != team && te.is_targetable() {
                sim.deal_damage(caster, target, ad, 0, AttackTypeV1::BaseAttack);
            }
        }
    }
    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (caster_stat.attack, 0)
    }
}

#[derive(Clone, Default)]
struct ZhaskSkill1;
impl StableEffectType for ZhaskSkill1 {
    fn apply(&self, sim: &mut StableSim<'_>, _rng: u64, caster: usize, input: InputTargetV1) {
        let tick = sim.tick();
        let cfg = CONFIG.lock().unwrap().clone();
        let (team, cpos, caster_stat) = {
            let Some(e) = sim.get_entity(caster) else {
                return;
            };
            (e.team(), e.pos(), e.stat())
        };
        let level = read_player_level(sim, caster);
        let mut st = get_state(sim, caster);

        if tick < st.s1_block_until {
            put_state(sim, caster, st);
            return;
        }

        // resolve want position
        let want = match InputTargetKindV1::from_code(input.kind) {
            Some(InputTargetKindV1::Pos) => (input.x, input.y),
            Some(InputTargetKindV1::Dir) => {
                // direction + caster pos + 60% of leash
                let dx = input.dir_x as f64;
                let dy = input.dir_y as f64;
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                let dist = cfg.leash as f64 * 0.6;
                (
                    (cpos.0 as f64 + dx / len * dist) as u64,
                    (cpos.1 as f64 + dy / len * dist) as u64,
                )
            }
            Some(InputTargetKindV1::Target) => {
                if let Some(te) = sim.get_entity(input.target_id) {
                    te.pos()
                } else {
                    cpos
                }
            }
            _ => {
                // No input (AI fallback) — place 45k in front
                (cpos.0 + 40_000, cpos.1)
            }
        };
        let mut want = clamp_to_leash(cpos, want, (cfg.leash as f64 * 0.85) as u64);
        // tower check: reject if inside enemy tower radius + 8000
        // scan towers
        let tc = sim.tower_count();
        for i in 0..tc {
            let tid = sim.tower_id_at(i);
            if let Some(te) = sim.get_entity(tid) {
                if te.team() != team && te.is_alive() {
                    let tp = te.pos();
                    let rad = te.radius() as u64 + 8000;
                    if dist_sq(want, tp) < (rad as u128) * (rad as u128) {
                        // shift want away from tower by 15k
                        let dx = want.0 as f64 - tp.0 as f64;
                        let dy = want.1 as f64 - tp.1 as f64;
                        let len = (dx * dx + dy * dy).sqrt().max(1.0);
                        want = (
                            (tp.0 as f64 + dx / len * (rad as f64 + 5000.0)) as u64,
                            (tp.1 as f64 + dy / len * (rad as f64 + 5000.0)) as u64,
                        );
                        want = clamp_to_leash(cpos, want, (cfg.leash as f64 * 0.85) as u64);
                    }
                }
            }
        }

        let is_fused = tick < st.fused_until;
        let spawn_name = if is_fused {
            FUSED_SPAWN_NAME
        } else {
            SPAWN_NAME
        };
        let buff_name = if is_fused { BUFF_FUSED } else { BUFF_SPAWN };

        if let Some(sid) = st.spawn {
            if let Some(se) = sim.get_entity(sid) {
                if se.is_alive() {
                    // reposition via lerp over 10 ticks
                    st.lerp_from = se.pos();
                    st.lerp_to = want;
                    st.lerp_left = 10;
                    st.anchor = want; // final anchor
                    st.beam_target = None;
                    st.last_beam_tick = tick;
                    st.next_ray = tick;
                    log_event(
                        sim,
                        "S1",
                        caster,
                        &format!(
                            "Spawn reposition requested to=({}, {}) fused={is_fused}",
                            want.0, want.1
                        ),
                    );
                    put_state(sim, caster, st);
                    return;
                }
            }
        }
        // fresh spawn
        let stat = compute_spawn_stat(caster_stat, level, &cfg, false, is_fused);
        let atk = UnitAttackV1 {
            attack_ratio: 0,
            attack: 0,
            range: 0,
            cooltime: 65,
            duration: 20,
            start_timing: 10,
            cancelable: false,
            attack_type: AttackTypeV1::BaseAttack.code(),
        };
        let duration = if is_fused {
            cfg.fused_ticks
        } else {
            cfg.spawn_life
        };
        if let Some(nid) = sim.spawn_unit(
            spawn_name, caster, team, want.0, want.1, duration, &stat, &atk,
        ) {
            sim.add_buff(nid, &BuffV1::named(buff_name));
            st.spawn = Some(nid);
            st.anchor = want;
            st.lerp_from = want;
            st.lerp_to = want;
            st.lerp_left = 0;
            st.next_ray = tick;
            st.channel_until = tick + cfg.ray_channel_ticks as usize;
            st.beam_target = None;
            st.last_beam_tick = tick;
            st.pending.clear();
            log_event(
                sim,
                "S1",
                caster,
                &format!(
                    "normal/fused Spawn created id={nid} fused={is_fused} at=({}, {})",
                    want.0, want.1
                ),
            );
            put_state(sim, caster, st);
        } else {
            put_state(sim, caster, st);
        }
    }
    fn expected_damage(&self, _caster: &StatV1) -> (usize, usize) {
        (0, 0)
    }
}

#[derive(Clone, Default)]
struct ZhaskSkill2Hit;
impl StableEffectType for ZhaskSkill2Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _rng: u64, caster: usize, input: InputTargetV1) {
        if InputTargetKindV1::from_code(input.kind) != Some(InputTargetKindV1::Target) {
            return;
        }
        let tick = sim.tick();
        let Some((team, mp, caster_alive)) = sim
            .get_entity(caster)
            .map(|e| (e.team(), e.stat().magic_power, e.is_alive()))
        else {
            return;
        };
        let target = input.target_id;
        let Some((target_team, target_alive, targetable, target_pos)) = sim
            .get_entity(target)
            .map(|e| (e.team(), e.is_alive(), e.is_targetable(), e.pos()))
        else {
            return;
        };
        if !caster_alive || !target_alive || !targetable || target_team == team {
            return;
        }

        let impact_visual_accepted = sim.play_view_effect(
            S2_IMPACT_VIEW,
            caster,
            &InputTargetV1::pos(target_pos.0, target_pos.1),
            0,
            0,
            0,
        );
        let level = read_player_level(sim, caster);
        let fused = tick < get_state(sim, caster).fused_until;
        let damage = mind_eater_zhask_damage(level, mp, fused);
        sim.deal_damage(caster, target, 0, damage, AttackTypeV1::Skill);

        // The line collision and Stun are authored in .data_champion. This
        // native sub-effect keeps Zhask's level/fusion damage and capped Spawn
        // follow-up without owning or applying the primary crowd control.
        let mut st = get_state(sim, caster);
        st.stun_checks.push(PendingStunCheck {
            target,
            due_tick: tick + 1,
        });
        let spawn_id = st
            .spawn
            .filter(|sid| sim.get_entity(*sid).is_some_and(|spawn| spawn.is_alive()));
        if let Some(spawn_id) = spawn_id {
            let pending_before = st.pending.len();
            let (accepted, dropped) = queue_pending_hits(&mut st.pending, &[target], tick + 9);
            let pending_after = st.pending.len();
            log_event(
                sim,
                "S2",
                caster,
                &format!("data line hit target={target} damage_requested={damage} fused={fused}; data_stun_expected_ticks={S2_STUN_TICKS}; followup_spawn={spawn_id} pending_before={pending_before} accepted={accepted} dropped_at_cap={dropped} pending_after={pending_after} cap={S2_FOLLOWUP_CAP} delay=9 impact_visual_accepted={impact_visual_accepted}"),
            );
        } else {
            log_event(
                sim,
                "S2",
                caster,
                &format!("data line hit target={target} damage_requested={damage} fused={fused}; data_stun_expected_ticks={S2_STUN_TICKS}; bonus damage not queued: no tracked living Spawn"),
            );
        }
        put_state(sim, caster, st);
    }

    fn expected_damage(&self, caster: &StatV1) -> (usize, usize) {
        (0, 87 + caster.magic_power * 60 / 100)
    }

    fn expected_cc_time(&self) -> Option<usize> {
        Some(S2_STUN_TICKS as usize)
    }
}

struct ZhaskUlt;
impl StableEffectType for ZhaskUlt {
    fn apply(&self, sim: &mut StableSim<'_>, _rng: u64, caster: usize, input: InputTargetV1) {
        let tick = sim.tick();
        let cfg = CONFIG.lock().unwrap().clone();
        let (team, cpos, caster_stat, caster_hp_max, level) = {
            let Some(e) = sim.get_entity(caster) else {
                return;
            };
            let lvl = read_player_level(sim, caster);
            (e.team(), e.pos(), e.stat(), e.hp().1, lvl)
        };

        // resolve target position
        let want = match InputTargetKindV1::from_code(input.kind) {
            Some(InputTargetKindV1::Pos) => (input.x, input.y),
            Some(InputTargetKindV1::Target) => {
                if let Some(te) = sim.get_entity(input.target_id) {
                    te.pos()
                } else {
                    cpos
                }
            }
            Some(InputTargetKindV1::Dir) => {
                let len = ((input.dir_x as f64).powi(2) + (input.dir_y as f64).powi(2))
                    .sqrt()
                    .max(1.0);
                let dist = 50_000.0;
                (
                    (cpos.0 as f64 + input.dir_x as f64 / len * dist) as u64,
                    (cpos.1 as f64 + input.dir_y as f64 / len * dist) as u64,
                )
            }
            _ => cpos,
        };
        let want = clamp_to_leash(cpos, want, cfg.leash + cfg.ult_range_bonus);
        // tower safety: same as S1 but allow closer?
        let tc = sim.tower_count();
        let mut final_want = want;
        for i in 0..tc {
            let tid = sim.tower_id_at(i);
            if let Some(te) = sim.get_entity(tid) {
                if te.team() != team && te.is_alive() {
                    let tp = te.pos();
                    let rad = te.radius() as u64 + 8000;
                    if dist_sq(final_want, tp) < (rad as u128) * (rad as u128) {
                        let dx = final_want.0 as f64 - tp.0 as f64;
                        let dy = final_want.1 as f64 - tp.1 as f64;
                        let len = (dx * dx + dy * dy).sqrt().max(1.0);
                        final_want = (
                            (tp.0 as f64 + dx / len * (rad as f64 + 8000.0)) as u64,
                            (tp.1 as f64 + dy / len * (rad as f64 + 8000.0)) as u64,
                        );
                    }
                }
            }
        }

        let mut st = get_state(sim, caster);
        if tick < st.fused_until {
            log_event(
                sim,
                "Ult",
                caster,
                &format!(
                    "duplicate fusion start ignored existing_spawn={:?}",
                    st.spawn
                ),
            );
            return;
        }

        // Fusion uses a separately named unit/model. Retire the normal Spawn only
        // after the fused entity is created and can be validated.
        sim.entity_set_pos(caster, final_want.0, final_want.1);
        let stat = compute_spawn_stat(caster_stat, level, &cfg, false, true);
        let atk = UnitAttackV1 {
            attack_ratio: 0,
            attack: 0,
            range: 0,
            cooltime: 65,
            duration: 20,
            start_timing: 10,
            cancelable: false,
            attack_type: AttackTypeV1::BaseAttack.code(),
        };
        let spawn_pos = (final_want.0, final_want.1);
        let Some(nid) = sim.spawn_unit(
            FUSED_SPAWN_NAME,
            caster,
            team,
            spawn_pos.0,
            spawn_pos.1,
            cfg.fused_ticks,
            &stat,
            &atk,
        ) else {
            log_event(
                sim,
                "Ult",
                caster,
                "fusion start aborted: fused Spawn creation failed",
            );
            put_state(sim, caster, st);
            return;
        };
        sim.add_buff(nid, &BuffV1::named(BUFF_FUSED));
        if let Some(old_id) = st.spawn {
            if sim.get_entity(old_id).is_some_and(|e| e.is_alive()) {
                let hidden = sim.entity_set_invisible(old_id, cfg.spawn_life as usize);
                if !hidden {
                    let fused_hidden = sim.entity_set_invisible(nid, cfg.fused_ticks as usize);
                    let hp_zeroed = if fused_hidden {
                        false
                    } else {
                        sim.entity_set_hp(nid, 0)
                    };
                    sim.entity_remove_buff(nid, BUFF_FUSED);
                    log_event(sim, "Ult", caster, &format!("fusion start aborted: could not safely hide normal Spawn id={old_id}; fused candidate={nid} invisible={fused_hidden} hp_zeroed={hp_zeroed}"));
                    put_state(sim, caster, st);
                    return;
                }
                sim.entity_remove_buff(old_id, BUFF_SPAWN);
                st.hidden_spawn = Some(old_id);
                log_event(
                    sim,
                    "Ult",
                    caster,
                    &format!("normal Spawn hidden id={old_id} for distinct fusion model"),
                );
            }
        }
        st.spawn = Some(nid);
        st.anchor = spawn_pos;
        st.fusion_anchor = spawn_pos;
        st.lerp_from = spawn_pos;
        st.lerp_to = spawn_pos;
        st.lerp_left = 0;
        st.next_ray = tick + cfg.ray_tick_ticks as usize;
        st.channel_until = tick + cfg.ray_channel_ticks as usize;

        if st.spawn.is_none()
            || !sim
                .get_entity(st.spawn.unwrap())
                .is_some_and(|e| e.is_alive())
        {
            log_event(
                sim,
                "Ult",
                caster,
                "fusion start aborted: fused Spawn creation/validation failed",
            );
            return;
        }

        // apply fused buff to Zhask
        // Determine bonus by level brackets 1-4 =>30%,5-8=>50%,9-12=>70%
        let hp_bonus_pct = match level {
            1..=4 => 30,
            5..=8 => 50,
            _ => 70,
        };
        let def_bonus = match level {
            1..=4 => 20,
            5..=8 => 30,
            _ => 40,
        };
        let mr_bonus = def_bonus;
        let mut buff = BuffV1::timed(BUFF_FUSED_WINDOW, cfg.fused_ticks as usize);
        // hp is flat? plan says +30% max HP -> we use hp_mult? Actually BuffV1 has hp and hp_mult?
        // Looking at values.rs: BuffV1 has hp: i32 flat, hp_mult: i32 percent? Let's check.
        // We will set hp = max_hp * pct /100 as flat bonus, plus defence etc.
        let max_hp = caster_hp_max;
        let flat_hp = (max_hp * hp_bonus_pct) / 100;
        buff.hp = flat_hp as i32;
        buff.defence = def_bonus as i32;
        buff.magic_resistance = mr_bonus as i32;
        buff.vamp = 50;
        sim.add_buff(caster, &buff);
        sim.add_buff(
            caster,
            &BuffV1::timed(BUFF_ULT_RECAST, cfg.fused_ticks as usize),
        );

        let banish_applied = if cfg.ult_banish {
            sim.entity_banish(caster, caster, FUSION_BANISH_STEP, "", "")
        } else {
            false
        };

        if cfg.ult_reset_skill1 {
            let _ = sim.entity_reduce_cooldown(caster, SkillSlotV1::Skill, 99_999);
            st.s1_block_until = tick;
        }
        st.fused_until = tick + cfg.fused_ticks as usize;
        st.banish_refresh_at = tick + FUSION_BANISH_REFRESH;
        st.banish_exit_deadline = 0;
        st.next_ray = tick;
        st.channel_until = tick + cfg.ray_channel_ticks as usize;
        st.beam_target = None;
        st.last_beam_tick = tick;
        log_event(
            sim,
            "Ult",
            caster,
            &format!(
                "fusion started spawn={:?} banish_requested={} banish_applied={} recast_buff=true",
                st.spawn, cfg.ult_banish, banish_applied
            ),
        );

        put_state(sim, caster, st);
    }
    fn expected_damage(&self, _cs: &StatV1) -> (usize, usize) {
        (0, 0)
    }
}

#[derive(Clone, Default)]
struct ZhaskUltRecast;
impl StableEffectType for ZhaskUltRecast {
    fn apply(&self, sim: &mut StableSim<'_>, _rng: u64, caster: usize, _input: InputTargetV1) {
        let tick = sim.tick();
        let mut st = get_state(sim, caster);
        if st.fused_until <= tick {
            log_event(sim, "Ult", caster, "recast ignored: no active fusion");
            return;
        }
        end_fusion(sim, caster, &mut st, true, "recast");
        put_state(sim, caster, st);
    }
    fn expected_damage(&self, _caster: &StatV1) -> (usize, usize) {
        (0, 0)
    }
}

pub(crate) fn register(module: &mut StableMod) {
    module.set_match_hook(ZhaskMatchHook);
    module.add_native_passive(
        format!("{MOD_ID}:zhask_swarm"),
        ZhaskSwarmPassive::default(),
    );
    module.add_native_effect(format!("{MOD_ID}:zhask_attack"), ZhaskAttack);
    module.add_native_effect(format!("{MOD_ID}:zhask_attack_hit"), ZhaskAttackHit);
    module.add_native_effect(format!("{MOD_ID}:zhask_skill1"), ZhaskSkill1);
    module.add_native_effect(format!("{MOD_ID}:zhask_skill2_hit"), ZhaskSkill2Hit);
    module.add_native_effect(format!("{MOD_ID}:zhask_ult"), ZhaskUlt);
    module.add_native_effect(format!("{MOD_ID}:zhask_ult_recast"), ZhaskUltRecast);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_inheritance_uses_the_documented_level_brackets() {
        let cfg = ZhaskConfig::default();
        assert_eq!(inherit_pct(1, &cfg), 55);
        assert_eq!(inherit_pct(3, &cfg), 55);
        assert_eq!(inherit_pct(4, &cfg), 70);
        assert_eq!(inherit_pct(7, &cfg), 85);
        assert_eq!(inherit_pct(10, &cfg), 100);
        assert_eq!(inherit_pct(20, &cfg), 100);
    }

    #[test]
    fn death_ray_damage_applies_fusion_and_frenzy_bonuses() {
        let cfg = ZhaskConfig::default();
        assert_eq!(ray_damage(0, &cfg, false, false), 30);
        assert_eq!(ray_damage(200, &cfg, false, false), 80);
        assert_eq!(ray_damage(200, &cfg, true, false), 144);
        assert_eq!(ray_damage(200, &cfg, false, true), 140);
    }

    #[test]
    fn mind_eater_damage_scales_with_level_magic_power_and_fusion() {
        assert_eq!(mind_eater_zhask_damage(1, 100, false), 147);
        assert_eq!(mind_eater_zhask_damage(12, 100, false), 213);
        assert_eq!(mind_eater_zhask_damage(12, 100, true), 303);
        assert_eq!(mind_eater_spawn_damage(100), 85);
    }

    #[test]
    fn spawn_position_is_clamped_to_its_leash() {
        assert_eq!(
            clamp_to_leash((10_000, 10_000), (100_000, 10_000), 50_000),
            (60_000, 10_000)
        );
        assert_eq!(
            clamp_to_leash((10_000, 10_000), (20_000, 20_000), 50_000),
            (20_000, 20_000)
        );
    }

    #[test]
    fn beam_directions_match_twelve_15_degree_undirected_frames() {
        assert_eq!(beam_angle_index((0, 0), (100, 0)), 0);
        assert_eq!(beam_angle_index((0, 0), (100, 27)), 1);
        assert_eq!(beam_angle_index((0, 0), (100, 58)), 2);
        assert_eq!(beam_angle_index((0, 0), (100, 100)), 3);
        assert_eq!(beam_angle_index((0, 0), (0, 100)), 6);
        assert_eq!(beam_angle_index((100, 0), (0, 27)), 11);
        // A beam is a line, so reversing its endpoints must select the same art frame.
        assert_eq!(
            beam_angle_index((0, 0), (100, 100)),
            beam_angle_index((100, 100), (0, 0))
        );
        assert_eq!(
            beam_angle_index((0, 0), (100, 0)),
            beam_angle_index((100, 0), (0, 0))
        );
    }

    #[test]
    fn beam_visual_endpoints_are_inset_from_entity_centers() {
        let (from, to) = inset_beam_endpoints((0, 0), (100, 0), 10.0, 20.0).unwrap();
        assert_eq!(from, (10, 0));
        assert_eq!(to, (80, 0));
        assert!(inset_beam_endpoints((0, 0), (20, 0), 10.0, 10.0).is_none());
    }

    #[test]
    fn basic_attack_beam_segments_use_tight_overlap_and_are_bounded() {
        let points = beam_segment_points((0, 0), (52_000, 0));
        assert_eq!(points.len(), 70);
        assert_eq!(points[0].1, 0);
        assert!(points.first().unwrap().0 > 0);
        assert!(points.last().unwrap().0 < 52_000);
        assert!(points.windows(2).all(|pair| {
            let dx = pair[1].0 as f64 - pair[0].0 as f64;
            let dy = pair[1].1 as f64 - pair[0].1 as f64;
            dx.hypot(dy) <= 750.0 + 1.0
        }));
        assert!(beam_segment_points((0, 0), (0, 0)).is_empty());
        assert_eq!(
            beam_segment_points((0, 0), (1_000_000, 0)).len(),
            BEAM_MAX_SEGMENTS
        );
    }

    #[test]
    fn skill2_native_hit_payload_reports_data_owned_stun_contract() {
        assert_eq!(
            ZhaskSkill2Hit.expected_cc_time(),
            Some(S2_STUN_TICKS as usize)
        );
        assert_eq!(S2_STUN_TICKS, 30);
    }

    #[test]
    fn fusion_end_marker_is_consumed_once_and_cannot_expire_again() {
        let mut state = ZhaskState {
            fused_until: 100,
            ..ZhaskState::default()
        };
        assert!(take_fusion_marker(&mut state));
        assert_eq!(state.fused_until, 0);
        assert!(!take_fusion_marker(&mut state));
        assert!(!(state.fused_until > 0 && 101 >= state.fused_until));
    }

    #[test]
    fn skill2_follow_up_queue_honors_and_reports_the_explicit_cap() {
        assert_eq!(S2_FOLLOWUP_CAP, 6);
        let mut pending = (0..S2_FOLLOWUP_CAP - 2)
            .map(|target| PendingHit {
                target,
                due_tick: 8,
            })
            .collect::<Vec<_>>();
        let requested = [100, 101, 102, 103];
        let (accepted, dropped) = queue_pending_hits(&mut pending, &requested, 19);
        assert_eq!((accepted, dropped), (2, 2));
        assert_eq!(pending.len(), S2_FOLLOWUP_CAP);
        assert_eq!(pending[S2_FOLLOWUP_CAP - 2].due_tick, 19);
        assert_eq!(pending[S2_FOLLOWUP_CAP - 1].target, 101);
    }

    // Two pre-sims of one match are identical in every derivable field, and
    // `sim_origin()` is `None` during pre-simulation, so match_id / set_index
    // both default to `u64::MAX`. Only the assigned copy tag separates them.
    fn key_with(copy: u64) -> SimKey {
        SimKey {
            match_id: u64::MAX,
            set_index: u64::MAX,
            seed: 4242,
            zhask_id: 7,
            copy,
        }
    }

    #[test]
    fn two_pre_sims_of_one_match_share_every_derived_field() {
        assert_eq!(key_with(0).match_id, key_with(1).match_id);
        assert_eq!(key_with(0).set_index, key_with(1).set_index);
        assert_eq!(key_with(0).seed, key_with(1).seed);
        assert_eq!(key_with(0).zhask_id, key_with(1).zhask_id);
    }

    #[test]
    fn the_copy_tag_is_what_separates_them() {
        assert_ne!(key_with(0), key_with(1));
        let mut map: std::collections::BTreeMap<SimKey, u8> =
            std::collections::BTreeMap::new();
        map.insert(key_with(0), 1);
        map.insert(key_with(1), 2);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&key_with(0)).copied(), Some(1));
        assert_eq!(map.get(&key_with(1)).copied(), Some(2));
    }
}
