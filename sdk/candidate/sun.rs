//! Sun's two summoned bodies follow his actions. The host still drives the owner.
use crate::{audio, entity_key, entity_key_ro, fanny_flight, units, EntityKey, Grid};
use mod_api_stable::*;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

type Point = (f64, f64);

const CLONE_UNIT: &str = "mlbb_sun_clone";
const SHRED: &str = "mlbb_sun_armor_shred";
const SLOW: &str = "mlbb_sun_converge_slow";
const STAFF_RANGE: f64 = 54_000.0;
const STAFF_SPEED: f64 = 3_000.0;
const STRIKE_RANGE: f64 = 46_000.0;
const BASIC_RANGE: f64 = 22_000.0;
const CLONE_LEASH: f64 = 68_000.0;
const STAFF_TRAVEL_TICKS: usize = 18;
const STRIKE_WINDUP: usize = 8;
const CAMP_BONUS: usize = 12;
const MOVE_LINES: &[&str] = &[
    "mlbb_sun_voice_move01",
    "mlbb_sun_voice_move02",
    "mlbb_sun_voice_move03",
    "mlbb_sun_voice_move04",
    "mlbb_sun_voice_move05",
    "mlbb_sun_voice_move06",
    "mlbb_sun_voice_move07",
    "mlbb_sun_voice_move08",
    "mlbb_sun_voice_move09",
    "mlbb_sun_voice_move10",
    "mlbb_sun_voice_move11",
];
const BASIC_LINES: &[&str] = &[
    "mlbb_sun_voice_basic01",
    "mlbb_sun_voice_basic02",
    "mlbb_sun_voice_basic03",
];
const KILL_LINES: &[&str] = &[
    "mlbb_sun_voice_kill01",
    "mlbb_sun_voice_kill02",
    "mlbb_sun_voice_kill03",
];
const DEATH_LINES: &[&str] = &["mlbb_sun_voice_death01", "mlbb_sun_voice_death02"];
const RESPAWN_LINES: &[&str] = &["mlbb_sun_voice_respawn01", "mlbb_sun_voice_respawn02"];
const ULT_LINES: &[&str] = &["mlbb_sun_voice_ult01", "mlbb_sun_voice_ult02"];

#[derive(Clone, Copy)]
enum Slot {
    Staff = 0,
    Ultimate = 1,
}

impl Slot {
    fn settings(self) -> (usize, usize, usize, usize, usize, SkillSlotV1) {
        match self {
            Self::Staff => (300, 390, 42, 55, 50, SkillSlotV1::Skill),
            Self::Ultimate => (480, 1920, 65, 75, 20, SkillSlotV1::Ult),
        }
    }
}

struct CloneBody {
    id: usize,
    expires: usize,
    seen: bool,
    pos: Point,
    attack: usize,
    power: usize,
    next_basic: usize,
}

struct Staff {
    from: Point,
    aim: Point,
    travel: f64,
    exchange: bool,
    seen: Vec<usize>,
}

struct Converge {
    target: usize,
    due: usize,
    bodies: Vec<usize>,
}

#[derive(Default)]
struct State {
    clones: [Option<CloneBody>; 2],
    staff: Option<Staff>,
    strike: Option<Converge>,
    intent: Option<(usize, usize)>,
    grid: Option<Arc<Grid>>,
    basic_b: bool,
    spawn_pos: Option<Point>,
    last_pos: Option<Point>,
    updated: Option<usize>,
}

static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> = LazyLock::new(Mutex::default);

fn take(key: EntityKey) -> State {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key)
        .unwrap_or_default()
}

fn save(sim: &mut StableSim<'_>, key: EntityKey, state: State) {
    if sim.is_end() || sim.get_entity(key.entity_id).is_none_or(|e| !e.is_alive()) {
        cleanup(sim, state);
    } else {
        STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, state);
    }
}

fn cleanup(sim: &mut StableSim<'_>, state: State) {
    for body in state.clones.into_iter().flatten() {
        if is_clone(sim, body.id) {
            sim.entity_set_hp(body.id, 0);
        }
    }
}

