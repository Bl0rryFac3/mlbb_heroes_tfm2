//! Yu Zhong's one-actor forms, timed contacts, and Sha Residue.
use crate::{entity_key, entity_key_ro, EntityKey, Grid};
use mod_api_stable::*;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

type Pos = (u64, u64);
const DRAGON_BUFF: &str = "mlbb_yu_dragon_movement";
const SPEED: &str = "mlbb_yu_residue_speed";
const SLOW: &str = "mlbb_yu_grip_slow";
const POSE_PREFIX: &str = "mlbb_yu_pose_";
const MARK_THRESHOLD: usize = 3;
const ERUPTION_PULSES: usize = 5;
include!("yu_zhong_body.rs");
const DRAGONOID_DURATION: usize = 15 * 60;
const RESIDUE_HEAL_PERCENT: usize = 8;
const MONSTER_MISSING_HP_CAP: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    Human,
    Channel(usize),
    Dragon(usize),
    Dragonoid(usize),
}

struct Mark {
    target: EntityKey,
    count: usize,
    pulses_left: usize,
    expires: usize,
    next: Option<usize>,
    render_at: usize,
}

#[derive(Clone, Copy)]
struct Dive {
    from: Pos,
    dest: Pos,
    end: usize,
    scale: f64,
}

struct State {
    form: Form,
    marks: Vec<Mark>,
    hit_by_dragon: Vec<EntityKey>,
    tail: Option<(usize, f64)>,
    grip_ready: usize,
    dive: Option<Dive>,
    dive_impact: Option<(usize, Pos, f64)>,
    grid: Option<Arc<Grid>>,
    landing: Pos,
    last_pos: Pos,
    last_tick: Option<usize>,
    cast_at: [Option<usize>; 4],
    motion: Option<(&'static str, usize)>,
    last_body: Option<&'static str>,
    body_next: usize,
    left: bool,
    moving: bool,
}

impl State {
    fn new(pos: Pos) -> Self {
        Self {
            form: Form::Human,
            marks: Vec::new(),
            hit_by_dragon: Vec::new(),
            tail: None,
            grip_ready: 0,
            dive: None,
            dive_impact: None,
            grid: None,
            landing: pos,
            last_pos: pos,
            last_tick: None,
            cast_at: [None; 4],
            motion: None,
            last_body: None,
            body_next: 0,
            left: false,
            moving: false,
        }
    }

    fn scale(&self) -> f64 {
        if matches!(self.form, Form::Dragonoid(_)) {
            1.2
        } else {
            1.
        }
    }

    fn play(&mut self, now: usize, name: &'static str, ticks: usize) {
        self.motion = Some((name, now + ticks));
    }

    fn locked(&self) -> bool {
        matches!(self.form, Form::Channel(_) | Form::Dragon(_))
    }

