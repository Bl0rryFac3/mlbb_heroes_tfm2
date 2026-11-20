//! Granger technical probe. Independent resource clocks and bounded native
//! projectile receipts; no render-frame gameplay and no UI-tree searches.
use crate::{EntityKey, SimScope};
use mod_api_stable::*;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
type Pos = (u64, u64);
#[path = "granger_visual.rs"]
mod visual;
#[path = "granger_voice.rs"]
mod voice;
const SPEED_BUFF: &str = "mlbb_granger_rhapsody_speed";
const CRIT_BUFF: &str = "mlbb_granger_round6_crit_probe";
const CDR_BUFF: &str = "mlbb_granger_cdr_ad";
const SLOTS: usize = 64;
const FLIGHT_TICKS: usize = 180;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Energy {
    current: usize,
    max: usize,
    carry: usize,
}
impl Default for Energy {
    fn default() -> Self {
        Self {
            current: 800,
            max: 800,
            carry: 0,
        }
    }
}
impl Energy {
    fn regen(&mut self, percent: usize) {
        if self.current == self.max {
            self.carry = 0;
            return;
        }
        self.carry = self.carry.saturating_add(self.max.saturating_mul(percent));
        self.current = self.max.min(self.current.saturating_add(self.carry / 6000));
        self.carry %= 6000;
        if self.current == self.max {
            self.carry = 0;
        }
    }
    fn restore(&mut self, percent: usize) {
        self.current = self.max.min(
            self.current
                .saturating_add(self.max.saturating_mul(percent) / 100),
        );
    }
    fn grow(&mut self, amount: usize, percent: usize) {
        self.max = self.max.saturating_add(amount);
        self.restore(percent);
    }
    fn spend(&mut self, amount: usize) -> bool {
        if self.current < amount {
            false
        } else {
            self.current -= amount;
            true
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct Burst {
    direction: (f64, f64),
    target: EntityKey,
    next: usize,
    round: usize,
    damage: usize,
}
#[derive(Clone, Copy, Debug)]
struct Dash {
    direction: (f64, f64),
    remaining: usize,
    speed: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum ShotKind {
    Basic,
    Round(usize),
    Cannon,
}
#[derive(Clone, Copy, Debug)]
struct Shot {
    kind: ShotKind,
    damage: usize,
    expires: usize,
    cancelled: bool,
    target: EntityKey,
}
#[derive(Clone, Debug, Default)]
struct State {
    visual: visual::Visual,
    voice: voice::Voice,
    gun: Energy,
    cannon: Energy,
    updated: Option<usize>,
    burst: Option<Burst>,
    dash: Option<Dash>,
    last_cast: [Option<usize>; 4],
    rondo_ready: usize,
    burst_ready: usize,
    cannon_ready: usize,
    roll_at: Option<(usize, (f64, f64))>,
    cdr_bonus: usize,
    player: usize,
    credited: usize,
    cursor: usize,
    flights: HashMap<usize, Shot>,
    farm: Vec<(EntityKey, usize)>,
    logs: usize,
    dead: bool,
    expired_quiet: Option<usize>,
}
static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static OWNERS: LazyLock<Mutex<HashMap<SimScope, Vec<EntityKey>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn put(key: EntityKey, state: State) {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, state);
}
fn owned(sim: &StableSim<'_>, key: EntityKey) -> bool {
    SimScope::of(sim) == key.scope && crate::sim_copy_of(sim, key.entity_id) == Some(key.copy)
}
fn pos(sim: &StableSim<'_>, id: usize) -> Option<Pos> {
    sim.get_entity(id).map(|e| e.pos())
}
fn dist(a: Pos, b: Pos) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
}
fn unit(a: Pos, b: Pos) -> (f64, f64) {
    let x = b.0 as f64 - a.0 as f64;
    let y = b.1 as f64 - a.1 as f64;
    let n = x.hypot(y);
    if n < 1. {
        (1., 0.)
    } else {
        (x / n, y / n)
    }
}
fn endpoint(p: Pos, d: (f64, f64), range: f64) -> Pos {
    (
        (p.0 as f64 + d.0 * range).max(0.) as u64,
        (p.1 as f64 + d.1 * range).max(0.) as u64,
    )
}
fn foe(sim: &StableSim<'_>, owner: usize, target: usize, structures: bool) -> bool {
    let (Some(a), Some(b)) = (sim.get_entity(owner), sim.get_entity(target)) else {
        return false;
    };
    b.is_alive()
        && b.is_targetable()
        && a.team() != b.team()
        && (structures || !matches!(b.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus)))
}
fn blocked(sim: &StableSim<'_>, id: usize, moving: bool) -> bool {
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
        ) || c.kind == CcKindV1::BlockSkill.code()
            || if moving {
                matches!(
                    CcKindV1::from_code(c.kind),
                    Some(CcKindV1::Bind | CcKindV1::BlockMoveSkill)
                )
            } else {
                c.kind == CcKindV1::BlockSkill.code()
            }
    })
}
fn nearest(sim: &StableSim<'_>, owner: usize, range: f64, heroes: bool) -> Option<usize> {
    let a = sim.get_entity(owner)?;
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| {
            foe(sim, owner, e.id(), false)
                && (!heroes || e.is_champion())
                && sim.is_visible(a.team(), e.id())
                && dist(a.pos(), e.pos()) <= range
        })
        .min_by_key(|e| (dist(a.pos(), e.pos()) as u64, e.id()))
        .map(|e| e.id())
}
fn valid_lock(
    sim: &StableSim<'_>,
    owner: usize,
    target: EntityKey,
    range: f64,
    heroes: bool,
) -> bool {
    owned(sim, target)
        && foe(sim, owner, target.entity_id, false)
        && sim.get_entity(owner).is_some_and(|e| {
            sim.is_visible(e.team(), target.entity_id)
                && sim.get_entity(target.entity_id).is_some_and(|t| {
                    (!heroes || t.is_champion()) && dist(e.pos(), t.pos()) <= range
                })
        })
}
fn lock_target(
    sim: &mut StableSim<'_>,
    owner: usize,
    input: InputTargetV1,
    range: f64,
    heroes: bool,
) -> Option<EntityKey> {
    if input.kind == InputTargetKindV1::Target.code() {
        let key = crate::entity_key(sim, input.target_id);
        if valid_lock(sim, owner, key, range, heroes) {
            return Some(key);
        }
        return None;
    }
    nearest(sim, owner, range, true)
        .or_else(|| {
            if heroes {
                None
            } else {
                nearest(sim, owner, range, false)
            }
        })
        .map(|id| crate::entity_key(sim, id))
}
fn reap_flights(sim: &StableSim<'_>, owner: usize, s: &mut State) {
    let now = sim.tick();
    if now % 30 != 0 || !s.flights.values().any(|f| f.expires < now) {
        return;
    }
    // Homing shots may outlive the damage deadline. Keep their callback slots
    // reserved until the host reports no live projectile for this caster twice.
    let live = (0..sim.projectile_count())
        .filter_map(|i| sim.projectile_at(i))
        .any(|p| p.caster_id == owner && !p.is_end);
    if live {
        s.expired_quiet = None;
        return;
    }
    if s.expired_quiet.is_some_and(|t| now > t) {
        s.flights.retain(|_, f| f.expires >= now);
        s.expired_quiet = None;
    } else {
        s.expired_quiet = Some(now);
    }
}
fn trace(sim: &StableSim<'_>, owner: usize, s: &mut State, message: &str) {
    if s.logs >= 48 {
        return;
    }
    s.logs += 1;
    if sim.sim_origin().is_some_and(|o| {
        matches!(
            SimOriginKindV1::from_code(o.kind),
            Some(
                SimOriginKindV1::ClientMatchView
                    | SimOriginKindV1::ClientSpectate
                    | SimOriginKindV1::ClientReplay
            )
        )
    }) {
        crate::wlog(format!(
            "[GRANGER probe] tick={} owner={owner} gun={}/{} cannon={}/{} {message}",
            sim.tick(),
            s.gun.current,
            s.gun.max,
            s.cannon.current,
            s.cannon.max
        ));
    }
}
fn effect(slot: usize) -> String {
    format!("mlbb_heroes:granger_hit_{slot}")
}