fn is_clone(sim: &StableSim<'_>, id: usize) -> bool {
    sim.get_entity(id)
        .is_some_and(|e| e.name().as_deref() == Some(CLONE_UNIT))
}

fn grid(sim: &StableSim<'_>, state: &mut State) -> Arc<Grid> {
    Arc::clone(
        state
            .grid
            .get_or_insert_with(|| Arc::new(fanny_flight::grid(sim, None, &units(sim)))),
    )
}

fn pos(sim: &StableSim<'_>, id: usize) -> Option<Point> {
    sim.get_entity(id).filter(|e| e.is_alive()).map(|e| {
        let (x, y) = e.pos();
        (x as f64, y as f64)
    })
}

fn direction(a: Point, b: Point) -> Option<Point> {
    let n = crate::distance(a, b);
    (n > 1.0).then(|| ((b.0 - a.0) / n, (b.1 - a.1) / n))
}

fn along(from: Point, aim: Point, distance: f64) -> Point {
    (from.0 + aim.0 * distance, from.1 + aim.1 * distance)
}

fn input(p: Point) -> InputTargetV1 {
    InputTargetV1::pos(crate::world_u64(p.0), crate::world_u64(p.1))
}

fn clear(grid: &Grid, p: Point) -> bool {
    [
        (0., 0.),
        (4_000., 0.),
        (-4_000., 0.),
        (0., 4_000.),
        (0., -4_000.),
    ]
    .into_iter()
    .all(|d| !grid.solid(p.0 + d.0, p.1 + d.1))
}

fn approach(grid: &Grid, start: Point, end: Point, max_step: f64) -> Point {
    let Some(aim) = direction(start, end) else {
        return start;
    };
    let length = crate::distance(start, end).min(max_step);
    let samples = (length / 1_000.).ceil().max(1.) as usize;
    let mut last = start;
    for i in 1..=samples {
        let next = along(start, aim, length * i as f64 / samples as f64);
        if !clear(grid, next) {
            break;
        }
        last = next;
    }
    last
}

fn move_to(sim: &mut StableSim<'_>, id: usize, destination: Point) {
    let p = input(destination);
    sim.entity_set_pos(id, p.x, p.y);
}

fn visible_target(sim: &StableSim<'_>, owner: usize, target: usize) -> Option<EntityKindV1> {
    let (owner, target) = (sim.get_entity(owner)?, sim.get_entity(target)?);
    (owner.is_alive()
        && target.is_alive()
        && target.is_targetable()
        && owner.team() != target.team()
        && sim.is_visible(owner.team(), target.id()))
    .then(|| target.kind())
    .flatten()
}

fn structure(kind: EntityKindV1) -> bool {
    kind == EntityKindV1::Tower || kind == EntityKindV1::Nexus
}

fn blocked(sim: &StableSim<'_>, id: usize, moving: bool) -> bool {
    sim.get_entity(id).is_none_or(|e| {
        (0..e.cc_count()).filter_map(|i| e.cc_at(i)).any(|cc| {
            cc.tick > 0
                && match CcKindV1::from_code(cc.kind) {
                    Some(
                        CcKindV1::Stun
                        | CcKindV1::Airborne
                        | CcKindV1::ForceMove
                        | CcKindV1::Fear
                        | CcKindV1::Charm
                        | CcKindV1::Taunt,
                    ) => true,
                    Some(CcKindV1::Bind) => moving,
                    Some(CcKindV1::BlockAttack) => !moving,
                    _ => false,
                }
        })
    })
}

fn animate(sim: &mut StableSim<'_>, id: usize, tag: &str, ticks: u64) {
    let mut cc = CcV1::of_kind(CcKindV1::Animation, ticks);
    cc.set_name(tag);
    sim.apply_cc(id, &cc);
}

fn is_match_view(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|origin| SimOriginKindV1::from_code(origin.kind))
        .is_some_and(|kind| {
            matches!(
                kind,
                SimOriginKindV1::ClientMatchView
                    | SimOriginKindV1::ClientSpectate
                    | SimOriginKindV1::ClientReplay
            )
        })
}

fn view(sim: &mut StableSim<'_>, owner: usize, name: &str, p: InputTargetV1) {
    if is_match_view(sim) {
        sim.play_view_effect(name, owner, &p, 0, 0, 0);
    }
}