    fn playing_skill(&self, now: usize) -> bool {
        self.motion.is_some_and(|(name, end)| {
            now < end && !matches!(name, "basic" | "empowered_basic" | "dragonoid_attack")
        })
    }
}

static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn take(k: EntityKey) -> Option<State> {
    STATES.lock().unwrap_or_else(|e| e.into_inner()).remove(&k)
}
fn put(k: EntityKey, s: State) {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(k, s);
}
pub(crate) fn clear_state() {
    STATES.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

fn position(sim: &StableSim<'_>, id: usize) -> Option<Pos> {
    sim.get_entity(id).map(|e| e.pos())
}
fn delta(a: Pos, b: Pos) -> (f64, f64) {
    (b.0 as f64 - a.0 as f64, b.1 as f64 - a.1 as f64)
}
fn distance(a: Pos, b: Pos) -> f64 {
    let (x, y) = delta(a, b);
    x.hypot(y)
}
fn to(a: Pos, dir: (f64, f64), n: f64) -> Pos {
    (
        (a.0 as f64 + dir.0 * n).max(0.) as u64,
        (a.1 as f64 + dir.1 * n).max(0.) as u64,
    )
}
fn direction(a: Pos, b: Pos) -> (f64, f64) {
    let (x, y) = delta(a, b);
    let l = x.hypot(y).max(1.);
    (x / l, y / l)
}
fn segment(p: Pos, a: Pos, b: Pos) -> f64 {
    let (x, y) = delta(a, b);
    let (px, py) = delta(a, p);
    let length = x * x + y * y;
    if length < 1. {
        return px.hypot(py);
    }
    let t = ((px * x + py * y) / length).clamp(0., 1.);
    (px - x * t).hypot(py - y * t)
}

fn enemy(sim: &StableSim<'_>, id: usize, team: usize) -> bool {
    sim.get_entity(id).is_some_and(|e| {
        e.team() != team
            && e.is_alive()
            && e.is_targetable()
            && !matches!(e.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus))
    })
}
fn targets(sim: &StableSim<'_>, team: usize, champions: bool) -> Vec<(usize, Pos)> {
    (0..if champions {
        sim.champion_count()
    } else {
        sim.entity_count()
    })
        .filter_map(|i| {
            if champions {
                sim.get_entity(sim.champion_id_at(i))
            } else {
                sim.entity_at(i)
            }
        })
        .filter(|e| enemy(sim, e.id(), team))
        .map(|e| (e.id(), e.pos()))
        .collect()
}
fn damage(sim: &mut StableSim<'_>, id: usize, target: usize, base: usize, scale: usize) {
    let attack = sim.get_entity(id).map_or(0, |e| e.stat().attack);
    sim.deal_damage(
        id,
        target,
        base + attack * scale / 100,
        0,
        AttackTypeV1::Skill,
    );
}

fn cue(sim: &mut StableSim<'_>, id: usize, name: &str) {
    if sim
        .sim_origin()
        .and_then(|origin| SimOriginKindV1::from_code(origin.kind))
        .is_some_and(|kind| {
            matches!(
                kind,
                SimOriginKindV1::ClientMatchView
                    | SimOriginKindV1::ClientReplay
                    | SimOriginKindV1::ClientSpectate
            )
        })
    {
        let _ = sim.play_sfx(name, id, &InputTargetV1::target(id));
    }
}
fn visible(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
}
fn fx(sim: &mut StableSim<'_>, id: usize, p: Pos, name: &str) {
    if visible(sim) {
        sim.play_view_effect(
            &format!("mlbb_yu_fx_{name}"),
            id,
            &InputTargetV1::pos(p.0, p.1),
            0,
            0,
            0,
        );
    }
}
fn mark_fx(sim: &mut StableSim<'_>, target: usize, count: usize) {
    if let Some(p) = position(sim, target) {
        fx(sim, target, p, &format!("mark_{count}"));
    }
}
fn pose(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    let now = sim.tick();
    if now < s.body_next || !visible(sim) {
        return;
    }
    let Some(p) = sim
        .get_entity(id)
        .filter(|e| e.is_alive() && e.hp().0 > 0)
        .map(|e| e.pos())
    else {
        return;
    };
    let idle = match (s.form, s.moving) {
        (Form::Dragon(_), _) => "black_dragon_flight",
        (Form::Dragonoid(_), true) => "dragonoid_jog",
        (Form::Dragonoid(_), false) => "dragonoid_idle",
        (_, true) => "human_jog",
        (_, false) => "human_idle",
    };
    let motion = s.motion.filter(|(_, end)| now < *end);
    let name = motion.map_or(idle, |(name, _)| name);
    let (frames, ticks) = body_clip(name);
    let age = motion.map_or(now % ticks, |(_, end)| {
        now.saturating_sub(end.saturating_sub(ticks))
    });
    let frame = (age * frames / ticks).min(frames - 1);
    let side = if s.left { "_left" } else { "" };
    let binding = format!("mlbb_yu_fx_body_{name}{side}_{frame}");
    // Match the world-effect lifetime to the sampling interval. No animation
    // is attached to the actor, so death cannot retain an actor-bound body.
    sim.play_view_effect(
        &binding,
        id,
        &InputTargetV1::pos(p.0, p.1),
        0,
        0,
        BODY_SAMPLE_TICKS as u64,
    );
    s.last_body = Some(name);
    s.body_next = now + BODY_SAMPLE_TICKS;
}
fn size_suffix(scale: f64) -> &'static str {
    if scale > 1. {
        "_large"
    } else {
        ""
    }
}

fn action(s: &State, base: &'static str, upgraded: &'static str) -> &'static str {
    if matches!(s.form, Form::Dragonoid(_)) {
        upgraded
    } else {
        base
    }
}
fn monster(kind: Option<EntityKindV1>) -> bool {
    matches!(
        kind,
        Some(EntityKindV1::Jungle | EntityKindV1::Epic | EntityKindV1::Serpen)
    )
}

fn add_residue(sim: &mut StableSim<'_>, id: usize, target: usize, count: usize, s: &mut State) {
    if !sim.get_entity(target).is_some_and(|e| {
        e.is_alive() && e.is_targetable() && (e.is_champion() || monster(e.kind()))
    }) {
        return;
    }
    let now = sim.tick();
    let key = entity_key(sim, target);
    let i = s.marks.iter().position(|m| m.target == key);
    let mark = if let Some(i) = i {
        &mut s.marks[i]
    } else {
        if s.marks.len() >= 32 {
            return;
        }
        s.marks.push(Mark {
            target: key,
            count: 0,
            pulses_left: 0,
            expires: 0,
            next: None,
            render_at: 0,
        });
        s.marks.last_mut().unwrap()
    };
    if mark.next.is_some() {
        return;
    }
    if mark.expires <= now {
        mark.count = 0;
    }
    mark.count = (mark.count + count).min(MARK_THRESHOLD);
    mark.expires = now + 300;
    mark_fx(sim, target, mark.count);
    mark.render_at = now + 20;
    if mark.count == MARK_THRESHOLD {
        mark.pulses_left = ERUPTION_PULSES;
        mark.next = Some(now + 15);
        sim.entity_remove_buff(id, SPEED);
        let mut buff = BuffV1::timed(SPEED, 90);
        buff.move_speed_mult = 15;
        sim.add_buff(id, &buff);
        if visible(sim) {
            crate::wlog(format!(
                "[YU ZHONG] eruption tick={now} caster={id} target={target}"
            ));
        }
    }
}
fn tick_residue(sim: &mut StableSim<'_>, id: usize, team: usize, now: usize, s: &mut State) {
    s.marks.retain_mut(|m| {
        let target = m.target.entity_id;
        if entity_key_ro(sim, target) != m.target || !enemy(sim, target, team) {
            return false;
        }
        if let Some(next) = m.next {
            if now >= next {
                let victim = sim.get_entity(target).unwrap();
                let (hp, max) = victim.hp();
                let missing_bonus = max.saturating_sub(hp).saturating_mul(2) / 100;
                let missing_bonus = if monster(victim.kind()) {
                    missing_bonus.min(MONSTER_MISSING_HP_CAP)
                } else {
                    missing_bonus
                };
                damage(sim, id, target, 32 + missing_bonus, 18);
                let missing = sim
                    .get_entity(id)
                    .filter(|e| e.is_alive() && e.hp().0 > 0)
                    .map_or(0, |e| {
                        let (hp, max) = e.hp();
                        max.saturating_sub(hp)
                    });
                if missing > 0 {
                    sim.heal(id, id, missing * RESIDUE_HEAL_PERCENT / 100);
                }
                if let Some(target_pos) = position(sim, target) {
                    fx(sim, id, target_pos, "eruption_flash");
                }
                if let Some(caster_pos) = position(sim, id) {
                    fx(sim, id, caster_pos, "returning_mote");
                }
                m.pulses_left -= 1;
                m.next = Some(now + 15);
                return m.pulses_left > 0 && enemy(sim, target, team);
            }
            return true;
        }
        if now >= m.expires {
            return false;
        }
        if now >= m.render_at {
            mark_fx(sim, target, m.count);
            m.render_at = now + 20;
        }
        true
    });
}

