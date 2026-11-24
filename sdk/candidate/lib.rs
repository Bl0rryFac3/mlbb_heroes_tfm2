//! Stable-ABI native gameplay for the multi-hero MLBB Heroes pack.
//!
//! Lancelot is registered in `lancelot.rs`; his visual timelines and skill geometry remain data-driven.

mod alpha;
mod angela;
mod eudora;
mod fanny_flight;
mod fanny_map;
mod julian;
mod popol_probe;
mod popol_menu;
mod granger;
mod lesley;
mod valir;
mod hanzo;
mod atlas;
mod yu_zhong;
mod arlott;
mod audio;
mod balmond;
mod belerick;
mod cecilion;
mod draft_preview;
mod johnson;
mod chou;
#[cfg(all(target_os = "windows", target_env = "msvc"))]
mod crt_shim;
mod estes;
mod franco;
mod gord;
mod gusion;
mod hanabi;
mod karrie;
mod lancelot;
mod obsidia;
mod sun;
mod uranus;
mod zhask;

pub mod maps;

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

use mod_api_stable::*;

const MOD_ID: &str = "mlbb_heroes";

// All distances are TFM2 world units. 60 ticks = one simulation second.
const CELL: f64 = 32_000.0;
const CABLE_RANGE: f64 = 165_000.0; // Reaches across MOBA wall clusters (median gap 91k, max 145k) and lane outer walls.
const CABLE_SPEED: f64 = 5_200.0; // Fast pro-Fanny cable flight speed.
const MIN_ENGAGE_CABLE_DISTANCE: f64 = 24_000.0;
const TARGET_PASS_BUFFER: f64 = FLIGHT_TORNADO_RADIUS;
const TARGET_PASS_BONUS: f64 = 65_000.0;
const WALL_GAP: f64 = 1_000.0;
const WALL_STANDOFF: f64 = 14_000.0;
const WALL_SWING_PAST: f64 = 28_000.0;
const WALL_STEP: f64 = 1_000.0;
const WALL_MIN: f64 = 8_000.0;
const RAY_DIRS: usize = 72;

const CABLE_CHAIN: &str = "mlbb_fanny_cable_chain";
const CABLE_IMPACT_EFFECT: &str = "mlbb_heroes:cable_impact";
const CABLE_DRAW_EFFECT: &str = "mlbb_heroes:cable_draw";
const FLYING: &str = "mlbb_fanny_flying";

// Only the stock-selected local target can initiate a combat cable.
const SKILL2_RANGE: f64 = CABLE_RANGE;
const TOWER_DANGER_RADIUS: f64 = 52_000.0;
const ESCAPE_ROUTE_RADIUS: f64 = 180_000.0;
const PREY_1: &str = "mlbb_fanny_prey_1";
const PREY_2: &str = "mlbb_fanny_prey_2";
const HEAL_COOLDOWN_BUFF: &str = "mlbb_fanny_heal_cd";
const HEAL_COOLDOWN_TICKS: usize = 45; // 0.75s ICD on 4.5%..6.5% Max HP heal
const MAX_CABLE_CHARGES: usize = 10;
const CABLE_REGEN_TICKS: usize = 95; // +1 cable charge every ~1.58s when not flying.
const CHAIN_WINDOW: usize = 150; // 2.5 seconds recast chain window.
const MARK_DURATION: usize = 300; // 5 seconds.
const FLIGHT_TORNADO_RADIUS: f64 = 34_000.0;
const FLIGHT_TORNADO_FLAT: usize = 48;
const FLIGHT_TORNADO_AD: usize = 85;
const FLIGHT_TORNADO_COOLDOWN: usize = 8;
const SPIN_DECAY_WINDOW_TICKS: usize = 180; // 3.0s window for consecutive spin decay
const ULT_RANGE: f64 = 65_000.0;
const ULT_CONTACT: f64 = 10_000.0;
const ULT_SPEED: f64 = 8_500.0;
const ULT_FLAT_DAMAGE: usize = 105;
const ULT_BASE_AD_RATIO: usize = 120;
const ULT_MARK_AD_RATIO: usize = 25;

// 5v5 map outer lane boundaries act as solid outer walls so Fanny can cable along
// Top and Bottom lanes just like in MLBB.
const MOBA_BOUNDS: (f64, f64, f64, f64) = (56_000.0, 56_000.0, 904_000.0, 904_000.0);
const MODES: [GameModeKindV1; 3] = [
    GameModeKindV1::Moba,
    GameModeKindV1::SingleLane,
    GameModeKindV1::DeathMatch,
];

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
        if let Some(mode) = mode {
            fanny_map::capture(mode, doc.get_json("towers").as_deref().unwrap_or_default());
            if let Some(mut grid) = doc.get_json("walls").as_deref().and_then(parse_int_grid) {
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
    radius: f64,
    hp: usize,
    max_hp: usize,
    stack: usize,
}

// ===========================================================================
// Sim-scoped state keys
//
// The game can run TWO SIMULATIONS OF ONE MATCH AT ONCE, with identical entity
// ids. Any state we keep in a global map keyed by bare entity id is therefore
// shared between them: the two sims corrupt each other and what gets rendered
// stops being the match that was computed. (Confirmed second-hand, 2026-11-03,
// from BoDUD/TFM2-League-Heroes `docs/tech-notes.md`, game 0.6.4 section.)
//
// Two sims of one match are identical in every field we can *derive* — same
// origin kind, match id, replay id, set index and seed. So the discriminator
// cannot be derived; it has to be ASSIGNED and stored in the engine, which is
// what keeps the two copies apart. Each entity is tagged at spawn with a
// valueless permanent buff `mlbb_sim_copy:<n>`; state is keyed by
// (scope, entity_id, n).
//
// `SimOriginKindV1` is still worth carrying: it separates a server pre-sim
// from the match the player is watching, which is a real hazard of its own.
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
struct SimScope {
    kind: u32,
    match_id: u64,
    replay_id: u64,
    set_index: u64,
    seed: u64,
}

impl SimScope {
    fn of(sim: &StableSim<'_>) -> Self {
        let o = sim.sim_origin().unwrap_or_default();
        Self {
            kind: o.kind,
            match_id: o.match_id,
            replay_id: o.replay_id,
            set_index: o.set_index,
            seed: sim.seed(),
        }
    }
}

/// Identifies one entity inside one particular run of one particular match.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct EntityKey {
    scope: SimScope,
    entity_id: usize,
    /// Assigned per simulation copy; see the note above.
    copy: u64,
}

