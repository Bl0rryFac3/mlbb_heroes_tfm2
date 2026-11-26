//! Tick-owned combat. Pose playback never schedules damage or collection.
use super::*;

const FIRE_TICKS: usize = 90;
const FIRE_HEAL_CAP: usize = 12;
const STAKE_RECALL: usize = 72;
const STAKE_TRAVEL: usize = 18;
const SPIN_TICKS: usize = 180;
const SPIN_PULSES: usize = 6;
const SPIN_PULSE_TICKS: usize = SPIN_TICKS / SPIN_PULSES;
const STAKE_WIDTH: f64 = 4_000.;
const SPIN_RADIUS: f64 = 32_000.;
const BLAST_RADIUS: f64 = 42_000.;
const SUPPLY_RADIUS: f64 = 12_000.;
const MAX_HEAT: usize = 4;
const WARM_TIME: usize = 180;
const DROP_COOLDOWN: usize = 300;
const SUPPLY_TIME: usize = 360;

#[derive(Clone, Copy)]
struct Target {
    id: usize,
    pos: Pos,
    kind: Option<EntityKindV1>,
    max_hp: usize,
    radius: usize,
}

impl Target {
    fn structure(self) -> bool {
        matches!(self.kind, Some(EntityKindV1::Tower | EntityKindV1::Nexus))
    }
    fn champion(self) -> bool {
        self.kind == Some(EntityKindV1::Champion)
    }
    fn can_heat(self) -> bool {
        self.drops_supply() || matches!(self.kind, Some(EntityKindV1::Epic | EntityKindV1::Serpen))
    }
    fn drops_supply(self) -> bool {
        matches!(
            self.kind,
            Some(EntityKindV1::Champion | EntityKindV1::Jungle)
        )
    }
    fn fire_vamp_percent(self) -> usize {
        if self.champion() {
            10
        } else if self.can_heat() {
            5
        } else {
            0
        }
    }
}

fn of(sim: &StableSim<'_>, team: usize, id: usize) -> Option<Target> {
    let e = sim.get_entity(id)?;
    let (hp, max_hp) = e.hp();
    if !e.is_alive() || hp == 0 || e.team() == team {
        return None;
    }
    let target = Target {
        id,
        pos: e.pos(),
        kind: e.kind().or_else(|| {
            if e.is_champion() {
                Some(EntityKindV1::Champion)
            } else if e.is_tower() {
                Some(EntityKindV1::Tower)
            } else if e.is_minion() {
                Some(EntityKindV1::Minion)
            } else {
                None
            }
        }),
        max_hp,
        radius: e.radius(),
    };
    (e.is_targetable() || target.structure()).then_some(target)
}

fn enemies(sim: &StableSim<'_>, team: usize) -> Vec<Target> {
    (0..sim.entity_count())
        .filter_map(|i| of(sim, team, sim.entity_at(i)?.id()))
        .filter(|t| !t.structure())
        .collect()
}

#[derive(Clone, Copy)]
struct Fire {
    start: usize,
    dir: Direction,
    pulse: usize,
    heal_fraction: usize,
    firaga_fraction: usize,
}

struct Stakes {
    start: usize,
    origin: Pos,
    points: [Pos; 5],
    armored: bool,
    return_to: Option<Pos>,
    progress: usize,
    hit: HashSet<usize>,
}

struct Spin {
    start: usize,
    dir: Direction,
    dash_remaining: u64,
    charge: bool,
    pulse: usize,
    contact: Option<(usize, usize)>,
}

#[derive(Default)]
struct Heat {
    stacks: usize,
    until: usize,
    next_drop: usize,
}

struct Supply {
    where_: Pos,
    until: usize,
    queued: bool,
}

#[derive(Default)]
pub(super) struct Combat {
    fire: Option<Fire>,
    stakes: Option<Stakes>,
    spin: Option<Spin>,
    // This combat state already belongs to one caster in one simulation.
    // Neutral monsters can build heat without a buff or tag on the target.
    heat: HashMap<usize, Heat>,
    supplies: Vec<Supply>,
}

