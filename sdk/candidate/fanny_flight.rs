use super::*;
use std::collections::VecDeque;
use std::sync::Arc;

const HOOK_SPEED: f64 = 15_000.0;
const REDIRECT_IMPULSE: f64 = 8_200.0;
const MAX_FLIGHT_TICKS: usize = 120;
const MIN_ANCHOR_SPACING: f64 = 14_000.0;
pub(super) const TOWER_ATTACK_RANGE: f64 = 75_000.0;
const BODY_CLEARANCE: f64 = 4_000.0;
const DRAW_EVERY: usize = 4;
const ROUTE_CAST_GAP: usize = 7;
const ROUTE_DEPTH: usize = 3;
const ROUTE_BRANCHES: usize = 4;
const ROUTE_TICK_COST: f64 = 900.0;
const TRAVEL_DETOUR: f64 = 72_000.0;

type Point = (f64, f64);

#[derive(Clone, Copy, Debug)]
pub(super) struct Anchor {
    hit: WallHit,
    tower: Option<usize>,
}

#[derive(Clone, Copy)]
struct Shot {
    anchor: Anchor,
    start: Point,
    fired: usize,
    latch: usize,
}

#[derive(Clone)]
pub(super) struct Flight {
    grid: Arc<Grid>,
    start: usize,
    casts: usize,
    pending: VecDeque<Shot>,
    anchors: [Option<Anchor>; 2],
    velocity: Point,
    remaining: f64,
    target: Option<usize>,
    travel_goal: Option<Point>,
}

static FLIGHTS: LazyLock<Mutex<HashMap<EntityKey, Flight>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn read(key: EntityKey) -> Option<Flight> {
    FLIGHTS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&key)
        .cloned()
}

fn save(key: EntityKey, flight: Flight) {
    FLIGHTS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(key, flight);
}

pub(super) fn clear(sim: &mut StableSim<'_>, id: usize, key: EntityKey) {
    FLIGHTS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&key);
    sim.entity_remove_buff(id, FLYING);
}

fn unit_vector(from: Point, to: Point) -> Option<Point> {
    let length = distance(from, to);
    (length > 1.0).then(|| ((to.0 - from.0) / length, (to.1 - from.1) / length))
}

fn hook_travel(from: Point, anchor: Anchor) -> usize {
    (distance(from, anchor.hit.anchor()) / HOOK_SPEED)
        .ceil()
        .max(2.0) as usize
}

fn clear_step(grid: &Grid, here: Point, there: Point) -> bool {
    // Probe the body on both sides of a segment as well as its center.
    let Some(dir) = unit_vector(here, there) else {
        return false;
    };
    let side = (-dir.1, dir.0);
    let length = distance(here, there);
    let samples = (length / 1_000.0).ceil().max(1.0) as usize;
    (0..=samples).all(|index| {
        let center = point_on_ray(here, dir, length * index as f64 / samples as f64);
        [-BODY_CLEARANCE, 0.0, BODY_CLEARANCE]
            .into_iter()
            .all(|offset| {
                let (x, y) = point_on_ray(center, side, offset);
                !grid.solid(x, y)
            })
    })
}

impl Flight {
    pub(super) fn target(&self) -> Option<usize> {
        self.target
    }

    pub(super) fn travel_goal(&self) -> Option<Point> {
        self.travel_goal
    }

    fn new(grid: Grid, now: usize) -> Self {
        Self {
            grid: Arc::new(grid),
            start: now,
            casts: 0,
            pending: VecDeque::new(),
            anchors: [None; 2],
            velocity: (0.0, 0.0),
            remaining: 0.0,
            target: None,
            travel_goal: None,
        }
    }

    fn can_fire(&self, now: usize) -> bool {
        self.casts < MAX_CABLE_CHARGES
            && self.pending.len() < 2
            && now.saturating_sub(self.start) < MAX_FLIGHT_TICKS
    }

    fn distinct(&self, anchor: Anchor) -> bool {
        self.pending
            .back()
            .map(|shot| shot.anchor)
            .or(self.anchors[1])
            .is_none_or(|last| {
                distance(last.hit.anchor(), anchor.hit.anchor()) >= MIN_ANCHOR_SPACING
            })
    }

