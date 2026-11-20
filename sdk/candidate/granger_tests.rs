#![allow(dead_code)]
//! ABI callback tests. A mock host is not a live TFM2 renderer/pathfinder.
use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};
static SEED: AtomicU64 = AtomicU64::new(19_000_000);
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
    crit_samples: Vec<usize>,
    refunds: Vec<usize>,
    kills: usize,
    assists: usize,
    rondo_cycle: usize,
    ended: bool,
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
            crit_samples: vec![],
            refunds: vec![],
            kills: 0,
            assists: 0,
            rondo_cycle: 720,
            ended: false,
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
        t.is_end = Some(is_end);
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
        t.projectile_count = Some(projectile_count);
        t.projectile_at = Some(projectile_at);
        t.projectile_is_valid = Some(projectile_valid);
        t.projectile_info = Some(projectile_info);
        t.play_view_effect = Some(play_view);
        t.is_visible = Some(seen);
        t.entity_set_pos = Some(set_pos);
        t.entity_set_hp = Some(set_hp);
        t.deal_damage_typed = Some(damage);
        t.deal_damage = Some(full_damage);
        t.entity_reduce_cooldown = Some(reduce);
        t.player_kills = Some(kills);
        t.player_assists = Some(assists);
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
        }
    }
    fn land(&mut self, index: usize, target: usize) {
        self.projectiles[index].ended = true;
        let effect = self.projectiles[index].effect.clone();
        let slot = effect.rsplit('_').next().unwrap().parse().unwrap();
        let owner = self.projectiles[index].spec.caster_id;
        self.sim(|s| Hit(slot).apply(s, 0, owner, InputTargetV1::target(target)));
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
        Ai.think(
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
unsafe extern "C" fn is_end(p: *const c_void) -> bool {
    h(p).ended
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
    (*out).crit_chance = ((*out).crit_chance as i64
        + e.buffs.iter().map(|b| b.crit_chance as i64).sum::<i64>())
    .max(0) as usize;
    (*out).attack = ((*out).attack as i64 + e.buffs.iter().map(|b| b.attack as i64).sum::<i64>())
        .max(0) as usize;
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
    // Unlike an ability's aiming mode, the projectile callback contract must
    // identify the entity it hit. The real host panics on Direction here.
    if (*spec).casting_type != CastingTypeV1::Targeting.code() {
        return false;
    }
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
unsafe extern "C" fn projectile_count(p: *const c_void) -> usize {
    h(p).projectiles.len()
}
unsafe extern "C" fn projectile_at(p: *const c_void, index: usize) -> ProjectileHandleV1 {
    if index < h(p).projectiles.len() {
        ProjectileHandleV1::from_id(index)
    } else {
        ProjectileHandleV1::NULL
    }
}
unsafe extern "C" fn projectile_valid(p: *const c_void, id: ProjectileHandleV1) -> bool {
    id.id().is_some_and(|i| i < h(p).projectiles.len())
}
unsafe extern "C" fn projectile_info(
    p: *const c_void,
    id: ProjectileHandleV1,
    out: *mut ProjectileInfoV1,
) -> bool {
    let Some(shot) = id.id().and_then(|i| h(p).projectiles.get(i)) else {
        return false;
    };
    *out = ProjectileInfoV1 {
        x: shot.p.0 as u64,
        y: shot.p.1 as u64,
        caster_id: shot.spec.caster_id,
        team: shot.spec.team,
        is_end: shot.ended,
    };
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
    p: *const c_void,
    _: EntityHandleV1,
    slot: u32,
    out: *mut CooldownV1,
) -> bool {
    *out = CooldownV1 {
        unlocked: true,
        cooltime: if slot == SkillSlotV1::Skill2.code() {
            h(p).rondo_cycle
        } else {
            36
        },
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

unsafe extern "C" fn full_damage(
    p: *mut c_void,
    c: usize,
    t: usize,
    ad: usize,
    ap: usize,
    attack: u32,
) {
    let mut stat_ = StatV1::default();
    stat(p, EntityHandleV1::from_id(c), &mut stat_);
    m(p).crit_samples.push(stat_.crit_chance);
    // Deliberately NO mock critical multiplier: this tests ABI routing and
    // buff scope, not undocumented host crit math or item behavior.
    damage(p, c, t, ad + ap, DamageTypeV1::Ad.code(), attack);
}
unsafe extern "C" fn reduce(p: *mut c_void, _: EntityHandleV1, _: u32, n: usize) -> bool {
    m(p).refunds.push(n);
    true
}
unsafe extern "C" fn kills(p: *const c_void, _: PlayerHandleV1) -> usize {
    h(p).kills
}
unsafe extern "C" fn assists(p: *const c_void, _: PlayerHandleV1) -> usize {
    h(p).assists
}

#[test]
fn separate_energy_regeneration_preserves_fractional_units() {
    let mut h = Host::new();
    h.start(1);
    let k = h.key(1);
    {
        let mut all = STATES.lock().unwrap();
        let s = all.get_mut(&k).unwrap();
        s.gun.current = 0;
        s.cannon.current = 0;
    }
    h.step(60);
    let s = h.state(1);
    assert_eq!((s.gun.current, s.cannon.current), (64, 16));
    h.step(60);
    let s = h.state(1);
    assert_eq!((s.gun.current, s.cannon.current), (128, 32));
}
#[test]
fn barrage_has_six_emissions_costs_energy_and_deals_nothing_at_launch() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(91);
    assert_eq!(h.projectiles.len(), 6);
    assert!(h.damage.is_empty());
    assert_eq!(h.state(1).gun.current, 476); // First tick is full before shot; 90 effective regen ticks.
    assert!(h.state(1).burst.is_none());
    assert!(!h.has(1, SPEED_BUFF));
    for i in 0..6 {
        h.land(i, 2);
    }
    assert_eq!(h.damage.len(), 6);
    assert_eq!(h.refunds, vec![30; 6]);
    assert_eq!(
        h.damage_types.iter().map(|x| x.3).collect::<Vec<_>>(),
        vec![AttackTypeV1::Skill.code(); 6]
    );
    assert_eq!(h.crit_samples, vec![0, 0, 0, 0, 0, 100]);
    assert!(!h.has(1, CRIT_BUFF));
}
#[test]
fn exactly_420_cannot_start_and_spawn_failure_costs_nothing() {
    let mut h = Host::new();
    h.start(1);
    let k = h.key(1);
    STATES.lock().unwrap().get_mut(&k).unwrap().gun.current = 420;
    h.cast(1, Op::Burst, 2);
    assert!(h.state(1).burst.is_none());
    h.now = 1;
    STATES.lock().unwrap().get_mut(&k).unwrap().gun.current = 421;
    h.projectile_ok = false;
    h.cast(1, Op::Burst, 2);
    h.sim(tick_all);
    assert!(h.projectiles.is_empty());
    assert_eq!(h.state(1).gun.current, 422);
}
#[test]
fn interrupted_barrage_stops_future_shots_but_emitted_receipt_still_lands() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(1);
    h.entities.get_mut(&1).unwrap().cc.push(CcV1::stun(60));
    h.step(30);
    assert_eq!(h.projectiles.len(), 1);
    h.land(0, 2);
    h.land(0, 2);
    assert_eq!(h.damage.len(), 1);
    assert_eq!(h.refunds, vec![30]);
}
#[test]
fn dash_refills_and_barrage_uses_current_muzzle_after_repositioning() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(1);
    let before = h.state(1).gun.current;
    h.cast(1, Op::Rondo, 2);
    assert_eq!(h.state(1).gun.current, 800.min(before + 120));
    h.step(18);
    assert!(h.projectiles[1].spec.x > h.projectiles[0].spec.x);
    assert!(h.state(1).burst.is_some());
    let ready = h.state(1).rondo_ready;
    h.land(0, 2);
    assert_eq!(h.state(1).rondo_ready, ready - 30);
    h.land(0, 2);
    assert_eq!(h.state(1).rondo_ready, ready - 30);
}
#[test]
fn cannon_during_barrage_uses_hero_filter_and_hits_primary_once_with_area_slow() {
    let mut h = Host::new();
    h.start(1);
    h.add(3, 1, (255_000, 200_000));
    h.cast(1, Op::Burst, 2);
    h.step(1);
    h.cast(1, Op::Cannon, 2);
    assert_eq!(h.projectiles.len(), 2);
    assert_eq!(
        h.projectiles[1].spec.casting_target,
        CastingTargetV1::EnemyChampion.code()
    );
    assert_eq!(h.state(1).cannon.current, 550);
    assert!(h.state(1).burst.is_some());
    h.land(1, 2);
    h.land(1, 2);
    assert_eq!(h.damage.iter().map(|d| d.1).collect::<Vec<_>>(), vec![2, 3]);
    assert!(h.has(2, "mlbb_granger_slow_1"));
    assert!(h.has(3, "mlbb_granger_slow_1"));
}
#[test]
fn farm_is_personal_typed_deduplicated_and_excludes_objectives_and_summons() {
    let mut h = Host::new();
    let mut p = h.start(1);
    for (id, kind) in [
        (3, EntityKindV1::Minion),
        (4, EntityKindV1::Jungle),
        (5, EntityKindV1::Epic),
        (6, EntityKindV1::Summon),
    ] {
        h.add(id, 1, (300_000, 200_000));
        h.entities.get_mut(&id).unwrap().kind = kind;
        h.sim(|s| p.on_unit_kill(s, 1, 1, id));
        h.sim(|s| p.on_unit_kill(s, 1, 1, id));
    }
    let s = h.state(1);
    assert_eq!((s.gun.max, s.cannon.max), (806, 806));
}
#[test]
fn takedowns_are_counted_once_and_capacities_survive_respawn() {
    let mut h = Host::new();
    let mut p = h.start(1);
    h.kills = 1;
    h.assists = 2;
    h.step(2);
    assert_eq!(h.state(1).gun.max, 836);
    h.entities.get_mut(&1).unwrap().hp = 0;
    h.step(1);
    h.entities.get_mut(&1).unwrap().hp = 660;
    h.sim(|s| p.on_spawn(s, 1, 1));
    assert_eq!((h.state(1).gun.current, h.state(1).gun.max), (836, 836));
    h.step(1);
    assert_eq!(h.state(1).gun.max, 836);
    let mut other = Host::new();
    other.start(1);
    assert_eq!(other.state(1).gun.max, 800);
}
#[test]
fn duplicate_driver_tick_and_parallel_copies_do_not_double_regen_or_shots() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(1);
    let before = h.state(1).gun;
    h.sim(tick_all);
    h.sim(tick_all);
    assert_eq!(h.state(1).gun, before);
    let mut other = Host::new();
    other.seed = h.seed;
    other.start(1);
    other.step(1);
    assert_eq!(other.projectiles.len(), 0);
    assert_ne!(h.key(1), other.key(1));
}
#[test]
fn scoped_crit_does_not_change_basic_receipts_and_no_manual_crit_multiplier_is_used() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.cast(1, Op::Burst, 2);
    h.step(91);
    h.land(6, 2);
    h.land(0, 2);
    assert_eq!(h.crit_samples, vec![100, 0]);
    assert_eq!(h.damage[0].2, 72); // Buffed round at 82 AD; engine still owns crit.
    assert_eq!(h.damage[1].2, 82);
    assert_eq!(h.damage_types[1].3, AttackTypeV1::BaseAttack.code());
}
#[test]
fn cooldown_conversion_is_capped_and_not_reduced_by_hit_refunds() {
    let mut h = Host::new();
    h.rondo_cycle = 360;
    h.start(1);
    h.step(60);
    assert_eq!(h.state(1).cdr_bonus, 20);
    h.cast(1, Op::Burst, 2);
    h.step(1);
    h.land(0, 2);
    h.step(59);
    assert_eq!(h.state(1).cdr_bonus, 20);
}
#[test]
fn wall_sampling_stops_before_crossing_and_outside_map() {
    let g = crate::Grid::from_rows(&["..#.", "...."]);
    let p = legal_step(&g, (48_000, 16_000), (1., 0.), 35_000.);
    assert!(p.0 < 64_000);
    assert!(p.0 > 48_000);
    assert_eq!(
        legal_step(&g, (127_900, 48_000), (1., 0.), 35_000.).0,
        127_900
    );
}
#[test]
fn death_cancels_pending_burst_and_respawn_rejects_old_callbacks() {
    let mut h = Host::new();
    let mut p = h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(1);
    h.entities.get_mut(&1).unwrap().hp = 0;
    h.step(1);
    h.entities.get_mut(&1).unwrap().hp = 660;
    h.sim(|s| p.on_spawn(s, 1, 1));
    h.land(0, 2);
    assert!(h.damage.is_empty());
    h.step(100);
    assert_eq!(h.projectiles.len(), 1);
}