fn launch(
    sim: &mut StableSim<'_>,
    key: EntityKey,
    s: &mut State,
    kind: ShotKind,
    damage: usize,
    direction: (f64, f64),
    target: Option<usize>,
) -> bool {
    let now = sim.tick();
    reap_flights(sim, key.entity_id, s);
    let Some(target) = target else { return false };
    let Some(slot) = (0..SLOTS)
        .map(|i| (s.cursor + i) % SLOTS)
        .find(|i| !s.flights.contains_key(i))
    else {
        return false;
    };
    let Some(e) = sim.get_entity(key.entity_id) else {
        return false;
    };
    let start = e.pos();
    let team = e.team();
    let range = if kind == ShotKind::Cannon {
        140_000.
    } else {
        90_000.
    };
    let end = pos(sim, target).unwrap_or_else(|| endpoint(start, direction, range));
    // Reserve before crossing the host boundary. No projectile is immediate damage.
    s.cursor = (slot + 1) % SLOTS;
    s.flights.insert(
        slot,
        Shot {
            kind,
            damage,
            expires: now + FLIGHT_TICKS,
            cancelled: false,
            target: crate::entity_key(sim, target),
        },
    );
    let ok = sim.spawn_projectile(
        if kind == ShotKind::Cannon {
            "mlbb_granger_cannon"
        } else if kind == ShotKind::Round(6) {
            "mlbb_granger_sixth"
        } else {
            "mlbb_granger_bullet"
        },
        &effect(slot),
        &ProjectileSpawnV1 {
            caster_id: key.entity_id,
            team,
            x: start.0,
            y: start.1,
            radius: if kind == ShotKind::Cannon { 3500 } else { 1200 },
            speed: 6000,
            move_kind: ProjectileMoveKindV1::Target.code(),
            target_id: target,
            target_x: end.0,
            target_y: end.1,
            penetrate: false,
            attack_type: if kind == ShotKind::Basic {
                AttackTypeV1::BaseAttack.code()
            } else {
                AttackTypeV1::Skill.code()
            },
            // Ability aiming and projectile hit input are separate contracts.
            // Target movement homes toward the selected enemy; Targeting delivers it to
            // Hit. The host panics if Direction is used for a projectile.
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: if kind == ShotKind::Cannon {
                CastingTargetV1::EnemyChampion.code()
            } else if kind == ShotKind::Basic {
                CastingTargetV1::Enemy.code()
            } else {
                CastingTargetV1::EnemyWithoutTower.code()
            },
        },
    );
    if !ok {
        s.flights.remove(&slot);
    }
    if ok {
        visual::emitted(sim, key.entity_id, s, kind, target);
        voice::emitted(sim, key.entity_id, s, kind);
    }
    ok
}