impl Combat {
    pub fn is_firing(&self) -> bool {
        self.fire.is_some()
    }
    pub fn is_spinning(&self) -> bool {
        self.spin.is_some()
    }

    fn overheated(&self, target: usize, now: usize) -> bool {
        self.heat
            .get(&target)
            .is_some_and(|h| h.until > now && h.stacks == MAX_HEAT)
    }

    pub fn body_exclusive(
        &self,
        now: usize,
        full: bool,
        walking: bool,
    ) -> Option<(&'static str, usize, bool)> {
        if let Some(spin) = &self.spin {
            if spin.charge && now < spin.start + 13 {
                return Some(("armored_ult_charge", spin.start, false));
            }
            return Some(("armored_ult_spin", spin.start, true));
        }
        let fire = self.fire?;
        if now < fire.start + 12 {
            return Some((
                visuals::form_name(full, "armored_s1_start", "armorless_s1_start"),
                fire.start,
                false,
            ));
        }
        let name = if walking {
            visuals::form_name(full, "armored_s1_move", "armorless_s1_move")
        } else {
            visuals::form_name(full, "armored_s1_loop", "armorless_s1_loop")
        };
        Some((name, fire.start + 12, true))
    }

    pub fn draw_effects(&self, sim: &mut StableSim<'_>, id: usize, at: Pos, full: bool) {
        let now = sim.tick();
        if let Some(fire) = self.fire {
            let primary = visuals::form_name(full, "s1_armored_flame", "s1_armorless_flame");
            let index = (now - fire.start) / 4 % if full { 5 } else { 9 };
            let muzzle = advance(at, fire.dir, if full { 15_000. } else { 18_000. });
            let muzzle = (muzzle.0, muzzle.1.saturating_sub(22_000));
            visuals::effect(
                sim,
                id,
                &format!("{primary}_{}_{index}", visuals::angle(fire.dir)),
                muzzle,
                4,
            );
        }
        if let Some(stakes) = &self.stakes {
            let age = now - stakes.start;
            let frame = age / 4 % 9;
            for end in stakes.points {
                let from = stakes.origin;
                let (point, direction, name) = match stakes.return_to {
                    Some(to) => (
                        lerp(end, to, stakes.progress, STAKE_TRAVEL),
                        delta(end, to),
                        "s2_recall_trail",
                    ),
                    None => (
                        lerp(from, end, age.min(STAKE_TRAVEL), STAKE_TRAVEL),
                        delta(from, end),
                        "s2_fire_stake",
                    ),
                };
                visuals::effect(
                    sim,
                    id,
                    &format!("{name}_{}_{frame}", visuals::angle(direction)),
                    point,
                    4,
                );
            }
        }
        for supply in &self.supplies {
            visuals::effect(
                sim,
                id,
                &format!("firaga_supply_{}", (now / 6) % 9),
                supply.where_,
                4,
            );
        }
        let mut marks: Vec<_> = self
            .heat
            .iter()
            .filter(|(_, h)| h.stacks > 0 && h.until > now)
            .map(|(&target, h)| (target, h.stacks))
            .collect();
        marks.sort_unstable();
        for (target, count) in marks {
            if let Some(target_pos) = position(sim, target) {
                visuals::effect(
                    sim,
                    id,
                    &format!("heat_{count}"),
                    (target_pos.0, target_pos.1.saturating_sub(50_000)),
                    4,
                );
            }
        }
    }
}

pub(super) fn allows(cast: Cast, state: &State) -> bool {
    match cast {
        Cast::Basic => !state.combat.is_spinning() && !state.combat.is_firing(),
        Cast::Fire => !state.combat.is_spinning() && !state.combat.is_firing(),
        Cast::Stakes => !state.combat.is_spinning() && state.combat.stakes.is_none(),
        Cast::Spin => state.armor.armored() && !state.combat.is_spinning(),
        Cast::BasicHit => true,
    }
}