fn sound(sim: &mut StableSim<'_>, owner: usize, name: &str, p: InputTargetV1) {
    if is_match_view(sim) {
        sim.play_sfx(name, owner, &p);
    }
}

fn attack_stat(sim: &StableSim<'_>, id: usize) -> usize {
    sim.get_entity(id).map_or(0, |e| e.stat().attack)
}

fn damage(
    sim: &mut StableSim<'_>,
    owner: usize,
    body: usize,
    target: usize,
    raw: usize,
    kind: AttackTypeV1,
) -> bool {
    let Some(target_kind) = visible_target(sim, owner, target) else {
        return false;
    };
    let Some(before) = sim
        .get_entity(target)
        .map(|e| e.hp().0.saturating_add(e.shield()))
    else {
        return false;
    };
    sim.deal_damage(body, target, raw, 0, kind);
    let after = sim
        .get_entity(target)
        .map_or(0, |e| e.hp().0.saturating_add(e.shield()));
    if after >= before {
        return false;
    }
    view(
        sim,
        owner,
        "sun_staff_hit_spark",
        InputTargetV1::target(target),
    );
    if structure(target_kind) {
        return true;
    }
    if sim.get_entity(target).is_some_and(|e| e.is_alive()) {
        let mut buff = BuffV1::timed(SHRED, 240);
        buff.defence_mult = -5;
        sim.entity_stack_buff(target, &buff, 6, true);
    }
    true
}

fn heal_from_clone(sim: &mut StableSim<'_>, owner: usize, attack: usize) {
    if sim.get_entity(owner).is_some_and(|e| {
        let (hp, max) = e.hp();
        e.is_alive() && hp < max
    }) {
        sim.heal(owner, owner, 10 + attack * 8 / 100);
        view(
            sim,
            owner,
            "sun_simian_heal_pulse",
            InputTargetV1::target(owner),
        );
    }
}

fn spawn(sim: &mut StableSim<'_>, owner: usize, state: &mut State, slot: Slot, p: Point) -> bool {
    let Some(e) = sim.get_entity(owner).filter(|e| e.is_alive()) else {
        return false;
    };
    let (life, _, power, hp, amplify, _) = slot.settings();
    let team = e.team();
    let mut stat = e.stat();
    stat.hp = (stat.hp * hp / 100).max(1);
    stat.hp_regen = 0;
    stat.crit_chance = 0;
    // Attacks are directed by the owner, so the carrier has no autonomous attack.
    stat.move_speed = 0;
    let attack = UnitAttackV1 {
        attack_ratio: 0,
        attack: 0,
        range: 0,
        cooltime: 360_000,
        ..UnitAttackV1::default()
    };
    let p_input = input(p);
    let Some(id) = sim.spawn_unit(
        CLONE_UNIT,
        owner,
        team,
        p_input.x,
        p_input.y,
        life as u64,
        &stat,
        &attack,
    ) else {
        return false;
    };
    let now = sim.tick();
    let seen = is_clone(sim, id);
    let mut body = CloneBody {
        id,
        expires: now + life,
        seen,
        pos: p,
        attack: stat.attack * power / 100,
        power,
        next_basic: now + 14,
    };
    if seen {
        mark_clone(sim, &mut body, amplify);
    }
    state.clones[slot as usize] = Some(body);
    let cue = match slot {
        Slot::Staff => "sun_clone_spawn_smoke",
        Slot::Ultimate => "sun_clone_summon",
    };
    view(sim, owner, cue, p_input);
    true
}

fn mark_clone(sim: &mut StableSim<'_>, body: &mut CloneBody, amplify: usize) {
    let mut buff = BuffV1::named("mlbb_sun_clone_fragility");
    buff.damaged_amplify = amplify;
    sim.add_buff(body.id, &buff);
    body.seen = true;
}

