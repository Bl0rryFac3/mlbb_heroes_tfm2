use super::*;
use crate::hanzo::tests::{add_unit, Host};

fn setup(h: &mut Host) -> Passive {
    let mut passive = Passive::default();
    h.sim(|sim| passive.on_spawn(sim, 0, 1));
    passive
}

fn apply(h: &mut Host, cast: Cast, input: InputTargetV1) {
    h.sim(|sim| cast.apply(sim, 0, 1, input));
}

fn tick(h: &mut Host, p: &mut Passive, now: usize) {
    h.now = now;
    h.sim(|sim| p.on_update(sim, 0, 0, 1));
}

fn hp(h: &mut Host, id: usize) -> usize {
    h.sim(|sim| sim.get_entity(id).unwrap().hp().0)
}

#[test]
fn host_selected_structures_take_one_basic_and_show_each_forms_attack_pose() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    add_unit(&mut h, 3, EntityKindV1::Tower, 1, (240_000, 200_000));
    add_unit(&mut h, 4, EntityKindV1::Nexus, 1, (158_000, 200_000));
    add_unit(&mut h, 5, EntityKindV1::Tower, 0, (211_000, 200_000));

    apply(&mut h, Cast::Basic, InputTargetV1::target(3));
    apply(&mut h, Cast::Basic, InputTargetV1::target(5));
    tick(&mut h, &mut p, 4);
    assert_eq!(hp(&mut h, 3), 590, "one armored hit on the resolved tower");
    assert_eq!(hp(&mut h, 5), 700, "allied tower stays protected");
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| { name.starts_with("mlbb_xborg_body_armored_basic_attack_") }));

    h.sim(|sim| p.on_damaged(sim, 0, 1, 2, Firaga::capacity(5)));
    tick(&mut h, &mut p, 6);
    h.set_unit_position(1, (200_000, 200_000));
    apply(&mut h, Cast::Basic, InputTargetV1::target(3));
    tick(&mut h, &mut p, 10);
    assert_eq!(
        hp(&mut h, 3),
        480,
        "one armorless hit on the resolved tower"
    );
    apply(&mut h, Cast::Basic, InputTargetV1::target(4));
    tick(&mut h, &mut p, 14);
    assert_eq!(
        hp(&mut h, 4),
        590,
        "one armorless hit on the resolved nexus"
    );
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| { name.starts_with("mlbb_xborg_body_armorless_basic_attack_left_") }));
    assert!(h
        .effects
        .iter()
        .all(|(name, _, _)| !name.starts_with("mlbb_xborg_heat_")));
}

#[test]
fn firaga_rearms_from_one_owner_reserve() {
    let mut armor = Firaga::new(1);
    assert_eq!(armor.reserve, 264);
    assert!(!armor.hit(100, 1000));
    assert!(armor.hit(164, 1000));
    for _ in 0..3 {
        armor.pickup();
    }
    assert!(!armor.recharge(1383));
    assert!(armor.recharge(1384));
    assert_eq!(armor.reserve, 79);
    armor.pickup();
    assert_eq!(armor.reserve, 110);
}

#[test]
fn firaga_gauge_follows_actor_and_switches_from_reserve_to_recharge_and_back() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    for now in [4, 8] {
        h.now = now;
        h.sim(|sim| p.on_damaged(sim, 0, 1, 2, Firaga::capacity(5) / 2));
        tick(&mut h, &mut p, now);
    }
    let break_pos = h.sim(|sim| position(sim, 1).unwrap());
    h.set_unit_position(1, (222_000, 230_000));
    tick(&mut h, &mut p, 308);
    tick(&mut h, &mut p, 608);

    let gauges: Vec<_> = h
        .effects
        .iter()
        .filter_map(|(name, pos, _)| {
            name.strip_prefix("mlbb_xborg_firaga_gauge_")
                .map(|frame| (frame, *pos))
        })
        .collect();
    assert_eq!(
        gauges,
        [
            ("armor_20", (200_000, 164_000)),
            ("armor_10", (200_000, 164_000)),
            ("recharge_0", (break_pos.0, break_pos.1 - 36_000)),
            ("recharge_10", (222_000, 194_000)),
            ("armor_6", (222_000, 194_000)),
        ]
    );
}