pub(super) fn fire_damage(stat: StatV1) -> usize {
    20 + stat.attack * 20 / 100 + stat.magic_power * 15 / 100
}
pub(super) fn stake_damage(stat: StatV1) -> usize {
    50 + stat.attack * 40 / 100
}
pub(super) fn spin_damage(stat: StatV1) -> usize {
    22 + stat.attack * 15 / 100 + stat.magic_power * 12 / 100
}

fn blast_damage(target: Target, overheated: bool, forced: bool) -> usize {
    let base = 100 + target.max_hp * 6 / 100;
    let damage = if overheated { base + base / 2 } else { base };
    let damage = if matches!(target.kind, Some(EntityKindV1::Epic | EntityKindV1::Serpen)) {
        damage.min(150)
    } else {
        damage
    };
    if forced {
        damage / 2
    } else {
        damage
    }
}

fn lerp(a: Pos, b: Pos, n: usize, total: usize) -> Pos {
    let fraction = n as f64 / total as f64;
    let (x, y) = delta(a, b);
    advance(a, (x, y), fraction)
}

fn segment_gap(p: Pos, a: Pos, b: Pos) -> f64 {
    let (x, y) = delta(a, b);
    let (px, py) = delta(a, p);
    let frac = ((px * x + py * y) / (x * x + y * y).max(1.)).clamp(0., 1.);
    (px - x * frac).hypot(py - y * frac)
}

fn in_fire(origin: Pos, dir: Direction, target: Target, full: bool) -> bool {
    let (reach, half_angle) = if full {
        (50_000., 35_f64)
    } else {
        (65_000., 18_f64)
    };
    let (sin, cos) = half_angle.to_radians().sin_cos();
    let (x, y) = delta(origin, target.pos);
    let length = x.hypot(y);
    let radius = target.radius as f64;
    if length > reach + radius {
        return false;
    }
    let forward = x * dir.0 + y * dir.1;
    if forward >= length * cos {
        return true;
    }
    // Outside the angular span, the closest point is on the nearer side of
    // the sector. Clamp to the finite side so the rounded tip counts too.
    let sideways = (y * dir.0 - x * dir.1).abs();
    let along = (forward * cos + sideways * sin).clamp(0., reach);
    (forward - along * cos).hypot(sideways - along * sin) <= radius
}

fn mark_and_drop(s: &mut State, target: Target, now: usize) {
    if !target.can_heat() {
        return;
    }
    let mark = s.combat.heat.entry(target.id).or_default();
    if mark.until <= now {
        mark.stacks = 0;
    }
    if mark.stacks == MAX_HEAT
        && target.drops_supply()
        && mark.next_drop <= now
        && s.combat.supplies.len() < 6
    {
        mark.next_drop = now + DROP_COOLDOWN;
        s.combat.supplies.push(Supply {
            where_: target.pos,
            until: now + SUPPLY_TIME,
            queued: false,
        });
    }
    mark.stacks = (mark.stacks + 1).min(MAX_HEAT);
    mark.until = now + WARM_TIME;
}

fn skill_hit(
    sim: &mut StableSim<'_>,
    id: usize,
    s: &mut State,
    target: Target,
    damage: usize,
) -> Option<usize> {
    if !alive(sim, id) {
        return None;
    }
    let enemy = sim.get_entity(target.id)?;
    let before = enemy.hp().0;
    if !enemy.is_alive() || before == 0 {
        return None;
    }
    let now = sim.tick();
    let hot = s.combat.overheated(target.id, now);
    let kind = if hot {
        DamageTypeV1::Fixed
    } else {
        DamageTypeV1::Ad
    };
    if !sim.deal_damage_typed(id, target.id, damage, kind, AttackTypeV1::Skill) {
        return None;
    }
    let remaining = sim.get_entity(target.id).map(|e| e.hp().0);
    if remaining.is_some() {
        mark_and_drop(s, target, now);
    }
    Some(before.saturating_sub(remaining.unwrap_or(0)).min(damage))
}

