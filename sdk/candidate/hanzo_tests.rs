//! ABI/control-flow tests only. This mock's undying semantics are NOT engine evidence.
use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};
static SEED: AtomicU64 = AtomicU64::new(91_000_000);
struct Unit {
    p: Pos,
    hp: usize,
    base: StatV1,
    buffs: Vec<BuffV1>,
    alive: bool,
    team: usize,
    kind: EntityKindV1,
    shield: usize,
}
pub(crate) struct Host {
    pub(crate) queued: Vec<(String, usize, InputTargetV1, usize)>,
    pub(crate) knockback_allowed: bool,
    pub(crate) record_queued_effects: bool,
    pub(crate) now: usize,
    pub(crate) respawn_ticks: usize,
    seed: u64,
    units: HashMap<usize, Unit>,
    kills: usize,
    heals: Vec<usize>,
    moves: Vec<Pos>,
    banishes: usize,
    ended: bool,
    pub(crate) typed: Vec<u32>,
    hits: Vec<(usize, usize, u32, usize)>,
    pub(crate) cc: Vec<(usize, CcV1)>,
    pub(crate) pull_allowed: bool,
    pub(crate) pulls: Vec<(usize, usize, usize)>,
    skill_cycle: usize,
    origin_kind: u32,
    seen: bool,
    allow_skill: bool,
    allow_dash: bool,
    allow_ult: bool,
    pub(crate) effects: Vec<(String, Pos, u64)>,
    native_attack_range: usize,
    basic_metadata: bool,
    deferred_damage: bool,
    attack_remaining: usize,
    pending_damage: Vec<(usize, usize)>,
}
impl Host {
    pub(crate) fn new() -> Self {
        let mut h = Self {
            queued: vec![],
            knockback_allowed: false,
            record_queued_effects: false,
            now: 0,
            respawn_ticks: 0,
            seed: SEED.fetch_add(1, Ordering::Relaxed),
            units: HashMap::new(),
            kills: 0,
            heals: vec![],
            moves: vec![],
            banishes: 0,
            ended: false,
            typed: vec![],
            hits: vec![],
            cc: vec![],
            pull_allowed: false,
            pulls: vec![],
            skill_cycle: 30,
            origin_kind: 0,
            seen: true,
            allow_skill: true,
            allow_dash: true,
            allow_ult: true,
            effects: vec![],
            native_attack_range: 18000,
            basic_metadata: true,
            deferred_damage: false,
            attack_remaining: 0,
            pending_damage: vec![],
        };
        for (id, team, p) in [(1, 0, (200000, 200000)), (2, 1, (215000, 200000))] {
            h.units.insert(
                id,
                Unit {
                    p,
                    hp: 700,
                    base: StatV1 {
                        hp: 700,
                        attack: 110,
                        move_speed: 1080,
                        ..Default::default()
                    },
                    buffs: vec![],
                    alive: true,
                    team,
                    kind: EntityKindV1::Champion,
                    shield: 0,
                },
            );
        }
        h
    }
    fn table() -> SimVtableV1 {
        let mut t: SimVtableV1 = unsafe { std::mem::zeroed() };
        t.size = size_of::<SimVtableV1>();
        t.tick = Some(tick);
        t.seed = Some(seed);
        t.is_end = Some(ended);
        t.entity_is_valid = Some(valid);
        t.entity_hp = Some(hp);
        t.entity_shield = Some(shield);
        t.apply_cc = Some(cc);
        t.entity_stat = Some(stat);
        t.entity_pos = Some(pos);
        t.entity_set_pos = Some(set_pos);
        t.entity_set_hp = Some(set_hp);
        t.entity_is_alive = Some(alive);
        t.entity_is_champion = Some(champion);
        t.entity_is_tower = Some(is_tower);
        t.entity_is_targetable = Some(alive);
        t.entity_team = Some(team);
        t.entity_level = Some(level);
        t.entity_radius = Some(radius);
        t.sim_origin = Some(origin);
        t.play_view_effect = Some(play_effect);
        t.player_is_valid = Some(player_valid);
        t.player_champion = Some(player_champion);
        t.player_respawn_time = Some(player_respawn_time);
        t.entity_cooldown = Some(cooldown);
        t.entity_kind = Some(kind);
        t.entity_buff_count = Some(buff_count);
        t.entity_buff_at = Some(buff_at);
        t.add_buff = Some(add_buff);
        t.entity_remove_buff = Some(remove_buff);
        t.entity_banish = Some(banish);
        t.entity_pull = Some(pull);
        t.queue_effect = Some(record_queue);
        t.entity_knockback = Some(allow_knockback);
        t.heal = Some(heal);
        t.deal_damage = Some(damage);
        t.deal_damage_typed = Some(typed_damage);
        t.entity_count = Some(count);
        t.entity_at = Some(at);
        t.is_visible = Some(visible);
        t.champion_count = Some(count);
        t.champion_id_at = Some(champ_id);
        t
    }
    pub(crate) fn sim<T>(&mut self, f: impl FnOnce(&mut StableSim<'_>) -> T) -> T {
        let t = Self::table();
        let mut c = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            state: self as *mut _ as *mut c_void,
            sim: &t,
            frame: std::ptr::null(),
        };
        let mut s = unsafe { StableSim::from_raw(&mut c) }.unwrap();
        f(&mut s)
    }
    pub(crate) fn set_unit_position(&mut self, id: usize, p: (u64, u64)) {
        self.units.get_mut(&id).unwrap().p = p;
    }

    pub(crate) fn set_unit_kind(&mut self, id: usize, kind: EntityKindV1) {
        self.units.get_mut(&id).unwrap().kind = kind;
    }

    pub(crate) fn set_unit_max_hp(&mut self, id: usize, hp: usize) {
        self.units.get_mut(&id).unwrap().base.hp = hp;
    }

    pub(crate) fn respawn_unit(&mut self, id: usize, position: Pos) {
        let unit = self.units.get_mut(&id).unwrap();
        unit.alive = true;
        unit.hp = max(unit);
        unit.p = position;
    }

    fn ai(&mut self, base: Option<InputV1>) -> Option<InputV1> {
        self.think_with(&mut Ai, base)
    }

    pub(crate) fn think_with(
        &mut self,
        ai: &mut impl StablePlayerAi,
        base: Option<InputV1>,
    ) -> Option<InputV1> {
        let sim = Self::table();
        let mut a: AiVtableV1 = unsafe { std::mem::zeroed() };
        a.size = size_of::<AiVtableV1>();
        a.player_id = Some(ai_player);
        a.is_valid_input = Some(ai_valid);
        let ptr = self as *mut _ as *mut c_void;
        let mut raw = AiCtxV1 {
            size: size_of::<AiCtxV1>(),
            vtable: &a,
            state: ptr,
            sim: &sim,
            sim_state: ptr,
        };
        let mut ctx = unsafe { StableAiContext::from_raw(&mut raw) }.unwrap();
        ai.think(&mut ctx, base)
    }
    fn setup(&mut self) -> (Passive, EntityKey) {
        let mut p = Passive::default();
        let k = self.sim(|s| {
            p.on_spawn(s, 1, 1);
            let k = crate::entity_key(s, 1);
            let mut state = take(k).unwrap();
            state.grid = Some(Arc::new(open_grid()));
            put(k, state);
            k
        });
        (p, k)
    }
    fn enter(&mut self) -> (Passive, EntityKey) {
        let (p, k) = self.setup();
        self.sim(|sim| {
            let mut s = take(k).unwrap();
            assert!(enter(sim, 1, &mut s));
            s.grid = Some(Arc::new(open_grid()));
            put(k, s)
        });
        (p, k)
    }
    fn tick(&mut self, k: EntityKey) {
        self.now += 1;
        self.sim(|sim| update(sim, k));
    }
}
fn open_grid() -> crate::Grid {
    crate::Grid {
        cells: vec![vec![0; 30]; 30],
        bounds: None,
    }
}

