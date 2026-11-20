//! Callback regression tests with an ABI host, including a host that refuses
//! all movement of banished units. These do not claim in-game verification.
use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};
static SEED: AtomicU64 = AtomicU64::new(100_000);
#[derive(Clone)]
struct Entity {
    team: usize,
    p: Pos,
    hp: usize,
    buffs: Vec<BuffV1>,
    banished_until: usize,
    shield: usize,
}
struct Host {
    now: usize,
    seed: u64,
    e: HashMap<usize, Entity>,
    damage: Vec<(usize, usize, usize)>,
    heals: Vec<(usize, usize)>,
    roots: Vec<(usize, u64)>,
    views: Vec<(String, InputTargetV1)>,
    shield_calls: usize,
    ult_unlocked: bool,
    ult_charges: usize,
    ai_valid: bool,
    sets: Vec<(usize, Pos)>,
}
impl Host {
    fn new() -> Self {
        let mut h = Self {
            now: 0,
            seed: SEED.fetch_add(1, Ordering::SeqCst),
            e: HashMap::new(),
            damage: vec![],
            heals: vec![],
            roots: vec![],
            views: vec![],
            shield_calls: 0,
            ult_unlocked: true,
            ult_charges: 1,
            ai_valid: true,
            sets: vec![],
        };
        for (id, team, p) in [
            (1, 0, (200_000, 200_000)),
            (2, 1, (250_000, 200_000)),
            (3, 0, (240_000, 200_000)),
            (4, 0, (210_000, 200_000)),
        ] {
            h.e.insert(
                id,
                Entity {
                    team,
                    p,
                    hp: 700,
                    buffs: vec![],
                    banished_until: 0,
                    shield: 0,
                },
            );
        }
        h
    }
    fn sim<T>(&mut self, f: impl FnOnce(&mut StableSim<'_>) -> T) -> T {
        let t = Self::table();
        let mut ctx = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            sim: &t,
            frame: std::ptr::null(),
            state: self as *mut _ as *mut c_void,
        };
        let mut sim = unsafe { StableSim::from_raw(&mut ctx).unwrap() };
        f(&mut sim)
    }
    fn table() -> SimVtableV1 {
        let mut t: SimVtableV1 = unsafe { std::mem::zeroed() };
        t.size = size_of::<SimVtableV1>();
        t.tick = Some(tick);
        t.seed = Some(seed);
        t.entity_count = Some(count);
        t.entity_at = Some(at);
        t.entity_is_valid = Some(valid);
        t.entity_pos = Some(pos);
        t.entity_stat = Some(stat);
        t.entity_hp = Some(hp);
        t.entity_team = Some(team);
        t.entity_is_alive = Some(alive);
        t.entity_is_champion = Some(champion);
        t.entity_is_targetable = Some(targetable);
        t.entity_buff_count = Some(buff_count);
        t.entity_buff_at = Some(buff_at);
        t.entity_stack_buff = Some(stack);
        t.add_buff = Some(add);
        t.entity_remove_buff = Some(remove);
        t.deal_damage_typed = Some(damage);
        t.heal = Some(heal);
        t.apply_cc = Some(cc);
        t.entity_add_shield = Some(shield);
        t.entity_shield = Some(shield_value);
        t.entity_banish = Some(banish);
        t.entity_set_pos = Some(set_pos);
        t.play_view_effect = Some(view);
        t.sim_origin = Some(sim_origin);
        t.entity_cooldown = Some(cooldown);
        t.player_is_valid = Some(player_valid);
        t.player_champion = Some(player_champion);
        t
    }
    fn ai(&mut self, base: Option<InputV1>) -> Option<InputV1> {
        let sim = Self::table();
        let mut table: AiVtableV1 = unsafe { std::mem::zeroed() };
        table.size = size_of::<AiVtableV1>();
        table.player_id = Some(ai_player);
        table.tick = Some(tick);
        table.is_valid_input = Some(ai_valid);
        let mut raw = AiCtxV1 {
            size: size_of::<AiCtxV1>(),
            vtable: &table,
            state: self as *mut _ as *mut c_void,
            sim: &sim,
            sim_state: self as *mut _ as *mut c_void,
        };
        let mut context = unsafe { StableAiContext::from_raw(&mut raw).unwrap() };
        AngelaAi.think(&mut context, base)
    }
    fn step(&mut self, n: usize) {
        for _ in 0..n {
            self.now += 1;
            self.sim(tick_all);
        }
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
unsafe extern "C" fn count(p: *const c_void) -> usize {
    h(p).e.len()
}
unsafe extern "C" fn at(p: *const c_void, i: usize) -> EntityHandleV1 {
    let mut ids: Vec<_> = h(p).e.keys().copied().collect();
    ids.sort();
    ids.get(i)
        .map_or(EntityHandleV1::NULL, |id| EntityHandleV1::from_id(*id))
}
unsafe extern "C" fn valid(p: *const c_void, id: EntityHandleV1) -> bool {
    id.id().is_some_and(|id| h(p).e.contains_key(&id))
}
unsafe extern "C" fn pos(p: *const c_void, id: EntityHandleV1, x: *mut u64, y: *mut u64) -> bool {
    if let Some(e) = id.id().and_then(|id| h(p).e.get(&id)) {
        *x = e.p.0;
        *y = e.p.1;
        true
    } else {
        false
    }
}
unsafe extern "C" fn stat(_: *const c_void, _: EntityHandleV1, out: *mut StatV1) -> bool {
    *out = StatV1 {
        magic_power: 58,
        ..StatV1::default()
    };
    true
}
unsafe extern "C" fn hp(
    p: *const c_void,
    id: EntityHandleV1,
    v: *mut usize,
    max: *mut usize,
) -> bool {
    *v = h(p).e[&id.id().unwrap()].hp;
    *max = 700;
    true
}
unsafe extern "C" fn team(p: *const c_void, id: EntityHandleV1) -> usize {
    h(p).e[&id.id().unwrap()].team
}
unsafe extern "C" fn alive(p: *const c_void, id: EntityHandleV1) -> bool {
    h(p).e[&id.id().unwrap()].hp > 0
}
unsafe extern "C" fn champion(_: *const c_void, _: EntityHandleV1) -> bool {
    true
}
unsafe extern "C" fn targetable(p: *const c_void, id: EntityHandleV1) -> bool {
    h(p).e[&id.id().unwrap()].banished_until <= h(p).now
}
unsafe extern "C" fn buff_count(p: *const c_void, id: EntityHandleV1) -> usize {
    h(p).e[&id.id().unwrap()].buffs.len()
}
unsafe extern "C" fn buff_at(
    p: *const c_void,
    id: EntityHandleV1,
    i: usize,
    out: *mut BuffV1,
) -> bool {
    if let Some(b) = h(p).e[&id.id().unwrap()].buffs.get(i) {
        *out = *b;
        true
    } else {
        false
    }
}
unsafe extern "C" fn stack(
    p: *mut c_void,
    id: EntityHandleV1,
    b: *const BuffV1,
    cap: usize,
    refresh: u32,
) -> usize {
    let e = m(p).e.get_mut(&id.id().unwrap()).unwrap();
    let count = e.buffs.iter().filter(|x| x.name() == (*b).name()).count();
    if refresh != 0 {
        for x in &mut e.buffs {
            if x.name() == (*b).name() {
                x.duration_tick = (*b).duration_tick;
            }
        }
    }
    if cap == 0 || count < cap {
        e.buffs.push(*b);
        count + 1
    } else {
        count
    }
}
unsafe extern "C" fn add(p: *mut c_void, id: usize, b: *const BuffV1) {
    m(p).e.get_mut(&id).unwrap().buffs.push(*b);
}
unsafe extern "C" fn remove(
    p: *mut c_void,
    id: EntityHandleV1,
    name: *const u8,
    len: usize,
) -> usize {
    let name = std::str::from_utf8(std::slice::from_raw_parts(name, len)).unwrap();
    let Some(e) = m(p).e.get_mut(&id.id().unwrap()) else {
        return 0;
    };
    let n = e.buffs.len();
    e.buffs.retain(|b| b.name() != name);
    n - e.buffs.len()
}
unsafe extern "C" fn damage(p: *mut c_void, c: usize, t: usize, n: usize, _: u32, _: u32) -> bool {
    m(p).damage.push((c, t, n));
    true
}
unsafe extern "C" fn heal(p: *mut c_void, _: usize, t: usize, n: usize) {
    m(p).heals.push((t, n));
}
unsafe extern "C" fn cc(p: *mut c_void, t: usize, c: *const CcV1) {
    assert_eq!((*c).kind, CcKindV1::Bind.code());
    m(p).roots.push((t, (*c).tick));
}
unsafe extern "C" fn shield(p: *mut c_void, t: EntityHandleV1, n: usize, _: usize) -> bool {
    m(p).shield_calls += 1;
    m(p).e.get_mut(&t.id().unwrap()).unwrap().shield += n;
    true
}
unsafe extern "C" fn shield_value(p: *const c_void, t: EntityHandleV1) -> usize {
    h(p).e[&t.id().unwrap()].shield
}
unsafe extern "C" fn banish(
    p: *mut c_void,
    _: usize,
    t: EntityHandleV1,
    n: usize,
    _: *const u8,
    _: usize,
    _: *const u8,
    _: usize,
) -> bool {
    let until = m(p).now + n;
    m(p).e.get_mut(&t.id().unwrap()).unwrap().banished_until = until;
    true
}
unsafe extern "C" fn set_pos(p: *mut c_void, t: EntityHandleV1, x: u64, y: u64) -> bool {
    let id = t.id().unwrap();
    if !targetable(p, t) {
        return false;
    }
    m(p).e.get_mut(&id).unwrap().p = (x, y);
    m(p).sets.push((id, (x, y)));
    true
}
unsafe extern "C" fn view(
    p: *mut c_void,
    name: *const u8,
    len: usize,
    _: usize,
    input: *const InputTargetV1,
    _: u64,
    _: u64,
    _: u64,
) -> bool {
    m(p).views.push((
        String::from_utf8_lossy(std::slice::from_raw_parts(name, len)).to_string(),
        *input,
    ));
    true
}
unsafe extern "C" fn sim_origin(_: *const c_void, out: *mut SimOriginV1) -> bool {
    *out = SimOriginV1 {
        kind: SimOriginKindV1::ClientMatchView.code(),
        ..SimOriginV1::default()
    };
    true
}

#[test]
fn s2_callback_roots_and_detonates_through_the_match_tick_without_a_second_passive() {
    let mut h = Host::new();
    h.sim(|s| AngelaPuppetString.apply(s, 0, 1, InputTargetV1::target(2)));
    assert_eq!(h.damage, [(1, 2, 109)]);
    h.step(179);
    assert!(h.roots.is_empty());
    assert!(h.e[&2]
        .buffs
        .iter()
        .any(|b| b.name() == slow_name(1) && b.move_speed_mult == -44));
    h.step(1);
    assert_eq!(h.roots, [(2, 42)]);
    assert_eq!(h.damage, [(1, 2, 109), (1, 2, 160)]);
    h.step(5);
    assert_eq!(h.roots.len(), 1);
    assert!(h.e[&2].buffs.iter().all(|b| b.name() != string_name(1)));
}
#[test]
fn broken_or_purged_string_never_roots_or_deals_final_damage() {
    for purge in [false, true] {
        let mut h = Host::new();
        h.sim(|s| AngelaPuppetString.apply(s, 0, 1, InputTargetV1::target(2)));
        if purge {
            h.e.get_mut(&2).unwrap().buffs.clear();
        } else {
            h.e.get_mut(&2).unwrap().p = (900_000, 200_000);
        }
        h.step(181);
        assert!(h.roots.is_empty());
        assert_eq!(h.damage.len(), 1);
    }
}
#[test]
fn simultaneous_angela_strings_are_source_owned() {
    let mut h = Host::new();
    h.sim(|s| {
        AngelaPuppetString.apply(s, 0, 1, InputTargetV1::target(2));
        AngelaPuppetString.apply(s, 0, 4, InputTargetV1::target(2));
    });
    h.step(180);
    assert_eq!(h.roots.len(), 2);
    assert_eq!(h.damage.iter().filter(|(_, _, n)| *n == 160).count(), 2);
}
#[test]
fn s2_rejects_an_ally_and_out_of_cast_range() {
    let mut h = Host::new();
    h.sim(|s| AngelaPuppetString.apply(s, 0, 1, InputTargetV1::target(3)));
    h.e.get_mut(&2).unwrap().p = (310_000, 200_000);
    h.sim(|s| AngelaPuppetString.apply(s, 0, 1, InputTargetV1::target(2)));
    assert!(h.damage.is_empty());
}
#[test]
fn wave_travels_heals_and_hits_once_with_mark_slow() {
    let mut h = Host::new();
    h.sim(|s| AngelaLoveWaves.apply(s, 0, 1, InputTargetV1::target(2)));
    assert!(h.damage.is_empty());
    h.step(10);
    assert!(h.damage.is_empty());
    h.step(18);
    assert_eq!(h.damage, [(1, 2, 97)]);
    assert!(h.heals.contains(&(3, 69)));
    assert!(h.e[&2]
        .buffs
        .iter()
        .any(|b| b.name() == MARK_BUFF && b.move_speed_mult == -6));
    assert!(h.views.iter().any(|(n, _)| n.starts_with("angela_wave_")));
}
#[test]
fn marks_scale_s2_final_damage_and_cap_at_five() {
    let mut h = Host::new();
    h.sim(|s| {
        for _ in 0..8 {
            apply_mark(s, 2);
        }
        AngelaPuppetString.apply(s, 0, 1, InputTargetV1::target(2));
    });
    h.step(180);
    assert_eq!(h.damage.last(), Some(&(1, 2, 256)));
    assert_eq!(
        h.e[&2]
            .buffs
            .iter()
            .filter(|b| b.name() == MARK_BUFF)
            .count(),
        5
    );
}
#[test]
fn attach_shields_once_banishes_and_casts_from_the_host_without_moving_angela() {
    let mut h = Host::new();
    let old = h.e[&1].p;
    h.e.get_mut(&3).unwrap().p = (1_000_000, 1_000_000);
    h.e.get_mut(&2).unwrap().p = (1_050_000, 1_000_000);
    h.sim(|s| AngelaHeartguard.apply(s, 0, 1, InputTargetV1::target(3)));
    assert_eq!(h.shield_calls, 1);
    assert_eq!(h.e[&3].shield, 324);
    h.step(80);
    assert_eq!(h.e[&1].p, old);
    assert!(h.e[&1].banished_until > h.now);
    assert!(h.damage.contains(&(1, 2, 109)));
    assert!(h
        .damage
        .iter()
        .any(|&(c, t, n)| c == 1 && t == 2 && n == 97));
    assert!(h.heals.contains(&(3, 69)));
    assert!(h.e[&3].buffs.iter().any(|b| b.name() == smart_name(1)));
    assert!(h.views.iter().any(|(n, _)| n == "angela_attached_doll"));
}
#[test]
fn host_death_releases_after_banish_at_the_last_host_position() {
    let mut h = Host::new();
    h.sim(|s| AngelaHeartguard.apply(s, 0, 1, InputTargetV1::target(3)));
    h.step(3);
    let p = (700_000, 800_000);
    h.e.get_mut(&3).unwrap().p = p;
    h.step(1);
    h.e.get_mut(&3).unwrap().hp = 0;
    h.step(1);
    assert!(h.sets.is_empty());
    h.step(3);
    assert_eq!(h.e[&1].p, p);
}
#[test]
fn natural_expiry_and_recast_release_without_waiting_twelve_more_seconds() {
    for recast in [false, true] {
        let mut h = Host::new();
        h.sim(|s| AngelaHeartguard.apply(s, 0, 1, InputTargetV1::target(3)));
        h.step(if recast { 5 } else { 720 });
        if recast {
            h.sim(|s| AngelaHeartguard.apply(s, 0, 1, InputTargetV1::target(3)));
        }
        h.step(3);
        assert_eq!(h.e[&1].p, h.e[&3].p);
        assert!(h.e[&1].banished_until <= h.now);
    }
}
#[test]
fn cooldown_debt_preserves_five_uses_and_regenerates_one_use_per_sixty_ticks() {
    let mut c = Clock::new(300, 5);
    for _ in 0..5 {
        assert!(c.spend());
    }
    assert!(!c.spend());
    c.elapse(59);
    assert!(!c.ready());
    c.elapse(1);
    assert!(c.spend());
    assert_eq!(c.debt, 300);
}
#[test]
fn tether_recomputes_moving_endpoints_and_stops_drawing_when_broken() {
    let mut h = Host::new();
    h.sim(|s| AngelaPuppetString.apply(s, 0, 1, InputTargetV1::target(2)));
    h.step(2);
    let first = h
        .views
        .iter()
        .find(|(n, _)| n.starts_with("angela_wire_"))
        .unwrap()
        .1;
    h.e.get_mut(&2).unwrap().p = (200_000, 260_000);
    h.views.clear();
    h.step(2);
    let next = h
        .views
        .iter()
        .find(|(n, _)| n.starts_with("angela_wire_"))
        .unwrap()
        .1;
    assert_ne!((first.x, first.y), (next.x, next.y));
    h.e.get_mut(&2).unwrap().p = (900_000, 200_000);
    h.views.clear();
    h.step(4);
    assert!(h.views.is_empty());
}
#[test]
fn duplicate_passive_and_match_updates_do_not_cast_twice_in_the_same_tick() {
    let mut h = Host::new();
    h.sim(|s| AngelaHeartguard.apply(s, 0, 1, InputTargetV1::target(3)));
    h.now = 1;
    h.sim(|s| {
        tick_runtime(s, 1);
        tick_all(s);
    });
    assert_eq!(h.damage.iter().filter(|(_, _, n)| *n == 109).count(), 1);
}
#[test]
fn smart_heart_restores_the_approved_thirty_percent_cap_and_moves_to_the_host() {
    let mut h = Host::new();
    let mut state = Runtime::new(0);
    h.sim(|sim| {
        for _ in 0..7 {
            smart(sim, 1, &state);
        }
    });
    let buffs: Vec<_> = h.e[&1]
        .buffs
        .iter()
        .filter(|b| b.name() == smart_name(1))
        .collect();
    assert_eq!(buffs.len(), 4);
    assert_eq!(buffs.iter().map(|b| b.move_speed_mult).sum::<i32>(), 30);
    assert!(buffs
        .iter()
        .all(|b| b.move_speed_mult == 7 || b.move_speed_mult == 8));
    h.sim(|sim| {
        assert!(cast_heartguard(sim, 1, 3, &mut state));
    });
    assert!(h.e[&1].buffs.iter().all(|b| b.name() != smart_name(1)));
    assert_eq!(
        h.e[&3]
            .buffs
            .iter()
            .filter(|b| b.name() == smart_name(1))
            .count(),
        4
    );
}

unsafe extern "C" fn cooldown(
    p: *const c_void,
    _: EntityHandleV1,
    slot: u32,
    out: *mut CooldownV1,
) -> bool {
    let ult = slot == SkillSlotV1::Ult.code();
    let max = if slot == SkillSlotV1::Skill.code() {
        5
    } else {
        1
    };
    *out = CooldownV1 {
        unlocked: !ult || h(p).ult_unlocked,
        charges: if ult { h(p).ult_charges } else { max },
        max_charges: max,
        cooltime: if ult {
            2400
        } else if max == 5 {
            300
        } else {
            480
        },
        remaining: if ult && h(p).ult_charges == 0 {
            2400
        } else {
            0
        },
        ..CooldownV1::default()
    };
    true
}
unsafe extern "C" fn player_valid(_: *const c_void, p: PlayerHandleV1) -> bool {
    p.id() == Some(1)
}
unsafe extern "C" fn player_champion(_: *const c_void, _: PlayerHandleV1) -> EntityHandleV1 {
    EntityHandleV1::from_id(1)
}
unsafe extern "C" fn ai_player(_: *const c_void) -> usize {
    1
}
unsafe extern "C" fn ai_valid(p: *const c_void, input: *const InputV1) -> bool {
    let h = h(p);
    h.ai_valid
        && (*input).kind == InputKindV1::Ult.code()
        && (*input).target.target_id != 1
        && h.e
            .get(&(*input).target.target_id)
            .is_some_and(|e| e.team == 0 && e.hp > 0 && e.banished_until <= h.now)
}
#[test]
fn heartguard_exposes_support_shield_value_not_offensive_damage() {
    let stat = StatV1 {
        magic_power: 58,
        ..StatV1::default()
    };
    assert_eq!(AngelaHeartguard.expected_shield(&stat), 324);
    assert_eq!(AngelaHeartguard.expected_damage(&stat), (0, 0));
}
#[test]
fn actual_ai_callback_requests_ult_even_when_builtin_ai_proposes_nothing() {
    let mut h = Host::new();
    h.now = 1;
    h.e.get_mut(&3).unwrap().hp = 250;
    let buffs = h.e.values().map(|e| e.buffs.len()).sum::<usize>();
    let input = h
        .ai(None)
        .expect("Heartguard must not depend on a base-AI ult choice");
    assert_eq!(input.kind, InputKindV1::Ult.code());
    assert_eq!(input.target.target_id, 3);
    assert_eq!(
        h.e.values().map(|e| e.buffs.len()).sum::<usize>(),
        buffs,
        "AI must be read-only"
    );
    h.sim(|sim| AngelaHeartguard.apply(sim, 0, 1, input.target));
    assert_eq!(h.shield_calls, 1);
    assert_eq!(h.e[&3].shield, 324);
}
#[test]
fn ult_selection_redirects_a_builtin_self_target_to_an_ally() {
    let mut h = Host::new();
    h.now = 1;
    let input = h
        .ai(Some(InputV1::action(
            InputKindV1::Ult,
            InputTargetV1::target(1),
        )))
        .unwrap();
    assert_eq!(input.kind, InputKindV1::Ult.code());
    assert_ne!(input.target.target_id, 1);
    assert_eq!(h.e[&input.target.target_id].team, 0);
}
#[test]
fn ult_ai_respects_level_lock_engine_cooldown_native_debt_and_engine_validity() {
    let base = Some(InputV1::action(
        InputKindV1::Attack,
        InputTargetV1::target(2),
    ));
    for gate in 0..4 {
        let mut h = Host::new();
        h.now = 1;
        match gate {
            0 => h.ult_unlocked = false,
            1 => h.ult_charges = 0,
            2 => {
                h.sim(|sim| {
                    let (k, mut s) = take(sim, 1);
                    s.ult.debt = 2400;
                    put(k, s);
                });
            }
            _ => h.ai_valid = false,
        }
        assert_eq!(h.ai(base), base, "gate {gate} must suppress the ultimate");
    }
}
#[test]
fn ult_host_selection_skips_dead_banished_enemies_and_quiet_allies() {
    let mut h = Host::new();
    h.now = 1;
    h.e.get_mut(&3).unwrap().hp = 0;
    h.e.get_mut(&4).unwrap().banished_until = 10;
    assert!(h.ai(None).is_none());
    h.e.get_mut(&3).unwrap().hp = 200;
    h.e.get_mut(&3).unwrap().p = (1_000_000, 1_000_000);
    assert!(
        h.ai(None).is_none(),
        "wounded ally far from combat should not waste shield"
    );
}
#[test]
fn speed_stat_uses_whole_percent_and_never_exceeds_thirty_after_recasts() {
    let mut h = Host::new();
    let state = Runtime::new(0);
    for expected in [7, 15, 22, 30, 30, 30] {
        h.sim(|s| smart(s, 1, &state));
        let bonus: i32 = h.e[&1]
            .buffs
            .iter()
            .filter(|b| b.name() == smart_name(1))
            .map(|b| b.move_speed_mult)
            .sum();
        assert_eq!(bonus, expected);
        assert!(1040 * (100 + bonus) / 100 <= 1352);
    }
}

#[test]
fn an_engine_valid_ult_is_not_starved_by_the_native_skill_timer() {
    let mut h = Host::new();
    h.now = 1;
    h.sim(|sim| {
        let (k, mut s) = take(sim, 1);
        s.action_until = 25;
        put(k, s);
    });
    let input = h
        .ai(Some(InputV1::action(
            InputKindV1::Skill,
            InputTargetV1::target(2),
        )))
        .unwrap();
    assert_eq!(input.kind, InputKindV1::Ult.code());
}
