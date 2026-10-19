//! Stable-ABI wall/cable add-on for the MLBB Fanny mod (v0.2.0).
//!
//! Features in v0.2.0:
//! - True wall-to-wall multi-cable chaining across the map (2-hop lookahead +
//!   swing-past-wall landing geometry so Fanny is never trapped against a wall face).
//! - Outer map boundary wall cabling along Top and Bottom lanes.
//! - Energy system completely removed (pure cooldown-driven Steel Cable chaining).
//! - Fog-of-War & Level-3 unlock awareness.
//! - Smart Target Priority: Enemy Champions first; Jungle Monsters second when
//!   no enemy champions are visible; never confuses target priority in fights.

pub mod crt_shim;
mod maps;


use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use mod_api_stable::*;

const MOD_ID: &str = "mlbb_heroes";

// All distances are TFM2 world units. 60 ticks = one simulation second.
const CELL: f64 = 32_000.0;
const CABLE_RANGE: f64 = 165_000.0; // Reaches across MOBA wall clusters (median gap 91k, max 145k) and lane outer walls.
const CABLE_SPEED: f64 = 5_200.0; // Fast pro-Fanny cable flight speed.
const MIN_ENGAGE_CABLE_DISTANCE: f64 = 24_000.0;
const TARGET_PASS_BUFFER: f64 = 28_000.0; // Matches mid-flight Tornado hit check.
const TARGET_PASS_BONUS: f64 = 65_000.0;
const WALL_GAP: f64 = 1_000.0;
const WALL_STANDOFF: f64 = 14_000.0;
const WALL_SWING_PAST: f64 = 28_000.0;
const WALL_STEP: f64 = 1_000.0;
const WALL_MIN: f64 = 8_000.0;
const RAY_DIRS: usize = 72;

const CABLE_CHAIN: &str = "mlbb_fanny_cable_chain";
const CABLE_ESCAPE: &str = "mlbb_fanny_cable_escape";
const CABLE_IMPACT_EFFECT: &str = "mlbb_heroes:cable_impact";
const CABLE_DRAW_EFFECT: &str = "mlbb_heroes:cable_draw";
const FLYING: &str = "mlbb_fanny_flying";

// Allow target acquisition across the standard 5v5 map.
const SKILL2_RANGE: f64 = 1_350_000.0;
const JUNGLE_CABLE_RANGE: f64 = 420_000.0;
const ESCAPE_ROUTE_RADIUS: f64 = 180_000.0;
const PREY_1: &str = "mlbb_fanny_prey_1";
const PREY_2: &str = "mlbb_fanny_prey_2";
const CABLE_RECHARGE_LOCK: &str = "mlbb_fanny_cable_recharge_lock";
const BASE_CABLE_CHARGES: usize = 4;
const MAX_CABLE_CHARGES: usize = 5;
const MIN_CHARGES_TO_START_CHAIN: usize = 2;
const CABLE_REGEN_TICKS: usize = 100; // +1 cable charge every 1.67s when not flying (~3.3s from 0 -> 2).
const CHAIN_WINDOW: usize = 150; // 2.5 seconds recast chain window.
const MARK_DURATION: usize = 300; // 5 seconds.
const FLIGHT_TORNADO_RADIUS: f64 = 34_000.0;
const FLIGHT_TORNADO_FLAT: usize = 95;
const FLIGHT_TORNADO_AD: usize = 135;
const FLIGHT_TORNADO_COOLDOWN: usize = 8;
const ULT_RANGE: f64 = 65_000.0;
const ULT_CONTACT: f64 = 10_000.0;
const ULT_SPEED: f64 = 8_500.0;
const ULT_FLAT_DAMAGE: usize = 180;
const ULT_BASE_AD_RATIO: usize = 190;
const ULT_MARK_AD_RATIO: usize = 45;

// 5v5 map outer lane boundaries act as solid outer walls so Fanny can cable along
// Top and Bottom lanes just like in MLBB.
const MOBA_BOUNDS: (f64, f64, f64, f64) = (56_000.0, 56_000.0, 904_000.0, 904_000.0);
const MODES: [GameModeKindV1; 3] = [
    GameModeKindV1::Moba,
    GameModeKindV1::SingleLane,
    GameModeKindV1::DeathMatch,
];
const LOG_EVERY: usize = 30;

static LOG_LOCK: Mutex<()> = Mutex::new(());

fn log_path() -> PathBuf {
    if let Some(path) = std::env::var_os("MLBB_FANNY_WALL_LOG") {
        return PathBuf::from(path);
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let dir = PathBuf::from(appdata)
            .join("TeamSamoyed")
            .join("TeamfightManager2")
            .join("data");
        let _ = std::fs::create_dir_all(&dir);
        if dir.is_dir() {
            return dir.join("mlbb_heroes.log");
        }
    }
    PathBuf::from("mlbb_heroes.log")
}

static LOG_PATH: LazyLock<PathBuf> = LazyLock::new(log_path);

fn wlog(message: impl AsRef<str>) {
    let _guard = LOG_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&*LOG_PATH)
    {
        let _ = writeln!(file, "{}", message.as_ref());
        let _ = file.flush();
    }
}

fn sim_head(sim: &StableSim<'_>, caster_id: usize) -> String {
    let origin = sim.sim_origin().unwrap_or_default();
    let kind = match SimOriginKindV1::from_code(origin.kind) {
        Some(SimOriginKindV1::ServerPresim) => "presim",
        Some(SimOriginKindV1::ClientMatchView) => "view",
        Some(SimOriginKindV1::ClientSpectate) => "spectate",
        Some(SimOriginKindV1::ClientReplay) => "replay",
        Some(SimOriginKindV1::Tool) => "tool",
        _ => "unknown",
    };
    let match_id = if origin.match_id == SimOriginV1::NONE {
        "-".to_string()
    } else {
        origin.match_id.to_string()
    };
    let set_index = if origin.set_index == SimOriginV1::NONE {
        "-".to_string()
    } else {
        origin.set_index.to_string()
    };
    format!(
        "[{kind} m={match_id} s={set_index}] t={} fanny#{caster_id}",
        sim.tick()
    )
}

/// `cells[y][x]`, with nonzero values marking collision walls.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub cells: Vec<Vec<i64>>,
    pub bounds: Option<(f64, f64, f64, f64)>,
}

impl Grid {
    pub fn from_rows(rows: &[&str]) -> Self {
        Self {
            cells: rows
                .iter()
                .map(|row| row.chars().map(|ch| i64::from(ch == '#')).collect())
                .collect(),
            bounds: None,
        }
    }

    pub fn builtin(mode: GameModeKindV1) -> Self {
        let rows = match mode {
            GameModeKindV1::Moba => &maps::MOBA,
            GameModeKindV1::SingleLane => &maps::SINGLE_LANE,
            GameModeKindV1::DeathMatch => &maps::DEATH_MATCH,
        };
        let mut grid = Self::from_rows(rows);
        if mode == GameModeKindV1::Moba {
            grid.bounds = Some(MOBA_BOUNDS);
        }
        grid
    }

    pub fn is_wall(&self, x: f64, y: f64) -> bool {
        if x < 0.0 || y < 0.0 {
            return true;
        }
        let (cx, cy) = ((x / CELL) as usize, (y / CELL) as usize);
        self.cells
            .get(cy)
            .and_then(|row| row.get(cx))
            .is_none_or(|value| *value != 0)
    }

    pub fn beyond_edge(&self, x: f64, y: f64) -> bool {
        self.bounds
            .is_some_and(|(x0, y0, x1, y1)| x < x0 || y < y0 || x > x1 || y > y1)
    }

    pub fn solid(&self, x: f64, y: f64) -> bool {
        self.is_wall(x, y) || self.beyond_edge(x, y)
    }

    pub fn on_map(&self, x: f64, y: f64) -> bool {
        let width = self.cells.iter().map(Vec::len).max().unwrap_or(0) as f64;
        x >= 0.0 && y >= 0.0 && x < width * CELL && y < self.cells.len() as f64 * CELL
    }

    pub fn wall_count(&self) -> usize {
        self.cells
            .iter()
            .flatten()
            .filter(|cell| **cell != 0)
            .count()
    }
}

/// Parse the game's `walls` JSON field (an integer 2-D array) without bringing
/// a JSON dependency across the native ABI boundary.
pub fn parse_int_grid(json: &str) -> Option<Grid> {
    let mut parser = GridParser {
        bytes: json.as_bytes(),
        index: 0,
    };
    parser.whitespace();
    parser.eat(b'[')?;
    let mut rows = Vec::new();
    loop {
        parser.whitespace();
        if parser.peek()? == b']' {
            break;
        }
        parser.eat(b'[')?;
        let mut row = Vec::new();
        loop {
            parser.whitespace();
            if parser.peek()? == b']' {
                parser.index += 1;
                break;
            }
            row.push(parser.number()?);
            parser.whitespace();
            if parser.peek() == Some(b',') {
                parser.index += 1;
            } else if parser.peek() != Some(b']') {
                return None;
            }
        }
        rows.push(row);
        parser.whitespace();
        if parser.peek() == Some(b',') {
            parser.index += 1;
        } else if parser.peek() != Some(b']') {
            return None;
        }
    }
    (!rows.is_empty()).then_some(Grid {
        cells: rows,
        bounds: None,
    })
}

