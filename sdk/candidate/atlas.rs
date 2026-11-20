//! Atlas's native skill state. The pilot remains the original champion throughout Perfect Match.
use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use crate::{entity_key, entity_key_ro, EntityKey};
use mod_api_stable::*;

#[path = "atlas_visual.rs"]
mod visual;

type Pos = (u64, u64);
const AURA: &str = "mlbb_atlas_frigid_breath";
const EJECTED: &str = "mlbb_atlas_ejected";
const AURA_TICKS: usize = 240;
const EJECTION_TICKS: usize = 120;
const LINK_WINDUP: usize = 24;
const PULL_TICKS: usize = 18;
const LINK_RADIUS: f64 = 42_000.;
const LINK_LEASH: f64 = 55_000.;
const PULSE_RADIUS: f64 = 30_000.;

#[derive(Default)]
struct State {
    tick: Option<usize>,
    previous: Option<Pos>,
    left: bool,
    moving_until: usize,
    aura_until: usize,
    next_frost: usize,
    exposure: HashMap<EntityKey, Exposure>,
    ejection: Option<Ejection>,
    pulse: Option<Pulse>,
    ultimate: Option<Ultimate>,
    motion: Option<visual::Motion>,
    pose: Option<String>,
    pose_dirty: bool,
    sampled_at: usize,
    contacts: [Option<usize>; 4],
}

#[derive(Default)]
struct Exposure {
    entered: Option<usize>,
    freeze_at: usize,
}

struct Ejection {
    since: usize,
    suit: Pos,
    left: bool,
    route: VecDeque<Pos>,
    suit_attack: Option<usize>,
}

struct Pulse {
    next: usize,
    remaining: u8,
}

enum Ultimate {
    Windup { due: usize, links: Vec<EntityKey> },
    Pulling { due: usize, links: Vec<EntityKey> },
}

static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn take(key: EntityKey) -> Option<State> {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key)
}

fn put(key: EntityKey, state: State) {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, state);
}

pub(crate) fn clear_state() {
    STATES.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

fn distance(a: Pos, b: Pos) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
}

fn toward(from: Pos, to: Pos, length: f64) -> Pos {
    let delta = distance(from, to);
    if delta <= length {
        return to;
    }
    let f = length / delta;
    (
        (from.0 as f64 + (to.0 as f64 - from.0 as f64) * f).round() as u64,
        (from.1 as f64 + (to.1 as f64 - from.1 as f64) * f).round() as u64,
    )
}

fn enemies(sim: &StableSim<'_>, team: usize, center: Pos, radius: f64) -> Vec<usize> {
    (0..sim.champion_count())
        .map(|i| sim.champion_id_at(i))
        .filter(|&id| {
            sim.get_entity(id).is_some_and(|e| {
                e.is_alive()
                    && e.is_targetable()
                    && e.is_champion()
                    && e.team() != team
                    && distance(e.pos(), center) <= radius
            })
        })
        .collect()
}

fn valid_link(sim: &StableSim<'_>, target: EntityKey, team: usize, p: Pos) -> bool {
    target == entity_key_ro(sim, target.entity_id)
        && sim.get_entity(target.entity_id).is_some_and(|e| {
            e.is_alive()
                && e.is_targetable()
                && e.is_champion()
                && e.team() != team
                && distance(e.pos(), p) <= LINK_LEASH
        })
}

fn immune(entity: &StableEntity<'_, '_>) -> bool {
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .any(|buff| buff.cc_immune)
}

fn hard_cc(entity: &StableEntity<'_, '_>) -> bool {
    (0..entity.cc_count())
        .filter_map(|i| entity.cc_at(i))
        .any(|cc| {
            matches!(
                CcKindV1::from_code(cc.kind),
                Some(
                    CcKindV1::Airborne
                        | CcKindV1::Stun
                        | CcKindV1::BlockSkill
                        | CcKindV1::ForceMove
                        | CcKindV1::Taunt
                        | CcKindV1::Fear
                        | CcKindV1::Charm
                )
            )
        })
}

