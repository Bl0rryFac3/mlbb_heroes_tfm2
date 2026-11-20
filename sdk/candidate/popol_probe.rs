//! Popol and Kupa: companion, cooperative passive, traps and mounted combat.
//! Body sprites are cosmetic view buffs on transparent carrier atlases. No hide,
//! banish, protection or targetability change is used to switch the pose.
use crate::EntityKey;
use mod_api_stable::*;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

type Pos = (u64, u64);
#[path = "popol_navigation.rs"]
mod navigation;
use navigation::Navigation;
#[path = "popol_combat.rs"]
mod combat;
#[path = "popol_cooperation.rs"]
mod cooperation;
#[path = "popol_map.rs"]
mod maplayout;
#[path = "popol_traps.rs"]
mod traps;
#[path = "popol_visual.rs"]
mod visual;
const UNIT: &str = "mlbb_kupa_probe";
const OWNER_BODY: &str = "mlbb_popol_probe_body";
const KUPA_BODY: &str = "mlbb_kupa_probe_body";
const ALPHA_BODY: &str = "mlbb_kupa_probe_alpha";
const RIDE_BODY: &str = "mlbb_popol_probe_ride";
const ALPHA_STATS: &str = "mlbb_kupa_probe_alpha_stats";
const RIDE_SPEED: &str = "mlbb_popol_probe_ride_speed";
const RIDE_TICKS: usize = 90;
const ALPHA_TICKS: usize = 540;
const FOLLOW: f64 = 18_000.;
const LEASH: f64 = 100_000.;
const REACH: f64 = 14_000.;
const SPEED: f64 = 1_600.;
const INTERVAL: usize = 66;

#[derive(Clone, Default)]
struct State {
    traps: traps::Traps,
    cooperation: cooperation::Cooperation,
    action_until: usize,
    intro_attempted: bool,
    visual: visual::Visual,
    kupa: Option<EntityKey>,
    pending_spawn: Option<(usize, usize)>,
    spawn_attempted: bool,
    updated: Option<usize>,
    target: Option<(EntityKey, usize, bool)>,
    bite_at: Option<(EntityKey, usize, bool)>,
    next_bite: usize,
    alpha_until: usize,
    recalling: bool,
    recall_until: usize,
    ride_until: usize,
    last_skill: [Option<usize>; 3],
    navigation: Navigation,
    dead_since: Option<usize>,
    prayer_started: Option<usize>,
    revive_ready: usize,
    last_combat: usize,
    last_owner_pos: Option<Pos>,
    last_owner_hp: usize,
    last_kupa_hp: usize,
    observed_kupa: Option<(Pos, usize)>,
    jump_logs: usize,
}
static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static OWNERS: LazyLock<Mutex<HashMap<crate::SimScope, Vec<EntityKey>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn index_owner(key: EntityKey) {
    let mut scopes = OWNERS.lock().unwrap_or_else(|e| e.into_inner());
    let keys = scopes.entry(key.scope).or_default();
    if !keys.contains(&key) {
        keys.push(key);
    }
}
fn forget_owner(key: EntityKey) {
    let mut scopes = OWNERS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(keys) = scopes.get_mut(&key.scope) {
        keys.retain(|k| *k != key);
        if keys.is_empty() {
            scopes.remove(&key.scope);
        }
    }
}

fn pos(sim: &StableSim<'_>, id: usize) -> Option<Pos> {
    sim.get_entity(id).map(|e| e.pos())
}
fn dist(a: Pos, b: Pos) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
}
fn owned(sim: &StableSim<'_>, key: EntityKey) -> bool {
    crate::sim_copy_of(sim, key.entity_id) == Some(key.copy)
        && crate::SimScope::of(sim) == key.scope
}
fn alive(sim: &StableSim<'_>, key: EntityKey) -> bool {
    owned(sim, key) && sim.get_entity(key.entity_id).is_some_and(|e| e.is_alive())
}
fn foe(sim: &StableSim<'_>, owner: usize, target: usize, structures: bool) -> bool {
    let (Some(a), Some(b)) = (sim.get_entity(owner), sim.get_entity(target)) else {
        return false;
    };
    a.is_alive()
        && b.is_alive()
        && b.is_targetable()
        && a.team() != b.team()
        && (structures || !matches!(b.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus)))
}
fn blocked(sim: &StableSim<'_>, id: usize, movement: bool) -> bool {
    let Some(e) = sim.get_entity(id) else {
        return true;
    };
    (0..e.cc_count()).filter_map(|i| e.cc_at(i)).any(|c| {
        matches!(
            CcKindV1::from_code(c.kind),
            Some(
                CcKindV1::Stun
                    | CcKindV1::Airborne
                    | CcKindV1::ForceMove
                    | CcKindV1::Fear
                    | CcKindV1::Charm
                    | CcKindV1::Taunt
            )
        ) || (movement && c.kind == CcKindV1::Bind.code())
            || (!movement && c.kind == CcKindV1::BlockAttack.code())
    })
}
fn put(key: EntityKey, state: State) {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, state);
}
fn take(sim: &mut StableSim<'_>, owner: usize) -> (EntityKey, State) {
    let key = crate::entity_key(sim, owner);
    let state = STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key);
    if state.is_none() {
        index_owner(key);
    }
    (key, state.unwrap_or_default())
}
fn body(sim: &mut StableSim<'_>, id: usize, wanted: Option<&str>, choices: &[&str]) {
    for name in choices {
        if Some(*name) != wanted {
            sim.entity_remove_buff(id, name);
        }
    }
    if let Some(name) = wanted {
        let present = sim.get_entity(id).is_some_and(|e| {
            (0..e.buff_count())
                .filter_map(|i| e.buff_at(i))
                .any(|b| b.name() == name)
        });
        if !present {
            sim.add_buff(id, &BuffV1::named(name));
        }
    }
}
fn show_bodies(sim: &mut StableSim<'_>, owner: usize, state: &mut State) {
    visual::show(sim, owner, state);
}