fn start_cooldown(sim: &mut StableSim<'_>, owner: usize, slot: Slot) {
    let (life, base, _, _, _, skill) = slot.settings();
    if let Some(cd) = sim.get_entity(owner).and_then(|e| e.cooldown(skill)) {
        let travel = if matches!(slot, Slot::Staff) {
            STAFF_TRAVEL_TICKS
        } else {
            0
        };
        let effective_base =
            (cd.cooltime * base + (life + base + travel) / 2) / (life + base + travel);
        sim.entity_reduce_cooldown(owner, skill, cd.remaining.saturating_sub(effective_base));
    }
}

fn safe_exchange(sim: &StableSim<'_>, owner: usize, grid: &Grid, from: Point, end: Point) -> bool {
    let Some(e) = sim.get_entity(owner) else {
        return false;
    };
    let (hp, max) = e.hp();
    hp * 100 >= max * 65
        && !blocked(sim, owner, true)
        && crate::distance(approach(grid, from, end, STAFF_RANGE), end) <= 1.
        && !(0..sim.tower_count())
            .filter_map(|i| sim.get_entity(sim.tower_id_at(i)))
            .any(|tower| {
                let p = tower.pos();
                tower.team() != e.team()
                    && tower.is_alive()
                    && crate::segment_distance((p.0 as f64, p.1 as f64), from, end)
                        <= fanny_flight::TOWER_ATTACK_RANGE
            })
}

fn staff_tick(sim: &mut StableSim<'_>, owner: usize, state: &mut State, mut staff: Staff) {
    let grid = grid(sim, state);
    let start = along(staff.from, staff.aim, staff.travel);
    let limit = (staff.travel + STAFF_SPEED).min(STAFF_RANGE);
    let mut end = along(staff.from, staff.aim, limit);
    let mut hits: Vec<_> = (0..sim.entity_count())
        .filter_map(|i| {
            let entity = sim.entity_at(i)?;
            let kind = visible_target(sim, owner, entity.id())?;
            if structure(kind) || staff.seen.contains(&entity.id()) {
                return None;
            }
            let (x, y) = entity.pos();
            let rel = (x as f64 - staff.from.0, y as f64 - staff.from.1);
            let distance = rel.0 * staff.aim.0 + rel.1 * staff.aim.1;
            let lateral = (rel.0 * staff.aim.1 - rel.1 * staff.aim.0).abs();
            let radius = 5_000. + entity.radius() as f64;
            if lateral > radius {
                return None;
            }
            let half_chord = (radius * radius - lateral * lateral).sqrt();
            let first = (distance - half_chord).max(staff.travel);
            (distance + half_chord >= staff.travel && first <= limit).then_some((
                first,
                entity.id(),
                kind,
            ))
        })
        .collect();
    hits.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut finished = limit >= STAFF_RANGE;
    for (contact, target, kind) in hits {
        staff.seen.push(target);
        let raw = 52 + attack_stat(sim, owner) * 45 / 100;
        if damage(sim, owner, owner, target, raw, AttackTypeV1::Skill) {
            state.intent = Some((target, sim.tick() + 75));
        }
        if !staff.exchange && (kind == EntityKindV1::Champion || kind == EntityKindV1::Jungle) {
            end = along(staff.from, staff.aim, contact);
            finished = true;
            break;
        }
    }
    if (staff.travel as usize / STAFF_SPEED as usize) % 2 == 0 {
        let frame = staff.travel as usize / (2 * STAFF_SPEED as usize);
        view(sim, owner, &format!("sun_staff_step_{frame}"), input(end));
    }
    if staff.exchange {
        if let Some(here) = pos(sim, owner).filter(|_| !blocked(sim, owner, true)) {
            let destination = approach(&grid, here, end, STAFF_SPEED * 1.5);
            move_to(sim, owner, destination);
        }
        if finished {
            animate(sim, owner, "exchange_arrive", 22);
            if let Some(at) = pos(sim, owner) {
                view(sim, owner, "sun_clone_spawn_smoke", input(at));
            }
        }
    } else if finished {
        let legal = if clear(&grid, end) {
            end
        } else {
            approach(&grid, staff.from, start, STAFF_RANGE)
        };
        if !spawn(sim, owner, state, Slot::Staff, legal) {
            start_cooldown(sim, owner, Slot::Staff);
        }
    }
    if !finished {
        staff.travel = limit;
        state.staff = Some(staff);
    }
}

