use crate::{entity_key, entity_key_ro, sim_copy_of, EntityKey};
use mod_api_stable::*;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

type Pos = (u64, u64);
const RECHARGE: usize = 540;
const RECAST: usize = 108;
const REFUND_GATE: usize = 216;
const SPEED: u64 = 6_000;
const TORRENT_RANGE: f64 = 38_000.;
const TORRENT_PUSH_SPEED: usize = 4_000;
const TORRENT_PUSH_TICKS: usize = 12;
const EMPOWER_WINDOW: usize = 540;
const FRANCO_SUPPRESSION: &str = "mlbb_franco_ult_grip";
const MARKS: [&str; 3] = [
    "mlbb_valir_mark_1",
    "mlbb_valir_mark_2",
    "mlbb_valir_mark_3",
];
const HASTE: &str = "mlbb_valir_haste";
const EMBERS: [&str; 4] = [
    "mlbb_valir_vengeance_1",
    "mlbb_valir_vengeance_2",
    "mlbb_valir_vengeance_3",
    "mlbb_valir_vengeance_4",
];

#[derive(Clone, Copy)]
struct Fireball {
    end: Pos,
    arrival: usize,
    power: usize,
}
struct Blast {
    at: usize,
    center: Pos,
    power: usize,
}
struct Wall {
    at: usize,
    left: usize,
    center: Pos,
    along: (f64, f64),
    power: usize,
}
struct Mark {
    count: usize,
    expires: usize,
    burn: usize,
    stun_at: usize,
}

struct State {
    charges: usize,
    recharge_at: usize,
    recast_at: usize,
    refund_at: usize,
    flames: usize,
    ult_start: usize,
    reactive_ult_until: usize,
    haste: i32,
    last_tick: Option<usize>,
    fireball: Option<Fireball>,
    blasts: Vec<Blast>,
    wall: Option<Wall>,
    marks: HashMap<EntityKey, Mark>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            charges: 2,
            recharge_at: 0,
            recast_at: 0,
            refund_at: 0,
            flames: 0,
            ult_start: 0,
            reactive_ult_until: 0,
            haste: 0,
            last_tick: None,
            fireball: None,
            blasts: Vec::new(),
            wall: None,
            marks: HashMap::new(),
        }
    }
}