#[test]
fn matches_without_granger_do_no_entity_enumeration_or_buff_writes() {
    let mut h = Host::new();
    h.step(600);
    assert_eq!(h.enum_reads.get(), 0);
    assert_eq!(h.buff_writes, 0);
    assert!(h.projectiles.is_empty());
    assert!(h.damage.is_empty());
}
#[test]
fn end_cleanup_removes_only_this_simulation_copy() {
    let mut h = Host::new();
    h.start(1);
    let key = h.key(1);
    let mut other = Host::new();
    other.seed = h.seed;
    other.start(1);
    let other_key = other.key(1);
    h.ended = true;
    h.sim(tick_all);
    let states = STATES.lock().unwrap();
    assert!(!states.contains_key(&key));
    assert!(states.contains_key(&other_key));
}
#[test]
fn silence_blocks_rondo_and_interrupted_burst_still_obeys_repeat_gate() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(1);
    h.entities
        .get_mut(&1)
        .unwrap()
        .cc
        .push(CcV1::of_kind(CcKindV1::BlockSkill, 1));
    h.cast(1, Op::Rondo, 2);
    assert!(h.state(1).dash.is_none());
    let key = h.key(1);
    STATES.lock().unwrap().get_mut(&key).unwrap().burst = None;
    h.entities.get_mut(&1).unwrap().cc.clear();
    h.now = 2;
    h.cast(1, Op::Burst, 2);
    assert!(h.state(1).burst.is_none());
}