#[test]
fn human_and_spirit_presentation_never_apply_self_animation_crowd_control() {
    let mut h = Host::new();
    let (_, k) = h.setup();
    for _ in 0..10 {
        h.tick(k);
    }
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        assert!(enter(sim, 1, &mut s));
        visual::action(&mut s, "spirit_attack", sim.tick(), 18);
        pose(sim, 1, &mut s);
        put(k, s);
    });
    for _ in 0..20 {
        h.tick(k);
    }
    assert!(h.cc.iter().all(|(owner, _)| *owner != 1));
}
unsafe fn h<'a>(s: *const c_void) -> &'a mut Host {
    &mut *(s as *mut Host)
}
fn id(e: EntityHandleV1) -> usize {
    e.id().unwrap()
}
fn max(u: &Unit) -> usize {
    let flat: i32 = u.buffs.iter().map(|b| b.hp).sum();
    let mult: i32 = u.buffs.iter().map(|b| b.hp_mult).sum();
    ((u.base.hp as i64 + flat as i64).max(1) * (100 + mult).max(1) as i64 / 100).max(1) as usize
}
unsafe extern "C" fn tick(s: *const c_void) -> usize {
    h(s).now
}
unsafe extern "C" fn seed(s: *const c_void) -> u64 {
    h(s).seed
}
unsafe extern "C" fn ended(s: *const c_void) -> bool {
    h(s).ended
}
unsafe extern "C" fn valid(s: *const c_void, e: EntityHandleV1) -> bool {
    h(s).units.contains_key(&id(e))
}
unsafe extern "C" fn hp(s: *const c_void, e: EntityHandleV1, a: *mut usize, b: *mut usize) -> bool {
    let u = &h(s).units[&id(e)];
    *a = u.hp;
    *b = max(u);
    true
}
unsafe extern "C" fn stat(s: *const c_void, e: EntityHandleV1, out: *mut StatV1) -> bool {
    let u = &h(s).units[&id(e)];
    *out = u.base;
    (*out).crit_chance = (u.base.crit_chance as i64
        + u.buffs.iter().map(|b| b.crit_chance as i64).sum::<i64>())
    .max(0) as usize;
    true
}
unsafe extern "C" fn pos(s: *const c_void, e: EntityHandleV1, x: *mut u64, y: *mut u64) -> bool {
    let p = h(s).units[&id(e)].p;
    *x = p.0;
    *y = p.1;
    true
}
unsafe extern "C" fn set_pos(s: *mut c_void, e: EntityHandleV1, x: u64, y: u64) -> bool {
    h(s).units.get_mut(&id(e)).unwrap().p = (x, y);
    h(s).moves.push((x, y));
    true
}
unsafe extern "C" fn set_hp(s: *mut c_void, e: EntityHandleV1, n: usize) -> bool {
    let u = h(s).units.get_mut(&id(e)).unwrap();
    u.hp = n.min(max(u));
    true
}
unsafe extern "C" fn alive(s: *const c_void, e: EntityHandleV1) -> bool {
    h(s).units[&id(e)].alive
}
unsafe extern "C" fn champion(s: *const c_void, e: EntityHandleV1) -> bool {
    h(s).units[&id(e)].kind == EntityKindV1::Champion
}
unsafe extern "C" fn team(s: *const c_void, e: EntityHandleV1) -> usize {
    h(s).units[&id(e)].team
}
unsafe extern "C" fn level(_: *const c_void, _: EntityHandleV1) -> usize {
    5
}
unsafe extern "C" fn kind(s: *const c_void, e: EntityHandleV1) -> u32 {
    h(s).units[&id(e)].kind.code()
}
unsafe extern "C" fn buff_count(s: *const c_void, e: EntityHandleV1) -> usize {
    h(s).units[&id(e)].buffs.len()
}
unsafe extern "C" fn buff_at(
    s: *const c_void,
    e: EntityHandleV1,
    i: usize,
    out: *mut BuffV1,
) -> bool {
    if let Some(b) = h(s).units[&id(e)].buffs.get(i) {
        *out = *b;
        true
    } else {
        false
    }
}
unsafe extern "C" fn add_buff(s: *mut c_void, e: usize, b: *const BuffV1) {
    h(s).units.get_mut(&e).unwrap().buffs.push(*b);
}
unsafe extern "C" fn remove_buff(
    s: *mut c_void,
    e: EntityHandleV1,
    p: *const u8,
    n: usize,
) -> usize {
    let name = std::str::from_utf8(std::slice::from_raw_parts(p, n)).unwrap();
    let u = h(s).units.get_mut(&id(e)).unwrap();
    let before = u.buffs.len();
    u.buffs.retain(|b| b.name() != name);
    before - u.buffs.len()
}
unsafe extern "C" fn banish(
    s: *mut c_void,
    _: usize,
    _: EntityHandleV1,
    _: usize,
    _: *const u8,
    _: usize,
    _: *const u8,
    _: usize,
) -> bool {
    h(s).banishes += 1;
    true
}
unsafe extern "C" fn heal(s: *mut c_void, _: usize, e: usize, n: usize) {
    h(s).heals.push(n);
    let u = h(s).units.get_mut(&e).unwrap();
    u.hp = max(u).min(u.hp + n);
}
unsafe extern "C" fn damage(s: *mut c_void, c: usize, e: usize, ad: usize, ap: usize, attack: u32) {
    if h(s).deferred_damage {
        h(s).hits.push((e, ad + ap, attack, 0));
        h(s).pending_damage.push((e, ad + ap));
        return;
    }
    let u = &h(s).units[&c];
    let crit = (u.base.crit_chance as i64
        + u.buffs.iter().map(|b| b.crit_chance as i64).sum::<i64>())
    .max(0) as usize;
    h(s).hits.push((e, ad + ap, attack, crit));
    hit(h(s), e, ad + ap);
}
unsafe extern "C" fn shield(s: *const c_void, e: EntityHandleV1) -> usize {
    h(s).units[&id(e)].shield
}
unsafe extern "C" fn record_queue(
    s: *mut c_void, name: *const u8, len: usize, _: u32, caster: usize,
    input: *const InputTargetV1, delay: usize,
) -> bool {
    let Ok(name) = std::str::from_utf8(std::slice::from_raw_parts(name, len)) else {
        return false;
    };
    if !h(s).record_queued_effects { return false; }
    h(s).queued.push((name.to_owned(), caster, *input, delay));
    true
}

unsafe extern "C" fn allow_knockback(
    s: *mut c_void, _: usize, _: EntityHandleV1, _: usize, _: usize,
) -> bool {
    h(s).knockback_allowed
}

unsafe extern "C" fn cc(s: *mut c_void, e: usize, c: *const CcV1) {
    h(s).cc.push((e, *c));
}
unsafe extern "C" fn typed_damage(
    s: *mut c_void,
    _: usize,
    e: usize,
    n: usize,
    t: u32,
    _: u32,
) -> bool {
    h(s).typed.push(t);
    if h(s).deferred_damage {
        h(s).pending_damage.push((e, n));
    } else {
        hit(h(s), e, n);
    }
    true
}
unsafe extern "C" fn pull(
    state: *mut c_void,
    caster: usize,
    target: EntityHandleV1,
    _: usize,
    ticks: usize,
) -> bool {
    let host = h(state);
    let Some(target) = target.id() else { return false; };
    host.pulls.push((caster, target, ticks));
    host.pull_allowed
}
fn hit(h: &mut Host, id: usize, n: usize) {
    let u = h.units.get_mut(&id).unwrap();
    let absorbed = n.min(u.shield);
    u.shield -= absorbed;
    let n = n - absorbed;
    if n >= u.hp {
        if u.buffs.iter().any(|b| b.undying) {
            u.hp = 1;
        } else {
            u.hp = 0;
            u.alive = false;
            h.kills += 1;
        }
    } else {
        u.hp -= n;
    }
}
unsafe extern "C" fn count(s: *const c_void) -> usize {
    h(s).units.len()
}
unsafe extern "C" fn at(s: *const c_void, i: usize) -> EntityHandleV1 {
    let mut ids: Vec<_> = h(s).units.keys().copied().collect();
    ids.sort();
    ids.get(i)
        .map_or(EntityHandleV1::NULL, |id| EntityHandleV1::from_id(*id))
}
unsafe extern "C" fn champ_id(_: *const c_void, i: usize) -> usize {
    i + 1
}
unsafe extern "C" fn visible(s: *const c_void, _: usize, _: usize) -> bool {
    h(s).seen
}
fn state(k: EntityKey) -> State {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&k)
        .unwrap()
        .clone()
}

