//! Approved TFM2 skill adaptation; all hits use the host's credited pipeline.
use super::*;
const NO_CRIT: &str = "mlbb_hanzo_mist_no_crit";
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum DashKind {
    HumanMist,
    SpiritMist,
    Snatch,
}
#[derive(Clone, Debug)]
pub(super) struct Zone {
    pub from: Pos,
    pub to: Pos,
    pub next: usize,
    pub remaining: usize,
    pub spirit: bool,
    pub damage: usize,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Snatch {
    pub target: EntityKey,
    pub at: usize,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Digest {
    pub next: usize,
    pub remaining: usize,
    pub amount: usize,
}
/// A cast receipt survives callbacks; never assume the damage call updated the read view inline.
#[derive(Clone, Copy, Debug)]
pub(super) struct Feast {
    pub target: EntityKey,
    pub ordinary: bool,
    pub credited: bool,
    before: (usize, usize),
    expires: usize,
    heal: usize,
    digest: usize,
    short_ready: Option<usize>,
    spirit: bool,
}
fn settle_feast(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    let Some(f) = s.feast else {
        return;
    };
    if f.spirit && !s.form.projected() {
        s.feast = None;
        return;
    }
    let same = owned(sim, f.target);
    let observed = same
        && sim.get_entity(f.target.entity_id).is_some_and(|e| {
            if f.ordinary {
                !e.is_alive()
            } else {
                !e.is_alive() || e.hp().0 < f.before.0 || e.shield() < f.before.1
            }
        });
    if f.credited || observed {
        s.feast = None;
        s.marks.remove(&f.target);
        if f.ordinary {
            s.digest = Some(Digest {
                next: sim.tick() + 30,
                remaining: 4,
                amount: f.digest,
            });
        } else {
            if let Some(ready) = f.short_ready {
                s.skill_ready[0][0] = ready;
            }
            if f.heal > 0 {
                sim.heal(id, id, f.heal);
            }
        }
        if s.feast_logs < 8 {
            s.feast_logs += 1;
            trace(
                sim,
                id,
                s,
                &format!(
                    "S1_COMPLETE target={} ordinary={} credited={} digest={} spirit={}",
                    f.target.entity_id, f.ordinary, f.credited, f.ordinary, f.spirit
                ),
            );
        }
    } else if sim.tick() >= f.expires || !same {
        s.feast = None;
        if s.feast_logs < 8 {
            s.feast_logs += 1;
            trace(
                sim,
                id,
                s,
                &format!(
                    "S1_UNCONFIRMED target={} no reward/retry; identity_or_receipt_timeout",
                    f.target.entity_id
                ),
            );
        }
    }
}
pub(super) fn clock(s: &State, slot: usize) -> usize {
    s.skill_ready[usize::from(s.form.projected())][slot]
}
fn effective(sim: &StableSim<'_>, id: usize, slot: SkillSlotV1, base: usize) -> usize {
    sim.get_entity(id)
        .and_then(|e| e.cooldown(slot))
        .filter(|c| c.cooltime > 0)
        .map_or(base, |c| (base.saturating_mul(c.cooltime) / 30).max(1))
}
fn set_clock(s: &mut State, slot: usize, at: usize) {
    s.skill_ready[usize::from(s.form.projected())][slot] = at;
}
pub(super) fn gain(s: &mut State, amount: usize) {
    if let Form::Spirit { energy, .. } = &mut s.form {
        *energy = 1000.min(energy.saturating_add(amount));
    }
}
fn hostile(sim: &StableSim<'_>, owner: usize, target: usize) -> bool {
    let (Some(a), Some(b)) = (sim.get_entity(owner), sim.get_entity(target)) else {
        return false;
    };
    b.is_alive()
        && b.is_targetable()
        && a.team() != b.team()
        && !matches!(b.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus))
}
fn contact(
    sim: &mut StableSim<'_>,
    owner: usize,
    s: &mut State,
    target: usize,
    damage: usize,
    attack: AttackTypeV1,
    nocrit: bool,
) -> bool {
    let Some(t) = sim.get_entity(target) else {
        return false;
    };
    let before = (t.hp().0, t.shield());
    if nocrit {
        // Scoped flat subtraction cancels the actual current chance only for this packet.
        let crit = sim.get_entity(owner).map_or(0, |e| e.stat().crit_chance);
        let mut b = BuffV1::named(NO_CRIT);
        b.crit_chance = -(crit.min(i32::MAX as usize) as i32);
        sim.add_buff(owner, &b);
    }
    sim.deal_damage(owner, target, damage, 0, attack);
    if nocrit {
        sim.entity_remove_buff(owner, NO_CRIT);
    }
    let hit = sim
        .get_entity(target)
        .is_some_and(|t| t.hp().0 < before.0 || t.shield() < before.1);
    if hit && sim.get_entity(target).is_some_and(|t| !t.is_alive()) {
        death_energy(sim, owner, s, target);
    }
    hit
}
pub(super) fn basic(sim: &mut StableSim<'_>, id: usize, s: &mut State, input: InputTargetV1) {
    if input.kind != InputTargetKindV1::Target.code()
        || !resolved_foe(sim, id, input.target_id, true)
    {
        return;
    }
    let target = input.target_id;
    let from = position(sim, id).unwrap();
    let to = position(sim, target).unwrap();
    // The engine validated this attack input using both collision radii.

    s.left = to.0 < from.0;
    let key = crate::entity_key(sim, target);
    let amount = sim.get_entity(id).unwrap().stat().attack;
    let kind = sim.get_entity(target).and_then(|e| e.kind());
    let dealt = contact(sim, id, s, target, amount, AttackTypeV1::BaseAttack, false);
    // The void damage ABI has no receipt. An immediate HP snapshot can miss a
    // queued host packet; mark the accepted native basic contact, including shields.
    if amount > 0 {
        if !matches!(kind, Some(EntityKindV1::Tower | EntityKindV1::Nexus)) {
            mark(s, key, sim.tick());
        }
        gain(
            s,
            match kind {
                Some(EntityKindV1::Champion) => 15,
                Some(EntityKindV1::Epic | EntityKindV1::Serpen) => 45,
                _ => 0,
            },
        );
        visual::fx(sim, id, to, "basic_slash", 10);
    }
    if s.basic_logs < 8 {
        s.basic_logs += 1;
        trace(
            sim,
            id,
            s,
            &format!(
                "BASIC target={target} inline_hp_delta={dealt} center={:.0} edge={:.0}",
                distance(from, to),
                edge_distance(sim, id, target)
            ),
        );
    }
    visual::action(
        s,
        if s.form.projected() {
            "spirit_attack"
        } else {
            "human_attack"
        },
        sim.tick(),
        18,
    );
}
pub(super) fn can_feast(sim: &StableSim<'_>, id: usize, s: &State, target: usize) -> bool {
    if !resolved_foe(sim, id, target, false) {
        return false;
    }
    if s.form.projected() {
        return true;
    }
    let key = crate::entity_key_ro(sim, target);
    let marked = s
        .marks
        .get(&key)
        .is_some_and(|(n, t)| *n == 5 && *t > sim.tick());
    marked
        && sim.get_entity(target).is_some_and(|e| {
            matches!(
                e.kind(),
                Some(
                    EntityKindV1::Champion
                        | EntityKindV1::Epic
                        | EntityKindV1::Serpen
                        | EntityKindV1::Jungle
                )
            )
        })
}
pub(super) fn cast_feast(sim: &mut StableSim<'_>, id: usize, s: &mut State, input: InputTargetV1) {
    if sim.tick() < clock(s, 0)
        || s.snatch.is_some()
        || s.feast.is_some()
        || input.kind != InputTargetKindV1::Target.code()
        || !can_feast(sim, id, s, input.target_id)
    {
        return;
    }
    let target = input.target_id;
    let t = crate::entity_key(sim, target);
    let p = position(sim, id).unwrap();
    let to = position(sim, target).unwrap();
    let spirit = s.form.projected();
    if edge_distance(sim, id, target) > if spirit { 75000. } else { 28000. } {
        return;
    }
    s.left = to.0 < p.0;
    if s.skill_logs < 8 {
        s.skill_logs += 1;
        trace(
            sim,
            id,
            s,
            &format!(
                "S1 cast target={target} spirit={spirit} edge={:.0}",
                edge_distance(sim, id, target)
            ),
        );
    }
    if spirit {
        s.snatch = Some(Snatch {
            target: t,
            at: sim.tick() + 15,
        });
        s.grid.get_or_insert_with(|| grid(sim));
        s.dash = Some(Dash {
            direction: direction(p, to),
            left: 15,
            speed: 5000.,
            kind: DashKind::Snatch,
            hits: vec![],
            refunded: false,
        });
        set_clock(
            s,
            0,
            sim.tick() + effective(sim, id, SkillSlotV1::Skill, 600),
        );
        visual::action(s, "spirit_snatch", sim.tick(), 15);
    } else {
        let ordinary = sim
            .get_entity(target)
            .is_some_and(|e| e.kind() == Some(EntityKindV1::Jungle));
        let l = sim.get_entity(id).unwrap().level().saturating_sub(1);
        let ad = sim.get_entity(id).unwrap().stat().attack;
        let before = sim
            .get_entity(target)
            .map(|e| (e.hp().0, e.shield()))
            .unwrap();
        let max = sim.get_entity(id).unwrap().hp().1;
        s.feast = Some(Feast {
            target: t,
            ordinary,
            credited: false,
            before,
            expires: sim.tick() + 30,
            heal: 50 + 3 * l + ad * 30 / 100,
            digest: max * 5 / 100,
            short_ready: Some(sim.tick() + effective(sim, id, SkillSlotV1::Skill, 288)),
            spirit: false,
        });
        set_clock(
            s,
            0,
            sim.tick() + effective(sim, id, SkillSlotV1::Skill, 720),
        );
        let submitted = if ordinary {
            sim.deal_damage_typed(
                id,
                target,
                before.0.saturating_add(before.1),
                DamageTypeV1::Fixed,
                AttackTypeV1::Skill,
            )
        } else {
            let bonus_ad = bonus(sim, id);
            let damage = if sim.get_entity(target).is_some_and(|e| e.is_champion()) {
                200 + 7 * l + bonus_ad * 115 / 100
            } else {
                170 + 6 * l + bonus_ad
            };
            sim.deal_damage(
                id,
                target,
                damage,
                0,
                AttackTypeV1::Skill,
            );
            true
        };
        if submitted {
            // A bite is the cast visual; rewards wait for a subsequent read/credited kill.
            visual::fx(sim, id, to, "feast_bite", 24);
            settle_feast(sim, id, s);
        } else {
            s.feast = None;
            trace(
                sim,
                id,
                s,
                "S1_SUBMIT_FAILED typed damage unavailable; no reward/retry",
            );
        }
        visual::action(s, "human_feast", sim.tick(), 20);
    }
}
pub(super) fn cast_mist(sim: &mut StableSim<'_>, id: usize, s: &mut State, input: InputTargetV1) {
    if sim.tick() < clock(s, 1) || s.dash.is_some() || s.snatch.is_some() {
        return;
    }
    let from = position(sim, id).unwrap();
    let aim = if input.kind == InputTargetKindV1::Target.code() {
        position(sim, input.target_id).unwrap_or(from)
    } else if input.kind == InputTargetKindV1::Dir.code() {
        (
            (from.0 as i128 + input.dir_x as i128).max(0) as u64,
            (from.1 as i128 + input.dir_y as i128).max(0) as u64,
        )
    } else {
        (input.x, input.y)
    };
    if distance(from, aim) < 1. {
        return;
    }
    let d = direction(from, aim);
    s.left = d.0 < 0.;
    let spirit = s.form.projected();
    s.grid.get_or_insert_with(|| grid(sim));
    s.dash = Some(Dash {
        direction: d,
        left: if spirit { 15 } else { 12 },
        speed: if spirit { 40000. / 15. } else { 2500. },
        kind: if spirit {
            DashKind::SpiritMist
        } else {
            DashKind::HumanMist
        },
        hits: vec![],
        refunded: false,
    });
    s.dash_from = from;
    set_clock(
        s,
        1,
        sim.tick() + effective(sim, id, SkillSlotV1::Skill2, 600),
    );
    if !spirit {
        let mut b = BuffV1::timed("mlbb_hanzo_dark_mist_speed", 120);
        b.move_speed_mult = 15;
        sim.entity_remove_buff(id, b.name());
        sim.add_buff(id, &b);
    }
    visual::action(
        s,
        if spirit { "spirit_dash" } else { "human_dash" },
        sim.tick(),
        if spirit { 15 } else { 12 },
    );
    visual::fx(sim, id, from, "dash_streak", 12);
}
fn segment_distance(p: Pos, a: Pos, b: Pos) -> f64 {
    let dx = b.0 as f64 - a.0 as f64;
    let dy = b.1 as f64 - a.1 as f64;
    let n = dx * dx + dy * dy;
    let t = if n < 1. {
        0.
    } else {
        (((p.0 as f64 - a.0 as f64) * dx + (p.1 as f64 - a.1 as f64) * dy) / n).clamp(0., 1.)
    };
    (p.0 as f64 - (a.0 as f64 + t * dx)).hypot(p.1 as f64 - (a.1 as f64 + t * dy))
}
fn victims(sim: &StableSim<'_>, id: usize, from: Pos, to: Pos) -> Vec<usize> {
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| hostile(sim, id, e.id()) && segment_distance(e.pos(), from, to) <= 18000.)
        .map(|e| e.id())
        .take(64)
        .collect()
}
fn zone(sim: &mut StableSim<'_>, id: usize, s: &mut State, to: Pos, spirit: bool) {
    let e = sim.get_entity(id).unwrap();
    let l = e.level().saturating_sub(1);
    let damage = if spirit { 30 + l } else { 24 + l } + bonus(sim, id) * 20 / 100;
    if s.zones.len() >= 4 {
        s.zones.remove(0);
    }
    s.zones.push(Zone {
        from: s.dash_from,
        to,
        next: sim.tick() + 30,
        remaining: if spirit { 4 } else { 5 },
        spirit,
        damage,
    });
    let middle = ((s.dash_from.0 + to.0) / 2, (s.dash_from.1 + to.1) / 2);
    visual::fx(
        sim,
        id,
        middle,
        if spirit {
            "mist_ground_spirit"
        } else {
            "mist_ground_human"
        },
        if spirit { 120 } else { 150 },
    );
}
pub(super) fn snatch_base(levels_after_one: usize, bonus_ad: usize) -> usize {
    135 + 6 * levels_after_one + bonus_ad * 95 / 100
}