fn collect(s: &mut State, now: usize, at: Pos, radius: f64, include_queued: bool) -> usize {
    let mut count = 0;
    s.combat.supplies.retain(|supply| {
        let found = supply.until > now
            && (dist(at, supply.where_) <= radius || include_queued && supply.queued);
        count += usize::from(found);
        supply.until > now && !found
    });
    for _ in 0..count {
        s.armor.pickup();
    }
    count
}

pub(super) fn cast(
    cast: Cast,
    sim: &mut StableSim<'_>,
    id: usize,
    input: InputTargetV1,
    s: &mut State,
    seed: u64,
) {
    let now = sim.tick();
    if !allows(cast, s) || cast != Cast::BasicHit && interrupted(sim, id, false) {
        return;
    }
    let Some(caster) = sim.get_entity(id) else {
        return;
    };
    let (from, team, stat) = (caster.pos(), caster.team(), caster.stat());
    match cast {
        Cast::Basic | Cast::BasicHit => {
            if input.kind != InputTargetKindV1::Target.code() {
                return;
            }
            let Some(t) = of(sim, team, input.target_id) else {
                return;
            };
            if cast == Cast::Basic {
                if !t.structure() && dist(from, t.pos) > cast.reach() as f64 + t.radius as f64 {
                    return;
                }
                if let Some(d) = unit(delta(from, t.pos)) {
                    s.visual.face(d);
                }
                s.visual.play(
                    visuals::form_name(
                        s.armor.armored(),
                        "armored_basic_attack",
                        "armorless_basic_attack",
                    ),
                    now,
                );
                voice::play(
                    sim,
                    id,
                    voice::BASIC[seed as usize % voice::BASIC.len()],
                    false,
                );
                if t.structure() {
                    // The host selected this structure using its own attack range.
                    // Resolve once through the ordinary attack path, including defenses.
                    sim.deal_damage(id, t.id, stat.attack, 0, AttackTypeV1::BaseAttack);
                    visuals::effect(sim, id, "projectile_impact", t.pos, 15);
                    trace(sim, id, &format!("basic against structure target={}", t.id));
                    return;
                }
                if !s.armor.armored()
                    && sim.spawn_projectile(
                        "mlbb_xborg_basic_projectile",
                        "mlbb_heroes:xborg_basic_hit",
                        &ProjectileSpawnV1 {
                            caster_id: id,
                            team,
                            x: from.0,
                            y: from.1.saturating_sub(12_000),
                            radius: 1800,
                            speed: 5400,
                            target_id: t.id,
                            target_x: t.pos.0,
                            target_y: t.pos.1,
                            attack_type: AttackTypeV1::BaseAttack.code(),
                            ..ProjectileSpawnV1::default()
                        },
                    )
                {
                    return;
                }
            }
            let landed = sim.deal_damage_typed(
                id,
                t.id,
                stat.attack,
                DamageTypeV1::Ad,
                AttackTypeV1::BaseAttack,
            );
            if landed && sim.get_entity(t.id).is_some() {
                mark_and_drop(s, t, now);
            }
            visuals::effect(sim, id, "projectile_impact", t.pos, 15);
        }
        Cast::Fire | Cast::Stakes | Cast::Spin => {
            let Some(dir) = aiming(sim, from, input) else {
                return;
            };
            if cast != Cast::Stakes || !s.combat.is_firing() {
                s.visual.face(dir);
            }
            let full = s.armor.armored();
            match cast {
                Cast::Fire => {
                    voice::play(sim, id, "mlbb_xborg_voice_skill01", false);
                    s.combat.fire = Some(Fire {
                        start: now,
                        dir,
                        pulse: 0,
                        heal_fraction: 0,
                        firaga_fraction: 0,
                    });
                    trace(sim, id, "S1 start");
                }
                Cast::Stakes => {
                    voice::play(sim, id, "mlbb_xborg_voice_skill02", false);
                    let spread: [f64; 5] = if full {
                        [-30., -15., 0., 15., 30.]
                    } else {
                        [-16., -8., 0., 8., 16.]
                    };
                    let heading = dir.1.atan2(dir.0);
                    let reach = if full { 55_000. } else { 70_000. };
                    let points = spread.map(|off| {
                        let angle = heading + off.to_radians();
                        advance(from, (angle.cos(), angle.sin()), reach)
                    });
                    s.combat.stakes = Some(Stakes {
                        start: now,
                        origin: from,
                        points,
                        armored: full,
                        return_to: None,
                        progress: 0,
                        hit: HashSet::new(),
                    });
                    s.visual.play(
                        visuals::form_name(full, "armored_s2_cast", "armorless_s2_cast"),
                        now,
                    );
                    trace(
                        sim,
                        id,
                        if full {
                            "S2 launch armored stakes=5"
                        } else {
                            "S2 launch armorless stakes=5"
                        },
                    );
                }
                Cast::Spin => {
                    voice::play(sim, id, voice::ULT[seed as usize % voice::ULT.len()], true);
                    s.combat.fire = None;
                    s.combat.spin = Some(Spin {
                        start: now,
                        dir,
                        dash_remaining: 42_000,
                        charge: true,
                        pulse: 0,
                        contact: None,
                    });
                    let mut buff = BuffV1::timed(SPIN_SPEED, SPIN_TICKS + 2);
                    buff.move_speed_mult = -20;
                    sim.add_buff(id, &buff);
                    trace(sim, id, "R start");
                }
                _ => unreachable!(),
            }
        }
    }
}

