use super::*;
use crate::hanzo::tests::{add_unit, Host};

fn spawn(host: &mut Host) -> Passive {
    let mut passive = Passive::default();
    host.sim(|sim| passive.on_spawn(sim, 1, 1));
    passive
}

fn cast(host: &mut Host, action: Cast, input: InputTargetV1) {
    host.sim(|sim| action.apply(sim, 0, 1, input));
}

fn tick(host: &mut Host, passive: &mut Passive, at: usize) {
    host.now = at;
    host.sim(|sim| passive.on_update(sim, 0, 1, 1));
}

fn hp(host: &mut Host, id: usize) -> usize {
    host.sim(|sim| sim.get_entity(id).unwrap().hp().0)
}

fn set_hp(host: &mut Host, id: usize, amount: usize) {
    host.sim(|sim| assert!(sim.entity_set_hp(id, amount)));
}

fn buff(host: &mut Host, name: &str) -> Option<BuffV1> {
    host.sim(|sim| {
        let owner = sim.get_entity(1).unwrap();
        (0..owner.buff_count())
            .filter_map(|i| owner.buff_at(i))
            .find(|b| b.name() == name)
    })
}

#[test]
fn stomp_moves_then_lands_on_tick_24_with_a_separate_position_effect() {
    let mut h = Host::new();
    h.set_unit_position(2, (228_000, 200_000));
    add_unit(&mut h, 3, EntityKindV1::Jungle, 1, (229_000, 200_000));
    add_unit(&mut h, 4, EntityKindV1::Tower, 1, (234_000, 200_000));
    add_unit(&mut h, 5, EntityKindV1::Champion, 0, (232_000, 200_000));
    let mut p = spawn(&mut h);
    cast(&mut h, Cast::Stomp, InputTargetV1::pos(228_000, 200_000));
    tick(&mut h, &mut p, 12);
    h.sim(|s| assert_eq!(s.get_entity(1).unwrap().pos(), (214_000, 200_000)));
    assert_eq!(hp(&mut h, 2), 700);
    assert!(!h
        .effects
        .iter()
        .any(|(name, _, _)| name.contains("stomp_wave")));
    tick(&mut h, &mut p, 24);
    tick(&mut h, &mut p, 24);
    assert_eq!((hp(&mut h, 2), hp(&mut h, 3)), (596, 596));
    assert_eq!((hp(&mut h, 4), hp(&mut h, 5)), (700, 700));
    assert_eq!(
        h.cc,
        vec![
            (2, CcV1::of_kind(CcKindV1::Airborne, 24)),
            (3, CcV1::of_kind(CcKindV1::Airborne, 24))
        ]
    );
    let impacts: Vec<_> = h
        .effects
        .iter()
        .filter(|(name, _, _)| name == "mlbb_minotaur_stomp")
        .collect();
    assert_eq!(impacts.len(), 1);
    assert_eq!(impacts[0].1, (228_000, 200_000));
    let waves: Vec<_> = h
        .effects
        .iter()
        .filter(|(name, _, _)| name.contains("stomp_wave"))
        .collect();
    assert_eq!(
        waves,
        vec![&(
            String::from("mlbb_minotaur_stomp_wave"),
            (228_000, 200_000),
            30
        )]
    );
}

#[test]
fn stomp_wall_clearance_and_displacement_abort_do_not_damage() {
    let wall = Grid::from_rows(&[".#.", ".#."]);
    assert_eq!(
        landing(&wall, (20_000, 10_000), (52_000, 10_000)),
        (31_000, 10_000)
    );
    let mut h = Host::new();
    let mut p = spawn(&mut h);
    cast(&mut h, Cast::Stomp, InputTargetV1::pos(215_000, 200_000));
    h.set_unit_position(1, (190_000, 215_000));
    tick(&mut h, &mut p, 24);
    assert_eq!(hp(&mut h, 2), 700);
    assert!(h.cc.is_empty());
    assert!(!h
        .effects
        .iter()
        .any(|(name, _, _)| name.contains("stomp_wave")));
}