static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn take(key: EntityKey) -> Option<State> {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key)
}
fn put(key: EntityKey, state: State) {
    let mut states = STATES.lock().unwrap_or_else(|e| e.into_inner());
    if states.len() > 4096 {
        states.retain(|other, _| other.scope == key.scope);
    }
    states.insert(key, state);
}
impl State {
    fn refill(&mut self, now: usize) {
        while self.charges < 2 && now >= self.recharge_at {
            self.charges += 1;
            self.recharge_at += RECHARGE;
        }
    }
    fn spend(&mut self, now: usize) -> bool {
        self.refill(now);
        if self.charges == 0 || now < self.recast_at {
            return false;
        }
        if self.charges == 2 {
            self.recharge_at = now + RECHARGE;
        }
        self.charges -= 1;
        self.recast_at = now + RECAST;
        true
    }
    fn refund(&mut self, kind: Option<EntityKindV1>, champion: bool, now: usize) {
        if (champion || matches!(kind, Some(EntityKindV1::Epic | EntityKindV1::Serpen)))
            && now >= self.refund_at
        {
            self.charges = (self.charges + 1).min(2);
            self.refund_at = now + REFUND_GATE;
        } else if kind == Some(EntityKindV1::Minion) && self.charges < 2 {
            self.recharge_at = self.recharge_at.saturating_sub(120).max(now);
            self.refill(now);
        }
    }
    fn empower(&mut self, now: usize) -> bool {
        if self.flames == 0 || now >= self.ult_start + EMPOWER_WINDOW {
            return false;
        }
        self.flames -= 1;
        true
    }
    fn remaining(&self, now: usize) -> usize {
        let recharge = if self.charges == 0 {
            self.recharge_at
        } else {
            0
        };
        self.recast_at.max(recharge).saturating_sub(now)
    }
}
fn pos(sim: &StableSim<'_>, id: usize) -> Option<Pos> {
    sim.get_entity(id).map(|e| e.pos())
}
fn v(a: Pos, b: Pos) -> (f64, f64) {
    (b.0 as f64 - a.0 as f64, b.1 as f64 - a.1 as f64)
}
fn shift(p: Pos, x: f64, y: f64) -> Pos {
    (
        (p.0 as f64 + x).max(0.) as u64,
        (p.1 as f64 + y).max(0.) as u64,
    )
}
fn cast_direction(
    from: Pos,
    input: InputTargetV1,
    target_pos: impl FnOnce(usize) -> Option<Pos>,
) -> Option<(f64, f64)> {
    let (x, y) = match InputTargetKindV1::from_code(input.kind)? {
        InputTargetKindV1::Target => v(from, target_pos(input.target_id)?),
        InputTargetKindV1::Pos => v(from, (input.x, input.y)),
        InputTargetKindV1::Dir => (input.dir_x as f64, input.dir_y as f64),
        InputTargetKindV1::None => return None,
    };
    let len = x.hypot(y);
    (len > 0.).then(|| (x / len, y / len))
}
fn angle((x, y): (f64, f64)) -> usize {
    ((y.atan2(x).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * 8.).round() as usize)
        % 8
}
fn in_torrent(from: Pos, to: Pos, direction: (f64, f64)) -> bool {
    let (x, y) = v(from, to);
    let distance = x.hypot(y);
    distance <= TORRENT_RANGE && x * direction.0 + y * direction.1 >= distance * 0.887
}
fn fireball_damage(power: usize) -> usize {
    70 + power * 65 / 100
}
fn torrent_damage(power: usize) -> usize {
    95 + power * 65 / 100
}
fn torrent_pushes(entity: &StableEntity<'_, '_>) -> bool {
    entity.is_alive()
        && (entity.is_champion()
            || matches!(
                entity.kind(),
                Some(EntityKindV1::Minion | EntityKindV1::Jungle)
            ))
}
fn visible(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
}
fn fx(sim: &mut StableSim<'_>, id: usize, p: Pos, name: &str, time: u64) {
    if visible(sim) {
        sim.play_view_effect(
            &format!("mlbb_valir_{name}"),
            id,
            &InputTargetV1::pos(p.0, p.1),
            0,
            0,
            time,
        );
    }
}
fn cue(sim: &mut StableSim<'_>, id: usize, name: &str) {
    if visible(sim) {
        sim.play_sfx(
            &format!("mlbb_valir_{name}"),
            id,
            &InputTargetV1::target(id),
        );
    }
}
fn enemy(sim: &StableSim<'_>, c: usize, t: usize, structures: bool) -> bool {
    sim.get_entity(c)
        .zip(sim.get_entity(t))
        .is_some_and(|(a, b)| {
            a.team() != b.team()
                && b.is_alive()
                && b.is_targetable()
                && (structures
                    || !matches!(b.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus)))
        })
}
fn targets(sim: &StableSim<'_>, id: usize, select: impl Fn(Pos) -> bool) -> Vec<(usize, Pos)> {
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| enemy(sim, id, e.id(), false) && select(e.pos()))
        .map(|e| (e.id(), e.pos()))
        .collect()
}
fn damage(sim: &mut StableSim<'_>, id: usize, target: usize, amount: usize) {
    sim.deal_damage_typed(id, target, amount, DamageTypeV1::Ap, AttackTypeV1::Skill);
}
fn slow(sim: &mut StableSim<'_>, target: usize, name: &str, amount: i32, ticks: usize) {
    sim.entity_remove_buff(target, name);
    sim.add_buff(
        target,
        &BuffV1 {
            move_speed_mult: amount,
            ..BuffV1::timed(name, ticks)
        },
    );
}
fn set_embers(sim: &mut StableSim<'_>, id: usize, before: usize, state: &State) {
    if let Some(name) = before.checked_sub(1).and_then(|i| EMBERS.get(i)) {
        sim.entity_remove_buff(id, name);
    }
    if let Some(name) = state.flames.checked_sub(1).and_then(|i| EMBERS.get(i)) {
        let remaining = (state.ult_start + EMPOWER_WINDOW).saturating_sub(sim.tick());
        if remaining > 0 {
            sim.add_buff(id, &BuffV1::timed(name, remaining));
        }
    }
}
fn suppressed(entity: &StableEntity<'_, '_>) -> bool {
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .any(|buff| buff.name() == FRANCO_SUPPRESSION)
}

fn triggers_vengeance(cc: CcV1) -> bool {
    cc.tick > 0
        && matches!(
            CcKindV1::from_code(cc.kind),
            Some(
                CcKindV1::Stun
                    | CcKindV1::Airborne
                    | CcKindV1::Bind
                    | CcKindV1::BlockSkill
                    | CcKindV1::Fear
                    | CcKindV1::Charm
                    | CcKindV1::Taunt
            )
        )
}

