//! ABI callback tests. A mock host is not a live TFM2 renderer/pathfinder.
use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};
static SEED: AtomicU64 = AtomicU64::new(9_000_000);
#[derive(Clone)]
struct Entity {
    team: usize,
    kind: EntityKindV1,
    p: Pos,
    hp: usize,
    stat: StatV1,
    buffs: Vec<BuffV1>,
    cc: Vec<CcV1>,
    shield: usize,
    targetable: bool,
}
#[derive(Clone)]
struct Projectile {
    effect: String,
    spec: ProjectileSpawnV1,
    p: (f64, f64),
    ended: bool,
}
struct Host {
    now: usize,
    seed: u64,
    next: usize,
    entities: HashMap<usize, Entity>,
    damage: Vec<(usize, usize, usize)>,
    moves: Vec<(usize, Pos)>,
    spawned: Vec<(usize, UnitAttackV1)>,
    hidden: usize,
    banished: usize,
    owner: usize,
    projectiles: Vec<Projectile>,
    projectile_ok: bool,
    damage_types: Vec<(usize, usize, u32, u32)>,
    origin_kind: u32,
    buff_reads: std::cell::Cell<usize>,
    buff_writes: usize,
    enum_reads: std::cell::Cell<usize>,
    view_calls: Vec<(String, u64)>,
    unseen: std::collections::HashSet<(usize, usize)>,
    levels: HashMap<usize, usize>,
    stat_writes: usize,
}
impl Host {
    fn new() -> Self {
        let mut h = Self {
            now: 0,
            seed: SEED.fetch_add(1, Ordering::Relaxed),
            next: 100,
            entities: HashMap::new(),
            damage: vec![],
            moves: vec![],
            spawned: vec![],
            hidden: 0,
            banished: 0,
            owner: 1,
            projectiles: vec![],
            projectile_ok: true,
            damage_types: vec![],
            origin_kind: SimOriginKindV1::ClientMatchView.code(),
            buff_reads: std::cell::Cell::new(0),
            buff_writes: 0,
            enum_reads: std::cell::Cell::new(0),
            view_calls: vec![],
            unseen: std::collections::HashSet::new(),
            levels: HashMap::new(),
            stat_writes: 0,
        };
        h.add(1, 0, (200_000, 200_000));
        h.add(2, 1, (250_000, 200_000));
        h
    }
    fn add(&mut self, id: usize, team: usize, p: Pos) {
        self.entities.insert(
            id,
            Entity {
                team,
                kind: EntityKindV1::Champion,
                p,
                hp: 660,
                stat: StatV1 {
                    attack: 82,
                    hp: 660,
                    defence: 20,
                    magic_resistance: 20,
                    move_speed: 1080,
                    ..Default::default()
                },
                buffs: vec![],
                cc: vec![],
                shield: 0,
                targetable: true,
            },
        );
    }
    fn table() -> SimVtableV1 {
        let mut t: SimVtableV1 = unsafe { std::mem::zeroed() };
        t.size = size_of::<SimVtableV1>();
        t.tick = Some(tick);
        t.seed = Some(seed);
        t.sim_origin = Some(origin);
        t.entity_count = Some(count);
        t.entity_at = Some(at);
        t.entity_is_valid = Some(valid);
        t.entity_pos = Some(position);
        t.entity_stat = Some(stat);
        t.entity_hp = Some(hp);
        t.entity_level = Some(level);
        t.entity_set_base_stat = Some(set_base_stat);
        t.entity_attack_speed_mult = Some(attack_speed);
        t.entity_team = Some(team);
        t.entity_is_alive = Some(alive_);
        t.entity_is_targetable = Some(targetable);
        t.entity_is_champion = Some(champion);
        t.entity_kind = Some(kind);
        t.entity_buff_count = Some(buff_count);
        t.entity_buff_at = Some(buff_at);
        t.add_buff = Some(add_buff);
        t.entity_remove_buff = Some(remove_buff);
        t.entity_cc_count = Some(cc_count);
        t.entity_cc_at = Some(cc_at);
        t.apply_cc = Some(add_cc);
        t.spawn_unit = Some(spawn_unit);
        t.spawn_projectile = Some(spawn_projectile);
        t.play_view_effect = Some(play_view);
        t.is_visible = Some(seen);
        t.entity_set_pos = Some(set_pos);
        t.entity_set_hp = Some(set_hp);
        t.deal_damage_typed = Some(damage);
        t.heal = Some(heal);
        t.entity_add_shield = Some(shield);
        t.entity_shield = Some(read_shield);
        t.entity_set_invisible = Some(hide);
        t.entity_banish = Some(banish);
        t.entity_cooldown = Some(cooldown);
        t.player_is_valid = Some(player_valid);
        t.player_champion = Some(player_champion);
        t
    }
    fn sim<T>(&mut self, f: impl FnOnce(&mut StableSim<'_>) -> T) -> T {
        let t = Self::table();
        let mut raw = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            sim: &t,
            frame: std::ptr::null(),
            state: self as *mut _ as *mut c_void,
        };
        f(&mut unsafe { StableSim::from_raw(&mut raw).unwrap() })
    }
    fn start(&mut self, id: usize) -> Passive {
        let mut p = Passive::default();
        self.sim(|s| p.on_spawn(s, id, id));
        p
    }
    fn key(&mut self, id: usize) -> EntityKey {
        self.sim(|s| crate::entity_key(s, id))
    }
    fn state(&mut self, id: usize) -> State {
        let k = self.key(id);
        STATES.lock().unwrap().get(&k).unwrap().clone()
    }
    fn pet(&mut self, id: usize) -> usize {
        self.state(id).kupa.unwrap().entity_id
    }
    fn cast(&mut self, id: usize, op: Op, target: usize) {
        self.sim(|s| op.apply(s, 0, id, InputTargetV1::target(target)));
    }
    fn step(&mut self, n: usize) {
        for _ in 0..n {
            self.now += 1;
            for e in self.entities.values_mut() {
                for b in &mut e.buffs {
                    if b.duration_kind == BuffDurationV1::Time.code() {
                        b.duration_tick = b.duration_tick.saturating_sub(1);
                    }
                }
                e.buffs.retain(|b| {
                    b.duration_kind != BuffDurationV1::Time.code() || b.duration_tick > 0
                });
                for c in &mut e.cc {
                    c.tick = c.tick.saturating_sub(1);
                }
                e.cc.retain(|c| c.tick > 0);
            }
            self.sim(tick_all);
            let mut hits = Vec::new();
            for shot in &mut self.projectiles {
                if shot.ended {
                    continue;
                }
                let Some(target) = self
                    .entities
                    .get(&shot.spec.target_id)
                    .filter(|e| e.hp > 0 && e.targetable)
                else {
                    shot.ended = true;
                    continue;
                };
                let end = (target.p.0 as f64, target.p.1 as f64);
                let dx = end.0 - shot.p.0;
                let dy = end.1 - shot.p.1;
                let d = dx.hypot(dy);
                if d <= shot.spec.speed as f64 + shot.spec.radius as f64 {
                    shot.ended = true;
                    hits.push((
                        shot.effect.clone(),
                        shot.spec.caster_id,
                        shot.spec.target_id,
                    ));
                } else {
                    shot.p.0 += dx / d * shot.spec.speed as f64;
                    shot.p.1 += dy / d * shot.spec.speed as f64;
                }
            }
            for (effect, owner, target) in hits {
                self.sim(|s| combat::impact(s, &effect, owner, target));
            }
        }
    }
    fn incoming(&mut self, id: usize, n: usize) {
        self.sim(|s| {
            s.deal_damage_typed(2, id, n, DamageTypeV1::Ad, AttackTypeV1::BaseAttack);
        });
    }
    fn has(&self, id: usize, name: &str) -> bool {
        self.entities[&id].buffs.iter().any(|b| b.name() == name)
    }
    fn ai(&mut self, base: Option<InputV1>) -> Option<InputV1> {
        let s = Self::table();
        let mut t: AiVtableV1 = unsafe { std::mem::zeroed() };
        t.size = size_of::<AiVtableV1>();
        t.player_id = Some(ai_player);
        t.is_valid_input = Some(ai_valid);
        let mut raw = AiCtxV1 {
            size: size_of::<AiCtxV1>(),
            vtable: &t,
            state: self as *mut _ as *mut c_void,
            sim: &s,
            sim_state: self as *mut _ as *mut c_void,
        };
        ProbeAi.think(
            &mut unsafe { StableAiContext::from_raw(&mut raw).unwrap() },
            base,
        )
    }
}
unsafe fn h<'a>(p: *const c_void) -> &'a Host {
    &*(p as *const Host)
}
unsafe fn m<'a>(p: *mut c_void) -> &'a mut Host {
    &mut *(p as *mut Host)
}
unsafe extern "C" fn tick(p: *const c_void) -> usize {
    h(p).now
}
unsafe extern "C" fn seed(p: *const c_void) -> u64 {
    h(p).seed
}
unsafe extern "C" fn origin(p: *const c_void, o: *mut SimOriginV1) -> bool {
    *o = SimOriginV1 {
        kind: h(p).origin_kind,
        match_id: 555,
        ..Default::default()
    };
    true
}
unsafe extern "C" fn count(p: *const c_void) -> usize {
    h(p).entities.len()
}
unsafe extern "C" fn at(p: *const c_void, i: usize) -> EntityHandleV1 {
    h(p).enum_reads.set(h(p).enum_reads.get() + 1);
    let mut ids: Vec<_> = h(p).entities.keys().copied().collect();
    ids.sort();
    ids.get(i)
        .map_or(EntityHandleV1::NULL, |i| EntityHandleV1::from_id(*i))
}
unsafe extern "C" fn valid(p: *const c_void, id: EntityHandleV1) -> bool {
    id.id().is_some_and(|i| h(p).entities.contains_key(&i))
}
unsafe extern "C" fn position(
    p: *const c_void,
    id: EntityHandleV1,
    x: *mut u64,
    y: *mut u64,
) -> bool {
    let Some(e) = id.id().and_then(|i| h(p).entities.get(&i)) else {
        return false;
    };
    *x = e.p.0;
    *y = e.p.1;
    true
}
fn max_hp(e: &Entity) -> usize {
    let mult: i32 = e.buffs.iter().map(|b| b.hp_mult).sum();
    ((e.stat.hp as i64 * (100 + mult) as i64) / 100).max(1) as usize
}
unsafe extern "C" fn stat(p: *const c_void, id: EntityHandleV1, out: *mut StatV1) -> bool {
    let Some(e) = id.id().and_then(|i| h(p).entities.get(&i)) else {
        return false;
    };
    *out = e.stat;
    (*out).hp = max_hp(e);
    (*out).defence = (e.stat.defence as i64 + e.buffs.iter().map(|b| b.defence as i64).sum::<i64>())
        .max(0) as usize;
    (*out).magic_resistance = (e.stat.magic_resistance as i64
        + e.buffs
            .iter()
            .map(|b| b.magic_resistance as i64)
            .sum::<i64>())
    .max(0) as usize;
    true
}
unsafe extern "C" fn hp(
    p: *const c_void,
    id: EntityHandleV1,
    cur: *mut usize,
    max: *mut usize,
) -> bool {
    let Some(e) = id.id().and_then(|i| h(p).entities.get(&i)) else {
        return false;
    };
    *cur = e.hp;
    *max = max_hp(e);
    true
}
unsafe extern "C" fn level(p: *const c_void, id: EntityHandleV1) -> usize {
    h(p).levels.get(&id.id().unwrap()).copied().unwrap_or(1)
}
unsafe extern "C" fn set_base_stat(
    p: *mut c_void,
    id: EntityHandleV1,
    stat: *const StatV1,
) -> bool {
    m(p).stat_writes += 1;
    let Some(e) = id.id().and_then(|i| m(p).entities.get_mut(&i)) else {
        return false;
    };
    e.stat = *stat;
    true
}
unsafe extern "C" fn attack_speed(p: *const c_void, id: EntityHandleV1) -> usize {
    (100 + h(p).entities[&id.id().unwrap()]
        .buffs
        .iter()
        .map(|b| b.attack_speed_mult)
        .sum::<i32>())
    .max(1) as usize
}
unsafe extern "C" fn team(p: *const c_void, id: EntityHandleV1) -> usize {
    h(p).entities[&id.id().unwrap()].team
}
unsafe extern "C" fn alive_(p: *const c_void, id: EntityHandleV1) -> bool {
    id.id()
        .and_then(|i| h(p).entities.get(&i))
        .is_some_and(|e| e.hp > 0)
}
unsafe extern "C" fn targetable(p: *const c_void, id: EntityHandleV1) -> bool {
    alive_(p, id) && h(p).entities[&id.id().unwrap()].targetable
}
unsafe extern "C" fn champion(p: *const c_void, id: EntityHandleV1) -> bool {
    h(p).entities[&id.id().unwrap()].kind == EntityKindV1::Champion
}
unsafe extern "C" fn kind(p: *const c_void, id: EntityHandleV1) -> u32 {
    h(p).entities[&id.id().unwrap()].kind.code()
}
unsafe extern "C" fn buff_count(p: *const c_void, id: EntityHandleV1) -> usize {
    h(p).entities[&id.id().unwrap()].buffs.len()
}
unsafe extern "C" fn buff_at(
    p: *const c_void,
    id: EntityHandleV1,
    i: usize,
    out: *mut BuffV1,
) -> bool {
    h(p).buff_reads.set(h(p).buff_reads.get() + 1);
    if let Some(b) = h(p).entities[&id.id().unwrap()].buffs.get(i) {
        *out = *b;
        true
    } else {
        false
    }
}
unsafe extern "C" fn add_buff(p: *mut c_void, id: usize, b: *const BuffV1) {
    m(p).buff_writes += 1;
    if let Some(e) = m(p).entities.get_mut(&id) {
        e.buffs.push(*b);
    }
}
unsafe extern "C" fn remove_buff(
    p: *mut c_void,
    id: EntityHandleV1,
    n: *const u8,
    len: usize,
) -> usize {
    m(p).buff_writes += 1;
    let name = String::from_utf8_lossy(std::slice::from_raw_parts(n, len));
    let Some(e) = id.id().and_then(|i| m(p).entities.get_mut(&i)) else {
        return 0;
    };
    let old = e.buffs.len();
    e.buffs.retain(|b| b.name() != name);
    old - e.buffs.len()
}
unsafe extern "C" fn cc_count(p: *const c_void, id: EntityHandleV1) -> usize {
    h(p).entities[&id.id().unwrap()].cc.len()
}
unsafe extern "C" fn cc_at(p: *const c_void, id: EntityHandleV1, i: usize, out: *mut CcV1) -> bool {
    if let Some(c) = h(p).entities[&id.id().unwrap()].cc.get(i) {
        *out = *c;
        true
    } else {
        false
    }
}
unsafe extern "C" fn add_cc(p: *mut c_void, id: usize, c: *const CcV1) {
    m(p).entities.get_mut(&id).unwrap().cc.push(*c);
}
unsafe extern "C" fn spawn_unit(
    p: *mut c_void,
    _: *const u8,
    _: usize,
    _: usize,
    team: usize,
    x: u64,
    y: u64,
    _: u64,
    stat: *const StatV1,
    attack: *const UnitAttackV1,
) -> usize {
    let id = m(p).next;
    m(p).next += 1;
    m(p).entities.insert(
        id,
        Entity {
            team,
            kind: EntityKindV1::Summon,
            p: (x, y),
            hp: (*stat).hp,
            stat: *stat,
            buffs: vec![],
            cc: vec![],
            shield: 0,
            targetable: true,
        },
    );
    m(p).spawned.push((id, *attack));
    id
}
unsafe extern "C" fn set_pos(p: *mut c_void, id: EntityHandleV1, x: u64, y: u64) -> bool {
    let Some(e) = id.id().and_then(|i| m(p).entities.get_mut(&i)) else {
        return false;
    };
    e.p = (x, y);
    m(p).moves.push((id.id().unwrap(), (x, y)));
    true
}