// Uses Fanny's captured real wall cells but removes its virtual cable border.
// The short dash checks real ground at each movement step.
fn ground(sim: &StableSim<'_>) -> crate::Grid {
    let live: Vec<_> = (0..sim.champion_count())
        .filter_map(|i| sim.get_entity(sim.champion_id_at(i)))
        .map(|e| e.pos())
        .collect();
    let cache = crate::MAP_GRIDS.lock().unwrap_or_else(|e| e.into_inner());
    let mut g = crate::MODES
        .iter()
        .map(|m| {
            cache
                .get(&m.code())
                .cloned()
                .unwrap_or_else(|| crate::Grid::builtin(*m))
        })
        .min_by_key(|g| {
            live.iter()
                .filter(|p| g.is_wall(p.0 as f64, p.1 as f64))
                .count()
        })
        .unwrap();
    g.bounds = None;
    g
}
fn legal_step(g: &crate::Grid, from: Pos, direction: (f64, f64), length: f64) -> Pos {
    let mut result = from;
    let n = (length / 1000.).ceil().max(1.) as usize;
    for i in 1..=n {
        let p = endpoint(from, direction, length * i as f64 / n as f64);
        if g.solid(p.0 as f64, p.1 as f64) {
            break;
        }
        result = p;
    }
    result
}