#[test]
fn projection_stores_body_and_caps_spirit_without_duplicate_unit() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().hp = 600;
    let (_, k) = h.enter();
    assert_eq!(state(k).stored_hp, 600);
    assert_eq!(h.units[&1].hp, 490);
    assert_eq!(h.units.len(), 2);
    assert!(h.units[&1].buffs.iter().any(|b| b.undying && b.ignore_wall));
}
#[test]
fn recall_restores_saved_pool_not_spirit_healing() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().hp = 350;
    let (_, k) = h.enter();
    h.units.get_mut(&1).unwrap().hp = 490;
    h.units.get_mut(&1).unwrap().p = (400000, 400000);
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        begin_return(sim, 1, &mut s, Exit::Recall);
        put(k, s)
    });
    for _ in 0..18 {
        h.tick(k);
    }
    assert_eq!(h.units[&1].p, (200000, 200000));
    assert_eq!(h.units[&1].hp, 440);
    assert_eq!(state(k).form, Form::Human);
    assert!(h.units[&1]
        .buffs
        .iter()
        .all(|b| !b.undying && !b.ignore_wall));
    assert_eq!(h.heals, vec![90]);
}
#[test]
fn modeled_lethal_physical_magic_and_fixed_force_return_without_kill_then_human_can_die() {
    for damage_type in [DamageTypeV1::Ad, DamageTypeV1::Ap, DamageTypeV1::Fixed] {
        let mut h = Host::new();
        let (mut p, k) = h.enter();
        h.sim(|s| s.deal_damage_typed(2, 1, 99999, damage_type, AttackTypeV1::Skill));
        assert_eq!(h.units[&1].hp, 1);
        h.sim(|s| p.on_damaged(s, 1, 1, 2, 99999));
        for _ in 0..19 {
            h.tick(k);
        }
        assert_eq!(h.kills, 0);
        assert_eq!(state(k).returns, 1);
        assert!(h.units[&1].buffs.iter().any(|b| b.name() == PENALTY));
        assert_eq!(state(k).ready[2], 3121);
        h.sim(|s| s.deal_damage_typed(2, 1, 99999, damage_type, AttackTypeV1::Skill));
        assert_eq!(h.kills, 1);
        assert!(!h.units[&1].alive);
    }
}
#[test]
fn duplicate_tick_does_not_drain_energy_or_finish_twice() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.tick(k);
    let a = state(k).form;
    for _ in 0..100 {
        h.sim(|s| update(s, k));
    }
    assert_eq!(state(k).form, a);
    assert_eq!(h.heals.len(), 0);
}
#[test]
fn exhaustion_draws_body_for_one_second_and_merges_at_spirit() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.units.get_mut(&1).unwrap().p = (300000, 200000);
    for _ in 0..600 {
        h.tick(k);
    }
    assert!(matches!(state(k).form, Form::Drag { .. }));
    for _ in 0..30 {
        h.tick(k);
    }
    assert_eq!(state(k).body, (250000, 200000));
    for _ in 0..48 {
        h.tick(k);
    }
    assert_eq!(state(k).form, Form::Human);
    assert_eq!(h.units[&1].p, (300000, 200000));
    assert_eq!(state(k).returns, 1);
}
#[test]
fn forced_return_from_wall_lands_on_confirmed_ground_before_flags_clear() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    let mut g = open_grid();
    g.cells[10][10] = 1;
    let goal = (336000, 336000);
    h.units.get_mut(&1).unwrap().p = goal;
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        s.grid = Some(Arc::new(g.clone()));
        begin_return(sim, 1, &mut s, Exit::Exhausted);
        put(k, s)
    });
    for _ in 0..18 {
        h.tick(k);
    }
    assert!(standable(&g, h.units[&1].p));
    assert_ne!(h.units[&1].p, goal);
    assert!(h.units[&1].buffs.iter().all(|b| !b.ignore_wall));
}
#[test]
fn safe_dash_checks_real_walls_and_phased_dash_stays_on_map() {
    let mut g = open_grid();
    g.cells[6][7] = 1;
    let start = (200000, 208000);
    let human = legal_step(&g, start, (1., 0.), 80000., false);
    let spirit = legal_step(&g, start, (1., 0.), 80000., true);
    assert!(human.0 < 224000);
    assert_eq!(spirit.0, 280000);
    assert!(legal_step(&g, (950000, 200000), (1., 0.), 100000., true).0 < 960000);
}
#[test]
fn same_entity_ids_in_two_sim_copies_keep_separate_health_and_forms() {
    let mut a = Host::new();
    let (_, ka) = a.enter();
    let mut b = Host::new();
    let (_, kb) = b.setup();
    assert_ne!(ka, kb);
    assert!(state(ka).form.projected());
    assert_eq!(state(kb).form, Form::Human);
    b.tick(kb);
    assert!(state(ka).form.projected());
}
#[test]
fn respawn_clears_protection_and_owned_pools() {
    let mut h = Host::new();
    let (mut p, k) = h.enter();
    h.sim(|s| p.on_dead(s, 1));
    assert!(state(k).blocked_architecture);
    h.sim(|s| p.on_spawn(s, 1, 1));
    assert_eq!(state(k).stored_hp, 0);
    assert_eq!(state(k).form, Form::Human);
    assert!(state(k).blocked_architecture);
    assert!(h.units[&1]
        .buffs
        .iter()
        .all(|b| !b.undying && !b.ignore_wall));
}
#[test]
fn failed_native_interception_never_resurrects_real_dead_champion() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.units.get_mut(&1).unwrap().hp = 0;
    h.units.get_mut(&1).unwrap().alive = false;
    h.tick(k);
    assert!(state(k).blocked_architecture);
    assert_eq!(h.units[&1].hp, 0);
    assert!(h.heals.is_empty());
    assert!(h.units[&1].buffs.iter().all(|b| !b.undying));
}
#[test]
fn other_max_hp_buffs_survive_and_saved_pool_clamps() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().buffs.push(BuffV1 {
        hp: 300,
        ..BuffV1::named("equipment")
    });
    h.units.get_mut(&1).unwrap().hp = 950;
    let (_, k) = h.enter();
    h.units.get_mut(&1).unwrap().base.hp = 500;
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        finish(sim, 1, &mut s, (200000, 200000), Exit::Recall);
        put(k, s)
    });
    assert_eq!(h.units[&1].hp, 800);
    assert!(h.units[&1].buffs.iter().any(|b| b.name() == "equipment"));
}
#[test]
fn terminal_tick_removes_only_owned_protection() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.ended = true;
    h.tick(k);
    assert!(!STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains_key(&k));
    assert!(h.units[&1].buffs.iter().all(|b| !b.undying));
}
#[test]
fn hard_cap_commits_drag_even_if_energy_remains() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    let mut s = take(k).unwrap();
    s.form = Form::Spirit {
        since: 0,
        energy: 999999,
        carry: 0,
    };
    put(k, s);
    h.now = 900;
    h.sim(|s| update(s, k));
    assert!(matches!(state(k).form, Form::Drag { .. }));
}
#[test]
fn repeat_return_calls_never_duplicate_heal_or_cooldown() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        begin_return(sim, 1, &mut s, Exit::Recall);
        begin_return(sim, 1, &mut s, Exit::Recall);
        put(k, s)
    });
    assert_eq!(h.banishes, 1);
    for _ in 0..30 {
        h.tick(k);
    }
    assert_eq!(h.heals.len(), 1);
    assert_eq!(state(k).returns, 1);
}