fn fire_tick(
    sim: &mut StableSim<'_>,
    id: usize,
    s: &mut State,
    at: Pos,
    team: usize,
    stat: StatV1,
    now: usize,
) {
    let Some(mut fire) = s.combat.fire.take() else {
        return;
    };
    if interrupted(sim, id, false) {
        trace(sim, id, "S1 interrupted");
        return;
    }
    let age = now.saturating_sub(fire.start);
    let full = s.armor.armored();
    let raw = fire_damage(stat);
    let damage = if full { raw } else { raw * 60 / 100 };
    if fire.pulse < 6 && age >= fire.pulse * 15 {
        fire.pulse += 1;
        let targets = enemies(sim, team)
            .into_iter()
            .filter(|t| in_fire(at, fire.dir, *t, full));
        let mut hits = 0;
        let mut weighted_damage = 0;
        for t in targets {
            let value = if t.kind == Some(EntityKindV1::Minion) {
                damage * 70 / 100
            } else {
                damage
            };
            if let Some(hp_lost) = skill_hit(sim, id, s, t, value) {
                hits += 1;
                weighted_damage += hp_lost * t.fire_vamp_percent();
            }
        }
        let hundredths = fire.heal_fraction + weighted_damage;
        fire.heal_fraction = hundredths % 100;
        let heal = (hundredths / 100).min(FIRE_HEAL_CAP);
        // Restore half the built-in body healing as reserve, or one quarter
        // as recharge energy. Keep the fractional credit between pulses.
        let firaga_divisor = if full { 2 } else { 4 };
        let firaga = fire.firaga_fraction + weighted_damage;
        let firaga_unit = 100 * firaga_divisor;
        let recovered = (firaga / firaga_unit).min(FIRE_HEAL_CAP / firaga_divisor);
        fire.firaga_fraction = firaga % firaga_unit;
        if alive(sim, id) {
            s.armor.recover(recovered);
        }
        if heal > 0 {
            if let Some(caster) = sim.get_entity(id).filter(|e| e.is_alive()) {
                let (hp, max) = caster.hp();
                if hp > 0 && hp < max {
                    sim.heal(id, id, heal.min(max - hp));
                }
            }
        }
        trace(
            sim,
            id,
            &format!("S1 pulse={}/6 hits={hits} firaga={recovered}", fire.pulse),
        );
    }
    if age < FIRE_TICKS - 1 {
        s.combat.fire = Some(fire);
    } else if !s.visual.action_active(now) {
        s.visual.play(
            visuals::form_name(full, "armored_s1_recover", "armorless_s1_recover"),
            now,
        );
    }
}