#[test]
fn an_overlapping_stake_cast_does_not_flip_the_flame_pose_or_muzzle() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    apply(&mut h, Cast::Fire, InputTargetV1::dir(-1000, 0));
    apply(&mut h, Cast::Stakes, InputTargetV1::dir(1000, 0));
    tick(&mut h, &mut p, 4);
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| { name.starts_with("mlbb_xborg_body_armored_s1_start_left_") }));
    assert!(h.effects.iter().any(|(name, pos, _)| {
        name.starts_with("mlbb_xborg_s1_armored_flame_8_") && *pos == (185_000, 178_000)
    }));
}

#[test]
fn s1_tracks_its_caster_and_only_the_last_two_cold_target_hits_are_true() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    add_unit(&mut h, 3, EntityKindV1::Tower, 1, (213_000, 200_000));
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 0..95 {
        h.set_unit_position(1, (200_000 + now as u64 * 50, 200_000));
        tick(&mut h, &mut p, now);
    }
    assert_eq!(
        h.typed,
        vec![
            DamageTypeV1::Ad.code(),
            DamageTypeV1::Ad.code(),
            DamageTypeV1::Ad.code(),
            DamageTypeV1::Ad.code(),
            DamageTypeV1::Fixed.code(),
            DamageTypeV1::Fixed.code()
        ]
    );
    assert_eq!(hp(&mut h, 2), 448);
    assert_eq!(hp(&mut h, 3), 700);
    let paths: Vec<_> = h
        .effects
        .iter()
        .filter(|(name, _, _)| name.starts_with("mlbb_xborg_s1_armored_flame_"))
        .collect();
    assert!(paths.len() > 5);
    assert!(paths.last().unwrap().1 .0 > paths.first().unwrap().1 .0);
}

#[test]
fn s1_hits_multiple_champions_and_monsters_including_hitboxes_at_the_fan_edge() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    for (id, kind, team, pos) in [
        (3, EntityKindV1::Champion, 1, (240_000, 234_000)), // Crosses the side.
        (4, EntityKindV1::Jungle, 2, (229_000, 182_000)),
        (5, EntityKindV1::Epic, 2, (257_000, 200_000)), // Crosses the tip.
        (6, EntityKindV1::Serpen, 2, (242_000, 181_000)),
        (7, EntityKindV1::Champion, 1, (222_000, 237_000)),
        (8, EntityKindV1::Champion, 1, (180_000, 200_000)),
        (9, EntityKindV1::Tower, 1, (221_000, 200_000)),
        (10, EntityKindV1::Champion, 0, (211_000, 200_000)),
    ] {
        add_unit(&mut h, id, kind, team, pos);
    }
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 0..90 {
        tick(&mut h, &mut p, now);
    }
    for id in 2..=6 {
        assert_eq!(hp(&mut h, id), 448, "target {id} takes all six pulses");
    }
    for id in 7..=10 {
        assert_eq!(hp(&mut h, id), 700, "target {id} stays outside damage");
    }
    assert_eq!(
        h.typed,
        [
            vec![DamageTypeV1::Ad.code(); 20],
            vec![DamageTypeV1::Fixed.code(); 10],
        ]
        .concat()
    );
    let marker_count = h
        .effects
        .iter()
        .rev()
        .take_while(|(name, _, _)| name == "mlbb_xborg_heat_4")
        .count();
    assert_eq!(marker_count, 5);
}

