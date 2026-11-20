//! Angela: live Love Waves, source-owned Puppet String, and banished Heartguard.
//! All authoritative timers/casts run in the simulation, never the read-only AI hook.
//! Match-tick fallback is composed with Zhask's existing hook; tick guards prevent
//! double updates. State uses the pack's simulation-copy keys (pre-sims/replays).
use crate::EntityKey;
use mod_api_stable::*;
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

const MARK_BUFF: &str = "mlbb_heroes_angela_lovers_mark";
const SHIELD_BUFF: &str = "mlbb_heroes_angela_heartguard";
const MARK_TICKS: usize = 180;
const MARK_MAX_STACKS: usize = 5;
const MARK_DAMAGE_PER_STACK: usize = 12;
const STRING_TICKS: usize = 180;
const ROOT_TICKS: usize = 42;
const STRING_CAST_RANGE: f64 = 95_000.0;
const STRING_BREAK_RANGE: f64 = 150_000.0;
const S1_RANGE: f64 = 92_000.0;
const S1_CONE_COS: f64 = 0.42;
const S1_BASE: usize = 80;
const S1_AP_PCT: usize = 30;
const S1_HEAL_BASE: usize = 46;
const S1_HEAL_AP_PCT: usize = 40;
const WAVE_TICKS: usize = 27;
const HEARTGUARD_SHIELD: usize = 220;
const HEARTGUARD_AP_PCT: usize = 180;
const HEARTGUARD_TICKS: usize = 360;
const ATTACH_TICKS: usize = 720;
const BANISH_TICKS: usize = 2;
const SMART_HEART_TICKS: usize = 240;
const SMART_HEART_MAX_STACKS: usize = 4;
const ACTION_TICKS: usize = 24;

type Pos = (u64, u64);
fn percent(v: usize, p: usize) -> usize {
    v.saturating_mul(p) / 100
}
fn distance_sq(a: Pos, b: Pos) -> f64 {
    let (x, y) = (a.0 as f64 - b.0 as f64, a.1 as f64 - b.1 as f64);
    x * x + y * y
}
fn unit(x: f64, y: f64) -> (f64, f64) {
    let n = x.hypot(y);
    if n > 1.0 {
        (x / n, y / n)
    } else {
        (1.0, 0.0)
    }
}
fn pos_of(sim: &StableSim<'_>, id: usize) -> Option<Pos> {
    sim.get_entity(id).map(|e| e.pos())
}
fn named(sim: &StableSim<'_>, id: usize, name: &str) -> bool {
    stacks(sim, id, name) > 0
}
fn stacks(sim: &StableSim<'_>, id: usize, name: &str) -> usize {
    sim.get_entity(id).map_or(0, |e| {
        (0..e.buff_count())
            .filter_map(|i| e.buff_at(i))
            .filter(|b| b.name() == name)
            .count()
    })
}
fn string_name(id: usize) -> String {
    format!("angela_string_{id}")
}
fn slow_name(id: usize) -> String {
    format!("angela_string_slow_{id}")
}
fn smart_name(id: usize) -> String {
    format!("angela_smart_{id}")
}
fn mark_bonus_percent(sim: &StableSim<'_>, id: usize) -> usize {
    stacks(sim, id, MARK_BUFF).min(5) * MARK_DAMAGE_PER_STACK
}
fn apply_mark(sim: &mut StableSim<'_>, id: usize) {
    // Per-stack slow, not a team-wide damage-taken amplifier. Only S1 and S2
    // detonation explicitly read the mark damage bonus, as the approved kit says.
    let b = BuffV1 {
        move_speed_mult: -6,
        ..BuffV1::timed(MARK_BUFF, MARK_TICKS)
    };
    sim.entity_stack_buff(id, &b, MARK_MAX_STACKS, true);
}
fn can_render(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        .is_some_and(|k| {
            matches!(
                k,
                SimOriginKindV1::ClientMatchView
                    | SimOriginKindV1::ClientReplay
                    | SimOriginKindV1::ClientSpectate
            )
        })
}