fn update_clone(sim: &mut StableSim<'_>, owner: usize, state: &mut State, slot: Slot) {
    let index = slot as usize;
    let Some(mut body) = state.clones[index].take() else {
        return;
    };
    let now = sim.tick();
    if !is_clone(sim, body.id) && !body.seen && now < body.next_basic {
        // Some hosts insert the carrier at the end of the action tick.
        state.clones[index] = Some(body);
        return;
    }
    if !is_clone(sim, body.id) || pos(sim, body.id).is_none() || now >= body.expires {
        if is_clone(sim, body.id) {
            sim.entity_set_hp(body.id, 0);
        }
        view(sim, owner, "sun_clone_expire", input(body.pos));
        start_cooldown(sim, owner, slot);
        return;
    }
    if !body.seen {
        mark_clone(sim, &mut body, slot.settings().4);
    }
    let Some(owner_pos) = pos(sim, owner) else {
        return;
    };
    body.pos = pos(sim, body.id).unwrap_or(body.pos);
    let target = state
        .intent
        .filter(|(id, until)| {
            now <= *until
                && visible_target(sim, owner, *id).is_some()
                && pos(sim, *id)
                    .is_some_and(|p| crate::distance(owner_pos, p) <= CLONE_LEASH - 10_000.)
                && crate::distance(owner_pos, body.pos) <= CLONE_LEASH
        })
        .and_then(|(id, _)| pos(sim, id).map(|p| (id, p)));
    let grid = grid(sim, state);
    if let Some((target, p)) = target.filter(|(_, p)| crate::distance(body.pos, *p) <= BASIC_RANGE)
    {
        if now >= body.next_basic
            && !blocked(sim, body.id, false)
            && crate::distance(approach(&grid, body.pos, p, BASIC_RANGE), p) < 5_000.
        {
            let target_kind = visible_target(sim, owner, target);
            let camp = (target_kind == Some(EntityKindV1::Jungle)) as usize;
            animate(sim, body.id, "attack", 22);
            view(sim, owner, "sun_basic_staff_swing_arc", input(body.pos));
            if damage(
                sim,
                owner,
                body.id,
                target,
                body.attack + camp * CAMP_BONUS,
                AttackTypeV1::BaseAttack,
            ) && target_kind.is_some_and(|kind| !structure(kind))
            {
                heal_from_clone(sim, owner, body.attack);
            }
            let speed = sim
                .get_entity(owner)
                .map_or(100, |e| e.attack_speed_mult())
                .max(1);
            body.next_basic = now + (6_600 / speed).max(36);
        }
    } else if !blocked(sim, body.id, true) {
        let goal = target.map_or(owner_pos, |(_, p)| p);
        let stop = if target.is_some() {
            16_000.
        } else {
            12_000. + index as f64 * 5_000.
        };
        let max_step = (crate::distance(goal, body.pos) - stop).clamp(0., 1_150.);
        let next = approach(&grid, body.pos, goal, max_step);
        if crate::distance(owner_pos, next) <= CLONE_LEASH
            || crate::distance(owner_pos, next) < crate::distance(owner_pos, body.pos)
        {
            move_to(sim, body.id, next);
            body.pos = next;
        }
    }
    state.clones[index] = Some(body);
}