unsafe extern "C" fn spawn_projectile(
    p: *mut c_void,
    _: *const u8,
    _: usize,
    effect: *const u8,
    effect_len: usize,
    spec: *const ProjectileSpawnV1,
) -> bool {
    if !m(p).projectile_ok {
        return false;
    }
    let spec = *spec;
    m(p).projectiles.push(Projectile {
        effect: String::from_utf8_lossy(std::slice::from_raw_parts(effect, effect_len))
            .into_owned(),
        p: (spec.x as f64, spec.y as f64),
        spec,
        ended: false,
    });
    true
}
unsafe extern "C" fn set_hp(p: *mut c_void, id: EntityHandleV1, n: usize) -> bool {
    let Some(e) = id.id().and_then(|i| m(p).entities.get_mut(&i)) else {
        return false;
    };
    e.hp = n;
    true
}
unsafe extern "C" fn damage(
    p: *mut c_void,
    c: usize,
    t: usize,
    n: usize,
    damage_type: u32,
    attack_type: u32,
) -> bool {
    let Some(e) = m(p).entities.get_mut(&t) else {
        return false;
    };
    if e.hp == 0 {
        return false;
    }
    let absorbed = e.shield.min(n);
    e.shield -= absorbed;
    e.hp = e.hp.saturating_sub(n - absorbed);
    m(p).damage.push((c, t, n));
    m(p).damage_types.push((c, t, damage_type, attack_type));
    true
}
unsafe extern "C" fn heal(p: *mut c_void, _: usize, id: usize, n: usize) {
    let e = m(p).entities.get_mut(&id).unwrap();
    e.hp = (e.hp + n).min(max_hp(e));
}
unsafe extern "C" fn shield(p: *mut c_void, id: EntityHandleV1, n: usize, _: usize) -> bool {
    m(p).entities.get_mut(&id.id().unwrap()).unwrap().shield += n;
    true
}