#[test]
fn respawn_copy_marker_reset_migrates_capacity_through_passive_update() {
    let mut h = Host::new();
    let mut p = h.start(1);
    h.kills = 2;
    h.step(1);
    h.entities.get_mut(&1).unwrap().buffs.clear();
    h.sim(|s| p.on_update(s, 0, 1, 1));
    assert_eq!(h.state(1).gun.max, 824);
    assert_eq!(h.state(1).gun.current, 824);
}

#[test]
fn all_granger_projectiles_use_targeting_hit_callbacks_and_homing_travel() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.cast(1, Op::Burst, 2);
    h.step(91);
    h.cast(1, Op::Cannon, 2);
    assert_eq!(
        h.projectiles.len(),
        8,
        "basic + all six rounds + cannon must be accepted by the host contract"
    );
    for p in h.projectiles.iter() {
        assert_eq!(p.spec.casting_type, CastingTypeV1::Targeting.code());
        assert_eq!(p.spec.move_kind, ProjectileMoveKindV1::Target.code());
        assert_eq!(p.spec.target_id, 2);
    }
    h.land(1, 2);
    h.land(7, 2);
    assert_eq!(h.damage.len(), 2);
    assert_eq!(h.refunds, vec![30]);
}

#[test]
fn barrage_keeps_selected_enemy_and_aims_later_rounds_at_its_new_position() {
    let mut h = Host::new();
    h.start(1);
    h.add(3, 1, (260_000, 200_000));
    h.cast(1, Op::Burst, 3);
    h.step(1);
    h.entities.get_mut(&3).unwrap().p = (230_000, 240_000);
    h.step(90);
    assert_eq!(h.projectiles.len(), 6);
    assert!(h
        .projectiles
        .iter()
        .all(|p| p.spec.target_id == 3 && p.spec.move_kind == ProjectileMoveKindV1::Target.code()));
    assert_eq!(h.projectiles[1].spec.target_y, 240_000);
}
#[test]
fn dead_burst_target_reacquires_visible_enemy_without_refunding_a_dead_hit() {
    let mut h = Host::new();
    h.start(1);
    h.add(3, 1, (260_000, 200_000));
    h.cast(1, Op::Burst, 2);
    h.step(1);
    h.entities.get_mut(&2).unwrap().hp = 0;
    h.step(90);
    assert_eq!(h.projectiles[0].spec.target_id, 2);
    assert!(h.projectiles[1..].iter().all(|p| p.spec.target_id == 3));
    h.land(0, 2);
    assert!(h.damage.is_empty());
    assert!(h.refunds.is_empty());
}
#[test]
fn no_replacement_target_stops_unemitted_rounds_and_their_cost() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(1);
    h.entities.get_mut(&2).unwrap().targetable = false;
    h.step(90);
    assert_eq!(h.projectiles.len(), 1);
    assert!(h.state(1).burst.is_none());
    assert_eq!(h.state(1).gun.current, 800);
}
#[test]
fn cannon_rejects_nonhero_and_out_of_range_targets_without_energy_charge() {
    let mut h = Host::new();
    h.start(1);
    h.add(3, 1, (250_000, 200_000));
    h.entities.get_mut(&3).unwrap().kind = EntityKindV1::Minion;
    h.cast(1, Op::Cannon, 3);
    h.entities.get_mut(&2).unwrap().p = (500_000, 200_000);
    h.cast(1, Op::Cannon, 2);
    assert_eq!(h.state(1).cannon.current, 800);
    assert!(h.projectiles.is_empty());
    h.entities.get_mut(&2).unwrap().p = (250_000, 200_000);
    h.cast(1, Op::Cannon, 2);
    assert_eq!(h.projectiles[0].spec.target_id, 2);
    assert_eq!(h.state(1).cannon.current, 550);
}
#[test]
fn expired_homing_slots_wait_for_host_projectile_cleanup_before_reuse() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.step(240);
    assert_eq!(h.state(1).flights.len(), 1);
    h.projectiles[0].ended = true;
    h.step(60);
    assert!(h.state(1).flights.is_empty());
}
#[test]
fn ai_requests_target_ids_for_barrage_and_cannon_instead_of_guessing_direction() {
    let mut h = Host::new();
    h.start(1);
    let recall = h.ai(Some(InputV1::return_home())).unwrap();
    assert_eq!(recall.kind, InputKindV1::Return.code());
    let input = h.ai(None).unwrap();
    assert_eq!(input.kind, InputKindV1::Ult.code());
    assert_eq!(input.target.kind, InputTargetKindV1::Target.code());
    assert_eq!(input.target.target_id, 2);
    let k = h.key(1);
    {
        let mut all = STATES.lock().unwrap();
        let s = all.get_mut(&k).unwrap();
        s.cannon.current = 0;
        s.rondo_ready = 10000;
    }
    let input = h.ai(None).unwrap();
    assert_eq!(input.kind, InputKindV1::Skill.code());
    assert_eq!(input.target.kind, InputTargetKindV1::Target.code());
    assert_eq!(input.target.target_id, 2);
}

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