// Full-kit packet/cadence regressions; mock mitigation remains deliberately simple.
fn five(h: &mut Host, k: EntityKey, target: usize) {
    h.sim(|sim| {
        let t = crate::entity_key(sim, target);
        let mut s = take(k).unwrap();
        s.marks.insert(t, (5, sim.tick() + 360));
        put(k, s);
    });
}
fn energy(k: EntityKey) -> usize {
    if let Form::Spirit { energy, .. } = state(k).form {
        energy
    } else {
        0
    }
}
fn set_energy(k: EntityKey, n: usize) {
    let mut s = take(k).unwrap();
    if let Form::Spirit { energy, .. } = &mut s.form {
        *energy = n;
    }
    put(k, s);
}
#[test]
fn full_ordinary_jungle_devour_uses_credited_damage_then_four_digest_heals() {
    let mut h = Host::new();
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Jungle;
    h.units.get_mut(&1).unwrap().hp = 300;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert!(!h.units[&2].alive);
    assert_eq!(h.kills, 1);
    assert_eq!(h.typed, vec![DamageTypeV1::Fixed.code()]);
    assert_eq!(state(k).skill_ready[0][0], 720);
    for _ in 0..120 {
        h.tick(k);
    }
    assert_eq!(h.heals, vec![35; 4]);
    assert_eq!(h.units[&1].hp, 440);
    assert!(state(k).digest.is_none());
}
#[test]
fn full_devour_never_executes_minions_pets_or_structures() {
    for kind_ in [
        EntityKindV1::Minion,
        EntityKindV1::Summon,
        EntityKindV1::Tower,
        EntityKindV1::Nexus,
    ] {
        let mut h = Host::new();
        h.units.get_mut(&2).unwrap().kind = kind_;
        let (_, k) = h.setup();
        five(&mut h, k, 2);
        h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
        assert_eq!(h.units[&2].hp, 700);
        assert!(h.typed.is_empty());
        assert!(h.heals.is_empty());
    }
}
#[test]
fn full_human_feast_formula_refund_and_consumption() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().base.attack = 150;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert_eq!(h.hits.len(), 1);
    assert_eq!(h.hits[0].1, 257);
    assert_eq!(h.heals, vec![107]);
    assert_eq!(state(k).skill_ready[0][0], 288);
    assert!(state(k).marks.is_empty());
}
#[test]
fn full_soul_snatch_lunge_single_packet_and_five_mark_payoff() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().base.attack = 150;
    h.units.get_mut(&2).unwrap().base.hp = 1000;
    h.units.get_mut(&2).unwrap().hp = 1000;
    h.units.get_mut(&2).unwrap().p = (260000, 200000);
    let (_, k) = h.enter();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert!(h.hits.is_empty());
    for _ in 0..15 {
        h.tick(k);
    }
    assert_eq!(h.hits.len(), 1);
    assert_eq!(h.hits[0].1, 464);
    assert_eq!(h.hits[0].2, AttackTypeV1::Skill.code());
    assert_eq!(h.units[&2].hp, 536);
    assert_eq!(h.heals, vec![107]);
    assert!(state(k).marks.is_empty());
}
#[test]
fn full_soul_snatch_boss_component_caps_at220() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().base.attack = 150;
    let t = h.units.get_mut(&2).unwrap();
    t.kind = EntityKindV1::Epic;
    t.base.hp = 100000;
    t.hp = 100000;
    let (_, k) = h.enter();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    for _ in 0..15 {
        h.tick(k);
    }
    assert_eq!(h.hits.len(), 1);
    assert_eq!(h.hits[0].1, 586);
}
#[test]
fn full_human_mist_exactly_five_noncrit_basic_packets_and_one_slow() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().base.attack = 150;
    h.units.get_mut(&1).unwrap().base.crit_chance = 80;
    let (_, k) = h.setup();
    h.sim(|sim| Op::Dash.apply(sim, 0, 1, InputTargetV1::dir(100000, 0)));
    for _ in 0..180 {
        h.tick(k);
    }
    assert_eq!(h.hits.len(), 5);
    assert!(h
        .hits
        .iter()
        .all(|hit| hit.1 == 33 && hit.2 == AttackTypeV1::BaseAttack.code() && hit.3 == 0));
    assert_eq!(h.units[&1].base.crit_chance, 80);
    assert!(h.units[&1]
        .buffs
        .iter()
        .all(|b| b.name() != "mlbb_hanzo_mist_no_crit"));
    assert_eq!(
        h.units[&2]
            .buffs
            .iter()
            .filter(|b| b.name().starts_with("mlbb_hanzo_mist_slow_"))
            .count(),
        1
    );
    assert!(state(k).zones.is_empty());
}
#[test]
fn full_spirit_mist_path_hits_once_airborne_once_refund_once_and_four_pulses() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().base.attack = 150;
    let (_, k) = h.enter();
    set_energy(k, 500);
    h.sim(|sim| Op::Dash.apply(sim, 0, 1, InputTargetV1::dir(100000, 0)));
    for _ in 0..15 {
        h.tick(k);
    }
    assert_eq!(energy(k), 675);
    assert_eq!(h.cc.len(), 1);
    assert_eq!(h.cc[0].1.kind, CcKindV1::Airborne.code());
    assert_eq!(h.cc[0].1.tick, 24);
    for _ in 0..130 {
        h.tick(k);
    }
    assert_eq!(
        h.hits
            .iter()
            .filter(|x| x.2 == AttackTypeV1::Skill.code())
            .count(),
        1
    );
    assert_eq!(
        h.hits
            .iter()
            .filter(|x| x.2 == AttackTypeV1::BaseAttack.code())
            .count(),
        4
    );
    assert_eq!(h.units[&2].hp, 404);
}
#[test]
fn full_spirit_basic_energy_requires_real_contact_and_cannot_restart_drag() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    set_energy(k, 400);
    h.sim(|sim| Op::Basic.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert_eq!(energy(k), 415);
    h.now += 1;
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Epic;
    h.sim(|sim| Op::Basic.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert_eq!(energy(k), 460);
    let mut s = take(k).unwrap();
    s.form = Form::Drag {
        since: 1,
        from: s.body,
    };
    combat::gain(&mut s, 900);
    assert!(matches!(s.form, Form::Drag { .. }));
    put(k, s);
}
#[test]
fn full_nearby_death_energy_receipts_do_not_repeat() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    set_energy(k, 400);
    hit(&mut h, 2, 99999);
    for _ in 0..15 {
        h.tick(k);
    }
    assert_eq!(energy(k), 575);
    for _ in 0..15 {
        h.tick(k);
    }
    assert_eq!(energy(k), 550);
}
#[test]
fn full_return_cancels_spirit_damage_and_pending_snatch() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.sim(|sim| Op::Dash.apply(sim, 0, 1, InputTargetV1::dir(100000, 0)));
    for _ in 0..15 {
        h.tick(k);
    }
    let before = h.hits.len();
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        begin_return(sim, 1, &mut s, Exit::Recall);
        put(k, s)
    });
    for _ in 0..180 {
        h.tick(k);
    }
    assert_eq!(h.hits.len(), before);
    assert!(state(k).snatch.is_none());
    assert!(state(k).zones.is_empty());
}
#[test]
fn full_each_form_keeps_its_own_skill_cooldown() {
    let mut h = Host::new();
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert_eq!(state(k).skill_ready[0][0], 288);
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        assert!(enter(sim, 1, &mut s));
        put(k, s)
    });
    assert_eq!(state(k).skill_ready[1][0], 0);
    h.now = 1;
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert_eq!(state(k).skill_ready[1][0], 601);
    assert_eq!(state(k).skill_ready[0][0], 288);
}
#[test]
fn full_digest_stops_on_owner_death() {
    let mut h = Host::new();
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Jungle;
    let (mut p, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    h.units.get_mut(&1).unwrap().alive = false;
    h.units.get_mut(&1).unwrap().hp = 0;
    h.sim(|sim| p.on_dead(sim, 1));
    for _ in 0..150 {
        h.tick(k);
    }
    assert!(h.heals.is_empty());
    assert!(state(k).digest.is_none());
}
unsafe extern "C" fn cooldown(
    s: *const c_void,
    _: EntityHandleV1,
    slot: u32,
    out: *mut CooldownV1,
) -> bool {
    if slot == SkillSlotV1::Attack.code() && !h(s).basic_metadata {
        return false;
    }
    *out = CooldownV1 {
        cooltime: h(s).skill_cycle,
        remaining: if slot == SkillSlotV1::Attack.code() {
            h(s).attack_remaining
        } else {
            0
        },
        charges: 1,
        max_charges: 1,
        unlocked: true,
        ..Default::default()
    };
    true
}
#[test]
fn full_skill_clocks_follow_native_cooldown_reduction() {
    let mut h = Host::new();
    h.skill_cycle = 15;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert_eq!(state(k).skill_ready[0][0], 144);
    h.sim(|sim| Op::Dash.apply(sim, 0, 1, InputTargetV1::dir(100000, 0)));
    assert_eq!(state(k).skill_ready[0][1], 300);
}
#[test]
fn full_credited_death_and_nearby_poll_share_one_energy_receipt() {
    let mut h = Host::new();
    let (mut passive, k) = h.enter();
    set_energy(k, 300);
    hit(&mut h, 2, 99999);
    h.sim(|sim| passive.on_kill(sim, 1, 1, 2));
    assert_eq!(energy(k), 500);
    h.sim(|sim| passive.on_kill(sim, 1, 1, 2));
    assert_eq!(energy(k), 500);
    for _ in 0..15 {
        h.tick(k);
    }
    assert_eq!(energy(k), 475);
}

unsafe extern "C" fn radius(_: *const c_void, _: EntityHandleV1) -> usize {
    10000
}
unsafe extern "C" fn origin(s: *const c_void, out: *mut SimOriginV1) -> bool {
    *out = SimOriginV1 {
        kind: h(s).origin_kind,
        ..Default::default()
    };
    true
}
unsafe extern "C" fn player_valid(_: *const c_void, _: PlayerHandleV1) -> bool {
    true
}
unsafe extern "C" fn player_respawn_time(s: *const c_void, _: PlayerHandleV1) -> usize {
    h(s).respawn_ticks
}
unsafe extern "C" fn player_champion(_: *const c_void, _: PlayerHandleV1) -> EntityHandleV1 {
    EntityHandleV1::from_id(1)
}
unsafe extern "C" fn play_effect(
    s: *mut c_void,
    p: *const u8,
    n: usize,
    _: usize,
    input: *const InputTargetV1,
    _: u64,
    _: u64,
    time: u64,
) -> bool {
    let name = std::str::from_utf8(std::slice::from_raw_parts(p, n))
        .unwrap()
        .to_string();
    h(s).effects.push((name, ((*input).x, (*input).y), time));
    true
}
unsafe extern "C" fn ai_player(_: *const c_void) -> usize {
    1
}
unsafe extern "C" fn ai_valid(s: *const c_void, p: *const InputV1) -> bool {
    let i = *p;
    let h = h(s);
    match InputKindV1::from_code(i.kind) {
        Some(InputKindV1::Ult) => h.allow_ult,
        Some(InputKindV1::Skill2) => h.allow_dash,
        Some(InputKindV1::Move | InputKindV1::Return) => true,
        Some(InputKindV1::Skill) if i.target.target_id == 0 => h.allow_skill,
        Some(InputKindV1::Attack | InputKindV1::Skill) => {
            let Some(t) = h.units.get(&i.target.target_id) else {
                return false;
            };
            let a = &h.units[&1];
            t.alive
                && t.team != a.team
                && (distance(a.p, t.p) - 20000.).max(0.)
                    <= if i.kind == InputKindV1::Attack.code() {
                        h.native_attack_range as f64
                    } else if h.allow_skill {
                        75000.
                    } else {
                        return false;
                    }
        }
        _ => false,
    }
}
#[test]
fn repair_basic_reports_positive_native_expected_damage() {
    let stat = StatV1 {
        attack: 151,
        ..Default::default()
    };
    assert_eq!(Op::Basic.expected_damage(&stat), (151, 0));
}
#[test]
fn repair_native_melee_center35000_basic_hits_both_forms() {
    for spirit in [false, true] {
        let mut h = Host::new();
        h.units.get_mut(&2).unwrap().p = (235000, 200000);
        let (_, k) = if spirit { h.enter() } else { h.setup() };
        h.sim(|s| Op::Basic.apply(s, 0, 1, InputTargetV1::target(2)));
        assert_eq!(h.hits.len(), 1);
        assert_eq!(state(k).marks.values().next().unwrap().0, 1);
    }
}
#[test]
fn repair153_no_builtin_input_preserves_ongoing_action() {
    for spirit in [false, true] {
        let mut h = Host::new();
        if spirit {
            h.enter();
        } else {
            h.setup();
        }
        assert!(h.ai(None).is_none());
        assert!(h.hits.is_empty());
    }
}

#[test]
fn repair_invalid_s1_proposal_falls_through_to_valid_basic() {
    let mut h = Host::new();
    h.allow_skill = false;
    h.allow_dash = false;
    h.allow_ult = false;
    h.units.get_mut(&2).unwrap().p = (235000, 200000);
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    let input = h.ai(Some(base)).unwrap_or(base);
    assert_eq!(input.kind, InputKindV1::Attack.code());
}
#[test]
fn repair_five_edge_range_basics_unlock_and_resolve_human_s1() {
    let mut h = Host::new();
    h.allow_dash = false;
    h.allow_ult = false;
    h.units.get_mut(&2).unwrap().base.hp = 3000;
    h.units.get_mut(&2).unwrap().hp = 3000;
    h.units.get_mut(&2).unwrap().p = (235000, 200000);
    let (_, k) = h.setup();
    for tick in 0..5 {
        h.now = tick * 54;
        let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
        let i = h.ai(Some(base)).unwrap_or(base);
        assert_eq!(i.kind, InputKindV1::Attack.code());
        h.sim(|s| Op::Basic.apply(s, 0, 1, i.target));
    }
    h.now = 235;
    let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    let i = h.ai(Some(base)).unwrap_or(base);
    assert_eq!(i.kind, InputKindV1::Skill.code());
    h.sim(|s| Op::Feast.apply(s, 0, 1, i.target));
    assert_eq!(h.hits.len(), 6);
    assert!(state(k).marks.is_empty());
    assert!(!h.heals.is_empty());
}
#[test]
fn repair_s1_selects_marked_camp_instead_of_unmarked_far_champion() {
    let mut h = Host::new();
    h.allow_dash = false;
    h.allow_ult = false;
    let mut camp = Unit {
        p: (235000, 200000),
        hp: 400,
        base: StatV1 {
            hp: 400,
            ..Default::default()
        },
        buffs: vec![],
        alive: true,
        team: 1,
        kind: EntityKindV1::Jungle,
        shield: 0,
    };
    camp.buffs.clear();
    h.units.insert(3, camp);
    h.units.get_mut(&2).unwrap().p = (285000, 200000);
    let (_, k) = h.setup();
    five(&mut h, k, 3);
    let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(3));
    let i = h.ai(Some(base)).unwrap_or(base);
    assert_eq!(i.kind, InputKindV1::Skill.code());
    assert_eq!(i.target.target_id, 3);
    h.sim(|s| Op::Feast.apply(s, 0, 1, i.target));
    assert!(!h.units[&3].alive);
}
#[test]
fn repair_resolved_basic_does_not_double_reject_native_visibility() {
    let mut h = Host::new();
    h.seen = false;
    let (_, k) = h.setup();
    let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    let i = h.ai(Some(base)).unwrap_or(base);
    assert_eq!(i.kind, InputKindV1::Attack.code());
    h.sim(|s| Op::Basic.apply(s, 0, 1, i.target));
    assert_eq!(h.hits.len(), 1);
    assert_eq!(state(k).marks.values().next().unwrap().0, 1);
}
#[test]
fn repair_native_body_link_and_marks_emit_without_client_drawing_map() {
    let mut h = Host::new();
    h.origin_kind = SimOriginKindV1::ClientMatchView.code();
    let (_, k) = h.enter();
    h.units.get_mut(&1).unwrap().p = (350000, 200000);
    five(&mut h, k, 2);
    h.sim(|sim| update(sim, k));
    assert!(h
        .effects
        .iter()
        .any(|(n, _, _)| n.starts_with("mlbb_hanzo_overlay_human_project_")));
    assert!(h
        .effects
        .iter()
        .any(|(n, _, _)| n.starts_with("mlbb_hanzo_overlay_link_")));
    assert!(h
        .effects
        .iter()
        .any(|(n, _, _)| n == "mlbb_hanzo_overlay_mark_5"));
    let before = h.effects.len();
    for _ in 0..20 {
        h.sim(|sim| update(sim, k));
    }
    assert_eq!(h.effects.len(), before);
}
#[test]
fn repair_unknown_origin_native_overlays_follow_working_skill_fx_policy() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.sim(|sim| update(sim, k));
    assert!(h
        .effects
        .iter()
        .any(|(n, _, _)| n.starts_with("mlbb_hanzo_overlay_")));
}
#[test]
fn repair_presim_emits_no_native_overlay_work() {
    let mut h = Host::new();
    h.origin_kind = SimOriginKindV1::ServerPresim.code();
    let (_, k) = h.enter();
    for _ in 0..20 {
        h.tick(k);
    }
    assert!(h.effects.is_empty());
}
#[test]
fn repair_return_stops_overlay_stamps_with_bounded_lifetime() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.units.get_mut(&1).unwrap().p = (350000, 200000);
    h.tick(k);
    let before = h
        .effects
        .iter()
        .filter(|(n, _, _)| n.starts_with("mlbb_hanzo_overlay_"))
        .count();
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        finish(sim, 1, &mut s, (200000, 200000), Exit::Recall);
        put(k, s)
    });
    for _ in 0..30 {
        h.tick(k);
    }
    assert_eq!(
        h.effects
            .iter()
            .filter(|(n, _, _)| n.starts_with("mlbb_hanzo_overlay_"))
            .count(),
        before
    );
    assert!(h
        .effects
        .iter()
        .filter(|(n, _, _)| n.starts_with("mlbb_hanzo_overlay_"))
        .all(|(_, _, ttl)| *ttl <= 12));
}

