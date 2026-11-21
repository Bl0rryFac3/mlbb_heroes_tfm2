//! Isolated projection proof. One real champion, one noncombat parked visual.
//! Native lethal interception and renderer placement still require client evidence.
use crate::{EntityKey, SimScope};
use mod_api_stable::*;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};
type Pos = (u64, u64);
const SPIRIT: &str = "mlbb_hanzo_projection";
const PENALTY: &str = "mlbb_hanzo_failed_return";
#[path = "hanzo_ai.rs"]
mod ai;
#[path = "hanzo_bindings.rs"]
mod bindings;
#[path = "hanzo_combat.rs"]
mod combat;
#[path = "hanzo_visual.rs"]
mod visual;
use combat::DashKind;
const RETURN_TICKS: usize = 18;
const MAX_SPIRIT: usize = 900;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exit {
    Recall,
    Exhausted,
    Defeated,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Form {
    #[default]
    Human,
    Spirit {
        since: usize,
        energy: usize,
        carry: usize,
    },
    Drag {
        since: usize,
        from: Pos,
    },
    Returning {
        until: usize,
        to: Pos,
        reason: Exit,
    },
}
impl Form {
    fn projected(self) -> bool {
        matches!(self, Self::Spirit { .. } | Self::Drag { .. })
    }
    fn input_locked(self) -> bool {
        matches!(self, Self::Drag { .. } | Self::Returning { .. })
    }
}
#[derive(Clone, Debug)]
struct Dash {
    direction: (f64, f64),
    left: usize,
    speed: f64,
    kind: DashKind,
    hits: Vec<EntityKey>,
    refunded: bool,
}
#[derive(Clone, Debug, Default)]
struct State {
    form: Form,
    stored_hp: usize,
    body: Pos,
    body_left: bool,
    left: bool,
    previous: Option<Pos>,
    last_legal: Option<Pos>,
    grid: Option<Arc<crate::Grid>>,
    updated: Option<usize>,
    ready: [usize; 3],
    last_cast: [Option<usize>; 4],
    marks: HashMap<EntityKey, (usize, usize)>,
    dash: Option<Dash>,
    pose: Option<&'static str>,
    dead: bool,
    blocked_architecture: bool,
    returns: usize,
    logs: usize,
    visual: visual::Visual,
    skill_ready: [[usize; 2]; 2],
    zones: Vec<combat::Zone>,
    snatch: Option<combat::Snatch>,
    digest: Option<combat::Digest>,
    feast: Option<combat::Feast>,
    feast_logs: usize,
    dash_from: Pos,
    watched: HashMap<EntityKey, bool>,
    next_deaths: usize,
    body_since: usize,
    overlay_next: usize,
    mark_next: usize,
    basic_logs: usize,
    skill_logs: usize,
    overlay_logs: usize,
}
static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static OWNERS: LazyLock<Mutex<HashMap<SimScope, Vec<EntityKey>>>> =
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
fn owned(sim: &StableSim<'_>, k: EntityKey) -> bool {
    SimScope::of(sim) == k.scope && crate::sim_copy_of(sim, k.entity_id) == Some(k.copy)
}
fn viewed(sim: &StableSim<'_>) -> bool {
    sim.sim_origin().is_some_and(|o| {
        matches!(
            SimOriginKindV1::from_code(o.kind),
            Some(
                SimOriginKindV1::ClientMatchView
                    | SimOriginKindV1::ClientSpectate
                    | SimOriginKindV1::ClientReplay
            )
        )
    })
}
fn trace(sim: &StableSim<'_>, id: usize, s: &mut State, msg: &str) {
    if viewed(sim) && s.logs < 96 {
        s.logs += 1;
        crate::wlog(format!(
            "[HANZO proof] tick={} owner={id} {msg}",
            sim.tick()
        ));
    }
}
fn position(sim: &StableSim<'_>, id: usize) -> Option<Pos> {
    sim.get_entity(id).map(|e| e.pos())
}
fn distance(a: Pos, b: Pos) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
}
fn direction(a: Pos, b: Pos) -> (f64, f64) {
    let d = distance(a, b).max(1.);
    ((b.0 as f64 - a.0 as f64) / d, (b.1 as f64 - a.1 as f64) / d)
}
fn endpoint(a: Pos, d: (f64, f64), n: f64) -> Pos {
    (
        (a.0 as f64 + d.0 * n).max(0.) as u64,
        (a.1 as f64 + d.1 * n).max(0.) as u64,
    )
}
pub(crate) fn grid(sim: &StableSim<'_>) -> Arc<crate::Grid> {
    // Select once per projection using actual map-hook cells and live champion positions.
    let live: Vec<_> = (0..sim.champion_count())
        .filter_map(|i| sim.get_entity(sim.champion_id_at(i)))
        .map(|e| e.pos())
        .collect();
    let cache = crate::MAP_GRIDS.lock().unwrap_or_else(|e| e.into_inner());
    let mut g = crate::MODES
        .iter()
        .map(|m| {
            cache
                .get(&m.code())
                .cloned()
                .unwrap_or_else(|| crate::Grid::builtin(*m))
        })
        .min_by_key(|g| {
            live.iter()
                .filter(|p| g.is_wall(p.0 as f64, p.1 as f64))
                .count()
        })
        .unwrap();
    g.bounds = None;
    Arc::new(g)
}
pub(crate) fn standable(g: &crate::Grid, p: Pos) -> bool {
    [
        (0., 0.),
        (8000., 0.),
        (-8000., 0.),
        (0., 8000.),
        (0., -8000.),
    ]
    .iter()
    .all(|(x, y)| !g.is_wall(p.0 as f64 + x, p.1 as f64 + y))
}
pub(crate) fn landing(g: &crate::Grid, p: Pos, fallback: Pos) -> Pos {
    if standable(g, p) {
        return p;
    }
    for ring in 1..=16 {
        for ray in 0..16 {
            let a = ray as f64 * std::f64::consts::TAU / 16.;
            let q = endpoint(p, (a.cos(), a.sin()), ring as f64 * 16000.);
            if standable(g, q) {
                return q;
            }
        }
    }
    fallback // known human/last legal location, never guessed lane coordinates
}
pub(crate) fn legal_step(g: &crate::Grid, p: Pos, d: (f64, f64), n: f64, phase: bool) -> Pos {
    let mut to = p;
    let count = (n / 1000.).ceil().max(1.) as usize;
    for i in 1..=count {
        let q = endpoint(p, d, n * i as f64 / count as f64);
        if !g.on_map(q.0 as f64, q.1 as f64) || (!phase && !standable(g, q)) {
            break;
        }
        to = q;
    }
    to
}
fn resolved_foe(sim: &StableSim<'_>, id: usize, target: usize, structures: bool) -> bool {
    let (Some(a), Some(b)) = (sim.get_entity(id), sim.get_entity(target)) else {
        return false;
    };
    b.is_alive()
        && b.is_targetable()
        && a.team() != b.team()
        && (structures || !matches!(b.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus)))
}
fn edge_distance(sim: &StableSim<'_>, id: usize, target: usize) -> f64 {
    let (Some(a), Some(b)) = (sim.get_entity(id), sim.get_entity(target)) else {
        return f64::INFINITY;
    };
    (distance(a.pos(), b.pos()) - a.radius().max(10000) as f64 - b.radius().max(10000) as f64)
        .max(0.)
}
fn foe(sim: &StableSim<'_>, id: usize, target: usize, structures: bool) -> bool {
    let (Some(a), Some(b)) = (sim.get_entity(id), sim.get_entity(target)) else {
        return false;
    };
    b.is_alive()
        && b.is_targetable()
        && a.team() != b.team()
        && (structures || !matches!(b.kind(), Some(EntityKindV1::Tower | EntityKindV1::Nexus)))
        && sim.is_visible(a.team(), target)
}
pub(crate) fn blocked(sim: &StableSim<'_>, id: usize, moving: bool) -> bool {
    sim.get_entity(id).is_none_or(|e| {
        (0..e.cc_count()).filter_map(|i| e.cc_at(i)).any(|c| {
            matches!(
                CcKindV1::from_code(c.kind),
                Some(
                    CcKindV1::Stun
                        | CcKindV1::Airborne
                        | CcKindV1::ForceMove
                        | CcKindV1::Fear
                        | CcKindV1::Charm
                        | CcKindV1::Taunt
                        | CcKindV1::BlockSkill
                )
            ) || (moving
                && matches!(
                    CcKindV1::from_code(c.kind),
                    Some(CcKindV1::Bind | CcKindV1::BlockMoveSkill)
                ))
        })
    })
}
fn bonus(sim: &StableSim<'_>, id: usize) -> usize {
    sim.get_entity(id).map_or(0, |e| {
        e.stat()
            .attack
            .saturating_sub(100 + 6 * e.level().saturating_sub(1))
    })
}
fn mark(s: &mut State, key: EntityKey, now: usize) {
    if s.marks.len() >= 64 && !s.marks.contains_key(&key) {
        s.marks.retain(|_, (_, t)| *t > now);
        if s.marks.len() >= 64 {
            return;
        }
    }
    let n = s
        .marks
        .get(&key)
        .filter(|(_, t)| *t > now)
        .map_or(0, |(n, _)| *n);
    s.marks.insert(key, ((n + 1).min(5), now + 360));
}
fn pose(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    visual::pose(sim, id, s);
}
fn clear(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    sim.entity_remove_buff(id, SPIRIT);
    if let Some(old) = s.pose.take() {
        sim.entity_remove_buff(id, old);
    }
    s.dash = None;
    s.snatch = None;
    s.zones.clear();
    s.digest = None;
    s.feast = None;
    s.watched.clear();
}
fn enter(sim: &mut StableSim<'_>, id: usize, s: &mut State) -> bool {
    if s.form != Form::Human || s.blocked_architecture || sim.tick() < s.ready[2] {
        return false;
    }
    let Some(e) = sim.get_entity(id) else {
        return false;
    };
    if e.level() < 5 || !e.is_alive() {
        return false;
    }
    let hp = e.hp().0;
    let p = e.pos();
    let g = grid(sim);
    if !standable(&g, p) {
        trace(
            sim,
            id,
            s,
            "entry deferred: body is not on confirmed legal ground",
        );
        return false;
    }
    let mut b = BuffV1::timed(SPIRIT, 1200);
    b.undying = true;
    b.ignore_wall = true;
    b.hp_mult = -30;
    b.move_speed_mult = 35;
    b.attack_speed_mult = 25;
    sim.add_buff(id, &b);
    let max = sim.get_entity(id).map_or(1, |e| e.hp().1).max(1);
    if !sim.entity_set_hp(id, hp.min(max).max(1)) {
        sim.entity_remove_buff(id, SPIRIT);
        trace(sim, id, s, "entry rejected: silent HP setter unavailable");
        return false;
    }
    s.body_since = sim.tick();
    s.watched.clear();
    s.next_deaths = sim.tick();
    s.stored_hp = hp;
    s.body = p;
    s.body_left = s.left;
    s.last_legal = Some(p);
    s.grid = Some(g);
    s.form = Form::Spirit {
        since: sim.tick(),
        energy: 1000,
        carry: 0,
    };
    s.dash = None;
    combat::observe_deaths(sim, id, s);
    visual::fx(sim, id, p, "projection_flare", 18);
    pose(sim, id, s);
    trace(
        sim,
        id,
        s,
        &format!("ENTER body={p:?} stored_hp={hp} spirit_max={max} undying=1 ignore_wall=1"),
    );
    true
}
fn finish(sim: &mut StableSim<'_>, id: usize, s: &mut State, to: Pos, reason: Exit) {
    // Move to validated ground while phasing still exists. Restore before removing undying.
    if !sim.entity_set_pos(id, to.0, to.1) {
        trace(sim, id, s, "ERROR position setter failed during return");
    }
    sim.entity_remove_buff(id, SPIRIT);
    let max = sim.get_entity(id).map_or(1, |e| e.hp().1).max(1);
    sim.entity_set_hp(id, s.stored_hp.min(max).max(1));
    sim.heal(id, id, 90 + bonus(sim, id) / 2);
    if reason == Exit::Defeated {
        let mut b = BuffV1::timed(PENALTY, 360);
        b.move_speed_mult = -50;
        sim.entity_remove_buff(id, PENALTY);
        sim.add_buff(id, &b);
    }
    s.form = Form::Human;
    s.ready[2] = sim.tick() + if reason == Exit::Defeated { 3120 } else { 1560 };
    visual::action(s, "body_merge", sim.tick(), 18);
    visual::fx(sim, id, to, "return_merge", 18);
    visual::fx(sim, id, to, "heal_return", 24);
    s.returns += 1;
    s.dash = None;
    s.updated = Some(sim.tick());
    pose(sim, id, s);
    let hp = sim.get_entity(id).map(|e| e.hp());
    trace(
        sim,
        id,
        s,
        &format!(
            "RETURN_COMPLETE reason={reason:?} to={to:?} hp={hp:?} stored={} undying=0 returns={}",
            s.stored_hp, s.returns
        ),
    );
}
fn begin_return(sim: &mut StableSim<'_>, id: usize, s: &mut State, reason: Exit) {
    if !s.form.projected() {
        return;
    }
    let p = position(sim, id).unwrap_or(s.body);
    let Some(g) = s.grid.as_ref() else {
        return;
    };
    let wanted = if reason == Exit::Exhausted { p } else { s.body };
    let to = landing(g, wanted, s.last_legal.unwrap_or(s.body));
    s.dash = None;
    s.snatch = None;
    s.zones.retain(|z| !z.spirit);
    visual::fx(
        sim,
        id,
        p,
        &format!(
            "{}{}",
            if reason == Exit::Defeated {
                "spirit_break"
            } else {
                "spirit_return"
            },
            if s.left { "_left" } else { "" }
        ),
        18,
    );
    if reason == Exit::Defeated {
        visual::fx(sim, id, p, "spirit_failure", 18);
    }
    let banished = sim.entity_banish(id, id, RETURN_TICKS, "", "");
    trace(
        sim,
        id,
        s,
        &format!("RETURN_BEGIN reason={reason:?} from={p:?} to={to:?} banish={banished}"),
    );
    if banished {
        s.form = Form::Returning {
            until: sim.tick() + RETURN_TICKS,
            to,
            reason,
        };
    } else {
        finish(sim, id, s, to, reason);
    }
}
fn update(sim: &mut StableSim<'_>, k: EntityKey) {
    if !owned(sim, k) {
        return;
    }
    let Some(mut s) = take(k) else {
        return;
    };
    let id = k.entity_id;
    let now = sim.tick();
    if s.updated == Some(now) {
        put(k, s);
        return;
    }
    s.updated = Some(now);
    if sim.is_end() {
        clear(sim, id, &mut s);
        forget_snapshot(k);
        return;
    }
    let Some(e) = sim.get_entity(id) else {
        forget_snapshot(k);
        return;
    };
    if !e.is_alive() {
        if s.form.projected() || matches!(s.form, Form::Returning { .. }) {
            s.blocked_architecture = true;
            trace(sim,id,&mut s,"NATIVE_PROOF_FAILED actual champion death during projection; no resurrection attempted");
        }
        clear(sim, id, &mut s);
        s.dead = true;
        s.form = Form::Human;
        forget_snapshot(k);
        put(k, s);
        return;
    }
    let hp = e.hp().0;
    let p = e.pos();
    if let Some(old) = s.previous {
        if !visual::locked(&s, now) && p.0.abs_diff(old.0) > 100 {
            s.left = p.0 < old.0;
        }
    }
    s.visual.moving = s.previous.is_some_and(|old| distance(old, p) > 100.);
    s.previous = Some(p);
    combat::observe_deaths(sim, id, &mut s);
    s.marks.retain(|_, (_, until)| *until > now);
    if let Some(g) = s.grid.as_ref() {
        if standable(g, p) {
            s.last_legal = Some(p);
        }
    }
    if (s.form.projected() || matches!(s.form, Form::Returning { .. })) && hp <= 1 {
        if let Form::Returning { to, .. } = s.form {
            finish(sim, id, &mut s, to, Exit::Defeated);
        } else {
            begin_return(sim, id, &mut s, Exit::Defeated);
        }
    }
    match s.form {
        Form::Spirit {
            since,
            mut energy,
            mut carry,
        } => {
            carry += 100;
            energy = energy.saturating_sub(carry / 60);
            carry %= 60;
            if energy == 0 || now.saturating_sub(since) >= MAX_SPIRIT {
                s.form = Form::Drag {
                    since: now,
                    from: s.body,
                };
                s.dash = None;
                trace(
                    sim,
                    id,
                    &mut s,
                    "BODY_CATCHUP_BEGIN energy exhausted or duration cap",
                );
            } else {
                s.form = Form::Spirit {
                    since,
                    energy,
                    carry,
                };
            }
        }
        Form::Drag { since, from } => {
            let n = now.saturating_sub(since).min(60) as u64;
            s.body = (
                (from.0 as i128 + (p.0 as i128 - from.0 as i128) * n as i128 / 60).max(0) as u64,
                (from.1 as i128 + (p.1 as i128 - from.1 as i128) * n as i128 / 60).max(0) as u64,
            );
            if n >= 60 {
                begin_return(sim, id, &mut s, Exit::Exhausted);
            }
        }
        Form::Returning { until, to, reason } => {
            if now >= until {
                finish(sim, id, &mut s, to, reason);
            }
        }
        Form::Human => {}
    }
    combat::tick(sim, id, &mut s);
    pose(sim, id, &mut s);
    visual::overlays(sim, id, &mut s);
    snapshot(sim, k, &s);
    put(k, s);
}
pub(crate) fn tick_all(sim: &mut StableSim<'_>) {
    observe(sim);
    let scope = SimScope::of(sim);
    let keys = OWNERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&scope)
        .cloned()
        .unwrap_or_default();
    for k in keys {
        update(sim, k);
    }
    if sim.is_end() {
        OWNERS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&scope);
    }
}