fn ignite_vengeance(sim: &mut StableSim<'_>, id: usize, state: &mut State, now: usize) {
    let before = state.flames;
    sim.entity_clear_cc(id);
    state.flames = 4;
    state.ult_start = now;
    state.haste = 30;
    slow(sim, id, HASTE, 30, 60);
    set_embers(sim, id, before, state);
    if let Some(point) = pos(sim, id) {
        fx(sim, id, point, "cleanse_flash", 0);
    }
    cue(sim, id, "ult");
}

fn update_vengeance(sim: &mut StableSim<'_>, id: usize, state: &mut State, now: usize) {
    let Some(entity) = sim.get_entity(id) else {
        return;
    };
    let Some(ult) = entity.cooldown(SkillSlotV1::Ult) else {
        return;
    };
    let remaining = state.reactive_ult_until.saturating_sub(now);
    if remaining > 0 {
        // The stable API cannot start a cooldown. If the stock AI starts the
        // action after a reactive cast, match its clock to the original cast.
        if ult.remaining > remaining {
            sim.entity_reduce_cooldown(id, SkillSlotV1::Ult, ult.remaining - remaining);
        }
        return;
    }
    if ult.unlocked
        && ult.charges > 0
        && ult.remaining == 0
        && ult.cooltime > 0
        && entity.is_targetable()
        && (0..entity.cc_count())
            .filter_map(|i| entity.cc_at(i))
            .any(triggers_vengeance)
        && !suppressed(&entity)
    {
        state.reactive_ult_until = now + ult.cooltime;
        ignite_vengeance(sim, id, state, now);
    }
}
fn marker_name(caster: usize, count: usize) -> String {
    format!("mlbb_valir_owner_{caster}_{count}")
}
fn paint(sim: &mut StableSim<'_>, id: usize, target: usize, count: usize) {
    for (i, name) in MARKS.iter().enumerate() {
        sim.entity_remove_buff(target, &marker_name(id, i + 1));
        sim.entity_remove_buff(target, name);
    }
    if count > 0 {
        sim.add_buff(target, &BuffV1::timed(&marker_name(id, count), 180));
    }
    let display = sim.get_entity(target).and_then(|e| {
        (0..e.buff_count())
            .filter_map(|i| e.buff_at(i))
            .filter_map(|buff| {
                if !buff.name().starts_with("mlbb_valir_owner_") {
                    return None;
                }
                let count: usize = buff.name().rsplit('_').next()?.parse().ok()?;
                (count > 0 && count <= MARKS.len()).then_some((count, buff.duration_tick))
            })
            .max()
    });
    if let Some((count, remaining)) = display {
        sim.add_buff(target, &BuffV1::timed(MARKS[count - 1], remaining));
    }
}
fn mark(sim: &mut StableSim<'_>, id: usize, target: usize, power: usize, s: &mut State) {
    let Some(e) = sim.get_entity(target).filter(|e| e.is_alive()) else {
        return;
    };
    let champion = e.is_champion();
    let boss = matches!(e.kind(), Some(EntityKindV1::Epic | EntityKindV1::Serpen));
    if !champion && !boss && e.kind() != Some(EntityKindV1::Jungle) {
        return;
    }
    let (point, max_hp) = (e.pos(), e.hp().1);
    let key = entity_key(sim, target);
    let now = sim.tick();
    let m = s.marks.entry(key).or_insert(Mark {
        count: 0,
        expires: 0,
        burn: 0,
        stun_at: 0,
    });
    if m.expires <= now {
        m.count = 0;
        m.burn = now + 60;
    }
    m.expires = now + 180;
    m.count += 1;
    if m.count == 3 {
        m.count = 0;
        let percent = max_hp * 4 / 100;
        damage(
            sim,
            id,
            target,
            if champion { percent } else { percent.min(100) } + power / 4,
        );
        if !boss && now >= m.stun_at {
            sim.apply_cc(target, &CcV1::stun(39));
            m.stun_at = now + 480;
        }
        fx(sim, id, point, "ablaze_detonation", 0);
    }
    paint(sim, id, target, m.count);
}
fn impact(sim: &mut StableSim<'_>, id: usize, center: Pos, power: usize, s: &mut State) {
    fx(sim, id, center, "fireball_impact", 0);
    fx(sim, id, center, "fireball_waiting_patch", 0);
    s.blasts.push(Blast {
        at: sim.tick() + 45,
        center,
        power,
    });
}