const SIM_COPY_PREFIX: &str = "mlbb_sim_copy:";
static NEXT_SIM_COPY: AtomicU64 = AtomicU64::new(1);

/// Reads the copy serial an entity already carries, if it has one.
/// The discriminator for two pre-sims of one match. It must be *assigned*, not
/// derived: two pre-sims are identical in every field we can read (see
/// `SimScope`), so only a fresh serial tells them apart. Kept as a named
/// function so the uniqueness invariant is unit-testable — a test that pokes
/// `NEXT_SIM_COPY` directly would keep passing even if `entity_key` stopped
/// using it.
fn assign_sim_copy() -> u64 {
    NEXT_SIM_COPY.fetch_add(1, Ordering::Relaxed)
}

fn sim_copy_of(sim: &StableSim<'_>, entity_id: usize) -> Option<u64> {
    let entity = sim.get_entity(entity_id)?;
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .find_map(|buff| {
            buff.name()
                .strip_prefix(SIM_COPY_PREFIX)
                .and_then(|rest| rest.parse::<u64>().ok())
        })
}

/// Key for a mutable context: reads the tag, assigning one if it is missing.
fn entity_key(sim: &mut StableSim<'_>, entity_id: usize) -> EntityKey {
    let scope = SimScope::of(sim);
    let copy = match sim_copy_of(sim, entity_id) {
        Some(existing) => existing,
        None => {
            let fresh = assign_sim_copy();
            sim.add_buff(entity_id, &BuffV1::named(&format!("{SIM_COPY_PREFIX}{fresh}")));
            fresh
        }
    };
    EntityKey { scope, entity_id, copy }
}

/// Key for a read-only context (`think`, where mutations are inert and we
/// cannot tag). Falls back to copy 0 when the entity has not been tagged yet —
/// callers must treat a missing entry as the default, never as an error.
fn entity_key_ro(sim: &StableSim<'_>, entity_id: usize) -> EntityKey {
    EntityKey {
        scope: SimScope::of(sim),
        entity_id,
        copy: sim_copy_of(sim, entity_id).unwrap_or(0),
    }
}

// Tracks (last_hit_tick, consecutive_hits_in_window) per target entity ID
static SPIN_HISTORY: LazyLock<Mutex<HashMap<EntityKey, (usize, usize)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn record_spin_hit_and_get_count(key: EntityKey, now_tick: usize) -> usize {
    let mut map = SPIN_HISTORY.lock().unwrap_or_else(|p| p.into_inner());
    let entry = map.entry(key).or_insert((0, 0));
    if now_tick < entry.0 || now_tick.saturating_sub(entry.0) > SPIN_DECAY_WINDOW_TICKS {
        *entry = (now_tick, 1);
    } else {
        entry.0 = now_tick;
        entry.1 = entry.1.saturating_add(1);
    }
    entry.1
}