unsafe extern "C" fn read_shield(p: *const c_void, id: EntityHandleV1) -> usize {
    (*(p as *const Host))
        .entities
        .get(&id.id().unwrap())
        .map_or(0, |e| e.shield)
}
unsafe extern "C" fn hide(p: *mut c_void, _: EntityHandleV1, _: usize) -> bool {
    m(p).hidden += 1;
    false
}
unsafe extern "C" fn banish(
    p: *mut c_void,
    _: usize,
    _: EntityHandleV1,
    _: usize,
    _: *const u8,
    _: usize,
    _: *const u8,
    _: usize,
) -> bool {
    m(p).banished += 1;
    false
}
unsafe extern "C" fn cooldown(
    _: *const c_void,
    _: EntityHandleV1,
    _: u32,
    out: *mut CooldownV1,
) -> bool {
    *out = CooldownV1 {
        unlocked: true,
        charges: 1,
        max_charges: 1,
        ..Default::default()
    };
    true
}
unsafe extern "C" fn player_valid(p: *const c_void, id: PlayerHandleV1) -> bool {
    id.id().is_some_and(|i| h(p).entities.contains_key(&i))
}
unsafe extern "C" fn player_champion(_: *const c_void, id: PlayerHandleV1) -> EntityHandleV1 {
    EntityHandleV1::from_id(id.id().unwrap())
}
unsafe extern "C" fn ai_player(p: *const c_void) -> usize {
    h(p).owner
}
unsafe extern "C" fn ai_valid(_: *const c_void, _: *const InputV1) -> bool {
    true
}

