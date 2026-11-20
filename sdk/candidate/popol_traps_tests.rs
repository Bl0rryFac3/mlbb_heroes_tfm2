// Trap regression tests use the existing ABI fixture, not animation frames.
fn trap_step(h:&mut Host,state:&mut traps::Traps,nav:&mut Navigation,tick:usize,place:bool)->bool{
    h.now=tick;let key=h.key(1);h.sim(|s|traps::tick(s,key,state,nav,place))
}
#[test]
fn trap_waits_until_armed_then_pays_one_damage_and_root(){
    let mut h=Host::new();h.start(1);let mut t=traps::Traps::default();let mut nav=Navigation::default();
    t.seed_trap((250000,200000),30,900);
    for n in 0..30{assert!(!trap_step(&mut h,&mut t,&mut nav,n,false));}assert!(h.damage.is_empty());
    for n in 30..40{trap_step(&mut h,&mut t,&mut nav,n,false);}
    assert_eq!(h.damage,vec![(1,2,41)]);assert_eq!(t.counts(),(0,1));assert_eq!(h.entities[&2].cc.iter().filter(|c|c.kind==CcKindV1::Bind.code()).count(),1);
    for n in 40..100{trap_step(&mut h,&mut t,&mut nav,n,false);}assert_eq!(h.damage.len(),1);
}
#[test]
fn traps_choose_one_victim_but_frost_affects_nearby_valid_enemies(){
    let mut h=Host::new();h.start(1);h.add(3,1,(255000,200000));h.add(4,0,(250000,200000));h.add(5,1,(250000,200000));h.entities.get_mut(&5).unwrap().kind=EntityKindV1::Tower;
    let key=h.key(1);let name=traps::Traps::name(key);let mut t=traps::Traps::default();let mut nav=Navigation::default();t.seed_trap((250000,200000),0,900);
    assert!(trap_step(&mut h,&mut t,&mut nav,0,false));assert_eq!(h.damage,vec![(1,2,41)]);assert!(h.has(2,&name)&&h.has(3,&name));assert!(!h.has(4,&name)&&!h.has(5,&name));
    let b=h.entities[&3].buffs.iter().find(|b|b.name()==name).unwrap();assert_eq!((b.move_speed_mult,b.defence_mult),(-20,-12));
}
#[test]
fn overlapping_same_owner_frost_fields_do_not_stack_and_clean_up_on_exit(){
    let mut h=Host::new();h.start(1);let key=h.key(1);let name=traps::Traps::name(key);let mut t=traps::Traps::default();let mut nav=Navigation::default();
    t.seed_field((250000,200000),180);t.seed_field((255000,200000),180);trap_step(&mut h,&mut t,&mut nav,0,false);
    assert_eq!(h.entities[&2].buffs.iter().filter(|b|b.name()==name).count(),1);
    trap_step(&mut h,&mut t,&mut nav,8,false);assert_eq!(h.entities[&2].buffs.iter().filter(|b|b.name()==name).count(),1);
    h.entities.get_mut(&2).unwrap().p=(400000,400000);trap_step(&mut h,&mut t,&mut nav,16,false);assert!(!h.has(2,&name));
}
#[test]
fn field_expiry_and_owner_cleanup_remove_frost_modifiers(){
    let mut h=Host::new();h.start(1);let key=h.key(1);let name=traps::Traps::name(key);let mut t=traps::Traps::default();let mut nav=Navigation::default();
    t.seed_field((250000,200000),10);trap_step(&mut h,&mut t,&mut nav,0,false);assert!(h.has(2,&name));
    trap_step(&mut h,&mut t,&mut nav,10,false);assert!(!h.has(2,&name));assert_eq!(t.counts(),(0,0));
    t.seed_field((250000,200000),100);trap_step(&mut h,&mut t,&mut nav,18,false);assert!(h.has(2,&name));
    h.sim(|s|traps::clear(s,key,&mut t));assert!(!h.has(2,&name));assert_eq!(t.counts(),(0,0));
}
#[test]
fn expired_waiting_trap_cannot_trigger(){
    let mut h=Host::new();h.start(1);let mut t=traps::Traps::default();let mut nav=Navigation::default();t.seed_trap((250000,200000),0,30);
    trap_step(&mut h,&mut t,&mut nav,30,false);assert!(h.damage.is_empty());assert_eq!(t.counts(),(0,0));
}
#[test]
fn placement_cap_cooldown_and_blocked_action_gate_are_respected(){
    let mut h=Host::new();h.start(1);h.entities.get_mut(&2).unwrap().p=(208000,208000);let mut t=traps::Traps::default();let mut nav=Navigation::default();
    trap_step(&mut h,&mut t,&mut nav,0,false);assert_eq!(t.counts().0,0);
    trap_step(&mut h,&mut t,&mut nav,1,true);assert_eq!(t.counts().0,1);assert_eq!(t.deadline(),601);
    h.entities.get_mut(&2).unwrap().p=(300000,208000);trap_step(&mut h,&mut t,&mut nav,20,true);assert_eq!(t.counts().0,1);
    t.seed_trap((400000,400000),2000,2500);trap_step(&mut h,&mut t,&mut nav,601,true);assert!(t.counts().0<=2);
}
#[test]
fn allied_dead_untargetable_and_structure_units_cannot_trigger(){
    let mut h=Host::new();h.start(1);h.entities.get_mut(&2).unwrap().hp=0;
    h.add(3,0,(250000,200000));h.add(4,1,(250000,200000));h.entities.get_mut(&4).unwrap().targetable=false;
    h.add(5,1,(250000,200000));h.entities.get_mut(&5).unwrap().kind=EntityKindV1::Nexus;
    let mut t=traps::Traps::default();let mut nav=Navigation::default();t.seed_trap((250000,200000),0,900);trap_step(&mut h,&mut t,&mut nav,0,false);
    assert!(h.damage.is_empty());assert_eq!(t.counts().0,1);
}
#[test]
fn two_owners_have_independent_frost_modifiers(){
    let mut h=Host::new();h.start(1);h.add(3,0,(200000,200000));h.start(3);let a=h.key(1);let b=h.key(3);
    let mut ta=traps::Traps::default();let mut tb=traps::Traps::default();let mut na=Navigation::default();let mut nb=Navigation::default();
    ta.seed_field((250000,200000),180);tb.seed_field((250000,200000),180);h.sim(|s|{traps::tick(s,a,&mut ta,&mut na,false);traps::tick(s,b,&mut tb,&mut nb,false);});
    assert!(h.has(2,&traps::Traps::name(a)));assert!(h.has(2,&traps::Traps::name(b)));h.sim(|s|traps::clear(s,a,&mut ta));assert!(!h.has(2,&traps::Traps::name(a)));assert!(h.has(2,&traps::Traps::name(b)));
}
#[test]
fn duplicate_owner_tick_cannot_trigger_a_trap_twice(){
    let mut h=Host::new();h.start(1);let key=h.key(1);STATES.lock().unwrap().get_mut(&key).unwrap().traps.seed_trap((250000,200000),0,900);
    h.step(1);let hits=h.damage.len();h.sim(tick_all);h.sim(tick_all);assert_eq!(h.damage.len(),hits);assert_eq!(hits,1);
}