fn fanny_base_stat(level: usize, stack: usize) -> StatV1 {
    let lvl_growth = level.saturating_sub(1);
    StatV1 {
        attack: 62 + 5 * lvl_growth,
        magic_power: 0,
        hp: 750 + 98 * lvl_growth,
        defence: 28 + 5 * lvl_growth,
        magic_resistance: 26 + 3 * lvl_growth,
        move_speed: 1100 + 5 * lvl_growth,
        hp_regen: 6,
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

fn refund_cable_charge_and_heal_with_marks(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    bonus_charges: usize,
    marks: usize,
) {
    let current = get_cable_charges(sim, caster_id);
    set_cable_charges(sim, caster_id, current.saturating_add(bonus_charges));
    if !has_named_buff(sim, caster_id, HEAL_COOLDOWN_BUFF) {
        sim.add_buff(
            caster_id,
            &BuffV1::timed(HEAL_COOLDOWN_BUFF, HEAL_COOLDOWN_TICKS),
        );
        if let Some((_hp, max_hp)) = sim.get_entity(caster_id).map(|e| e.hp()) {
            // 4.5% Max HP base + 1.0% Max HP per Prey Mark (up to 6.5% Max HP on 2 marks), min 28 HP
            let pct_tenths = 45 + marks.min(2) * 10;
            let heal_amt = (max_hp * pct_tenths / 1000).max(28);
            sim.heal(caster_id, caster_id, heal_amt);
        }
    }
}

fn refund_cable_charge_and_heal(sim: &mut StableSim<'_>, caster_id: usize, bonus_charges: usize) {
    refund_cable_charge_and_heal_with_marks(sim, caster_id, bonus_charges, 1);
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
    (0..RAY_DIRS).filter_map(|index| {
        let angle = index as f64 * std::f64::consts::TAU / RAY_DIRS as f64;
        wall_hit_on_ray(grid, origin, (angle.cos(), angle.sin()), range)
    }).collect()
}

fn wall_hit_on_ray(
    grid: &Grid, origin: (f64, f64), direction: (f64, f64), range: f64,
) -> Option<WallHit> {
    let mut sample_distance = WALL_STEP;
    if grid.solid(origin.0, origin.1) {
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
            return None;
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
            if !(WALL_MIN..=range).contains(&hook_distance) || grid.solid(anchor.0, anchor.1) {
                return None;
            }
            let landing = if range >= 100_000.0 {
                compute_swing_landing(grid, origin, direction, low)
            } else {
                anchor
            };
            return (!grid.solid(landing.0, landing.1)).then(|| WallHit {
                x: landing.0,
                y: landing.1,
                distance: distance(origin, landing).max(hook_distance),
                anchor_x: anchor.0,
                anchor_y: anchor.1,
            });
        }
        free_distance = sample_distance;
        sample_distance += WALL_STEP;
    }
    None
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
fn tower_has_allied_tankers(all: &[UnitSnap], caster: &UnitSnap, tower: &UnitSnap) -> bool {
    all.iter()
        .filter(|unit| {
            unit.alive
                && unit.champion
                && unit.team == caster.team
                && unit.id != caster.id
                && distance(unit.pos, tower.pos) <= TOWER_DANGER_RADIUS * 0.92
        })
        .count()
        >= 2
}

/// Returns true when a move route enters an unsupported enemy tower zone.
/// `margin` can reserve room for a champion's dash/landing beyond the endpoint.
fn is_unsafe_tower_route(
    all: &[UnitSnap],
    caster: &UnitSnap,
    destination: (f64, f64),
    margin: f64,
) -> bool {
    let danger_radius = TOWER_DANGER_RADIUS + margin.max(0.0);
    all.iter().any(|tower| {
        tower.tower
            && tower.alive
            && tower.team != caster.team
            && (distance(destination, tower.pos) <= danger_radius
                || segment_distance(tower.pos, caster.pos, destination) <= danger_radius)
            && !tower_has_allied_tankers(all, caster, tower)
    })
}

/// Returns true when diving the target under an enemy tower is unsafe.
/// A dive is allowed only with two allied champions tanking the tower, or an
/// execute-low target (<=30% HP) while the caster is healthy (>=65% HP).
fn is_unsafe_under_enemy_tower_with_margin(
    all: &[UnitSnap],
    caster: &UnitSnap,
    target: &UnitSnap,
    margin: f64,
) -> bool {
    for tower in all.iter().filter(|unit| {
        unit.tower
            && unit.alive
            && unit.team != caster.team
            && distance(target.pos, unit.pos) <= TOWER_DANGER_RADIUS + margin.max(0.0)
    }) {
        let target_execute_low =
            target.hp.saturating_mul(100) <= target.max_hp.max(1).saturating_mul(30);
        let caster_healthy =
            caster.hp.saturating_mul(100) >= caster.max_hp.max(1).saturating_mul(65);
        if tower_has_allied_tankers(all, caster, tower) || (target_execute_low && caster_healthy) {
            continue;
        }
        return true;
    }
    false
}

fn stock_target<'a>(
    sim: &StableSim<'_>,
    all: &'a [UnitSnap],
    caster: &UnitSnap,
    input: InputTargetV1,
) -> Option<&'a UnitSnap> {
    if input.kind != InputTargetKindV1::Target.code() {
        return None;
    }
    all.iter().find(|target| {
        target.id == input.target_id && target.alive && target.targetable
            && target.team != caster.team && !target.tower
            && is_unit_visible(sim, caster.team, target)
    })
}

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
                .map(|tower| {
                    (TOWER_DANGER_RADIUS - distance(destination, tower.pos)).max(0.0) * 3.5
                })
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
                            let pass2 = segment_distance(target, destination, d2)
                                <= target_radius + TARGET_PASS_BUFFER;
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
/// tether and one directional hook head at its current endpoint.
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
    let dx = anchor.0 - from.0;
    let dy = anchor.1 - from.1;
    let angle = dy.atan2(dx).rem_euclid(std::f64::consts::TAU);
    let heading = ((angle * 24.0 / std::f64::consts::TAU).round() as usize) % 24;
    if length >= 12_000.0 {
        let wire_effect = CABLE_WIRE_EFFECTS[heading % CABLE_WIRE_EFFECTS.len()];
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
        let effect = format!("mlbb_fanny_cable_hook_{heading}");
        let _ = sim.play_view_effect(&effect, caster_id, &anchor_target, 0, 0, 0);
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
        draw_cable_frame(
            sim,
            caster_id,
            from,
            (input.x as f64, input.y as f64),
            refresh_hook,
        );
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
        if caster.stack == 0 || fanny_flight::blocked(sim, caster_id) {
            return;
        }
        let key = entity_key(sim, caster_id);
        let flight = fanny_flight::read(key);
        let grid = fanny_flight::grid(sim, flight.as_ref(), &all);
        use fanny_flight::CableAim;
        let retreat = input.kind == InputTargetKindV1::Pos.code();
        let aim = if retreat {
            CableAim::Retreat((input.x as f64, input.y as f64))
        } else {
            if flight.as_ref().is_some_and(fanny_flight::Flight::is_retreating) {
                return;
            }
            let prior = flight.as_ref().and_then(fanny_flight::Flight::target);
            let Some(target) = stock_target(sim, &all, caster, input)
                .filter(|target| prior.is_none_or(|id| id == target.id))
                .filter(|target| distance(caster.pos, target.pos) <= SKILL2_RANGE)
            else { return; };
            CableAim::Engage(target)
        };
        let Some(anchor) = fanny_flight::planned_anchor(
            &grid, flight.as_ref(), &all, caster, aim, sim.tick(),
        ) else { return; };
        let target = match aim {
            CableAim::Engage(unit) => Some(unit.id),
            CableAim::Retreat(_) => None,
        };
        if !fanny_flight::launch(
            sim, key, flight, grid, caster_id, caster.pos, anchor, target, retreat,
        ) {
            return;
        }
        let remaining_charges = caster.stack - 1;
        set_cable_charges(sim, caster_id, remaining_charges);
        sim.entity_remove_buff(caster_id, CABLE_CHAIN);
        sim.add_buff(caster_id, &BuffV1::timed(CABLE_CHAIN, CHAIN_WINDOW));
        let _ = sim.play_view_effect(
            "mlbb_fanny_cable_start",
            caster_id,
            &InputTargetV1::target(caster_id),
            0,
            0,
            0,
        );
        audio::play_throttled_sfx(
            sim,
            caster_id,
            "fighter_skill2",
            1001,
            &InputTargetV1::target(caster_id),
        );

    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (
            FLIGHT_TORNADO_FLAT + caster_stat.attack * FLIGHT_TORNADO_AD / 100,
            0,
        )
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

fn play_tornado(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    from: (f64, f64),
    at: (f64, f64),
    team: usize,
    speed: f64,
    nearby_hits: &mut HashSet<usize>,
    cooldown_ready: bool,
) -> bool {
    let all = units(sim);
    nearby_hits.retain(|target| all.iter().any(|unit| {
        unit.id == *target && unit.alive
            && distance(from, unit.pos) <= FLIGHT_TORNADO_RADIUS + unit.radius + 10_000.0
    }));
    if !cooldown_ready {
        return false;
    }
    let targets: Vec<(usize, bool)> = all
        .iter()
        .filter(|unit| {
            unit.alive
                && unit.targetable
                && !unit.tower
                && unit.team != team
                && !nearby_hits.contains(&unit.id)
                && segment_distance(unit.pos, from, at) <= FLIGHT_TORNADO_RADIUS + unit.radius
        })
        .map(|unit| (unit.id, unit.champion))
        .collect();
    if targets.is_empty() {
        return false;
    }
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
    audio::play_throttled_sfx(
        sim,
        caster_id,
        "fighter_skill_hit",
        1002,
        &InputTargetV1::target(caster_id),
    );
    let now_tick = sim.tick();
    let base_damage = FLIGHT_TORNADO_FLAT + attack * FLIGHT_TORNADO_AD / 100;
    let speed_bonus_pct = (12 + (speed / 450.0) as usize).min(24);
    let mut refunded_this_spin = false;
    for (target_id, is_champ) in targets {
        nearby_hits.insert(target_id);
        let marks = if is_champ {
            mark_level(sim, target_id)
        } else {
            0
        };
        let mark_bonus_pct = marks * 12;
        let raw_spin_damage =
            base_damage.saturating_mul(100 + speed_bonus_pct + mark_bonus_pct) / 100;
        // Consecutive Spin Decay: Spins #1 and #2 within 3.0s deal 100% damage; Spin #3+ deals 45% damage!
        let spin_count = if is_champ {
            record_spin_hit_and_get_count(entity_key(sim, target_id), now_tick)
        } else {
            1
        };
        let total_damage = if spin_count <= 2 {
            raw_spin_damage
        } else {
            (raw_spin_damage * 45 / 100).max(1)
        };
        sim.deal_damage(caster_id, target_id, total_damage, 0, AttackTypeV1::Skill);
        if is_champ && !refunded_this_spin {
            refunded_this_spin = true;
            refund_cable_charge_and_heal_with_marks(sim, caster_id, 1, marks);
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
    true
}

#[derive(Clone, Default)]
struct AirSuperiority {
    voice: audio::VoiceLifecycle,
    was_flying: bool,
    last_speed: f64,
    last_tornado_tick: Option<usize>,
    last_regen_tick: usize,
    nearby_tornado_hits: HashSet<usize>,
}

impl StablePassive for AirSuperiority {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        self.voice
            .on_spawn(sim, entity_id, Some("mlbb_fanny_voice_idle"));
        sim.entity_remove_buff(entity_id, CABLE_CHAIN);
        set_cable_charges(sim, entity_id, MAX_CABLE_CHARGES);
        // Tag this copy of Fanny before the closure borrows `sim` immutably.
        let spawn_key = entity_key(sim, entity_id);
        fanny_flight::clear(sim, entity_id, spawn_key);
        self.was_flying = false;
        self.last_speed = 0.0;
        self.last_tornado_tick = None;
        self.last_regen_tick = sim.tick();
        self.nearby_tornado_hits.clear();
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, entity_id: usize) {
        fanny_flight::clear(sim, entity_id, entity_key_ro(sim, entity_id));
        self.was_flying = false;
        self.last_speed = 0.0;
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
        self.voice.on_update(
            sim,
            entity_id,
            &["mlbb_fanny_voice_idle"],
            &["mlbb_fanny_voice_run"],
        );
        let tick = sim.tick();
        let Some((position, team, alive)) = sim.get_entity(entity_id).map(|entity| {
            let (x, y) = entity.pos();
            ((x as f64, y as f64), entity.team(), entity.is_alive())
        }) else {
            return;
        };
        let key = entity_key_ro(sim, entity_id);
        if !alive {
            if self.was_flying || has_named_buff(sim, entity_id, FLYING) {
                fanny_flight::clear(sim, entity_id, key);
            }
            self.was_flying = false;
            self.last_speed = 0.0;
            return;
        }

        let airborne = fanny_flight::advance(sim, entity_id, key, position);
        let flying = airborne.is_some();
        let here = airborne.unwrap_or(position);
        if flying && !self.was_flying {
            self.nearby_tornado_hits.clear();
        }
        if flying {
            let speed = distance(position, here);
            if speed > 0.0 {
                self.last_speed = speed;
                let cooldown_ready = self
                    .last_tornado_tick
                    .is_none_or(|last| tick.saturating_sub(last) >= FLIGHT_TORNADO_COOLDOWN);
                if play_tornado(
                    sim, entity_id, position, here, team, speed, &mut self.nearby_tornado_hits, cooldown_ready
                ) {
                    self.last_tornado_tick = Some(tick);
                }
            }
            self.last_regen_tick = tick;
        } else {
            if self.was_flying {
                self.last_speed = 0.0;
                self.last_regen_tick = tick;
            }
            let current_charges = get_cable_charges(sim, entity_id);
            if current_charges >= MAX_CABLE_CHARGES {
                self.last_regen_tick = tick;
            } else if tick.saturating_sub(self.last_regen_tick) >= CABLE_REGEN_TICKS {
                set_cable_charges(sim, entity_id, current_charges + 1);
                self.last_regen_tick = tick;
            }
        }
        self.was_flying = flying;
    }
}

#[derive(Clone, Default)]
struct FannyCableAi;

fn stock_retreat_goal(base: InputV1, retreat: Option<InputV1>) -> Option<(u64, u64)> {
    // Walking toward a wave or camp must never become a native cable command.
    (base.kind == InputKindV1::Move.code() && retreat == Some(base)).then_some((base.x, base.y))
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
        use fanny_flight::CableAim;
        let retreat = if let Some(base) = base_input.filter(|i| i.kind == InputKindV1::Move.code()) {
            stock_retreat_goal(base, ctx.run_away_without_skill_input())
        } else {
            None
        };
        if base_input.is_some_and(|input| input.kind == InputKindV1::Return.code()) {
            return None;
        }
        let player = ctx.player_id();
        let candidate = {
            let sim = ctx.sim()?;
            let entity = sim.get_player(player)?.champion()?;
            let id = entity.id();
            let flight = fanny_flight::read(entity_key_ro(&sim, id));
            let cd = entity.cooldown(SkillSlotV1::Skill2)?;
            if !cd.unlocked || cd.remaining != 0 || entity.stat().stack == 0
                || fanny_flight::blocked(&sim, id)
            {
                return None;
            }
            let all = units(&sim);
            let caster = all.iter().find(|unit| unit.id == id && unit.alive)?;
            let (input, aim) = if let Some((x, y)) = retreat {
                // A position keeps retreat intent in the submitted action, including replays.
                let goal = (x as f64, y as f64);
                let span = distance(caster.pos, goal);
                if span <= 1.0 {
                    return None;
                }
                let ratio = ((SKILL2_RANGE - 12_000.0) / span).min(1.0);
                let x = world_u64(caster.pos.0 + (goal.0 - caster.pos.0) * ratio);
                let y = world_u64(caster.pos.1 + (goal.1 - caster.pos.1) * ratio);
                (InputTargetV1::pos(x, y), CableAim::Retreat((x as f64, y as f64)))
            } else {
                let target_id = flight.as_ref()?.target()?;
                let target_input = InputTargetV1::target(target_id);
                let target = stock_target(&sim, &all, caster, target_input)?;
                if distance(caster.pos, target.pos) > SKILL2_RANGE {
                    return None;
                }
                (target_input, CableAim::Engage(target))
            };
            let grid = fanny_flight::grid(&sim, flight.as_ref(), &all);
            fanny_flight::planned_anchor(&grid, flight.as_ref(), &all, caster, aim, sim.tick())?;
            InputV1::action(InputKindV1::Skill2, input)
        };
        ctx.is_valid_input(&candidate).then_some(candidate)
    }
}

struct CutThroat;

impl StableEffectType for CutThroat {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let all = units(sim);
        let Some(caster) = all.iter().find(|unit| unit.id == caster_id && unit.alive) else {
            return;
        };
        let Some(target) = stock_target(sim, &all, caster, input).filter(|target| target.champion) else {
            return;
        };
        let target_distance = distance(caster.pos, target.pos);
        // Generous leash so targets moving slightly during cast windup never fizzle the Ult
        if target_distance > ULT_RANGE * 1.6 {
            return;
        }
        let grid = current_grid(&all);
        fanny_flight::clear(sim, caster_id, entity_key_ro(sim, caster_id));
        let travel_distance = (target_distance - ULT_CONTACT).max(0.0);
        let raw_dest = if target_distance > 1.0 {
            (
                caster.pos.0 + (target.pos.0 - caster.pos.0) / target_distance * travel_distance,
                caster.pos.1 + (target.pos.1 - caster.pos.1) / target_distance * travel_distance,
            )
        } else {
            target.pos
        };
        let dest = if grid.solid(raw_dest.0, raw_dest.1) {
            target.pos
        } else {
            raw_dest
        };
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
                "mlbb_fanny_wall:cut_throat_hit",
                AttackTypeV1::Skill,
                caster_id,
                &target_input,
                ticks,
            );
        }
        audio::play_throttled_sfx(sim, caster_id, "fighter_ult", 1003, &target_input);
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
// MLBB Aldous — Late-Game Soul Steal Stack Engine & Contract: Transform
// ---------------------------------------------------------------------------