#[test]
fn armorless_basic_and_long_range_stake_each_add_heat_before_s1() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    h.sim(|sim| p.on_damaged(sim, 0, 1, 2, Firaga::capacity(5)));
    tick(&mut h, &mut p, 0);
    h.set_unit_position(1, (200_000, 200_000));
    h.sim(|sim| {
        assert!(!has_buff(sim, 1, ARMOR));
        assert!(Cast::Stakes.can_activate(sim, 1));
    });
    apply(&mut h, Cast::Basic, InputTargetV1::target(2));
    h.set_unit_position(2, (264_000, 200_000));
    apply(&mut h, Cast::Stakes, InputTargetV1::dir(1000, 0));
    for now in 1..100 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 2), 496);
    assert_eq!(h.pulls.len(), 1);
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| name == "mlbb_xborg_heat_2"));
    assert!(!h.sim(|sim| has_buff(sim, 1, ARMOR)));
    h.typed.clear();
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 100..190 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(
        h.typed,
        [
            vec![DamageTypeV1::Ad.code(); 2],
            vec![DamageTypeV1::Fixed.code(); 4],
        ]
        .concat()
    );
}

#[test]
fn s1_heals_from_actual_health_damage_in_both_phases_and_caps_group_sustain() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    add_unit(&mut h, 3, EntityKindV1::Minion, 1, (218_000, 200_000));
    h.sim(|sim| {
        sim.entity_set_hp(1, 300);
        sim.entity_set_hp(2, 12);
    });
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 0..90 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 1), 301, "no healing from overkill or the minion");
    assert_eq!(hp(&mut h, 3), 526, "the minion still takes all pulses");

    h.now = 300;
    h.respawn_unit(2, (215_000, 200_000));
    add_unit(&mut h, 4, EntityKindV1::Jungle, 2, (220_000, 200_000));
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 300..390 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 1), 338, "both warm and true hits contribute");

    h.now = 600;
    for id in 5..=10 {
        add_unit(&mut h, id, EntityKindV1::Champion, 1, (223_000, 200_000));
    }
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 600..690 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 1), 410, "healing is capped at 12 per pulse");
    for id in 5..=10 {
        assert_eq!(hp(&mut h, id), 448, "the healing cap never caps targets");
    }
}

#[test]
fn s1_repairs_firaga_at_full_body_hp_and_refills_armorless_energy() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    add_unit(&mut h, 3, EntityKindV1::Jungle, 2, (222_000, 200_000));
    h.sim(|sim| p.on_damaged(sim, 0, 1, 2, 100));
    tick(&mut h, &mut p, 0);
    h.now = 1;
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 1..91 {
        tick(&mut h, &mut p, now);
    }
    let reserve = h.sim(|sim| STATES.lock().unwrap()[&entity_key_ro(sim, 1)].armor.reserve);
    assert_eq!(reserve, Firaga::capacity(5) - 100 + 18);
    assert_eq!(hp(&mut h, 1), 700);

    h.now = 300;
    h.sim(|sim| p.on_damaged(sim, 0, 1, 2, reserve));
    tick(&mut h, &mut p, 300);
    h.set_unit_position(1, (200_000, 200_000));
    apply(&mut h, Cast::Fire, InputTargetV1::dir(1000, 0));
    for now in 300..390 {
        tick(&mut h, &mut p, now);
    }
    let energy = h.sim(|sim| STATES.lock().unwrap()[&entity_key_ro(sim, 1)].armor.energy);
    assert_eq!(energy, 14 + 5, "natural recharge plus skill hits");
}

#[test]
fn s2_cast_waits_then_returns_to_current_point_and_deduplicates_overlap() {
    let mut h = Host::new();
    h.pull_allowed = true;
    let mut p = setup(&mut h);
    h.set_unit_position(2, (205_000, 202_000));
    apply(&mut h, Cast::Stakes, InputTargetV1::dir(1000, 0));
    for now in 0..72 {
        h.set_unit_position(1, (201_000, 202_000));
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 2), 700);
    for now in 72..101 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 2), 606);
    assert_eq!(h.pulls.len(), 1);
    assert_eq!(h.pulls[0], (1, 2, 12));
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| name.starts_with("mlbb_xborg_s2_recall_trail_")));
}

