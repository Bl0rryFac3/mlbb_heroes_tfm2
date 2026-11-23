use crate::{entity_key, entity_key_ro, sim_copy_of, EntityKey};
use mod_api_stable::*;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

type Position = (u64, u64);
const READY: &str = "mlbb_lesley_lethal_ready";
const CAMO: &str = "mlbb_lesley_camouflage_edge";
const LOCK: &str = "mlbb_lesley_ult_lock";
const ULT_RANGE: f64 = 150_000.;
const GRENADE_RANGE: f64 = 30_000.;
const WINDUP: usize = 30;
const CADENCE: usize = 21;
const CAMO_DURATION: usize = 150;
const CAMO_HIT_GRACE: usize = 30;
const CAMO_REFRESH: usize = 2;

fn lethal_shot_champion_damage(attack: usize) -> usize {
    75 + attack * 110 / 100
}

#[derive(Clone, Copy)]
struct Volley {
    target: EntityKey,
    origin: Position,
    lock_tick: usize,
    fired: usize,
}

#[derive(Default)]
struct State {
    ready_at: usize,
    ready: bool,
    camo_until: usize,
    camo_grace: usize,
    volley: Option<Volley>,
}

static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn take(key: EntityKey) -> State {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key)
        .unwrap_or_default()
}

fn put(key: EntityKey, state: State) {
    let mut states = STATES.lock().unwrap_or_else(|e| e.into_inner());
    if states.len() > 4096 {
        states.retain(|other, _| other.scope == key.scope);
    }
    states.insert(key, state);
}

fn position(sim: &StableSim<'_>, id: usize) -> Option<Position> {
    sim.get_entity(id).map(|e| e.pos())
}

fn enemy(sim: &StableSim<'_>, caster: usize, target: usize) -> bool {
    match (sim.get_entity(caster), sim.get_entity(target)) {
        (Some(a), Some(b)) => a.team() != b.team() && b.is_alive() && b.is_targetable(),
        _ => false,
    }
}

fn vector(a: Position, b: Position) -> (f64, f64) {
    (b.0 as f64 - a.0 as f64, b.1 as f64 - a.1 as f64)
}

fn offset(p: Position, x: f64, y: f64) -> Position {
    (
        (p.0 as f64 + x).max(0.) as u64,
        (p.1 as f64 + y).max(0.) as u64,
    )
}

fn distance(a: Position, b: Position) -> f64 {
    let (x, y) = vector(a, b);
    x.hypot(y)
}

fn ground_fx(sim: &mut StableSim<'_>, caster: usize, point: Position, name: &str) {
    if sim
        .sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
    {
        sim.play_view_effect(name, caster, &InputTargetV1::pos(point.0, point.1), 0, 0, 0);
    }
}

fn cue(sim: &mut StableSim<'_>, id: usize, name: &str) {
    if sim
        .sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
    {
        sim.play_sfx(
            &format!("mlbb_lesley_{name}"),
            id,
            &InputTargetV1::target(id),
        );
    }
}

fn prime(sim: &mut StableSim<'_>, caster: usize, state: &mut State) {
    if !state.ready {
        sim.add_buff(
            caster,
            &BuffV1 {
                range: 17_000,
                ..BuffV1::named(READY)
            },
        );
        state.ready = true;
    }
}

fn unprime(sim: &mut StableSim<'_>, caster: usize, state: &mut State, now: usize) {
    if state.ready {
        sim.entity_remove_buff(caster, READY);
        state.ready = false;
    }
    state.ready_at = now + 300;
}

fn reveal(sim: &mut StableSim<'_>, caster: usize, state: &mut State) {
    if state.camo_until != 0 {
        sim.entity_remove_buff(caster, CAMO);
        state.camo_until = 0;
    }
}

fn refresh_camouflage(sim: &mut StableSim<'_>, caster: usize, state: &mut State, now: usize) {
    let remaining = state.camo_until.saturating_sub(now);
    if remaining == 0 {
        reveal(sim, caster, state);
    } else {
        // The stable API has a timed vision flag and no explicit reveal call.
        // Short renewals let a shot or hit reveal her within one or two ticks.
        sim.entity_set_invisible(caster, remaining.min(CAMO_REFRESH));
    }
}