fn end_ride(sim: &mut StableSim<'_>, owner: usize, state: &mut State, why: &str) {
    if state.ride_until == 0 && !state.recalling {
        return;
    }
    if state.ride_until > 0
        && why != "companion_dead"
        && why != "owner_dead"
        && !blocked(sim, owner, true)
        && sim.get_entity(owner).is_some_and(|e| e.is_alive())
    {
        state.visual.dismount_until = sim.tick() + 21;
        state.visual.dismount_alpha = state.alpha_until > sim.tick();
    }
    if why == "companion_dead" || why == "owner_relocated" || blocked(sim, owner, true) {
        state.visual.owner_action = None;
        state.visual.dismount_until = 0;
    }
    state.ride_until = 0;
    state.recalling = false;
    state.recall_until = 0;
    sim.entity_remove_buff(owner, RIDE_SPEED);
    show_bodies(sim, owner, state);
    crate::wlog(format!(
        "[POPOL probe] dismount owner={owner} tick={} reason={why}; owner position unchanged",
        sim.tick()
    ));
}
fn legal_step(grid: &crate::Grid, from: Pos, to: Pos, speed: f64) -> Pos {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let n = dx.hypot(dy);
    if n < 1. {
        return from;
    }
    let travel = n.min(speed);
    let endpoint = (
        (from.0 as f64 + dx / n * travel).max(0.) as u64,
        (from.1 as f64 + dy / n * travel).max(0.) as u64,
    );
    // Sweep the short step, including a small body clearance. Do not cut walls.
    let clear = |p: Pos| {
        [
            (-4_000., 0.),
            (4_000., 0.),
            (0., -4_000.),
            (0., 4_000.),
            (0., 0.),
        ]
        .into_iter()
        .all(|(x, y)| !grid.solid(p.0 as f64 + x, p.1 as f64 + y))
    };
    if (1..=4).all(|i| {
        clear((
            (from.0 as f64 + (endpoint.0 as f64 - from.0 as f64) * i as f64 / 4.) as u64,
            (from.1 as f64 + (endpoint.1 as f64 - from.1 as f64) * i as f64 / 4.) as u64,
        ))
    }) {
        return endpoint;
    }
    // Slide along a wall. Full cross-map routing is outside this proof build.
    for p in [(endpoint.0, from.1), (from.0, endpoint.1)] {
        if clear(p) && dist(p, to) < n {
            return p;
        }
    }
    from
}
fn move_toward(
    sim: &mut StableSim<'_>,
    id: usize,
    target: Pos,
    nav: &mut Navigation,
    speed: f64,
) -> bool {
    if blocked(sim, id, true) {
        return false;
    }
    let Some(from) = pos(sim, id) else {
        return false;
    };
    let (mode, grid, changed) = nav.layout.select(sim, from, target);
    if changed {
        nav.clear_route();
        crate::wlog(format!("[POPOL navigation] layout={mode} companion={id} tick={} from={from:?} target={target:?}; shared Fanny wall data, no virtual cable bounds",sim.tick()));
    }
    let next = nav.step_with_speed(&grid, from, target, sim.tick(), speed);
    let moved = next != from && sim.entity_set_pos(id, next.0, next.1);
    if moved {
        nav.blocked_count = 0;
    } else {
        nav.blocked_count += 1;
        if nav.blocked_count == 180 || nav.blocked_count % 600 == 0 {
            crate::wlog(format!("[POPOL navigation] stalled companion={id} layout={mode} tick={} attempts={} from={from:?} target={target:?} start_wall={} target_wall={}",sim.tick(),nav.blocked_count,grid.is_wall(from.0 as f64,from.1 as f64),grid.is_wall(target.0 as f64,target.1 as f64)));
        }
    }
    moved
}
fn spawn(sim: &mut StableSim<'_>, owner: usize, state: &mut State) {
    if let Some((id, requested)) = state.pending_spawn {
        if sim.get_entity(id).is_some_and(|e| e.is_alive()) {
            state.kupa = Some(crate::entity_key(sim, id));
            state.observed_kupa = None;
            state.pending_spawn = None;
            show_bodies(sim, owner, state);
        } else if sim.tick() > requested + 60 {
            state.pending_spawn = None;
            crate::wlog(format!(
                "[POPOL probe] companion spawn not readable owner={owner} id={id}"
            ));
        }
        return;
    }
    if state.spawn_attempted {
        return;
    }
    state.spawn_attempted = true;
    let Some(e) = sim.get_entity(owner) else {
        return;
    };
    let stat = cooperation::companion_stat(sim, owner);
    let p = e.pos();
    let team = e.team();
    // Native state drives movement and damage. The carrier's engine attack has zero reach/damage.
    let attack = UnitAttackV1 {
        attack_ratio: 0,
        attack: 0,
        range: 0,
        cooltime: 360_000,
        duration: 0,
        start_timing: 0,
        ..Default::default()
    };
    if let Some(id) = sim.spawn_unit(UNIT, owner, team, p.0, p.1, 180_000, &stat, &attack) {
        state.pending_spawn = Some((id, sim.tick()));
        spawn(sim, owner, state);
        crate::wlog(format!(
            "[POPOL probe] companion requested owner={owner} id={id} tick={}",
            sim.tick()
        ));
    } else {
        crate::wlog(format!(
            "[POPOL probe] companion spawn refused owner={owner}"
        ));
    }
}
fn nearest(sim: &StableSim<'_>, owner: usize) -> Option<usize> {
    let p = pos(sim, owner)?;
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| foe(sim, owner, e.id(), false) && dist(p, e.pos()) <= LEASH)
        .min_by_key(|e| (!e.is_champion(), dist(p, e.pos()) as u64, e.id()))
        .map(|e| e.id())
}
fn update(sim: &mut StableSim<'_>, key: EntityKey, mut state: State) {
    let owner = key.entity_id;
    let now = sim.tick();
    if state.updated.is_some_and(|t| t >= now) {
        put(key, state);
        return;
    }
    state.updated = Some(now);
    if !alive(sim, key) {
        traps::clear(sim, key, &mut state.traps);
        state.cooperation.clear(
            sim,
            owner,
            state.kupa.filter(|k| owned(sim, *k)).map(|k| k.entity_id),
        );
        forget_owner(key);
        combat::reset(key);
        end_ride(sim, owner, &mut state, "owner_dead");
        if let Some(k) = state.kupa.filter(|k| owned(sim, *k)) {
            sim.entity_set_hp(k.entity_id, 0);
        }
        state.kupa = None;
        state.bite_at = None;
        show_bodies(sim, owner, &mut state);
        put(key, state);
        return;
    }
    let owner_pos_now = pos(sim, owner).unwrap();
    state.navigation.layout.observe(owner_pos_now);
    let owner_hp_now = sim.get_entity(owner).unwrap().hp().0;
    let moved = state
        .last_owner_pos
        .is_some_and(|p| dist(p, owner_pos_now) > 500.);
    if state.last_owner_hp > owner_hp_now {
        state.last_combat = now;
    }
    state.last_owner_hp = owner_hp_now;
    state.last_owner_pos = Some(owner_pos_now);
    let may_place = state.traps.may_attempt(now)
        && (state.last_combat > 0 || state.action_until > 0)
        && now.saturating_sub(state.last_combat) <= 120
        && state.kupa.is_some_and(|k| alive(sim, k))
        && now >= state.action_until
        && state.prayer_started.is_none()
        && !state.recalling
        && state.ride_until <= now
        && sim.get_entity(owner).is_some_and(|e| e.is_targetable())
        && !blocked(sim, owner, true);
    let trap_triggered = traps::tick(sim, key, &mut state.traps, &mut state.navigation, may_place);
    if trap_triggered {
        state.last_combat = now;
        state.prayer_started = None;
    }
    if trap_triggered && !alive(sim, key) {
        traps::clear(sim, key, &mut state.traps);
        state.cooperation.clear(
            sim,
            owner,
            state.kupa.filter(|k| owned(sim, *k)).map(|k| k.entity_id),
        );
        forget_owner(key);
        combat::reset(key);
        end_ride(sim, owner, &mut state, "owner_dead");
        if let Some(k) = state.kupa.filter(|k| owned(sim, *k)) {
            sim.entity_set_hp(k.entity_id, 0);
        }
        state.kupa = None;
        show_bodies(sim, owner, &mut state);
        put(key, state);
        return;
    }
    spawn(sim, owner, &mut state);
    let Some(k) = state.kupa.filter(|k| alive(sim, *k)) else {
        state.cooperation.clear(
            sim,
            owner,
            state.kupa.filter(|k| owned(sim, *k)).map(|k| k.entity_id),
        );
        end_ride(sim, owner, &mut state, "companion_dead");
        state.bite_at = None;
        if state.pending_spawn.is_none() && state.spawn_attempted {
            if state.dead_since.is_none() {
                if let Some(k) = state.kupa.filter(|k| owned(sim, *k)) {
                    visual::once(sim, owner, k.entity_id, "kupa_dead", false, 54);
                }
                state.dead_since = Some(now);
                state.last_combat = now;
            }
            let safe = now.saturating_sub(state.last_combat) >= 180
                && !blocked(sim, owner, true)
                && sim.get_entity(owner).is_some_and(|e| e.is_targetable());
            if let Some(start) = state.prayer_started {
                if moved || !safe {
                    state.prayer_started = None;
                    crate::wlog(format!(
                        "[POPOL recovery] prayer interrupted owner={owner} tick={now}"
                    ));
                } else if now >= start + 180 {
                    state.kupa = None;
                    state.pending_spawn = None;
                    state.spawn_attempted = false;
                    state.prayer_started = None;
                    state.dead_since = None;
                    state.alpha_until = 0;
                    state.target = None;
                    state.navigation = Navigation::default();
                    state.last_kupa_hp = 0;
                    state.revive_ready = now + 1800;
                    state.visual.revived_until = now + 36;
                    spawn(sim, owner, &mut state);
                    crate::wlog(format!("[POPOL recovery] prayer completed; fresh companion requested owner={owner} tick={now}"));
                }
            } else if safe && !moved && now >= state.revive_ready {
                state.prayer_started = Some(now);
                crate::wlog(format!(
                    "[POPOL recovery] prayer started owner={owner} tick={now}"
                ));
            }
        }
        show_bodies(sim, owner, &mut state);
        put(key, state);
        return;
    };
    let kupa_pos_now = pos(sim, k.entity_id).unwrap();
    if let Some((previous, tick)) = state.observed_kupa {
        if now == tick + 1 && dist(previous, kupa_pos_now) > 12_000. && state.jump_logs < 8 {
            state.jump_logs += 1;
            crate::wlog(format!("[POPOL navigation] observed position jump companion={} tick={now} from={previous:?} to={kupa_pos_now:?} mounted={} recalling={}",k.entity_id,state.ride_until>now,state.recalling));
        }
    }
    state.observed_kupa = Some((kupa_pos_now, now));
    let kupa_hp_now = sim.get_entity(k.entity_id).unwrap().hp().0;
    if state.last_kupa_hp > kupa_hp_now {
        state.visual.kupa_action = Some(("hit", now + 12));
        state.last_combat = now;
    }
    state.last_kupa_hp = kupa_hp_now;
    if state.alpha_until > 0 && now >= state.alpha_until {
        state.alpha_until = 0;
        sim.entity_remove_buff(k.entity_id, ALPHA_STATS);
        let (h, m) = sim.get_entity(k.entity_id).unwrap().hp();
        if h > m {
            sim.entity_set_hp(k.entity_id, m);
        }
        show_bodies(sim, owner, &mut state);
        crate::wlog(format!(
            "[POPOL probe] alpha ended owner={owner} companion={} tick={now}",
            k.entity_id
        ));
    }
    state.cooperation.tick(
        sim,
        owner,
        k.entity_id,
        state.alpha_until > now,
        state.last_combat,
    );
    state.last_kupa_hp = sim.get_entity(k.entity_id).unwrap().hp().0;
    if state.ride_until > 0 {
        let attach_gap = pos(sim, k.entity_id).map_or(f64::MAX, |p| dist(p, owner_pos_now));
        if attach_gap > 12_000. {
            crate::wlog(format!("[POPOL navigation] owner relocated during mount owner={owner} companion={} tick={now} gap={attach_gap}; dismount without companion warp",k.entity_id));
            end_ride(sim, owner, &mut state, "owner_relocated");
        } else if now >= state.ride_until
            || !sim.get_entity(owner).is_some_and(|e| e.is_targetable())
            || blocked(sim, owner, true)
            || blocked(sim, k.entity_id, true)
        {
            end_ride(sim, owner, &mut state, "expired_or_cc");
        } else {
            if let Some(p) = pos(sim, owner) {
                sim.entity_set_pos(k.entity_id, p.0, p.1);
            }
            if now % 6 == 0 && state.alpha_until <= now {
                sim.heal(
                    owner,
                    k.entity_id,
                    (sim.get_entity(k.entity_id).unwrap().hp().1 / 60).max(1),
                );
            }
            show_bodies(sim, owner, &mut state);
            put(key, state);
            return;
        }
    }
    if state.recalling {
        if now >= state.recall_until || blocked(sim, owner, true) || blocked(sim, k.entity_id, true)
        {
            end_ride(sim, owner, &mut state, "recall_timeout_or_cc");
            show_bodies(sim, owner, &mut state);
            put(key, state);
            return;
        }
        let p = pos(sim, owner).unwrap();
        let q = pos(sim, k.entity_id).unwrap();
        if dist(p, q) <= 8_000. && !blocked(sim, owner, true) && !blocked(sim, k.entity_id, true) {
            // Popol remains the ONLY moving root. Kupa follows this ordinary targetable entity.
            state.recalling = false;
            state.recall_until = 0;
            state.ride_until = now + RIDE_TICKS;
            state.visual.mounted_at = now;
            state.visual.dismount_until = 0;
            state.bite_at = None;
            if state.alpha_until > now {
                let max = sim.get_entity(k.entity_id).unwrap().hp().1;
                sim.heal(owner, k.entity_id, max);
            }
            sim.add_buff(
                owner,
                &BuffV1 {
                    move_speed_mult: 50,
                    ..BuffV1::timed(RIDE_SPEED, RIDE_TICKS)
                },
            );
            show_bodies(sim, owner, &mut state);
            crate::wlog(format!("[POPOL probe] mount owner={owner} companion={} tick={now} targetable={}; no banish/invisible",k.entity_id,sim.get_entity(owner).is_some_and(|e|e.is_targetable())));
        } else {
            move_toward(
                sim,
                k.entity_id,
                p,
                &mut state.navigation,
                SPEED
                    * (1.
                        + (state.cooperation.stacks
                            * 2
                            * if state.alpha_until > now { 2 } else { 1 })
                            as f64
                            / 100.),
            );
        }
        show_bodies(sim, owner, &mut state);
        put(key, state);
        return;
    }
    if let Some((target, at, command)) = state.bite_at {
        if now >= at {
            state.bite_at = None;
            if alive(sim, target)
                && foe(sim, owner, target.entity_id, false)
                && !blocked(sim, k.entity_id, false)
                && dist(
                    pos(sim, k.entity_id).unwrap(),
                    pos(sim, target.entity_id).unwrap(),
                ) <= REACH + 2_000.
            {
                let ad = sim.get_entity(owner).unwrap().stat().attack;
                let alpha = state.alpha_until > now;
                let amount = if command {
                    if alpha {
                        90 + ad * 75 / 100
                    } else {
                        60 + ad * 60 / 100
                    }
                } else {
                    ad * if alpha { 75 } else { 45 } / 100
                };
                state.last_combat = now;
                if command && alpha {
                    let victims =
                        combat::alpha_bite(sim, owner, pos(sim, k.entity_id).unwrap(), amount);
                    if alive(sim, key) && alive(sim, k) {
                        for victim in victims {
                            state
                                .cooperation
                                .hit(crate::entity_key_ro(sim, victim), now, true);
                        }
                    }
                } else {
                    let before = cooperation::health(sim, target.entity_id);
                    sim.deal_damage_typed(
                        owner,
                        target.entity_id,
                        amount,
                        DamageTypeV1::Ad,
                        if command {
                            AttackTypeV1::Skill
                        } else {
                            AttackTypeV1::BaseAttack
                        },
                    );
                    if cooperation::health(sim, target.entity_id) < before
                        && alive(sim, key)
                        && alive(sim, k)
                    {
                        state.cooperation.hit(target, now, true);
                    }
                    if command {
                        sim.add_buff(
                            target.entity_id,
                            &BuffV1 {
                                move_speed_mult: -50,
                                ..BuffV1::timed("mlbb_popol_probe_slow", 60)
                            },
                        );
                    }
                }
                if command {
                    state.target = None;
                }
            }
        }
        if !alive(sim, key) {
            state.cooperation.clear(sim, owner, Some(k.entity_id));
            traps::clear(sim, key, &mut state.traps);
            combat::reset(key);
            forget_owner(key);
            sim.entity_set_hp(k.entity_id, 0);
        }
        show_bodies(sim, owner, &mut state);
        put(key, state);
        return;
    }
    let chosen = state
        .target
        .filter(|(t, until, _)| {
            *until > now && alive(sim, *t) && foe(sim, owner, t.entity_id, false)
        })
        .map(|(t, _, command)| (t.entity_id, command))
        .or_else(|| nearest(sim, owner).map(|id| (id, false)));
    let owner_pos = pos(sim, owner).unwrap();
    let pet_pos = pos(sim, k.entity_id).unwrap();
    if dist(owner_pos, pet_pos) > LEASH * 0.7 {
        move_toward(
            sim,
            k.entity_id,
            owner_pos,
            &mut state.navigation,
            SPEED
                * (1.
                    + (state.cooperation.stacks * 2 * if state.alpha_until > now { 2 } else { 1 })
                        as f64
                        / 100.),
        );
        show_bodies(sim, owner, &mut state);
        put(key, state);
        return;
    }
    if let Some((target, command)) =
        chosen.filter(|(t, _)| pos(sim, *t).is_some_and(|p| dist(owner_pos, p) <= LEASH))
    {
        let p = pos(sim, target).unwrap();
        if dist(p, pet_pos) > REACH {
            move_toward(
                sim,
                k.entity_id,
                p,
                &mut state.navigation,
                SPEED
                    * (1.
                        + (state.cooperation.stacks
                            * 2
                            * if state.alpha_until > now { 2 } else { 1 })
                            as f64
                            / 100.),
            );
        } else if now >= state.next_bite && !blocked(sim, k.entity_id, false) {
            state.visual.kupa_action = Some((if command { "skill" } else { "attack" }, now + 20));
            state.bite_at = Some((crate::entity_key(sim, target), now + 12, command));
            state.next_bite = now + cooperation::interval(sim, owner);
        }
    } else if dist(owner_pos, pet_pos) > FOLLOW {
        move_toward(
            sim,
            k.entity_id,
            owner_pos,
            &mut state.navigation,
            SPEED
                * (1.
                    + (state.cooperation.stacks * 2 * if state.alpha_until > now { 2 } else { 1 })
                        as f64
                        / 100.),
        );
    }
    show_bodies(sim, owner, &mut state);
    put(key, state);
}