#[test]
fn repair153_host_reach_rejection_never_creates_movement() {
    let mut h = Host::new();
    h.allow_skill = false;
    h.allow_dash = false;
    h.allow_ult = false;
    h.native_attack_range = 12000;
    h.units.get_mut(&2).unwrap().p = (235000, 200000);
    h.setup();
    assert!(h
        .ai(Some(InputV1::action(
            InputKindV1::Skill,
            InputTargetV1::target(2)
        )))
        .is_none());
}

#[test]
fn repair_basic_does_not_require_optional_attack_charge_metadata() {
    let mut h = Host::new();
    h.basic_metadata = false;
    h.allow_dash = false;
    h.allow_ult = false;
    h.units.get_mut(&2).unwrap().p = (235000, 200000);
    h.setup();
    let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    let i = h.ai(Some(base)).unwrap_or(base);
    assert_eq!(i.kind, InputKindV1::Attack.code());
}

#[test]
fn repair152_attack_lock_preserves_facing_and_pose_despite_position_noise() {
    for spirit in [false, true] {
        let mut h = Host::new();
        let (_, k) = if spirit { h.enter() } else { h.setup() };
        h.sim(|s| Op::Basic.apply(s, 0, 1, InputTargetV1::target(2)));
        let initial = state(k);
        let pose = initial.pose;
        let left = initial.left;
        for dx in [3000, -3500, 3200, -3100] {
            h.units.get_mut(&1).unwrap().p.0 = (200000i64 + dx) as u64;
            h.tick(k);
            h.sim(|sim| Passive::default().on_damaged(sim, 0, 1, 2, 10));
            assert_eq!(state(k).left, left);
            assert_eq!(state(k).pose, pose);
            assert!(h.ai(Some(InputV1::move_to(100000, 200000))).is_none());
        }
    }
}
#[test]
fn repair152_damage_without_inline_receipt_still_builds_basic_marks() {
    let mut h = Host::new();
    h.deferred_damage = true;
    let (_, k) = h.setup();
    for tick in [0, 54, 108, 162, 216] {
        h.now = tick;
        h.sim(|s| Op::Basic.apply(s, 0, 1, InputTargetV1::target(2)));
    }
    assert_eq!(h.units[&2].hp, 700);
    assert_eq!(h.hits.len(), 5);
    assert_eq!(state(k).marks.values().next().unwrap().0, 5);
}
#[test]
fn repair153_never_creates_chase_or_replaces_lane_movement() {
    let mut h = Host::new();
    h.units.get_mut(&2).unwrap().p = (320000, 200000);
    h.setup();
    assert!(h.ai(None).is_none());
    assert!(h.ai(Some(InputV1::move_to(200000, 400000))).is_none());
}