fn damage(sim: &mut StableSim<'_>, owner: usize, target: usize, base: usize, scale: usize) {
    let Some(ap) = sim
        .get_entity(owner)
        .filter(|e| e.is_alive())
        .map(|e| e.stat().magic_power)
    else {
        return;
    };
    if sim
        .get_entity(target)
        .is_some_and(|e| e.is_alive() && e.is_targetable())
    {
        sim.deal_damage_typed(
            owner,
            target,
            base + ap * scale / 100,
            DamageTypeV1::Ap,
            AttackTypeV1::Skill,
        );
    }
}

fn slow(sim: &mut StableSim<'_>, id: usize, name: &str, amount: i32, ticks: usize) {
    let Some(e) = sim
        .get_entity(id)
        .filter(|e| e.is_alive() && e.is_targetable())
    else {
        return;
    };
    if immune(&e)
        || (0..e.buff_count())
            .filter_map(|i| e.buff_at(i))
            .any(|buff| buff.name() == name && buff.move_speed_mult < -amount)
    {
        return;
    }
    sim.entity_remove_buff(id, name);
    let mut buff = BuffV1::timed(name, ticks);
    buff.move_speed_mult = -amount;
    sim.add_buff(id, &buff);
}

fn aura_cast(sim: &mut StableSim<'_>, owner: usize, state: &mut State) {
    let now = sim.tick();
    if now >= state.aura_until {
        state.exposure.retain(|_, exposure| {
            exposure.entered = None;
            exposure.freeze_at > now
        });
    }
    state.aura_until = now + AURA_TICKS;
    state.next_frost = now;
    sim.entity_remove_buff(owner, AURA);
    let mut buff = BuffV1::timed(AURA, AURA_TICKS);
    buff.defence = 10;
    buff.magic_resistance = 10;
    sim.add_buff(owner, &buff);
}

fn frost(sim: &mut StableSim<'_>, owner: EntityKey, state: &mut State, p: Pos, team: usize) {
    let now = sim.tick();
    if state.aura_until == 0 || now < state.next_frost {
        return;
    }
    state.next_frost = now + 6;
    let name = format!("mlbb_atlas_chill_{}", owner.copy);
    let nearby = if now < state.aura_until {
        enemies(sim, team, p, 35_000.)
    } else {
        Vec::new()
    };
    let mut present: Vec<EntityKey> = Vec::with_capacity(nearby.len());
    for id in nearby {
        let key = entity_key(sim, id);
        present.push(key);
        let record = state.exposure.entry(key).or_default();
        let entered = *record.entered.get_or_insert(now);
        let age = now.saturating_sub(entered);
        let amount = (5 + age / 6).min(25) as i32;
        slow(sim, id, &name, amount, 7);
        if let Some((pos, cc_immune)) = sim.get_entity(id).map(|e| (e.pos(), immune(&e))) {
            if age >= 120 && now >= record.freeze_at && !cc_immune {
                record.freeze_at = now + 240;
                sim.apply_cc(id, &CcV1::stun(30));
                visual::fx(sim, owner.entity_id, pos, "freeze_impact");
            } else if age >= 36 && now % 24 == 0 {
                visual::fx(sim, owner.entity_id, pos, "frost_exposure");
            }
        }
    }
    state.exposure.retain(|target, entry| {
        if present.contains(target) {
            return true;
        }
        if entry.entered.take().is_some() && *target == entity_key_ro(sim, target.entity_id) {
            sim.entity_remove_buff(target.entity_id, &name);
        }
        entry.freeze_at > now
    });
}