fn resolve_snatch(sim: &mut StableSim<'_>, id: usize, s: &mut State, cast: Snatch) {
    if !owned(sim, cast.target)
        || !s.form.projected()
        || !resolved_foe(sim, id, cast.target.entity_id, false)
    {
        return;
    }
    let target = cast.target.entity_id;
    let to = position(sim, target).unwrap();
    if edge_distance(sim, id, target) > 28000. {
        return;
    }
    let marks = s
        .marks
        .get(&cast.target)
        .filter(|(_, t)| *t > sim.tick())
        .map_or(0, |(n, _)| *n);
    let e = sim.get_entity(id).unwrap();
    let l = e.level().saturating_sub(1);
    let ad = e.stat().attack;
    let b = bonus(sim, id);
    let t = sim.get_entity(target).unwrap();
    let per_mark = if t.is_champion() { 24 } else { 20 };
    let main = snatch_base(l, b) * (100 + per_mark * marks) / 100;
    let extra = if marks == 5 {
        let n = t.hp().1 * (600 + b).min(1200) / 10000;
        if matches!(t.kind(), Some(EntityKindV1::Epic | EntityKindV1::Serpen)) {
            n.min(220)
        } else {
            n
        }
    } else {
        0
    };
    // Keep physical mitigation/kill credit; render the maw independently of the read-view delta.
    let before = sim
        .get_entity(target)
        .map(|e| (e.hp().0, e.shield()))
        .unwrap();
    s.feast = Some(Feast {
        target: cast.target,
        ordinary: false,
        credited: false,
        before,
        expires: sim.tick() + 30,
        heal: if marks == 5 {
            50 + 3 * l + ad * 30 / 100
        } else {
            0
        },
        digest: 0,
        short_ready: None,
        spirit: true,
    });
    sim.deal_damage(id, target, main + extra, 0, AttackTypeV1::Skill);
    visual::fx(sim, id, to, "spirit_maw", 36);
    settle_feast(sim, id, s);
}