#[derive(Clone, Copy)]
enum Op {
    Basic,
    Feast,
    Dash,
    Ult,
}
impl StableEffectType for Op {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, input: InputTargetV1) {
        let k = crate::entity_key(sim, id);
        let Some(mut s) = take(k) else {
            return;
        };
        let now = sim.tick();
        let index = match self {
            Self::Basic => 0,
            Self::Feast => 1,
            Self::Dash => 2,
            Self::Ult => 3,
        };
        if !sim.get_entity(id).is_some_and(|e| e.is_alive())
            || s.form.input_locked()
            || s.last_cast[index] == Some(now)
            || (!matches!(self, Self::Basic) && blocked(sim, id, matches!(self, Self::Dash)))
        {
            put(k, s);
            return;
        }
        s.last_cast[index] = Some(now);
        match self {
            Self::Ult => {
                if s.form.projected() {
                    begin_return(sim, id, &mut s, Exit::Recall);
                } else {
                    enter(sim, id, &mut s);
                }
            }
            Self::Basic => combat::basic(sim, id, &mut s, input),
            Self::Feast => combat::cast_feast(sim, id, &mut s, input),
            Self::Dash => combat::cast_mist(sim, id, &mut s, input),
        }

        pose(sim, id, &mut s);
        snapshot(sim, k, &s);
        put(k, s);
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        match self {
            Self::Basic => (stat.attack, 0),
            Self::Feast => (200 + stat.attack * 115 / 100, 0),
            Self::Dash => (110 + stat.attack * 55 / 100, 0),
            Self::Ult => (0, 0),
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
        let k = crate::entity_key(sim, id);
        let mut blocked_architecture = false;
        if let Some(old) = self.owner.take() {
            if let Some(mut s) = take(old) {
                blocked_architecture = old.scope == k.scope && s.blocked_architecture;
                if owned(sim, old) {
                    clear(sim, old.entity_id, &mut s);
                }
            }
            if let Some(v) = OWNERS
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut(&old.scope)
            {
                v.retain(|x| *x != old);
            }
            forget_snapshot(old);
        }
        let mut s = State {
            blocked_architecture,
            ..Default::default()
        };
        sim.entity_remove_buff(id, SPIRIT);
        sim.entity_remove_buff(id, PENALTY);
        pose(sim, id, &mut s);
        trace(
            sim,
            id,
            &mut s,
            "SPAWN human; static master art; full skill/owner animation candidate1.1.156",
        );
        let mut owners = OWNERS.lock().unwrap_or_else(|e| e.into_inner());
        let v = owners.entry(k.scope).or_default();
        if !v.contains(&k) {
            v.push(k);
        }
        drop(owners);
        self.owner = Some(k);
        snapshot(sim, k, &s);
        put(k, s);
    }
    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, id: usize) {
        let k = crate::entity_key(sim, id);
        if self.owner != Some(k) {
            self.on_spawn(sim, player, id);
        }
        update(sim, k);
    }
    fn on_damaged(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize, _: usize, _: usize) {
        let k = crate::entity_key_ro(sim, id);
        if let Some(mut s) = take(k) {
            // Only inspect actual post-hit HP. Do not mistake pre-mitigation damage for lethal.
            if s.form.projected()
                && sim
                    .get_entity(id)
                    .is_some_and(|e| e.is_alive() && e.hp().0 <= 1)
            {
                begin_return(sim, id, &mut s, Exit::Defeated);
            }
            if !visual::locked(&s, sim.tick()) && s.visual.action.is_none_or(|(_, until)| until <= sim.tick()) {
                let motion = if s.form.projected() {
                    "spirit_hit"
                } else {
                    "human_hit"
                };
                visual::action(&mut s, motion, sim.tick(), 12);
            }
            snapshot(sim, k, &s);
            put(k, s);
        }
    }
    fn on_kill(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize, victim: usize) {
        let k = crate::entity_key_ro(sim, id);
        if let Some(mut s) = take(k) {
            if let Some(f) = s.feast.as_mut().filter(|f| f.target.entity_id == victim) {
                f.credited = true;
            }
            combat::death_energy(sim, id, &mut s, victim);
            put(k, s);
        }
    }
    fn on_unit_kill(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize, victim: usize) {
        self.on_kill(sim, 0, id, victim);
    }
    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        if let Some(k) = self.owner {
            if let Some(mut s) = take(k) {
                if s.form != Form::Human {
                    s.blocked_architecture = true;
                    trace(sim,k.entity_id,&mut s,"NATIVE_PROOF_FAILED on_dead during projection; architecture disabled for this actor");
                }
                if !s.dead {
                    visual::death(sim, k.entity_id, &mut s);
                }
                clear(sim, k.entity_id, &mut s);
                s.dead = true;
                s.form = Form::Human;
                s.marks.clear();
                forget_snapshot(k);
                put(k, s);
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
        "mlbb_hanzo_ai".into()
    }
    fn priority(&self) -> i32 {
        175
    }
    fn matches(&self, i: &StableAiInit) -> bool {
        i.champion_name == "mlbb_hanzo"
    }
    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        let player = ctx.player_id();
        let candidates = (|| {
            let sim = ctx.sim()?;
            let e = sim.get_player(player)?.champion()?;
            let k = crate::entity_key_ro(&sim, e.id());
            let state = STATES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&k)?
                .clone();
            Some(ai::choices(&sim, e.id(), &state, base))
        })();
        candidates.and_then(|items| items.into_iter().find(|i| ctx.is_valid_input(i)))
    }
}