    fn next_latch(&self, start: Point, anchor: Anchor, now: usize) -> usize {
        (now + hook_travel(start, anchor)).max(self.pending.back().map_or(0, |shot| shot.latch + 1))
    }

    fn fire(&mut self, start: Point, anchor: Anchor, now: usize) {
        let latch = self.next_latch(start, anchor, now);
        self.pending.push_back(Shot {
            anchor,
            start,
            fired: now,
            latch,
        });
        self.casts += 1;
    }

    fn latch(&mut self, here: Point, anchor: Anchor) -> bool {
        let fallback = unit_vector(here, anchor.hit.pos());
        let pull = unit_vector(here, anchor.hit.anchor());
        let direction = if self.velocity != (0.0, 0.0) {
            pull.and_then(|pull| {
                unit_vector(
                    (0.0, 0.0),
                    (
                        self.velocity.0 + pull.0 * REDIRECT_IMPULSE,
                        self.velocity.1 + pull.1 * REDIRECT_IMPULSE,
                    ),
                )
            })
        } else {
            fallback
        };

        // Use momentum plus the new pull. If the resulting heading immediately
        // points into terrain, try the clear swing side picked by the ray cast.
        if let Some(dir) = direction
            .into_iter()
            .chain(fallback)
            .find(|&dir| clear_step(&self.grid, here, point_on_ray(here, dir, CABLE_SPEED)))
        {
            self.anchors = [self.anchors[1], Some(anchor)];
            self.velocity = (dir.0 * CABLE_SPEED, dir.1 * CABLE_SPEED);
            self.remaining = distance(here, anchor.hit.pos()).max(CABLE_SPEED);
            true
        } else {
            false
        }
    }

    fn step(&mut self, here: Point) -> Option<Point> {
        if self.remaining <= 1.0 {
            return None;
        }
        let ratio = (self.remaining / CABLE_SPEED).min(1.0);
        let there = point_on_ray(here, self.velocity, ratio);
        if !clear_step(&self.grid, here, there) {
            self.remaining = 0.0;
            return None;
        }
        self.remaining = (self.remaining - CABLE_SPEED).max(0.0);
        Some(there)
    }

    fn predict(&self, mut here: Point, now: usize, until: usize) -> Option<(Point, Self)> {
        let mut flight = self.clone();
        for tick in (now + 1)..until {
            while flight
                .pending
                .front()
                .is_some_and(|shot| shot.latch <= tick)
            {
                let shot = flight.pending.pop_front().unwrap();
                flight.latch(here, shot.anchor);
            }
            if let Some(there) = flight.step(here) {
                here = there;
            } else if flight.pending.is_empty() || flight.velocity != (0.0, 0.0) {
                return None;
            }
        }
        Some((here, flight))
    }
}

fn ray_tower(tower: &UnitSnap, here: Point, dir: Point) -> Option<Anchor> {
    let to_tower = (tower.pos.0 - here.0, tower.pos.1 - here.1);
    let along = to_tower.0 * dir.0 + to_tower.1 * dir.1;
    let across = (to_tower.0 * dir.1 - to_tower.1 * dir.0).abs();
    if across >= tower.radius {
        return None;
    }
    let contact = along - (tower.radius.powi(2) - across.powi(2)).sqrt();
    let landing_dist = contact - WALL_STANDOFF;
    if contact > CABLE_RANGE || landing_dist < WALL_MIN {
        return None;
    }
    let landing = point_on_ray(here, dir, landing_dist);
    let anchor = point_on_ray(here, dir, contact);
    Some(Anchor {
        hit: WallHit {
            x: landing.0,
            y: landing.1,
            distance: landing_dist,
            anchor_x: anchor.0,
            anchor_y: anchor.1,
        },
        tower: Some(tower.id),
    })
}

pub(super) fn aimed_anchor(
    grid: &Grid,
    all: &[UnitSnap],
    here: Point,
    aim: Point,
) -> Option<Anchor> {
    let dir = unit_vector(here, aim)?;
    let wall = wall_hit_on_ray(grid, here, dir, CABLE_RANGE);
    closest_anchor(grid, all, here, dir, wall)
}