fn converge(sim: &mut StableSim<'_>, owner: usize, state: &mut State, strike: Converge) {
    let target = strike.target;
    let Some(p) = pos(sim, target) else { return };
    let Some(kind) = visible_target(sim, owner, target) else {
        return;
    };
    if structure(kind) {
        return;
    }
    let grid = grid(sim, state);
    let mut played_sound = false;
    for body in strike.bodies {
        let Some(from) = pos(sim, body).filter(|_| !blocked(sim, body, false)) else {
            continue;
        };
        let Some(aim) = direction(from, p) else {
            continue;
        };
        if crate::distance(from, p) > STRIKE_RANGE + 10_000. {
            continue;
        }
        let attack = attack_stat(sim, owner);
        let clone = state
            .clones
            .iter_mut()
            .flatten()
            .find(|c| c.id == body && is_clone(sim, c.id));
        let raw = if body == owner {
            60 + attack * 50 / 100
        } else if let Some(clone) = clone {
            clone.next_basic = clone.next_basic.max(sim.tick() + 20);
            (60 + attack * 50 / 100) * clone.power / 100
        } else {
            continue;
        };
        let stand_off = along(p, aim, -13_000.);
        let landing = if blocked(sim, body, true) {
            from
        } else {
            approach(&grid, from, stand_off, STRIKE_RANGE)
        };
        if crate::distance(landing, p) > BASIC_RANGE + 2_000.
            || crate::distance(approach(&grid, landing, p, BASIC_RANGE + 2_000.), p) > 5_000.
        {
            continue;
        }
        move_to(sim, body, landing);
        animate(sim, body, "strike", 18);
        if !played_sound {
            sound(sim, owner, "fighter_skill2", input(landing));
            if is_match_view(sim) {
                audio::play_voice(sim, owner, "mlbb_sun_voice_skill03");
            }
            played_sound = true;
        }
        view(sim, owner, "sun_instantaneous_dash_streak", input(landing));
        let mut victims = vec![target];
        victims.extend((0..sim.entity_count()).filter_map(|i| {
            let e = sim.entity_at(i)?;
            if e.id() == target || visible_target(sim, owner, e.id()).is_none_or(structure) {
                return None;
            }
            let (x, y) = e.pos();
            let offset = (x as f64 - p.0, y as f64 - p.1);
            let forward = offset.0 * aim.0 + offset.1 * aim.1;
            let side = (offset.0 * aim.1 - offset.1 * aim.0).abs();
            (forward > 0.
                && forward <= 21_000.
                && side <= 8_000.
                && crate::distance(
                    approach(&grid, p, (x as f64, y as f64), 24_000.),
                    (x as f64, y as f64),
                ) <= 5_000.)
                .then_some(e.id())
        }));
        victims[1..].sort_unstable();
        for victim in victims {
            let primary = victim == target;
            if damage(
                sim,
                owner,
                body,
                victim,
                if primary { raw } else { raw * 55 / 100 },
                AttackTypeV1::Skill,
            ) {
                let mut slow = BuffV1::timed(SLOW, 90);
                slow.move_speed_mult = -12;
                sim.entity_stack_buff(victim, &slow, 3, true);
                if primary && body != owner {
                    let clone_attack = state
                        .clones
                        .iter()
                        .flatten()
                        .find(|c| c.id == body)
                        .map_or(0, |c| c.attack);
                    heal_from_clone(sim, owner, clone_attack);
                }
            }
        }
    }
}

#[derive(Clone, Default)]
struct SunPassive {
    voice: audio::VoiceLifecycle,
}

