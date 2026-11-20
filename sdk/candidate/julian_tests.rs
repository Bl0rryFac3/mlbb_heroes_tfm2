//! Real Julian callbacks exercised against a deterministic stable-ABI host.
#![allow(dead_code)]
use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};
static SEED: AtomicU64 = AtomicU64::new(2_000_000);
#[derive(Clone)]
struct Entity {
    kind: EntityKindV1,
    team: usize,
    p: Pos,
    hp: usize,
    buffs: Vec<BuffV1>,
    banished_until: usize,
    shield: usize,
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
        JulianAi.think(&mut context, base)
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
unsafe extern "C" fn stat(p: *const c_void, _: EntityHandleV1, out: *mut StatV1) -> bool {
    *out = StatV1 {
        attack:80,magic_power: h(p).ap,
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
    assert!([CcKindV1::Stun.code(),CcKindV1::Bind.code(),CcKindV1::Airborne.code()].contains(&(*c).kind));
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
#[test]fn six_orders_enhance_only_the_third_distinct_skill_and_lock_all_three(){
 for order in [[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]]{let mut r=Runtime::default();for(i,slot)in order.into_iter().enumerate(){assert!(r.can_cast(slot,i));assert_eq!(r.accept(slot,i,540),i==2);}assert_eq!(r.mask,0);assert!(r.ready.iter().all(|n|*n==542));for slot in 0..3{assert!(!r.can_cast(slot,541));assert!(r.can_cast(slot,542));}}
}
#[test]fn duplicate_skill_cannot_progress_combo_and_expired_partial_sequence_resets(){let mut r=Runtime::default();r.accept(0,0,540);assert!(!r.can_cast(0,1));r.reset_expired(241);assert_eq!(r.mask,0);assert!(r.can_cast(1,241));assert!(!r.can_cast(0,241));}
#[test]fn normal_scythe_travels_pierces_minions_then_stops_on_champion(){let mut h=Host::new();h.e.get_mut(&3).unwrap().team=1;h.e.get_mut(&3).unwrap().kind=EntityKindV1::Minion;h.e.get_mut(&3).unwrap().p=(220_000,200_000);h.e.get_mut(&4).unwrap().team=1;h.e.get_mut(&4).unwrap().p=(265_000,200_000);apply(&mut h,Op::Scythe,2);assert!(h.damage.is_empty());h.step(30);assert_eq!(h.damage.iter().map(|x|x.1).collect::<Vec<_>>(),vec![3,2]);assert!(h.e[&2].buffs.iter().any(|b|b.move_speed_mult==-30));}
#[test]fn enhanced_scythe_pays_continuous_pulses_and_no_repeated_tick_payment(){let mut h=Host::new();h.e.get_mut(&2).unwrap().p=(270_000,200_000);h.sim(|s|{let(k,mut r)=take(s,1);r.mask=6;put(k,r);});apply(&mut h,Op::Scythe,2);h.step(18);for _ in 0..112{let q=h.sim(|s|{let k=crate::entity_key_ro(s,1);STATES.lock().unwrap().get(&k).and_then(|r|r.zones.iter().find(|z|z.kind==1)).map(|z|point(z.center,z.dir,(s.tick()+1-z.start).min(96) as f64*250.))});if let Some(p)=q{h.e.get_mut(&2).unwrap().p=p;}h.step(1);}assert_eq!(h.damage.iter().map(|x|x.2).sum::<usize>(),spell(ENH_SCYTHE,64)+CONTINUOUS_HITS*spell(SPIN,64));let before=h.damage.len();h.sim(|s|{tick_all(s);tick_all(s);});assert_eq!(h.damage.len(),before);}
#[test]fn chain_delay_binds_only_enemies_in_area_once(){let mut h=Host::new();apply(&mut h,Op::Chain,2);h.step(17);assert!(h.damage.is_empty());h.step(1);assert_eq!(h.damage,vec![(1,2,spell(CHAIN,64))]);assert_eq!(h.roots,vec![(2,60)]);h.step(50);assert_eq!(h.damage.len(),1);}
#[test]fn enhanced_chain_pays_four_pulses_then_one_airborne_hit(){let mut h=Host::new();h.sim(|s|{let(k,mut r)=take(s,1);r.mask=3;put(k,r);});apply(&mut h,Op::Chain,2);h.step(65);assert_eq!(h.damage.len(),4);h.step(1);assert_eq!(h.damage.len(),5);assert_eq!(h.damage.iter().map(|x|x.2).sum::<usize>(),4*spell(CHAIN_PULSE,64)+spell(LAUNCH,64));assert_eq!(h.roots,vec![(2,48)]);h.step(60);assert_eq!(h.damage.len(),5);}
#[test]fn enhanced_sword_moves_before_banish_and_global_hook_pays_all_six_blade_hits(){let mut h=Host::new();h.sim(|s|{let(k,mut r)=take(s,1);r.mask=5;put(k,r);});apply(&mut h,Op::Sword,2);assert!(!h.sets.is_empty());assert_eq!(h.e[&1].banished_until,36);h.step(36);assert_eq!(h.damage.iter().filter(|x|x.1==2).count(),6);assert_eq!(h.damage.iter().map(|x|x.2).sum::<usize>(),6*spell(BLADE,64));}
#[test]fn boosted_basic_is_one_magic_base_attack_and_expires(){let mut h=Host::new();h.e.get_mut(&2).unwrap().p=(220_000,200_000);apply(&mut h,Op::Chain,2);apply(&mut h,Op::Basic,2);assert_eq!(h.typed.last().unwrap().3,DamageTypeV1::Ap.code());assert_eq!(h.typed.last().unwrap().4,AttackTypeV1::BaseAttack.code());assert_eq!(h.damage.last().unwrap().2,145);h.step(121);apply(&mut h,Op::Basic,2);assert_eq!(h.typed.last().unwrap().3,DamageTypeV1::Ad.code());assert_eq!(h.damage.last().unwrap().2,80);}
#[test]fn skills_reject_structures_and_friendly_targets(){let mut h=Host::new();h.e.get_mut(&2).unwrap().kind=EntityKindV1::Tower;apply(&mut h,Op::Scythe,2);apply(&mut h,Op::Chain,3);h.step(30);assert!(h.damage.is_empty());}
#[test]fn hero_sustain_does_not_duplicate_one_cast_or_trigger_on_lane_minions(){let mut h=Host::new();h.e.get_mut(&2).unwrap().kind=EntityKindV1::Minion;apply(&mut h,Op::Chain,2);h.step(18);assert!(!h.e[&1].buffs.iter().any(|b|b.name()==SUSTAIN));h.e.get_mut(&2).unwrap().kind=EntityKindV1::Champion;h.now=600;apply(&mut h,Op::Chain,2);h.step(18);assert_eq!(h.e[&1].buffs.iter().filter(|b|b.name()==SUSTAIN).map(|b|b.vamp).sum::<i32>(),10);}
#[test]fn death_cancels_pending_damage_and_other_simulation_cannot_advance_it(){let mut h=Host::new();apply(&mut h,Op::Chain,2);let mut other=Host::new();other.step(80);assert!(h.damage.is_empty());h.e.get_mut(&1).unwrap().hp=0;h.step(30);assert!(h.damage.is_empty());}
#[test]fn engine_locked_chain_is_assisted_without_mutating_the_ai_context(){let mut h=Host::new();h.ready[3]=false;h.sim(|s|Passive::default().on_spawn(s,1,1));h.step(1);h.sim(|s|{let k=crate::entity_key_ro(s,1);assert_eq!(STATES.lock().unwrap().get(&k).unwrap().mask,4);});let n=h.damage.len();let input=h.ai(None).unwrap();assert_eq!(input.kind,InputKindV1::Skill2.code());assert_eq!(h.damage.len(),n);}
#[test]fn group_ai_commits_scythe_sword_enhanced_chain_order_and_checks_engine_validity(){let mut h=Host::new();h.e.get_mut(&3).unwrap().team=1;assert_eq!(h.ai(None).unwrap().kind,InputKindV1::Skill.code());apply(&mut h,Op::Scythe,2);assert_eq!(h.ai(None).unwrap().kind,InputKindV1::Skill2.code());h.ai_valid=false;assert!(h.ai(None).is_none());}
#[test]fn targetless_ai_preserves_a_valid_baseline_action(){let mut h=Host::new();h.e.get_mut(&2).unwrap().hp=0;let base=InputV1::action(InputKindV1::Attack,InputTargetV1::target(2));assert_eq!(h.ai(Some(base)),Some(base));}
#[test]
// Tooltip hints are asserted as literals on purpose: if a kit constant moves and this test does
// not, the numbers players see would have drifted from the numbers the engine deals. 1.1.129 raised
// SCYTHE 120/58 -> 155/72, SWORD 105/44 -> 130/52, CHAIN 92/42 -> 112/50, so all three moved.
fn ordinary_attack_and_skill_hints_share_the_actual_normal_damage_profiles(){let stat=StatV1{attack:80,magic_power:64,..Default::default()};assert_eq!(Op::Scythe.expected_damage(&stat),(0,201));assert_eq!(Op::Sword.expected_damage(&stat),(0,163));assert_eq!(Op::Chain.expected_damage(&stat),(0,144));assert_eq!(Op::Basic.expected_damage(&stat),(80,0));}

#[test]fn death_callback_clears_owned_queues_even_when_the_live_champion_lookup_is_unavailable(){let mut h=Host::new();let mut p=Passive::default();h.sim(|s|p.on_spawn(s,1,1));apply(&mut h,Op::Chain,2);h.sim(|s|p.on_dead(s,1));h.step(50);assert!(h.damage.is_empty());}

// Regression: every Julian voice cue must have a lock duration, or the global
// voice gate falls back to 180 ticks and the line is silently swallowed.
#[test]fn every_julian_voice_cue_has_a_lock_duration(){
    let mut missing=vec![];
    for (_,line,_,_) in crate::audio::VOICE_CUES.iter().filter(|(s,_,_,_)|s.starts_with("julian_")){
        if crate::audio::lock_duration_for_test(line).is_none(){missing.push(*line);}
    }
    assert!(missing.is_empty(),"cues with no lock duration: {missing:?}");
}

// Regression: omnivamp must actually restore HP on a skill hit.
#[test]fn skill_hits_restore_health_through_omnivamp(){
    let mut h=Host::new();
    h.e.get_mut(&1).unwrap().hp=h.e[&1].hp/2;   // wounded so the heal is not clamped away
    let heals_before=h.heals.len();
    apply(&mut h,Op::Scythe,2);
    h.step(20);
    let casts=h.heals.len()-heals_before;
    assert!(casts>0,"no heal calls issued on a skill hit");
    assert!(h.heals.iter().all(|(target,amt)| *target==1 && *amt>0),"heals must be self-targeted and positive");
}