fn credits(sim: &StableSim<'_>, s: &mut State) {
    if let Some(p) = sim.get_player(s.player) {
        let total = p.kills().saturating_add(p.assists());
        let n = total.saturating_sub(s.credited);
        for _ in 0..n.min(100) {
            s.gun.grow(12, 10);
            s.cannon.grow(12, 10);
        }
        s.credited = total;
    }
}
fn stop_burst(sim: &mut StableSim<'_>, owner: usize, s: &mut State) {
    s.burst = None;
    sim.entity_remove_buff(owner, SPEED_BUFF);
}
fn update(sim: &mut StableSim<'_>, key: EntityKey) {
    if !owned(sim, key) {
        return;
    }
    let now = sim.tick();
    let Some(mut s) = ({
        let mut all = STATES.lock().unwrap_or_else(|e| e.into_inner());
        if all.get(&key).is_some_and(|s| s.updated == Some(now)) {
            None
        } else {
            all.remove(&key)
        }
    }) else {
        return;
    };
    s.updated = Some(now);
    reap_flights(sim, key.entity_id, &mut s);
    let alive = sim.get_entity(key.entity_id).is_some_and(|e| e.is_alive());
    if !alive {
        stop_burst(sim, key.entity_id, &mut s);
        s.dash = None;
        s.roll_at = None;
        if !s.dead {
            visual::death(sim, key.entity_id, &mut s);
            voice::death(sim, key.entity_id);
        }
        s.dead = true;
        put(key, s);
        return;
    }
    if s.dead {
        s.voice.reset(now, true);
        s.gun.current = s.gun.max;
        s.cannon.current = s.cannon.max;
        s.gun.carry = 0;
        s.cannon.carry = 0;
        s.dead = false;
    }
    voice::tick(sim, key.entity_id, &mut s);
    s.gun.regen(8);
    s.cannon.regen(2);
    credits(sim, &mut s);
    if now % 60 == 0 {
        // Full cycle contains equipment CDR; remaining-time hit refunds do not.
        let cycle = sim
            .get_entity(key.entity_id)
            .and_then(|e| e.cooldown(SkillSlotV1::Skill2))
            .map(|c| c.cooltime)
            .unwrap_or(720);
        let bonus = if cycle > 0 && cycle <= 720 {
            ((720 - cycle) * 100 / 720).min(40) / 2
        } else {
            0
        };
        if bonus != s.cdr_bonus {
            sim.entity_remove_buff(key.entity_id, CDR_BUFF);
            if bonus > 0 {
                let mut b = BuffV1::named(CDR_BUFF);
                b.attack = bonus as i32;
                sim.add_buff(key.entity_id, &b);
            }
            s.cdr_bonus = bonus;
        }
    }
    if blocked(sim, key.entity_id, false) {
        stop_burst(sim, key.entity_id, &mut s);
    }
    if blocked(sim, key.entity_id, true) {
        s.dash = None;
        s.roll_at = None;
    }
    if let Some((at, direction)) = s.roll_at {
        if now >= at {
            visual::action(
                &mut s,
                visual::Motion::Roll,
                now,
                12,
                (-direction.0, -direction.1),
            );
            s.dash = Some(Dash {
                direction,
                remaining: 12,
                speed: 20_000. / 12.,
            });
            s.roll_at = None;
        }
    }
    if let Some(mut dash) = s.dash {
        if let Some(from) = pos(sim, key.entity_id) {
            let to = legal_step(&ground(sim), from, dash.direction, dash.speed);
            sim.entity_set_pos(key.entity_id, to.0, to.1);
            dash.remaining -= 1;
            s.dash = if dash.remaining == 0 || to == from {
                None
            } else {
                Some(dash)
            };
        }
    }
    if let Some(mut b) = s.burst {
        if now >= b.next {
            if !valid_lock(sim, key.entity_id, b.target, 90_000., false) {
                let next = nearest(sim, key.entity_id, 90_000., true)
                    .or_else(|| nearest(sim, key.entity_id, 90_000., false));
                if let Some(id) = next {
                    b.target = crate::entity_key(sim, id);
                } else {
                    stop_burst(sim, key.entity_id, &mut s);
                    visual::show(sim, key.entity_id, &mut s);
                    snapshot(sim, key, &s);
                    put(key, s);
                    return;
                }
            }
            if s.gun.current < 70 {
                stop_burst(sim, key.entity_id, &mut s);
            } else {
                let round = b.round + 1;
                if launch(
                    sim,
                    key,
                    &mut s,
                    ShotKind::Round(round),
                    b.damage,
                    b.direction,
                    Some(b.target.entity_id),
                ) {
                    s.gun.spend(70);
                    b.round = round;
                    b.next = now + 18;
                    trace(
                        sim,
                        key.entity_id,
                        &mut s,
                        &format!("Rhapsody round={round} emitted"),
                    );
                    if round == 6 {
                        stop_burst(sim, key.entity_id, &mut s);
                    } else {
                        s.burst = Some(b);
                    }
                } else {
                    stop_burst(sim, key.entity_id, &mut s);
                }
            }
        }
    }
    visual::show(sim, key.entity_id, &mut s);
    snapshot(sim, key, &s);
    put(key, s);
}
pub(crate) fn tick_all(sim: &mut StableSim<'_>) {
    observe_view(sim);
    let keys = OWNERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&SimScope::of(sim))
        .cloned()
        .unwrap_or_default();
    for key in keys {
        if sim.is_end() && owned(sim, key) {
            STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&key);
            if let Some(keys) = OWNERS
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut(&key.scope)
            {
                keys.retain(|k| *k != key);
            }
            continue;
        }
        update(sim, key);
    }
}