pub(crate) fn tick_all(sim: &mut StableSim<'_>) {
    let scope = crate::SimScope::of(sim);
    let mut keys: Vec<_> = OWNERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&scope)
        .cloned()
        .unwrap_or_default();
    keys.sort_by_key(|k| k.entity_id);
    for key in keys {
        if !owned(sim, key) {
            continue;
        }
        let state = {
            let mut states = STATES.lock().unwrap_or_else(|e| e.into_inner());
            if states
                .get(&key)
                .is_some_and(|s| s.updated.is_some_and(|t| t >= sim.tick()))
            {
                None
            } else {
                states.remove(&key)
            }
        };
        if let Some(state) = state {
            update(sim, key, state);
        }
    }
}

#[derive(Clone, Copy)]
enum Op {
    Basic,
    Command,
    Ride,
    Alpha,
}
impl StableEffectType for Op {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, owner: usize, input: InputTargetV1) {
        if !sim
            .get_entity(owner)
            .is_some_and(|e| e.is_alive() && e.is_targetable())
        {
            return;
        }
        let (key, mut state) = take(sim, owner);
        let now = sim.tick();
        if matches!(self, Self::Basic) {
            if state.ride_until > now {
                end_ride(sim, owner, &mut state, "mounted_attack");
            }
        } else if matches!(self, Self::Command | Self::Alpha) {
            end_ride(sim, owner, &mut state, "new_skill");
        }
        if !matches!(self, Self::Ride) {
            state.prayer_started = None;
            state.last_combat = now;
        }
        match self {
            Self::Basic => {
                state.visual.owner_action = Some(("attack", now + 20));
                if input.kind == InputTargetKindV1::Target.code()
                    && foe(sim, owner, input.target_id, true)
                {
                    state.action_until = now + 20;
                    combat::launch(sim, owner, input.target_id, combat::SpearKind::Basic);
                    if foe(sim, owner, input.target_id, false)
                        && !state
                            .target
                            .is_some_and(|(_, until, command)| command && until > now)
                    {
                        state.target =
                            Some((crate::entity_key(sim, input.target_id), now + 180, false));
                    }
                }
            }
            Self::Command => {
                if state.last_skill[0] != Some(now)
                    && input.kind == InputTargetKindV1::Target.code()
                    && foe(sim, owner, input.target_id, false)
                {
                    state.last_skill[0] = Some(now);
                    state.action_until = now + 18;
                    state.visual.owner_action = Some(("skill", now + 18));
                    combat::launch(sim, owner, input.target_id, combat::SpearKind::Command);
                    state.target = Some((crate::entity_key(sim, input.target_id), now + 300, true));
                    state.next_bite = now;
                    state.bite_at = None;
                }
            }
            Self::Ride => {
                if state.last_skill[1] != Some(now)
                    && state.ride_until == 0
                    && state.kupa.is_some_and(|k| alive(sim, k))
                {
                    state.last_skill[1] = Some(now);
                    state.action_until = now + 12;
                    state.visual.owner_action = Some(("skill2", now + 18));
                    state.visual.shield_until = now + 180;
                    state.visual.shield_start_until = now + 27;
                    crate::wlog(format!("[POPOL recall] requested owner={owner} tick={now} companion={:?} distance={:?}",state.kupa.map(|k|k.entity_id),state.kupa.and_then(|k|pos(sim,k.entity_id)).map(|p|dist(p,pos(sim,owner).unwrap()))));
                    state.recalling = true;
                    state.recall_until = now + 180;
                    state.target = None;
                    state.bite_at = None;
                    let ad = sim.get_entity(owner).unwrap().stat().attack;
                    sim.entity_add_shield(
                        owner,
                        if state.alpha_until > now {
                            140 + ad * 70 / 100
                        } else {
                            90 + ad / 2
                        },
                        180,
                    );
                }
            }
            Self::Alpha => {
                if state.last_skill[2] != Some(now) {
                    if let Some(k) = state.kupa.filter(|k| alive(sim, *k)) {
                        state.last_skill[2] = Some(now);
                        state.action_until = now + 18;
                        state.alpha_until = now + ALPHA_TICKS;
                        state.visual.owner_action = Some(("ult", now + 18));
                        state.visual.kupa_action = Some(("transform", now + 36));
                        sim.entity_remove_buff(k.entity_id, ALPHA_STATS);
                        sim.add_buff(
                            k.entity_id,
                            &BuffV1 {
                                hp_mult: 35,
                                ..BuffV1::timed(ALPHA_STATS, ALPHA_TICKS)
                            },
                        );
                        let max = sim.get_entity(k.entity_id).unwrap().hp().1;
                        sim.heal(owner, k.entity_id, max);
                        show_bodies(sim, owner, &mut state);
                        crate::wlog(format!("[POPOL probe] alpha owner={owner} companion={} tick={now}; same entity preserved",k.entity_id));
                    }
                }
            }
        }
        // One intro opportunity on a real combat action; no spawn/pause backlog,
        // repeated basic-attack chatter or background-simulation audio.
        if !state.intro_attempted
            && matches!(self, Self::Basic | Self::Command)
            && now >= 60
            && alive(sim, key)
            && matches!(
                sim.sim_origin()
                    .and_then(|o| SimOriginKindV1::from_code(o.kind)),
                Some(
                    SimOriginKindV1::ClientMatchView
                        | SimOriginKindV1::ClientSpectate
                        | SimOriginKindV1::ClientReplay
                )
            )
        {
            state.intro_attempted = true;
            crate::audio::play_voice(sim, owner, "mlbb_popol_voice_intro");
        }
        put(key, state);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            match self {
                Self::Basic => stat.attack,
                Self::Command => 50 + stat.attack / 2,
                _ => 0,
            },
            0,
        )
    }
}