fn closest_anchor(
    grid: &Grid,
    all: &[UnitSnap],
    here: Point,
    dir: Point,
    wall: Option<WallHit>,
) -> Option<Anchor> {
    let wall_dist = wall.map_or(CABLE_RANGE + 1.0, |hit| distance(here, hit.anchor()));
    let tower = all
        .iter()
        .filter(|unit| unit.tower && unit.alive)
        .filter_map(|tower| ray_tower(tower, here, dir))
        .filter(|anchor| distance(here, anchor.hit.anchor()) < wall_dist)
        .filter(|anchor| path_is_clear(grid, here, anchor.hit.anchor()))
        .min_by(|a, b| a.hit.distance.total_cmp(&b.hit.distance));
    tower.or_else(|| wall.map(|hit| Anchor { hit, tower: None }))
}

fn unsheltered_towers<'a>(
    all: &'a [UnitSnap],
    caster: &'a UnitSnap,
) -> impl Iterator<Item = &'a UnitSnap> {
    all.iter().filter(move |tower| {
        tower.tower
            && tower.alive
            && tower.team != caster.team
            && !tower_has_allied_tankers(all, caster, tower)
    })
}

fn enters_tower_range(tower: &UnitSnap, from: Point, to: Point) -> bool {
    let start = distance(from, tower.pos);
    let closest = segment_distance(tower.pos, from, to);
    let outward =
        start < TOWER_ATTACK_RANGE && closest + 1.0 >= start && distance(to, tower.pos) > start;
    closest < TOWER_ATTACK_RANGE && !outward
}

#[derive(Clone, Copy)]
pub(super) enum CableAim<'a> {
    Engage(&'a UnitSnap),
    Travel(Point),
}

fn route_anchors<'a>(
    grid: &'a Grid,
    all: &'a [UnitSnap],
    here: Point,
) -> impl Iterator<Item = Anchor> + 'a {
    let wall_anchors = wall_hits(grid, here, CABLE_RANGE)
        .into_iter()
        .filter_map(move |wall| {
            let dir = unit_vector(here, wall.anchor())?;
            closest_anchor(grid, all, here, dir, Some(wall))
        });
    let tower_anchors = all
        .iter()
        .filter(|unit| unit.tower && unit.alive)
        .filter(move |tower| distance(here, tower.pos) <= CABLE_RANGE + tower.radius)
        .filter_map(move |tower| aimed_anchor(grid, all, here, tower.pos));
    wall_anchors.chain(tower_anchors)
}

#[derive(Clone)]
struct RouteState {
    flight: Flight,
    pos: Point,
    tick: usize,
    crossed_target: bool,
}

struct Route<'a> {
    all: &'a [UnitSnap],
    caster: &'a UnitSnap,
    aim: CableAim<'a>,
    start: usize,
    goal: Point,
    threats: Vec<(&'a UnitSnap, f64)>,
    towers: Vec<&'a UnitSnap>,
}

struct RouteOption {
    first: Anchor,
    score: f64,
    recast: Option<RouteState>,
}