fn grip(sim: &mut StableSim<'_>, id: usize, team: usize, p: Pos, aim: Pos, s: &mut State) {
    let now = sim.tick();
    let scale = s.scale();
    let dir = direction(p, aim);
    s.left = dir.0 < 0.;
    s.play(now, action(s, "soul_grip", "dragonoid_soul_grip"), 22);
    let angle = ((dir.1.atan2(dir.0).rem_euclid(std::f64::consts::TAU) * 16.
        / std::f64::consts::TAU)
        .round() as usize)
        % 16;
    fx(
        sim,
        id,
        to(p, dir, 26_000. * scale),
        &format!("grip_wave{}_{angle}", size_suffix(scale)),
    );
    let mut closest = None;
    let mut any = false;
    for (target, pos) in targets(sim, team, false) {
        let (x, y) = delta(p, pos);
        let along = x * dir.0 + y * dir.1;
        if !(0. ..=55_000. * scale).contains(&along)
            || (x * dir.1 - y * dir.0).abs() > 12_000. * scale
        {
            continue;
        }
        any = true;
        damage(sim, id, target, 95, 68);
        add_residue(sim, id, target, 1, s);
        sim.entity_remove_buff(target, SLOW);
        let mut slow = BuffV1::timed(SLOW, 60);
        slow.move_speed_mult = -30;
        sim.add_buff(target, &slow);
        if (23_000. ..=53_000. * scale).contains(&along)
            && sim
                .get_entity(target)
                .is_some_and(|e| e.is_champion() && e.is_alive())
            && closest.is_none_or(|(best, _)| along < best)
        {
            closest = Some((along, pos));
        }
    }
    if any {
        s.grip_ready = now + 180;
        fx(sim, id, p, "grip_ready_glow");
    }
    if let Some((gap, target)) = closest {
        if crate::hanzo::blocked(sim, id, true) {
            return;
        }
        let dest = to(
            p,
            direction(p, target),
            (gap - 16_000.).min(32_000. * scale),
        );
        let g = s.grid.get_or_insert_with(|| crate::hanzo::grid(sim));
        if crate::hanzo::standable(g, dest)
            && distance(
                crate::hanzo::legal_step(g, p, direction(p, dest), distance(p, dest), false),
                dest,
            ) < 3.
        {
            s.dive = Some(Dive {
                from: p,
                dest,
                end: now + if scale > 1. { 24 } else { 17 },
                scale,
            });
        }
    }
}
fn tick_dive(sim: &mut StableSim<'_>, id: usize, team: usize, p: Pos, now: usize, s: &mut State) {
    if let Some(dive) = s.dive.take() {
        if !crate::hanzo::blocked(sim, id, true) {
            if now + 12 >= dive.end
                && s.motion.is_none_or(|(name, _)| {
                    !matches!(name, "furious_dive" | "dragonoid_furious_dive")
                })
            {
                s.play(now, action(s, "furious_dive", "dragonoid_furious_dive"), 22);
            }
            if now + 12 < dive.end {
                s.dive = Some(dive);
            } else {
                let g = s.grid.as_ref().unwrap();
                let step = crate::hanzo::legal_step(
                    g,
                    p,
                    direction(p, dive.dest),
                    distance(p, dive.dest) / (dive.end.saturating_sub(now).max(1) as f64),
                    false,
                );
                let landed = if step != p && sim.entity_set_pos(id, step.0, step.1) {
                    step
                } else {
                    p
                };
                if now >= dive.end || landed == dive.dest || landed == p {
                    if distance(landed, dive.from) > 2_000. {
                        fx(
                            sim,
                            id,
                            landed,
                            &format!("dive_warning{}", size_suffix(dive.scale)),
                        );
                        s.dive_impact = Some((now + 24, landed, dive.scale));
                    }
                } else {
                    s.dive = Some(dive);
                }
            }
        }
    }
    if let Some((at, center, scale)) = s.dive_impact.filter(|(at, _, _)| now >= *at) {
        debug_assert!(now >= at);
        s.dive_impact = None;
        cue(sim, id, "mlbb_yu_dive");
        fx(
            sim,
            id,
            center,
            &format!("dive_spikes{}", size_suffix(scale)),
        );
        for (target, pos) in targets(sim, team, false) {
            if distance(center, pos) > 23_000. * scale {
                continue;
            }
            damage(sim, id, target, 64, 45);
            add_residue(sim, id, target, 1, s);
            if sim.get_entity(target).is_some_and(|e| {
                e.is_champion()
                    && e.is_alive()
                    && !(0..e.buff_count())
                        .filter_map(|i| e.buff_at(i))
                        .any(|b| b.cc_immune)
            }) {
                sim.apply_cc(target, &CcV1::of_kind(CcKindV1::Airborne, 30));
            }
        }
    }
}

