// Focused projectile-event and Alpha area regressions using the ABI host fixture.
fn land_spear(h: &mut Host, index: usize) {
    let shot = h.projectiles[index].clone();
    h.projectiles[index].ended = true;
    h.sim(|s| combat::impact(s, &shot.effect, shot.spec.caster_id, shot.spec.target_id));
}
fn hold_pet(h: &mut Host) {
    let k = h.pet(1);
    h.entities
        .get_mut(&k)
        .unwrap()
        .cc
        .push(CcV1::of_kind(CcKindV1::Bind, 600));
}
#[test]
fn basic_spear_waits_for_arrival_and_uses_base_attack_pipeline() {
    let mut h = Host::new();
    h.start(1);
    hold_pet(&mut h);
    h.cast(1, Op::Basic, 2);
    assert!(h.damage.is_empty());
    assert_eq!(h.projectiles.len(), 1);
    assert!(!h.projectiles[0].spec.penetrate);
    h.step(7);
    assert!(h.damage.is_empty());
    h.step(2);
    assert_eq!(h.damage, vec![(1, 2, 82)]);
    assert_eq!(
        h.damage_types[0],
        (
            1,
            2,
            DamageTypeV1::Ad.code(),
            AttackTypeV1::BaseAttack.code()
        )
    );
}
#[test]
fn command_spear_waits_for_arrival_and_uses_skill_pipeline() {
    let mut h = Host::new();
    h.start(1);
    hold_pet(&mut h);
    h.cast(1, Op::Command, 2);
    assert!(h.damage.is_empty());
    h.step(9);
    assert_eq!(h.damage, vec![(1, 2, 91)]);
    assert_eq!(h.damage_types[0].3, AttackTypeV1::Skill.code());
}
#[test]
fn a_duplicate_landing_cannot_consume_another_inflight_spear() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.now = 1;
    h.cast(1, Op::Basic, 2);
    let first = h.projectiles[0].clone();
    land_spear(&mut h, 0);
    h.sim(|s| combat::impact(s, &first.effect, 1, 2));
    assert_eq!(h.damage, vec![(1, 2, 82)]);
    land_spear(&mut h, 1);
    assert_eq!(h.damage, vec![(1, 2, 82), (1, 2, 82)]);
}
#[test]
fn simultaneous_basic_and_command_have_independent_hit_records() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.cast(1, Op::Command, 2);
    assert_eq!(h.projectiles.len(), 2);
    land_spear(&mut h, 1);
    land_spear(&mut h, 0);
    assert_eq!(h.damage, vec![(1, 2, 91), (1, 2, 82)]);
    assert_eq!(
        h.damage_types.iter().map(|x| x.3).collect::<Vec<_>>(),
        vec![AttackTypeV1::Skill.code(), AttackTypeV1::BaseAttack.code()]
    );
}
#[test]
fn out_of_order_spears_keep_each_casts_attack_snapshot() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.now = 1;
    h.entities.get_mut(&1).unwrap().stat.attack = 164;
    h.cast(1, Op::Basic, 2);
    land_spear(&mut h, 1);
    land_spear(&mut h, 0);
    assert_eq!(h.damage, vec![(1, 2, 164), (1, 2, 82)]);
}
#[test]
fn duplicate_basic_callback_in_one_tick_launches_only_one_projectile() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.cast(1, Op::Basic, 2);
    assert_eq!(h.projectiles.len(), 1);
    assert!(h.damage.is_empty());
}
#[test]
fn refused_projectile_has_no_instant_damage_fallback_or_late_receipt() {
    let mut h = Host::new();
    h.start(1);
    h.projectile_ok = false;
    h.cast(1, Op::Basic, 2);
    assert!(h.damage.is_empty());
    assert!(h.projectiles.is_empty());
    h.sim(|s| combat::impact(s, "mlbb_heroes:popol_spear_hit_0", 1, 2));
    assert!(h.damage.is_empty());
    h.now = 1;
    h.projectile_ok = true;
    h.cast(1, Op::Basic, 2);
    land_spear(&mut h, 0);
    assert_eq!(h.damage, vec![(1, 2, 82)]);
}
#[test]
fn dead_target_and_repeated_landing_cannot_pay_damage_after_revive() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    let effect = h.projectiles[0].effect.clone();
    h.entities.get_mut(&2).unwrap().hp = 0;
    land_spear(&mut h, 0);
    h.entities.get_mut(&2).unwrap().hp = 660;
    h.sim(|s| combat::impact(s, &effect, 1, 2));
    assert!(h.damage.is_empty());
}
#[test]
fn reused_target_copy_is_not_the_original_projectile_target() {
    let mut h = Host::new();
    h.start(1);
    h.cast(1, Op::Basic, 2);
    h.entities
        .get_mut(&2)
        .unwrap()
        .buffs
        .retain(|b| !b.name().starts_with(crate::SIM_COPY_PREFIX));
    h.sim(|s| {
        crate::entity_key(s, 2);
    });
    land_spear(&mut h, 0);
    assert!(h.damage.is_empty());
}
#[test]
fn owner_death_and_respawn_cancel_old_shot_without_stealing_new_one() {
    let mut h = Host::new();
    let mut passive = h.start(1);
    h.cast(1, Op::Basic, 2);
    h.entities.get_mut(&1).unwrap().hp = 0;
    h.sim(|s| passive.on_dead(s, 1));
    h.entities.get_mut(&1).unwrap().hp = 660;
    h.sim(|s| passive.on_spawn(s, 1, 1));
    h.cast(1, Op::Basic, 2);
    land_spear(&mut h, 0);
    assert!(h.damage.is_empty());
    land_spear(&mut h, 1);
    assert_eq!(h.damage, vec![(1, 2, 82)]);
}
#[test]
fn identical_metadata_in_separate_sim_copies_does_not_share_spear_receipts() {
    let mut a = Host::new();
    let mut b = Host::new();
    b.seed = a.seed;
    a.start(1);
    b.start(1);
    a.cast(1, Op::Basic, 2);
    b.cast(1, Op::Basic, 2);
    land_spear(&mut a, 0);
    assert_eq!(a.damage.len(), 1);
    assert!(b.damage.is_empty());
    land_spear(&mut b, 0);
    assert_eq!(b.damage.len(), 1);
}
#[test]
fn spear_target_filters_keep_basics_on_structures_and_skill1_off_them() {
    let mut h = Host::new();
    h.start(1);
    h.add(3, 1, (250000, 200000));
    h.entities.get_mut(&3).unwrap().kind = EntityKindV1::Tower;
    h.cast(1, Op::Basic, 3);
    h.cast(1, Op::Command, 3);
    assert_eq!(h.projectiles.len(), 1);
    land_spear(&mut h, 0);
    assert_eq!(h.damage, vec![(1, 3, 82)]);
}
#[test]
fn spear_rejects_a_wrong_delivery_target_and_expires_old_receipts() {
    let mut h = Host::new();
    h.start(1);
    h.add(3, 1, (240000, 200000));
    h.cast(1, Op::Basic, 2);
    let effect = h.projectiles[0].effect.clone();
    h.sim(|s| combat::impact(s, &effect, 1, 3));
    h.sim(|s| combat::impact(s, &effect, 1, 2));
    assert!(h.damage.is_empty());
    h.now = 1;
    h.cast(1, Op::Basic, 2);
    h.now = 602;
    land_spear(&mut h, 1);
    assert!(h.damage.is_empty());
}
#[test]
fn alpha_s1_hits_each_valid_unit_in_area_once_and_excludes_invalid_targets() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (250000, 200000);
    for (id, team, p) in [
        (3, 1, (262000, 200000)),
        (4, 1, (269000, 200000)),
        (5, 0, (251000, 200000)),
        (6, 1, (251000, 200000)),
        (7, 1, (251000, 200000)),
        (8, 1, (251000, 200000)),
        (9, 1, (251000, 200000)),
        (10, 1, (268000, 200000)),
        (11, 1, (251000, 200000)),
    ] {
        h.add(id, team, p);
    }
    h.entities.get_mut(&6).unwrap().kind = EntityKindV1::Tower;
    h.entities.get_mut(&7).unwrap().kind = EntityKindV1::Nexus;
    h.entities.get_mut(&8).unwrap().hp = 0;
    h.entities.get_mut(&9).unwrap().targetable = false;
    h.entities.get_mut(&11).unwrap().kind = EntityKindV1::Minion;
    h.cast(1, Op::Alpha, 1);
    h.cast(1, Op::Command, 2);
    assert!(h.damage.is_empty());
    h.step(13);
    assert_eq!(
        h.damage
            .iter()
            .filter(|(_, t, _)| *t == 2)
            .map(|x| x.2)
            .collect::<Vec<_>>(),
        vec![91, 151]
    );
    for id in [3, 10, 11] {
        assert_eq!(h.damage.iter().filter(|(_, t, _)| *t == id).count(), 1);
        assert_eq!(h.entities[&id].hp, 509);
        assert!(h.entities[&id]
            .cc
            .iter()
            .any(|c| c.kind == CcKindV1::Airborne.code()));
    }
    for id in [4, 5, 6, 7, 8, 9] {
        assert!(h.damage.iter().all(|(_, t, _)| *t != id));
        assert!(h.entities[&id].cc.is_empty());
    }
    assert_eq!(
        h.entities[&2]
            .cc
            .iter()
            .filter(|c| c.kind == CcKindV1::Airborne.code())
            .count(),
        1
    );
    let count = h.damage.len();
    h.sim(tick_all);
    h.sim(tick_all);
    assert_eq!(h.damage.len(), count);
}
#[test]
fn normal_s1_remains_a_single_target_bite_and_slow() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (250000, 200000);
    h.add(3, 1, (251000, 200000));
    h.cast(1, Op::Command, 2);
    h.step(13);
    assert_eq!(h.damage, vec![(1, 2, 91), (1, 2, 109)]);
    assert!(h.has(2, "mlbb_popol_probe_slow"));
    assert!(!h.has(3, "mlbb_popol_probe_slow"));
    assert!(h.entities[&3].cc.is_empty());
}
#[test]
fn alpha_expiring_before_bite_contact_does_not_pay_both_variants() {
    let mut h = Host::new();
    h.start(1);
    let k = h.pet(1);
    h.entities.get_mut(&k).unwrap().p = (250000, 200000);
    h.add(3, 1, (251000, 200000));
    h.cast(1, Op::Alpha, 1);
    h.now = 539;
    h.cast(1, Op::Command, 2);
    h.step(13);
    assert_eq!(h.damage, vec![(1, 2, 91), (1, 2, 109)]);
    assert!(h.entities[&2].cc.is_empty());
    assert_eq!(h.entities[&3].hp, 660);
}