#[derive(Clone, Copy)]
enum Op {
    Basic,
    Burst,
    Rondo,
    Cannon,
}
impl StableEffectType for Op {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, owner: usize, input: InputTargetV1) {
        if !sim.get_entity(owner).is_some_and(|e| e.is_alive()) {
            return;
        }
        let key = crate::entity_key(sim, owner);
        let now = sim.tick();
        let Some(mut s) = STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key)
        else {
            return;
        };
        let index = match self {
            Self::Basic => 0,
            Self::Burst => 1,
            Self::Rondo => 2,
            Self::Cannon => 3,
        };
        if s.last_cast[index] == Some(now)
            || (!matches!(self, Self::Basic) && blocked(sim, owner, matches!(self, Self::Rondo)))
        {
            put(key, s);
            return;
        }
        let from = pos(sim, owner).unwrap();
        let aim = if input.kind == InputTargetKindV1::Target.code() {
            pos(sim, input.target_id)
        } else if input.kind == InputTargetKindV1::Dir.code() {
            Some((
                (from.0 as i128 + input.dir_x as i128).clamp(0, u64::MAX as i128) as u64,
                (from.1 as i128 + input.dir_y as i128).clamp(0, u64::MAX as i128) as u64,
            ))
        } else {
            Some((input.x, input.y))
        };
        let direction = unit(from, aim.unwrap_or(endpoint(from, (1., 0.), 10_000.)));
        let ad = sim.get_entity(owner).unwrap().stat().attack;
        match self {
            Self::Basic => {
                if s.burst.is_none()
                    && input.kind == InputTargetKindV1::Target.code()
                    && foe(sim, owner, input.target_id, true)
                {
                    launch(
                        sim,
                        key,
                        &mut s,
                        ShotKind::Basic,
                        ad,
                        direction,
                        Some(input.target_id),
                    );
                }
            }
            Self::Burst => {
                if s.burst.is_none() && s.gun.current > 420 && now >= s.burst_ready {
                    let Some(target) = lock_target(sim, owner, input, 90_000., false) else {
                        put(key, s);
                        return;
                    };
                    s.burst_ready = now + 18;
                    s.burst = Some(Burst {
                        direction,
                        target,
                        next: now,
                        round: 0,
                        damage: 36 + ad * 45 / 100,
                    });
                    let mut b = BuffV1::named(SPEED_BUFF);
                    b.move_speed_mult = 10;
                    b.duration_kind = BuffDurationV1::Time.code();
                    b.duration_tick = 100;
                    sim.add_buff(owner, &b);
                    trace(sim, owner, &mut s, "Rhapsody accepted");
                }
            }
            Self::Rondo => {
                if now >= s.rondo_ready && s.dash.is_none() {
                    s.gun.restore(15);
                    s.rondo_ready = now + 720;
                    s.roll_at = None;
                    s.dash = Some(Dash {
                        direction,
                        remaining: 12,
                        speed: 35_000. / 12.,
                    });
                    visual::action(&mut s, visual::Motion::Rondo, now, 12, direction);
                    trace(sim, owner, &mut s, "Rondo accepted");
                }
            }
            Self::Cannon => {
                if now >= s.cannon_ready && s.cannon.current >= 250 {
                    let Some(target) = lock_target(sim, owner, input, 140_000., true) else {
                        put(key, s);
                        return;
                    };
                    if launch(
                        sim,
                        key,
                        &mut s,
                        ShotKind::Cannon,
                        155 + ad * 110 / 100,
                        direction,
                        Some(target.entity_id),
                    ) {
                        s.cannon.spend(250);
                        s.cannon_ready = now + 36;
                        // Short AI retreat roll, independent of the burst scheduler.
                        s.roll_at = Some((now + 12, (-direction.0, -direction.1)));
                        trace(sim, owner, &mut s, "Death Sonata emitted; roll queued");
                    }
                }
            }
        }
        visual::show(sim, owner, &mut s);
        s.last_cast[index] = Some(now);
        snapshot(sim, key, &s);
        put(key, s);
    }
}