/// Native cooldown debt follows the SDK charge model: every use adds one
/// full_cycle/max_charges interval. Debt is shared across ground/attachment,
/// so banish and detach never grant a second set of charges.
#[derive(Clone, Debug)]
struct Clock {
    debt: usize,
    cycle: usize,
    max: usize,
    unlocked: bool,
}
impl Clock {
    fn new(cycle: usize, max: usize) -> Self {
        Self {
            debt: 0,
            cycle,
            max,
            unlocked: true,
        }
    }
    fn cost(&self) -> usize {
        (self.cycle / self.max.max(1)).max(1)
    }
    fn ready(&self) -> bool {
        self.unlocked && self.debt <= self.cycle.saturating_sub(self.cost())
    }
    fn spend(&mut self) -> bool {
        if !self.ready() {
            return false;
        }
        self.debt += self.cost();
        true
    }
    fn elapse(&mut self, n: usize) {
        self.debt = self.debt.saturating_sub(n);
    }
}
#[derive(Clone)]
struct StringLink {
    target: usize,
    started: usize,
}
#[derive(Clone)]
struct Ride {
    host: usize,
    until: usize,
    last: Pos,
}
#[derive(Clone)]
struct Wave {
    origin: Pos,
    aim: (f64, f64),
    started: usize,
    hit: HashSet<usize>,
    damage: usize,
    heal: usize,
}
#[derive(Clone)]
struct Runtime {
    updated: Option<usize>,
    clock_tick: usize,
    action_until: usize,
    s1: Clock,
    s2: Clock,
    ult: Clock,
    link: Option<StringLink>,
    ride: Option<Ride>,
    release: Option<(usize, Pos)>,
    waves: Vec<Wave>,
    recast_reset: bool,
}
impl Runtime {
    fn new(now: usize) -> Self {
        Self {
            updated: None,
            clock_tick: now,
            action_until: 0,
            s1: Clock::new(300, 5),
            s2: Clock::new(480, 1),
            ult: Clock::new(2400, 1),
            link: None,
            ride: None,
            release: None,
            waves: vec![],
            recast_reset: false,
        }
    }
    fn elapse(&mut self, now: usize) {
        let n = now.saturating_sub(self.clock_tick);
        self.s1.elapse(n);
        self.s2.elapse(n);
        self.ult.elapse(n);
        self.clock_tick = now;
    }
}
static STATES: LazyLock<Mutex<HashMap<EntityKey, Runtime>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn take(sim: &mut StableSim<'_>, id: usize) -> (EntityKey, Runtime) {
    let k = crate::entity_key(sim, id);
    let mut s = STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&k)
        .unwrap_or_else(|| Runtime::new(sim.tick()));
    s.elapse(sim.tick());
    (k, s)
}
fn put(k: EntityKey, s: Runtime) {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(k, s);
}
fn origin(sim: &StableSim<'_>, id: usize, s: &Runtime) -> Option<Pos> {
    s.ride
        .as_ref()
        .and_then(|r| pos_of(sim, r.host))
        .or_else(|| pos_of(sim, id))
}
// BuffV1 multipliers are WHOLE percentages, not permille. Integer cumulative
// values approximate 7.5% per cast as 7%, 15%, 22%, 30% (never above 30%).
fn smart_total_percent(count: usize) -> i32 {
    (count.min(SMART_HEART_MAX_STACKS) * 15 / 2) as i32
}
fn smart(sim: &mut StableSim<'_>, id: usize, s: &Runtime) {
    let target = s.ride.as_ref().map_or(id, |r| r.host);
    let name = smart_name(id);
    let count = (stacks(sim, target, &name) + 1).min(SMART_HEART_MAX_STACKS);
    // Normalize and refresh only this Angela's copies, including a partial
    // dispel case. Contributions are 7/8/7/8%, adding to the intended cap.
    sim.entity_remove_buff(target, &name);
    for i in 0..count {
        let b = BuffV1 {
            move_speed_mult: smart_total_percent(i + 1) - smart_total_percent(i),
            ..BuffV1::timed(&name, SMART_HEART_TICKS)
        };
        sim.entity_stack_buff(target, &b, SMART_HEART_MAX_STACKS, false);
    }
}
fn move_smart(sim: &mut StableSim<'_>, id: usize, from: usize, to: usize) {
    let name = smart_name(id);
    let buffs: Vec<_> = sim
        .get_entity(from)
        .map(|e| {
            (0..e.buff_count())
                .filter_map(|i| e.buff_at(i))
                .filter(|b| b.name() == name)
                .collect()
        })
        .unwrap_or_default();
    sim.entity_remove_buff(from, &name);
    for b in buffs {
        sim.entity_stack_buff(to, &b, SMART_HEART_MAX_STACKS, false);
    }
}
fn direction(sim: &StableSim<'_>, p: Pos, input: InputTargetV1) -> Option<(f64, f64)> {
    if input.kind == InputTargetKindV1::Target.code() {
        let q = pos_of(sim, input.target_id)?;
        Some(unit(q.0 as f64 - p.0 as f64, q.1 as f64 - p.1 as f64))
    } else if input.kind == InputTargetKindV1::Pos.code() {
        Some(unit(
            input.x as f64 - p.0 as f64,
            input.y as f64 - p.1 as f64,
        ))
    } else if input.kind == InputTargetKindV1::Dir.code() {
        Some(unit(input.dir_x as f64, input.dir_y as f64))
    } else {
        None
    }
}
fn valid_enemy(sim: &StableSim<'_>, caster: usize, target: usize, p: Pos, range: f64) -> bool {
    let Some(c) = sim.get_entity(caster) else {
        return false;
    };
    sim.get_entity(target).is_some_and(|e| {
        e.team() != c.team()
            && e.is_alive()
            && e.is_champion()
            && e.is_targetable()
            && distance_sq(p, e.pos()) <= range * range
    })
}
fn sync_tuning(sim: &StableSim<'_>, id: usize, s: &mut Runtime) {
    // Read level locks and item-adjusted cycle lengths; never import engine
    // debt here, because native attached casts cannot increment that counter.
    if let Some(e) = sim.get_entity(id) {
        for (slot, c) in [
            (SkillSlotV1::Skill, &mut s.s1),
            (SkillSlotV1::Skill2, &mut s.s2),
            (SkillSlotV1::Ult, &mut s.ult),
        ] {
            if let Some(cd) = e.cooldown(slot) {
                c.unlocked = cd.unlocked;
                if cd.cooltime > 0 {
                    c.cycle = cd.cooltime;
                    c.max = cd.max_charges.max(1);
                }
            }
        }
    }
}
fn cast_wave(sim: &mut StableSim<'_>, id: usize, input: InputTargetV1, s: &mut Runtime) -> bool {
    let Some(p) = origin(sim, id, s) else {
        return false;
    };
    let Some(aim) = direction(sim, p, input) else {
        return false;
    };
    if sim.tick() < s.action_until || !s.s1.spend() {
        return false;
    }
    let ap = sim.get_entity(id).map_or(0, |e| e.stat().magic_power);
    s.waves.push(Wave {
        origin: p,
        aim,
        started: sim.tick(),
        hit: HashSet::new(),
        damage: S1_BASE + percent(ap, S1_AP_PCT),
        heal: S1_HEAL_BASE + percent(ap, S1_HEAL_AP_PCT),
    });
    s.action_until = sim.tick() + ACTION_TICKS;
    smart(sim, id, s);
    true
}
fn end_string(sim: &mut StableSim<'_>, id: usize, s: &mut Runtime) {
    if let Some(l) = s.link.take() {
        sim.entity_remove_buff(l.target, &string_name(id));
        sim.entity_remove_buff(l.target, &slow_name(id));
    }
}
fn cast_string(sim: &mut StableSim<'_>, id: usize, target: usize, s: &mut Runtime) -> bool {
    let Some(p) = origin(sim, id, s) else {
        return false;
    };
    if sim.tick() < s.action_until
        || !valid_enemy(sim, id, target, p, STRING_CAST_RANGE)
        || !s.s2.spend()
    {
        return false;
    }
    end_string(sim, id, s);
    sim.entity_stack_buff(
        target,
        &BuffV1::timed(&string_name(id), STRING_TICKS + 5),
        1,
        true,
    );
    let ap = sim.get_entity(id).map_or(0, |e| e.stat().magic_power);
    sim.deal_damage_typed(
        id,
        target,
        95 + percent(ap, 25),
        DamageTypeV1::Ap,
        AttackTypeV1::Skill,
    );
    s.link = Some(StringLink {
        target,
        started: sim.tick(),
    });
    s.action_until = sim.tick() + ACTION_TICKS;
    smart(sim, id, s);
    true
}
fn end_ride(sim: &mut StableSim<'_>, id: usize, s: &mut Runtime) {
    if let Some(r) = s.ride.take() {
        move_smart(sim, id, r.host, id);
        // SetPos/Grab do not move banished units. Stop refreshing and wait for
        // the short window to expire, then restore Angela at the last host pos.
        s.release = Some((sim.tick() + BANISH_TICKS + 1, r.last));
        s.recast_reset = false;
    }
}
fn cast_heartguard(sim: &mut StableSim<'_>, id: usize, target: usize, s: &mut Runtime) -> bool {
    if s.ride.is_some() {
        end_ride(sim, id, s);
        return true;
    }
    let Some(c) = sim.get_entity(id) else {
        return false;
    };
    let team = c.team();
    let ap = c.stat().magic_power;
    let Some(host) = sim.get_entity(target) else {
        return false;
    };
    if target == id
        || host.team() != team
        || !host.is_champion()
        || !host.is_alive()
        || !host.is_targetable()
        || s.release.is_some()
        || !s.ult.spend()
    {
        return false;
    }
    let p = host.pos();
    // Exactly one shield application. The data file deliberately has no Shield.
    sim.entity_add_shield(
        target,
        HEARTGUARD_SHIELD + percent(ap, HEARTGUARD_AP_PCT),
        HEARTGUARD_TICKS,
    );
    sim.entity_stack_buff(
        target,
        &BuffV1::timed(SHIELD_BUFF, HEARTGUARD_TICKS),
        1,
        true,
    );
    move_smart(sim, id, id, target);
    s.ride = Some(Ride {
        host: target,
        until: sim.tick() + ATTACH_TICKS,
        last: p,
    });
    smart(sim, id, s);
    s.recast_reset = true;
    sim.entity_banish(id, id, BANISH_TICKS, "", "");
    true
}