fn tick_forms(sim: &mut StableSim<'_>, id: usize, team: usize, p: Pos, now: usize, s: &mut State) {
    match s.form {
        Form::Channel(_) if crate::hanzo::blocked(sim, id, false) => {
            s.form = Form::Human;
            s.motion = None;
        }
        Form::Channel(end) if now >= end => {
            s.form = Form::Dragon(now + 210);
            s.motion = None;
            let mut buff = BuffV1::timed(DRAGON_BUFF, 211);
            buff.ignore_wall = true;
            buff.cc_immune = true;
            buff.move_speed_mult = 35;
            sim.add_buff(id, &buff);
            let _ = s.grid.get_or_insert_with(|| crate::hanzo::grid(sim));
            s.landing = p;
            s.hit_by_dragon.clear();
        }
        Form::Dragon(end) => {
            let g = s.grid.as_ref().unwrap();
            if crate::hanzo::standable(g, p) && g.on_map(p.0 as f64, p.1 as f64) {
                s.landing = p;
            }
            if now >= end || distance(p, s.last_pos) > 80_000. {
                let ground = crate::hanzo::landing(g, p, s.landing);
                if ground != p {
                    sim.entity_set_pos(id, ground.0, ground.1);
                }
                sim.entity_remove_buff(id, DRAGON_BUFF);
                s.form = Form::Dragonoid(now + DRAGONOID_DURATION);
                s.play(now, "dragon_exit", 24);
                fx(sim, id, ground, "dragon_exit_fx");
            } else {
                for (target, pos) in targets(sim, team, true) {
                    if segment(pos, s.last_pos, p) > 20_000. {
                        continue;
                    }
                    let key = entity_key(sim, target);
                    if s.hit_by_dragon.contains(&key) {
                        continue;
                    }
                    s.hit_by_dragon.push(key);
                    damage(sim, id, target, 125, 85);
                    add_residue(sim, id, target, 1, s);
                    sim.entity_knockback(id, target, 1350, 12);
                    fx(sim, id, pos, "dragon_contact_fx");
                    s.play(now, "black_dragon_contact", 14);
                }
            }
        }
        Form::Dragonoid(end) if now >= end => {
            s.form = Form::Human;
            s.play(now, "dragonoid_to_human", 18);
        }
        _ => {}
    }
}
fn actor_active(sim: &StableSim<'_>, player: usize, id: usize) -> bool {
    sim.get_entity(id)
        .is_some_and(|e| e.is_alive() && e.hp().0 > 0)
        && !sim.get_player(player).is_some_and(|p| p.respawn_time() > 0)
}

fn update(sim: &mut StableSim<'_>, player: usize, key: EntityKey, mut s: State) {
    let id = key.entity_id;
    let now = sim.tick();
    let Some((p, team)) = sim
        .get_entity(id)
        .filter(|e| e.is_alive())
        .map(|e| (e.pos(), e.team()))
    else {
        clear_actor(sim, id);
        return;
    };
    if s.last_tick == Some(now) {
        put(key, s);
        return;
    }
    if s.last_tick.is_some_and(|tick| tick > now) {
        clear_actor(sim, id);
        s = State::new(p);
    }
    s.last_tick = Some(now);
    let prev = s.last_pos;
    s.moving = distance(prev, p) > 100.;
    if p.0.abs_diff(prev.0) > 100
        && (s.moving
            && (s.motion.is_none_or(|(_, t)| t <= now) || matches!(s.form, Form::Dragon(_))))
    {
        s.left = p.0 < prev.0;
    }
    tick_forms(sim, id, team, p, now, &mut s);
    let p = position(sim, id).unwrap_or(p);
    if let Some((_, scale)) = s.tail.filter(|(at, _)| now >= *at) {
        s.tail = None;
        fx(sim, id, p, &format!("tail_crescents{}", size_suffix(scale)));
        for (target, q) in targets(sim, team, false) {
            let range = distance(p, q);
            if range > 35_000. * scale {
                continue;
            }
            let (flat, ratio, marks, flash) = if range >= 22_000. * scale {
                (138, 95, 2, "tail_outer_hit")
            } else {
                (80, 65, 1, "tail_inner_hit")
            };
            damage(sim, id, target, flat, ratio);
            add_residue(sim, id, target, marks, &mut s);
            fx(sim, id, q, flash);
        }
    }
    tick_dive(sim, id, team, p, now, &mut s);
    tick_residue(sim, id, team, now, &mut s);
    if !actor_active(sim, player, id) {
        clear_actor(sim, id);
        return;
    }
    s.last_pos = position(sim, id).unwrap_or(p);
    pose(sim, id, &mut s);
    put(key, s);
}

fn clear_actor(sim: &mut StableSim<'_>, id: usize) {
    if sim.get_entity(id).is_some() {
        sim.entity_remove_buff(id, DRAGON_BUFF);
        sim.entity_remove_buff(id, SPEED);
    }
}

fn attached_poses(sim: &StableSim<'_>, id: usize) -> usize {
    sim.get_entity(id).map_or(0, |e| {
        (0..e.buff_count())
            .filter_map(|i| e.buff_at(i))
            .filter(|b| b.name().starts_with(POSE_PREFIX))
            .count()
    })
}