#[test]
fn repair153_native_return_and_retreat_always_pass_through() {
    let mut h = Host::new();
    h.setup();
    assert!(h.ai(Some(InputV1::return_home())).is_none());
    assert!(h.ai(Some(InputV1::move_to(100000, 200000))).is_none());
}

#[test]
fn repair153_low_health_does_not_force_recall_or_stop() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().hp = 150;
    h.setup();
    for base in [
        None,
        Some(InputV1::return_home()),
        Some(InputV1::move_to(180000, 200000)),
    ] {
        assert!(h.ai(base).is_none());
    }
}

pub(crate) fn add_unit(h: &mut Host, id: usize, kind: EntityKindV1, team: usize, p: Pos) {
    h.units.insert(
        id,
        Unit {
            p,
            hp: 700,
            base: StatV1 {
                hp: 700,
                ..Default::default()
            },
            buffs: vec![],
            alive: true,
            team,
            kind,
            shield: 0,
        },
    );
}
#[test]
fn repair153_native_objective_plan_has_no_custom_escape_or_hold() {
    for kind in [EntityKindV1::Epic, EntityKindV1::Serpen] {
        let mut h = Host::new();
        h.units.get_mut(&2).unwrap().kind = kind;
        h.setup();
        for base in [
            InputV1::action(InputKindV1::Attack, InputTargetV1::target(2)),
            InputV1::move_to(215000, 200000),
        ] {
            assert!(h.ai(Some(base)).is_none());
        }
    }
}

#[test]
fn repair152_supported_healthy_objective_can_be_attacked() {
    let mut h = Host::new();
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Epic;
    add_unit(&mut h, 3, EntityKindV1::Champion, 0, (200000, 220000));
    add_unit(&mut h, 4, EntityKindV1::Champion, 0, (200000, 180000));
    h.setup();
    let attack = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    assert_eq!(h.ai(Some(attack)).unwrap_or(attack), attack);
}
#[test]
fn repair153_native_pathfinding_near_towers_is_not_overridden() {
    let mut h = Host::new();
    add_unit(&mut h, 3, EntityKindV1::Tower, 1, (260000, 200000));
    h.setup();
    for goal in [(260000, 200000), (130000, 200000), (200000, 200000)] {
        assert!(h.ai(Some(InputV1::move_to(goal.0, goal.1))).is_none());
    }
}