fn pulse(sim: &mut StableSim<'_>, owner: usize, p: Pos, suit: Option<Pos>, team: usize) {
    visual::fx(sim, owner, p, "annihilate_pulse");
    if let Some(center) = suit {
        visual::fx(sim, owner, center, "annihilate_pulse");
    }
    let hits: Vec<_> = (0..sim.entity_count())
        .filter_map(|i| {
            let e = sim.entity_at(i)?;
            if !e.is_alive()
                || !e.is_targetable()
                || e.team() == team
                || !(e.is_champion()
                    || matches!(e.kind(), Some(EntityKindV1::Minion | EntityKindV1::Jungle)))
            {
                return None;
            }
            let a = distance(e.pos(), p) <= PULSE_RADIUS;
            let b = suit.is_some_and(|s| distance(e.pos(), s) <= PULSE_RADIUS);
            (a || b).then_some((e.id(), a && b))
        })
        .collect();
    for (target, overlap) in hits {
        let (base, scale) = if overlap { (45, 15) } else { (30, 10) };
        damage(sim, owner, target, base, scale);
    }
}

fn begin_ejection(sim: &mut StableSim<'_>, id: usize, state: &mut State, p: Pos) {
    let now = sim.tick();
    state.ejection = Some(Ejection {
        since: now,
        suit: p,
        left: state.left,
        route: VecDeque::new(),
        suit_attack: None,
    });
    let mut buff = BuffV1::timed(EJECTED, EJECTION_TICKS);
    buff.move_speed_mult = 25;
    buff.damaged_reduce = 20;
    sim.add_buff(id, &buff);
    visual::motion(state, "skill2", now, 18);
    visual::fx(sim, id, p, "ejection_flash");
    let _ = sim.play_sfx("mlbb_atlas_ejection", id, &InputTargetV1::target(id));
}

fn rejoin(
    sim: &mut StableSim<'_>,
    owner: EntityKey,
    state: &mut State,
    p: Pos,
    team: usize,
    early: bool,
) {
    if state.ejection.take().is_none() {
        return;
    }
    let id = owner.entity_id;
    sim.entity_remove_buff(id, EJECTED);
    let _ = sim.play_sfx("mlbb_atlas_reunion", id, &InputTargetV1::target(id));
    if early {
        let name = format!("mlbb_atlas_link_slow_{}", owner.copy);
        for target in enemies(sim, team, p, 30_000.) {
            slow(sim, target, &name, 40, 36);
        }
        visual::fx(sim, id, p, "reunion_slow");
    } else {
        visual::motion(state, "rejoin", sim.tick(), 18);
        for target in enemies(sim, team, p, 30_000.) {
            damage(sim, id, target, 65, 20);
            if sim
                .get_entity(target)
                .is_some_and(|e| e.is_alive() && !immune(&e))
            {
                sim.apply_cc(target, &CcV1::stun(36));
            }
        }
        visual::fx(sim, id, p, "reunion_stun");
    }
}

fn advance_suit(ejection: &mut Ejection, previous: Option<Pos>, p: Pos, now: usize) -> bool {
    let age = now.saturating_sub(ejection.since);
    if previous.is_some_and(|old| distance(old, p) > 100.) {
        ejection.route.push_back(p);
    }
    if age < 18 {
        return false;
    }
    let mut budget = (550. + 22. * (age - 18) as f64).min(3200.);
    let old = ejection.suit;
    while let Some(next) = ejection.route.front().copied() {
        let len = distance(ejection.suit, next);
        if len > budget {
            ejection.suit = toward(ejection.suit, next, budget);
            break;
        }
        budget -= len;
        ejection.suit = next;
        ejection.route.pop_front();
    }
    if ejection.suit.0 != old.0 {
        ejection.left = ejection.suit.0 < old.0;
    }
    age >= EJECTION_TICKS
        || (age >= 36 && ejection.route.is_empty() && distance(ejection.suit, p) <= 6_000.)
}