fn stakes_tick(
    sim: &mut StableSim<'_>,
    id: usize,
    s: &mut State,
    at: Pos,
    team: usize,
    stat: StatV1,
    now: usize,
) {
    let Some(mut stakes) = s.combat.stakes.take() else {
        return;
    };
    if now < stakes.start + STAKE_RECALL {
        s.combat.stakes = Some(stakes);
        return;
    }
    let to = *stakes.return_to.get_or_insert(at);
    let progress = (now - (stakes.start + STAKE_RECALL) + 1).min(STAKE_TRAVEL);
    if stakes.progress == 0 {
        if !s.combat.is_spinning() && !s.combat.is_firing() && !s.visual.action_active(now) {
            let name = visuals::form_name(
                s.armor.armored(),
                "armored_s2_recall",
                "armorless_s2_recall",
            );
            s.visual.play(name, now);
        }
        trace(sim, id, "S2 recall paths=5");
    }
    let paths = stakes.points.map(|end| {
        (
            lerp(end, to, stakes.progress, STAKE_TRAVEL),
            lerp(end, to, progress, STAKE_TRAVEL),
        )
    });
    let on_path = |p: Pos, width: f64| paths.iter().any(|(a, b)| segment_gap(p, *a, *b) <= width);
    for t in enemies(sim, team) {
        if alive(sim, id)
            && on_path(t.pos, STAKE_WIDTH + (t.radius.min(4_000) as f64))
            && stakes.hit.insert(t.id)
        {
            if skill_hit(sim, id, s, t, stake_damage(stat)).is_some() {
                if alive(sim, t.id) {
                    sim.entity_pull(id, t.id, if stakes.armored { 1500 } else { 1000 }, 12);
                }
                visuals::effect(sim, id, "projectile_impact", t.pos, 15);
            }
        }
    }
    let spinning = s.combat.is_spinning();
    let mut supplies = 0;
    s.combat.supplies.retain_mut(|supply| {
        if !on_path(supply.where_, STAKE_WIDTH) {
            return true;
        }
        if spinning {
            supply.queued = true;
            return true;
        }
        supplies += 1;
        false
    });
    for _ in 0..supplies {
        s.armor.pickup();
    }
    stakes.progress = progress;
    if progress < STAKE_TRAVEL {
        s.combat.stakes = Some(stakes);
    } else {
        trace(sim, id, &format!("S2 landed unique={}", stakes.hit.len()));
    }
}

fn slow(sim: &mut StableSim<'_>, target: usize, amount: i32, ticks: usize) {
    const NAME: &str = "mlbb_xborg_r_slow";
    sim.entity_remove_buff(target, NAME);
    let mut buff = BuffV1::timed(NAME, ticks);
    buff.move_speed_mult = -amount;
    sim.add_buff(target, &buff);
}

fn spin_tick(
    sim: &mut StableSim<'_>,
    id: usize,
    s: &mut State,
    at: Pos,
    team: usize,
    stat: StatV1,
    now: usize,
) {
    let Some(mut spin) = s.combat.spin.take() else {
        return;
    };
    let age = now - spin.start;
    let targets = enemies(sim, team);
    let mut center = at;
    if spin.charge {
        let step = spin.dash_remaining.min(3500);
        if !interrupted(sim, id, true) && step > 0 {
            let to = clear_ground(sim, s, at, spin.dir, step);
            let collision = targets
                .iter()
                .filter(|t| t.champion() && segment_gap(t.pos, at, to) <= (t.radius + 7000) as f64)
                .min_by(|a, b| dist(at, a.pos).total_cmp(&dist(at, b.pos)));
            if let Some(t) = collision {
                spin.charge = false;
                spin.contact = Some((t.id, now + 30));
                slow(sim, t.id, 40, 30);
                trace(sim, id, "R contact dash stopped; spin continues");
            } else if to != at {
                sim.entity_set_pos(id, to.0, to.1);
                center = to;
            } else {
                spin.charge = false;
            }
            spin.dash_remaining -= step;
        } else {
            spin.charge = false;
        }
        if spin.dash_remaining == 0 {
            spin.charge = false;
        }
    }
    if spin.pulse < SPIN_PULSES && age >= spin.pulse * SPIN_PULSE_TICKS {
        spin.pulse += 1;
        for &t in targets
            .iter()
            .filter(|t| dist(center, t.pos) <= SPIN_RADIUS)
        {
            let _ = skill_hit(sim, id, s, t, spin_damage(stat));
            if alive(sim, t.id)
                && !spin
                    .contact
                    .is_some_and(|(who, until)| who == t.id && now < until)
            {
                slow(sim, t.id, 25, SPIN_PULSE_TICKS);
            }
        }
        trace(sim, id, &format!("R pulse={}/{SPIN_PULSES}", spin.pulse));
    }
    if alive(sim, id) && age >= SPIN_TICKS {
        detonate(sim, id, s, spin.dir, center, team, false);
    } else if alive(sim, id) {
        s.combat.spin = Some(spin);
    }
}