impl Route<'_> {
    fn safe_step(&self, from: Point, to: Point) -> bool {
        if self
            .towers
            .iter()
            .any(|tower| enters_tower_range(tower, from, to))
        {
            return false;
        }
        if let CableAim::Travel(_) = self.aim {
            let start = self.caster.pos;
            if segment_distance(to, start, self.goal) > TRAVEL_DETOUR
                || self.threats.iter().any(|(enemy, initial)| {
                    segment_distance(enemy.pos, from, to) < (initial - 3_000.0).min(55_000.0)
                })
            {
                return false;
            }
        }
        true
    }

    fn score(&self, state: &RouteState) -> f64 {
        let from_goal = distance(state.pos, self.goal);
        let time = state.tick.saturating_sub(self.start) as f64 * ROUTE_TICK_COST;
        let pass_bonus = if state.crossed_target {
            TARGET_PASS_BONUS
        } else {
            0.0
        };
        -from_goal - time + pass_bonus
    }

    fn preview(
        &self,
        start: &RouteState,
        anchor: Option<Anchor>,
        windup: usize,
    ) -> Option<(f64, Option<RouteState>)> {
        let mut state = start.clone();
        let ready = state.tick + ROUTE_CAST_GAP + windup;
        let mut recast = None;
        let mut latched = anchor.is_none();
        let new_latch = anchor.map(|anchor| {
            let fire_at = state.tick + windup;
            let latch_at = state.flight.next_latch(state.pos, anchor, fire_at);
            state.flight.fire(state.pos, anchor, fire_at);
            latch_at
        });
        let timeout = state.flight.start + MAX_FLIGHT_TICKS;
        while state.tick + 1 < timeout {
            state.tick += 1;
            while state
                .flight
                .pending
                .front()
                .is_some_and(|shot| shot.latch <= state.tick)
            {
                let shot = state.flight.pending.pop_front().unwrap();
                if !state.flight.latch(state.pos, shot.anchor) {
                    return None;
                }
                latched |= new_latch == Some(shot.latch);
            }
            if let Some(there) = state.flight.step(state.pos) {
                if !self.safe_step(state.pos, there) {
                    return None;
                }
                if let CableAim::Engage(target) = self.aim {
                    state.crossed_target |= segment_distance(target.pos, state.pos, there)
                        <= target.radius + TARGET_PASS_BUFFER;
                }
                state.pos = (world_u64(there.0) as f64, world_u64(there.1) as f64);
            } else if state.flight.pending.is_empty() || state.flight.velocity != (0.0, 0.0) {
                break;
            }
            if state.tick >= ready && recast.is_none() && state.flight.can_fire(state.tick) {
                recast = Some(state.clone());
            }
        }
        let score = if latched {
            self.score(&state)
        } else {
            f64::NEG_INFINITY
        };
        (score.is_finite() || recast.is_some()).then_some((score, recast))
    }

    fn options(
        &self,
        state: &RouteState,
        first: Option<Anchor>,
        windup: usize,
    ) -> Vec<RouteOption> {
        if !state.flight.can_fire(state.tick + windup) {
            return Vec::new();
        }
        route_anchors(&state.flight.grid, self.all, state.pos)
            .filter(|&anchor| state.flight.distinct(anchor))
            .filter_map(|anchor| {
                let (score, recast) = self.preview(state, Some(anchor), windup)?;
                Some(RouteOption {
                    first: first.unwrap_or(anchor),
                    score,
                    recast,
                })
            })
            .collect()
    }
}

pub(super) fn planned_anchor(
    grid: &Grid,
    flight: Option<&Flight>,
    all: &[UnitSnap],
    caster: &UnitSnap,
    aim: CableAim<'_>,
    now: usize,
) -> Option<Anchor> {
    plan(grid, flight, all, caster, aim, now, 1)
}

pub(super) fn planned_cast_anchor(
    grid: &Grid,
    flight: Option<&Flight>,
    all: &[UnitSnap],
    caster: &UnitSnap,
    aim: CableAim<'_>,
    now: usize,
) -> Option<Anchor> {
    plan(grid, flight, all, caster, aim, now, 0)
}