#[test]
fn r_spin_draws_different_poses_with_actor_motion_and_ends_once() {
    let mut h = Host::new();
    h.set_unit_max_hp(2, 5000);
    h.respawn_unit(2, (215_000, 200_000));
    let mut p = setup(&mut h);
    apply(&mut h, Cast::Spin, InputTargetV1::dir(1000, 0));
    for now in 0..217 {
        if (24..45).contains(&now) {
            h.set_unit_position(1, (200_000 + now as u64 * 20, 200_000));
        }
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 2), 4172);
    assert!(!h.sim(|sim| Cast::Spin.can_activate(sim, 1)));
    let spin: Vec<_> = h
        .effects
        .iter()
        .filter(|(name, _, _)| name.starts_with("mlbb_xborg_body_armored_ult_spin_"))
        .collect();
    assert!(spin.len() > 20);
    assert!(spin
        .windows(2)
        .any(|f| f[0].0 != f[1].0 && f[0].1 != f[1].1));
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| name.starts_with("mlbb_xborg_body_armorless_idle_")));
}

#[test]
fn shell_break_on_timer_tick_spends_own_guard_and_detonates_at_half() {
    let mut h = Host::new();
    h.set_unit_max_hp(2, 5000);
    h.respawn_unit(2, (215_000, 200_000));
    let mut p = setup(&mut h);
    h.sim(|sim| sim.add_buff(1, &BuffV1::named("ally_armor_marker")));
    apply(&mut h, Cast::Spin, InputTargetV1::dir(1000, 0));
    for now in 0..180 {
        tick(&mut h, &mut p, now);
    }
    h.now = 180;
    h.sim(|sim| {
        let level = sim.get_entity(1).unwrap().level();
        p.on_damaged(sim, 0, 1, 2, Firaga::capacity(level));
    });
    tick(&mut h, &mut p, 180);
    assert_eq!(hp(&mut h, 2), 4472);
    h.sim(|sim| {
        assert!(has_buff(sim, 1, "ally_armor_marker"));
        assert!(!has_buff(sim, 1, ARMOR));
        assert!(!Cast::Spin.can_activate(sim, 1));
    });
    assert_eq!(
        h.effects
            .iter()
            .filter(|(name, _, _)| name == "mlbb_xborg_ult_explosion_forced")
            .count(),
        1
    );
}

#[test]
fn r_completes_its_spin_and_applies_the_blast_bonus_per_target() {
    let mut h = Host::new();
    let mut p = setup(&mut h);
    add_unit(&mut h, 3, EntityKindV1::Champion, 1, (239_000, 210_000));
    add_unit(&mut h, 4, EntityKindV1::Epic, 2, (223_000, 218_000));
    h.set_unit_max_hp(4, 5000);
    h.respawn_unit(4, (223_000, 218_000));
    h.sim(|sim| {
        sim.entity_set_hp(2, 260);
    });
    apply(&mut h, Cast::Spin, InputTargetV1::dir(1000, 0));
    for now in 0..=92 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 2), 108);
    assert!(h
        .effects
        .iter()
        .any(|(name, _, _)| name == "mlbb_xborg_heat_4"));
    for now in 93..180 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(hp(&mut h, 2), 32, "the AI leaves time to finish stacking");
    assert_eq!(hp(&mut h, 3), 700, "enemy on the blast fringe stays cold");
    assert_eq!(hp(&mut h, 4), 4772);
    assert!(h.sim(|sim| has_buff(sim, 1, ARMOR)));
    tick(&mut h, &mut p, 180);
    assert_eq!(hp(&mut h, 2), 0);
    assert_eq!(hp(&mut h, 3), 558, "cold blast uses base damage");
    assert_eq!(hp(&mut h, 4), 4622, "epic cap applies even at full heat");
    assert!(!h.sim(|sim| Cast::Spin.can_activate(sim, 1)));
    apply(&mut h, Cast::Spin, InputTargetV1::dir(1000, 0));
    for now in 181..225 {
        tick(&mut h, &mut p, now);
    }
    assert_eq!(
        h.effects
            .iter()
            .filter(|(name, _, _)| name == "mlbb_xborg_ult_explosion")
            .count(),
        1
    );
}