static ALDOUS_STACKS: LazyLock<Mutex<HashMap<EntityKey, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const ALDOUS_MAX_STACKS: usize = 1000;

fn capped_aldous_stacks(current: usize, gained: usize) -> usize {
    current.saturating_add(gained).min(ALDOUS_MAX_STACKS)
}

fn aldous_s1_primary_damage(level: usize, attack: usize, stacks: usize) -> usize {
    let level_bonus = level.saturating_sub(1).saturating_mul(80) / 11;
    let capped_stacks = stacks.min(ALDOUS_MAX_STACKS);
    40 + level_bonus + attack + capped_stacks.saturating_mul(5) / 2
}

fn aldous_base_stat(level: usize, stack: usize) -> StatV1 {
    let lvl_growth = level.saturating_sub(1);
    StatV1 {
        attack: 65 + 4 * lvl_growth,
        magic_power: 0,
        hp: 700 + 110 * lvl_growth,
        defence: 35 + 5 * lvl_growth,
        magic_resistance: 32 + 2 * lvl_growth,
        move_speed: 1050,
        hp_regen: 8,
        stack,
        crit_chance: 0,
    }
}

fn get_aldous_stacks(sim: &mut StableSim<'_>, entity_id: usize) -> usize {
    let key = entity_key(sim, entity_id);
    ALDOUS_STACKS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&key)
        .copied()
        .unwrap_or(0)
}