#[derive(Clone, Copy)]
enum Op {
    Attack,
    BasicHit,
    Fireball,
    FireballHit,
    Torrent,
    Ult,
}
impl StableEffectType for Op {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, i: InputTargetV1) {
        let Some(c) = sim.get_entity(id).filter(|e| e.is_alive()) else {
            return;
        };
        let (from, team, stat) = (c.pos(), c.team(), c.stat());
        let now = sim.tick();
        let target = i.target_id;
        if matches!(self, Self::BasicHit) {
            if enemy(sim, id, target, true) {
                sim.deal_damage_typed(
                    id,
                    target,
                    stat.attack,
                    DamageTypeV1::Ad,
                    AttackTypeV1::BaseAttack,
                );
            }
            return;
        }
        if matches!(self, Self::Attack) {
            if !enemy(sim, id, target, true) {
                return;
            }
            let Some(end) = pos(sim, target) else { return };
            sim.spawn_projectile(
                "mlbb_valir_basic_ember",
                "mlbb_heroes:valir_basic_hit",
                &ProjectileSpawnV1 {
                    caster_id: id,
                    team,
                    x: from.0,
                    y: from.1,
                    radius: 1600,
                    speed: SPEED,
                    target_id: target,
                    target_x: end.0,
                    target_y: end.1,
                    attack_type: AttackTypeV1::BaseAttack.code(),
                    ..ProjectileSpawnV1::default()
                },
            );
            cue(sim, id, "basic");
            return;
        }
        let key = entity_key(sim, id);
        let mut s = take(key).unwrap_or_default();
        let before = s.flames;
        match self {
            Self::Fireball => {
                if let Some((dx, dy)) = cast_direction(from, i, |target| pos(sim, target)) {
                    if s.spend(now) {
                        let empowered = s.empower(now);
                        let reach: u64 = if empowered { 106_600 } else { 82_000 };
                        let end = shift(from, dx * reach as f64, dy * reach as f64);
                        let projectile = if empowered {
                            "mlbb_valir_fireball_empowered"
                        } else {
                            "mlbb_valir_fireball_projectile"
                        };
                        let spec = ProjectileSpawnV1 {
                            caster_id: id,
                            team,
                            x: from.0,
                            y: from.1,
                            radius: 3200,
                            speed: SPEED,
                            move_kind: ProjectileMoveKindV1::Linear.code(),
                            target_id: 0,
                            target_x: end.0,
                            target_y: end.1,
                            penetrate: false,
                            attack_type: AttackTypeV1::Skill.code(),
                            casting_type: CastingTypeV1::Targeting.code(),
                            casting_target: CastingTargetV1::EnemyWithoutTower.code(),
                        };
                        let born = sim.spawn_projectile(
                            projectile,
                            "mlbb_heroes:valir_fireball_hit",
                            &spec,
                        );
                        if born {
                            s.fireball = Some(Fireball {
                                end,
                                arrival: now + reach.div_ceil(SPEED) as usize + 2,
                                power: stat.magic_power,
                            });
                            cue(sim, id, "fireball");
                        } else {
                            s.charges = (s.charges + 1).min(2);
                            if empowered {
                                s.flames += 1;
                            }
                        }
                    }
                }
            }
            Self::FireballHit => {
                if i.kind == InputTargetKindV1::Target.code() && enemy(sim, id, target, false) {
                    if let Some(flight) = s.fireball.take() {
                        let e = sim.get_entity(target).unwrap();
                        let (center, kind, champion) = (e.pos(), e.kind(), e.is_champion());
                        s.refund(kind, champion, now);
                        damage(sim, id, target, fireball_damage(flight.power));
                        slow(sim, target, "mlbb_valir_fireball_slow", -25, 42);
                        mark(sim, id, target, flight.power, &mut s);
                        impact(sim, id, center, flight.power, &mut s);
                    }
                }
            }
            Self::Torrent => {
                if let Some(d) = cast_direction(from, i, |target| pos(sim, target)) {
                    let enhanced = s.empower(now);
                    let bucket = angle(d);
                    let name = if enhanced {
                        "torrent_empowered"
                    } else {
                        "torrent_wave"
                    };
                    fx(sim, id, from, &format!("{name}_{bucket}"), 0);
                    cue(sim, id, "torrent");
                    for (target, _) in targets(sim, id, |p| in_torrent(from, p, d)) {
                        damage(sim, id, target, torrent_damage(stat.magic_power));
                        if sim.get_entity(target).as_ref().is_some_and(torrent_pushes) {
                            sim.entity_knockback(
                                id,
                                target,
                                TORRENT_PUSH_SPEED,
                                TORRENT_PUSH_TICKS,
                            );
                        }
                        mark(sim, id, target, stat.magic_power, &mut s);
                    }
                    if enhanced {
                        let center = shift(from, d.0 * TORRENT_RANGE, d.1 * TORRENT_RANGE);
                        fx(sim, id, center, &format!("torrent_firewall_{bucket}"), 0);
                        s.wall = Some(Wall {
                            at: now + 60,
                            left: 3,
                            center,
                            along: d,
                            power: stat.magic_power,
                        });
                    }
                }
            }
            Self::Ult => {
                if now >= s.reactive_ult_until
                    && sim
                        .get_entity(id)
                        .is_some_and(|entity| !suppressed(&entity))
                {
                    ignite_vengeance(sim, id, &mut s, now);
                }
            }
            _ => {}
        }
        if s.flames != before && !matches!(self, Self::Ult) {
            set_embers(sim, id, before, &s);
        }
        put(key, s);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        match self {
            Self::Attack | Self::BasicHit => (stat.attack, 0),
            Self::Fireball | Self::FireballHit => (0, fireball_damage(stat.magic_power)),
            Self::Torrent => (0, torrent_damage(stat.magic_power)),
            Self::Ult => (0, 0),
        }
    }
}