struct GridParser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl GridParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        (self.peek()? == byte).then(|| self.index += 1)
    }

    fn number(&mut self) -> Option<i64> {
        let start = self.index;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.index += 1;
        }
        if start == self.index {
            return None;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.index]).ok()?;
        text.parse::<i64>()
            .ok()
            .or_else(|| text.parse::<f64>().ok().map(|value| value as i64))
    }
}

static MAP_GRIDS: LazyLock<Mutex<HashMap<u32, Grid>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct FannyMapReader;

impl StableMapCustomizer for FannyMapReader {
    fn customize(&self, mode: Option<GameModeKindV1>, doc: &mut StableJsonDoc<'_>) {
        if let (Some(mode), Some(raw)) = (mode, doc.get_json("walls")) {
            if let Some(mut grid) = parse_int_grid(&raw) {
                if mode == GameModeKindV1::Moba {
                    grid.bounds = Some(MOBA_BOUNDS);
                }
                MAP_GRIDS
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .insert(mode.code(), grid);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct UnitSnap {
    id: usize,
    pos: (f64, f64),
    team: usize,
    alive: bool,
    targetable: bool,
    champion: bool,
    tower: bool,
    monster: bool,
    level: usize,
    radius: f64,
    hp: usize,
    max_hp: usize,
    stack: usize,
}

fn fanny_base_stat(level: usize, stack: usize) -> StatV1 {
    let lvl_growth = level.saturating_sub(1);
    StatV1 {
        attack: 100 + 17 * lvl_growth,
        magic_power: 0,
        hp: 900 + 88 * lvl_growth,
        defence: 28 + 6 * lvl_growth,
        magic_resistance: 22 + 4 * lvl_growth,
        move_speed: 1100 + 10 * lvl_growth,
        hp_regen: 0,
        stack,
        crit_chance: 0,
    }
}

fn get_cable_charges(sim: &StableSim<'_>, entity_id: usize) -> usize {
    sim.get_entity(entity_id)
        .map_or(0, |entity| entity.stat().stack)
}

fn set_cable_charges(sim: &mut StableSim<'_>, entity_id: usize, charges: usize) {
    let clamped = charges.min(MAX_CABLE_CHARGES);
    let level = sim
        .get_entity(entity_id)
        .map_or(1, |entity| entity.level().max(1));
    let base_stat = fanny_base_stat(level, clamped);
    sim.entity_set_base_stat(entity_id, &base_stat);
}

fn refund_cable_charge_and_heal(sim: &mut StableSim<'_>, caster_id: usize, bonus_charges: usize) {
    let current = get_cable_charges(sim, caster_id);
    let next = (current + bonus_charges).min(MAX_CABLE_CHARGES);
    set_cable_charges(sim, caster_id, next);
    sim.entity_remove_buff(caster_id, CABLE_RECHARGE_LOCK);
    if let Some((_hp, max_hp)) = sim.get_entity(caster_id).map(|e| e.hp()) {
        let heal_amt = (max_hp / 20).max(25);
        sim.heal(caster_id, caster_id, heal_amt);
    }
}

fn units(sim: &StableSim<'_>) -> Vec<UnitSnap> {
    (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .map(|entity| {
            let (x, y) = entity.pos();
            let (hp, max_hp) = entity.hp();
            let champion = entity.is_champion();
            let tower = entity.is_tower();
            let monster = !champion && !tower && !entity.is_minion();
            UnitSnap {
                id: entity.id(),
                pos: (x as f64, y as f64),
                team: entity.team(),
                alive: entity.is_alive(),
                targetable: entity.is_targetable(),
                champion,
                tower,
                monster,
                level: entity.level().max(1),
                radius: entity.radius().max(8_000) as f64,
                hp,
                max_hp,
                stack: entity.stat().stack,
            }
        })
        .collect()
}

fn is_unit_visible(sim: &StableSim<'_>, viewer_team: usize, unit: &UnitSnap) -> bool {
    sim.is_visible(viewer_team, unit.id)
}

fn choose_current_grid(points: &[(f64, f64)]) -> Option<(GameModeKindV1, Grid)> {
    MODES
        .iter()
        .copied()
        .map(|mode| {
            let cached = MAP_GRIDS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(&mode.code())
                .cloned();
            let grid = cached.unwrap_or_else(|| Grid::builtin(mode));
            let wall_occupancy = points
                .iter()
                .filter(|point| grid.is_wall(point.0, point.1))
                .count();
            (wall_occupancy, grid.wall_count(), mode, grid)
        })
        .min_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|(_, _, mode, grid)| (mode, grid))
}

fn current_grid(all: &[UnitSnap]) -> Grid {
    let points: Vec<(f64, f64)> = all
        .iter()
        .filter(|unit| unit.alive && unit.champion)
        .map(|unit| unit.pos)
        .collect();
    choose_current_grid(&points)
        .map(|(_, grid)| grid)
        .unwrap_or_else(|| Grid::builtin(GameModeKindV1::Moba))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallHit {
    /// Swing landing coordinates (open space past the wall corner or stood off from wall face).
    pub x: f64,
    pub y: f64,
    /// Flight distance from origin to `(x, y)`.
    pub distance: f64,
    /// Exact wall hook contact coordinates for drawing the cable wire & hook icon.
    pub anchor_x: f64,
    pub anchor_y: f64,
}

impl WallHit {
    pub fn pos(self) -> (f64, f64) {
        (self.x, self.y)
    }

    pub fn anchor(self) -> (f64, f64) {
        (self.anchor_x, self.anchor_y)
    }
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn point_on_ray(origin: (f64, f64), direction: (f64, f64), d: f64) -> (f64, f64) {
    (origin.0 + direction.0 * d, origin.1 + direction.1 * d)
}

/// Given a ray that hits a wall at `wall_dist`, compute where Fanny's body
/// swings/lands such that the entire straight-line flight path from `origin`
/// to `landing` is 100% clear of `#` wall cells (`path_is_clear(grid, origin, landing)`):
/// 1. Swing laterally past the wall corner into open space so she has 360°
///    open sightlines for her next cable without colliding with the wall during `ForceMove`.
/// 2. If hitting an outer map boundary (`beyond_edge`) or wide flat wall face,
///    glide along the open tangent and stand off `WALL_STANDOFF` (`14,000`)
///    units from the wall face so she is never pinned at `1,000` units.
fn compute_swing_landing(
    grid: &Grid,
    origin: (f64, f64),
    direction: (f64, f64),
    wall_dist: f64,
) -> (f64, f64) {
    let perp_a = (-direction.1, direction.0);
    let perp_b = (direction.1, -direction.0);
    let mut best_pt: Option<(f64, f64)> = None;
    let mut best_reach = -1.0_f64;

    for forward in [36_000.0, 24_000.0, 12_000.0, 0.0, -8_000.0, -16_000.0] {
        let base_d = (wall_dist + forward).max(WALL_MIN);
        let base = point_on_ray(origin, direction, base_d);
        for side in [14_000.0, 24_000.0, 34_000.0, 46_000.0] {
            for perp in [perp_a, perp_b] {
                let candidate = (base.0 + perp.0 * side, base.1 + perp.1 * side);
                let d = distance(origin, candidate);
                if (WALL_MIN..=CABLE_RANGE + WALL_SWING_PAST).contains(&d)
                    && grid.on_map(candidate.0, candidate.1)
                    && !grid.solid(candidate.0, candidate.1)
                    && path_is_clear(grid, origin, candidate)
                    && d > best_reach
                {
                    best_reach = d;
                    best_pt = Some(candidate);
                }
            }
        }
        if let Some(pt) = best_pt {
            if forward >= 12_000.0 {
                return pt;
            }
        }
    }

    if let Some(pt) = best_pt {
        return pt;
    }

    // Fallback: clean stand-off before the wall so subsequent rays are not blocked.
    let standoff = (wall_dist - WALL_STANDOFF).max(WALL_MIN);
    let fallback = point_on_ray(origin, direction, standoff);
    if grid.on_map(fallback.0, fallback.1)
        && !grid.solid(fallback.0, fallback.1)
        && path_is_clear(grid, origin, fallback)
    {
        return fallback;
    }
    point_on_ray(origin, direction, (wall_dist - WALL_GAP).max(0.0))
}

/// Return one candidate per direction, stopping before the first wall or 5v5
/// map edge. The hook `anchor` sits on the wall surface (`low - WALL_GAP`),
/// while `(x, y)` is Fanny's swing landing position.
pub fn wall_hits(grid: &Grid, origin: (f64, f64), range: f64) -> Vec<WallHit> {
    let mut hits = Vec::new();
    let origin_in_solid = grid.solid(origin.0, origin.1);
    for index in 0..RAY_DIRS {
        let angle = index as f64 * std::f64::consts::TAU / RAY_DIRS as f64;
        let direction = (angle.cos(), angle.sin());
        let mut sample_distance = WALL_STEP;
        // If Fanny is currently mid-flight over a wall cell (`ignore_wall: true`),
        // step out of the current wall first before searching for the next wall.
        if origin_in_solid {
            while sample_distance <= range {
                let sample = point_on_ray(origin, direction, sample_distance);
                if !grid.solid(sample.0, sample.1) {
                    break;
                }
                sample_distance += WALL_STEP;
            }
        }
        let mut free_distance = sample_distance;
        while sample_distance <= range + WALL_GAP {
            let sample = point_on_ray(origin, direction, sample_distance);
            if grid.bounds.is_none() && !grid.on_map(sample.0, sample.1) {
                break;
            }
            if grid.solid(sample.0, sample.1) {
                let (mut low, mut high) = (free_distance, sample_distance);
                for _ in 0..14 {
                    let middle = (low + high) / 2.0;
                    let point = point_on_ray(origin, direction, middle);
                    if grid.solid(point.0, point.1) {
                        high = middle;
                    } else {
                        low = middle;
                    }
                }
                let hook_distance = (low - WALL_GAP).max(0.0);
                let anchor = point_on_ray(origin, direction, hook_distance);
                if hook_distance >= WALL_MIN
                    && hook_distance <= range
                    && !grid.is_wall(anchor.0, anchor.1)
                    && !grid.beyond_edge(anchor.0, anchor.1)
                {
                    let landing = if range >= 100_000.0 {
                        compute_swing_landing(grid, origin, direction, low)
                    } else {
                        anchor
                    };
                    let flight_dist = distance(origin, landing).max(hook_distance);
                    if !grid.is_wall(landing.0, landing.1)
                        && !grid.beyond_edge(landing.0, landing.1)
                    {
                        hits.push(WallHit {
                            x: landing.0,
                            y: landing.1,
                            distance: flight_dist,
                            anchor_x: anchor.0,
                            anchor_y: anchor.1,
                        });
                    }
                }
                break;
            }
            free_distance = sample_distance;
            sample_distance += WALL_STEP;
        }
    }
    hits
}

fn segment_distance(point: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let delta = (b.0 - a.0, b.1 - a.1);
    let length_squared = delta.0 * delta.0 + delta.1 * delta.1;
    let t = if length_squared > 0.0 {
        (((point.0 - a.0) * delta.0 + (point.1 - a.1) * delta.1) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    distance(point, (a.0 + delta.0 * t, a.1 + delta.1 * t))
}

fn path_is_clear(grid: &Grid, from: (f64, f64), to: (f64, f64)) -> bool {
    let length = distance(from, to);
    let steps = (length / 2_000.0).ceil().max(1.0) as usize;
    (0..=steps).all(|index| {
        let ratio = index as f64 / steps as f64;
        let point = (
            from.0 + (to.0 - from.0) * ratio,
            from.1 + (to.1 - from.1) * ratio,
        );
        !grid.is_wall(point.0, point.1)
    })
}

fn named_buff_count(sim: &StableSim<'_>, entity_id: usize, name: &str) -> usize {
    let Some(entity) = sim.get_entity(entity_id) else {
        return 0;
    };
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .filter(|buff| buff.name() == name)
        .count()
}

fn has_named_buff(sim: &StableSim<'_>, entity_id: usize, name: &str) -> bool {
    named_buff_count(sim, entity_id, name) > 0
}

fn mark_level(sim: &StableSim<'_>, target_id: usize) -> usize {
    if has_named_buff(sim, target_id, PREY_2) {
        2
    } else if has_named_buff(sim, target_id, PREY_1) {
        1
    } else {
        0
    }
}

fn apply_or_upgrade_prey_mark(sim: &mut StableSim<'_>, target_id: usize) -> bool {
    match mark_level(sim, target_id) {
        0 => {
            sim.add_buff(target_id, &BuffV1::timed(PREY_1, MARK_DURATION));
            true
        }
        1 => {
            sim.entity_remove_buff(target_id, PREY_1);
            sim.add_buff(target_id, &BuffV1::timed(PREY_2, MARK_DURATION));
            true
        }
        _ => false,
    }
}

/// Smart target selection with Fog-of-War filtering and strict priority:
/// 1. Visible living enemy Champions (preferred input target first, then nearest).
/// 2. Visible living Jungle Monsters (`unit.monster`) ONLY when zero enemy
///    champions are visible on the map.
fn choose_target<'a>(
    sim: &StableSim<'_>,
    all: &'a [UnitSnap],
    caster: &UnitSnap,
    input: InputTargetV1,
) -> Option<&'a UnitSnap> {
    let valid_enemy_champ = |unit: &&UnitSnap| {
        unit.id != caster.id
            && unit.team != caster.team
            && unit.alive
            && unit.targetable
            && unit.champion
            && is_unit_visible(sim, caster.team, unit)
            && within_cable_approach_range(distance(caster.pos, unit.pos))
    };

    // Priority 1: If the input explicitly targeted a visible enemy champion, honor it.
    if input.kind == InputTargetKindV1::Target.code() {
        if let Some(target) = all
            .iter()
            .find(|unit| unit.id == input.target_id && valid_enemy_champ(unit))
        {
            return Some(target);
        }
    }

    // Priority 1b: Nearest visible enemy champion on the map.
    if let Some(champ) = all
        .iter()
        .filter(valid_enemy_champ)
        .min_by(|a, b| distance(caster.pos, a.pos).total_cmp(&distance(caster.pos, b.pos)))
    {
        return Some(champ);
    }

    // Priority 2: No enemy champion is visible anywhere in range -> allow cabling
    // to a visible Jungle Monster within jungle approach range.
    let valid_monster = |unit: &&UnitSnap| {
        unit.id != caster.id
            && unit.team != caster.team
            && unit.alive
            && unit.targetable
            && unit.monster
            && is_unit_visible(sim, caster.team, unit)
            && distance(caster.pos, unit.pos) <= JUNGLE_CABLE_RANGE
    };

    if input.kind == InputTargetKindV1::Target.code() {
        if let Some(monster) = all
            .iter()
            .find(|unit| unit.id == input.target_id && valid_monster(unit))
        {
            return Some(monster);
        }
    }

    all.iter()
        .filter(valid_monster)
        .min_by(|a, b| distance(caster.pos, a.pos).total_cmp(&distance(caster.pos, b.pos)))
}

fn dangerous_escape(all: &[UnitSnap], caster: &UnitSnap) -> bool {
    let nearby_foes: Vec<&UnitSnap> = all
        .iter()
        .filter(|unit| {
            unit.alive
                && unit.targetable
                && unit.champion
                && unit.team != caster.team
                && distance(caster.pos, unit.pos) <= 48_000.0
        })
        .collect();
    let nearby_allies = all
        .iter()
        .filter(|unit| {
            unit.alive
                && unit.champion
                && unit.team == caster.team
                && unit.id != caster.id
                && distance(caster.pos, unit.pos) <= 42_000.0
        })
        .count();
    let low_hp = caster.max_hp > 0 && caster.hp * 100 / caster.max_hp <= 28;
    (low_hp && !nearby_foes.is_empty())
        || (nearby_foes.len() >= 2 && nearby_allies == 0 && caster.hp * 100 / caster.max_hp.max(1) <= 55)
        || (nearby_foes.len() >= 3 && nearby_allies <= 1 && caster.hp * 100 / caster.max_hp.max(1) <= 65)
}

#[cfg(test)]
fn choose_engage_hit(
    hits: &[WallHit],
    here: (f64, f64),
    target: (f64, f64),
    target_radius: f64,
    hostile_towers: &[UnitSnap],
) -> Option<WallHit> {
    choose_engage_hit_with_grid(None, hits, here, target, target_radius, hostile_towers)
}

/// 2-hop lookahead route planner: scores each first-cable candidate both by its
/// direct progress/slash through `target` and by the best second-cable chain
/// reachable from its landing position.
fn choose_engage_hit_with_grid(
    grid: Option<&Grid>,
    hits: &[WallHit],
    here: (f64, f64),
    target: (f64, f64),
    target_radius: f64,
    hostile_towers: &[UnitSnap],
) -> Option<WallHit> {
    let current_distance = distance(here, target);
    if current_distance < 17_000.0 {
        return None;
    }
    hits.iter()
        .copied()
        .filter_map(|hit| {
            let destination = hit.pos();
            let after = distance(destination, target);
            let progress = current_distance - after;
            let path_miss = segment_distance(target, here, destination);
            let passes_target = path_miss <= target_radius + TARGET_PASS_BUFFER;
            if progress < 2_500.0
                || (after < 12_000.0 && !passes_target)
                || (hit.distance < MIN_ENGAGE_CABLE_DISTANCE && !passes_target)
            {
                return None;
            }
            let move_vec = (destination.0 - here.0, destination.1 - here.1);
            let target_vec = (target.0 - here.0, target.1 - here.1);
            let move_len = distance(here, destination).max(1.0);
            let denom = (move_len * current_distance).max(1.0);
            let alignment =
                ((move_vec.0 * target_vec.0 + move_vec.1 * target_vec.1) / denom).clamp(-1.0, 1.0);
            if alignment < 0.05 && !passes_target {
                return None;
            }
            let tower_penalty = hostile_towers
                .iter()
                .filter(|tower| tower.alive)
                .map(|tower| (38_000.0 - distance(destination, tower.pos)).max(0.0) * 0.8)
                .sum::<f64>();
            let pass_bonus = if passes_target {
                TARGET_PASS_BONUS
            } else {
                0.0
            };

            // 2-hop lookahead: if the target is still far after hop 1, evaluate
            // whether hop 1's landing spot has a strong second wall cable toward `target`.
            let lookahead_adjustment = if let Some(g) = grid {
                if !passes_target && after > 32_000.0 {
                    let next_hits = wall_hits(g, destination, CABLE_RANGE);
                    let best_second_gain = next_hits
                        .iter()
                        .filter_map(|h2| {
                            let d2 = h2.pos();
                            let after2 = distance(d2, target);
                            let gain2 = after - after2;
                            let pass2 =
                                segment_distance(target, destination, d2) <= target_radius + TARGET_PASS_BUFFER;
                            if gain2 >= 8_000.0 || pass2 {
                                Some(gain2 + if pass2 { 35_000.0 } else { 0.0 })
                            } else {
                                None
                            }
                        })
                        .fold(0.0_f64, f64::max);
                    if best_second_gain > 0.0 {
                        -best_second_gain * 0.55
                    } else {
                        95_000.0 // Dead-end wall penalty so Fanny picks chainable walls instead.
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let score = after
                + hit.distance * 0.10
                + (1.0 - alignment) * 18_000.0
                + tower_penalty
                + lookahead_adjustment
                - pass_bonus;
            Some((score, hit))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, hit)| hit)
}

fn choose_escape_hit(hits: &[WallHit], here: (f64, f64), enemies: &[&UnitSnap]) -> Option<WallHit> {
    if enemies.is_empty() {
        return None;
    }
    let nearest_before = enemies
        .iter()
        .map(|enemy| distance(here, enemy.pos))
        .fold(f64::INFINITY, f64::min);
    hits.iter()
        .copied()
        .filter_map(|hit| {
            let nearest_after = enemies
                .iter()
                .map(|enemy| distance(hit.pos(), enemy.pos))
                .fold(f64::INFINITY, f64::min);
            if nearest_after < nearest_before + 3_000.0 {
                return None;
            }
            let score = -nearest_after + hit.distance * 0.08;
            Some((score, hit))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, hit)| hit)
}

fn choose_cable_route(
    all: &[UnitSnap],
    caster: &UnitSnap,
    target: &UnitSnap,
    escaping: bool,
) -> Option<WallHit> {
    let grid = current_grid(all);
    let hits = wall_hits(&grid, caster.pos, CABLE_RANGE);
    if escaping {
        let enemies: Vec<&UnitSnap> = all
            .iter()
            .filter(|unit| {
                unit.alive
                    && unit.targetable
                    && unit.champion
                    && unit.team != caster.team
                    && distance(caster.pos, unit.pos) <= ESCAPE_ROUTE_RADIUS
            })
            .collect();
        choose_escape_hit(&hits, caster.pos, &enemies)
    } else {
        let hostile_towers: Vec<UnitSnap> = all
            .iter()
            .filter(|unit| unit.tower && unit.alive && unit.team != caster.team)
            .cloned()
            .collect();
        choose_engage_hit_with_grid(
            Some(&grid),
            &hits,
            caster.pos,
            target.pos,
            target.radius,
            &hostile_towers,
        )
    }
}

fn force_move(
    sim: &mut StableSim<'_>,
    entity_id: usize,
    from: (f64, f64),
    to: (f64, f64),
    speed: f64,
) -> usize {
    let length = distance(from, to);
    if length < 1.0 {
        return 0;
    }
    let ticks = (length / speed).ceil().max(1.0) as usize;
    let mut cc = CcV1::of_kind(CcKindV1::ForceMove, ticks as u64);
    cc.dx = (to.0 - from.0).round() as i64;
    cc.dy = (to.1 - from.1).round() as i64;
    cc.speed = (length / ticks as f64).ceil() as u64;
    sim.apply_cc(entity_id, &cc);
    ticks
}

fn world_u64(value: f64) -> u64 {
    if value.is_finite() {
        value.round().max(0.0).min(u64::MAX as f64) as u64
    } else {
        0
    }
}

const CABLE_WIRE_EFFECTS: [&str; 12] = [
    "mlbb_fanny_cable_wire_0",
    "mlbb_fanny_cable_wire_1",
    "mlbb_fanny_cable_wire_2",
    "mlbb_fanny_cable_wire_3",
    "mlbb_fanny_cable_wire_4",
    "mlbb_fanny_cable_wire_5",
    "mlbb_fanny_cable_wire_6",
    "mlbb_fanny_cable_wire_7",
    "mlbb_fanny_cable_wire_8",
    "mlbb_fanny_cable_wire_9",
    "mlbb_fanny_cable_wire_10",
    "mlbb_fanny_cable_wire_11",
];

/// Draw the directional thin steel cable wire (`mlbb_fanny_cable_wire_0..11`) along the
/// tether and a SINGLE hook head (`mlbb_fanny_cable_hook`) at the wall anchor (`anchor`).
fn draw_cable_frame(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    from: (f64, f64),
    anchor: (f64, f64),
    draw_hook: bool,
) {
    const WIRE: u32 = 0xC8E6F4FF;
    const HOOK: u32 = 0xE8C36AFF;
    sim.debug_draw_line(
        world_u64(from.0),
        world_u64(from.1),
        world_u64(anchor.0),
        world_u64(anchor.1),
        WIRE,
    );

    let length = distance(from, anchor);
    if length >= 12_000.0 {
        let dx = anchor.0 - from.0;
        let dy = anchor.1 - from.1;
        let theta = dy.atan2(dx).rem_euclid(std::f64::consts::PI);
        let bucket = (((theta / std::f64::consts::PI) * 12.0).round() as usize) % 12;
        let wire_effect = CABLE_WIRE_EFFECTS[bucket];
        // Stop wire segments 10,000 units before the wall anchor so the wire feeds cleanly into the single hook head
        let wire_len = (length - 10_000.0).max(4_000.0);
        let step = 13_000.0;
        let count = (wire_len / step).ceil().clamp(1.0, 11.0) as usize;
        let usable_ratio = (wire_len / length).clamp(0.2, 0.92);
        for i in 0..count {
            let t = ((i as f64 + 0.5) / (count as f64)) * usable_ratio;
            let px = from.0 + dx * t;
            let py = from.1 + dy * t;
            let pos_target = InputTargetV1::pos(world_u64(px), world_u64(py));
            let _ = sim.play_view_effect(wire_effect, caster_id, &pos_target, 0, 0, 0);
        }
    }
    if draw_hook {
        let anchor_target = InputTargetV1::pos(world_u64(anchor.0), world_u64(anchor.1));
        let _ = sim.play_view_effect("mlbb_fanny_cable_hook", caster_id, &anchor_target, 0, 0, 0);
    }

    let length = length.max(1.0);
    let along = ((from.0 - anchor.0) / length, (from.1 - anchor.1) / length);
    let across = (-along.1, along.0);
    let root = (anchor.0 + along.0 * 6_000.0, anchor.1 + along.1 * 6_000.0);
    let bend = (
        anchor.0 + along.0 * 2_500.0 + across.0 * 4_500.0,
        anchor.1 + along.1 * 2_500.0 + across.1 * 4_500.0,
    );
    let tip = (anchor.0 + across.0 * 6_000.0, anchor.1 + across.1 * 6_000.0);
    for (start, end) in [(anchor, root), (root, bend), (bend, tip)] {
        sim.debug_draw_line(
            world_u64(start.0),
            world_u64(start.1),
            world_u64(end.0),
            world_u64(end.1),
            HOOK,
        );
    }
    sim.debug_draw_circle(world_u64(anchor.0), world_u64(anchor.1), 1_500, HOOK);
}

fn queue_cable_frames(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    from: (f64, f64),
    anchor: (f64, f64),
    flight_ticks: usize,
) -> usize {
    draw_cable_frame(sim, caster_id, from, anchor, true);
    let input = InputTargetV1::pos(world_u64(anchor.0), world_u64(anchor.1));
    let mut queued = 0;
    for delay in (4..=flight_ticks).step_by(4) {
        if sim.queue_effect(
            CABLE_DRAW_EFFECT,
            AttackTypeV1::Skill,
            caster_id,
            &input,
            delay,
        ) {
            queued += 1;
        } else {
            break;
        }
    }
    queued
}

struct CableDraw;

impl StableEffectType for CableDraw {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Pos.code() {
            return;
        }
        let Some((from, alive)) = sim.get_entity(caster_id).map(|entity| {
            let (x, y) = entity.pos();
            ((x as f64, y as f64), entity.is_alive())
        }) else {
            return;
        };
        if !alive {
            return;
        }
        let refresh_hook = sim.tick() % 6 == 0;
        draw_cable_frame(sim, caster_id, from, (input.x as f64, input.y as f64), refresh_hook);
    }
}

struct SteelCable;

impl StableEffectType for SteelCable {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let all = units(sim);
        let Some(caster) = all.iter().find(|unit| unit.id == caster_id && unit.alive) else {
            wlog(format!(
                "{} CABLE REFUSED: caster is not alive",
                sim_head(sim, caster_id)
            ));
            return;
        };
        let Some(target) = choose_target(sim, &all, caster, input) else {
            wlog(format!(
                "{} CABLE REFUSED: no visible enemy champion or jungle monster",
                sim_head(sim, caster_id)
            ));
            return;
        };
        let target_distance = distance(caster.pos, target.pos);
        if !within_cable_approach_range(target_distance) {
            wlog(format!(
                "{} CABLE REFUSED: target #{} is {:.0} away (action range {:.0})",
                sim_head(sim, caster_id),
                target.id,
                target_distance,
                SKILL2_RANGE
            ));
            return;
        }
        let chain_active = has_named_buff(sim, caster_id, CABLE_CHAIN);
        let escaping = dangerous_escape(&all, caster)
            || (chain_active && has_named_buff(sim, caster_id, CABLE_ESCAPE));
        let Some(destination) = choose_cable_route(&all, caster, target, escaping) else {
            wlog(format!(
                "{} CABLE REFUSED: no valid wall route toward target #{} at ({:.0},{:.0})",
                sim_head(sim, caster_id),
                target.id,
                target.pos.0,
                target.pos.1
            ));
            return;
        };

        let current_charges = caster.stack;
        if current_charges == 0 {
            wlog(format!(
                "{} CABLE REFUSED: 0 cable charges remaining (recharging)",
                sim_head(sim, caster_id)
            ));
            return;
        }
        let flight_ticks = force_move(sim, caster_id, caster.pos, destination.pos(), CABLE_SPEED);
        if flight_ticks == 0 {
            return;
        }
        let remaining_charges = current_charges.saturating_sub(1);
        set_cable_charges(sim, caster_id, remaining_charges);
        if remaining_charges == 0 {
            sim.entity_remove_buff(caster_id, CABLE_RECHARGE_LOCK);
            sim.add_buff(
                caster_id,
                &BuffV1::timed(CABLE_RECHARGE_LOCK, CABLE_REGEN_TICKS * 2),
            );
        }
        let wall_anchor = destination.anchor();
        let impact_at = InputTargetV1::pos(world_u64(wall_anchor.0), world_u64(wall_anchor.1));
        let draw_ticks =
            queue_cable_frames(sim, caster_id, caster.pos, wall_anchor, flight_ticks);
        let impact_queued = sim.queue_effect(
            CABLE_IMPACT_EFFECT,
            AttackTypeV1::Skill,
            caster_id,
            &impact_at,
            flight_ticks,
        );
        sim.entity_remove_buff(caster_id, FLYING);
        sim.add_buff(caster_id, &BuffV1::timed(FLYING, flight_ticks + 1));
        sim.entity_remove_buff(caster_id, CABLE_CHAIN);
        sim.add_buff(caster_id, &BuffV1::timed(CABLE_CHAIN, CHAIN_WINDOW));
        sim.entity_remove_buff(caster_id, CABLE_ESCAPE);
        if escaping {
            sim.add_buff(caster_id, &BuffV1::timed(CABLE_ESCAPE, CHAIN_WINDOW));
        }
        let start_effect = sim.play_view_effect(
            "mlbb_fanny_cable_start",
            caster_id,
            &InputTargetV1::target(caster_id),
            0,
            0,
            0,
        );
        sim.play_sfx(
            "fighter_skill2",
            caster_id,
            &InputTargetV1::target(caster_id),
        );
        wlog(format!(
            "{} CABLE CAST -> target #{} (champ={}, monster={}) ({:.0},{:.0}), anchor ({:.0},{:.0}) -> swing ({:.0},{:.0}), {:.0} units, {flight_ticks} ticks, {draw_ticks} line frames, chain={chain_active}, escape={escaping}, start_fx={start_effect}, impact_queued={impact_queued}",
            sim_head(sim, caster_id),
            target.id,
            target.champion,
            target.monster,
            target.pos.0,
            target.pos.1,
            wall_anchor.0,
            wall_anchor.1,
            destination.x,
            destination.y,
            destination.distance
        ));
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (95 + caster_stat.attack * 135 / 100, 0)
    }

    fn expected_move_distance(&self) -> Option<(usize, u64)> {
        Some((28, 165_000))
    }

    fn can_move(&self) -> bool {
        true
    }

    fn linear_move_speed(&self) -> Option<usize> {
        Some(CABLE_SPEED as usize)
    }
}

struct CableImpact;

impl StableEffectType for CableImpact {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Pos.code() {
            return;
        }
        let impact = sim.play_view_effect("mlbb_fanny_cable_impact", caster_id, &input, 0, 0, 0);
        let hook = sim.play_view_effect("mlbb_fanny_cable_hook", caster_id, &input, 0, 0, 0);
        wlog(format!(
            "{} CABLE IMPACT at ({},{}) spark={impact}, hook={hook}",
            sim_head(sim, caster_id),
            input.x,
            input.y
        ));
    }
}

fn play_tornado(sim: &mut StableSim<'_>, caster_id: usize, at: (f64, f64), team: usize, speed: f64) {
    let all = units(sim);
    let attack = sim
        .get_entity(caster_id)
        .map_or(0, |entity| entity.stat().attack);
    sim.play_view_effect(
        "mlbb_fanny_tornado",
        caster_id,
        &InputTargetV1::target(caster_id),
        0,
        FLIGHT_TORNADO_RADIUS as u64,
        0,
    );
    sim.play_sfx(
        "fighter_skill_hit",
        caster_id,
        &InputTargetV1::target(caster_id),
    );
    let base_damage = FLIGHT_TORNADO_FLAT + attack * FLIGHT_TORNADO_AD / 100;
    let speed_bonus_pct = (15 + (speed / 350.0) as usize).min(30);
    let targets: Vec<(usize, bool)> = all
        .iter()
        .filter(|unit| {
            unit.alive
                && unit.targetable
                && !unit.tower
                && unit.team != team
                && distance(at, unit.pos) <= FLIGHT_TORNADO_RADIUS + unit.radius
        })
        .map(|unit| (unit.id, unit.champion))
        .collect();
    let mut refunded_this_spin = false;
    for (target_id, is_champ) in targets {
        let marks = if is_champ { mark_level(sim, target_id) } else { 0 };
        let mark_bonus_pct = marks * 15;
        let total_damage = base_damage.saturating_mul(100 + speed_bonus_pct + mark_bonus_pct) / 100;
        sim.deal_damage(caster_id, target_id, total_damage, 0, AttackTypeV1::Skill);
        if is_champ && !refunded_this_spin {
            refunded_this_spin = true;
            refund_cable_charge_and_heal(sim, caster_id, 1);
        }
        if is_champ && apply_or_upgrade_prey_mark(sim, target_id) {
            let mark_effect = if mark_level(sim, target_id) >= 2 {
                "mlbb_fanny_prey_mark_2"
            } else {
                "mlbb_fanny_prey_mark_1"
            };
            let _ = sim.play_view_effect(
                mark_effect,
                caster_id,
                &InputTargetV1::target(target_id),
                0,
                0,
                0,
            );
        }
    }
}

#[derive(Clone, Default)]
struct AirSuperiority {
    last_pos: Option<(f64, f64)>,
    last_dir: Option<(f64, f64)>,
    was_flying: bool,
    last_speed: f64,
    last_tornado_tick: Option<usize>,
    last_regen_tick: usize,
    tornado_targets_this_flight: HashSet<usize>,
}

impl StablePassive for AirSuperiority {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        sim.entity_remove_buff(entity_id, CABLE_CHAIN);
        sim.entity_remove_buff(entity_id, CABLE_ESCAPE);
        sim.entity_remove_buff(entity_id, CABLE_RECHARGE_LOCK);
        sim.entity_remove_buff(entity_id, FLYING);
        set_cable_charges(sim, entity_id, BASE_CABLE_CHARGES);
        self.last_pos = sim.get_entity(entity_id).map(|entity| {
            let (x, y) = entity.pos();
            (x as f64, y as f64)
        });
        self.was_flying = false;
        self.last_speed = 0.0;
        self.last_tornado_tick = None;
        self.last_regen_tick = sim.tick();
        self.tornado_targets_this_flight.clear();
    }

    fn on_attack(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity_id: usize,
        target_id: usize,
        damage: &mut usize,
    ) {
        if !has_named_buff(sim, entity_id, FLYING) && !has_named_buff(sim, entity_id, CABLE_CHAIN) {
            return;
        }
        let bonus_percent = (10 + (self.last_speed / 500.0) as usize).min(25);
        *damage = damage.saturating_mul(100 + bonus_percent) / 100;

        let is_enemy_champion = sim.get_entity(target_id).is_some_and(|target| {
            target.is_alive()
                && target.is_champion()
                && target.is_targetable()
                && target.team()
                    != sim
                        .get_entity(entity_id)
                        .map_or(target.team(), |owner| owner.team())
        });
        if is_enemy_champion {
            refund_cable_charge_and_heal(sim, entity_id, 1);
        }
        if is_enemy_champion && apply_or_upgrade_prey_mark(sim, target_id) {
            let mark_effect = if mark_level(sim, target_id) >= 2 {
                "mlbb_fanny_prey_mark_2"
            } else {
                "mlbb_fanny_prey_mark_1"
            };
            let _ = sim.play_view_effect(
                mark_effect,
                entity_id,
                &InputTargetV1::target(target_id),
                0,
                0,
                0,
            );
        }
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _seed: u64, _player: usize, entity_id: usize) {
        let tick = sim.tick();
        let Some((here, team, alive)) = sim.get_entity(entity_id).map(|entity| {
            let (x, y) = entity.pos();
            ((x as f64, y as f64), entity.team(), entity.is_alive())
        }) else {
            return;
        };
        if !alive {
            self.was_flying = false;
            self.last_pos = Some(here);
            return;
        }

        let flying = has_named_buff(sim, entity_id, FLYING);
        if flying && !self.was_flying {
            self.tornado_targets_this_flight.clear();
            self.last_dir = None;
            self.last_pos = Some(here);
        }
        if flying {
            let from = self.last_pos.unwrap_or(here);
            let speed = distance(from, here);
            if speed > 0.0 {
                let dir = ((here.0 - from.0) / speed, (here.1 - from.1) / speed);
                if let Some(prev_dir) = self.last_dir {
                    let dot = prev_dir.0 * dir.0 + prev_dir.1 * dir.1;
                    // New cable hop changed flight angle -> allow Tornado Strike to hit targets again on the new swing!
                    if dot < 0.94 {
                        self.tornado_targets_this_flight.clear();
                    }
                }
                self.last_dir = Some(dir);
                self.last_speed = speed;
                let all = units(sim);
                let newly_crossed = all.iter().find(|unit| {
                    unit.id != entity_id
                        && unit.alive
                        && unit.targetable
                        && !unit.tower
                        && unit.team != team
                        && !self.tornado_targets_this_flight.contains(&unit.id)
                        && segment_distance(unit.pos, from, here) <= unit.radius + TARGET_PASS_BUFFER
                });
                if let Some(target) = newly_crossed {
                    let target_id = target.id;
                    self.tornado_targets_this_flight.insert(target_id);
                    let cooldown_ready = self
                        .last_tornado_tick
                        .is_none_or(|last| tick.saturating_sub(last) >= FLIGHT_TORNADO_COOLDOWN);
                    if cooldown_ready {
                        play_tornado(sim, entity_id, here, team, speed);
                        self.last_tornado_tick = Some(tick);
                    }
                }
            }
            self.last_regen_tick = tick;
        } else {
            if self.was_flying {
                self.last_speed = 0.0;
                self.last_dir = None;
                self.last_regen_tick = tick;
            }
            let current_charges = get_cable_charges(sim, entity_id);
            if current_charges < BASE_CABLE_CHARGES {
                if tick.saturating_sub(self.last_regen_tick) >= CABLE_REGEN_TICKS {
                    let next_charges = (current_charges + 1).min(BASE_CABLE_CHARGES);
                    set_cable_charges(sim, entity_id, next_charges);
                    self.last_regen_tick = tick;
                    if next_charges >= MIN_CHARGES_TO_START_CHAIN {
                        sim.entity_remove_buff(entity_id, CABLE_RECHARGE_LOCK);
                    }
                }
            } else {
                self.last_regen_tick = tick;
                sim.entity_remove_buff(entity_id, CABLE_RECHARGE_LOCK);
            }
        }
        self.last_pos = Some(here);
        self.was_flying = flying;
    }
}

#[derive(Clone, Default)]
struct FannyCableAi {
    last_log_tick: Option<usize>,
}

fn cable_action_ready(
    caster_level: usize,
    skill2_cooldown: Option<usize>,
    has_route: bool,
) -> bool {
    caster_level >= 3 && skill2_cooldown == Some(0) && has_route
}

fn within_cable_approach_range(distance: f64) -> bool {
    distance <= SKILL2_RANGE
}

impl FannyCableAi {
    fn log_once_in_a_while(&mut self, tick: usize, message: impl AsRef<str>) {
        if self
            .last_log_tick
            .is_none_or(|last| tick >= last.saturating_add(LOG_EVERY))
        {
            self.last_log_tick = Some(tick);
            wlog(message);
        }
    }
}

impl StablePlayerAi for FannyCableAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(self.clone())
    }

    fn id(&self) -> String {
        "mlbb_fanny_cable_chain_ai".to_string()
    }

    fn priority(&self) -> i32 {
        100
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name.to_ascii_lowercase().contains("fanny")
    }

    fn think(
        &mut self,
        ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        let athlete_id = ctx.athlete_id();
        let player_id = ctx.player_id();
        let tick = ctx.tick();
        let base_wants_cable =
            base_input.is_some_and(|input| input.kind == InputKindV1::Skill2.code());
        let base_wants_ult =
            base_input.is_some_and(|input| input.kind == InputKindV1::Ult.code());
        let preferred = base_input
            .filter(|input| input.target.kind == InputTargetKindV1::Target.code())
            .map(|input| input.target.target_id);

        let (
            caster_pos,
            caster_level,
            cable_charges,
            recharge_locked,
            target_id,
            target_pos,
            target_is_champ,
            route,
            fallback_target,
            skill1_cooldown,
            skill2_cooldown,
            ult_cooldown,
            ult_target,
            chain_active,
            flying,
            escaping,
            line,
        ) = {
            let sim = ctx.sim()?;
            let all = units(&sim);
            let player = sim.get_player(player_id)?;
            // Official 0.6.4 pattern: resolve controlled champion entity via player.champion()
            // first, falling back to athlete_id only if needed.
            let caster_id = player
                .champion()
                .map(|champ| champ.id())
                .unwrap_or(athlete_id);
            let caster = all
                .iter()
                .find(|unit| unit.id == caster_id && unit.alive)?;
            let skill1_cooldown = player.cooldowns().map(|cooldowns| cooldowns.1);
            let skill2_cooldown = player.cooldowns().map(|cooldowns| cooldowns.2);
            let ult_cooldown = player.cooldowns().map(|cooldowns| cooldowns.3);
            let player_level = (player.level() as usize).max(caster.level);
            let cable_charges = caster.stack;
            let recharge_locked = has_named_buff(&sim, caster.id, CABLE_RECHARGE_LOCK);
            let chain_active = has_named_buff(&sim, caster.id, CABLE_CHAIN);
            let flying = has_named_buff(&sim, caster.id, FLYING);
            let escaping = (chain_active && has_named_buff(&sim, caster.id, CABLE_ESCAPE))
                || dangerous_escape(&all, caster);

            // Smart Cut Throat (Ult) target selection:
            // When Level >= 5 and Ult is off cooldown, look for a visible enemy champion within ULT_RANGE
            // who has Prey Mark stacks, is <= 65% HP, or is in range right after a cable swing (`chain_active`).
            let ult_target = if player_level >= 5 && ult_cooldown == Some(0) && !escaping {
                all.iter()
                    .filter(|unit| {
                        unit.id != caster.id
                            && unit.team != caster.team
                            && unit.alive
                            && unit.targetable
                            && unit.champion
                            && is_unit_visible(&sim, caster.team, unit)
                            && distance(caster.pos, unit.pos) <= ULT_RANGE
                    })
                    .filter_map(|unit| {
                        let marks = mark_level(&sim, unit.id);
                        let low_hp = unit.hp.saturating_mul(100) <= unit.max_hp.max(1).saturating_mul(65);
                        if marks > 0 || low_hp || chain_active || base_wants_ult {
                            let hp_pct = unit.hp.saturating_mul(100) / unit.max_hp.max(1);
                            Some((unit.id, marks, hp_pct, distance(caster.pos, unit.pos)))
                        } else {
                            None
                        }
                    })
                    .max_by(|a, b| {
                        a.1.cmp(&b.1)
                            .then_with(|| b.2.cmp(&a.2))
                            .then_with(|| b.3.total_cmp(&a.3))
                    })
                    .map(|(uid, marks, hp_pct, _)| (uid, marks, hp_pct))
            } else {
                None
            };

            let champ_in_range = |unit: &&UnitSnap| {
                unit.id != caster.id
                    && unit.team != caster.team
                    && unit.alive
                    && unit.targetable
                    && unit.champion
                    && is_unit_visible(&sim, caster.team, unit)
                    && within_cable_approach_range(distance(caster.pos, unit.pos))
            };
            let monster_in_range = |unit: &&UnitSnap| {
                unit.id != caster.id
                    && unit.team != caster.team
                    && unit.alive
                    && unit.targetable
                    && unit.monster
                    && is_unit_visible(&sim, caster.team, unit)
                    && distance(caster.pos, unit.pos) <= JUNGLE_CABLE_RANGE
            };

            // Target selection:
            // 1. Visible enemy Champion (preferred target or nearest).
            // 2. Visible Jungle Monster ONLY if zero enemy champions are visible and not escaping.
            let target = if escaping {
                all.iter().filter(champ_in_range).min_by(|a, b| {
                    distance(caster.pos, a.pos).total_cmp(&distance(caster.pos, b.pos))
                })
            } else {
                preferred
                    .and_then(|id| all.iter().find(|unit| unit.id == id).filter(champ_in_range))
                    .or_else(|| {
                        all.iter().filter(champ_in_range).min_by(|a, b| {
                            distance(caster.pos, a.pos).total_cmp(&distance(caster.pos, b.pos))
                        })
                    })
                    .or_else(|| {
                        preferred
                            .and_then(|id| {
                                all.iter()
                                    .find(|unit| unit.id == id)
                                    .filter(monster_in_range)
                            })
                            .or_else(|| {
                                all.iter().filter(monster_in_range).min_by(|a, b| {
                                    distance(caster.pos, a.pos)
                                        .total_cmp(&distance(caster.pos, b.pos))
                                })
                            })
                    })
            };

            let route =
                target.and_then(|target| choose_cable_route(&all, caster, target, escaping));
            let fallback_target = target.map(|unit| (unit.id, unit.pos));
            (
                caster.pos,
                player_level,
                cable_charges,
                recharge_locked,
                target.map(|unit| unit.id),
                target.map(|unit| unit.pos),
                target.is_some_and(|unit| unit.champion),
                route,
                fallback_target,
                skill1_cooldown,
                skill2_cooldown,
                ult_cooldown,
                ult_target,
                chain_active,
                flying,
                escaping,
                sim_head(&sim, caster.id),
            )
        };

        // Priority 0: Execute Cut Throat (Ult) when unlocked, off cooldown, not mid-cable-flight,
        // and either a smart Ult target (Prey Mark / low HP / post-cable combo) is in range or base AI requested Ult.
        if !flying && caster_level >= 5 && ult_cooldown == Some(0) {
            if let Some((ult_tid, marks, hp_pct)) = ult_target {
                let ult_input = InputV1::action(InputKindV1::Ult, InputTargetV1::target(ult_tid));
                self.log_once_in_a_while(
                    tick,
                    format!(
                        "{line} AI ULT INPUT (Cut Throat): target #{ult_tid} (marks={marks}, hp={hp_pct}%), chain={chain_active}"
                    ),
                );
                return Some(ult_input);
            }
            if base_wants_ult {
                return base_input;
            }
        }

        let cooldown_ready = skill2_cooldown == Some(0);
        // Require >= 2 charges to start a fresh chain from the ground, or >= 1 charge to continue an active chain!
        let charges_ready = if chain_active && !recharge_locked {
            cable_charges >= 1
        } else {
            !recharge_locked && cable_charges >= MIN_CHARGES_TO_START_CHAIN
        };
        // Wait for the current cable flight (`flying`) to reach its swing-past landing point
        // before firing the next cable so long cables (>18 ticks) are never cut short mid-flight.
        let can_attempt = !flying
            && charges_ready
            && cable_action_ready(caster_level, skill2_cooldown, route.is_some());
        if can_attempt {
            if let (Some(target_id), Some(route)) = (target_id, route) {
                let candidate =
                    InputV1::action(InputKindV1::Skill2, InputTargetV1::target(target_id));
                let prevalidated = ctx.is_valid_input(&candidate);
                let input_status = if prevalidated {
                    "prevalidate=true"
                } else {
                    "prevalidate=false, overriding after local gates"
                };
                self.log_once_in_a_while(
                    tick,
                    format!(
                        "{line} AI CABLE INPUT: target #{target_id} (champ={target_is_champ}) at ({:.0},{:.0}), anchor ({:.0},{:.0}) -> swing ({:.0},{:.0}) dist {:.0}, cooldown={skill2_cooldown:?}, chain={chain_active}, escape={escaping}, {input_status}",
                        target_pos.map_or(0.0, |p| p.0),
                        target_pos.map_or(0.0, |p| p.1),
                        route.anchor_x,
                        route.anchor_y,
                        route.x,
                        route.y,
                        route.distance
                    ),
                );
                return Some(candidate);
            }
        }

        // Keep Fanny moving forward along her multi-cable chain during the brief
        // cooldown window between cables while she still has charges!
        if chain_active && charges_ready && !cooldown_ready && !flying {
            if let Some(route) = route {
                return Some(InputV1::move_to(world_u64(route.x), world_u64(route.y)));
            }
            if !escaping {
                if let Some((_tid, tpos)) = fallback_target {
                    if distance(caster_pos, tpos) > 22_000.0 {
                        return Some(InputV1::move_to(world_u64(tpos.0), world_u64(tpos.1)));
                    }
                }
            }
        }

        if base_wants_cable {
            let reason = if caster_level < 3 {
                format!("skill2 locked at level {caster_level}")
            } else if !charges_ready {
                format!("recharging cable charges ({cable_charges}/{BASE_CABLE_CHARGES})")
            } else if !cooldown_ready {
                format!("skill2 cooldown {:?}", skill2_cooldown)
            } else if target_id.is_none() {
                "no visible enemy champion or jungle monster found".to_string()
            } else if route.is_none() {
                "no valid wall route; repositioning toward target".to_string()
            } else {
                "TFM2 rejected the planned cable input".to_string()
            };
            self.log_once_in_a_while(tick, format!("{line} AI HOLD: {reason}"));
            if let Some((tid, tpos)) = fallback_target {
                let d = distance(caster_pos, tpos);
                // While recharging cable charges, use Ground Skill 1 (Tornado Strike) or Basic Attack in melee range!
                if !flying && skill1_cooldown == Some(0) && d <= 36_000.0 {
                    return Some(InputV1::action(InputKindV1::Skill, InputTargetV1::target(tid)));
                }
                if !flying && d <= 24_000.0 {
                    return Some(InputV1::action(InputKindV1::Attack, InputTargetV1::target(tid)));
                }
                return Some(InputV1::move_to(
                    world_u64(tpos.0),
                    world_u64(tpos.1),
                ));
            }
            return Some(InputV1::move_to(
                world_u64(caster_pos.0),
                world_u64(caster_pos.1),
            ));
        }
        None
    }
}

struct CutThroat;

impl StableEffectType for CutThroat {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let all = units(sim);
        let Some(caster) = all.iter().find(|unit| unit.id == caster_id && unit.alive) else {
            return;
        };
        let Some(target) = choose_target(sim, &all, caster, input) else {
            return;
        };
        let target_distance = distance(caster.pos, target.pos);
        // Generous leash so targets moving slightly during cast windup never fizzle the Ult
        if target_distance > ULT_RANGE * 1.6 {
            return;
        }
        let grid = current_grid(&all);
        sim.entity_remove_buff(caster_id, FLYING);
        let travel_distance = (target_distance - ULT_CONTACT).max(0.0);
        let raw_dest = if target_distance > 1.0 {
            (
                caster.pos.0 + (target.pos.0 - caster.pos.0) / target_distance * travel_distance,
                caster.pos.1 + (target.pos.1 - caster.pos.1) / target_distance * travel_distance,
            )
        } else {
            target.pos
        };
        let dest = if grid.solid(raw_dest.0, raw_dest.1) { target.pos } else { raw_dest };
        let ticks = if travel_distance > 1_000.0 {
            force_move(sim, caster_id, caster.pos, dest, ULT_SPEED)
        } else {
            0
        };
        let target_input = InputTargetV1::target(target.id);
        let _ = sim.play_view_effect("mlbb_fanny_cut_throat", caster_id, &target_input, 0, 0, 0);
        if ticks == 0 {
            cut_throat_hit(sim, caster_id, target.id);
        } else {
            sim.queue_effect(
                "mlbb_heroes:cut_throat_hit",
                AttackTypeV1::Skill,
                caster_id,
                &target_input,
                ticks,
            );
        }
        sim.play_sfx("fighter_ult", caster_id, &target_input);
        wlog(format!(
            "{} CUT THROAT CAST on target #{} dist={:.0} ticks={}",
            sim_head(sim, caster_id),
            target.id,
            target_distance,
            ticks
        ));
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (
            ULT_FLAT_DAMAGE + caster_stat.attack * (ULT_BASE_AD_RATIO + ULT_MARK_AD_RATIO) / 100,
            0,
        )
    }

    fn expected_move_distance(&self) -> Option<(usize, u64)> {
        Some((8, 52_000))
    }

    fn can_move(&self) -> bool {
        true
    }
}

struct CutThroatHit;

impl StableEffectType for CutThroatHit {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        cut_throat_hit(sim, caster_id, input.target_id);
    }
}

fn cut_throat_hit(sim: &mut StableSim<'_>, caster_id: usize, target_id: usize) {
    let Some(caster) = sim.get_entity(caster_id) else {
        return;
    };
    let Some(target) = sim.get_entity(target_id) else {
        return;
    };
    if !caster.is_alive() || !target.is_alive() || !target.is_targetable() {
        return;
    }
    let (cx, cy) = caster.pos();
    let (tx, ty) = target.pos();
    let dist = distance((cx as f64, cy as f64), (tx as f64, ty as f64));
    if dist > ULT_RANGE * 1.6 + target.radius() as f64 {
        return;
    }
    let attack = caster.stat().attack;
    let prey_stacks = mark_level(sim, target_id).min(2);
    let damage =
        ULT_FLAT_DAMAGE + attack * (ULT_BASE_AD_RATIO + prey_stacks * ULT_MARK_AD_RATIO) / 100;
    let target_input = InputTargetV1::target(target_id);
    let _ = sim.play_view_effect("mlbb_fanny_cut_throat", caster_id, &target_input, 0, 0, 0);
    sim.deal_damage(caster_id, target_id, damage, 0, AttackTypeV1::Skill);
    refund_cable_charge_and_heal(sim, caster_id, 1 + prey_stacks.min(1));
    sim.entity_remove_buff(target_id, PREY_1);
    sim.entity_remove_buff(target_id, PREY_2);
    wlog(format!(
        "{} CUT THROAT HIT target #{} stacks={} raw_dmg={}",
        sim_head(sim, caster_id),
        target_id,
        prey_stacks,
        damage
    ));
}


// ---------------------------------------------------------------------------
// MLBB Aldous — Contract: Transform & Soul Steal Companion Passive
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct AldousContractPassive {
    soul_stacks: usize,
    basic_attack_count: usize,
}

impl StablePassive for AldousContractPassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_attack(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity: usize,
        _target: usize,
        damage: &mut usize,
    ) {
        self.basic_attack_count += 1;
        if let Some(me) = sim.get_entity(entity) {
            let has_s1 = (0..me.buff_count())
                .filter_map(|i| me.buff_at(i))
                .any(|b| b.name() == "s1_ready" || b.name() == "mlbb_aldous_s1_ready");
            if has_s1 {
                *damage = damage.saturating_add(self.soul_stacks * 5);
            }
        }
        if self.basic_attack_count % 3 == 0 {
            let shield_amt = 140 + self.soul_stacks * 3;
            let _ = sim.entity_add_shield(entity, shield_amt, 180);
        }
    }