fn ultimate_tick(sim: &mut StableSim<'_>, owner: EntityKey, s: &mut State, p: Pos, team: usize) {
    let id = owner.entity_id;
    let now = sim.tick();
    match s.ultimate.take() {
        Some(Ultimate::Windup { due, mut links }) => {
            if sim.get_entity(id).is_some_and(|e| hard_cc(&e)) {
                s.motion = None;
                return;
            }
            links.retain(|target| valid_link(sim, *target, team, p));
            if links.is_empty() {
                return;
            }
            if now < due {
                if visual::sample(now) {
                    visual::chains(sim, id, p, &links);
                }
                if !links.is_empty() {
                    crate::audio::play_priority_voice(sim, id, "mlbb_atlas_voice_ult");
                }
                s.ultimate = Some(Ultimate::Windup { due, links });
            } else {
                links.retain(|t| sim.entity_pull(id, t.entity_id, 3_000, PULL_TICKS));
                if !links.is_empty() {
                    s.ultimate = Some(Ultimate::Pulling {
                        due: now + PULL_TICKS,
                        links,
                    });
                }
            }
        }
        Some(Ultimate::Pulling { due, links }) if now < due => {
            if visual::sample(now) {
                visual::chains(sim, id, p, &links);
            }
            s.ultimate = Some(Ultimate::Pulling { due, links });
        }
        Some(Ultimate::Pulling { links, .. }) => {
            visual::motion(s, "ult_slam", now, 24);
            visual::fx(sim, id, p, "slam_impact");
            for t in links
                .into_iter()
                .filter(|target| valid_link(sim, *target, team, p))
                .collect::<Vec<_>>()
            {
                if sim.get_entity(t.entity_id).is_some_and(|e| !immune(&e)) {
                    damage(sim, id, t.entity_id, 105, 25);
                    if sim.get_entity(t.entity_id).is_some_and(|e| e.is_alive()) {
                        sim.apply_cc(t.entity_id, &CcV1::of_kind(CcKindV1::Airborne, 21));
                    }
                }
            }
        }
        None => (),
    }
}

fn remove_pose(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    if let Some(name) = s.pose.take() {
        sim.entity_remove_buff(id, &name);
    }
    sim.entity_remove_buff(id, "mlbb_atlas_s1_ice");
    sim.entity_remove_buff(id, "mlbb_atlas_s1_ice_left");
}

fn update(sim: &mut StableSim<'_>, key: EntityKey, mut s: State) {
    let id = key.entity_id;
    let now = sim.tick();
    let Some((p, team)) = sim
        .get_entity(id)
        .filter(|e| e.is_alive())
        .map(|e| (e.pos(), e.team()))
    else {
        remove_pose(sim, id, &mut s);
        sim.entity_remove_buff(id, EJECTED);
        return;
    };
    if sim.is_end() {
        remove_pose(sim, id, &mut s);
        sim.entity_remove_buff(id, EJECTED);
        return;
    }
    if s.tick == Some(now) {
        put(key, s);
        return;
    }
    s.tick = Some(now);
    if let Some(old) = s.previous {
        // The stable ABI does not expose a recall channel. Discard pending
        // local effects when native relocation abruptly changes position.
        if distance(old, p) > 24_000. {
            s.ejection = None;
            s.pulse = None;
            s.ultimate = None;
            s.motion = None;
            sim.entity_remove_buff(id, EJECTED);
        } else if distance(old, p) > 100. {
            if old.0.abs_diff(p.0) > 100 && s.motion.is_none_or(|m| m.until <= now) {
                s.left = p.0 < old.0;
            }
            s.moving_until = now + 6;
        }
    }
    let previous = s.previous;
    if s.ejection
        .as_mut()
        .is_some_and(|e| advance_suit(e, previous, p, now))
    {
        rejoin(sim, key, &mut s, p, team, false);
    }
    s.previous = Some(p);
    ultimate_tick(sim, key, &mut s, p, team);
    if let Some(mut burst) = s.pulse.take() {
        if now >= burst.next {
            pulse(sim, id, p, s.ejection.as_ref().map(|e| e.suit), team);
            burst.remaining -= 1;
            burst.next = now + 12;
        }
        if burst.remaining != 0 {
            s.pulse = Some(burst);
        }
    }
    frost(sim, key, &mut s, p, team);
    visual::pose(sim, id, &mut s);
    visual::suit(sim, id, &mut s);
    put(key, s);
}

#[derive(Clone, Copy)]
enum Cast {
    Basic,
    Annihilate,
    PerfectMatch,
    FatalLinks,
}