fn update(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    let now = sim.tick();
    if s.last_tick == Some(now) {
        return;
    }
    s.last_tick = Some(now);
    update_vengeance(sim, id, s, now);
    s.refill(now);
    let remaining = s.remaining(now);
    if let Some(info) = sim
        .get_entity(id)
        .and_then(|e| e.cooldown(SkillSlotV1::Skill))
    {
        if info.unlocked && info.remaining > remaining {
            sim.entity_reduce_cooldown(id, SkillSlotV1::Skill, info.remaining - remaining);
        }
    }
    if s.flames > 0 && now >= s.ult_start + EMPOWER_WINDOW {
        let before = s.flames;
        s.flames = 0;
        set_embers(sim, id, before, s);
    }
    let haste = match now.saturating_sub(s.ult_start) {
        0..=59 if s.haste != 0 => 30,
        60..=119 if s.haste != 0 => 20,
        120..=179 if s.haste != 0 => 10,
        _ => 0,
    };
    if haste != s.haste {
        s.haste = haste;
        if haste == 0 {
            sim.entity_remove_buff(id, HASTE);
        } else {
            slow(sim, id, HASTE, haste, 60);
        }
    }
    if s.fireball.is_some_and(|f| now >= f.arrival) {
        let f = s.fireball.take().unwrap();
        impact(sim, id, f.end, f.power, s);
    }
    let pending = std::mem::take(&mut s.blasts);
    for b in pending {
        if now < b.at {
            s.blasts.push(b);
            continue;
        }
        fx(sim, id, b.center, "fireball_delayed_blast", 0);
        cue(sim, id, "blast");
        for (target, _) in targets(sim, id, |p| {
            let (x, y) = v(b.center, p);
            x.hypot(y) <= 14_000.
        }) {
            damage(sim, id, target, 42 + b.power * 30 / 100);
            mark(sim, id, target, b.power, s);
        }
    }
    if let Some(mut wall) = s.wall.take() {
        if now >= wall.at {
            for (target, _) in targets(sim, id, |p| {
                let (x, y) = v(wall.center, p);
                let (dx, dy) = wall.along;
                (x * dx + y * dy).abs() <= 4500. && (x * dy - y * dx).abs() <= 17_000.
            }) {
                damage(sim, id, target, 27 + wall.power * 22 / 100);
                slow(sim, target, "mlbb_valir_wall_slow", -20, 24);
            }
            wall.at += 60;
            wall.left -= 1;
        }
        if wall.left > 0 {
            s.wall = Some(wall);
        }
    }
    let mut keys: Vec<_> = s.marks.keys().copied().collect();
    keys.sort_by_key(|k| k.entity_id);
    for key in keys {
        let t = key.entity_id;
        let Some(m) = s.marks.get_mut(&key) else {
            continue;
        };
        if sim_copy_of(sim, t) != Some(key.copy) || !enemy(sim, id, t, false) {
            s.marks.remove(&key);
            continue;
        }
        if now <= m.expires && now >= m.burn {
            let e = sim.get_entity(t).unwrap();
            let (p, max_hp, hero) = (e.pos(), e.hp().1, e.is_champion());
            let n = max_hp * 8 / 1000;
            damage(sim, id, t, if hero { n } else { n.min(24) });
            fx(sim, id, p, "ablaze_burn", 0);
            m.burn += 60;
        }
        if now >= m.expires && now >= m.stun_at {
            s.marks.remove(&key);
        }
    }
}