#[test]
fn one_companion_is_spawned_and_engine_damage_is_disabled() {
    let mut h = Host::new();
    h.start(1);
    h.step(30);
    assert_eq!(h.spawned.len(), 1);
    let a = h.spawned[0].1;
    assert_eq!((a.attack_ratio, a.attack, a.range), (0, 0, 0));
}
#[test]
fn commanded_bite_delays_contact_and_cannot_pay_twice_in_one_tick() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = h.entities[&2].p;
    h.cast(1, Op::Command, 2);
    assert_eq!(h.damage.len(), 0);
    h.step(1);
    assert_eq!(h.damage.len(), 0);
    h.step(12);
    assert_eq!(h.damage.len(), 2);
    h.sim(tick_all);
    h.sim(tick_all);
    assert_eq!(h.damage.len(), 2);
}
#[test]
fn a_target_that_leaves_bite_reach_is_not_hit() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = h.entities[&2].p;
    h.cast(1, Op::Command, 2);
    h.step(1);
    h.entities.get_mut(&2).unwrap().p = (800_000, 800_000);
    h.step(12);
    assert_eq!(h.damage.len(), 0); // Spear is still chasing; the bite missed.
}
#[test]
fn the_same_frame_skill_callback_cannot_repeat_a_command_hit() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Command, 2);
    h.cast(1, Op::Command, 2);
    assert!(h.damage.is_empty());
    assert_eq!(h.projectiles.len(), 1);
    h.step(12);
    assert_eq!(h.damage.len(), 1);
}
#[test]
fn root_prevents_native_companion_movement() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities
        .get_mut(&k)
        .unwrap()
        .cc
        .push(CcV1::of_kind(CcKindV1::Bind, 90));
    let before = h.entities[&k].p;
    h.step(20);
    assert_eq!(h.entities[&k].p, before);
}
#[test]
fn wall_sweep_does_not_cross_a_solid_cell() {
    let grid = crate::Grid::from_rows(&["....", ".#..", "...."]);
    let from = (27_000, 48_000);
    let dest = (45_000, 48_000);
    for speed in [1_600., 8_000., 20_000.] {
        let p = legal_step(&grid, from, dest, speed);
        assert!(p.0 < 32_000, "wall penetration {p:?}");
    }
}
#[test]
fn open_step_is_bounded_in_distance() {
    let grid = crate::Grid::from_rows(&["....", "....", "...."]);
    let a = (48_000, 48_000);
    let b = (100_000, 80_000);
    let p = legal_step(&grid, a, b, SPEED);
    assert!(dist(a, p) <= SPEED + 2.);
    assert!(dist(p, b) < dist(a, b));
}
#[test]
fn mount_switches_only_cosmetic_buffs_and_still_takes_damage() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Ride, 1);
    h.step(1);
    assert!(h.state(1).ride_until > 0);
    assert!(h.has(1, "mlbb_pk_mount_start"));
    assert!(!h.has(1, OWNER_BODY));
    assert_eq!((h.hidden, h.banished), (0, 0));
    assert!(h.sim(|s| s.get_entity(1).unwrap().is_targetable()));
    let hp = h.entities[&1].hp;
    let shield = h.entities[&1].shield;
    h.incoming(1, shield + 50);
    assert_eq!(h.entities[&1].hp, hp - 50);
}