#[derive(Clone, Copy)]
struct Hit(usize);
impl StableEffectType for Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, owner: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let key = crate::entity_key_ro(sim, owner);
        let shot = STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&key)
            .and_then(|s| s.flights.remove(&self.0));
        let Some(shot) = shot else { return };
        if shot.cancelled
            || sim.tick() > shot.expires
            || !owned(sim, key)
            || !foe(sim, owner, input.target_id, shot.kind == ShotKind::Basic)
            || (input.target_id == shot.target.entity_id && !owned(sim, shot.target))
        {
            return;
        }
        if shot.kind == ShotKind::Cannon
            && !sim
                .get_entity(input.target_id)
                .is_some_and(|e| e.is_champion())
        {
            return;
        }
        if let Some(p) = pos(sim, input.target_id) {
            let fx = match shot.kind {
                ShotKind::Cannon => "cannon_impact",
                ShotKind::Round(6) => "sixth_hit",
                _ => "gun_hit",
            };
            visual::once(
                sim,
                owner,
                (p.0, p.1.saturating_sub(20000)),
                fx,
                if shot.kind == ShotKind::Cannon {
                    24
                } else {
                    15
                },
            );
        }
        // Consume before engine calls: callbacks may re-enter on_kill/on_assist.
        if matches!(shot.kind, ShotKind::Round(_)) {
            if let Some(s) = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut(&key)
            {
                s.rondo_ready = s.rondo_ready.saturating_sub(30).max(sim.tick());
            }
            sim.entity_reduce_cooldown(owner, SkillSlotV1::Skill2, 30);
        }
        let targets = if shot.kind == ShotKind::Cannon {
            let center = pos(sim, input.target_id).unwrap();
            let mut ids: Vec<_> = (0..sim.entity_count())
                .filter_map(|i| sim.entity_at(i))
                .filter(|e| foe(sim, owner, e.id(), false) && dist(center, e.pos()) <= 18_000.)
                .map(|e| e.id())
                .collect();
            ids.push(input.target_id);
            ids.sort_unstable();
            ids.dedup();
            ids
        } else {
            vec![input.target_id]
        };
        for target in targets {
            if shot.kind == ShotKind::Round(6) {
                // ABI has no force-crit bit. Probe its documented crit pipeline
                // with a strictly scoped chance buff; NEVER multiply manually.
                let chance = sim
                    .get_entity(owner)
                    .map(|e| e.stat().crit_chance)
                    .unwrap_or(0);
                let mut b = BuffV1::named(CRIT_BUFF);
                b.crit_chance = 100usize.saturating_sub(chance) as i32;
                sim.add_buff(owner, &b);
                sim.deal_damage(owner, target, shot.damage, 0, AttackTypeV1::Skill);
                sim.entity_remove_buff(owner, CRIT_BUFF);
            } else {
                sim.deal_damage(
                    owner,
                    target,
                    shot.damage,
                    0,
                    if shot.kind == ShotKind::Basic {
                        AttackTypeV1::BaseAttack
                    } else {
                        AttackTypeV1::Skill
                    },
                );
            }
            if shot.kind == ShotKind::Cannon {
                let mut b = BuffV1::named(&format!("mlbb_granger_slow_{owner}"));
                b.move_speed_mult = -30;
                b.duration_kind = BuffDurationV1::Time.code();
                b.duration_tick = 36;
                sim.entity_remove_buff(target, b.name());
                sim.add_buff(target, &b);
            }
        }
    }
}

#[derive(Default)]
struct Passive {
    owner: Option<EntityKey>,
}
impl StablePassive for Passive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(Self::default())
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, player: usize, owner: usize) {
        let key = crate::entity_key(sim, owner);
        let mut previous = self
            .owner
            .take()
            .filter(|old| old.scope == key.scope)
            .and_then(|old| {
                OWNERS
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get_mut(&old.scope)
                    .map(|v| v.retain(|k| *k != old));
                STATES
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&old)
            })
            .unwrap_or_default();
        visual::clear(sim, owner, &mut previous);
        previous.visual = visual::Visual::default();
        previous.voice.reset(sim.tick(), previous.dead);
        previous.burst = None;
        previous.dash = None;
        previous.roll_at = None;
        previous.updated = None;
        previous.player = player;
        previous.dead = false;
        previous.gun.current = previous.gun.max;
        previous.cannon.current = previous.cannon.max;
        previous.gun.carry = 0;
        previous.cannon.carry = 0;
        for shot in previous.flights.values_mut() {
            shot.cancelled = true;
        }
        let mut owners = OWNERS.lock().unwrap_or_else(|e| e.into_inner());
        let keys = owners.entry(key.scope).or_default();
        if !keys.contains(&key) {
            keys.push(key);
        }
        drop(owners);
        self.owner = Some(key);
        trace(
            sim,
            owner,
            &mut previous,
            "spawn; owner animations; round6 true-crit host validation pending",
        );
        visual::show(sim, owner, &mut previous);
        put(key, previous);
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, owner: usize) {
        let key = crate::entity_key(sim, owner);
        if self.owner != Some(key) {
            self.on_spawn(sim, player, owner);
        }
        update(sim, key);
    }
    fn on_damaged(
        &mut self,
        sim: &mut StableSim<'_>,
        _: usize,
        owner: usize,
        _: usize,
        damage: usize,
    ) {
        if damage == 0 {
            return;
        }
        let key = crate::entity_key_ro(sim, owner);
        let state = STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key);
        if let Some(mut s) = state {
            if s.visual.action.is_none_or(|(_, t)| t <= sim.tick()) {
                let dir = if s.visual.left { (-1., 0.) } else { (1., 0.) };
                visual::action(&mut s, visual::Motion::Hit, sim.tick(), 12, dir);
                visual::show(sim, owner, &mut s);
            }
            put(key, s);
        }
    }
    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        if let Some(key) = self.owner {
            let state = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&key);
            if let Some(mut s) = state {
                if !s.dead {
                    visual::death(sim, key.entity_id, &mut s);
                    voice::death(sim, key.entity_id);
                }
                stop_burst(sim, key.entity_id, &mut s);
                s.dash = None;
                s.roll_at = None;
                s.dead = true;
                put(key, s);
            }
        }
    }
    fn on_kill(&mut self, sim: &mut StableSim<'_>, _: usize, owner: usize, victim: usize) {
        if !sim.get_entity(victim).is_some_and(|e| e.is_champion()) {
            return;
        }
        let key = crate::entity_key_ro(sim, owner);
        let state = STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key);
        if let Some(mut s) = state {
            voice::kill(sim, owner, &mut s);
            put(key, s);
        }
    }
    fn on_unit_kill(&mut self, sim: &mut StableSim<'_>, _: usize, owner: usize, victim: usize) {
        let kind = sim.get_entity(victim).and_then(|e| e.kind());
        if !matches!(kind, Some(EntityKindV1::Minion | EntityKindV1::Jungle)) {
            return;
        }
        let v = crate::entity_key(sim, victim);
        let key = crate::entity_key_ro(sim, owner);
        let now = sim.tick();
        if let Some(s) = STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&key)
        {
            s.farm.retain(|(_, t)| now.saturating_sub(*t) <= 600);
            if s.farm.iter().any(|(k, _)| *k == v) {
                return;
            }
            if s.farm.len() >= 64 {
                s.farm.remove(0);
            }
            s.farm.push((v, now));
            if kind == Some(EntityKindV1::Minion) {
                s.gun.grow(6, 5);
            } else {
                s.cannon.grow(6, 5);
            }
        }
    }
}