impl StableEffectType for Cast {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, _: InputTargetV1) {
        let key = entity_key_ro(sim, id);
        let Some(mut s) = take(key) else {
            return;
        };
        let Some((p, team)) = sim
            .get_entity(id)
            .filter(|e| e.is_alive())
            .map(|e| (e.pos(), e.team()))
        else {
            return;
        };
        let now = sim.tick();
        let index = *self as usize;
        if s.contacts[index] == Some(now) {
            put(key, s);
            return;
        }
        s.contacts[index] = Some(now);
        if !matches!(self, Self::Basic) {
            aura_cast(sim, id, &mut s);
        }
        match self {
            Self::Basic => {
                if s.motion.is_none_or(|motion| motion.until <= now) && s.ultimate.is_none() {
                    let name = if s.ejection.is_some() {
                        "pilot_attack"
                    } else {
                        "attack"
                    };
                    visual::motion(&mut s, name, now, 22);
                }
            }
            Self::Annihilate => {
                crate::audio::play_voice(sim, id, "mlbb_atlas_voice_skill");
                let name = if let Some(e) = s.ejection.as_mut() {
                    e.suit_attack = Some(now);
                    "pilot_s1"
                } else {
                    "skill"
                };
                visual::motion(&mut s, name, now, 36);
                pulse(sim, id, p, s.ejection.as_ref().map(|e| e.suit), team);
                s.pulse = Some(Pulse {
                    next: now + 12,
                    remaining: 2,
                });
            }
            Self::PerfectMatch => {
                if s.ejection.is_none() {
                    begin_ejection(sim, id, &mut s, p);
                }
            }
            Self::FatalLinks => {
                rejoin(sim, key, &mut s, p, team, true);
                visual::motion(&mut s, "ult", now, LINK_WINDUP);
                let name = format!("mlbb_atlas_link_slow_{}", key.copy);
                let mut links = Vec::new();
                for target in enemies(sim, team, p, LINK_RADIUS) {
                    if !sim.is_visible(team, target) {
                        continue;
                    }
                    links.push(entity_key(sim, target));
                    damage(sim, id, target, 35, 10);
                    slow(sim, target, &name, 30, LINK_WINDUP);
                    if let Some(pos) = sim
                        .get_entity(target)
                        .filter(|e| e.is_alive())
                        .map(|e| e.pos())
                    {
                        visual::fx(sim, id, pos, "link_head");
                    }
                }
                if !links.is_empty() {
                    crate::audio::play_priority_voice(sim, id, "mlbb_atlas_voice_ult");
                }
                s.ultimate = Some(Ultimate::Windup {
                    due: now + LINK_WINDUP,
                    links,
                });
            }
        }
        visual::pose(sim, id, &mut s);
        put(key, s);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        match self {
            Self::Basic => (0, 0), // The declarative base attack owns its packet.
            Self::Annihilate => (0, 90 + stat.magic_power * 30 / 100),
            Self::PerfectMatch => (0, 65 + stat.magic_power * 20 / 100),
            Self::FatalLinks => (0, 140 + stat.magic_power * 35 / 100),
        }
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

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize) {
        let key = entity_key(sim, id);
        if let Some(old) = self.owner.replace(key) {
            take(old);
        }
        let mut state = State::default();
        visual::pose(sim, id, &mut state);
        put(key, state);
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, id: usize) {
        let key = entity_key_ro(sim, id);
        if self.owner != Some(key) {
            self.on_spawn(sim, player, id);
        }
        if let Some(key) = self.owner {
            if let Some(state) = take(key) {
                update(sim, key, state);
            }
        }
    }

    fn on_damaged(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize, _: usize, _: usize) {
        let key = entity_key_ro(sim, id);
        if let Some(mut s) = take(key) {
            if s.motion.is_none_or(|m| m.until <= sim.tick()) {
                let name = if s.ejection.is_some() {
                    "pilot_hit"
                } else {
                    "hit"
                };
                visual::motion(&mut s, name, sim.tick(), 12);
                visual::pose(sim, id, &mut s);
            }
            put(key, s);
        }
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        if let Some(key) = self.owner {
            if let Some(mut state) = take(key) {
                let id = key.entity_id;
                if let Some(p) = sim.get_entity(id).map(|e| e.pos()) {
                    visual::death(sim, id, p, state.ejection.is_some(), state.left);
                }
                remove_pose(sim, id, &mut state);
                sim.entity_remove_buff(id, EJECTED);
                sim.entity_remove_buff(id, AURA);
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
        "mlbb_atlas_ai".into()
    }
    fn priority(&self) -> i32 {
        175
    }
    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name == "mlbb_atlas"
    }

    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        let base = base?;
        if matches!(
            InputKindV1::from_code(base.kind),
            Some(InputKindV1::Move | InputKindV1::Return)
        ) || base.target.kind != InputTargetKindV1::Target.code()
        {
            return None;
        }
        let player = ctx.player_id();
        let proposals = {
            let sim = ctx.sim()?;
            let e = sim.get_player(player)?.champion()?;
            let target = sim.get_entity(base.target.target_id).filter(|t| {
                t.is_champion()
                    && t.is_alive()
                    && t.is_targetable()
                    && t.team() != e.team()
                    && sim.is_visible(e.team(), t.id())
            })?;
            let states = STATES.lock().unwrap_or_else(|e| e.into_inner());
            let s = states.get(&entity_key_ro(&sim, e.id()))?;
            if s.ultimate.is_some() || hard_cc(&e) {
                return None;
            }
            let ready = |slot| {
                e.cooldown(slot)
                    .is_some_and(|c| c.unlocked && c.charges > 0 && c.remaining == 0)
            };
            let close = distance(e.pos(), target.pos());
            let linked = enemies(&sim, e.team(), e.pos(), LINK_RADIUS)
                .into_iter()
                .filter(|&enemy| sim.is_visible(e.team(), enemy))
                .count();
            let nearby_ally = (0..sim.champion_count())
                .filter_map(|i| sim.get_entity(sim.champion_id_at(i)))
                .any(|ally| {
                    ally.id() != e.id()
                        && ally.is_alive()
                        && ally.team() == e.team()
                        && distance(ally.pos(), target.pos()) <= 55_000.
                });
            [
                (
                    ready(SkillSlotV1::Ult) && close <= LINK_RADIUS && (linked >= 2 || nearby_ally),
                    InputKindV1::Ult,
                ),
                (
                    ready(SkillSlotV1::Skill)
                        && (close <= PULSE_RADIUS
                            || s.ejection.as_ref().is_some_and(|suit| {
                                distance(suit.suit, target.pos()) <= PULSE_RADIUS
                            })),
                    InputKindV1::Skill,
                ),
                (
                    ready(SkillSlotV1::Skill2)
                        && s.ejection.is_none()
                        && (18_000. ..=55_000.).contains(&close),
                    InputKindV1::Skill2,
                ),
            ]
        };
        proposals
            .into_iter()
            .filter(|(ready, _)| *ready)
            .map(|(_, kind)| InputV1::action(kind, InputTargetV1::default()))
            .find(|i| ctx.is_valid_input(i))
    }
}