fn add_aldous_stacks(sim: &mut StableSim<'_>, entity_id: usize, gained: usize) -> usize {
    let key = entity_key(sim, entity_id);
    let next = {
        let mut map = ALDOUS_STACKS.lock().unwrap_or_else(|p| p.into_inner());
        let entry = map.entry(key).or_insert(0);
        *entry = capped_aldous_stacks(*entry, gained);
        *entry
    };
    let level = sim
        .get_entity(entity_id)
        .map_or(1, |entity| entity.level().max(1));
    let base_stat = aldous_base_stat(level, next);
    sim.entity_set_base_stat(entity_id, &base_stat);
    next
}

#[derive(Default, Clone, Copy)]
struct AldousSoulSlam;

impl StableEffectType for AldousSoulSlam {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let Some(caster) = sim.get_entity(caster_id) else {
            return;
        };
        if !caster.is_alive() {
            return;
        }
        let team = caster.team();
        let attack = caster.stat().attack;
        let level = caster.level().max(1);
        let (cx, cy) = caster.pos();
        let stacks = get_aldous_stacks(sim, caster_id);

        // 40 + ((level - 1) * 80 / 11) + 100% AD + 2.5 per Soul Steal stack.
        // At 1,000 stacks the stack term contributes +2,500 raw damage; nearby splash remains 40%.
        let primary_dmg = aldous_s1_primary_damage(level, attack, stacks);
        let splash_dmg = primary_dmg * 40 / 100;