fn plan(
    grid: &Grid,
    flight: Option<&Flight>,
    all: &[UnitSnap],
    caster: &UnitSnap,
    aim: CableAim<'_>,
    now: usize,
    windup: usize,
) -> Option<Anchor> {
    if caster.stack == 0 || flight.is_some_and(|flight| !flight.can_fire(now + windup)) {
        return None;
    }
    let goal = match aim {
        CableAim::Engage(target) => target.pos,
        CableAim::Travel(goal) => goal,
    };
    let initial_distance = distance(caster.pos, goal);
    let minimum_distance = match aim {
        CableAim::Engage(_) => 17_000.0,
        CableAim::Travel(_) => 32_000.0,
    };
    if initial_distance < minimum_distance {
        return None;
    }
    let route = Route {
        all,
        caster,
        aim,
        start: now,
        goal,
        towers: unsheltered_towers(all, caster).collect(),
        threats: if matches!(aim, CableAim::Travel(_)) {
            all.iter()
                .filter(|unit| {
                    unit.alive
                        && unit.team != caster.team
                        && unit.champion
                        && distance(caster.pos, unit.pos) <= ESCAPE_ROUTE_RADIUS
                })
                .map(|unit| (unit, distance(caster.pos, unit.pos)))
                .collect()
        } else {
            Vec::new()
        },
    };
    let state = RouteState {
        flight: flight
            .cloned()
            .unwrap_or_else(|| Flight::new(grid.clone(), now)),
        pos: caster.pos,
        tick: now,
        crossed_target: false,
    };
    let mut best_score = if flight.is_some() {
        route
            .preview(&state, None, 0)
            .map(|(score, _)| score)
            .filter(|score| score.is_finite())
            .unwrap_or(-initial_distance)
    } else {
        -initial_distance
    } + 6_000.0;

    let mut best = None;
    let mut options = route.options(&state, None, windup);
    // Compare a continuous sequence against the hook already in flight and the
    // single-cast options. A side anchor is useful when the sequence it opens
    // makes progress even if its own landing point does not.
    let depth = ROUTE_DEPTH.min(caster.stack);
    for step in 1..=depth {
        options.sort_by(|a, b| b.score.total_cmp(&a.score));
        if let Some(choice) = options.first().filter(|choice| choice.score > best_score) {
            best_score = choice.score;
            best = Some(choice.first);
        }
        if options.is_empty() || step == depth {
            break;
        }
        // Spread the search across redirect setups. Nearby rays hitting the
        // same wall face often yield almost identical trajectories.
        options.sort_by(|a, b| {
            let branch_score = |choice: &RouteOption| {
                choice.recast.as_ref().map_or(f64::NEG_INFINITY, |state| {
                    choice.score.max(route.score(state) - 24_000.0)
                })
            };
            branch_score(b).total_cmp(&branch_score(a))
        });
        let mut chosen_positions = Vec::new();
        options = options
            .into_iter()
            .filter_map(|option| option.recast.map(|state| (option.first, state)))
            .filter(|(_, state)| {
                let anchor = state
                    .flight
                    .pending
                    .back()
                    .map(|shot| shot.anchor)
                    .or(state.flight.anchors[1])
                    .map_or(state.pos, |anchor| anchor.hit.anchor());
                if chosen_positions.iter().any(|&(pos, heading)| {
                    distance(pos, state.pos) < MIN_ANCHOR_SPACING
                        && distance(heading, anchor) < 2.0 * MIN_ANCHOR_SPACING
                }) {
                    return false;
                }
                chosen_positions.push((state.pos, anchor));
                true
            })
            .take(ROUTE_BRANCHES)
            .flat_map(|(first, state)| route.options(&state, Some(first), 1))
            .collect();
    }
    best
}

pub(super) fn blocked(sim: &StableSim<'_>, id: usize) -> bool {
    sim.get_entity(id).is_none_or(|entity| {
        (0..entity.cc_count())
            .filter_map(|i| entity.cc_at(i))
            .any(|cc| {
                cc.tick > 0
                    && ![
                        CcKindV1::Animation,
                        CcKindV1::BlockAttack,
                        CcKindV1::BlockSkill,
                    ]
                    .iter()
                    .any(|kind| kind.code() == cc.kind)
            })
    })
}

pub(super) fn launch(
    sim: &mut StableSim<'_>,
    key: EntityKey,
    existing: Option<Flight>,
    grid: Grid,
    id: usize,
    here: Point,
    anchor: Anchor,
    aim: CableAim<'_>,
) -> bool {
    let now = sim.tick();
    let mut flight = existing.unwrap_or_else(|| Flight::new(grid, now));
    if !flight.can_fire(now) || !flight.distinct(anchor) {
        return false;
    }
    let latch = flight.next_latch(here, anchor, now);
    let forecast = if flight.casts == 0 {
        Some((here, flight.clone()))
    } else {
        flight.predict(here, now, latch)
    };
    let Some((turn_at, mut forecast)) = forecast else {
        return false;
    };
    if !forecast.latch(turn_at, anchor) {
        return false;
    }
    match aim {
        CableAim::Travel(goal) => {
            flight.travel_goal = Some(goal);
            flight.target = None;
        }
        CableAim::Engage(target) => {
            flight.travel_goal = None;
            flight.target = Some(target.id);
        }
    }
    flight.fire(here, anchor, now);
    let intent = match aim {
        CableAim::Engage(target) => format!("engage target={}", target.id),
        CableAim::Travel((x, y)) => format!("travel goal=({x:.0},{y:.0})"),
    };
    wlog(format!(
        "{} CABLE FIRE #{} ({:.0},{:.0}) tower={:?} mode={intent}",
        sim_head(sim, id),
        flight.casts,
        anchor.hit.anchor_x,
        anchor.hit.anchor_y,
        anchor.tower
    ));
    save(key, flight);
    if !has_named_buff(sim, id, FLYING) {
        sim.add_buff(id, &BuffV1::timed(FLYING, MAX_FLIGHT_TICKS + 1));
    }
    true
}