#[test]
fn visual_idle_is_stable_and_movement_selects_one_facing_pose() {
    let mut h = Host::new();
    h.start(1);
    assert!(h.has(1, "mlbb_granger_pose_idle"));
    let writes = h.buff_writes;
    h.step(600);
    assert_eq!(
        h.buff_writes, writes,
        "idle must not remove/re-add a body every tick"
    );
    h.entities.get_mut(&1).unwrap().p.0 -= 2000;
    h.step(1);
    assert!(h.has(1, "mlbb_granger_pose_run_left"));
    h.step(1);
    assert!(h.has(1, "mlbb_granger_pose_idle_left"));
    assert_eq!(
        h.entities[&1]
            .buffs
            .iter()
            .filter(|b| b.name().starts_with("mlbb_granger_pose_"))
            .count(),
        1
    );
}
#[test]
fn visual_failed_emission_and_replayed_hit_produce_no_extra_effects() {
    let mut h = Host::new();
    h.start(1);
    h.projectile_ok = false;
    h.cast(1, Op::Basic, 2);
    assert!(h.view_calls.is_empty());
    h.projectile_ok = true;
    h.step(1);
    h.cast(1, Op::Basic, 2);
    assert_eq!(h.view_calls.len(), 1);
    assert_eq!(h.view_calls[0].0, "mlbb_granger_fx_gun_muzzle");
    h.land(0, 2);
    assert_eq!(h.view_calls.len(), 2);
    h.land(0, 2);
    assert_eq!(h.view_calls.len(), 2);
}
#[test]
fn visual_cannon_aoe_has_one_primary_explosion() {
    let mut h = Host::new();
    h.add(3, 1, (252000, 200000));
    h.start(1);
    h.cast(1, Op::Cannon, 2);
    h.land(0, 2);
    assert_eq!(h.damage.len(), 2);
    assert_eq!(
        h.view_calls
            .iter()
            .filter(|(n, _)| n == "mlbb_granger_fx_cannon_impact")
            .count(),
        1
    );
    assert!(h.has(1, "mlbb_granger_pose_cannon"));
}
#[test]
fn visual_presim_skips_cosmetics_without_changing_burst_damage() {
    let mut h = Host::new();
    h.origin_kind = SimOriginKindV1::ServerPresim.code();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(91);
    for i in 0..6 {
        h.land(i, 2);
    }
    assert_eq!(h.damage.len(), 6);
    assert!(h.view_calls.is_empty());
    assert!(h.entities[&1]
        .buffs
        .iter()
        .all(|b| !b.name().starts_with("mlbb_granger_pose_")));
}
#[test]
fn visual_death_same_tick_clears_body_and_does_not_repeat() {
    let mut h = Host::new();
    let mut p = h.start(1);
    h.entities.get_mut(&1).unwrap().hp = 0;
    h.sim(|s| p.on_dead(s, 1));
    h.sim(|s| p.on_dead(s, 1));
    h.step(10);
    assert_eq!(
        h.view_calls
            .iter()
            .filter(|(n, _)| n == "mlbb_granger_fx_dead")
            .count(),
        1
    );
    assert!(h.entities[&1]
        .buffs
        .iter()
        .all(|b| !b.name().starts_with("mlbb_granger_pose_")));
    h.entities.get_mut(&1).unwrap().hp = 660;
    h.sim(|s| p.on_spawn(s, 1, 1));
    assert!(h.has(1, "mlbb_granger_pose_idle"));
}
#[test]
fn visual_concurrent_burst_preserves_dash_body_then_roll_faces_target() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.cast(1, Op::Rondo, 2);
    h.step(1);
    assert!(h.has(1, "mlbb_granger_pose_rondo"));
    assert_eq!(h.projectiles.len(), 1);
    h.step(20);
    h.cast(1, Op::Cannon, 2);
    h.step(12);
    assert!(h.has(1, "mlbb_granger_pose_roll"));
}