#[derive(Clone, Default)]
struct Passive {
    owner: Option<EntityKey>,
}
impl StablePassive for Passive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _: usize, owner: usize) {
        if let Some(old) = self.owner.take() {
            forget_owner(old);
            combat::reset(old);
            let previous = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&old);
            if let Some(mut previous) = previous {
                traps::clear(sim, old, &mut previous.traps);
                previous.cooperation.clear(
                    sim,
                    old.entity_id,
                    previous
                        .kupa
                        .filter(|k| owned(sim, *k))
                        .map(|k| k.entity_id),
                );
                visual::clear(sim, old.entity_id, &previous);
                if let Some(k) = previous.kupa.filter(|k| owned(sim, *k)) {
                    body(sim, k.entity_id, None, &[KUPA_BODY, ALPHA_BODY]);
                    sim.entity_set_hp(k.entity_id, 0);
                }
            }
        }
        let key = crate::entity_key(sim, owner);
        self.owner = Some(key);
        index_owner(key);
        combat::reset(key);
        let mut state = State::default();
        if let Some(p) = pos(sim, owner) {
            state.navigation.layout.observe(p);
        }
        spawn(sim, owner, &mut state);
        show_bodies(sim, owner, &mut state);
        put(key, state);
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, _: usize, owner: usize) {
        let key = crate::entity_key_ro(sim, owner);
        if STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
            .is_some_and(|s| s.updated.is_some_and(|t| t >= sim.tick()))
        {
            return;
        }
        let (k, s) = take(sim, owner);
        update(sim, k, s);
    }
    fn on_damaged(&mut self, sim: &mut StableSim<'_>, _: usize, _: usize, _: usize, damage: usize) {
        if damage > 0 {
            if let Some(key) = self.owner {
                if let Some(state) = STATES
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get_mut(&key)
                {
                    state.visual.owner_action = Some(("hit", sim.tick() + 12));
                    state.last_combat = sim.tick();
                    state.prayer_started = None;
                }
            }
        }
    }
    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        if let Some(key) = self.owner.take() {
            forget_owner(key);
            combat::reset(key);
            let state = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&key);
            if let Some(mut state) = state {
                traps::clear(sim, key, &mut state.traps);
                state.cooperation.clear(
                    sim,
                    key.entity_id,
                    state.kupa.filter(|k| owned(sim, *k)).map(|k| k.entity_id),
                );
                visual::death(sim, key.entity_id, &state);
                end_ride(sim, key.entity_id, &mut state, "owner_dead");
                body(sim, key.entity_id, None, &[OWNER_BODY, RIDE_BODY]);
                if let Some(k) = state.kupa.filter(|k| owned(sim, *k)) {
                    body(sim, k.entity_id, None, &[KUPA_BODY, ALPHA_BODY]);
                    sim.entity_set_hp(k.entity_id, 0);
                }
            }
        }
    }
}
struct ProbeAi;
#[derive(Clone, Copy)]
struct AiState {
    kupa: Option<EntityKey>,
    pending_spawn: Option<(usize, usize)>,
    last_combat: usize,
    revive_ready: usize,
    prayer_started: Option<usize>,
    recalling: bool,
    ride_until: usize,
    alpha_until: usize,
}
impl From<&State> for AiState {
    fn from(s: &State) -> Self {
        Self {
            kupa: s.kupa,
            pending_spawn: s.pending_spawn,
            last_combat: s.last_combat,
            revive_ready: s.revive_ready,
            prayer_started: s.prayer_started,
            recalling: s.recalling,
            ride_until: s.ride_until,
            alpha_until: s.alpha_until,
        }
    }
}
impl StablePlayerAi for ProbeAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(Self)
    }
    fn id(&self) -> String {
        "mlbb_popol_probe_ai".into()
    }
    fn priority(&self) -> i32 {
        170
    }
    fn matches(&self, i: &StableAiInit) -> bool {
        i.champion_name == "mlbb_popol_kupa"
    }
    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        let player = ctx.player_id();
        let mut suppress_fallback = false;
        let proposal = (|| {
            let sim = ctx.sim()?;
            let e = sim.get_player(player)?.champion()?;
            if !e.is_alive() || !e.is_targetable() {
                suppress_fallback = true;
                return None;
            }
            let state = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&crate::entity_key_ro(&sim, e.id()))
                .map(AiState::from)?;
            let companion_dead = state.kupa.is_some_and(|k| !alive(&sim, k));
            let may_pray = companion_dead
                && state.pending_spawn.is_none()
                && sim.tick().saturating_sub(state.last_combat) >= 180
                && sim.tick() >= state.revive_ready;
            if state.prayer_started.is_some() || may_pray || state.recalling {
                suppress_fallback = true;
                let p = e.pos();
                return Some(InputV1::move_to(p.0, p.1));
            }
            if state.ride_until > sim.tick() {
                suppress_fallback = true;
                if let Some(i) = base.filter(|i| i.kind == InputKindV1::Move.code()) {
                    return Some(i);
                }
                let from = e.pos();
                let target = nearest(&sim, e.id()).and_then(|id| pos(&sim, id))?;
                let dx = from.0 as f64 - target.0 as f64;
                let dy = from.1 as f64 - target.1 as f64;
                let n = dx.hypot(dy).max(1.);
                return Some(InputV1::move_to(
                    (from.0 as f64 + dx / n * 50_000.).max(0.) as u64,
                    (from.1 as f64 + dy / n * 50_000.).max(0.) as u64,
                ));
            }
            let target = nearest(&sim, e.id())?;
            let ready = |slot| {
                e.cooldown(slot)
                    .is_some_and(|c| c.unlocked && c.charges > 0 && c.remaining == 0)
            };
            let (hp, max) = e.hp();
            if ready(SkillSlotV1::Skill2)
                && hp * 100 < max * 85
                && state.kupa.is_some_and(|k| alive(&sim, k))
            {
                Some(InputV1::action(
                    InputKindV1::Skill2,
                    InputTargetV1::target(e.id()),
                ))
            } else if ready(SkillSlotV1::Ult)
                && state.alpha_until <= sim.tick()
                && state.kupa.is_some_and(|k| alive(&sim, k))
            {
                Some(InputV1::action(
                    InputKindV1::Ult,
                    InputTargetV1::target(e.id()),
                ))
            } else if ready(SkillSlotV1::Skill) {
                Some(InputV1::action(
                    InputKindV1::Skill,
                    InputTargetV1::target(target),
                ))
            } else {
                base
            }
        })();
        match proposal.filter(|i| ctx.is_valid_input(i)) {
            Some(input) => Some(input),
            None if !suppress_fallback => base,
            None => None,
        }
    }
}
pub fn register(module: &mut StableMod) {
    combat::register(module);
    for (name, op) in [
        ("basic", Op::Basic),
        ("command", Op::Command),
        ("ride", Op::Ride),
        ("alpha", Op::Alpha),
    ] {
        module.add_native_effect(&format!("mlbb_heroes:popol_probe_{name}"), op);
    }
    module.add_native_passive("mlbb_heroes:popol_probe", Passive::default());
    module.add_player_input_ai(ProbeAi);
}

#[cfg(test)]
#[path = "popol_probe_tests.rs"]
mod tests;
