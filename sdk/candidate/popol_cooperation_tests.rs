// End-to-end host receipts plus bounded bookkeeping checks.
fn pair(h: &mut Host, owner: usize, target: usize, now: usize) {
    let key = h.key(owner);
    let target = h.key(target);
    let mut all = STATES.lock().unwrap();
    let s = all.get_mut(&key).unwrap();
    s.cooperation.hit(target, now, false);
    s.cooperation.hit(target, now, true);
}
#[test]
fn cooperation_consumes_each_partner_receipt_once_and_caps_stacks() {
    let mut h = Host::new();
    let target = h.key(2);
    let mut c = cooperation::Cooperation::default();
    c.hit(target, 0, false);
    c.hit(target, 1, true);
    assert_eq!(c.stacks, 1);
    c.hit(target, 2, true);
    c.hit(target, 3, true);
    assert_eq!(c.stacks, 1);
    for n in 4..10 {
        c.hit(target, n, false);
        c.hit(target, n, true);
    }
    assert_eq!(c.stacks, 3);
}
#[test]
fn cooperation_requires_same_target_within_two_seconds() {
    let mut h = Host::new();
    h.add(3, 1, (260000, 200000));
    let a = h.key(2);
    let b = h.key(3);
    let mut c = cooperation::Cooperation::default();
    c.hit(a, 0, false);
    c.hit(b, 1, true);
    assert_eq!(c.stacks, 0);
    c.hit(a, 121, true);
    assert_eq!(c.stacks, 0);
    c.hit(a, 122, false);
    assert_eq!(c.stacks, 1);
}
#[test]
fn actual_spear_and_bite_impacts_build_shared_passive() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (240000, 200000);
    h.cast(1, Op::Basic, 2);
    h.step(15);
    assert_eq!(h.state(1).cooperation.stacks, 1);
    assert!(h.has(1, cooperation::BUFF));
    assert!(h.has(k, cooperation::BUFF));
    assert!(
        h.damage_types
            .iter()
            .filter(|(_, _, _, a)| *a == AttackTypeV1::BaseAttack.code())
            .count()
            >= 2
    );
    assert!(h.damage.iter().all(|(attacker, _, _)| *attacker == 1));
}
#[test]
fn spear_launch_and_traps_cannot_create_pair_stacks() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    assert_eq!(h.state(1).cooperation.stacks, 0);
    let key = h.key(1);
    STATES
        .lock()
        .unwrap()
        .get_mut(&key)
        .unwrap()
        .traps
        .seed_trap((250000, 200000), 0, 900);
    h.step(1);
    assert_eq!(h.state(1).cooperation.stacks, 0);
}
#[test]
fn cooperation_refreshes_without_duplicate_buffs_and_alpha_doubles_bonus() {
    let mut h = Host::new();
    h.start(1);
    h.entities.get_mut(&2).unwrap().hp = 0;
    let k = h.pet(1);
    pair(&mut h, 1, 2, 0);
    h.step(1);
    let bonus = |h: &Host, id| {
        h.entities[&id]
            .buffs
            .iter()
            .find(|b| b.name() == cooperation::BUFF)
            .unwrap()
            .attack_speed_mult
    };
    assert_eq!(bonus(&h, 1), 5);
    let writes = h.buff_writes;
    h.step(10);
    assert_eq!(h.buff_writes, writes);
    h.cast(1, Op::Alpha, 1);
    h.step(1);
    assert_eq!(bonus(&h, 1), 10);
    assert_eq!(bonus(&h, k), 10);
    pair(&mut h, 1, 2, 12);
    h.step(1);
    assert_eq!(bonus(&h, 1), 20);
    assert_eq!(
        h.entities[&1]
            .buffs
            .iter()
            .filter(|b| b.name() == cooperation::BUFF)
            .count(),
        1
    );
    h.step(240);
    assert!(!h.has(1, cooperation::BUFF));
    assert!(!h.has(k, cooperation::BUFF));
}
#[test]
fn death_clears_passive_and_revived_kupa_does_not_reuse_stacks() {
    let mut h = Host::new();
    let mut p = h.start(1);
    let k = h.pet(1);
    pair(&mut h, 1, 2, 0);
    h.step(1);
    assert!(h.has(1, cooperation::BUFF));
    h.entities.get_mut(&k).unwrap().hp = 0;
    h.step(1);
    assert!(!h.has(1, cooperation::BUFF));
    assert_eq!(h.state(1).cooperation.stacks, 0);
    h.sim(|s| p.on_spawn(s, 1, 1));
    assert_eq!(h.state(1).cooperation.stacks, 0);
    pair(&mut h, 1, 2, 3);
    h.step(1);
    h.entities.get_mut(&1).unwrap().hp = 0;
    h.sim(|s| p.on_dead(s, 1));
    assert!(!h.has(1, cooperation::BUFF));
}
#[test]
fn cooperation_isolated_between_owners_and_sim_copies() {
    let mut a = Host::new();
    a.start(1);
    a.add(3, 0, (200000, 210000));
    a.start(3);
    let mut b = Host::new();
    b.start(1);
    pair(&mut a, 1, 2, 0);
    assert_eq!(a.state(1).cooperation.stacks, 1);
    assert_eq!(a.state(3).cooperation.stacks, 0);
    assert_eq!(b.state(1).cooperation.stacks, 0);
}
#[test]
fn kupa_inherits_level_equipment_defenses_and_bonus_health_without_free_healing() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    assert_eq!(h.entities[&k].hp, 560);
    h.entities.get_mut(&k).unwrap().hp = 200;
    h.levels.insert(1, 5);
    let owner = h.entities.get_mut(&1).unwrap();
    owner.stat.hp = 700 + 4 * 90 + 400;
    owner.stat.defence = 85;
    owner.stat.magic_resistance = 70;
    h.step(1);
    assert_eq!(h.entities[&k].stat.hp, 1080);
    assert_eq!(h.entities[&k].stat.defence, 85);
    assert_eq!(h.entities[&k].stat.magic_resistance, 70);
    assert_eq!(h.entities[&k].hp, 200);
    h.entities.get_mut(&1).unwrap().stat.defence = 40;
    h.step(30);
    assert_eq!(h.entities[&k].stat.defence, 40);
    assert_eq!(h.entities[&k].hp, 200);
}
#[test]
fn inherited_stats_do_not_accumulate_and_alpha_is_applied_once() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.cast(1, Op::Alpha, 1);
    h.entities.get_mut(&1).unwrap().stat.hp = 700 + 400;
    h.step(1);
    assert_eq!(max_hp(&h.entities[&k]), 1026);
    let writes = h.stat_writes;
    h.step(90);
    assert_eq!(max_hp(&h.entities[&k]), 1026);
    assert_eq!(h.stat_writes, writes);
    h.entities.get_mut(&1).unwrap().stat.hp = 700;
    h.step(30);
    assert_eq!(max_hp(&h.entities[&k]), 756);
    assert!(h.entities[&k].hp <= 756);
}
#[test]
fn kupa_recovers_after_five_seconds_without_healing_during_combat() {
    let mut h = Host::new();
    h.start(1);
    h.entities.get_mut(&2).unwrap().hp = 0;
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().hp = 100;
    h.step(299);
    assert_eq!(h.entities[&k].hp, 100);
    h.step(61);
    assert_eq!(h.entities[&k].hp, 144);
    h.step(60);
    assert_eq!(h.entities[&k].hp, 188);
    h.entities.get_mut(&k).unwrap().hp = 150;
    h.step(60);
    assert_eq!(h.entities[&k].hp, 150);
}
#[test]
fn owner_attack_speed_items_and_passive_change_kupa_bite_interval() {
    let mut h = Host::new();
    h.start(1);
    assert_eq!(h.sim(|s| cooperation::interval(s, 1)), 66);
    h.entities.get_mut(&1).unwrap().buffs.push(BuffV1 {
        attack_speed_mult: 50,
        ..BuffV1::named("test_item")
    });
    assert_eq!(h.sim(|s| cooperation::interval(s, 1)), 44);
    pair(&mut h, 1, 2, 0);
    h.step(1);
    assert_eq!(h.sim(|s| cooperation::interval(s, 1)), 42);
}