    fn on_kill(&mut self, _sim: &mut StableSim<'_>, _player: usize, _entity: usize, _victim: usize) {
        self.soul_stacks = (self.soul_stacks + 12).min(1000);
    }
}

fn init(host: &StableHost) -> StableMod {
    let _ = std::fs::rename(&*LOG_PATH, LOG_PATH.with_extension("prev.log"));
    wlog("=== MLBB Fanny wall add-on v0.2.0 loaded: 2-hop multi-wall chaining, outer lane walls, champion/jungle priority, energy-free ===");
    host.log(
        LogLevel::Info,
        "MLBB Fanny wall add-on v0.2.0 loaded: multi-hop Steel Cable chaining, outer lane walls, champion-first/jungle-second priority, Prey Marks and Cut Throat.",
    );
    let mut module = StableMod::new(MOD_ID);
    module.add_native_effect("mlbb_heroes:cable", SteelCable);
    module.add_native_effect(CABLE_IMPACT_EFFECT, CableImpact);
    module.add_native_effect(CABLE_DRAW_EFFECT, CableDraw);
    module.add_native_effect("mlbb_heroes:cut_throat", CutThroat);
    module.add_native_effect("mlbb_heroes:cut_throat_hit", CutThroatHit);
    module.add_native_passive("mlbb_heroes:air_superiority", AirSuperiority::default());
    module.add_native_passive("mlbb_heroes:contract_transform", AldousContractPassive::default());
    module.add_native_passive("mlbb_aldous:contract_transform", AldousContractPassive::default());
    module.add_native_effect("mlbb_fanny_wall:cable", SteelCable);
    module.add_native_effect("mlbb_fanny_wall:cable_impact", CableImpact);
    module.add_native_effect("mlbb_fanny_wall:cable_draw", CableDraw);
    module.add_native_effect("mlbb_fanny_wall:cut_throat", CutThroat);
    module.add_native_effect("mlbb_fanny_wall:cut_throat_hit", CutThroatHit);
    module.add_native_passive("mlbb_fanny_wall:air_superiority", AirSuperiority::default());
    module.add_player_input_ai(FannyCableAi::default());
    module.set_map_customizer(FannyMapReader);
    module
}