struct Ai;
impl StablePlayerAi for Ai {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(Self)
    }
    fn id(&self) -> String {
        "mlbb_granger_ai".into()
    }
    fn priority(&self) -> i32 {
        170
    }
    fn matches(&self, i: &StableAiInit) -> bool {
        i.champion_name == "mlbb_granger"
    }
    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        if base.is_some_and(|input| InputKindV1::from_code(input.kind) == Some(InputKindV1::Return)) {
            return base;
        }
        let player = ctx.player_id();
        let proposal = (|| {
            let sim = ctx.sim()?;
            let e = sim.get_player(player)?.champion()?;
            let key = crate::entity_key_ro(&sim, e.id());
            let s = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&key)?
                .clone();
            if !e.is_alive() {
                return None;
            }
            let ready = |slot| {
                e.cooldown(slot)
                    .is_some_and(|c| c.unlocked && c.remaining == 0 && c.charges > 0)
            };
            let target = nearest(&sim, e.id(), 140_000., true)
                .or_else(|| nearest(&sim, e.id(), 90_000., false))?;
            let p = pos(&sim, target)?;
            let d = unit(e.pos(), p);
            if ready(SkillSlotV1::Ult)
                && s.cannon.current >= 250
                && sim.tick() >= s.cannon_ready
                && sim.get_entity(target)?.is_champion()
            {
                Some(InputV1::action(
                    InputKindV1::Ult,
                    InputTargetV1::target(target),
                ))
            } else if ready(SkillSlotV1::Skill2)
                && sim.tick() >= s.rondo_ready
                && s.dash.is_none()
                && (s.gun.current < s.gun.max * 3 / 4 || dist(e.pos(), p) < 35_000.)
            {
                Some(InputV1::action(
                    InputKindV1::Skill2,
                    InputTargetV1::dir((-d.0 * 100_000.) as i64, (-d.1 * 100_000.) as i64),
                ))
            } else if ready(SkillSlotV1::Skill)
                && s.gun.current > 420
                && s.burst.is_none()
                && sim.tick() >= s.burst_ready
                && nearest(&sim, e.id(), 90_000., false).is_some()
            {
                Some(InputV1::action(
                    InputKindV1::Skill,
                    InputTargetV1::target(
                        nearest(&sim, e.id(), 90_000., true)
                            .or_else(|| nearest(&sim, e.id(), 90_000., false))?,
                    ),
                ))
            } else {
                base
            }
        })();
        proposal.filter(|i| ctx.is_valid_input(i)).or_else(|| {
            base.filter(|i| {
                if !matches!(
                    InputKindV1::from_code(i.kind),
                    Some(InputKindV1::Skill | InputKindV1::Skill2 | InputKindV1::Ult)
                ) {
                    return true;
                }
                false
            })
        })
    }
}