pub(super) fn grid(sim: &StableSim<'_>, flight: Option<&Flight>, all: &[UnitSnap]) -> Grid {
    flight.map_or_else(
        || {
            let mut grid = fanny_map::identify(sim).map_or_else(
                || current_grid(all),
                |mode| {
                    MAP_GRIDS
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .get(&mode.code())
                        .cloned()
                        .unwrap_or_else(|| Grid::builtin(mode))
                },
            );
            let cols = grid.cells.iter().map(Vec::len).max().unwrap_or(0);
            grid.bounds = Some((0.0, 0.0, cols as f64 * CELL, grid.cells.len() as f64 * CELL));
            grid
        },
        |flight| (*flight.grid).clone(),
    )
}

pub(super) fn advance(
    sim: &mut StableSim<'_>,
    id: usize,
    key: EntityKey,
    here: Point,
) -> Option<Point> {
    let mut flight = read(key)?;
    let now = sim.tick();
    if blocked(sim, id)
        || now.saturating_sub(flight.start) >= MAX_FLIGHT_TICKS
        || !has_named_buff(sim, id, FLYING)
    {
        clear(sim, id, key);
        return None;
    }
    for anchor in &mut flight.anchors {
        if anchor
            .and_then(|anchor| anchor.tower)
            .is_some_and(|tower| sim.get_entity(tower).is_none_or(|unit| !unit.is_alive()))
        {
            *anchor = None;
        }
    }
    while flight.pending.front().is_some_and(|shot| shot.latch <= now) {
        let shot = flight.pending.pop_front().unwrap();
        if shot
            .anchor
            .tower
            .is_some_and(|tower| sim.get_entity(tower).is_none_or(|u| !u.is_alive()))
        {
            continue;
        }
        if !flight.latch(here, shot.anchor) {
            continue;
        }
        let at = shot.anchor.hit.anchor();
        let _ = sim.play_view_effect(
            "mlbb_fanny_cable_impact",
            id,
            &InputTargetV1::pos(world_u64(at.0), world_u64(at.1)),
            0,
            0,
            0,
        );
        wlog(format!(
            "{} CABLE LATCH ({:.0},{:.0}) velocity=({:.0},{:.0}) pending={}",
            sim_head(sim, id),
            at.0,
            at.1,
            flight.velocity.0,
            flight.velocity.1,
            flight.pending.len()
        ));
    }

    let mut position = here;
    if let Some(there) = flight.step(here) {
        let x = world_u64(there.0);
        let y = world_u64(there.1);
        if !sim.entity_set_pos(id, x, y) {
            clear(sim, id, key);
            return None;
        }
        position = (x as f64, y as f64);
    } else if flight.pending.is_empty() || flight.velocity != (0.0, 0.0) {
        clear(sim, id, key);
        return None;
    }
    if (now - flight.start) % DRAW_EVERY == 0 {
        // Pending heads and latched anchors share the same two visible slots.
        for anchor in flight
            .anchors
            .iter()
            .rev()
            .flatten()
            .take(2 - flight.pending.len())
        {
            draw_cable_frame(sim, id, position, anchor.hit.anchor(), true);
        }
        for shot in &flight.pending {
            let end = shot.anchor.hit.anchor();
            let fraction = now.saturating_sub(shot.fired) as f64 / (shot.latch - shot.fired) as f64;
            let hook = point_on_ray(
                shot.start,
                (end.0 - shot.start.0, end.1 - shot.start.1),
                fraction,
            );
            draw_cable_frame(sim, id, position, hook, true);
        }
    }
    save(key, flight);
    Some(position)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_grid() -> Grid {
        Grid::from_rows(&[".........."; 10])
    }

    fn hook(anchor: Point, landing: Point) -> Anchor {
        Anchor {
            hit: WallHit {
                x: landing.0,
                y: landing.1,
                distance: 0.0,
                anchor_x: anchor.0,
                anchor_y: anchor.1,
            },
            tower: None,
        }
    }

    #[test]
    fn can_fire_tenth_cable_but_not_eleventh() {
        let mut flight = Flight::new(open_grid(), 0);
        flight.casts = 9;
        assert!(flight.can_fire(54));
        flight.casts = 10;
        assert!(!flight.can_fire(60));
    }

    #[test]
    fn three_casts_redirect_twice_without_landing() {
        let mut here = (160_000.0, 160_000.0);
        let mut flight = Flight::new(open_grid(), 0);
        let first = hook((290_000.0, 160_000.0), (265_000.0, 160_000.0));
        let second = hook((160_000.0, 50_000.0), (160_000.0, 70_000.0));
        let third = hook((75_000.0, 145_000.0), (90_000.0, 145_000.0));
        let mut headings = Vec::new();
        for tick in 0..28 {
            let shot = match tick {
                0 => Some(first),
                6 => Some(second),
                16 => Some(third),
                _ => None,
            };
            if let Some(anchor) = shot {
                assert!(flight.can_fire(tick) && flight.distinct(anchor));
                flight.fire(here, anchor, tick);
                if tick == 6 {
                    assert_eq!(flight.pending.len(), 2);
                    assert_eq!(flight.velocity, (0.0, 0.0));
                }
            }
            if flight
                .pending
                .front()
                .is_some_and(|shot| shot.latch == tick)
            {
                let shot = flight.pending.pop_front().unwrap();
                assert!(flight.latch(here, shot.anchor));
                headings.push(flight.velocity);
            }
            if !headings.is_empty() {
                here = flight.step(here).expect("no landing between hooks");
            }
        }
        assert_eq!(flight.casts, 3);
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0], (CABLE_SPEED, 0.0));
        assert!(headings[1].1 < -CABLE_SPEED / 2.0);
        assert!(headings[2].0 < -CABLE_SPEED / 2.0);
        assert_eq!(
            flight.anchors.map(|a| a.unwrap().hit.anchor()),
            [second.hit.anchor(), third.hit.anchor(),]
        );
    }

    #[test]
    fn a_redirect_can_neither_cross_a_wall_nor_wait_on_the_ground_for_a_late_hook() {
        let grid = Grid::from_rows(&["....#....."; 10]);
        let mut flight = Flight::new(grid, 0);
        let mut here = (110_000.0, 160_000.0);
        let first = hook((128_000.0, 160_000.0), (126_000.0, 160_000.0));
        assert!(flight.latch(here, first));
        assert!(flight.predict(here, 0, 12).is_none());
        while let Some(next) = flight.step(here) {
            assert!(next.0 < 128_000.0);
            here = next;
        }
        assert_eq!(flight.remaining, 0.0);
    }

    fn tower() -> UnitSnap {
        UnitSnap {
            id: 7,
            pos: (85_000.0, 80_000.0),
            team: 2,
            alive: true,
            targetable: true,
            champion: false,
            tower: true,
            monster: false,
            radius: 10_000.0,
            hp: 1000,
            max_hp: 1000,
            stack: 0,
        }
    }

    #[test]
    fn a_tower_cable_uses_the_live_surface_and_respects_wall_occlusion() {
        let mut tower = tower();
        let here = (40_000.0, 80_000.0);
        let aim = tower.pos;
        let anchor = aimed_anchor(&open_grid(), &[tower.clone()], here, aim).unwrap();
        assert_eq!(anchor.tower, Some(7));
        assert_eq!(anchor.hit.anchor(), (75_000.0, 80_000.0));
        let grid = Grid::from_rows(&["..#......."; 10]);
        let anchor = aimed_anchor(&grid, &[tower.clone()], here, aim).unwrap();
        assert_eq!(anchor.tower, None);
        assert!(anchor.hit.anchor().0 < 64_000.0);
        tower.alive = false;
        assert!(aimed_anchor(&open_grid(), &[tower], here, aim).is_none());
    }

    #[test]
    fn hostile_tower_safety_checks_the_segment_and_allows_escape() {
        let tower = tower();
        assert!(enters_tower_range(
            &tower,
            (5_000.0, 85_000.0),
            (170_000.0, 85_000.0)
        ));
        assert!(!enters_tower_range(
            &tower,
            (100_000.0, 80_000.0),
            (170_000.0, 80_000.0)
        ));
    }

    #[test]
    fn travel_cable_follows_the_stock_goal_and_moves_away_from_danger() {
        let grid = Grid::from_rows(&[".........#"; 10]);
        let mut caster = tower();
        caster.pos = (150_000.0, 150_000.0);
        caster.team = 1;
        caster.stack = MAX_CABLE_CHARGES;
        let mut foe = tower();
        foe.tower = false;
        foe.champion = true;
        foe.pos = (120_000.0, 150_000.0);
        let goal = (270_000.0, 150_000.0);
        let anchor = planned_anchor(
            &grid,
            None,
            &[foe.clone()],
            &caster,
            CableAim::Travel(goal),
            0,
        )
        .expect("an open route toward the right wall");
        assert!(anchor.hit.x > caster.pos.0);
        assert!(distance(anchor.hit.pos(), goal) < distance(caster.pos, goal));
        assert!(distance(anchor.hit.pos(), foe.pos) > distance(caster.pos, foe.pos));
        assert!(planned_anchor(&grid, None, &[], &caster, CableAim::Travel(goal), 0).is_some());
    }

    fn assert_continuous_route(
        grid: &Grid,
        mut caster: UnitSnap,
        all: &[UnitSnap],
        aim: CableAim<'_>,
    ) {
        let mut flight = Flight::new(grid.clone(), 0);
        let mut last_fire = None;
        let mut headings = Vec::new();
        for now in 0..MAX_FLIGHT_TICKS {
            while flight.pending.front().is_some_and(|shot| shot.latch <= now) {
                let shot = flight.pending.pop_front().unwrap();
                assert!(flight.latch(caster.pos, shot.anchor));
                headings.push(flight.velocity);
            }
            if headings.len() == 3 {
                assert!(distance(headings[0], headings[1]) > CABLE_SPEED * 0.10);
                assert!(distance(headings[1], headings[2]) > CABLE_SPEED * 0.10);
                return;
            }
            if now != 0 {
                if let Some(next) = flight.step(caster.pos) {
                    caster.pos = next;
                } else if !headings.is_empty() {
                    panic!(
                        "flight ended after {} latches ({} casts)",
                        headings.len(),
                        flight.casts
                    );
                }
            }
            if flight.casts < 3 && last_fire.is_none_or(|tick| now >= tick + ROUTE_CAST_GAP) {
                let existing = (flight.casts > 0).then_some(&flight);
                if let Some(anchor) = planned_cast_anchor(grid, existing, all, &caster, aim, now) {
                    flight.fire(caster.pos, anchor, now);
                    caster.stack -= 1;
                    last_fire = Some(now);
                }
            }
        }
        panic!("the planner did not land three cables in a single flight");
    }

    #[test]
    fn the_moba_route_planner_redirects_twice_before_landing() {
        let grid = Grid::builtin(GameModeKindV1::Moba);
        let caster = UnitSnap {
            pos: (180_000.0, 780_000.0),
            tower: false,
            champion: true,
            stack: MAX_CABLE_CHARGES,
            ..tower()
        };
        let target = UnitSnap {
            id: caster.id + 1,
            team: 1,
            pos: (780_000.0, 180_000.0),
            ..caster.clone()
        };
        assert_continuous_route(&grid, caster, &[target.clone()], CableAim::Engage(&target));
    }

    #[test]
    fn the_moba_route_planner_chains_along_a_stock_lane_waypoint() {
        let mut grid = Grid::builtin(GameModeKindV1::Moba);
        grid.bounds = Some((0.0, 0.0, 960_000.0, 960_000.0));
        let caster = UnitSnap {
            pos: (310_000.0, 64_000.0),
            tower: false,
            champion: true,
            stack: MAX_CABLE_CHARGES,
            ..tower()
        };
        assert_continuous_route(&grid, caster, &[], CableAim::Travel((690_000.0, 64_000.0)));
    }
}