pub(crate) fn register(module: &mut StableMod) {
    module.add_native_effect("mlbb_heroes:atlas_basic", Cast::Basic);
    module.add_native_effect("mlbb_heroes:atlas_annihilate", Cast::Annihilate);
    module.add_native_effect("mlbb_heroes:atlas_perfect_match", Cast::PerfectMatch);
    module.add_native_effect("mlbb_heroes:atlas_fatal_links", Cast::FatalLinks);
    module.add_native_passive("mlbb_heroes:atlas", Passive::default());
    module.add_player_input_ai(Ai);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hanzo::tests::Host;

    fn spawn(host: &mut Host) -> Passive {
        let mut passive = Passive::default();
        host.sim(|sim| passive.on_spawn(sim, 1, 1));
        passive
    }

    fn use_skill(host: &mut Host, skill: Cast) {
        host.sim(|sim| skill.apply(sim, 0, 1, InputTargetV1::default()));
    }

    fn tick(host: &mut Host, passive: &mut Passive, now: usize) {
        host.now = now;
        host.sim(|sim| passive.on_update(sim, 0, 1, 1));
    }

    #[test]
    fn annihilate_overlap_pays_one_packet_per_pulse() {
        let mut host = Host::new();
        let mut passive = spawn(&mut host);
        use_skill(&mut host, Cast::PerfectMatch);
        use_skill(&mut host, Cast::Annihilate);
        use_skill(&mut host, Cast::Annihilate); // Duplicate apply at the same tick.
        tick(&mut host, &mut passive, 12);
        tick(&mut host, &mut passive, 12); // Duplicate update at the same tick.
        tick(&mut host, &mut passive, 24);
        assert_eq!(host.typed.len(), 3);
        host.sim(|sim| assert_eq!(sim.get_entity(2).unwrap().hp().0, 700 - 3 * 45));
    }

    #[test]
    fn fatal_links_rejoins_without_normal_reunion_damage_and_needs_accepted_pull() {
        let mut host = Host::new();
        let mut passive = spawn(&mut host);
        use_skill(&mut host, Cast::PerfectMatch);
        use_skill(&mut host, Cast::FatalLinks);
        host.sim(|sim| {
            let enemy = sim.get_entity(2).unwrap();
            assert_eq!(enemy.hp().0, 665);
            assert!((0..enemy.buff_count())
                .filter_map(|i| enemy.buff_at(i))
                .any(|b| b.move_speed_mult == -40));
            assert_eq!(sim.get_entity(1).unwrap().hp(), (700, 700));
        });
        tick(&mut host, &mut passive, 24);
        tick(&mut host, &mut passive, 42);
        assert_eq!(host.pulls, vec![(1, 2, 18)]);
        assert_eq!(host.typed.len(), 1);
        assert!(host.cc.is_empty());
    }

    #[test]
    fn accepted_pull_slams_once_and_revalidates_its_victim() {
        let mut host = Host::new();
        host.pull_allowed = true;
        let mut passive = spawn(&mut host);
        use_skill(&mut host, Cast::FatalLinks);
        tick(&mut host, &mut passive, 24);
        tick(&mut host, &mut passive, 42);
        tick(&mut host, &mut passive, 43);
        assert_eq!(host.typed.len(), 2);
        assert_eq!(host.cc, vec![(2, CcV1::of_kind(CcKindV1::Airborne, 21))]);

        host.now = 2700;
        use_skill(&mut host, Cast::FatalLinks);
        tick(&mut host, &mut passive, 2724);
        host.set_unit_position(2, (280_000, 200_000));
        tick(&mut host, &mut passive, 2742);
        assert_eq!(host.typed.len(), 3); // Initial contact, then target left the leash.
        assert_eq!(host.cc.len(), 1);
    }

    #[test]
    fn frost_leaving_resets_exposure_but_keeps_freeze_cooldown() {
        let mut host = Host::new();
        let mut passive = spawn(&mut host);
        use_skill(&mut host, Cast::Annihilate);
        for now in (0..=120).step_by(6) {
            tick(&mut host, &mut passive, now);
        }
        assert_eq!(host.cc, vec![(2, CcV1::stun(30))]);
        host.set_unit_position(2, (250_000, 200_000));
        tick(&mut host, &mut passive, 126);
        host.set_unit_position(2, (215_000, 200_000));
        host.now = 132;
        use_skill(&mut host, Cast::PerfectMatch);
        // Keep the suit distant enough that this test observes only frost CC.
        host.set_unit_position(1, (190_000, 200_000));
        for now in (132..=372).step_by(6) {
            tick(&mut host, &mut passive, now);
            if now == 150 {
                host.set_unit_position(2, (224_000, 200_000));
            }
        }
        assert_eq!(host.cc.iter().filter(|(_, cc)| cc.tick == 30).count(), 2);
    }
}