impl StablePassive for SunPassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, player: usize, owner: usize) {
        if is_match_view(sim) {
            let deaths = sim.get_player(player).map_or(0, |p| p.deaths());
            let line = if deaths == 0 {
                "mlbb_sun_voice_select"
            } else {
                RESPAWN_LINES[(deaths - 1) % RESPAWN_LINES.len()]
            };
            self.voice.on_spawn(sim, owner, Some(line));
        }
        let key = entity_key(sim, owner);
        let old = take(key);
        cleanup(sim, old);
        let spawn_pos = pos(sim, owner);
        save(
            sim,
            key,
            State {
                spawn_pos,
                last_pos: spawn_pos,
                ..State::default()
            },
        );
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        if is_match_view(sim) {
            self.voice.on_dead(sim, player, DEATH_LINES);
        }
        let Some(owner) = sim
            .player_at(player)
            .and_then(|p| p.champion())
            .map(|e| e.id())
        else {
            return;
        };
        let old = take(entity_key_ro(sim, owner));
        cleanup(sim, old);
    }

    fn on_kill(&mut self, sim: &mut StableSim<'_>, _: usize, owner: usize, victim: usize) {
        if is_match_view(sim) && sim.get_entity(victim).is_some_and(|e| e.is_champion()) {
            let line = KILL_LINES[sim.tick() % KILL_LINES.len()];
            audio::play_voice(sim, owner, line);
        }
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, _: usize, owner: usize) {
        let key = entity_key_ro(sim, owner);
        let mut state = take(key);
        let Some(here) = pos(sim, owner).filter(|_| !sim.is_end()) else {
            cleanup(sim, state);
            return;
        };
        let now = sim.tick();
        if state.updated == Some(now) {
            save(sim, key, state);
            return;
        }
        state.updated = Some(now);
        if is_match_view(sim) {
            self.voice.on_update(sim, owner, &[], MOVE_LINES);
        }
        if state
            .spawn_pos
            .is_some_and(|spawn| crate::distance(spawn, here) < 24_000.)
            && state
                .last_pos
                .is_some_and(|last| crate::distance(last, here) > 55_000.)
        {
            // Returning to the well cancels pending moves and both clones.
            if state.staff.is_some() || state.clones[Slot::Staff as usize].is_some() {
                start_cooldown(sim, owner, Slot::Staff);
            }
            if state.clones[Slot::Ultimate as usize].is_some() {
                start_cooldown(sim, owner, Slot::Ultimate);
            }
            let reset = State {
                spawn_pos: state.spawn_pos,
                last_pos: Some(here),
                updated: Some(now),
                ..State::default()
            };
            cleanup(sim, state);
            save(sim, key, reset);
            return;
        }
        state.last_pos = Some(here);
        if let Some(staff) = state.staff.take() {
            staff_tick(sim, owner, &mut state, staff);
        }
        if state.strike.as_ref().is_some_and(|s| s.due <= sim.tick()) {
            let strike = state.strike.take().unwrap();
            converge(sim, owner, &mut state, strike);
        }
        update_clone(sim, owner, &mut state, Slot::Staff);
        update_clone(sim, owner, &mut state, Slot::Ultimate);
        save(sim, key, state);
    }
}

#[derive(Clone, Copy)]
enum Action {
    Basic,
    Staff,
    Strike,
    Ultimate,
}