#[test]
fn stomp_charges_apply_to_two_landed_basics_and_leave_structures_normal() {
    let mut h = Host::new();
    add_unit(&mut h, 3, EntityKindV1::Nexus, 1, (244_000, 200_000));
    let mut p = spawn(&mut h);
    cast(&mut h, Cast::Stomp, InputTargetV1::pos(215_000, 200_000));
    tick(&mut h, &mut p, 24);
    h.now = 27;
    cast(&mut h, Cast::Basic, InputTargetV1::target(3));
    assert_eq!(hp(&mut h, 3), 590);
    for time in [30, 50, 70] {
        h.now = time;
        cast(&mut h, Cast::Basic, InputTargetV1::target(2));
    }
    assert_eq!(hp(&mut h, 2), 596 - 2 * 134 - 110);
    assert_eq!(
        h.effects
            .iter()
            .filter(|(name, _, _)| name == "mlbb_minotaur_basic_bonus")
            .count(),
        2
    );
}

#[test]
fn s1_enraged_reaches_a_ring_target_and_boosts_one_packet() {
    let mut h = Host::new();
    add_unit(&mut h, 3, EntityKindV1::Champion, 1, (242_000, 200_000));
    let mut p = spawn(&mut h);
    cast(&mut h, Cast::Fury, InputTargetV1::default());
    for t in [18, 48, 78, 84] {
        tick(&mut h, &mut p, t);
    }
    h.now = 85;
    cast(&mut h, Cast::Stomp, InputTargetV1::pos(215_000, 200_000));
    tick(&mut h, &mut p, 109);
    assert_eq!(hp(&mut h, 3), 572);
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| name == "mlbb_minotaur_stomp_enraged"));
    assert!(h.effects.iter().any(
        |(name, at, ticks)| name == "mlbb_minotaur_stomp_wave_enraged"
            && *at == (215_000, 200_000)
            && *ticks == 30
    ));
}

#[test]
fn roar_prioritizes_hp_percentage_and_enraged_reaches_each_eligible_ally() {
    let mut h = Host::new();
    add_unit(&mut h, 3, EntityKindV1::Champion, 0, (211_000, 200_000));
    add_unit(&mut h, 4, EntityKindV1::Champion, 0, (258_000, 200_000));
    add_unit(&mut h, 5, EntityKindV1::Champion, 0, (258_001, 200_000));
    h.set_unit_max_hp(4, 1_400);
    set_hp(&mut h, 1, 200);
    set_hp(&mut h, 3, 300);
    set_hp(&mut h, 4, 500);
    set_hp(&mut h, 5, 100);
    let mut p = spawn(&mut h);
    h.now = 12;
    cast(&mut h, Cast::Roar, InputTargetV1::default());
    assert_eq!(
        [hp(&mut h, 1), hp(&mut h, 3), hp(&mut h, 4), hp(&mut h, 5)],
        [460, 300, 720, 100]
    );
    h.now = 13;
    cast(&mut h, Cast::Fury, InputTargetV1::default());
    for t in [31, 61, 91, 97] {
        tick(&mut h, &mut p, t);
    }
    h.now = 98;
    cast(&mut h, Cast::Roar, InputTargetV1::default());
    assert_eq!(
        [hp(&mut h, 3), hp(&mut h, 4), hp(&mut h, 5)],
        [520, 940, 100]
    );
}

#[test]
fn fury_rechecks_positions_on_each_of_three_hits_and_keeps_rage_after_cast() {
    let mut h = Host::new();
    let mut p = spawn(&mut h);
    cast(&mut h, Cast::Fury, InputTargetV1::default());
    assert!(buff(&mut h, FURY).unwrap().cc_immune);
    tick(&mut h, &mut p, 17);
    assert_eq!(hp(&mut h, 2), 700);
    tick(&mut h, &mut p, 18);
    assert_eq!(hp(&mut h, 2), 591);
    h.set_unit_position(2, (251_000, 200_000));
    tick(&mut h, &mut p, 48);
    assert_eq!(hp(&mut h, 2), 591);
    h.set_unit_position(2, (235_000, 200_000));
    tick(&mut h, &mut p, 78);
    tick(&mut h, &mut p, 78);
    assert_eq!(hp(&mut h, 2), 431);
    assert_eq!(h.typed, [DamageTypeV1::Fixed.code()]);
    assert_eq!(h.cc, vec![(2, CcV1::of_kind(CcKindV1::Airborne, 42))]);
    tick(&mut h, &mut p, 84);
    assert!(buff(&mut h, FURY).is_none());
    assert_eq!(buff(&mut h, ENRAGED).unwrap().duration_tick, 636);
    assert_eq!(buff(&mut h, RAGE_WINDOW).unwrap().duration_tick, 720);
    let effects: Vec<_> = h
        .effects
        .iter()
        .filter_map(|(n, _, _)| n.strip_prefix("mlbb_minotaur_"))
        .filter(|&n| n == "slam" || n == "finisher")
        .collect();
    assert_eq!(effects, ["slam", "slam", "finisher"]);
}