#[test]
fn mounted_motion_faces_left_and_uses_only_one_pair_layer() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Ride, 1);
    h.step(22);
    let k = h.pet(1);
    assert!(h.has(1, RIDE_BODY));
    h.entities.get_mut(&1).unwrap().p.0 -= 2000;
    h.step(1);
    assert!(h.has(1, "mlbb_pk_mount_run_left"));
    assert_eq!(
        visual::BINDINGS
            .iter()
            .filter(|(_, _, _, name)| h.has(1, name))
            .count(),
        1
    );
    assert_eq!(
        visual::BINDINGS
            .iter()
            .filter(|(_, _, _, name)| h.has(k, name))
            .count(),
        0
    );
}

#[test]
fn consumed_shield_clears_its_visual_before_duration_expires() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Ride, 1);
    h.step(1);
    assert!(h.has(1, "mlbb_pk_fx_shield_start"));
    let amount = h.entities[&1].shield;
    h.incoming(1, amount);
    h.step(1);
    assert!(!h.has(1, "mlbb_pk_fx_shield_start"));
    assert!(!h.has(1, "mlbb_pk_fx_shield_loop"));
}

#[test]
fn prayer_pose_and_effect_end_when_movement_interrupts_channel() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().hp = 0;
    h.step(181);
    assert!(h.has(1, "mlbb_pk_popol_pray"));
    assert!(h.has(1, "mlbb_pk_fx_prayer"));
    h.entities.get_mut(&1).unwrap().p.0 += 2000;
    h.step(1);
    assert!(!h.has(1, "mlbb_pk_popol_pray"));
    assert!(!h.has(1, "mlbb_pk_fx_prayer"));
}
#[test]
fn mount_uses_owner_root_and_dismount_does_not_teleport_owner() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Ride, 1);
    h.step(1);
    let k = h.pet(1);
    h.entities.get_mut(&1).unwrap().p = (201_000, 201_000);
    h.step(1);
    assert_eq!(h.entities[&k].p, h.entities[&1].p);
    h.step(RIDE_TICKS);
    assert!(h.has(1, "mlbb_pk_mount_end"));
    h.step(21);
    assert_eq!(h.entities[&1].p, (201_000, 201_000));
    assert!(!h.has(1, RIDE_BODY));
    assert!(h.has(1, OWNER_BODY));
    assert!(h.moves.iter().all(|(id, _)| *id != 1));
}
#[test]
fn hard_cc_ends_mount_and_restores_both_body_layers() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Ride, 1);
    h.step(1);
    let k = h.pet(1);
    h.entities
        .get_mut(&1)
        .unwrap()
        .cc
        .push(CcV1::of_kind(CcKindV1::Stun, 30));
    h.step(1);
    assert_eq!(h.state(1).ride_until, 0);
    assert!(h.has(1, OWNER_BODY));
    assert!(h.has(k, KUPA_BODY));
}
#[test]
fn companion_defeat_ends_mount_without_hiding_owner() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Ride, 1);
    h.step(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().hp = 0;
    h.step(1);
    assert_eq!(h.state(1).ride_until, 0);
    assert!(h.has(1, OWNER_BODY));
    assert_eq!((h.hidden, h.banished), (0, 0));
}
#[test]
fn alpha_keeps_entity_identity_and_does_not_heal_each_update() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().hp = 50;
    h.cast(1, Op::Alpha, 1);
    assert_eq!(h.pet(1), k);
    assert!(h.has(k, "mlbb_pk_kupa_alpha_transform"));
    assert_eq!(h.entities[&k].hp, 756);
    h.entities.get_mut(&k).unwrap().hp = 100;
    h.step(15);
    assert_eq!(h.entities[&k].hp, 100);
    let key = h.key(1);
    STATES.lock().unwrap().get_mut(&key).unwrap().last_combat = ALPHA_TICKS;
    h.step(ALPHA_TICKS);
    assert_eq!(h.pet(1), k);
    assert!(!h.has(k, ALPHA_BODY));
    assert!(visual::BINDINGS
        .iter()
        .any(|(form, _, _, name)| *form == "kupa" && h.has(k, name)));
    assert_eq!(h.entities[&k].hp, 100);
    assert_eq!(h.spawned.len(), 1);
}
#[test]
fn alpha_expiry_clamps_extra_hp_without_a_fake_kill() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.cast(1, Op::Alpha, 1);
    h.step(ALPHA_TICKS + 1);
    assert_eq!(h.entities[&k].hp, 560);
    assert!(h.damage.iter().all(|(_, t, _)| *t != k));
}
#[test]
fn owner_death_cleans_up_the_companion() {
    let mut h = Host::new();
    let mut p = h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&1).unwrap().hp = 0;
    h.sim(|s| p.on_dead(s, 1));
    assert_eq!(h.entities[&k].hp, 0);
    let owner = h.key(1);
    assert!(!STATES.lock().unwrap().contains_key(&owner));
}
#[test]
fn two_owners_cannot_share_a_companion_or_alpha_state() {
    let mut h = Host::new();
    h.add(3, 0, (210_000, 200_000));
    h.start(1);
    h.start(3);
    let a = h.pet(1);
    let b = h.pet(3);
    assert_ne!(a, b);
    h.cast(1, Op::Alpha, 1);
    assert!(h.has(a, "mlbb_pk_kupa_alpha_transform"));
    assert!(!h.has(b, ALPHA_BODY));
}
#[test]
fn identical_match_metadata_and_entity_ids_still_isolate_sim_copies() {
    let mut a = Host::new();
    let mut b = Host::new();
    b.seed = a.seed;
    a.start(1);
    b.start(1);
    let ak = a.key(1);
    let bk = b.key(1);
    assert_ne!(ak, bk);
    a.cast(1, Op::Alpha, 1);
    assert_eq!(b.state(1).alpha_until, 0);
}
#[test]
fn death_before_mount_arrival_does_not_restore_the_dead_companion() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (400_000, 400_000);
    h.cast(1, Op::Ride, 1);
    h.entities.get_mut(&k).unwrap().hp = 0;
    h.step(1);
    assert!(!h.state(1).recalling);
    assert_eq!(h.entities[&k].hp, 0);
    assert_eq!(h.spawned.len(), 1);
}
#[test]
fn ai_is_read_only_and_can_trigger_a_defensive_ride() {
    let mut h = Host::new();
    h.start(1);
    h.entities.get_mut(&1).unwrap().hp = 400;
    let key = h.key(1);
    let before = STATES.lock().unwrap()[&key].updated;
    let hp = h.entities[&1].hp;
    let proposal = h
        .ai(Some(InputV1::action(
            InputKindV1::Attack,
            InputTargetV1::target(2),
        )))
        .unwrap();
    assert_eq!(proposal.kind, InputKindV1::Skill2.code());
    assert_eq!(h.entities[&1].hp, hp);
    assert_eq!(STATES.lock().unwrap()[&key].updated, before);
    assert!(!h.has(1, RIDE_BODY));
}
#[test]
fn ordinary_spear_attacks_do_not_turn_every_bite_into_skill1() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = h.entities[&2].p;
    h.cast(1, Op::Basic, 2);
    h.step(13);
    assert_eq!(
        h.damage.iter().map(|(_, _, n)| *n).collect::<Vec<_>>(),
        vec![82, 36]
    );
}
#[test]
fn unreachable_recall_times_out_instead_of_locking_the_hero_forever() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (1_500_000, 1_500_000);
    h.cast(1, Op::Ride, 1);
    h.step(181);
    assert!(!h.state(1).recalling);
    assert_eq!(h.state(1).ride_until, 0);
    assert!(h.has(1, OWNER_BODY));
}