fn draw_string(sim: &mut StableSim<'_>, caster: usize, from: Pos, to: Pos) {
    if !can_render(sim) || sim.tick() % 2 != 0 {
        return;
    }
    let (dx, dy) = (to.0 as f64 - from.0 as f64, to.1 as f64 - from.1 as f64);
    let n = dx.hypot(dy);
    if n < 1.0 {
        return;
    }
    // Estes's supplied-art technique: overlapping short-lived sprites in 12
    // pre-rotated buckets, regenerated from live endpoints every two ticks.
    let theta = dy.atan2(dx).rem_euclid(std::f64::consts::PI);
    let bucket = ((theta / std::f64::consts::PI * 12.0).round() as usize) % 12;
    let count = (n / 12_000.0).ceil().clamp(1.0, 24.0) as usize;
    for i in 0..count {
        let t = (i as f64 + 0.5) / count as f64;
        let p = InputTargetV1::pos(
            (from.0 as f64 + dx * t).max(0.0) as u64,
            (from.1 as f64 + dy * t).max(0.0) as u64,
        );
        sim.play_view_effect(&format!("angela_wire_{bucket}"), caster, &p, 0, 0, 0);
    }
}
fn tick_string(sim: &mut StableSim<'_>, id: usize, s: &mut Runtime) {
    let Some(l) = s.link.clone() else {
        return;
    };
    let Some(p) = origin(sim, id, s) else {
        end_string(sim, id, s);
        return;
    };
    if !valid_enemy(sim, id, l.target, p, STRING_BREAK_RANGE)
        || !named(sim, l.target, &string_name(id))
    {
        end_string(sim, id, s);
        return;
    }
    let elapsed = sim.tick().saturating_sub(l.started);
    if elapsed >= STRING_TICKS {
        let ap = sim.get_entity(id).map_or(0, |e| e.stat().magic_power);
        let damage = 140 + percent(ap, 35);
        let bonus = mark_bonus_percent(sim, l.target);
        sim.deal_damage_typed(
            id,
            l.target,
            damage + percent(damage, bonus),
            DamageTypeV1::Ap,
            AttackTypeV1::Skill,
        );
        sim.apply_cc(l.target, &CcV1::of_kind(CcKindV1::Bind, ROOT_TICKS as u64));
        sim.play_view_effect(
            "angela_immobilize",
            id,
            &InputTargetV1::target(l.target),
            0,
            0,
            0,
        );
        end_string(sim, id, s);
        return;
    }
    let slow = BuffV1 {
        move_speed_mult: -((elapsed * 45 / STRING_TICKS) as i32),
        ..BuffV1::timed(&slow_name(id), 3)
    };
    sim.entity_remove_buff(l.target, &slow_name(id));
    sim.entity_stack_buff(l.target, &slow, 1, true);
    if let Some(to) = pos_of(sim, l.target) {
        draw_string(sim, id, p, to);
    }
}
fn wave_contains(w: &Wave, p: Pos, travel: f64) -> bool {
    let (x, y) = (
        p.0 as f64 - w.origin.0 as f64,
        p.1 as f64 - w.origin.1 as f64,
    );
    let d = x.hypot(y);
    d <= travel && d <= S1_RANGE && (d < 1.0 || (x * w.aim.0 + y * w.aim.1) / d >= S1_CONE_COS)
}
fn tick_waves(sim: &mut StableSim<'_>, id: usize, s: &mut Runtime) {
    let Some(team) = sim.get_entity(id).map(|e| e.team()) else {
        return;
    };
    for w in &mut s.waves {
        let elapsed = sim.tick().saturating_sub(w.started);
        let travel = S1_RANGE * ((elapsed + 1).min(WAVE_TICKS) as f64 / WAVE_TICKS as f64);
        let hits: Vec<_> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i))
            .filter(|e| {
                e.is_champion()
                    && e.is_alive()
                    && (e.is_targetable() || e.id() == id)
                    && !w.hit.contains(&e.id())
                    && wave_contains(w, e.pos(), travel)
            })
            .map(|e| (e.id(), e.team() == team))
            .collect();
        for (target, ally) in hits {
            w.hit.insert(target);
            if ally {
                sim.heal(id, target, w.heal);
            } else {
                let bonus = mark_bonus_percent(sim, target);
                sim.deal_damage_typed(
                    id,
                    target,
                    w.damage + percent(w.damage, bonus),
                    DamageTypeV1::Ap,
                    AttackTypeV1::Skill,
                );
                apply_mark(sim, target);
            }
        }
        if can_render(sim) && elapsed % 3 == 0 && elapsed < WAVE_TICKS {
            let a = w.aim.1.atan2(w.aim.0).rem_euclid(std::f64::consts::TAU);
            let bucket = ((a / std::f64::consts::TAU * 16.0).round() as usize) % 16;
            let p = InputTargetV1::pos(
                (w.origin.0 as f64 + w.aim.0 * travel).max(0.0) as u64,
                (w.origin.1 as f64 + w.aim.1 * travel).max(0.0) as u64,
            );
            sim.play_view_effect(
                &format!("angela_wave_{bucket}_{}", elapsed / 3),
                id,
                &p,
                0,
                0,
                0,
            );
        }
    }
    s.waves
        .retain(|w| sim.tick().saturating_sub(w.started) + 1 < WAVE_TICKS);
}
fn choose_target(sim: &StableSim<'_>, id: usize, p: Pos, enemy: bool) -> Option<usize> {
    let team = sim.get_entity(id)?.team();
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| {
            e.is_champion()
                && e.is_alive()
                && e.is_targetable()
                && (e.team() != team) == enemy
                && distance_sq(p, e.pos()) <= S1_RANGE * S1_RANGE
                && (enemy || e.hp().0 < e.hp().1)
        })
        .min_by(|a, b| distance_sq(p, a.pos()).total_cmp(&distance_sq(p, b.pos())))
        .map(|e| e.id())
}
fn tick_runtime(sim: &mut StableSim<'_>, id: usize) {
    let (k, mut s) = take(sim, id);
    let now = sim.tick();
    if s.updated == Some(now) {
        put(k, s);
        return;
    }
    s.updated = Some(now);
    sync_tuning(sim, id, &mut s);
    let alive = sim.get_entity(id).is_some_and(|e| e.is_alive());
    if !alive {
        end_string(sim, id, &mut s);
        end_ride(sim, id, &mut s);
        s.waves.clear();
        s.release = None;
        put(k, s);
        return;
    }
    if let Some(r) = s.ride.clone() {
        let host_ok = sim
            .get_entity(r.host)
            .is_some_and(|e| e.is_alive() && e.is_targetable());
        if let Some(p) = pos_of(sim, r.host) {
            if let Some(r) = s.ride.as_mut() {
                r.last = p;
            }
        }
        if now >= r.until || !host_ok {
            end_ride(sim, id, &mut s);
        } else {
            sim.entity_banish(id, id, BANISH_TICKS, "", "");
            if s.recast_reset {
                sim.entity_reduce_cooldown(id, SkillSlotV1::Ult, usize::MAX);
                s.recast_reset = false;
            }
            if can_render(sim) && now % 6 == 0 {
                sim.play_view_effect(
                    "angela_attached_doll",
                    id,
                    &InputTargetV1::target(r.host),
                    0,
                    0,
                    0,
                );
            }
            // Banish blocks normal actions; invoke native skills here from the
            // host origin, keeping Angela's AP/team/damage credit and cooldowns.
            if now >= s.action_until {
                if let Some(p) = origin(sim, id, &s) {
                    let enemy = choose_target(sim, id, p, true);
                    if let Some(t) = enemy.filter(|_| s.s2.ready()) {
                        if cast_string(sim, id, t, &mut s) {
                            crate::audio::play_voice(sim, id, "mlbb_angela_voice_skill01");
                        }
                    } else if s.s1.ready() {
                        if let Some(t) = enemy.or_else(|| choose_target(sim, id, p, false)) {
                            if cast_wave(sim, id, InputTargetV1::target(t), &mut s) {
                                crate::audio::play_voice(sim, id, "mlbb_angela_voice_skill01");
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some((at, p)) = s.release {
        if now >= at {
            sim.entity_set_pos(id, p.0, p.1);
            s.release = None;
        }
    }
    // Early shield exhaustion must clear its visual even though attach lasts longer.
    let shields: Vec<_> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| named(sim, e.id(), SHIELD_BUFF) && e.shield() == 0)
        .map(|e| e.id())
        .collect();
    for target in shields {
        sim.entity_remove_buff(target, SHIELD_BUFF);
    }
    tick_string(sim, id, &mut s);
    tick_waves(sim, id, &mut s);
    put(k, s);
}
/// Called from the existing match hook as a fallback if banish suppresses a
/// champion's passive updates. It does not replace Zhask's hook.
pub(crate) fn tick_all(sim: &mut StableSim<'_>) {
    let keys: Vec<_> = STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .keys()
        .copied()
        .collect();
    let mut ids: Vec<_> = keys
        .into_iter()
        .filter(|k| {
            k.scope == crate::SimScope::of(sim)
                && crate::sim_copy_of(sim, k.entity_id) == Some(k.copy)
        })
        .map(|k| k.entity_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        tick_runtime(sim, id);
    }
}

#[derive(Clone, Copy, Default)]
pub struct AngelaLoversMark;
impl StableEffectType for AngelaLoversMark {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, _: usize, input: InputTargetV1) {
        if input.kind == InputTargetKindV1::Target.code() {
            apply_mark(sim, input.target_id);
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct AngelaLoveWaves;
impl StableEffectType for AngelaLoveWaves {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, input: InputTargetV1) {
        let (k, mut s) = take(sim, id);
        sync_tuning(sim, id, &mut s);
        cast_wave(sim, id, input, &mut s);
        put(k, s);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (0, S1_BASE + percent(stat.magic_power, S1_AP_PCT))
    }
}
#[derive(Clone, Copy, Default)]
pub struct AngelaPuppetString;
impl StableEffectType for AngelaPuppetString {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let (k, mut s) = take(sim, id);
        sync_tuning(sim, id, &mut s);
        cast_string(sim, id, input.target_id, &mut s);
        put(k, s);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            0,
            95 + percent(stat.magic_power, 25) + 140 + percent(stat.magic_power, 35),
        )
    }
}
#[derive(Clone, Copy, Default)]
pub struct AngelaHeartguard;
impl StableEffectType for AngelaHeartguard {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let (k, mut s) = take(sim, id);
        sync_tuning(sim, id, &mut s);
        cast_heartguard(sim, id, input.target_id, &mut s);
        put(k, s);
    }
    fn expected_shield(&self, stat: &StatV1) -> usize {
        HEARTGUARD_SHIELD + percent(stat.magic_power, HEARTGUARD_AP_PCT)
    }
}
#[derive(Clone, Default)]
pub struct AngelaSmartHeart {
    voice: crate::audio::VoiceLifecycle,
}
impl StablePassive for AngelaSmartHeart {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize) {
        let k = crate::entity_key(sim, id);
        put(k, Runtime::new(sim.tick()));
        self.voice
            .on_spawn(sim, id, Some("mlbb_angela_voice_select"));
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, _: usize, id: usize) {
        self.voice.on_update(
            sim,
            id,
            &["mlbb_angela_voice_select"],
            &[
                "mlbb_angela_voice_move01",
                "mlbb_angela_voice_move02",
                "mlbb_angela_voice_move03",
                "mlbb_angela_voice_move04",
                "mlbb_angela_voice_move05",
                "mlbb_angela_voice_move06",
                "mlbb_angela_voice_move07",
                "mlbb_angela_voice_move08",
                "mlbb_angela_voice_move09",
            ],
        );
        tick_runtime(sim, id);
    }
    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        self.voice.on_dead(
            sim,
            player,
            &[
                "mlbb_angela_voice_death01",
                "mlbb_angela_voice_death02",
                "mlbb_angela_voice_death03",
            ],
        );
    }
}
/// Kept as a registered compatibility alias; the selected Smart Heart passive
/// now calls the same runtime, so S2 is never an unreferenced second passive.
#[derive(Clone, Default)]
pub struct AngelaStringTimer;
impl StablePassive for AngelaStringTimer {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, _: usize, id: usize) {
        tick_runtime(sim, id);
    }
}
/// Normal AI must respect native cooldown debt accrued while banished. There
/// is no stable ABI setter for increasing engine cooldowns/consuming charges.
/// The native ledger is authoritative; engine buttons can look ready earlier.
/// Deterministic ally choice. Shield a living allied champion who is fighting,
/// or a wounded ally near a fight. Never choose Angela, enemies or quiet allies
/// back at base; prefer the lowest HP ratio, then nearest host and smallest id.
fn heartguard_host(sim: &StableSim<'_>, id: usize) -> Option<usize> {
    let caster = sim.get_entity(id)?;
    let team = caster.team();
    let from = caster.pos();
    let enemies: Vec<Pos> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| e.team() != team && e.is_champion() && e.is_alive() && e.is_targetable())
        .map(|e| e.pos())
        .collect();
    let mut allies: Vec<(usize, usize, f64)> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| {
            e.id() != id && e.team() == team && e.is_champion() && e.is_alive() && e.is_targetable()
        })
        .filter_map(|e| {
            let (hp, max) = e.hp();
            if max == 0 {
                return None;
            }
            let hp_ratio = hp.saturating_mul(1000) / max;
            let nearest = enemies
                .iter()
                .map(|p| distance_sq(e.pos(), *p))
                .min_by(f64::total_cmp)?;
            let threatened = nearest <= 150_000.0_f64.powi(2)
                || (hp_ratio <= 650 && nearest <= 300_000.0_f64.powi(2));
            let already_protected = named(sim, e.id(), SHIELD_BUFF) && e.shield() > 0;
            (threatened && !already_protected).then_some((
                e.id(),
                hp_ratio,
                distance_sq(from, e.pos()),
            ))
        })
        .collect();
    allies.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.2.total_cmp(&b.2))
            .then_with(|| a.0.cmp(&b.0))
    });
    allies.first().map(|a| a.0)
}
fn heartguard_proposal(sim: &StableSim<'_>, id: usize, state: &Runtime) -> Option<InputV1> {
    if state.ride.is_some() || state.release.is_some() || !state.ult.ready() {
        return None;
    }
    // Check BOTH native debt and the engine's real level lock/cooldown. A
    // direct AI input must not turn the native clock into a cooldown bypass.
    let caster = sim.get_entity(id)?;
    if !caster.is_alive() || !caster.is_targetable() {
        return None;
    }
    let cooldown = caster.cooldown(SkillSlotV1::Ult)?;
    if !cooldown.unlocked || cooldown.charges == 0 {
        return None;
    }
    Some(InputV1::action(
        InputKindV1::Ult,
        InputTargetV1::target(heartguard_host(sim, id)?),
    ))
}
pub struct AngelaAi;
impl StablePlayerAi for AngelaAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(Self)
    }
    fn id(&self) -> String {
        "mlbb_heroes_angela".into()
    }
    fn priority(&self) -> i32 {
        160
    }
    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name.to_ascii_lowercase().contains("angela")
    }
    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        let player = ctx.player_id();
        let (proposal, fallback, id) = {
            let sim = ctx.sim()?;
            let e = sim.get_player(player)?.champion()?;
            let id = e.id();
            let pos = e.pos();
            let key = crate::entity_key_ro(&sim, id);
            let mut s = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&key)
                .cloned()
                .unwrap_or_else(|| Runtime::new(sim.tick()));
            s.elapse(sim.tick());
            sync_tuning(&sim, id, &mut s);
            if s.ride.is_some() || s.release.is_some() {
                return Some(InputV1::move_to(pos.0, pos.1));
            }
            let proposal = heartguard_proposal(&sim, id, &s);
            let mut fallback = base;
            if let Some(i) = base {
                let blocked = (i.kind == InputKindV1::Skill.code() && !s.s1.ready())
                    || (i.kind == InputKindV1::Skill2.code() && !s.s2.ready())
                    // All ult inputs go through explicit ally selection below;
                    // do not pass an AllyChampion self-target into a rejected cast.
                    || i.kind == InputKindV1::Ult.code();
                if blocked {
                    fallback = Some(InputV1::move_to(pos.0, pos.1));
                    if let Some(target) = choose_target(&sim, id, pos, true) {
                        let input = InputTargetV1::target(target);
                        fallback = if s.s1.ready() {
                            Some(InputV1::action(InputKindV1::Skill, input))
                        } else if s.s2.ready() {
                            Some(InputV1::action(InputKindV1::Skill2, input))
                        } else if valid_enemy(&sim, id, target, pos, 88_000.0) {
                            Some(InputV1::action(InputKindV1::Attack, input))
                        } else {
                            fallback
                        };
                    }
                }
            }
            (proposal, fallback, id)
        };
        if let Some(input) = proposal {
            let valid = ctx.is_valid_input(&input);
            // Small diagnostic breadcrumb for the next actual engine playtest.
            // No sim mutation inside the read-only AI boundary.
            if ctx.tick() % 120 == 0 {
                crate::wlog(format!(
                    "Angela Heartguard AI tick={} caster#{} host#{} engine_valid={}",
                    ctx.tick(),
                    id,
                    input.target.target_id,
                    valid
                ));
            }
            if valid {
                return Some(input);
            }
        }
        fallback
    }
}
pub fn register(module: &mut StableMod) {
    module.add_native_effect("mlbb_heroes:angela_lovers_mark", AngelaLoversMark);
    module.add_native_effect("mlbb_heroes:angela_love_waves", AngelaLoveWaves);
    module.add_native_effect("mlbb_heroes:angela_puppet_string", AngelaPuppetString);
    module.add_native_effect("mlbb_heroes:angela_heartguard", AngelaHeartguard);
    module.add_native_passive(
        "mlbb_heroes:angela_smart_heart",
        AngelaSmartHeart::default(),
    );
    module.add_native_passive("mlbb_heroes:angela_string_timer", AngelaStringTimer);
    module.add_player_input_ai(AngelaAi);
}

#[cfg(test)]
#[path = "angela_tests.rs"]
mod tests;