#[test]
fn basic_attack_is_not_re_gated_on_distance_once_the_engine_has_committed_it(){
    // Regression for 1.1.126: the native re-checked `within(.., 28_000.)` on top of the
    // action's own `range`, so a target whose centre sat just past 28_000 took the attack
    // animation and ZERO damage. Chou/Johnson/Balmond/Eudora ship no such gate.
    let mut h=Host::new();
    h.e.get_mut(&2).unwrap().p=(229_000,200_000); // 29_000 from the caster at 200_000
    apply(&mut h,Op::Basic,2);
    assert_eq!(h.damage.len(),1,"basic attack must connect just beyond the old centre-to-centre gate");
    assert_eq!(h.damage.last().unwrap().2,80);
}

#[test]
fn basic_attack_still_rejects_allies_and_dead_targets(){
    let mut h=Host::new();
    // ally: both team 0
    h.e.get_mut(&2).unwrap().team=0;
    h.e.get_mut(&2).unwrap().p=(220_000,200_000);
    apply(&mut h,Op::Basic,2);
    assert!(h.damage.is_empty(),"ally must not be hit");
    // dead target
    h.e.get_mut(&2).unwrap().team=1;
    h.e.get_mut(&2).unwrap().hp=0;
    apply(&mut h,Op::Basic,2);
    assert!(h.damage.is_empty(),"dead target must not be hit");
}