#[derive(Clone)]
struct Row {
    key: EntityKey,
    form: Form,
    stored_hp: usize,
    ready: usize,
    now: usize,
}
static ROWS: LazyLock<Mutex<Vec<Row>>> = LazyLock::new(|| Mutex::new(Vec::new()));
fn forget_snapshot(k: EntityKey) {
    ROWS.lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|r| r.key != k);
}
fn observe(sim: &StableSim<'_>) {
    if viewed(sim) {
        let scope = SimScope::of(sim);
        ROWS.lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|r| r.key.scope == scope);
    }
}
fn snapshot(sim: &StableSim<'_>, key: EntityKey, s: &State) {
    if !viewed(sim) {
        return;
    }
    let mut rows = ROWS.lock().unwrap_or_else(|e| e.into_inner());
    rows.retain(|r| r.key.scope == key.scope && r.key.entity_id != key.entity_id);
    if !s.dead {
        rows.push(Row {
            key,
            form: s.form,
            stored_hp: s.stored_hp,
            ready: s.ready[2],
            now: sim.tick(),
        });
    }
}
fn visible(ctx: &StableClient<'_>) -> bool {
    ctx.scene_kind() == Some(SceneKindV1::InGame)
        && matches!(
            ctx.client_scene_kind(),
            Some(
                ClientSceneKindV1::Match
                    | ClientSceneKindV1::InGame
                    | ClientSceneKindV1::SetFeedback
                    | ClientSceneKindV1::Tutorial5v5
                    | ClientSceneKindV1::TutorialMorgad
                    | ClientSceneKindV1::TutorialSerpen
            )
        )
}
pub(crate) struct Extension;
impl StableExtension for Extension {
    fn pre_update(&self, c: &mut StableClient<'_>, dt: u64) {
        crate::granger::Extension.pre_update(c, dt);
        if !visible(c) {
            ROWS.lock().unwrap_or_else(|e| e.into_inner()).clear();
        }
    }
    fn post_update(&self, c: &mut StableClient<'_>, dt: u64) {
        crate::granger::Extension.post_update(c, dt);
        crate::popol_menu::update(c, dt);
        crate::draft_preview::update(c, dt);
    }
    fn on_end(&self) {
        crate::granger::Extension.on_end();
        crate::atlas::clear_state();
        crate::yu_zhong::clear_state();
        crate::popol_menu::clear();
        crate::draft_preview::clear();
        ROWS.lock().unwrap_or_else(|e| e.into_inner()).clear();
        STATES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        OWNERS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    fn post_render(&self, c: &mut StableClient<'_>) {
        crate::granger::Extension.post_render(c);
        crate::popol_menu::render(c);
        crate::draft_preview::render(c);
        if !c.can_draw() || !visible(c) {
            return;
        }
        let rows = ROWS.lock().unwrap_or_else(|e| e.into_inner()).clone();
        for (i, row) in rows.iter().take(10).enumerate() {
            if let Some((_, h)) = c.draw_map_size("UI") {
                let y = (h - 80. - i as f32 * 46.).max(0.);
                let label = match row.form {
                    Form::Human => format!(
                        "Hanzo #{}  Return {:.1}s",
                        row.key.entity_id,
                        row.ready.saturating_sub(row.now) as f32 / 60.
                    ),
                    Form::Spirit { energy, .. } => format!(
                        "Hanzo #{}  Hanekage {}%  Body {} HP",
                        row.key.entity_id,
                        energy / 10,
                        row.stored_hp
                    ),
                    Form::Drag { .. } => format!("Hanzo #{}  Body returning", row.key.entity_id),
                    Form::Returning { .. } => format!("Hanzo #{}  Returning", row.key.entity_id),
                };
                c.draw_rect("UI", 20., y, 340., 38., 90, 4., 0x201225d0);
                c.draw_text(
                    "UI",
                    &label,
                    "asset/base/font/set/regular",
                    (28., y + 4., 324., 28.),
                    91,
                    13.,
                    0xf5c4edff,
                    TextAlignXV1::Left,
                    TextAlignYV1::Center,
                );
            }
        }
    }
}
pub fn register(m: &mut StableMod) {
    for (name, op) in [
        ("basic", Op::Basic),
        ("feast", Op::Feast),
        ("dash", Op::Dash),
        ("ult", Op::Ult),
    ] {
        m.add_native_effect(&format!("mlbb_heroes:hanzo_{name}"), op);
    }
    m.add_native_passive("mlbb_heroes:hanzo", Passive::default());
    m.add_player_input_ai(Ai);
}

#[cfg(test)]
#[path = "hanzo_tests.rs"]
pub(crate) mod tests;