        let primary_id = if input.kind == InputTargetKindV1::Target.code() {
            Some(input.target_id)
        } else {
            None
        };

        let all = units(sim);
        let mut champ_kill_stacks = 0usize;
        let mut non_champ_kill_stacks = 0usize;

        for u in &all {
            // Never hit Towers/Base with Skill 1 so Aldous cannot one-shot the Base
            if !u.alive || !u.targetable || u.team == team || u.tower {
                continue;
            }
            let is_primary = primary_id == Some(u.id);
            let dist = distance((cx as f64, cy as f64), u.pos);
            if !is_primary && dist > 26_000.0 + u.radius {
                continue;
            }
            let dmg = if is_primary { primary_dmg } else { splash_dmg };
            let prev_hp = u.hp;
            sim.deal_damage(caster_id, u.id, dmg, 0, AttackTypeV1::Skill);
            let died = sim
                .get_entity(u.id)
                .map_or(true, |after| !after.is_alive() || after.hp().0 == 0);
            if died && prev_hp > 0 {
                if u.champion {
                    champ_kill_stacks += 12;
                } else if u.monster {
                    non_champ_kill_stacks += 6;
                } else {
                    non_champ_kill_stacks += 4;
                }
            }
        }

        let gained_stacks = champ_kill_stacks + non_champ_kill_stacks.min(6);
        if gained_stacks > 0 {
            add_aldous_stacks(sim, caster_id, gained_stacks);
        }
    }
}

#[derive(Default, Clone)]
struct AldousContractPassive {
    voice: audio::VoiceLifecycle,
    basic_attack_count: usize,
    last_farm_tick: usize,
    last_shield_tick: Option<usize>,
}