#[test]
fn companion_outside_fanny_cable_border_can_walk_back_to_owner() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&2).unwrap().hp = 0;
    h.entities.get_mut(&k).unwrap().p = (0, 0);
    let mut last = (0, 0);
    for _ in 0..400 {
        h.step(1);
        let next = h.entities[&k].p;
        assert!(dist(last, next) <= SPEED + 2., "unmounted companion warped");
        last = next;
    }
    assert!(dist(last, h.entities[&1].p) <= FOLLOW + 2000.);
}

#[test]
fn owner_teleport_dismounts_without_teleporting_the_companion() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Ride, 1);
    h.step(1);
    let k = h.pet(1);
    let old = h.entities[&k].p;
    h.entities.get_mut(&1).unwrap().p = (850000, 850000);
    h.step(1);
    assert_eq!(h.state(1).ride_until, 0);
    assert!(dist(old, h.entities[&k].p) <= SPEED + 2.);
    assert_eq!(h.entities[&1].p, (850000, 850000));
    assert!(!visual::BINDINGS
        .iter()
        .any(|(form, _, _, name)| form.starts_with("mount") && h.has(1, name)));
}

#[test]
fn distant_skill2_plays_recall_while_kupa_walks_then_mounts_on_arrival() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (176000, 208000);
    h.cast(1, Op::Ride, 1);
    let old = h.entities[&k].p;
    h.step(1);
    assert!(h.state(1).recalling);
    assert_eq!(h.state(1).ride_until, 0);
    assert!(h.has(k, "mlbb_pk_fx_recall"));
    assert!(dist(old, h.entities[&k].p) <= SPEED + 2.);
    h.step(90);
    assert!(h.state(1).ride_until > h.now);
    assert!(!h.has(k, "mlbb_pk_fx_recall"));
}
#[test]
fn respawn_callback_disposes_of_the_previous_companion() {
    let mut h = Host::new();
    let mut p = h.start(1);
    let old = h.pet(1);
    h.sim(|s| p.on_spawn(s, 1, 1));
    let new = h.pet(1);
    assert_ne!(old, new);
    assert_eq!(h.entities[&old].hp, 0);
    assert!(h.entities[&new].hp > 0);
}
#[test]
fn alpha_expires_even_if_mount_is_still_active() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Alpha, 1);
    h.step(500);
    let pet = h.pet(1);
    h.entities.get_mut(&pet).unwrap().p = h.entities[&1].p;
    h.cast(1, Op::Ride, 1);
    h.step(1);
    let k = h.pet(1);
    assert!(h.state(1).ride_until > h.now);
    h.step(40);
    assert_eq!(h.state(1).alpha_until, 0);
    assert!(!h.has(k, ALPHA_STATS));
    assert!(h.state(1).ride_until > h.now);
}
#[test]
fn no_nearby_enemy_preserves_baseline_lane_movement() {
    let mut h = Host::new();
    h.start(1);
    h.entities.get_mut(&2).unwrap().hp = 0;
    let base = InputV1::move_to(350_000, 250_000);
    let result = h.ai(Some(base)).unwrap();
    assert_eq!(result.kind, InputKindV1::Move.code());
    assert_eq!((result.x, result.y), (350_000, 250_000));
}
#[test]
fn a_committed_basic_attack_during_recall_does_not_cancel_the_return() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (230_000, 200_000);
    h.cast(1, Op::Ride, 1);
    assert!(h.state(1).recalling);
    h.cast(1, Op::Basic, 2);
    assert!(h.state(1).recalling);
}
#[test]
fn recall_ai_waits_for_kupa_instead_of_running_away() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (230_000, 200_000);
    h.cast(1, Op::Ride, 1);
    let i = h.ai(Some(InputV1::move_to(100_000, 100_000))).unwrap();
    assert_eq!(i.kind, InputKindV1::Move.code());
    assert_eq!((i.x, i.y), h.entities[&1].p);
}
#[test]
fn defeated_kupa_returns_as_a_fresh_unit_after_safe_prayer() {
    let mut h = Host::new();
    h.start(1);
    let old = h.pet(1);
    h.entities.get_mut(&old).unwrap().hp = 0;
    h.step(181);
    assert!(h.state(1).prayer_started.is_some());
    h.step(180);
    let new = h.pet(1);
    assert_ne!(new, old);
    assert!(h.entities[&new].hp > 0);
    assert_eq!(h.spawned.len(), 2);
}
#[test]
fn movement_interrupts_prayer_instead_of_reviving_while_running() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().hp = 0;
    h.step(181);
    assert!(h.state(1).prayer_started.is_some());
    h.entities.get_mut(&1).unwrap().p.0 += 2_000;
    h.step(1);
    assert!(h.state(1).prayer_started.is_none());
    assert_eq!(h.spawned.len(), 1);
}
#[test]
fn incoming_shield_hit_interrupts_prayer_without_needing_hp_loss() {
    let mut h = Host::new();
    let mut passive = h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().hp = 0;
    h.step(181);
    h.sim(|s| passive.on_damaged(s, 1, 1, 2, 25));
    assert!(h.state(1).prayer_started.is_none());
    h.step(100);
    assert_eq!(h.spawned.len(), 1);
}
#[test]
fn revival_cooldown_prevents_immediate_successive_resurrections() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().hp = 0;
    h.step(361);
    let new = h.pet(1);
    h.entities.get_mut(&new).unwrap().hp = 0;
    h.step(400);
    assert_eq!(h.spawned.len(), 2);
    assert!(h.state(1).prayer_started.is_none());
}