#[test]
fn repair152_native_basic_focus_beats_nearby_alternate_target() {
    let mut h = Host::new();
    h.allow_ult = false;
    h.allow_dash = false;
    add_unit(&mut h, 3, EntityKindV1::Jungle, 1, (220000, 200000));
    h.setup();
    let attack = InputV1::action(InputKindV1::Attack, InputTargetV1::target(3));
    assert_eq!(h.ai(Some(attack)).unwrap_or(attack), attack);
}
#[test]
fn repair152_real_cadence_basic_chain_devours_ordinary_camp() {
    let mut h = Host::new();
    h.allow_ult = false;
    h.allow_dash = false;
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Jungle;
    h.units.get_mut(&2).unwrap().hp = 3000;
    h.units.get_mut(&2).unwrap().base.hp = 3000;
    let (_, k) = h.setup();
    for tick in [0, 54, 108, 162, 216] {
        h.now = tick;
        let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
        let i = h.ai(Some(base)).unwrap_or(base);
        assert_eq!(i.kind, InputKindV1::Attack.code());
        h.sim(|s| Op::Basic.apply(s, 0, 1, i.target));
        h.now = tick + 6;
        h.sim(|s| update(s, k));
        assert!(h
            .effects
            .iter()
            .any(|(n, p, _)| n.starts_with("mlbb_hanzo_overlay_mark_") && *p == h.units[&2].p));
    }
    h.now = 235;
    let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    let i = h.ai(Some(base)).unwrap_or(base);
    assert_eq!(i.kind, InputKindV1::Skill.code());
    h.sim(|s| Op::Feast.apply(s, 0, 1, i.target));
    assert!(!h.units[&2].alive);
    assert!(state(k).digest.is_some());
    for _ in 0..130 {
        h.tick(k);
    }
    assert_eq!(h.heals.len(), 4);
}

unsafe extern "C" fn is_tower(s: *const c_void, e: EntityHandleV1) -> bool {
    h(s).units[&id(e)].kind == EntityKindV1::Tower
}

#[test]
fn repair153_attack_cooldown_never_generates_stationary_move() {
    let mut h = Host::new();
    h.attack_remaining = 20;
    h.setup();
    assert!(h.ai(Some(InputV1::move_to(240000, 200000))).is_none());
    assert!(h.ai(None).is_none());
}

#[test]
fn repair152_dead_or_friendly_target_never_gains_marks() {
    for friendly in [false, true] {
        let mut h = Host::new();
        if friendly {
            h.units.get_mut(&2).unwrap().team = 0;
        } else {
            h.units.get_mut(&2).unwrap().alive = false;
        }
        let (_, k) = h.setup();
        h.sim(|sim| Op::Basic.apply(sim, 0, 1, InputTargetV1::target(2)));
        assert!(state(k).marks.is_empty());
        assert!(h.hits.is_empty());
    }
}

#[test]
fn repair153_objective_spawn_and_recall_sequence_are_native_for_300_ticks() {
    let mut h = Host::new();
    let (_, k) = h.setup();
    for now in 0..300 {
        h.now = now;
        h.units.get_mut(&2).unwrap().kind = if now < 100 {
            EntityKindV1::Jungle
        } else {
            EntityKindV1::Epic
        };
        let base = match now % 3 {
            0 => Some(InputV1::move_to(215000, 200000)),
            1 => Some(InputV1::return_home()),
            _ => None,
        };
        assert!(h.ai(base).is_none());
        h.sim(|sim| update(sim, k));
    }
}

fn flush_packets(h: &mut Host) {
    for (target, amount) in std::mem::take(&mut h.pending_damage) {
        let alive = h.units[&target].alive;
        hit(h, target, amount);
        if alive && !h.units[&target].alive {
            h.sim(|sim| Passive::default().on_unit_kill(sim, 1, 1, target));
        }
    }
}
#[test]
fn repair153_delayed_devour_plays_bite_then_four_heals_once_after_kill_receipt() {
    let mut h = Host::new();
    h.deferred_damage = true;
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Jungle;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert!(h.units[&2].alive);
    assert!(state(k).digest.is_none());
    assert!(state(k).feast.is_some());
    assert_eq!(
        h.effects
            .iter()
            .filter(|(n, _, _)| n == "mlbb_hanzo_fx_feast_bite")
            .count(),
        1
    );
    // Reads can stay unchanged through multiple callbacks without resubmitting damage.
    for _ in 0..3 {
        h.tick(k);
    }
    assert!(h.heals.is_empty());
    assert_eq!(h.pending_damage.len(), 1);
    flush_packets(&mut h);
    assert!(!h.units[&2].alive);
    // The credited receipt still resolves when the engine has removed the monster.
    h.units.remove(&2);
    h.tick(k);
    assert!(state(k).feast.is_none());
    assert!(state(k).digest.is_some());
    for _ in 0..150 {
        h.tick(k);
    }
    assert_eq!(h.heals.len(), 4);
    assert_eq!(h.typed.len(), 1);
}
#[test]
fn repair153_delayed_devour_can_confirm_from_next_tick_death_without_callback() {
    let mut h = Host::new();
    h.deferred_damage = true;
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Jungle;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    for (id, n) in std::mem::take(&mut h.pending_damage) {
        hit(&mut h, id, n);
    }
    h.tick(k);
    assert!(state(k).digest.is_some());
}
#[test]
fn repair153_delayed_human_burst_waits_for_damage_then_heals_and_refunds_once() {
    let mut h = Host::new();
    h.deferred_damage = true;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    assert!(h.heals.is_empty());
    assert_eq!(state(k).skill_ready[0][0], 720);
    h.tick(k);
    assert!(h.heals.is_empty());
    flush_packets(&mut h);
    h.tick(k);
    assert_eq!(h.heals.len(), 1);
    assert_eq!(state(k).skill_ready[0][0], 288);
    assert!(state(k).marks.is_empty());
    for _ in 0..10 {
        h.tick(k);
    }
    assert_eq!(h.heals.len(), 1);
    assert_eq!(h.hits.len(), 1);
}
#[test]
fn repair153_unconfirmed_devour_expires_without_free_heals_or_repeat_damage() {
    let mut h = Host::new();
    h.deferred_damage = true;
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Jungle;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    h.pending_damage.clear();
    for _ in 0..160 {
        h.tick(k);
    }
    assert!(h.units[&2].alive);
    assert!(h.heals.is_empty());
    assert!(state(k).feast.is_none());
    assert!(state(k).digest.is_none());
    assert_eq!(h.typed.len(), 1);
}
#[test]
fn repair153_owner_death_cancels_pending_devour_reward() {
    let mut h = Host::new();
    h.deferred_damage = true;
    h.units.get_mut(&2).unwrap().kind = EntityKindV1::Jungle;
    let (mut passive, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    h.units.get_mut(&1).unwrap().alive = false;
    h.sim(|sim| passive.on_dead(sim, 1));
    flush_packets(&mut h);
    for _ in 0..160 {
        h.tick(k);
    }
    assert!(h.heals.is_empty());
    assert!(state(k).feast.is_none());
}
#[test]
fn repair153_recycled_target_does_not_inherit_feast_reward() {
    let mut h = Host::new();
    h.deferred_damage = true;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    h.pending_damage.clear();
    h.units.get_mut(&2).unwrap().buffs.clear();
    h.units.get_mut(&2).unwrap().hp = 1;
    h.sim(|sim| {
        crate::entity_key(sim, 2);
    });
    h.tick(k);
    assert!(h.heals.is_empty());
    assert!(state(k).feast.is_none());
}
#[test]
fn repair153_walk_facing_updates_at_normal_speed_in_both_directions() {
    for spirit in [false, true] {
        let mut h = Host::new();
        let (_, k) = if spirit { h.enter() } else { h.setup() };
        h.tick(k);
        for left in [true, false] {
            let x = h.units[&1].p.0;
            h.units.get_mut(&1).unwrap().p.0 = if left { x - 1084 } else { x + 1084 };
            h.tick(k);
            assert_eq!(state(k).left, left);
            assert!(state(k).visual.moving);
            assert_eq!(
                state(k).pose,
                Some(bindings::pose(
                    if spirit { "spirit_move" } else { "human_run" },
                    left
                ))
            );
        }
    }
}
#[test]
fn repair153_skill_choice_does_not_steal_another_native_target() {
    let mut h = Host::new();
    h.allow_ult = false;
    h.allow_dash = false;
    add_unit(&mut h, 3, EntityKindV1::Jungle, 1, (220000, 200000));
    let (_, k) = h.setup();
    five(&mut h, k, 3);
    let input = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    assert!(h.ai(Some(input)).is_none());
}

#[test]
fn repair154_credited_champion_kill_settles_burst_after_victim_removal_once() {
    let mut h = Host::new();
    h.deferred_damage = true;
    h.units.get_mut(&2).unwrap().hp = 100;
    let (_, k) = h.setup();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    for (id, n) in std::mem::take(&mut h.pending_damage) {
        hit(&mut h, id, n);
    }
    h.sim(|sim| Passive::default().on_kill(sim, 1, 1, 2));
    h.units.remove(&2);
    h.tick(k);
    assert_eq!(h.heals.len(), 1);
    assert_eq!(state(k).skill_ready[0][0], 288);
    assert!(state(k).feast.is_none());
    for _ in 0..60 {
        h.tick(k);
    }
    assert_eq!(h.heals.len(), 1);
}

#[test]
fn vfx155_trail_pulses_add_one_mark_with_deferred_damage_in_both_forms() {
    for spirit in [false, true] {
        let mut h = Host::new();
        h.deferred_damage = true;
        let (_, k) = if spirit { h.enter() } else { h.setup() };
        h.sim(|sim| {
            let mut s = take(k).unwrap();
            s.zones.push(combat::Zone {
                from: (200000, 200000),
                to: (240000, 200000),
                next: 30,
                remaining: if spirit { 4 } else { 5 },
                spirit,
                damage: 20,
            });
            put(k, s);
        });
        let count = if spirit { 4 } else { 5 };
        for n in 1..=count {
            h.now = n * 30;
            h.sim(|sim| update(sim, k));
            assert_eq!(state(k).marks.values().next().unwrap().0, n);
            assert_eq!(h.pending_damage.len(), n);
            assert_eq!(h.units[&2].hp, 700);
            h.sim(|sim| update(sim, k));
            assert_eq!(state(k).marks.values().next().unwrap().0, n);
        }
        assert!(state(k).zones.is_empty());
    }
}
#[test]
fn vfx155_trail_marks_are_not_doubled_with_immediate_damage() {
    let mut h = Host::new();
    let (_, k) = h.setup();
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        s.zones.push(combat::Zone {
            from: (200000, 200000),
            to: (240000, 200000),
            next: 0,
            remaining: 1,
            spirit: false,
            damage: 20,
        });
        combat::tick(sim, 1, &mut s);
        put(k, s);
    });
    assert_eq!(state(k).marks.values().next().unwrap().0, 1);
    assert_eq!(h.hits.len(), 1);
}
#[test]
fn vfx155_trail_never_marks_outside_area_allies_or_structures() {
    let mut h = Host::new();
    h.deferred_damage = true;
    h.units.get_mut(&2).unwrap().p = (215000, 250000);
    add_unit(&mut h, 3, EntityKindV1::Champion, 0, (220000, 200000));
    add_unit(&mut h, 4, EntityKindV1::Tower, 1, (220000, 200000));
    let (_, k) = h.setup();
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        s.zones.push(combat::Zone {
            from: (200000, 200000),
            to: (240000, 200000),
            next: 0,
            remaining: 1,
            spirit: false,
            damage: 20,
        });
        combat::tick(sim, 1, &mut s);
        put(k, s);
    });
    assert!(state(k).marks.is_empty());
    assert!(h.pending_damage.is_empty());
}
#[test]
fn vfx155_ward_stamp_uses_body_foot_anchor_without_map_displacement() {
    let mut h = Host::new();
    let (_, k) = h.enter();
    h.tick(k);
    let body = state(k).body;
    assert!(h
        .effects
        .iter()
        .any(|(name, p, _)| name.starts_with("mlbb_hanzo_overlay_hanzo_body_ward_") && *p == body));
}