#[test]
fn failed_spawn_never_consumes_granger_intro_and_presim_never_advances_voice_state() {
    let mut h = Host::new();
    h.start(1);
    h.step(60);
    h.projectile_ok = false;
    h.cast(1, Op::Basic, 2);
    assert!(!h.state(1).voice.introduced);
    h.step(1);
    h.projectile_ok = true;
    h.cast(1, Op::Basic, 2);
    assert!(h.state(1).voice.introduced);
    let mut headless = Host::new();
    headless.origin_kind = SimOriginKindV1::ServerPresim.code();
    headless.start(1);
    headless.step(60);
    headless.cast(1, Op::Cannon, 2);
    assert!(!headless.state(1).voice.introduced);
    assert_eq!(headless.state(1).voice.next_skill, 0);
}

#[test]
fn buffed_level_five_skills_land_full_damage_without_changing_energy_costs() {
    let mut h = Host::new();
    h.entities.get_mut(&1).unwrap().stat.attack = 104;
    h.start(1);
    h.cast(1, Op::Burst, 2);
    h.step(91);
    assert_eq!(h.projectiles.len(), 6);
    assert_eq!(h.state(1).gun.current, 476);
    for i in 0..6 {
        h.land(i, 2);
    }
    assert_eq!(
        h.damage.iter().map(|d| d.2).collect::<Vec<_>>(),
        vec![82; 6]
    );
    assert_eq!(h.crit_samples, vec![0, 0, 0, 0, 0, 100]);
    h.damage.clear();
    h.cast(1, Op::Cannon, 2);
    assert_eq!(h.state(1).cannon.current, 550);
    assert!(h.damage.is_empty());
    h.land(6, 2);
    h.land(6, 2);
    assert_eq!(h.damage.iter().map(|d| d.2).collect::<Vec<_>>(), vec![269]);
}

#[test]
fn presentation_updates_never_apply_self_animation_crowd_control() {
    let mut h = Host::new();
    h.start(1);
    h.step(180);
    h.cast(1, Op::Burst, 2);
    h.step(91);
    h.cast(1, Op::Rondo, 2);
    h.step(60);
    assert!(h.entities[&1].cc.is_empty());
    assert_eq!(h.projectiles.len(), 6);
    assert!(h.entities[&1].targetable);
}