#[derive(Clone)]
struct Hud {
    scope: SimScope,
    owner: usize,
    gun: Energy,
    cannon: Energy,
    rondo: usize,
}
static HUD: LazyLock<Mutex<Vec<Hud>>> = LazyLock::new(|| Mutex::new(Vec::new()));
fn observe_view(sim: &StableSim<'_>) {
    let scope = SimScope::of(sim);
    if matches!(
        SimOriginKindV1::from_code(scope.kind),
        Some(
            SimOriginKindV1::ClientMatchView
                | SimOriginKindV1::ClientSpectate
                | SimOriginKindV1::ClientReplay
        )
    ) {
        HUD.lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|h| h.scope == scope);
    }
}
fn visible_scene(ctx: &StableClient<'_>) -> bool {
    ctx.scene_kind() == Some(SceneKindV1::InGame)
        && matches!(
            ctx.client_scene_kind(),
            Some(
                ClientSceneKindV1::Match
                    | ClientSceneKindV1::InGame
                    | ClientSceneKindV1::SetFeedback
                    | ClientSceneKindV1::TutorialMorgad
                    | ClientSceneKindV1::TutorialSerpen
                    | ClientSceneKindV1::Tutorial5v5
            )
        )
}
fn snapshot(sim: &StableSim<'_>, key: EntityKey, s: &State) {
    if !matches!(
        SimOriginKindV1::from_code(key.scope.kind),
        Some(
            SimOriginKindV1::ClientMatchView
                | SimOriginKindV1::ClientSpectate
                | SimOriginKindV1::ClientReplay
        )
    ) {
        return;
    }
    let mut hud = HUD.lock().unwrap_or_else(|e| e.into_inner());
    hud.retain(|h| h.scope == key.scope && h.owner != key.entity_id);
    hud.push(Hud {
        scope: key.scope,
        owner: key.entity_id,
        gun: s.gun,
        cannon: s.cannon,
        rondo: s.rondo_ready.saturating_sub(sim.tick()),
    });
    hud.sort_by_key(|h| h.owner);
}
pub(crate) struct Extension;
impl StableExtension for Extension {
    fn pre_update(&self, ctx: &mut StableClient<'_>, dt: u64) {
        crate::audio::VoicePathExtension.pre_update(ctx, dt);
        if !visible_scene(ctx) {
            HUD.lock().unwrap_or_else(|e| e.into_inner()).clear();
        }
    }
    fn post_update(&self, ctx: &mut StableClient<'_>, dt: u64) {
        crate::audio::VoicePathExtension.post_update(ctx, dt);
    }
    fn on_end(&self) {
        crate::audio::VoicePathExtension.on_end();
        HUD.lock().unwrap_or_else(|e| e.into_inner()).clear();
        STATES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        OWNERS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    fn post_render(&self, ctx: &mut StableClient<'_>) {
        if !ctx.can_draw() || !visible_scene(ctx) {
            return;
        }
        let Some((w, h)) = ctx.draw_map_size("UI") else {
            return;
        };
        let rows = HUD.lock().unwrap_or_else(|e| e.into_inner()).clone();
        for (i, row) in rows.iter().take(10).enumerate() {
            let x = (w - 270.).max(0.);
            let y = (h * 0.4 + i as f32 * 66.).min(h - 66.);
            ctx.draw_rect("UI", x, y, 264., 62., 90, 4., 0x111820e0);
            for (j, (pool, color)) in [(row.gun, 0xffc459ff), (row.cannon, 0xe76b78ff)]
                .into_iter()
                .enumerate()
            {
                let by = y + 26. + j as f32 * 14.;
                ctx.draw_rect("UI", x + 8., by, 248., 9., 91, 2., 0x333b47ff);
                ctx.draw_rect(
                    "UI",
                    x + 8.,
                    by,
                    248. * pool.current as f32 / pool.max.max(1) as f32,
                    9.,
                    92,
                    2.,
                    color,
                );
            }
            ctx.draw_text(
                "UI",
                &format!(
                    "Granger #{}  G {}/{}  C {}/{}  R {:.1}s",
                    row.owner,
                    row.gun.current,
                    row.gun.max,
                    row.cannon.current,
                    row.cannon.max,
                    row.rondo as f32 / 60.
                ),
                "asset/base/font/set/regular",
                (x + 8., y + 2., 248., 22.),
                93,
                12.,
                0xffffffff,
                TextAlignXV1::Left,
                TextAlignYV1::Center,
            );
        }
    }
}
pub fn register(module: &mut StableMod) {
    for (name, op) in [
        ("basic", Op::Basic),
        ("burst", Op::Burst),
        ("rondo", Op::Rondo),
        ("cannon", Op::Cannon),
    ] {
        module.add_native_effect(&format!("mlbb_heroes:granger_{name}"), op);
    }
    for i in 0..SLOTS {
        module.add_native_effect(&effect(i), Hit(i));
    }
    module.add_native_passive("mlbb_heroes:granger", Passive::default());
    module.add_player_input_ai(Ai);
}

#[cfg(test)]
#[path = "granger_tests.rs"]
mod tests;