#[test]
fn basic_attack_hits_towers_and_the_nexus(){
    // 1.1.127 had added a structure rejection to the reject chain, so a committed attack on a
    // tower produced the animation and no damage at all. 1.1.128 restores the MOBA rule:
    // structures take the plain AD hit (the enhanced AP variant still skips them via enemy()).
    let mut h=Host::new();
    h.e.get_mut(&2).unwrap().kind=EntityKindV1::Tower;
    apply(&mut h,Op::Basic,2);
    assert_eq!(h.damage.len(),1,"tower must take the basic attack");
    assert_eq!(h.damage.last().unwrap().2,80,"structure damage is the raw AD hit");
    let mut h=Host::new();
    h.e.get_mut(&2).unwrap().kind=EntityKindV1::Nexus;
    apply(&mut h,Op::Basic,2);
    assert_eq!(h.damage.len(),1,"nexus must take the basic attack");
}

#[test]
fn omnivamp_never_procs_off_a_structure(){
    let mut h=Host::new();
    h.e.get_mut(&1).unwrap().hp=400; // caster hurt, so a heal WOULD be possible
    h.e.get_mut(&2).unwrap().kind=EntityKindV1::Tower;
    apply(&mut h,Op::Basic,2);
    assert_eq!(h.damage.len(),1,"the structure still takes damage");
    assert!(h.heals.is_empty(),"omnivamp must not proc off structures (owner rule)");
    let mut h=Host::new();
    h.e.get_mut(&1).unwrap().hp=400;
    apply(&mut h,Op::Basic,2);
    assert_eq!(h.heals.len(),1,"and it must still proc off a champion");
}