#[test]
fn polish156_spirit_maw_plays_before_deferred_damage_and_confirms_five_marks_once() {
    let mut h = Host::new();
    h.deferred_damage = true;
    h.allow_ult = false;
    h.allow_dash = false;
    h.units.get_mut(&1).unwrap().base.attack = 150;
    h.units.get_mut(&2).unwrap().base.hp = 1000;
    h.units.get_mut(&2).unwrap().hp = 1000;
    let (_, k) = h.enter();
    five(&mut h, k, 2);
    let base = InputV1::action(InputKindV1::Attack, InputTargetV1::target(2));
    let input = h.ai(Some(base)).unwrap();
    assert_eq!(input.kind, InputKindV1::Skill.code());
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, input.target));
    for _ in 0..15 {
        h.tick(k);
    }
    assert_eq!(h.units[&2].hp, 1000);
    assert_eq!(h.pending_damage.len(), 1);
    assert_eq!(
        h.effects
            .iter()
            .filter(|(n, _, _)| n == "mlbb_hanzo_fx_spirit_maw")
            .count(),
        1
    );
    assert_eq!(state(k).marks.values().next().unwrap().0, 5);
    assert!(h.heals.is_empty());
    flush_packets(&mut h);
    h.tick(k);
    assert_eq!(h.units[&2].hp, 536);
    assert!(state(k).marks.is_empty());
    assert_eq!(h.heals.len(), 1);
    assert_eq!(state(k).skill_ready[0][0], 0);
    assert_eq!(state(k).skill_ready[1][0], 600);
    for _ in 0..20 {
        h.tick(k);
    }
    assert_eq!(h.heals.len(), 1);
    assert_eq!(h.hits.len(), 1);
}
#[test]
fn polish156_spirit_maw_without_five_marks_has_no_free_heal_or_human_refund() {
    let mut h = Host::new();
    h.deferred_damage = true;
    let (_, k) = h.enter();
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    for _ in 0..15 {
        h.tick(k);
    }
    assert!(h
        .effects
        .iter()
        .any(|(n, _, _)| n == "mlbb_hanzo_fx_spirit_maw"));
    flush_packets(&mut h);
    h.tick(k);
    assert!(h.heals.is_empty());
    assert_eq!(state(k).skill_ready[0][0], 0);
    assert_eq!(state(k).skill_ready[1][0], 600);
}
#[test]
fn polish156_spirit_receipt_cannot_heal_after_projection_return() {
    let mut h = Host::new();
    h.deferred_damage = true;
    let (_, k) = h.enter();
    five(&mut h, k, 2);
    h.sim(|sim| Op::Feast.apply(sim, 0, 1, InputTargetV1::target(2)));
    for _ in 0..15 {
        h.tick(k);
    }
    h.sim(|sim| {
        let mut s = take(k).unwrap();
        finish(sim, 1, &mut s, (200000, 200000), Exit::Recall);
        put(k, s);
    });
    let heals = h.heals.len();
    flush_packets(&mut h);
    h.tick(k);
    assert_eq!(h.heals.len(), heals);
    assert!(state(k).feast.is_none());
}
#[test]
fn polish156_base_attack_buff_does_not_count_as_bonus_attack() {
    let mut h = Host::new();
    h.units.get_mut(&1).unwrap().base.attack = 124;
    h.setup();
    assert_eq!(h.sim(|sim| bonus(sim, 1)), 0);
    h.units.get_mut(&1).unwrap().base.attack = 150;
    assert_eq!(h.sim(|sim| bonus(sim, 1)), 26);
}