#[derive(Copy, Clone)]
enum Cast {
    Basic,
    Tail,
    Grip,
    Dragon,
}
impl StableEffectType for Cast {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, input: InputTargetV1) {
        let key = entity_key_ro(sim, id);
        let Some(mut s) = take(key) else {
            return;
        };
        let Some((p, team)) = sim
            .get_entity(id)
            .filter(|e| e.is_alive())
            .map(|e| (e.pos(), e.team()))
        else {
            clear_actor(sim, id);
            return;
        };
        let now = sim.tick();
        let index = *self as usize;
        if s.cast_at[index] == Some(now) || s.locked() {
            put(key, s);
            return;
        }
        s.cast_at[index] = Some(now);
        match self {
            Self::Basic => {
                let target = input.target_id;
                if let Some(q) = position(sim, target) {
                    s.left = q.0 < p.0;
                    let show_attack =
                        !matches!(s.form, Form::Dragonoid(_)) || !s.playing_skill(now);
                    if enemy(sim, target, team) {
                        let primed = s.grip_ready > now;
                        if primed {
                            s.grip_ready = 0;
                            damage(sim, id, target, 55, 40);
                            fx(sim, id, q, "grip_empowered_flash");
                        }
                        let normal = if primed { "empowered_basic" } else { "basic" };
                        if show_attack {
                            s.play(now, action(&s, normal, "dragonoid_attack"), 24);
                        }
                        add_residue(sim, id, target, 1, &mut s);
                    } else if show_attack {
                        s.play(now, action(&s, "basic", "dragonoid_attack"), 24);
                    }
                }
            }
            Self::Tail => {
                s.tail = Some((now + 24, s.scale()));
                cue(sim, id, "mlbb_yu_tail");
                s.play(now, action(&s, "dragon_tail", "dragonoid_dragon_tail"), 38);
                fx(
                    sim,
                    id,
                    p,
                    &format!("tail_warning{}", size_suffix(s.scale())),
                );
            }
            Self::Grip => {
                let aim = position(sim, input.target_id).unwrap_or(p);
                if aim != p {
                    grip(sim, id, team, p, aim, &mut s);
                    cue(sim, id, "mlbb_yu_grip");
                }
            }
            Self::Dragon => {
                if let Some(q) = position(sim, input.target_id) {
                    s.left = q.0 < p.0;
                }
                s.dive = None;
                s.tail = None;
                s.form = Form::Channel(now + 36);
                cue(sim, id, "mlbb_yu_dragon");
                s.play(now, "black_dragon_entry", 36);
                fx(sim, id, p, "dragon_channel");
            }
        }
        if !sim
            .get_entity(id)
            .is_some_and(|e| e.is_alive() && e.hp().0 > 0)
        {
            clear_actor(sim, id);
            return;
        }
        if visible(sim) && matches!(s.form, Form::Dragonoid(_)) {
            let skill = match self {
                Self::Tail => Some("tail"),
                Self::Grip => Some("grip"),
                _ => None,
            };
            if let Some(skill) = skill {
                crate::wlog(format!(
                    "[YU ZHONG] dragonoid cast={skill} tick={now} caster={id}"
                ));
            }
        }
        pose(sim, id, &mut s);
        put(key, s);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            match self {
                Self::Basic => stat.attack,
                Self::Tail => 138 + stat.attack * 95 / 100,
                Self::Grip => 95 + stat.attack * 68 / 100,
                Self::Dragon => 125 + stat.attack * 85 / 100,
            },
            0,
        )
    }
}

#[derive(Default)]
struct Passive {
    owner: Option<EntityKey>,
}
impl StablePassive for Passive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(Self::default())
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, player: usize, id: usize) {
        if let Some(old) = self.owner.take() {
            take(old);
            if old.entity_id != id && entity_key_ro(sim, old.entity_id) == old {
                clear_actor(sim, old.entity_id);
            }
        }
        clear_actor(sim, id);
        if !actor_active(sim, player, id) {
            return;
        }
        let key = entity_key(sim, id);
        let mut s = State::new(position(sim, id).unwrap());
        pose(sim, id, &mut s);
        put(key, s);
        self.owner = Some(key);
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, id: usize) {
        if !actor_active(sim, player, id) {
            // The explicit callback can arrive after update or without a live
            // champion handle. Drop state once and never reattach a dead pose.
            if self.owner.is_some() {
                self.on_dead(sim, player);
            }
            return;
        }
        if self.owner != Some(entity_key_ro(sim, id)) {
            self.on_spawn(sim, player, id);
        }
        if let Some(key) = self.owner {
            if let Some(s) = take(key) {
                update(sim, player, key, s);
            } else {
                self.on_spawn(sim, player, id);
            }
        }
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        let old = self.owner.take();
        let state = old.and_then(take);
        let owned_id = old
            .filter(|key| entity_key_ro(sim, key.entity_id) == *key)
            .map(|key| key.entity_id);
        let actor = sim.get_player(player).and_then(|p| p.champion());
        let respawn = sim
            .get_player(player)
            .map(|p| p.respawn_time())
            .unwrap_or(0);
        let dead_id = actor
            .filter(|e| respawn > 0 || !e.is_alive() || e.hp().0 == 0)
            .map(|e| e.id());
        if let Some(id) = owned_id.or(dead_id) {
            let before = attached_poses(sim, id);
            clear_actor(sim, id);
            if visible(sim) && (state.is_some() || before > 0) {
                let form = state.as_ref().map(|s| s.form);
                let last_body = state.and_then(|s| s.last_body);
                let after = attached_poses(sim, id);
                let position = position(sim, id);
                crate::wlog(format!(
                    "[YU ZHONG] death tick={} caster={id} form={form:?} last_body={last_body:?} position={position:?} respawn_ticks={respawn} attached_poses_before={before} attached_poses_after={after}",
                    sim.tick()
                ));
            }
        }
    }
}