include!("popol_combat_tests.rs");

#[test]
#[ignore = "manual release-mode before/after performance probe"]
fn popol_performance_probe() {
    use std::time::Instant;
    let mut h = Host::new();
    h.start(1);
    h.entities.get_mut(&2).unwrap().hp = 0;
    let key = h.key(1);
    let mut historical = Vec::new();
    for n in 0..10000u64 {
        let mut old = key;
        old.scope.seed = old.scope.seed.wrapping_add(n + 1);
        old.copy = old.copy.wrapping_add(n + 1);
        STATES.lock().unwrap().insert(old, State::default());
        historical.push(old);
    }
    let start = Instant::now();
    h.step(2000);
    let ticks = start.elapsed();
    let mut layout = maplayout::Layout::default();
    let start = Instant::now();
    for _ in 0..1000 {
        h.sim(|s| {
            let _ = layout.select(s, (200000, 200000), (300000, 200000));
        });
    }
    println!(
        "PERF ticks_2000_history_10000_ms={:.3} map_select_1000_ms={:.3}",
        ticks.as_secs_f64() * 1000.,
        start.elapsed().as_secs_f64() * 1000.
    );
    for old in historical {
        STATES.lock().unwrap().remove(&old);
    }
}

#[test]
fn repeated_tick_dispatch_does_not_repeat_cosmetic_work() {
    let mut h = Host::new();
    let mut passive = h.start(1);
    h.step(1);
    let reads = h.buff_reads.get();
    let writes = h.buff_writes;
    h.sim(tick_all);
    h.sim(tick_all);
    h.sim(|s| passive.on_update(s, 0, 1, 1));
    assert!(
        h.buff_reads.get() - reads <= 4,
        "duplicate dispatch rescanned cosmetic buffs"
    );
    assert_eq!(h.buff_writes, writes);
}
#[test]
fn stationary_visual_refresh_uses_cached_pose_and_effect_identity() {
    let mut h = Host::new();
    h.start(1);
    h.entities.get_mut(&2).unwrap().hp = 0;
    h.step(1);
    let mut state = h.state(1);
    let reads = h.buff_reads.get();
    let writes = h.buff_writes;
    for _ in 0..10 {
        h.sim(|s| show_bodies(s, 1, &mut state));
    }
    assert!(
        h.buff_reads.get() - reads <= 24,
        "stationary pose rescanned all visual bindings"
    );
    assert_eq!(h.buff_writes, writes);
}
#[test]
fn presimulation_omits_cosmetic_buffs_but_retains_projectile_damage() {
    let mut h = Host::new();
    h.origin_kind = SimOriginKindV1::ServerPresim.code();
    h.start(1);
    let k = h.pet(1);
    assert!(!visual::BINDINGS
        .iter()
        .any(|(_, _, _, name)| h.has(1, name) || h.has(k, name)));
    hold_pet(&mut h);
    h.cast(1, Op::Basic, 2);
    h.step(9);
    assert_eq!(h.damage, vec![(1, 2, 82)]);
    assert!(!visual::BINDINGS
        .iter()
        .any(|(_, _, _, name)| h.has(1, name) || h.has(k, name)));
}
#[test]
fn map_selection_reuses_one_grid_between_scheduled_checks() {
    let mut h = Host::new();
    h.start(1);
    let mut layout = maplayout::Layout::default();
    let (_, a, _) = h.sim(|s| layout.select(s, (200000, 200000), (300000, 200000)));
    for _ in 0..100 {
        let (_, b, _) = h.sim(|s| layout.select(s, (200000, 200000), (300000, 200000)));
        assert!(std::sync::Arc::ptr_eq(&a, &b));
    }
}

#[test]
fn separate_owners_share_immutable_wall_storage() {
    let mut h = Host::new();
    h.start(1);
    let mut a = maplayout::Layout::default();
    let mut b = maplayout::Layout::default();
    let (_, ga, _) = h.sim(|s| a.select(s, (200000, 200000), (300000, 200000)));
    let (_, gb, _) = h.sim(|s| b.select(s, (200000, 200000), (300000, 200000)));
    assert!(std::sync::Arc::ptr_eq(&ga, &gb));
}

include!("popol_traps_tests.rs");

unsafe extern "C" fn play_view(
    p: *mut c_void,
    name: *const u8,
    len: usize,
    _: usize,
    _: *const InputTargetV1,
    _: u64,
    _: u64,
    time: u64,
) -> bool {
    let name = String::from_utf8_lossy(std::slice::from_raw_parts(name, len)).into_owned();
    m(p).view_calls.push((name, time));
    true
}

unsafe extern "C" fn seen(p: *const c_void, team: usize, id: usize) -> bool {
    !h(p).unseen.contains(&(team, id))
}

include!("popol_cooperation_tests.rs");