#[test]
fn roar_still_heals_allies_after_nine_seconds_and_returns_to_one_at_twelve() {
    let mut h = Host::new();
    add_unit(&mut h, 3, EntityKindV1::Champion, 0, (218_000, 200_000));
    add_unit(&mut h, 4, EntityKindV1::Champion, 0, (225_000, 200_000));
    let mut p = spawn(&mut h);
    cast(&mut h, Cast::Fury, InputTargetV1::default());
    for t in [18, 48, 78, 84] {
        tick(&mut h, &mut p, t);
    }
    for id in [3, 4] {
        set_hp(&mut h, id, 100);
    }
    h.now = 719;
    cast(&mut h, Cast::Roar, InputTargetV1::default());
    assert_eq!([hp(&mut h, 3), hp(&mut h, 4)], [320, 320]);
    h.now = 720;
    cast(&mut h, Cast::Roar, InputTargetV1::default());
    assert_eq!([hp(&mut h, 3), hp(&mut h, 4)], [540, 320]);
}

#[test]
fn motivation_uses_observed_health_loss_and_never_heals_unattributed_damage() {
    let mut h = Host::new();
    add_unit(&mut h, 3, EntityKindV1::Tower, 1, (205_000, 200_000));
    let mut p = spawn(&mut h);
    cast(&mut h, Cast::Roar, InputTargetV1::default());
    h.sim(|s| p.on_damaged(s, 1, 1, 2, 200)); // A notification without body HP loss.
    tick(&mut h, &mut p, 1);
    assert_eq!(hp(&mut h, 1), 700);
    h.sim(|s| p.on_damaged(s, 1, 1, 2, 200)); // The host can notify before HP changes.
    set_hp(&mut h, 1, 600);
    tick(&mut h, &mut p, 2);
    assert_eq!(hp(&mut h, 1), 610); // 10% of 100 actual HP, not of the 200 notification.
    set_hp(&mut h, 1, 510);
    h.sim(|s| p.on_damaged(s, 1, 1, 3, 100));
    tick(&mut h, &mut p, 3);
    assert_eq!(hp(&mut h, 1), 510);
    set_hp(&mut h, 1, 410);
    h.sim(|s| {
        p.on_damaged(s, 1, 1, 2, 50);
        p.on_damaged(s, 1, 1, 3, 50);
    });
    tick(&mut h, &mut p, 4);
    assert_eq!(hp(&mut h, 1), 410);
    h.now = 5;
    cast(&mut h, Cast::Fury, InputTargetV1::default());
    set_hp(&mut h, 1, 310);
    h.sim(|s| p.on_damaged(s, 1, 1, 2, 100));
    tick(&mut h, &mut p, 6);
    assert_eq!(hp(&mut h, 1), 330);
    set_hp(&mut h, 1, 0);
    h.sim(|s| p.on_damaged(s, 1, 1, 2, 500));
    tick(&mut h, &mut p, 7);
    assert_eq!(hp(&mut h, 1), 0);
    assert!(buff(&mut h, FURY).is_none());
    assert!(p.owner.is_none());
}

#[test]
fn ai_uses_heal_on_an_injured_ally_and_ult_in_groups_or_for_a_low_hp_peel() {
    let mut h = Host::new();
    add_unit(&mut h, 3, EntityKindV1::Champion, 0, (220_000, 200_000));
    let _p = spawn(&mut h);
    let base = Some(InputV1::action(
        InputKindV1::Attack,
        InputTargetV1::target(2),
    ));
    assert_eq!(
        h.think_with(&mut Ai, base).unwrap().kind,
        InputKindV1::Skill.code()
    );
    set_hp(&mut h, 3, 460);
    assert_eq!(
        h.think_with(&mut Ai, base).unwrap().kind,
        InputKindV1::Skill2.code()
    );
    set_hp(&mut h, 3, 320);
    assert_eq!(
        h.think_with(&mut Ai, base).unwrap().kind,
        InputKindV1::Ult.code()
    );
    set_hp(&mut h, 3, 700);
    add_unit(&mut h, 4, EntityKindV1::Champion, 1, (230_000, 200_000));
    assert_eq!(
        h.think_with(&mut Ai, base).unwrap().kind,
        InputKindV1::Ult.code()
    );
    assert!(h
        .think_with(&mut Ai, Some(InputV1::return_home()))
        .is_none());
}