struct Ai;
impl StablePlayerAi for Ai {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(Self)
    }
    fn id(&self) -> String {
        "mlbb_yu_zhong_ai".into()
    }
    fn priority(&self) -> i32 {
        176
    }
    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name == "mlbb_yu_zhong"
    }
    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        let base = base?;
        if matches!(
            InputKindV1::from_code(base.kind),
            Some(InputKindV1::Return | InputKindV1::Ult)
        ) {
            return None;
        }
        let player = ctx.player_id();
        let proposals = {
            let sim = ctx.sim()?;
            let e = sim.get_player(player)?.champion()?;
            let states = STATES.lock().unwrap_or_else(|e| e.into_inner());
            let s = states.get(&entity_key_ro(&sim, e.id()))?;
            if !actor_active(&sim, player, e.id()) {
                return None;
            }
            let p = e.pos();
            let dragonoid = matches!(s.form, Form::Dragonoid(_));
            if s.locked()
                || crate::hanzo::blocked(&sim, e.id(), false)
                || (dragonoid && s.playing_skill(sim.tick()))
            {
                return None;
            }
            let move_order = InputKindV1::from_code(base.kind) == Some(InputKindV1::Move);
            let target = if move_order && dragonoid {
                // Only borrow an approach order. A move away from the
                // enemy or to a different part of the map passes through.
                let goal = (base.x, base.y);
                targets(&sim, e.team(), true)
                    .into_iter()
                    .filter(|(id, q)| {
                        let gap = distance(p, *q);
                        gap <= 55_000. * s.scale()
                            && distance(goal, *q) < gap
                            && sim.is_visible(e.team(), *id)
                    })
                    .min_by(|(_, a), (_, b)| distance(p, *a).total_cmp(&distance(p, *b)))
                    .and_then(|(id, _)| sim.get_entity(id))?
            } else {
                if move_order || base.target.kind != InputTargetKindV1::Target.code() {
                    return None;
                }
                sim.get_entity(base.target.target_id)?
            };
            if !enemy(&sim, target.id(), e.team()) || !sim.is_visible(e.team(), target.id()) {
                return None;
            }
            let close = distance(p, target.pos());
            let scale = s.scale();
            let ready = |slot| {
                e.cooldown(slot)
                    .is_some_and(|c| c.unlocked && c.charges > 0 && c.remaining == 0)
            };
            let aim = InputTargetV1::target(target.id());
            let mut out = Vec::new();
            if ready(SkillSlotV1::Skill) && close <= 35_000. * scale {
                out.push(InputV1::action(
                    InputKindV1::Skill,
                    InputTargetV1::default(),
                ));
            }
            if ready(SkillSlotV1::Skill2) && close <= 55_000. * scale {
                out.push(InputV1::action(InputKindV1::Skill2, aim));
            }
            out
        };
        proposals.into_iter().find(|i| ctx.is_valid_input(i))
    }
}