declare_stable_mod!(init, requires = 9);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_cable_input_range_covers_the_standard_map_diagonal() {
        let map_width = MOBA_BOUNDS.2 - MOBA_BOUNDS.0;
        let map_height = MOBA_BOUNDS.3 - MOBA_BOUNDS.1;
        assert!(SKILL2_RANGE >= map_width.hypot(map_height));
        assert!(within_cable_approach_range(SKILL2_RANGE));
        assert!(!within_cable_approach_range(SKILL2_RANGE + 1.0));
    }

    #[test]
    fn cable_ai_requires_level3_cooldown_and_a_wall_route_without_energy() {
        assert!(cable_action_ready(3, Some(0), true));
        assert!(cable_action_ready(5, Some(0), true));
        assert!(!cable_action_ready(2, Some(0), true));
        assert!(!cable_action_ready(3, Some(1), true));
        assert!(!cable_action_ready(3, Some(0), false));
        assert!(!cable_action_ready(3, None, true));
    }

    #[test]
    fn parses_the_collision_grid() {
        let grid = parse_int_grid("[[0, 1], [2, 0]]").expect("valid grid");
        assert_eq!(grid.cells, vec![vec![0, 1], vec![2, 0]]);
        assert_eq!(grid.wall_count(), 2);
        assert!(parse_int_grid("[[0, bad]]").is_none());
    }

    #[test]
    fn cable_ray_stops_before_the_first_wall() {
        let grid = Grid::from_rows(&[
            "........", "........", "...#....", "...#....", "...#....", "...#....", "........",
        ]);
        let origin = (1.5 * CELL, 3.5 * CELL);
        let east = wall_hits(&grid, origin, 60_000.0)
            .into_iter()
            .find(|hit| hit.x > origin.0 && (hit.y - origin.1).abs() < 1_000.0)
            .expect("east wall");
        assert!(east.x < 3.0 * CELL && east.x > 3.0 * CELL - 2_500.0);
        assert!(east.distance < 60_000.0);
        assert!(!grid.is_wall(east.x, east.y));
    }

    #[test]
    fn multi_cable_chain_reaches_distant_target_across_moba_walls() {
        let grid = Grid::builtin(GameModeKindV1::Moba);
        let mut pos = (180_000.0, 780_000.0);
        let target = (780_000.0, 180_000.0);
        let initial_dist = distance(pos, target);
        let mut hops = 0;
        for _ in 0..4 {
            let hits = wall_hits(&grid, pos, CABLE_RANGE);
            let Some(next) = choose_engage_hit_with_grid(
                Some(&grid),
                &hits,
                pos,
                target,
                10_000.0,
                &[],
            ) else {
                break;
            };
            pos = next.pos();
            hops += 1;
        }
        assert!(hops >= 3, "expected at least 3 chained wall cables, got {hops}");
        assert!(distance(pos, target) < initial_dist * 0.45);
    }

    #[test]
    fn outer_lane_boundary_allows_cabling_along_top_and_bottom_lanes() {
        let grid = Grid::builtin(GameModeKindV1::Moba);
        let top_lane_pos = (480_000.0, 112_000.0);
        let top_lane_target = (720_000.0, 112_000.0);
        let hits = wall_hits(&grid, top_lane_pos, CABLE_RANGE);
        let best = choose_engage_hit_with_grid(
            Some(&grid),
            &hits,
            top_lane_pos,
            top_lane_target,
            10_000.0,
            &[],
        )
        .expect("outer boundary wall cable in top lane");
        assert!(best.x > top_lane_pos.0 + 40_000.0);
    }

    #[test]
    fn engage_route_progresses_toward_the_ai_target() {
        let here = (160_000.0, 160_000.0);
        let target = (260_000.0, 160_000.0);
        let hits = vec![
            WallHit {
                x: 190_000.0,
                y: 160_000.0,
                distance: 30_000.0,
                anchor_x: 190_000.0,
                anchor_y: 160_000.0,
            },
            WallHit {
                x: 160_000.0,
                y: 210_000.0,
                distance: 50_000.0,
                anchor_x: 160_000.0,
                anchor_y: 210_000.0,
            },
            WallHit {
                x: 120_000.0,
                y: 160_000.0,
                distance: 40_000.0,
                anchor_x: 120_000.0,
                anchor_y: 160_000.0,
            },
        ];
        let result = choose_engage_hit(&hits, here, target, 10_000.0, &[]).expect("forward hook");
        assert_eq!(result, hits[0]);
    }

    #[test]
    fn engage_route_prefers_a_cable_that_passes_through_the_target() {
        let here = (160_000.0, 160_000.0);
        let target = (220_000.0, 160_000.0);
        let hits = vec![
            WallHit {
                x: 185_000.0,
                y: 160_000.0,
                distance: 25_000.0,
                anchor_x: 185_000.0,
                anchor_y: 160_000.0,
            },
            WallHit {
                x: 245_000.0,
                y: 160_000.0,
                distance: 85_000.0,
                anchor_x: 245_000.0,
                anchor_y: 160_000.0,
            },
        ];
        let result =
            choose_engage_hit(&hits, here, target, 10_000.0, &[]).expect("target-crossing hook");
        assert_eq!(result, hits[1]);
    }

    #[test]
    fn escape_route_increases_the_nearest_enemy_distance() {
        let here = (160_000.0, 160_000.0);
        let enemies = [UnitSnap {
            id: 5,
            pos: (160_000.0, 120_000.0),
            team: 1,
            alive: true,
            targetable: true,
            champion: true,
            tower: false,
            monster: false,
            level: 5,
            radius: 10_000.0,
            hp: 500,
            max_hp: 1_000,
        }];
        let refs: Vec<&UnitSnap> = enemies.iter().collect();
        let hits = vec![
            WallHit {
                x: 160_000.0,
                y: 220_000.0,
                distance: 60_000.0,
                anchor_x: 160_000.0,
                anchor_y: 220_000.0,
            },
            WallHit {
                x: 220_000.0,
                y: 160_000.0,
                distance: 60_000.0,
                anchor_x: 220_000.0,
                anchor_y: 160_000.0,
            },
        ];
        let result = choose_escape_hit(&hits, here, &refs).expect("away route");
        assert!(distance(result.pos(), enemies[0].pos) > distance(here, enemies[0].pos));
    }

    #[test]
    fn path_check_rejects_crossing_a_wall() {
        let grid = Grid::from_rows(&["....", "..#.", "....", "...."]);
        assert!(!path_is_clear(
            &grid,
            (16_000.0, 48_000.0),
            (112_000.0, 48_000.0)
        ));
        assert!(path_is_clear(
            &grid,
            (16_000.0, 16_000.0),
            (48_000.0, 16_000.0)
        ));
    }
}