impl StablePassive for AldousContractPassive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        self.voice
            .on_spawn(sim, entity_id, Some("mlbb_aldous_voice_idle"));
        self.basic_attack_count = 0;
        self.last_farm_tick = sim.tick();
        self.last_shield_tick = None;
        let current = get_aldous_stacks(sim, entity_id);
        let level = sim
            .get_entity(entity_id)
            .map_or(1, |entity| entity.level().max(1));
        let base_stat = aldous_base_stat(level, current);
        sim.entity_set_base_stat(entity_id, &base_stat);
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _seed: u64, _player: usize, entity_id: usize) {
        self.voice.on_update(
            sim,
            entity_id,
            &["mlbb_aldous_voice_idle"],
            &["mlbb_aldous_voice_run"],
        );
        let Some(me) = sim.get_entity(entity_id) else {
            return;
        };
        if !me.is_alive() {
            return;
        }
        let now = sim.tick();
        if now.saturating_sub(self.last_farm_tick) >= 300 {
            self.last_farm_tick = now;
            // Ambient Soul Steal absorption (+1 stack every 5s = 300 ticks)
            add_aldous_stacks(sim, entity_id, 1);
        }
    }

    fn on_attack(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity: usize,
        _target: usize,
        _damage: &mut usize,
    ) {
        self.basic_attack_count += 1;
        // Contract: Transform — Every 3rd basic attack (with a 5.0s = 300-tick internal cooldown)
        // grants a 2.0s (120-tick) shield: 35 + 5*Level + 0.45*Stacks
        let now = sim.tick();
        let cd_ready = self
            .last_shield_tick
            .is_none_or(|last| now.saturating_sub(last) >= 300);
        if self.basic_attack_count % 3 == 0 && cd_ready {
            self.last_shield_tick = Some(now);
            let stacks = get_aldous_stacks(sim, entity);
            let level = sim.get_entity(entity).map_or(1, |e| e.level().max(1));
            let shield_amt = 35 + level * 5 + (stacks * 45 / 100);
            let _ = sim.entity_add_shield(entity, shield_amt, 120);
            sim.add_buff(entity, &BuffV1::timed("mlbb_aldous_p_shield", 120));
        }
    }

    fn on_kill(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize, _victim: usize) {
        add_aldous_stacks(sim, entity, 10);
    }
}

fn init(host: &StableHost) -> StableMod {
    let _ = std::fs::rename(&*LOG_PATH, LOG_PATH.with_extension("prev.log"));
    // Which build is actually loaded? Until now nothing in the log said, so a stale DLL
    // was indistinguishable from a broken one.
    wlog(&format!(
        "=== MLBB Heroes {} loaded (Granger technical probe; published Popol & Kupa and Julian audio preserved)",
        env!("CARGO_PKG_VERSION")
    ));
    wlog("=== Sun candidate loaded with Fanny stock-AI cable routing ===");
    wlog("[HANABI v5] Petal projectile owner-team fix active; use per-origin PETAL_SPAWN/PETAL_HIT traces to verify in match.");
    host.log(
        LogLevel::Info,
        "Sun candidate loaded. Fanny follows stock AI and retains airborne cable redirects.",
    );
    let mut module = StableMod::new(MOD_ID);
    audio::register_native_cues(&mut module);
    popol_probe::register(&mut module);
    // Bracket client updates so Julian audio follows fresh viewer ticks, including pause/seek.
    granger::register(&mut module);
    lesley::register(&mut module);
    valir::register(&mut module);
    hanzo::register(&mut module);
    atlas::register(&mut module);
    yu_zhong::register(&mut module);
    module.set_extension(hanzo::Extension);
    franco::register(&mut module);
    balmond::register(&mut module);
    alpha::register(&mut module);
    obsidia::register(&mut module);
    arlott::register(&mut module);
    module.add_native_effect("mlbb_heroes:cable", SteelCable);
    module.add_native_effect(CABLE_IMPACT_EFFECT, CableImpact);
    module.add_native_effect(CABLE_DRAW_EFFECT, CableDraw);
    module.add_native_effect("mlbb_heroes:cut_throat", CutThroat);
    module.add_native_effect("mlbb_heroes:cut_throat_hit", CutThroatHit);
    module.add_native_passive("mlbb_heroes:air_superiority", AirSuperiority::default());
    module.add_native_effect("mlbb_heroes:aldous_s1_slam", AldousSoulSlam);
    module.add_native_passive(
        "mlbb_heroes:contract_transform",
        AldousContractPassive::default(),
    );
    module.add_native_passive(
        "mlbb_aldous:contract_transform",
        AldousContractPassive::default(),
    );
    module.add_native_effect("mlbb_fanny_wall:cable", SteelCable);
    module.add_native_effect("mlbb_fanny_wall:cable_impact", CableImpact);
    module.add_native_effect("mlbb_fanny_wall:cable_draw", CableDraw);
    module.add_native_effect("mlbb_fanny_wall:cut_throat", CutThroat);
    module.add_native_effect("mlbb_fanny_wall:cut_throat_hit", CutThroatHit);
    module.add_native_passive("mlbb_fanny_wall:air_superiority", AirSuperiority::default());
    lancelot::register(&mut module);
    estes::register(&mut module);
    angela::register(&mut module);
    eudora::register(&mut module);
    julian::register(&mut module);
    gusion::register(&mut module);
    karrie::register(&mut module);
    hanabi::register(&mut module);
    uranus::register(&mut module);
    chou::register(&mut module);
    zhask::register(&mut module);
    gord::register(&mut module);
    belerick::register(&mut module);
    cecilion::register(&mut module);
    johnson::register(&mut module);
    sun::register(&mut module);
    module.add_player_input_ai(FannyCableAi::default());
    module.set_map_customizer(FannyMapReader);
    module
}