pub(crate) fn register(module: &mut StableMod) {
    for (name, cast) in [
        ("basic", Cast::Basic),
        ("tail", Cast::Tail),
        ("grip", Cast::Grip),
        ("dragon", Cast::Dragon),
    ] {
        module.add_native_effect(&format!("mlbb_heroes:yu_{name}"), cast);
    }
    module.add_native_passive("mlbb_heroes:yu_zhong", Passive::default());
    module.add_player_input_ai(Ai);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hanzo::tests::Host;

    fn setup(h: &mut Host) -> Passive {
        let mut p = Passive::default();
        h.sim(|sim| p.on_spawn(sim, 1, 1));
        let key = p.owner.unwrap();
        let mut s = take(key).unwrap();
        s.grid = Some(Arc::new(Grid {
            cells: vec![vec![0; 80]; 80],
            bounds: None,
        }));
        put(key, s);
        p
    }
    fn cast(h: &mut Host, spell: Cast) {
        h.sim(|sim| spell.apply(sim, 0, 1, InputTargetV1::target(2)));
    }
    fn tick(h: &mut Host, p: &mut Passive, n: usize) {
        h.now = n;
        h.sim(|sim| p.on_update(sim, 0, 1, 1));
    }
    fn hp(h: &mut Host, id: usize) -> usize {
        h.sim(|sim| sim.get_entity(id).unwrap().hp().0)
    }
    #[test]
    fn tail_hits_the_outer_band_once_and_gives_two_marks() {
        let mut h = Host::new();
        let mut p = setup(&mut h);
        h.set_unit_position(2, (222_000, 200_000));
        cast(&mut h, Cast::Tail);
        tick(&mut h, &mut p, 23);
        assert_eq!(hp(&mut h, 2), 700);
        tick(&mut h, &mut p, 24);
        tick(&mut h, &mut p, 24);
        assert_eq!(hp(&mut h, 2), 458);
        assert_eq!(take(p.owner.unwrap()).unwrap().marks[0].count, 2);
    }
    #[test]
    fn three_marks_start_five_pulses_and_cannot_recurse() {
        let mut h = Host::new();
        let mut p = setup(&mut h);
        h.sim(|sim| {
            sim.entity_set_hp(1, 400);
        });
        for now in 0..MARK_THRESHOLD {
            h.now = now;
            cast(&mut h, Cast::Basic);
            cast(&mut h, Cast::Basic);
        }
        tick(&mut h, &mut p, 16);
        assert_eq!(hp(&mut h, 2), 700);
        for now in [17, 32, 47, 62, 77] {
            tick(&mut h, &mut p, now);
            if now < 77 {
                cast(&mut h, Cast::Basic);
            }
            tick(&mut h, &mut p, now);
        }
        let final_hp = hp(&mut h, 2);
        assert_eq!(final_hp, 435);
        assert_eq!(hp(&mut h, 1), 501);
        tick(&mut h, &mut p, 99);
        assert_eq!(hp(&mut h, 2), final_hp);
        assert!(take(p.owner.unwrap()).unwrap().marks.is_empty());
    }
    #[test]
    fn grip_primes_one_basic_and_dive_resolves_after_its_warning() {
        let mut h = Host::new();
        let mut p = setup(&mut h);
        h.set_unit_position(2, (240_000, 200_000));
        cast(&mut h, Cast::Grip);
        assert_eq!(hp(&mut h, 2), 531);
        for t in 1..=18 {
            tick(&mut h, &mut p, t);
        }
        assert!(h.cc.is_empty());
        cast(&mut h, Cast::Basic);
        assert_eq!(hp(&mut h, 2), 432);
        h.now = 19;
        cast(&mut h, Cast::Basic);
        assert_eq!(hp(&mut h, 2), 432);
        tick(&mut h, &mut p, 34);
        assert_eq!(hp(&mut h, 2), 376);
        tick(&mut h, &mut p, 42);
        assert_eq!(hp(&mut h, 2), 263);
        assert_eq!(h.cc, vec![(2, CcV1::of_kind(CcKindV1::Airborne, 30))]);
    }
    #[test]
    fn flight_contact_is_once_and_form_expiry_clears_movement_buff() {
        let mut h = Host::new();
        let mut p = setup(&mut h);
        cast(&mut h, Cast::Dragon);
        tick(&mut h, &mut p, 35);
        assert_eq!(hp(&mut h, 2), 700);
        tick(&mut h, &mut p, 36);
        tick(&mut h, &mut p, 37);
        tick(&mut h, &mut p, 38);
        assert_eq!(hp(&mut h, 2), 482);
        tick(&mut h, &mut p, 246);
        h.sim(|sim| {
            let e = sim.get_entity(1).unwrap();
            assert!(!(0..e.buff_count())
                .filter_map(|i| e.buff_at(i))
                .any(|b| b.name() == DRAGON_BUFF));
        });
        let s = take(p.owner.unwrap()).unwrap();
        assert_eq!(s.form, Form::Dragonoid(1146));
        put(p.owner.unwrap(), s);
        tick(&mut h, &mut p, 1145);
        assert_eq!(
            h.sim(|sim| {
                let key = entity_key_ro(sim, 1);
                STATES.lock().unwrap().get(&key).unwrap().form
            }),
            Form::Dragonoid(1146)
        );
        tick(&mut h, &mut p, 1146);
        assert_eq!(take(p.owner.unwrap()).unwrap().form, Form::Human);
    }
    #[test]
    fn detached_bodies_stop_at_death_and_respawn_starts_fresh() {
        for form in [
            Form::Human,
            Form::Channel(100),
            Form::Dragon(100),
            Form::Dragonoid(100),
        ] {
            let mut h = Host::new();
            let mut p = setup(&mut h);
            let key = p.owner.unwrap();
            let mut s = take(key).unwrap();
            s.form = form;
            s.tail = Some((16, 1.2));
            s.grip_ready = 100;
            put(key, s);
            tick(&mut h, &mut p, BODY_SAMPLE_TICKS);
            let target_hp_at_death = hp(&mut h, 2);
            let prefix = "mlbb_yu_fx_body_";
            assert_eq!(
                h.effects
                    .iter()
                    .filter(|(name, _, _)| name.starts_with(prefix))
                    .count(),
                2
            );
            assert!(h
                .effects
                .iter()
                .filter(|(name, _, _)| name.starts_with(prefix))
                .all(|(_, _, lifetime)| *lifetime == BODY_SAMPLE_TICKS as u64));
            h.sim(|sim| {
                sim.add_buff(1, &BuffV1::named(DRAGON_BUFF));
                sim.add_buff(1, &BuffV1::named(SPEED));
                sim.deal_damage(2, 1, 999, 0, AttackTypeV1::Skill);
                // Test both orders. The update handler is also the fallback
                // when the host misses the explicit callback.
                if !matches!(form, Form::Dragon(_)) {
                    p.on_dead(sim, 1);
                }
            });
            tick(&mut h, &mut p, 16);
            h.sim(|sim| {
                p.on_dead(sim, 1);
                let e = sim.get_entity(1).unwrap();
                assert!((0..e.buff_count()).filter_map(|i| e.buff_at(i)).all(|b| {
                    !b.name().starts_with(POSE_PREFIX) && !matches!(b.name(), DRAGON_BUFF | SPEED)
                }));
            });
            assert!(p.owner.is_none() && take(key).is_none());
            assert_eq!(hp(&mut h, 2), target_hp_at_death);
            assert_eq!(
                h.effects
                    .iter()
                    .filter(|(name, _, _)| name.starts_with(prefix))
                    .count(),
                2
            );

            // A replacement at spawn uses the same ID in this mock. Its old
            // simulation state must not carry over.
            h.respawn_unit(1, (90_000, 900_000));
            h.respawn_ticks = 1;
            tick(&mut h, &mut p, 17);
            assert!(p.owner.is_none());
            h.respawn_ticks = 0;
            tick(&mut h, &mut p, 18);
            let s = take(p.owner.take().unwrap()).unwrap();
            assert_eq!(s.form, Form::Human);
            assert_eq!(s.last_body, Some("human_idle"));
            let (name, pos, _) = h.effects.last().unwrap();
            assert!(name.starts_with("mlbb_yu_fx_body_human_idle_"));
            assert_eq!(*pos, (90_000, 900_000));
            assert!(s.tail.is_none() && s.marks.is_empty() && s.grip_ready == 0);
        }
    }

    #[test]
    fn game_owns_ultimate_choice_and_flight_orders() {
        let mut h = Host::new();
        let mut p = setup(&mut h);
        let raw_ult = InputV1::action(InputKindV1::Ult, InputTargetV1::target(2));
        assert!(h.think_with(&mut Ai, Some(raw_ult)).is_none());
        h.sim(|sim| Cast::Dragon.apply(sim, 0, 1, raw_ult.target));
        tick(&mut h, &mut p, 36);
        assert!(matches!(
            h.sim(|sim| STATES.lock().unwrap()[&entity_key_ro(sim, 1)].form),
            Form::Dragon(_)
        ));
        for order in [
            InputV1::move_to(150_000, 200_000),
            InputV1::move_to(220_000, 250_000),
            InputV1::action(InputKindV1::Attack, InputTargetV1::target(2)),
            InputV1::return_home(),
        ] {
            assert!(h.think_with(&mut Ai, Some(order)).is_none());
        }
    }

    #[test]
    fn dragonoid_ai_casts_both_skills_while_approaching_and_respects_retreat() {
        let mut h = Host::new();
        let mut p = setup(&mut h);
        let key = p.owner.unwrap();
        let mut s = take(key).unwrap();
        s.form = Form::Dragonoid(DRAGONOID_DURATION);
        put(key, s);

        h.set_unit_position(2, (260_000, 200_000));
        let approach = Some(InputV1::move_to(260_000, 200_000));
        assert!(h
            .think_with(&mut Ai, Some(InputV1::move_to(170_000, 200_000)))
            .is_none());
        assert!(h
            .think_with(&mut Ai, Some(InputV1::return_home()))
            .is_none());
        let grip = h.think_with(&mut Ai, approach).unwrap();
        assert_eq!(grip.kind, InputKindV1::Skill2.code());
        h.sim(|sim| Cast::Grip.apply(sim, 0, 1, grip.target));
        assert_eq!(hp(&mut h, 2), 531);
        h.now = 5;
        cast(&mut h, Cast::Basic);
        assert_eq!(
            h.sim(|sim| {
                let key = entity_key_ro(sim, 1);
                STATES.lock().unwrap().get(&key).unwrap().motion.unwrap().0
            }),
            "dragonoid_soul_grip"
        );
        for now in 6..=48 {
            tick(&mut h, &mut p, now);
        }
        assert!(h.cc.iter().any(|(id, _)| *id == 2));
        h.set_unit_position(1, (200_000, 200_000));
        h.set_unit_position(2, (240_000, 200_000));
        let tail = h
            .think_with(&mut Ai, Some(InputV1::move_to(240_000, 200_000)))
            .unwrap();
        assert_eq!(tail.kind, InputKindV1::Skill.code());
        h.sim(|sim| Cast::Tail.apply(sim, 0, 1, tail.target));
        tick(&mut h, &mut p, 63);
        let before = hp(&mut h, 2);
        tick(&mut h, &mut p, 72);
        assert_eq!(hp(&mut h, 2), before - 242);
        take(key);
    }

    #[test]
    fn residue_works_on_all_monsters_and_lethal_pulse_pays_once() {
        for kind in [
            EntityKindV1::Jungle,
            EntityKindV1::Epic,
            EntityKindV1::Serpen,
        ] {
            let mut h = Host::new();
            let mut p = setup(&mut h);
            let key = p.owner.unwrap();
            h.set_unit_kind(2, kind);
            h.set_unit_max_hp(2, 10_000);
            h.sim(|sim| {
                sim.entity_set_hp(1, 100);
                sim.entity_set_hp(2, 200);
            });
            for now in 0..MARK_THRESHOLD {
                h.now = now;
                cast(&mut h, Cast::Basic);
            }
            tick(&mut h, &mut p, 17);
            assert_eq!(hp(&mut h, 2), 109);
            assert_eq!(hp(&mut h, 1), 148);
            h.sim(|sim| {
                sim.entity_set_hp(2, 50);
            });
            tick(&mut h, &mut p, 34);
            tick(&mut h, &mut p, 49);
            assert_eq!(hp(&mut h, 2), 0);
            assert_eq!(hp(&mut h, 1), 192);
            assert!(take(key).unwrap().marks.is_empty());
        }
        for kind in [
            EntityKindV1::Minion,
            EntityKindV1::Summon,
            EntityKindV1::Tower,
            EntityKindV1::Nexus,
        ] {
            let mut h = Host::new();
            let p = setup(&mut h);
            h.set_unit_kind(2, kind);
            cast(&mut h, Cast::Basic);
            assert!(take(p.owner.unwrap()).unwrap().marks.is_empty());
        }
    }
}