/// The .129 buff moved eight damage constants and **nothing failed**: the tooltip text in
/// `text/champion.i18n` kept printing pre-buff numbers, and it took the owner asking "is his skill
/// descriptions updated" to find it. So the fragments are built FROM the Rust constants here - buff a
/// number without updating all 8 locales and this test fails instead of shipping a wrong tooltip.
#[test]
fn skill_tooltip_damage_matches_the_rust_constants() {
    use std::path::PathBuf;
    let start = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = start.clone();
    let file = loop {
        let cand = dir.join("text").join("champion.i18n");
        if cand.is_file() {
            break cand;
        }
        if !dir.pop() {
            panic!("no text/champion.i18n found above {start:?}");
        }
    };
    let source = std::fs::read_to_string(&file).expect("read champion.i18n");
    let raw = source
        .split("\"mlbb_julian\": {")
        .skip(1)
        .filter_map(|section| section.split_once("\n      }").map(|(fields, _)| fields))
        .collect::<Vec<_>>()
        .join("\n");
    // One occurrence per locale. The locales share the ASCII "N + M% AP" fragment, so a single
    // count checks the English and the seven translations at once.
    const LOCALES: usize = 8;
    let kit = [
        ("Scythe", SCYTHE),
        ("Enhanced scythe", ENH_SCYTHE),
        ("Scythe spin (per tick)", SPIN),
        ("Sword", SWORD),
        ("Storm of swords (per hit)", BLADE),
        ("Chain", CHAIN),
        ("Chain pulse (per tick)", CHAIN_PULSE),
        ("Chain launch", LAUNCH),
    ];
    for (label, (flat, ratio)) in kit {
        let frag = format!("{flat} + {ratio}% AP");
        assert_eq!(
            raw.matches(&frag).count(),
            LOCALES,
            "{label}: `{frag}` must appear once per locale in {file:?}. The description text has \
             drifted from the damage constants - update all 8 locales of description.mlbb_julian."
        );
    }
    // Boosted basic attack is `30 + ad + ap * 55 / 100` in `basic` - the text says 30 + 55% AP and
    // is currently correct, but it was verified by hand once, so it is pinned here too.
    assert_eq!(
        raw.matches("30 + 55% AP").count(),
        LOCALES,
        "boosted basic-attack text drifted from julian.rs `basic`"
    );
}