fn lock_name(caster: usize) -> String {
    format!("{LOCK}_{caster}")
}

fn clear_lock(sim: &mut StableSim<'_>, state: &mut State, caster: usize) {
    if let Some(volley) = state.volley.take() {
        let name = lock_name(caster);
        if sim_copy_of(sim, volley.target.entity_id) == Some(volley.target.copy) {
            let target = volley.target.entity_id;
            sim.entity_remove_buff(target, &name);
            let prefix = "mlbb_lesley_ult_lock_";
            let has_other_lock = sim.get_entity(target).is_some_and(|entity| {
                (0..entity.buff_count())
                    .filter_map(|i| entity.buff_at(i))
                    .any(|buff| buff.name().starts_with(prefix))
            });
            if !has_other_lock {
                sim.entity_remove_buff(target, LOCK);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shot {
    Basic,
    Lethal,
    Ultimate,
}

impl Shot {
    fn tracer(self) -> &'static str {
        match self {
            Self::Basic => "mlbb_lesley_basic_tracer",
            Self::Lethal => "mlbb_lesley_lethal_tracer",
            Self::Ultimate => "mlbb_lesley_ult_projectile",
        }
    }
    fn receipt(self) -> &'static str {
        match self {
            Self::Basic => "mlbb_heroes:lesley_basic_hit",
            Self::Lethal => "mlbb_heroes:lesley_lethal_hit",
            Self::Ultimate => "mlbb_heroes:lesley_ult_hit",
        }
    }
    fn stem(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Lethal => "lethal",
            Self::Ultimate => "ult",
        }
    }
    fn attack_type(self) -> AttackTypeV1 {
        if self == Self::Ultimate {
            AttackTypeV1::Skill
        } else {
            AttackTypeV1::BaseAttack
        }
    }
}

fn fire(sim: &mut StableSim<'_>, caster: usize, target: usize, shot: Shot) -> bool {
    let (Some(a), Some(b)) = (sim.get_entity(caster), sim.get_entity(target)) else {
        return false;
    };
    let from = a.pos();
    let end = b.pos();
    let team = a.team();
    let (x, y) = vector(from, end);
    let span = x.hypot(y).max(1.);
    let muzzle = offset(from, x * 6500. / span, y * 6500. / span);
    let spawned = sim.spawn_projectile(
        shot.tracer(),
        shot.receipt(),
        &ProjectileSpawnV1 {
            caster_id: caster,
            team,
            x: muzzle.0,
            y: muzzle.1,
            radius: 1400,
            speed: 9000,
            move_kind: ProjectileMoveKindV1::Target.code(),
            target_id: target,
            target_x: end.0,
            target_y: end.1,
            penetrate: false,
            attack_type: shot.attack_type().code(),
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: if shot == Shot::Ultimate {
                CastingTargetV1::EnemyChampion.code()
            } else {
                CastingTargetV1::Enemy.code()
            },
        },
    );
    if spawned {
        cue(sim, caster, shot.stem());
        let suffix = if x < 0. { "_left" } else { "" };
        ground_fx(
            sim,
            caster,
            muzzle,
            &format!("mlbb_lesley_{}_muzzle{suffix}", shot.stem()),
        );
    }
    spawned
}

fn damage(sim: &mut StableSim<'_>, caster: usize, target: usize, shot: Shot) {
    if !enemy(sim, caster, target) {
        return;
    }
    let Some(victim) = sim.get_entity(target) else {
        return;
    };
    let impact = victim.pos();
    let attack = sim.get_entity(caster).map_or(0, |e| e.stat().attack);
    let (hp, max_hp) = victim.hp();
    let structure = matches!(
        victim.kind(),
        Some(EntityKindV1::Tower | EntityKindV1::Nexus)
    );
    let amount = match shot {
        Shot::Basic => attack,
        Shot::Lethal if structure => attack,
        Shot::Lethal if victim.is_champion() => lethal_shot_champion_damage(attack),
        Shot::Lethal => 75 + attack * 125 / 100,
        Shot::Ultimate => 70 + attack * 32 / 100 + max_hp.saturating_sub(hp) * 6 / 100,
    };
    let flavor = if shot == Shot::Lethal && victim.is_champion() {
        DamageTypeV1::Fixed
    } else {
        DamageTypeV1::Ad
    };
    sim.deal_damage_typed(caster, target, amount, flavor, shot.attack_type());
    ground_fx(
        sim,
        caster,
        impact,
        &format!("mlbb_lesley_{}_impact", shot.stem()),
    );
}

fn first_in_line(sim: &StableSim<'_>, caster: usize, locked: usize) -> Option<usize> {
    let (from, to) = position(sim, caster).zip(position(sim, locked))?;
    let (dx, dy) = vector(from, to);
    let length = dx.hypot(dy);
    if length <= 1. {
        return Some(locked);
    }
    (0..sim.champion_count())
        .map(|i| sim.champion_id_at(i))
        .filter(|&id| enemy(sim, caster, id))
        .filter_map(|id| {
            let (x, y) = vector(from, position(sim, id)?);
            let forward = (x * dx + y * dy) / length;
            let lateral = (x * dy - y * dx).abs() / length;
            (forward > 0. && forward < length && lateral <= 4600.).then_some((forward, id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, id)| id)
        .or(Some(locked))
}

fn valid_volley(sim: &StableSim<'_>, caster: usize, aim: Volley) -> bool {
    let id = aim.target.entity_id;
    sim_copy_of(sim, id) == Some(aim.target.copy)
        && enemy(sim, caster, id)
        && position(sim, caster)
            .zip(position(sim, id))
            .is_some_and(|(a, b)| distance(aim.origin, a) < 4800. && distance(a, b) <= ULT_RANGE)
        && !crate::granger::blocked(sim, caster, false)
}

fn grenade(sim: &mut StableSim<'_>, caster: usize, input: InputTargetV1) {
    let Some(from) = position(sim, caster) else {
        return;
    };
    let goal = if input.kind == InputTargetKindV1::Target.code() {
        let Some(target) = position(sim, input.target_id) else {
            return;
        };
        target
    } else {
        (input.x, input.y)
    };
    let (x, y) = vector(from, goal);
    let length = x.hypot(y);
    if length < 1. {
        return;
    }
    let (dx, dy) = (x / length, y / length);
    let attack = sim.get_entity(caster).map_or(0, |e| e.stat().attack);
    let mut targets = Vec::new();
    for index in 0..sim.entity_count() {
        let Some(e) = sim.entity_at(index) else {
            continue;
        };
        if !enemy(sim, caster, e.id())
            || matches!(e.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus))
        {
            continue;
        }
        let point = e.pos();
        let (tx, ty) = vector(from, point);
        let r = tx.hypot(ty);
        if r <= GRENADE_RANGE && tx * dx + ty * dy >= r * 0.7 {
            targets.push((e.id(), point, e.is_champion()));
        }
    }
    let name = if dx < 0. {
        "mlbb_lesley_grenade_fan_left"
    } else {
        "mlbb_lesley_grenade_fan"
    };
    cue(sim, caster, "grenade");
    ground_fx(sim, caster, from, name);
    for (id, point, champion) in targets {
        sim.deal_damage_typed(
            caster,
            id,
            115 + attack * 65 / 100,
            DamageTypeV1::Ad,
            AttackTypeV1::Skill,
        );
        if champion && sim.get_entity(id).is_some_and(|e| e.is_alive()) {
            sim.entity_knockback(caster, id, 1100, 8);
        }
        ground_fx(sim, caster, point, "mlbb_lesley_grenade_impact");
    }
    if !crate::granger::blocked(sim, caster, true) {
        let next =
            crate::granger::legal_step(&crate::granger::ground(sim), from, (-dx, -dy), 16000.);
        if next != from {
            sim.entity_set_pos(caster, next.0, next.1);
        }
    }
}

#[derive(Clone, Copy)]
enum Op {
    Attack,
    Camouflage,
    Grenade,
    Ult,
    Hit(Shot),
}

impl StableEffectType for Op {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, target: InputTargetV1) {
        if let Self::Hit(shot) = *self {
            if target.kind == InputTargetKindV1::Target.code() {
                damage(sim, caster, target.target_id, shot);
            }
            return;
        }
        if !sim.get_entity(caster).is_some_and(|e| e.is_alive()) {
            return;
        }
        let key = entity_key(sim, caster);
        let mut state = take(key);
        let now = sim.tick();
        match *self {
            Self::Attack => {
                reveal(sim, caster, &mut state);
                clear_lock(sim, &mut state, caster);
                if target.kind == InputTargetKindV1::Target.code()
                    && enemy(sim, caster, target.target_id)
                {
                    let shot = if state.ready {
                        Shot::Lethal
                    } else {
                        Shot::Basic
                    };
                    if fire(sim, caster, target.target_id, shot) && shot == Shot::Lethal {
                        unprime(sim, caster, &mut state, now);
                    }
                }
            }
            Self::Camouflage => {
                clear_lock(sim, &mut state, caster);
                reveal(sim, caster, &mut state);
                prime(sim, caster, &mut state);
                state.camo_until = now + CAMO_DURATION;
                state.camo_grace = now + CAMO_HIT_GRACE;
                sim.add_buff(
                    caster,
                    &BuffV1 {
                        move_speed_mult: 30,
                        ..BuffV1::timed(CAMO, CAMO_DURATION)
                    },
                );
                cue(sim, caster, "camouflage");
                refresh_camouflage(sim, caster, &mut state, now);
                if let Some(p) = position(sim, caster) {
                    ground_fx(sim, caster, p, "mlbb_lesley_camouflage_ripple");
                }
            }
            Self::Grenade => {
                reveal(sim, caster, &mut state);
                if let Some(v) = state.volley.filter(|v| valid_volley(sim, caster, *v)) {
                    if let Some(id) = first_in_line(sim, caster, v.target.entity_id) {
                        fire(sim, caster, id, Shot::Ultimate);
                    }
                }
                clear_lock(sim, &mut state, caster);
                prime(sim, caster, &mut state);
                grenade(sim, caster, target);
            }
            Self::Ult => {
                reveal(sim, caster, &mut state);
                clear_lock(sim, &mut state, caster);
                let id = target.target_id;
                if target.kind == InputTargetKindV1::Target.code()
                    && enemy(sim, caster, id)
                    && sim.get_entity(id).is_some_and(|e| e.is_champion())
                {
                    if let Some(origin) = position(sim, caster) {
                        prime(sim, caster, &mut state);
                        state.volley = Some(Volley {
                            target: entity_key(sim, id),
                            origin,
                            lock_tick: now,
                            fired: 0,
                        });
                        let name = lock_name(caster);
                        sim.add_buff(id, &BuffV1::timed(&name, 94));
                        sim.entity_remove_buff(id, LOCK);
                        sim.add_buff(id, &BuffV1::timed(LOCK, 94));
                    }
                }
            }
            Self::Hit(_) => {}
        }
        put(key, state);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        let ad = stat.attack;
        (
            match self {
                Self::Attack | Self::Hit(Shot::Basic) => ad,
                Self::Hit(Shot::Lethal) => lethal_shot_champion_damage(ad),
                Self::Camouflage => 0,
                Self::Grenade => 115 + ad * 65 / 100,
                Self::Ult => 4 * (70 + ad * 32 / 100),
                Self::Hit(Shot::Ultimate) => 70 + ad * 32 / 100,
            },
            0,
        )
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

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize) {
        let key = entity_key(sim, id);
        if let Some(old) = self.owner.replace(key) {
            take(old);
        }
        put(
            key,
            State {
                ready_at: sim.tick() + 300,
                ..State::default()
            },
        );
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, caster: usize) {
        if self.owner != Some(entity_key_ro(sim, caster)) {
            self.on_spawn(sim, player, caster);
        }
        let Some(key) = self.owner else { return };
        if !sim.get_entity(caster).is_some_and(|e| e.is_alive()) {
            return;
        }
        let mut state = take(key);
        let now = sim.tick();
        if now >= state.ready_at {
            prime(sim, caster, &mut state);
        }
        if state.camo_until != 0 {
            refresh_camouflage(sim, caster, &mut state, now);
        }
        if let Some(mut volley) = state.volley {
            if !valid_volley(sim, caster, volley) {
                clear_lock(sim, &mut state, caster);
            } else if now >= volley.lock_tick + WINDUP + CADENCE * volley.fired {
                if let Some(id) = first_in_line(sim, caster, volley.target.entity_id) {
                    fire(sim, caster, id, Shot::Ultimate);
                }
                volley.fired += 1;
                if volley.fired == 4 {
                    clear_lock(sim, &mut state, caster);
                } else {
                    state.volley = Some(volley);
                }
            }
        }
        put(key, state);
    }

    fn on_damaged(
        &mut self,
        sim: &mut StableSim<'_>,
        _: usize,
        id: usize,
        _: usize,
        damage: usize,
    ) {
        if damage == 0 {
            return;
        }
        let key = entity_key_ro(sim, id);
        let mut state = take(key);
        let now = sim.tick();
        // Damage delays recovery of an empty charge. A skill can still ready
        // a shot during combat, and the next successful basic consumes it.
        state.ready_at = now + 300;
        if now >= state.camo_grace {
            reveal(sim, id, &mut state);
        }
        put(key, state);
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        if let Some(key) = self.owner.take() {
            let mut state = take(key);
            let now = sim.tick();
            clear_lock(sim, &mut state, key.entity_id);
            reveal(sim, key.entity_id, &mut state);
            unprime(sim, key.entity_id, &mut state, now);
        }
    }
}

pub fn register(module: &mut StableMod) {
    for (name, effect) in [
        ("mlbb_heroes:lesley_attack", Op::Attack),
        ("mlbb_heroes:lesley_skill", Op::Camouflage),
        ("mlbb_heroes:lesley_skill2", Op::Grenade),
        ("mlbb_heroes:lesley_ult", Op::Ult),
        (Shot::Basic.receipt(), Op::Hit(Shot::Basic)),
        (Shot::Lethal.receipt(), Op::Hit(Shot::Lethal)),
        (Shot::Ultimate.receipt(), Op::Hit(Shot::Ultimate)),
    ] {
        module.add_native_effect(name, effect);
    }
    module.add_native_passive("mlbb_heroes:lesley", Passive::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::c_void, mem::size_of};

    unsafe extern "C" fn record_invisible(
        host: *mut c_void,
        _: EntityHandleV1,
        ticks: usize,
    ) -> bool {
        (*(host as *mut Vec<usize>)).push(ticks);
        true
    }

    #[test]
    fn lethal_shot_expected_champion_damage_uses_new_attack_ratio() {
        let stats = StatV1 {
            attack: 104,
            ..StatV1::default()
        };
        assert_eq!(Op::Hit(Shot::Lethal).expected_damage(&stats), (189, 0));
    }

    #[test]
    fn camouflage_uses_short_native_timers_and_stops_renewing_on_reveal() {
        let mut durations = Vec::<usize>::new();
        let mut table: SimVtableV1 = unsafe { std::mem::zeroed() };
        table.size = size_of::<SimVtableV1>();
        table.entity_set_invisible = Some(record_invisible);
        let mut context = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            sim: &table,
            frame: std::ptr::null(),
            state: std::ptr::from_mut(&mut durations).cast(),
        };
        let mut sim = unsafe { StableSim::from_raw(&mut context).unwrap() };
        let mut active = State {
            camo_until: 160,
            ..State::default()
        };
        refresh_camouflage(&mut sim, 1, &mut active, 10);
        refresh_camouflage(&mut sim, 1, &mut active, 159);
        refresh_camouflage(&mut sim, 1, &mut active, 160);
        assert_eq!(active.camo_until, 0);

        active.camo_until = 310;
        refresh_camouflage(&mut sim, 1, &mut active, 200);
        reveal(&mut sim, 1, &mut active);
        refresh_camouflage(&mut sim, 1, &mut active, 201);
        assert_eq!(durations, [2, 1, 2]);
    }
}