pub(super) fn tick(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    settle_feast(sim, id, s);
    let now = sim.tick();
    if let Some(mut d) = s.digest.take() {
        if now >= d.next {
            sim.heal(id, id, d.amount);
            d.remaining -= 1;
            d.next += 30;
        }
        if d.remaining > 0 {
            s.digest = Some(d);
        }
    }
    if let Some(mut dash) = s.dash.take() {
        if !s.form.input_locked() && !blocked(sim, id, true) {
            let p = position(sim, id).unwrap();
            let length = if dash.kind == DashKind::Snatch {
                if let Some(to) = s
                    .snatch
                    .filter(|c| owned(sim, c.target))
                    .and_then(|c| position(sim, c.target.entity_id))
                {
                    dash.direction = direction(p, to);
                    dash.speed.min((distance(p, to) - 16000.).max(0.))
                } else {
                    0.
                }
            } else {
                dash.speed
            };
            let g = s.grid.get_or_insert_with(|| grid(sim));
            let to = legal_step(g, p, dash.direction, length, s.form.projected());
            sim.entity_set_pos(id, to.0, to.1);
            if dash.kind == DashKind::SpiritMist {
                for target in victims(sim, id, p, to) {
                    let k = crate::entity_key(sim, target);
                    if dash.hits.contains(&k) {
                        continue;
                    }
                    if dash.hits.len() >= 64 {
                        break;
                    }
                    dash.hits.push(k);
                    let e = sim.get_entity(id).unwrap();
                    let l = e.level().saturating_sub(1);
                    let b = bonus(sim, id);
                    let hero = sim.get_entity(target).is_some_and(|t| t.is_champion());
                    if contact(
                        sim,
                        id,
                        s,
                        target,
                        110 + 4 * l + b * 55 / 100,
                        AttackTypeV1::Skill,
                        false,
                    ) {
                        mark(s, k, now);
                        sim.apply_cc(target, &CcV1::of_kind(CcKindV1::Airborne, 24));
                        visual::fx(
                            sim,
                            id,
                            position(sim, target).unwrap_or(to),
                            "airborne_contact",
                            24,
                        );
                        if hero && !dash.refunded {
                            gain(s, 200);
                            dash.refunded = true;
                        }
                    }
                }
            }
            dash.left = dash.left.saturating_sub(1);
            if dash.left > 0 && (to != p || dash.kind == DashKind::Snatch) {
                s.dash = Some(dash);
            } else if dash.kind != DashKind::Snatch {
                zone(sim, id, s, to, dash.kind == DashKind::SpiritMist);
            }
        }
    }
    if let Some(cast) = s.snatch {
        if now >= cast.at {
            s.snatch = None;
            resolve_snatch(sim, id, s, cast);
        }
    }
    let mut zones = std::mem::take(&mut s.zones);
    for z in &mut zones {
        if z.spirit && !s.form.projected() {
            z.remaining = 0;
            continue;
        }
        if z.remaining == 0 || now < z.next {
            continue;
        }
        for target in victims(sim, id, z.from, z.to) {
            let k = crate::entity_key(sim, target);
            // One mark per valid positive mist pulse, matching accepted basic-contact bookkeeping.
            // The host can process damage after this callback's immutable read view.
            if z.damage > 0 {
                mark(s, k, now);
            }
            if contact(sim, id, s, target, z.damage, AttackTypeV1::BaseAttack, true) {
                let mut slow = BuffV1::timed(&format!("mlbb_hanzo_mist_slow_{id}"), 36);
                slow.move_speed_mult = -35;
                sim.entity_remove_buff(target, slow.name());
                sim.add_buff(target, &slow);
            }
        }
        visual::fx(
            sim,
            id,
            ((z.from.0 + z.to.0) / 2, (z.from.1 + z.to.1) / 2),
            "mist_pulse",
            9,
        );
        z.remaining -= 1;
        z.next += 30;
    }
    zones.retain(|z| z.remaining > 0);
    s.zones = zones;
}
pub(super) fn observe_deaths(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    if !s.form.projected() || sim.tick() < s.next_deaths {
        return;
    }
    s.next_deaths = sim.tick() + 15;
    let p = position(sim, id).unwrap();
    let ids: Vec<_> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| {
            e.id() != id
                && !matches!(e.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus))
                && distance(p, e.pos()) <= 80000.
        })
        .map(|e| e.id())
        .take(128)
        .collect();
    s.watched.retain(|k, _| owned(sim, *k));
    for target in ids {
        let alive = sim.get_entity(target).is_some_and(|e| e.is_alive());
        let k = if alive {
            crate::entity_key(sim, target)
        } else {
            crate::entity_key_ro(sim, target)
        };
        if s.watched.get(&k) == Some(&true) && !alive {
            let hero = sim.get_entity(target).is_some_and(|e| e.is_champion());
            gain(s, if hero { 200 } else { 20 });
        }
        if s.watched.len() < 128 || s.watched.contains_key(&k) {
            s.watched.insert(k, alive);
        }
    }
}

pub(super) fn death_energy(sim: &StableSim<'_>, id: usize, s: &mut State, victim: usize) {
    if !s.form.projected() {
        return;
    }
    let Some(e) = sim.get_entity(victim) else {
        return;
    };
    if matches!(e.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus))
        || distance(position(sim, id).unwrap_or(s.body), e.pos()) > 80000.
    {
        return;
    }
    let k = crate::entity_key_ro(sim, victim);
    if s.watched.get(&k) == Some(&false) {
        return;
    }
    gain(s, if e.is_champion() { 200 } else { 20 });
    if s.watched.len() >= 128 && !s.watched.contains_key(&k) {
        if let Some(old) = s
            .watched
            .iter()
            .find(|(_, alive)| !**alive)
            .map(|(k, _)| *k)
        {
            s.watched.remove(&old);
        }
    }
    if s.watched.len() < 128 || s.watched.contains_key(&k) {
        s.watched.insert(k, false);
    }
}