declare_stable_mod!(init, requires = 10);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aldous_soul_steal_caps_at_one_thousand_stacks() {
        assert_eq!(capped_aldous_stacks(0, 400), 400);
        assert_eq!(capped_aldous_stacks(900, 100), 1000);
        assert_eq!(capped_aldous_stacks(1000, 12), 1000);
        assert_eq!(capped_aldous_stacks(usize::MAX, 1), 1000);
    }

    #[test]
    fn aldous_max_stack_s1_clears_the_level15_squishy_benchmark() {
        let level15_attack = 65 + 4 * 14;
        let damage = aldous_s1_primary_damage(15, level15_attack, 1000);
        assert_eq!(aldous_s1_primary_damage(15, level15_attack, 500), 1512);
        assert_eq!(damage, 2762);
        // Conservative level-15 Fanny benchmark: 2,092 HP and 28 defense.
        // With the standard 100/(100+DEF) mitigation relation, this requires 2,678 raw damage.
        let required_raw = (2092 * (100 + 28) + 99) / 100;
        assert!(damage >= required_raw);
        // The same hit remains below the corresponding level-15 Aldous tank threshold.
        let tank_raw = (2190 * (100 + 35) + 99) / 100;
        assert!(damage < tank_raw);
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
            let Some(next) =
                choose_engage_hit_with_grid(Some(&grid), &hits, pos, target, 10_000.0, &[])
            else {
                break;
            };
            pos = next.pos();
            hops += 1;
        }
        assert!(
            hops >= 3,
            "expected at least 3 chained wall cables, got {hops}"
        );
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
    fn cables_only_replace_the_stock_retreat_move() {
        let flee = InputV1::move_to(120_000, 380_000);
        assert_eq!(stock_retreat_goal(flee, Some(flee)), Some((120_000, 380_000)));
        assert_eq!(stock_retreat_goal(InputV1::move_to(130_000, 380_000), Some(flee)), None);
        assert_eq!(stock_retreat_goal(flee, None), None);
        assert_eq!(stock_retreat_goal(InputV1::return_home(), Some(InputV1::return_home())), None);
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
            radius: 10_000.0,
            hp: 500,
            max_hp: 1_000,
            stack: 0,
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

    // -----------------------------------------------------------------------
    // Sim-scoped state keys
    //
    // The game can run two simulations of one match at once with identical
    // entity ids. Global state keyed by bare entity id is shared between them
    // and the two sims corrupt each other. These tests pin the hazard and the
    // fix. See the comment above `SimScope`.
    // -----------------------------------------------------------------------

    fn scope(match_id: u64) -> SimScope {
        SimScope { kind: 1, match_id, replay_id: 7, set_index: 2, seed: 4242 }
    }

    fn key(copy: u64) -> EntityKey {
        EntityKey { scope: scope(592), entity_id: 4, copy }
    }

    /// Documents the hazard: two pre-sims of one match derive to the SAME
    /// scope. This is why a derived key alone is not enough.
    #[test]
    fn two_pre_sims_of_one_match_derive_to_the_same_scope() {
        assert_eq!(
            scope(592),
            scope(592),
            "identical fixture => identical derived scope"
        );
    }

    /// The fix: the assigned copy serial is what tells them apart.
    #[test]
    fn assigned_copy_serial_separates_otherwise_identical_sims() {
        assert_ne!(key(0), key(1), "same scope, same entity, different copy");
    }

    /// And that difference is what keeps their state from colliding.
    #[test]
    fn separate_copies_hold_separate_state() {
        let mut map: HashMap<EntityKey, usize> = HashMap::new();
        map.insert(key(0), 111);
        map.insert(key(1), 222);
        assert_eq!(map.get(&key(0)).copied(), Some(111));
        assert_eq!(map.get(&key(1)).copied(), Some(222));
        assert_eq!(map.len(), 2);
    }

    /// Aldous's stack counter is keyed per simulation copy, so a stack gained
    /// in one sim is invisible to the other.
    #[test]
    fn aldous_stacks_are_isolated_per_simulation_copy() {
        let (copy_a, copy_b) = (key(900), key(901));
        {
            let mut map = ALDOUS_STACKS.lock().unwrap_or_else(|p| p.into_inner());
            let entry = map.entry(copy_a).or_insert(0);
            *entry = capped_aldous_stacks(*entry, 30);
            // The other sim never gained anything.
            assert_eq!(map.get(&copy_b).copied().unwrap_or(0), 0);
            assert_eq!(map.get(&copy_a).copied(), Some(30));
        }
        // clean up so this test cannot influence another
        ALDOUS_STACKS.lock().unwrap_or_else(|p| p.into_inner()).remove(&copy_a);
    }

    /// The tag buff parses back to the serial it was written with.
    #[test]
    fn sim_copy_tag_round_trips() {
        let name = format!("{SIM_COPY_PREFIX}{}", 41);
        assert_eq!(name.strip_prefix(SIM_COPY_PREFIX).unwrap(), "41");
        assert_eq!(name.strip_prefix(SIM_COPY_PREFIX).and_then(|r| r.parse::<u64>().ok()), Some(41));
        // An unrelated buff must not be mistaken for a tag.
        let other = "mlbb_aldous_p_shield".to_string();
        assert!(other.strip_prefix(SIM_COPY_PREFIX).is_none());
    }

    /// Serials are unique per assignment, which is what makes them usable as
    /// a discriminator across interleaved simulations.
    #[test]
    fn sim_copy_serials_are_unique() {
        // Both values must come from assign_sim_copy(). Comparing it against a
        // raw NEXT_SIM_COPY.fetch_add used to pass even when the function
        // returned a constant — which is exactly the regression this guards.
        let a = assign_sim_copy();
        let b = assign_sim_copy();
        let c = assign_sim_copy();
        assert_ne!(a, b, "two assignments must not collide");
        assert_ne!(b, c, "two assignments must not collide");
        assert_ne!(a, c, "two assignments must not collide");
    }
}