fn detonate(
    sim: &mut StableSim<'_>,
    id: usize,
    s: &mut State,
    dir: Direction,
    center: Pos,
    team: usize,
    forced: bool,
) {
    let now = sim.tick();
    s.armor.spend(now);
    sim.entity_remove_buff(id, ARMOR);
    sim.entity_remove_buff(id, SPIN_SPEED);
    let mut hot_hits = 0;
    for t in enemies(sim, team)
        .into_iter()
        .filter(|t| dist(center, t.pos) <= BLAST_RADIUS)
    {
        if !alive(sim, id) {
            break;
        }
        let hot = s.combat.overheated(t.id, now);
        let landed = sim.deal_damage_typed(
            id,
            t.id,
            blast_damage(t, hot, forced),
            DamageTypeV1::Fixed,
            AttackTypeV1::Skill,
        );
        hot_hits += usize::from(landed && hot);
    }
    if !alive(sim, id) {
        return;
    }
    collect(s, now, center, BLAST_RADIUS, true);
    if forced {
        s.visual.play("armor_break", now);
        visuals::effect(sim, id, "ult_explosion_forced", center, 15);
    } else {
        s.visual.play("armored_ult_explosion", now);
        visuals::effect(sim, id, "ult_explosion", center, 26);
    }
    recoil(sim, id, s, (-dir.0, -dir.1));
    let reason = if forced { "break" } else { "timer" };
    trace(
        sim,
        id,
        &format!("R detonation {reason} overheated={hot_hits}"),
    );
}

pub(super) fn broken(sim: &mut StableSim<'_>, id: usize, s: &mut State, attacker: usize) {
    let Some(caster) = sim.get_entity(id) else {
        return;
    };
    let (at, team) = (caster.pos(), caster.team());
    if let Some(spin) = s.combat.spin.take() {
        detonate(sim, id, s, spin.dir, at, team, true);
        return;
    }
    let now = sim.tick();
    let away = position(sim, attacker).and_then(|p| unit(delta(p, at)));
    if let Some(dir) = away {
        recoil(sim, id, s, dir);
    }
    visuals::effect(sim, id, "armor_break", at, 28);
    s.visual.play("armor_break", now);
    trace(sim, id, "Firaga break");
}

pub(super) fn update(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    let Some(caster) = sim.get_entity(id) else {
        return;
    };
    let (at, team, stat, now) = (caster.pos(), caster.team(), caster.stat(), sim.tick());
    s.combat
        .heat
        .retain(|&target, heat| (heat.until > now || heat.next_drop > now) && alive(sim, target));
    s.combat.supplies.retain(|supply| supply.until > now);
    fire_tick(sim, id, s, at, team, stat, now);
    if !alive(sim, id) {
        return;
    }
    stakes_tick(sim, id, s, at, team, stat, now);
    if !alive(sim, id) {
        return;
    }
    spin_tick(sim, id, s, at, team, stat, now);
    if alive(sim, id) && !s.combat.is_spinning() {
        collect(s, now, position(sim, id).unwrap_or(at), SUPPLY_RADIUS, true);
    }
}