impl StableEffectType for Action {
    fn apply(&self, sim: &mut StableSim<'_>, seed: u64, owner: usize, requested: InputTargetV1) {
        let Some(from) = pos(sim, owner) else { return };
        let key = entity_key(sim, owner);
        let mut state = take(key);
        let now = sim.tick();
        let target =
            (requested.kind == InputTargetKindV1::Target.code()).then_some(requested.target_id);
        let target_kind = target.and_then(|id| visible_target(sim, owner, id));
        match self {
            Self::Basic => {
                if let Some((target, kind)) = target.zip(target_kind) {
                    state.intent = Some((target, now + 75));
                    sound(sim, owner, "fighter_attack", InputTargetV1::target(target));
                    if is_match_view(sim) {
                        audio::play_voice(
                            sim,
                            owner,
                            BASIC_LINES[seed as usize % BASIC_LINES.len()],
                        );
                    }
                    if state.basic_b {
                        animate(sim, owner, "attack_b", 18);
                    }
                    state.basic_b = !state.basic_b;
                    view(sim, owner, "sun_basic_staff_swing_arc", input(from));
                    let bonus = if kind == EntityKindV1::Jungle {
                        CAMP_BONUS
                    } else {
                        0
                    };
                    damage(
                        sim,
                        owner,
                        owner,
                        target,
                        attack_stat(sim, owner) + bonus,
                        AttackTypeV1::BaseAttack,
                    );
                }
            }
            Self::Staff
                if state.staff.is_none() && state.clones[Slot::Staff as usize].is_none() =>
            {
                if let Some((id, to)) = target
                    .filter(|_| target_kind.is_some())
                    .and_then(|id| pos(sim, id).map(|p| (id, p)))
                {
                    if let Some(aim) = direction(from, to) {
                        state.intent = Some((id, now + 75));
                        sound(sim, owner, "fighter_skill", input(from));
                        if is_match_view(sim) {
                            audio::play_voice(sim, owner, "mlbb_sun_voice_skill01");
                        }
                        let grid = grid(sim, &mut state);
                        let end = along(from, aim, STAFF_RANGE);
                        let exchange = target_kind == Some(EntityKindV1::Champion)
                            && crate::distance(from, to) >= 33_000.
                            && safe_exchange(sim, owner, &grid, from, end)
                            && spawn(sim, owner, &mut state, Slot::Staff, from);
                        if exchange {
                            animate(sim, owner, "exchange_depart", 10);
                            sim.entity_set_invisible(owner, STAFF_TRAVEL_TICKS);
                        }
                        state.staff = Some(Staff {
                            from,
                            aim,
                            travel: 0.,
                            exchange,
                            seen: Vec::new(),
                        });
                    } else {
                        start_cooldown(sim, owner, Slot::Staff);
                    }
                } else {
                    start_cooldown(sim, owner, Slot::Staff);
                }
            }
            Self::Strike => {
                if let Some(target) =
                    target.filter(|_| target_kind.is_some_and(|kind| !structure(kind)))
                {
                    state.intent = Some((target, now + 75));
                    let mut bodies = vec![owner];
                    bodies.extend(
                        state
                            .clones
                            .iter()
                            .flatten()
                            .filter(|c| c.expires > now + STRIKE_WINDUP)
                            .map(|c| c.id),
                    );
                    state.strike = Some(Converge {
                        target,
                        due: now + STRIKE_WINDUP,
                        bodies,
                    });
                }
            }
            Self::Ultimate if state.clones[Slot::Ultimate as usize].is_none() => {
                let grid = grid(sim, &mut state);
                let offset = [(-15_000., 0.), (15_000., 0.), (0., -15_000.), (0., 15_000.)]
                    .into_iter()
                    .map(|d| (from.0 + d.0, from.1 + d.1))
                    .find(|p| crate::distance(approach(&grid, from, *p, 15_000.), *p) < 1.)
                    .unwrap_or(from);
                if target_kind.is_some_and(|kind| !structure(kind)) {
                    state.intent = target.map(|id| (id, now + 75));
                }
                if spawn(sim, owner, &mut state, Slot::Ultimate, offset) {
                    sound(sim, owner, "fighter_ult", input(offset));
                    if is_match_view(sim) {
                        audio::play_priority_voice(
                            sim,
                            owner,
                            ULT_LINES[seed as usize % ULT_LINES.len()],
                        );
                    }
                } else {
                    start_cooldown(sim, owner, Slot::Ultimate);
                }
            }
            _ => {}
        }
        save(sim, key, state);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            match self {
                Self::Basic => stat.attack,
                Self::Staff => 52 + stat.attack * 45 / 100,
                Self::Strike => 60 + stat.attack * 50 / 100,
                Self::Ultimate => stat.attack * 65 / 100,
            },
            0,
        )
    }
}

pub(super) fn register(module: &mut StableMod) {
    module.add_native_passive("mlbb_heroes:sun_passive", SunPassive::default());
    module.add_native_effect("mlbb_heroes:sun_basic", Action::Basic);
    module.add_native_effect("mlbb_heroes:sun_staff", Action::Staff);
    module.add_native_effect("mlbb_heroes:sun_strike", Action::Strike);
    module.add_native_effect("mlbb_heroes:sun_ultimate", Action::Ultimate);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_path_stops_before_a_wall_even_with_full_range() {
        let grid = Grid::from_rows(&[".....", "..#..", "....."]);
        let from = (42_000., 48_000.);
        let behind_wall = (108_000., 48_000.);
        let stop = approach(&grid, from, behind_wall, 90_000.);
        assert!(stop.0 < 60_000.);
        assert!(clear(&grid, stop));

        let open = (55_000., 48_000.);
        assert_eq!(approach(&grid, from, open, STRIKE_RANGE), open);
    }

    #[test]
    fn staff_and_ultimate_reserve_independent_clone_lifetimes() {
        let (staff_life, staff_recovery, staff_power, _, _, staff_action) = Slot::Staff.settings();
        let (ult_life, ult_recovery, ult_power, _, _, ult_action) = Slot::Ultimate.settings();
        assert_eq!(staff_life + staff_recovery + STAFF_TRAVEL_TICKS, 708);
        assert_eq!(ult_life + ult_recovery, 2400);
        assert_ne!(staff_action.code(), ult_action.code());
        assert!(staff_power < ult_power);
    }
}