#[derive(Default)]
struct Passive {
    owner: Option<EntityKey>,
    reactive_ult_until: usize,
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
        for name in EMBERS {
            sim.entity_remove_buff(id, name);
        }
        put(
            key,
            State {
                reactive_ult_until: self.reactive_ult_until,
                ..State::default()
            },
        );
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, id: usize) {
        if !sim.get_entity(id).is_some_and(|e| e.is_alive()) {
            return;
        }
        if self.owner != Some(entity_key_ro(sim, id)) {
            self.on_spawn(sim, player, id);
        }
        let Some(key) = self.owner else { return };
        if let Some(mut s) = take(key) {
            update(sim, id, &mut s);
            self.reactive_ult_until = s.reactive_ult_until;
            put(key, s);
        }
    }
    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        if let Some(key) = self.owner.take() {
            if let Some(s) = take(key) {
                for target in s.marks.keys() {
                    if sim_copy_of(sim, target.entity_id) == Some(target.copy) {
                        paint(sim, key.entity_id, target.entity_id, 0);
                    }
                }
            }
            sim.entity_remove_buff(key.entity_id, HASTE);
            for name in EMBERS {
                sim.entity_remove_buff(key.entity_id, name);
            }
        }
    }
}
pub fn register(module: &mut StableMod) {
    for (name, op) in [
        ("attack", Op::Attack),
        ("basic_hit", Op::BasicHit),
        ("skill", Op::Fireball),
        ("fireball_hit", Op::FireballHit),
        ("skill2", Op::Torrent),
        ("ult", Op::Ult),
    ] {
        module.add_native_effect(&format!("mlbb_heroes:valir_{name}"), op);
    }
    module.add_native_passive("mlbb_heroes:valir", Passive::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::c_void, mem::size_of, ptr};

    #[test]
    fn directional_casts_decode_all_eight_bearings() {
        let origin = (80_000, 80_000);
        for (bucket, (dx, dy)) in [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ]
        .into_iter()
        .enumerate()
        {
            let direction = cast_direction(origin, InputTargetV1::dir(dx, dy), |_| None).unwrap();
            assert_eq!(angle(direction), bucket);
            let forward = shift(origin, direction.0 * 30_000., direction.1 * 30_000.);
            let behind = shift(origin, -direction.0 * 30_000., -direction.1 * 30_000.);
            assert!(in_torrent(origin, forward, direction));
            assert!(!in_torrent(origin, behind, direction));
        }
        let enemy = (80_000, 70_000);
        let expected = Some((0., -1.));
        assert_eq!(
            cast_direction(origin, InputTargetV1::target(4), |id| {
                (id == 4).then_some(enemy)
            }),
            expected
        );
        assert_eq!(
            cast_direction(origin, InputTargetV1::pos(enemy.0, enemy.1), |_| None),
            expected
        );
        assert_eq!(
            cast_direction(origin, InputTargetV1::dir(0, 0), |_| None),
            None
        );
        assert_eq!(cast_direction(origin, InputTargetV1::NONE, |_| None), None);
    }

    #[derive(Default)]
    struct CastCalls {
        projectile: Option<Pos>,
        hits: Vec<(usize, usize)>,
        push: Vec<(usize, usize, usize, usize)>,
    }

    const CAST_ENTITIES: [(EntityKindV1, Pos); 9] = [
        (EntityKindV1::Champion, (80_000, 80_000)),
        (EntityKindV1::Champion, (100_000, 60_000)),
        (EntityKindV1::Minion, (96_000, 62_000)),
        (EntityKindV1::Jungle, (94_000, 66_000)),
        (EntityKindV1::Epic, (93_000, 67_000)),
        (EntityKindV1::Serpen, (92_000, 68_000)),
        (EntityKindV1::Tower, (95_000, 65_000)),
        (EntityKindV1::Nexus, (95_000, 65_000)),
        (EntityKindV1::Champion, (60_000, 100_000)),
    ];

    #[test]
    fn direction_actions_launch_forward_and_request_knockback_on_the_victim() {
        let mut calls = CastCalls::default();
        let mut table: SimVtableV1 = unsafe { std::mem::zeroed() };
        table.size = size_of::<SimVtableV1>();
        table.entity_count = Some(count);
        table.entity_at = Some(at);
        table.entity_is_valid = Some(valid);
        table.entity_is_alive = Some(valid);
        table.entity_is_targetable = Some(valid);
        table.entity_is_champion = Some(is_champion);
        table.entity_kind = Some(entity_kind);
        table.entity_stat = Some(stat);
        table.entity_team = Some(team);
        table.entity_pos = Some(position);
        table.spawn_projectile = Some(projectile);
        table.deal_damage_typed = Some(hit);
        table.entity_knockback = Some(push);
        let mut context = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            sim: &table,
            frame: ptr::null(),
            state: ptr::from_mut(&mut calls).cast(),
        };
        let mut sim = unsafe { StableSim::from_raw(&mut context).unwrap() };
        let northeast = InputTargetV1::dir(20_000, -20_000);
        Op::Fireball.apply(&mut sim, 0, 0, northeast);
        Op::Torrent.apply(&mut sim, 0, 0, northeast);
        assert_eq!(calls.projectile, Some((137_982, 22_017)));
        assert_eq!(
            calls.hits,
            [(1, 117), (2, 117), (3, 117), (4, 117), (5, 117)]
        );
        assert_eq!(
            calls.push,
            [(0, 1, 4000, 12), (0, 2, 4000, 12), (0, 3, 4000, 12)]
        );
        assert_eq!(
            Op::Fireball.expected_damage(&StatV1 {
                magic_power: 35,
                ..StatV1::default()
            }),
            (0, 92)
        );
    }

    unsafe extern "C" fn count(_: *const c_void) -> usize {
        CAST_ENTITIES.len()
    }
    unsafe extern "C" fn at(_: *const c_void, i: usize) -> EntityHandleV1 {
        EntityHandleV1::from_id(i)
    }
    unsafe extern "C" fn valid(_: *const c_void, h: EntityHandleV1) -> bool {
        h.id().is_some_and(|i| i < CAST_ENTITIES.len())
    }
    unsafe extern "C" fn is_champion(_: *const c_void, h: EntityHandleV1) -> bool {
        CAST_ENTITIES[h.id().unwrap()].0 == EntityKindV1::Champion
    }
    unsafe extern "C" fn entity_kind(_: *const c_void, h: EntityHandleV1) -> u32 {
        CAST_ENTITIES[h.id().unwrap()].0.code()
    }
    unsafe extern "C" fn stat(_: *const c_void, _: EntityHandleV1, out: *mut StatV1) -> bool {
        *out = StatV1 {
            magic_power: 35,
            ..StatV1::default()
        };
        true
    }
    unsafe extern "C" fn team(_: *const c_void, h: EntityHandleV1) -> usize {
        usize::from(h.0 != 1)
    }
    unsafe extern "C" fn position(
        _: *const c_void,
        h: EntityHandleV1,
        x: *mut u64,
        y: *mut u64,
    ) -> bool {
        (*x, *y) = CAST_ENTITIES[h.id().unwrap()].1;
        true
    }
    unsafe extern "C" fn projectile(
        state: *mut c_void,
        _: *const u8,
        _: usize,
        _: *const u8,
        _: usize,
        spec: *const ProjectileSpawnV1,
    ) -> bool {
        (*(state as *mut CastCalls)).projectile = Some(((*spec).target_x, (*spec).target_y));
        true
    }
    unsafe extern "C" fn hit(
        state: *mut c_void,
        _: usize,
        target: usize,
        amount: usize,
        _: u32,
        _: u32,
    ) -> bool {
        (*(state as *mut CastCalls)).hits.push((target, amount));
        true
    }
    unsafe extern "C" fn push(
        state: *mut c_void,
        caster: usize,
        target: EntityHandleV1,
        speed: usize,
        ticks: usize,
    ) -> bool {
        (*(state as *mut CastCalls))
            .push
            .push((caster, target.id().unwrap(), speed, ticks));
        true
    }

    struct VengeanceCalls {
        now: usize,
        ult: CooldownV1,
        cc: Option<CcV1>,
        suppression: bool,
        cleanses: usize,
    }

    #[test]
    fn ready_vengeance_breaks_stun_once_and_synchronizes_the_next_native_cooldown() {
        let mut calls = VengeanceCalls {
            now: 100,
            ult: CooldownV1 {
                unlocked: true,
                charges: 1,
                cooltime: 2_520,
                ..CooldownV1::default()
            },
            cc: Some(CcV1::stun(60)),
            suppression: true,
            cleanses: 0,
        };
        let mut table: SimVtableV1 = unsafe { std::mem::zeroed() };
        table.size = size_of::<SimVtableV1>();
        table.tick = Some(vengeance_tick);
        table.entity_is_valid = Some(valid);
        table.entity_is_targetable = Some(valid);
        table.entity_cooldown = Some(vengeance_cooldown);
        table.entity_cc_count = Some(vengeance_cc_count);
        table.entity_cc_at = Some(vengeance_cc_at);
        table.entity_buff_count = Some(vengeance_buff_count);
        table.entity_buff_at = Some(vengeance_buff_at);
        table.entity_clear_cc = Some(vengeance_clear);
        table.entity_reduce_cooldown = Some(vengeance_reduce);
        let mut context = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            sim: &table,
            frame: ptr::null(),
            state: ptr::from_mut(&mut calls).cast(),
        };
        let mut sim = unsafe { StableSim::from_raw(&mut context).unwrap() };
        let mut state = State::default();

        update_vengeance(&mut sim, 0, &mut state, calls.now);
        assert_eq!(calls.cleanses, 0);
        calls.suppression = false;
        update_vengeance(&mut sim, 0, &mut state, calls.now);
        assert_eq!(calls.cleanses, 1);
        assert!(calls.cc.is_none());
        assert_eq!(state.flames, 4);
        assert_eq!(state.reactive_ult_until, 2_620);

        calls.now = 250;
        calls.cc = Some(CcV1::stun(30));
        update_vengeance(&mut sim, 0, &mut state, calls.now);
        assert_eq!(calls.cleanses, 1);
        calls.ult.remaining = 2_520;
        update_vengeance(&mut sim, 0, &mut state, calls.now);
        assert_eq!(calls.ult.remaining, 2_370);
        assert_eq!(state.ult_start, 100);
    }

    unsafe extern "C" fn vengeance_tick(state: *const c_void) -> usize {
        (*(state as *const VengeanceCalls)).now
    }
    unsafe extern "C" fn vengeance_cooldown(
        state: *const c_void,
        _: EntityHandleV1,
        slot: u32,
        out: *mut CooldownV1,
    ) -> bool {
        if slot != SkillSlotV1::Ult.code() {
            return false;
        }
        *out = (*(state as *const VengeanceCalls)).ult;
        true
    }
    unsafe extern "C" fn vengeance_cc_count(state: *const c_void, _: EntityHandleV1) -> usize {
        usize::from((*(state as *const VengeanceCalls)).cc.is_some())
    }
    unsafe extern "C" fn vengeance_cc_at(
        state: *const c_void,
        _: EntityHandleV1,
        _: usize,
        out: *mut CcV1,
    ) -> bool {
        *out = (*(state as *const VengeanceCalls)).cc.unwrap();
        true
    }
    unsafe extern "C" fn vengeance_buff_count(state: *const c_void, _: EntityHandleV1) -> usize {
        usize::from((*(state as *const VengeanceCalls)).suppression)
    }
    unsafe extern "C" fn vengeance_buff_at(
        _: *const c_void,
        _: EntityHandleV1,
        _: usize,
        out: *mut BuffV1,
    ) -> bool {
        *out = BuffV1::named(FRANCO_SUPPRESSION);
        true
    }
    unsafe extern "C" fn vengeance_clear(state: *mut c_void, _: EntityHandleV1) -> usize {
        let calls = &mut *(state as *mut VengeanceCalls);
        calls.cleanses += 1;
        usize::from(calls.cc.take().is_some())
    }
    unsafe extern "C" fn vengeance_reduce(
        state: *mut c_void,
        _: EntityHandleV1,
        _: u32,
        ticks: usize,
    ) -> bool {
        (*(state as *mut VengeanceCalls)).ult.remaining -= ticks;
        true
    }

    #[test]
    fn recharge_and_refund_gate_are_independent_of_recast() {
        let mut s = State::default();
        assert!(s.spend(10));
        s.refund(Some(EntityKindV1::Champion), true, 20);
        assert_eq!(s.charges, 2);
        assert!(!s.spend(117));
        assert!(s.spend(118));
        s.refund(Some(EntityKindV1::Champion), true, 130);
        assert_eq!(s.charges, 1);
        assert!(s.spend(226));
        assert_eq!(s.remaining(226), 432);
        s.refund(Some(EntityKindV1::Minion), false, 230);
        assert_eq!(s.remaining(230), 308);
        s.refill(538);
        assert_eq!(s.charges, 1);
    }
    #[test]
    fn four_empowered_casts_and_window_expiration() {
        let mut s = State {
            flames: 4,
            ult_start: 100,
            ..State::default()
        };
        for _ in 0..4 {
            assert!(s.empower(101));
        }
        assert!(!s.empower(102));
        s.flames = 4;
        assert!(!s.empower(640));
    }
}
