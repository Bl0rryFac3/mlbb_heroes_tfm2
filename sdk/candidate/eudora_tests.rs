//! Real Eudora callbacks exercised against a deterministic stable-ABI host.
#![allow(dead_code)]
use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};
static SEED: AtomicU64 = AtomicU64::new(100_000);
#[derive(Clone)]
struct Entity {
    kind: EntityKindV1,
    team: usize,
    p: Pos,
    hp: usize,
    buffs: Vec<BuffV1>,
    banished_until: usize,
    shield: usize,
    radius: usize,
    move_speed: usize,
}
struct Host {
    ap: usize,
    ready: [bool;4],
    refunds: Vec<(usize,u32,usize)>,
    projectiles: Vec<(String,String,ProjectileSpawnV1)>,
    typed: Vec<(usize,usize,usize,u32,u32)>,
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
            ap: 64,
            ready:[true;4],refunds:vec![],projectiles:vec![],typed:vec![],
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
                    kind:EntityKindV1::Champion,
                    team,
                    p,
                    hp: 700,
                    buffs: vec![],
                    banished_until: 0,
                    shield: 0,
                    radius: 0,
                    move_speed: 0,
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
        t.entity_radius = Some(radius);
        t.entity_hp = Some(hp);
        t.entity_team = Some(team);
        t.entity_is_alive = Some(alive);
        t.entity_is_champion = Some(champion);
        t.entity_kind=Some(kind);t.entity_is_minion=Some(minion);t.entity_is_tower=Some(tower);
        t.entity_reduce_cooldown=Some(refund);t.spawn_projectile=Some(spawn);
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
        EudoraAi.think(&mut context, base)
    }
    fn step(&mut self, n: usize) {
        for _ in 0..n {
            self.now += 1;
            for e in self.e.values_mut(){
                for b in &mut e.buffs {if b.duration_kind==BuffDurationV1::Time.code(){b.duration_tick=b.duration_tick.saturating_sub(1);}}
                e.buffs.retain(|b|b.duration_kind!=BuffDurationV1::Time.code()||b.duration_tick>0);
            }
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
unsafe extern "C" fn stat(p: *const c_void, id: EntityHandleV1, out: *mut StatV1) -> bool {
    *out = StatV1 {
        magic_power: h(p).ap,
        move_speed: h(p).e[&id.id().unwrap()].move_speed,
        ..StatV1::default()
    };
    true
}
unsafe extern "C" fn radius(p: *const c_void, id: EntityHandleV1) -> usize {
    h(p).e[&id.id().unwrap()].radius
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
unsafe extern "C" fn champion(p: *const c_void, id: EntityHandleV1) -> bool {h(p).e[&id.id().unwrap()].kind==EntityKindV1::Champion}
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
unsafe extern "C" fn damage(p: *mut c_void, c: usize, t: usize, n: usize, dt: u32, at: u32) -> bool {
    m(p).typed.push((c,t,n,dt,at));
    m(p).damage.push((c, t, n));
    true
}
unsafe extern "C" fn heal(p: *mut c_void, _: usize, t: usize, n: usize) {
    m(p).heals.push((t, n));
}
unsafe extern "C" fn cc(p: *mut c_void, t: usize, c: *const CcV1) {
    assert_eq!((*c).kind, CcKindV1::Stun.code());
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


unsafe extern "C" fn kind(p:*const c_void,id:EntityHandleV1)->u32{h(p).e[&id.id().unwrap()].kind.code()}
unsafe extern "C" fn minion(p:*const c_void,id:EntityHandleV1)->bool{kind(p,id)==EntityKindV1::Minion.code()}
unsafe extern "C" fn tower(p:*const c_void,id:EntityHandleV1)->bool{kind(p,id)==EntityKindV1::Tower.code()}
unsafe extern "C" fn refund(p:*mut c_void,id:EntityHandleV1,slot:u32,n:usize)->bool{m(p).refunds.push((id.id().unwrap(),slot,n));true}
unsafe extern "C" fn spawn(p:*mut c_void,n:*const u8,nl:usize,f:*const u8,fl:usize,s:*const ProjectileSpawnV1)->bool{
 m(p).projectiles.push((String::from_utf8_lossy(std::slice::from_raw_parts(n,nl)).into(),String::from_utf8_lossy(std::slice::from_raw_parts(f,fl)).into(),*s));true
}
unsafe extern "C" fn cooldown(p:*const c_void,_:EntityHandleV1,slot:u32,out:*mut CooldownV1)->bool{
 *out=CooldownV1{unlocked:h(p).ready[slot as usize],charges:usize::from(h(p).ready[slot as usize]),max_charges:1,remaining:if h(p).ready[slot as usize]{0}else{999},..Default::default()};true
}
unsafe extern "C" fn player_valid(_: *const c_void,p:PlayerHandleV1)->bool{p.id()==Some(1)}
unsafe extern "C" fn player_champion(_: *const c_void,_:PlayerHandleV1)->EntityHandleV1{EntityHandleV1::from_id(1)}
unsafe extern "C" fn ai_player(_: *const c_void)->usize{1}
unsafe extern "C" fn ai_valid(p:*const c_void,_:*const InputV1)->bool{h(p).ai_valid}
fn apply(h:&mut Host,op:Op,t:usize){h.sim(|s|op.apply(s,0,1,InputTargetV1::target(t)));}
fn mark(h:&mut Host,c:usize,t:usize){h.sim(|s|{let k=crate::entity_key(s,c);apply_mark(s,k,t);});}
#[test]fn unmarked_fan_marks_but_does_not_link_itself(){let mut h=Host::new();apply(&mut h,Op::Fork,2);h.step(60);assert_eq!(h.damage,[(1,2,161)]);assert!(h.refunds.is_empty());}
#[test]fn linked_fan_pays_four_pulses_one_finish_and_one_refund(){let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Fork,2);h.step(59);assert_eq!(h.damage.len(),4);h.step(1);assert_eq!(h.damage.iter().map(|x|x.2).sum::<usize>(),316);assert_eq!(h.refunds,[(1,1,90)]);h.step(5);assert_eq!(h.damage.len(),6);assert!(!h.e[&1].buffs.iter().any(|b|b.name()==SPEED));}
#[test]fn several_links_refund_once_and_speed_does_not_stack(){let mut h=Host::new();h.e.get_mut(&3).unwrap().team=1;for t in [2,3]{mark(&mut h,1,t);}apply(&mut h,Op::Fork,2);let speed:i32=h.e[&1].buffs.iter().map(|b|b.move_speed_mult).sum();assert_eq!(speed,20);h.step(60);assert_eq!(h.refunds.len(),1);assert_eq!(h.damage.iter().filter(|x| x.2 == 161 || x.2 == 135).count(),4);}
#[test]fn broken_or_banished_link_pays_no_completion(){for banish in [true,false]{let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Fork,2);if banish{h.e.get_mut(&2).unwrap().banished_until=100;}else{h.e.get_mut(&2).unwrap().p=(500_000,200_000);}h.step(61);assert_eq!(h.damage.len(),1);assert!(h.refunds.is_empty());}}
#[test]fn target_identity_reuse_cannot_inherit_link(){let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Fork,2);h.e.get_mut(&2).unwrap().buffs.clear();h.sim(|s|{crate::entity_key(s,2);});h.step(61);assert_eq!(h.damage.len(),1);}
#[test]fn lane_minions_take_double_fan_damage_without_marks_but_jungle_is_marked(){let mut h=Host::new();h.e.get_mut(&2).unwrap().kind=EntityKindV1::Minion;apply(&mut h,Op::Fork,2);assert_eq!(h.damage,[(1,2,270)]);assert!(h.e[&2].buffs.is_empty());h.e.get_mut(&2).unwrap().kind=EntityKindV1::Jungle;apply(&mut h,Op::Fork,2);h.sim(|s|{let k=crate::entity_key(s,1);assert!(named(s,2,&marker(k)));});}
#[test]fn tower_is_excluded_from_skills_but_basic_attacks_still_hit(){let mut h=Host::new();h.e.get_mut(&2).unwrap().kind=EntityKindV1::Tower;apply(&mut h,Op::Fork,2);apply(&mut h,Op::BallHit,2);assert!(h.damage.is_empty());apply(&mut h,Op::BasicHit,2);assert_eq!(h.typed.len(),1);assert_eq!(h.typed[0].3,DamageTypeV1::Ad.code());assert_eq!(h.typed[0].4,AttackTypeV1::BaseAttack.code());}
#[test]fn ball_travels_without_damage_until_impact(){let mut h=Host::new();apply(&mut h,Op::Ball,2);assert!(h.damage.is_empty());assert_eq!(h.projectiles.len(),1);assert_eq!(h.projectiles[0].1,"mlbb_heroes:eudora_ball_hit");assert_eq!(h.projectiles[0].2.target_id,2);apply(&mut h,Op::BallHit,2);assert_eq!(h.damage,[(1,2,143)]);assert_eq!(h.roots,[(2,48)]);}
#[test]fn marked_ball_splashes_neighbors_and_hits_primary_once(){let mut h=Host::new();h.e.get_mut(&3).unwrap().team=1;mark(&mut h,1,2);apply(&mut h,Op::BallHit,2);assert_eq!(h.damage.len(),2);assert_eq!(h.damage.iter().filter(|x|x.1==2).count(),1);assert_eq!(h.roots.len(),2);for t in [2,3]{let mr: i32=h.e[&t].buffs.iter().map(|b|b.magic_resistance).sum();assert_eq!(mr,-8);}h.step(108);assert!(h.e[&2].buffs.iter().all(|b|b.magic_resistance==0));}
#[test]fn a_different_eudoras_mark_does_not_enable_splash(){let mut h=Host::new();h.e.get_mut(&3).unwrap().team=1;mark(&mut h,4,2);apply(&mut h,Op::BallHit,2);assert_eq!(h.damage.len(),1);}
#[test]fn ult_does_not_enable_its_own_delayed_hit(){let mut h=Host::new();apply(&mut h,Op::Ult,2);h.step(36);assert_eq!(h.damage,[(1,2,330)]);}
#[test]fn delayed_ult_waits_36_ticks_and_overlap_hits_each_victim_once(){let mut h=Host::new();h.e.get_mut(&3).unwrap().team=1;for t in [2,3]{mark(&mut h,1,t);}apply(&mut h,Op::Ult,2);assert_eq!(h.damage.len(),2);h.step(35);assert_eq!(h.damage.len(),2);h.step(1);assert_eq!(h.damage.len(),4);assert_eq!(h.damage.iter().filter(|x|x.2==206).count(),2);}
#[test]fn delayed_hit_stays_on_ground_and_uses_cast_ap(){let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Ult,2);h.e.get_mut(&2).unwrap().p=(600_000,200_000);h.step(36);assert_eq!(h.damage.len(),1);}
#[test]fn released_ult_finishes_after_death_while_original_identity_remains(){let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Ult,2);h.e.get_mut(&1).unwrap().hp=0;h.step(36);assert_eq!(h.damage,[(1,2,330),(1,2,206)]);}
#[test]fn duplicate_passive_and_match_tick_cannot_pay_twice(){let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Fork,2);h.now=15;h.sim(|s|{tick_all(s);Passive::default().on_update(s,0,1,1);tick_all(s);});assert_eq!(h.damage.len(),2);}
#[test]fn expiring_marks_do_not_leave_permanent_enhancement(){let mut h=Host::new();mark(&mut h,1,2);h.step(300);apply(&mut h,Op::Fork,2);h.step(60);assert_eq!(h.damage.len(),1);}
#[test]fn group_ai_sets_up_a_mark_and_engine_rejection_returns_the_base_input(){let mut h=Host::new();h.e.get_mut(&3).unwrap().team=1;let i=h.ai(None).unwrap();assert_eq!(i.kind,InputKindV1::Skill.code());assert_eq!(i.target.kind,InputTargetKindV1::Pos.code());h.ai_valid=false;assert!(h.ai(None).is_none());}
#[test]fn single_target_ai_uses_stun_then_ult_without_ignoring_cooldowns(){let mut h=Host::new();assert_eq!(h.ai(None).unwrap().kind,InputKindV1::Skill2.code());h.ready[2]=false;assert_eq!(h.ai(None).unwrap().kind,InputKindV1::Ult.code());h.ready[3]=false;assert_eq!(h.ai(None).unwrap().kind,InputKindV1::Skill.code());h.ready[1]=false;assert!(h.ai(None).is_none());}
#[test]fn marks_and_damage_are_isolated_between_simulation_copies(){let mut a=Host::new();let mut b=Host::new();b.seed=a.seed;mark(&mut a,1,2);apply(&mut a,Op::Fork,2);apply(&mut b,Op::Fork,2);b.step(60);assert_eq!(b.damage.len(),1);a.step(60);assert_eq!(a.damage.len(),6);}
#[test]
fn fan_rejects_rear_and_out_of_range_targets() {
    let from = (100_000, 100_000);
    let east = (1., 0.);
    assert!(!fork_hits(from, (90_000, 100_000), east, 3_000.));
    assert!(!fork_hits(from, (170_001, 100_000), east, 0.));
    assert!(fork_hits(from, (170_000, 100_000), east, 0.));
}

#[test]
fn fork_releases_at_an_overlapping_lane_minion() {
    let mut host = Host::new();
    let origin = host.e[&1].p;
    let minion = host.e.get_mut(&2).unwrap();
    minion.kind = EntityKindV1::Minion;
    minion.p = origin;
    host.sim(|sim| Op::Fork.apply(sim, 0, 1, InputTargetV1::pos(origin.0, origin.1)));
    assert_eq!(host.damage, [(1, 2, 270)]);
}

#[test]
fn signed_fork_keeps_its_bearing_at_the_map_edge() {
    let mut host = Host::new();
    host.e.get_mut(&1).unwrap().p = (1_000, 40_000);
    let minion = host.e.get_mut(&2).unwrap();
    minion.kind = EntityKindV1::Minion;
    minion.p = (100, 39_700);
    host.sim(|sim| Op::Fork.apply(sim, 0, 1, InputTargetV1::dir(-30_000, -12_000)));
    assert_eq!(host.damage, [(1, 2, 270)]);
    assert!(host.views.iter().any(|(name, _)| name == "eudora_fan_9"));
}

#[test]
fn minion_collision_radius_counts_at_fan_arc_and_side() {
    let mut host = Host::new();
    let minion = host.e.get_mut(&2).unwrap();
    minion.kind = EntityKindV1::Minion;
    minion.radius = 3_000;
    for (point, expected) in [
        ((272_000, 200_000), true),
        ((273_001, 200_000), false),
        ((220_000, 225_000), true),
        ((220_000, 231_000), false),
    ] {
        host.e.get_mut(&2).unwrap().p = point;
        host.damage.clear();
        host.sim(|sim| Op::Fork.apply(sim, 0, 1, InputTargetV1::dir(1, 0)));
        assert_eq!(!host.damage.is_empty(), expected, "point {point:?}");
    }
}

#[test]
fn lane_ai_waits_at_the_range_edge_and_aims_at_a_windup_safe_minion() {
    let mut host = Host::new();
    let minion = host.e.get_mut(&2).unwrap();
    minion.kind = EntityKindV1::Minion;
    minion.radius = 3_000;
    minion.move_speed = 1_000;
    minion.p = (266_000, 200_000);
    let base = InputV1::action(InputKindV1::Skill, InputTargetV1::pos(200_000, 250_000));
    assert_eq!(host.ai(Some(base)), None);

    let near = (264_000, 200_000);
    host.e.get_mut(&2).unwrap().p = near;
    let corrected = host.ai(Some(base)).unwrap();
    assert_eq!(corrected.target, InputTargetV1::pos(near.0, near.1));
    assert_eq!(host.ai(Some(corrected)), Some(corrected));

    host.ai_valid = false;
    assert_eq!(host.ai(Some(base)), None);
}

#[test]fn tick_rewind_discards_pending_timers_even_if_the_host_reuses_identity(){let mut h=Host::new();h.now=100;mark(&mut h,1,2);apply(&mut h,Op::Ult,2);h.step(1);h.now=0;h.step(60);assert_eq!(h.damage.len(),1);}
#[test]fn unknown_render_origin_does_not_hide_effects(){let mut h=Host::new();let mut t=Host::table();t.sim_origin=None;let mut raw=SimCtxV1{size:size_of::<SimCtxV1>(),sim:&t,frame:std::ptr::null(),state:&mut h as *mut _ as *mut c_void};let mut sim=unsafe{StableSim::from_raw(&mut raw).unwrap()};Op::Fork.apply(&mut sim,0,1,InputTargetV1::target(2));assert!(h.views.iter().any(|(n,_)|n.starts_with("eudora_fan_")));}
#[test]fn ground_ult_can_be_centered_on_the_caster(){let mut h=Host::new();h.e.get_mut(&2).unwrap().p=(200_000,200_000);h.sim(|s|Op::Ult.apply(s,0,1,InputTargetV1::pos(200_000,200_000)));assert_eq!(h.damage,[(1,2,330)]);}

#[test]fn link_pulses_refresh_marks_without_creating_another_link(){let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Fork,2);h.step(60);assert_eq!(h.damage.len(),6);h.step(299);h.sim(|s|{let k=crate::entity_key(s,1);assert!(named(s,2,&marker(k)));});h.step(1);h.sim(|s|{let k=crate::entity_key(s,1);assert!(!named(s,2,&marker(k)));});assert_eq!(h.damage.len(),6);}
#[test]fn thunderburst_marks_new_victims_without_a_recursive_explosion(){let mut h=Host::new();mark(&mut h,1,2);apply(&mut h,Op::Ult,2);h.e.get_mut(&3).unwrap().team=1;h.e.get_mut(&3).unwrap().p=(250_000,200_000);h.step(36);h.sim(|s|{let k=crate::entity_key(s,1);assert!(named(s,3,&marker(k)));});let n=h.damage.len();h.step(36);assert_eq!(h.damage.len(),n);}

#[test]
fn full_pickoff_combo_damage_at_base_and_item_ap() {
    for (ap, expected) in [(64, 995), (200, 1481), (400, 2191)] {
        let mut h = Host::new();
        h.ap = ap;
        // Opening S2 applies the mark; ult and S1 can then use it.
        apply(&mut h, Op::BallHit, 2);
        apply(&mut h, Op::Ult, 2);
        apply(&mut h, Op::Fork, 2);
        h.step(60);
        assert_eq!(h.damage.iter().filter(|x| x.1 == 2).map(|x| x.2).sum::<usize>(), expected, "AP {ap}");
        assert_eq!(h.refunds, [(1, 1, 90)]);
    }
}
#[test]
fn outer_ult_ring_uses_only_outer_damage() {
    let mut h = Host::new();
    h.e.get_mut(&2).unwrap().p = (270_000, 200_000);
    h.sim(|s| Op::Ult.apply(s, 0, 1, InputTargetV1::pos(250_000, 200_000)));
    assert_eq!(h.damage, [(1, 2, 165)]);
    h.step(36);
    assert_eq!(h.damage.len(), 1); // unmarked: no delayed burst
}