#[test]
fn owner_death_clears_live_frost_buffs(){
    let mut h=Host::new();let mut passive=h.start(1);let key=h.key(1);let name=traps::Traps::name(key);
    STATES.lock().unwrap().get_mut(&key).unwrap().traps.seed_field((250000,200000),180);h.step(1);assert!(h.has(2,&name));
    h.entities.get_mut(&1).unwrap().hp=0;h.sim(|s|passive.on_dead(s,1));assert!(!h.has(2,&name));
}
#[test]
fn placement_waits_for_cast_and_mount_to_finish(){
    let mut h=Host::new();h.start(1);let key=h.key(1);
    {let mut states=STATES.lock().unwrap();let state=states.get_mut(&key).unwrap();state.last_combat=1;state.action_until=100;}
    h.step(1);assert_eq!(h.state(1).traps.counts().0,0);
    {let mut states=STATES.lock().unwrap();let state=states.get_mut(&key).unwrap();state.action_until=0;state.ride_until=100;}
    h.step(1);assert_eq!(h.state(1).traps.counts().0,0);
    STATES.lock().unwrap().get_mut(&key).unwrap().ride_until=0;h.step(1);assert_eq!(h.state(1).traps.counts().0,1);
}
#[test]
fn inactive_traps_do_not_scan_entities_or_emit_effects(){
    let mut h=Host::new();h.start(1);let mut t=traps::Traps::default();let mut nav=Navigation::default();let reads=h.enum_reads.get();
    for n in 0..1000{trap_step(&mut h,&mut t,&mut nav,n,false);}
    assert_eq!(h.enum_reads.get(),reads);assert!(h.view_calls.is_empty());
}
#[test]
fn trap_visual_leases_are_bounded_and_absent_in_presimulation(){
    let mut h=Host::new();h.start(1);let mut t=traps::Traps::default();let mut nav=Navigation::default();
    t.seed_trap((400000,400000),0,900);t.seed_trap((450000,450000),0,900);t.seed_field((250000,200000),180);t.seed_field((260000,200000),180);
    for n in 0..60{trap_step(&mut h,&mut t,&mut nav,n,false);}
    assert_eq!(h.view_calls.len(),32);assert!(h.view_calls.iter().all(|(_,time)|*time<=8));
    let mut p=Host::new();p.origin_kind=SimOriginKindV1::ServerPresim.code();p.start(1);let mut pt=traps::Traps::default();let mut pn=Navigation::default();pt.seed_trap((250000,200000),0,900);
    trap_step(&mut p,&mut pt,&mut pn,0,false);assert_eq!(p.damage,vec![(1,2,41)]);assert!(p.view_calls.is_empty());
}
#[test]
fn one_simulations_frost_cannot_modify_an_identical_sim_copy(){
    let mut a=Host::new();let mut b=Host::new();b.seed=a.seed;a.start(1);b.start(1);let mut t=traps::Traps::default();let mut nav=Navigation::default();t.seed_field((250000,200000),180);
    trap_step(&mut a,&mut t,&mut nav,0,false);let key=a.key(1);let name=traps::Traps::name(key);assert!(a.has(2,&name));assert!(!b.has(2,&name));
}

#[test]
fn automatic_placement_does_not_track_unseen_opponents(){
    let mut h=Host::new();h.start(1);h.unseen.insert((0,2));let mut t=traps::Traps::default();let mut nav=Navigation::default();
    trap_step(&mut h,&mut t,&mut nav,0,true);assert_eq!(t.counts().0,0);
    h.unseen.clear();trap_step(&mut h,&mut t,&mut nav,30,true);assert_eq!(t.counts().0,1);
}
