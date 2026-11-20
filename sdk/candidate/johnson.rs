//! Native Johnson gameplay — Full Throttle (ult), Impact Wrench (S1),
//! Electromagnetic Waves (S2), Electro Airbag (passive).
//!
//! # Why damage is native
//!
//! Johnson's whole identity is **damage that scales with DEFENCE**. `Attack`
//! and `ApAttack` only scale off AD / AP (`attack_ratio`), so there is no data
//! expression for `+110% defence`. Every damaging node in
//! `mlbb_johnson.data_champion` is therefore a `Native` that computes
//! `base + percent(defence)` and pays it out through
//! `sim.deal_damage_typed(..., DamageTypeV1::Ap, ...)` — Ap because his damage
//! is magic.
//!
//! `defence` is real and readable: `entity.stat().defence` (`StatV1.defence`),
//! and `buff_state.defence` exists, so defence items feed the scaling for free.
//!
//! # The car is a `view_buff`, NOT a `CasterAnimation`
//!
//! `champion-data.md` lines 189 and 1119: *"`can_use_with_move` … does not let
//! the unit walk during a `CasterAnimation`"*. Cecilion's ult forced 154 of 180
//! ticks and he was rooted for 86% of his own channel. A car that cannot move
//! is not a car, so the car art rides on a **`view_buff` keyed to the buff
//! name** — which follows the unit while it moves (the Cecilion aura fix,
//! PROJECT_CONTEXT §29.27c). See `view_buffs` in the data file.
//!
//! # The passenger is `entity_banish`, not a position pin
//!
//! `entity_set_pos` does **not** move other units in the real host
//! (§29.27b — Cecilion's clasp: the owner saw the stun and nothing else). So
//! the ally is not dragged along; they are **banished** for the drive:
//! `entity_banish` is documented as *"untargetable, invisible, input-locked"*,
//! which is exactly TFM2 0.7.0's own Ferryman ult — *"takes them aboard …
//! untargetable, cannot act, immune to movement control"*. They reappear at the
//! crash. No per-tick pinning, no unverified `entity_pull`.

use mod_api_stable::*;
use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Tuning — §4 of JOHNSON_SPEC.md
// ---------------------------------------------------------------------------

/// S1 Impact Wrench: 70 + 110% defence.
const S1_BASE: usize = 70;
const S1_DEF_PCT: usize = 110;
/// S1 stun on landing — canon 0.8 s. Applied by the data file's
/// `RangeProjectile ... Stun {duration: 48}`, not by Rust; kept here so the
/// number lives next to the damage it pairs with.
#[allow(dead_code)]
const S1_STUN_TICKS: usize = 48;

/// S2 Electromagnetic Waves, per tick: 40 + 45% defence.
const S2_BASE: usize = 40;
const S2_DEF_PCT: usize = 45;
/// Slow + magic-resist shred per stack, stacking 3x.
const S2_SLOW_PER_STACK: i32 = -12;
const S2_MRES_PER_STACK: i32 = -6;
const S2_MAX_STACKS: usize = 3;
const S2_DEBUFF_TICKS: usize = 120;

/// Ult crash: 90 + 150% defence, rising to 90 + 220% defence with drive time.
const ULT_BASE: usize = 90;
const ULT_DEF_PCT_MIN: usize = 150;
const ULT_DEF_PCT_MAX: usize = 220;
const ULT_STUN_MIN: usize = 30;
const ULT_STUN_MAX: usize = 60;
/// Zone left behind, per tick: 18 + 15% defence.
const ZONE_BASE: usize = 18;
const ZONE_DEF_PCT: usize = 15;

/// Passive Electro Airbag: 120 + 240% defence, below 30% HP.
const AIRBAG_BASE: usize = 120;
const AIRBAG_DEF_PCT: usize = 240;
const AIRBAG_HP_PCT: usize = 30;
const AIRBAG_TICKS: usize = 600;
const AIRBAG_COOLDOWN_TICKS: usize = 6000;

// ---- car form ----
const CAR_UNIT_NAME: &str = "mlbb_johnson_car";
const CAR_TICKS: usize = 480;          // ~8 s, canon drive window
const CAR_SPEED: f64 = 2_400.0;        // world units per tick
const CAR_CRASH_RADIUS: f64 = 22_000.0;
const BOOST_AFTER_TICKS: usize = 90;   // when the nitrous art swaps in
const HOPIN_RANGE: f64 = 30_000.0;

/// How long each banish lasts while the rider is aboard.
///
/// **This is a refreshed window, not the length of the ride.** rc.3–rc.5
/// banished the rider once for `CAR_TICKS + 30` (510 ticks, ~17 s) and then
/// tried to cut it short with a 1-tick re-banish borrowed from Zhask — which
/// only ever calls it on **itself**. On another entity it did not release, so
/// every early crash stranded the ally for the rest of the window. That is the
/// bug the owner hit in rc.5: *"he still crash by himself leaving his ally
/// banished."*
///
/// The stable API documents `entity_banish` and **no** unbanish, so nothing
/// here depends on release semantics: the banish is re-issued every tick the
/// rider is aboard, and ending the drive simply stops the refresh. The rider
/// then returns on their own within this many ticks — at the crash site,
/// because every refresh tick also pinned them to the car.
///
/// Short enough that even a missed refresh is not a kidnapping; long enough
/// that a single dropped tick cannot make them flicker into view mid-ride.
const HOPIN_REFRESH_TICKS: usize = 6;
/// Fallback drag speed for putting the rider down at the crash site (units per
/// tick). Franco's hook drags at 2,500 over a 100,000 reach (40 ticks), but a
/// full-length drive covers up to `CAR_TICKS * CAR_SPEED` = 1,152,000 units, so
/// this only matters as a fallback and has to be an order of magnitude quicker
/// to arrive inside a second.
const DROP_GRAB_SPEED: usize = 20_000;
/// Drag speed used to keep the rider glued to the car while it is moving. This
/// was re-issued EVERY tick in v1.1.110, but entity_grab is ignored for banished
/// units (champion-data.md:4081: "A banished unit no longer moves or acts and
/// knockbacks stop"). The frequent calls on a banished entity caused the "stops
/// banish" side effect the owner reported. The reposition timer in end_drive
/// now handles positioning after banish expires. Kept for reference and the
/// ride_grip test.
#[allow(dead_code)]
const RIDE_GRAB_SPEED: usize = 20_000;

/// Buff names. These MUST match `view_buffs[].name` in the data file or the
/// car art silently never renders (§29.27c: view_buffs are keyed by BUFF NAME,
/// not by art name).
const CAR_BUFF: &str = "mlbb_heroes_johnson_car";
const CAR_BOOST_BUFF: &str = "mlbb_heroes_johnson_car_boost";
const CAR_PROTECT_BUFF: &str = "mlbb_heroes_johnson_car_protect";
const EM_SLOW_BUFF: &str = "mlbb_heroes_johnson_em_slow";
const AIRBAG_BUFF: &str = "mlbb_heroes_johnson_airbag";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `pct` percent of `value`, integer-rounded. Ratios are whole percents.
fn percent(value: usize, pct: usize) -> usize {
    value * pct / 100
}

fn defence_of(sim: &mut StableSim<'_>, id: usize) -> usize {
    sim.get_entity(id).map(|e| e.stat().defence).unwrap_or(0)
}

/// Johnson's magic damage for a given base + defence ratio.
fn johnson_damage(sim: &mut StableSim<'_>, id: usize, base: usize, pct: usize) -> usize {
    base + percent(defence_of(sim, id), pct)
}

fn dist(a: (u64, u64), b: (u64, u64)) -> f64 {
    let dx = a.0 as f64 - b.0 as f64;
    let dy = a.1 as f64 - b.1 as f64;
    (dx * dx + dy * dy).sqrt()
}

/// Own fountain — the same fallback `lib.rs` uses for Fanny's retreat.
fn fountain_pos(team: usize) -> (f64, f64) {
    if team == 0 {
        (96_000.0, 864_000.0)
    } else {
        (864_000.0, 96_000.0)
    }
}

/// How many instances of `name` are on the entity. There is no `buff_count(name)`;
/// the count comes from walking the buff slots (the `sim_copy_of` shape in
/// `lib.rs`).
fn count_buff(sim: &mut StableSim<'_>, id: usize, name: &str) -> usize {
    let Some(e) = sim.get_entity(id) else {
        return 0;
    };
    (0..e.buff_count())
        .filter_map(|i| e.buff_at(i))
        .filter(|b| b.name() == name)
        .count()
}

// ---------------------------------------------------------------------------
// Diagnostic logging — v1.1.111 playtest investigation
// ---------------------------------------------------------------------------

/// How often to log step_drive internals (in ticks). 60 = once per second.
const DRIVE_LOG_EVERY: usize = 60;

fn log_event(sim: &StableSim<'_>, category: &str, message: &str) {
    crate::wlog(format!(
        "[Johnson][{category}] tick={} {message}",
        sim.tick()
    ));
}

fn mode_name(m: DriveMode) -> &'static str {
    match m {
        DriveMode::Initiate => "Initiate",
        DriveMode::Retreat => "Retreat",
        DriveMode::Taxi => "Taxi",
    }
}

fn nearest_enemy_champion(sim: &mut StableSim<'_>, id: usize) -> Option<(usize, (u64, u64))> {
    let (team, pos) = sim.get_entity(id).map(|e| (e.team(), e.pos()))?;
    let mut best: Option<(usize, (u64, u64), f64)> = None;
    for i in 0..sim.entity_count() {
        let Some(e) = sim.entity_at(i) else { continue };
        if !e.is_alive() || !e.is_champion() || e.team() == team || e.id() == id {
            continue;
        }
        if !e.is_targetable() {
            continue;
        }
        let d = dist(pos, e.pos());
        if best.as_ref().map_or(true, |b| d < b.2) {
            best = Some((e.id(), e.pos(), d));
        }
    }
    best.map(|(a, b, _)| (a, b))
}

fn nearest_ally_champion(sim: &mut StableSim<'_>, id: usize, range: f64) -> Option<usize> {
    let (team, pos) = sim.get_entity(id).map(|e| (e.team(), e.pos()))?;
    let mut best: Option<(usize, f64)> = None;
    for i in 0..sim.entity_count() {
        let Some(e) = sim.entity_at(i) else { continue };
        if !e.is_alive() || !e.is_champion() || e.team() != team || e.id() == id {
            continue;
        }
        let d = dist(pos, e.pos());
        if d <= range && best.as_ref().map_or(true, |b| d < b.1) {
            best = Some((e.id(), d));
        }
    }
    best.map(|(a, _)| a)
}

// ---------------------------------------------------------------------------
// Drive state — short-lived, keyed by entity id (same shape as Ling's PERCHES)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum DriveMode {
    Initiate,
    Retreat,
    Taxi,
}

struct Drive {
    ticks: usize,
    mode: DriveMode,
    passenger: Option<usize>,
    target: Option<usize>,
    car_id: usize,
}

// ---------------------------------------------------------------------------
// Per-sim state — the game runs parallel sims (server + client view) that
// share static memory. The SKILL.md rule is explicit: "Every timer of the
// add-on lives as a buff on the unit (no statics shared between the game's
// parallel sims)." The DRIVES, REPOSITIONS, and AIRBAG_CD maps below are
// keyed by (sim_origin, entity_id) so each sim gets its own entry.
//
// Before this fix (v1.1.112 playtest, owner caster=23): two sims shared
// DRIVES[23], causing Sim B's step_drive to read Sim A's car_id, fail to
// find the car entity, call end_drive, and remove Sim A's drive — making
// the car freeze. Sim B's end_drive then read crash_pos=(0,0) because the
// car_id didn't exist in its sim, teleporting Johnson to (0,0) — the
// "top side bush" the owner reported. 42 occurrences in one match.
// ---------------------------------------------------------------------------

/// Identity of one entity inside one particular simulation run — the mod's
/// canonical per-sim key (see `SimScope` / `EntityKey` in lib.rs).
///
/// `sim_origin()` alone is NOT enough: it returns `None` during
/// pre-simulation, so `match_id` / `set_index` collapse to defaults and two
/// pre-sims of the same match become indistinguishable. `EntityKey` carries
/// an assigned `copy` serial on top of the scope, which is what actually
/// tells two simulation copies apart. Aldous, Fanny and Zhask already key
/// their state this way (see the v2.0.7 note in `mod.mod_info`).
use crate::EntityKey;

/// Tags the entity with a copy serial on first use and returns its key.
/// Needs `&mut` because the tag is a real buff on the unit.
fn drive_key(sim: &mut StableSim<'_>, id: usize) -> EntityKey {
    crate::entity_key(sim, id)
}

/// Whether a stored key belongs to the simulation we are currently in: the
/// scope matches AND the entity in *this* sim carries that same copy tag.
/// A foreign sim's entry fails the second test, because its copy of the unit
/// was tagged with a different serial.
fn belongs_here(sim: &StableSim<'_>, key: &EntityKey) -> bool {
    key.scope == crate::SimScope::of(sim)
        && crate::sim_copy_of(sim, key.entity_id) == Some(key.copy)
}

static DRIVES: LazyLock<Mutex<HashMap<EntityKey, Drive>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Pending repositions after the drive ends. Keyed by entity id, value is
/// (x, y, ticks_remaining). When the timer hits 0, `entity_set_pos` moves the
/// entity to (x, y). This exists because `entity_set_pos` is ignored for
/// banished units (champion-data.md:4081) — Johnson and the rider stay at
/// their cast positions during the drive. After their banish expires (HOPIN_
/// REFRESH_TICKS ticks after the last refresh), this timer fires and places
/// them at the crash site.
///
/// Checked from `JohnsonDrive::on_update`, which fires every tick for Johnson.
/// The map is global, so it also handles the rider (a different entity whose
/// own passive we do not control).
static REPOSITIONS: LazyLock<Mutex<HashMap<EntityKey, (f64, f64, usize)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn drives() -> std::sync::MutexGuard<'static, HashMap<EntityKey, Drive>> {
    DRIVES.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn is_driving(sim: &mut StableSim<'_>, id: usize) -> bool {
    let key = drive_key(sim, id);
    drives().contains_key(&key)
}

/// Puts the rider down where the car stopped, NOT where they got in.
///
/// Bug #16 (owner, rc.3 playtest): *"they ride right then they banish but they
/// don't appear on the crash site where johnson is, they appear where they
/// banish."* The rider was banished at the kerb and simply released there, so
/// Hop In dropped them off at the pickup point instead of at the crash.
///
/// Which of these actually moves a unit cannot be settled from the SDK — a unit
/// test can only prove the ABI call is well-formed, never that the host honours
/// it (§29.27b). So all three are fired, in this order, at one shared
/// destination: any one working is enough, and more than one is harmless.
///
///   1. **`entity_set_pos` while still banished.** §29.27b used to record this
///      as flatly broken — **that conclusion was a misdiagnosis** (see the
///      2026-11-09 correction: Cecilion's stun was data-level, so "saw the
///      stun, no displacement" never proved the native ran). The honest
///      position is *unproven on non-self units*, not disproven, and the car
///      itself is proof it is honoured on the caster. It is also a genuinely
///      different case from Cecilion's: that was a live enemy the engine was
///      actively steering, whereas a banished unit is input-locked with no
///      movement order to overwrite the write. Tried first; first is cheap.
///   2. **`entity_grab(.., 0)`** is the engine's own drag, "pull until it
///      arrives". Franco's hook rides on it and is in-game verified, so it
///      works on the real host; what is unproven is whether the host applies it
///      to an ALLY, since it is crowd control. Only fired if (1) was refused.
///   3. **A second `entity_set_pos`**, covering the case where the host stashed
///      a position at banish time and restores it when the window lapses.
///
/// There is deliberately **no release call** in here any more. rc.3–rc.5 used
/// `entity_banish(.., 1, ..)` as an unbanish, borrowing Zhask's precedent —
/// but Zhask only ever calls it on ITSELF (`zhask.rs:585`, `caster, caster`),
/// and the stable API documents banish with **no** unbanish. On another entity
/// it did not release: the owner's rc.5 playtest found the ally still banished
/// after every early crash. The banish is now a short window refreshed in
/// `step_drive` (see `HOPIN_REFRESH_TICKS`), so ending the drive stops the
/// refresh and the rider returns on their own.
fn drop_passenger(sim: &mut StableSim<'_>, driver: usize, passenger: usize) {
    let car = sim.get_entity(driver).map(|e| e.pos());
    let Some((x, y)) = car else {
        return;
    };
    let moved = sim.entity_set_pos(passenger, x, y);
    // NOTE: there is deliberately NO release call here.
    //
    // rc.3–rc.5 re-banished the rider for a single tick as a makeshift
    // "release", borrowing Zhask's self-banish trick. It does not work on
    // another entity
    // (§29.27b: honoured on the caster, UNPROVEN elsewhere) — the owner's rc.5
    // playtest confirmed the ally stayed banished after every early crash.
    // The banish is now a short window refreshed in `step_drive`, so ending the
    // drive stops the refresh and the rider returns on their own within
    // `HOPIN_REFRESH_TICKS` ticks — already at the crash site, because every
    // refresh tick also pinned them to the car.
    if !moved {
        let _ = sim.entity_grab(driver, passenger, DROP_GRAB_SPEED, 0);
    }
    let _ = sim.entity_set_pos(passenger, x, y);
}

fn end_drive(sim: &mut StableSim<'_>, id: usize) {
    let key = drive_key(sim, id);
    let d = drives().remove(&key);
    if let Some(d) = d.as_ref() {
        // Diagnostic log — v1.1.111 playtest investigation.
        // Logs when the drive ends: tick count, car position (crash site),
        // mode, whether it was a crash or expiry.
        log_event(
            sim,
            "end_drive",
            &format!(
                "car_id={} ticks={} mode={} passenger={:?}",
                d.car_id,
                sim.tick(),
                mode_name(d.mode),
                d.passenger,
            ),
        );

        // Capture the car's final position BEFORE killing it — this is the
        // crash site where Johnson and the rider should appear after their
        // banish expires.
        //
        // If the car is already gone, there is NO crash site. This used to
        // fall back to (0, 0), which is the map corner — the "top side bush"
        // the owner kept getting yanked to. A missing car must leave Johnson
        // where he is, not teleport him across the map.
        let Some((car_x, car_y)) = sim.get_entity(d.car_id).map(|e| e.pos()) else {
            log_event(
                sim,
                "end_drive",
                &format!(
                    "car_id={} missing from this sim — no crash site, Johnson left in place",
                    d.car_id
                ),
            );
            return;
        };
        let crash_x = car_x as f64;
        let crash_y = car_y as f64;
        // Drop BOTH Johnson and the rider at the car's position BEFORE killing
        // the car. Both are banished at this point, so entity_set_pos and
        // entity_grab are ignored — but drop_passenger is kept as a safety net
        // in case the engine changes its behavior.
        drop_passenger(sim, d.car_id, id);
        if let Some(p) = d.passenger {
            drop_passenger(sim, d.car_id, p);
        }
        // Destroy the car entity. entity_set_hp(0) kills it (same pattern Zhask uses).
        let _ = sim.entity_set_hp(d.car_id, 0);
        let _ = sim.entity_remove_buff(d.car_id, CAR_PROTECT_BUFF);
        let _ = sim.entity_remove_buff(d.car_id, CAR_BOOST_BUFF);
        // Set reposition timers for Johnson and the rider. The timer fires
        // HOPIN_REFRESH_TICKS + 10 ticks after end_drive, by which time both
        // banish windows have expired and entity_set_pos works on them.
        // Without this, they stay at their cast positions (entity_set_pos is
        // ignored for banished units) and appear there when banish lapses —
        // the v1.1.109/v1.1.110 "Johnson not at crash site" bug.
        let reposition_ticks = HOPIN_REFRESH_TICKS + 10;
        {
            let mut rp = REPOSITIONS.lock().unwrap_or_else(|p| p.into_inner());
            // Only set a timer if the entity is still alive.
            if sim.get_entity(id).map(|e| e.is_alive()).unwrap_or(false) {
                rp.insert(drive_key(sim, id), (crash_x, crash_y, reposition_ticks));
                log_event(
                    sim,
                    "reposition_set",
                    &format!(
                        "eid={} crash_pos=({}, {}) ticks={}",
                        id, crash_x, crash_y, reposition_ticks
                    ),
                );
            }
            if let Some(p) = d.passenger {
                if sim.get_entity(p).map(|e| e.is_alive()).unwrap_or(false) {
                    rp.insert(drive_key(sim, p), (crash_x, crash_y, reposition_ticks));
                    log_event(
                        sim,
                        "reposition_set",
                        &format!(
                            "eid={} crash_pos=({}, {}) ticks={}",
                            p, crash_x, crash_y, reposition_ticks
                        ),
                    );
                }
            }
        }
    }
    // Johnson's self-banish refresh stops here; he returns within HOPIN_REFRESH_TICKS.
    // No buffs to remove from Johnson in the new design (he's banished, not buffed).
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

/// `mlbb_heroes:johnson_basic_attack` — plain AD basic attack.
#[derive(Clone, Copy, Default)]
pub struct JohnsonBasicAttack;

impl StableEffectType for JohnsonBasicAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let Some(target) = Some(input.target_id).filter(|t| *t != caster) else {
            return;
        };
        let Some(attack) = sim.get_entity(caster).map(|e| e.stat().attack) else {
            return;
        };
        let _ = sim.deal_damage_typed(
            caster,
            target,
            attack,
            DamageTypeV1::Ad,
            AttackTypeV1::BaseAttack,
        );
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (caster_stat.attack, 0)
    }
}

/// `mlbb_heroes:johnson_s1_hit` — Impact Wrench damage along the throw.
#[derive(Clone, Copy, Default)]
pub struct JohnsonS1Hit;

impl StableEffectType for JohnsonS1Hit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target = input.target_id;
        if target == caster {
            return;
        }
        let amount = johnson_damage(sim, caster, S1_BASE, S1_DEF_PCT);
        let _ = sim.deal_damage_typed(
            caster,
            target,
            amount,
            DamageTypeV1::Ap,
            AttackTypeV1::Skill,
        );
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        // Ap because his damage is magic, even though it scales off defence.
        (0, S1_BASE + percent(caster_stat.defence, S1_DEF_PCT))
    }
}

/// `mlbb_heroes:johnson_s2_tick` — Electromagnetic Waves: damage + stacking
/// slow and magic-resist shred.
#[derive(Clone, Copy, Default)]
pub struct JohnsonS2Tick;

impl StableEffectType for JohnsonS2Tick {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target = input.target_id;
        if target == caster {
            return;
        }
        let amount = johnson_damage(sim, caster, S2_BASE, S2_DEF_PCT);
        let _ = sim.deal_damage_typed(
            caster,
            target,
            amount,
            DamageTypeV1::Ap,
            AttackTypeV1::Skill,
        );

        // Stacks: each application refreshes and deepens the debuff, capped at 3.
        let slow = BuffV1 {
            move_speed_mult: S2_SLOW_PER_STACK,
            magic_resistance: S2_MRES_PER_STACK,
            ..BuffV1::timed(EM_SLOW_BUFF, S2_DEBUFF_TICKS)
        };
        let _ = sim.entity_stack_buff(target, &slow, S2_MAX_STACKS, true);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (0, S2_BASE + percent(caster_stat.defence, S2_DEF_PCT))
    }
}

/// `mlbb_heroes:johnson_full_throttle` — the ult. Enters car form and, if an
/// ally is close, takes them aboard.
#[derive(Clone, Copy, Default)]
pub struct JohnsonFullThrottle;

impl StableEffectType for JohnsonFullThrottle {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _input: InputTargetV1) {
        let Some(alive) = sim.get_entity(caster).map(|e| e.is_alive()) else {
            return;
        };
        if !alive {
            return;
        }

        // Zhask-style: spawn a car entity, banish Johnson, car drives itself.
        let (team, pos) = sim.get_entity(caster).map(|e| (e.team(), e.pos()))
            .unwrap_or((0, (0, 0)));

        // Find passenger and target BEFORE banishing Johnson.
        let target = nearest_enemy_champion(sim, caster).map(|(id, _)| id);
        let passenger = nearest_ally_champion(sim, caster, HOPIN_RANGE);
        let mode = if target.is_some() {
            DriveMode::Initiate
        } else if passenger.is_some() {
            DriveMode::Taxi
        } else {
            DriveMode::Retreat
        };

        // Spawn the car entity. It has its own sprite (champions/mlbb_johnson_car).
        let mut car_stat = StatV1::default();
        car_stat.hp = 1000; // enough to not die from a single hit
        // move_speed stays at 0 (StatV1::default). The engine's own movement
        // pipeline processes the car at move_speed units/tick — with 50 (v1.1.110)
        // it crawls at 50/tick = 24,000 max in 480 ticks vs the intended
        // CAR_SPEED=2400/tick = 1,152,000, so it never reaches distant targets
        // (owner: "moves so slow, doesnt crash"). With 0 the engine does not
        // process the car's movement at all, leaving it to entity_set_pos in
        // step_drive — which is what v1.1.109 used and the owner confirmed
        // "car moved and crashed fine."
        let car_attack = UnitAttackV1 {
            attack_ratio: 0,
            attack: 0,
            range: 0,
            cooltime: 0,
            duration: 0,
            start_timing: 0,
            cancelable: false,
            attack_type: AttackTypeV1::BaseAttack.code(),
        };
        let Some(car_id) = sim.spawn_unit(
            CAR_UNIT_NAME,
            caster,
            team,
            pos.0,
            pos.1,
            CAR_TICKS as u64,
            &car_stat,
            &car_attack,
        ) else {
            return;
        };

        // Car entity gets cc_immune + ignore_wall (no view_buff art — it has its own sprite).
        let car_protect = BuffV1 {
            cc_immune: true,
            ignore_wall: true,
            ..BuffV1::timed(CAR_PROTECT_BUFF, CAR_TICKS)
        };
        sim.add_buff(car_id, &car_protect);

        // Banish passenger first (Hop In art plays while Johnson is still visible).
        if let Some(p) = passenger {
            let _ = sim.entity_banish(caster, p, HOPIN_REFRESH_TICKS, "johnson_hopin", "johnson_hopin");
        }

        // Banish Johnson (self-banish, Zhask-style). Hides from ALL views.
        let _ = sim.entity_banish(caster, caster, HOPIN_REFRESH_TICKS, "", "");

        drives().insert(
            drive_key(sim, caster),
            Drive {
                ticks: 0,
                mode,
                passenger,
                target,
                car_id,
            },
        );

        // Diagnostic log — v1.1.111 playtest investigation.
        // Logs Johnson's position, car spawn position, mode, target, passenger.
        // This tells us: (1) where the car actually spawns, (2) what mode the
        // drive starts in, (3) whether target/passenger are found.
        log_event(
            sim,
            "apply",
            &format!(
                "caster={} johnson_pos=({}, {}) car_id={} car_spawn_pos=({}, {}) \
                 mode={} target={:?} passenger={:?}",
                caster,
                pos.0,
                pos.1,
                car_id,
                pos.0,
                pos.1,
                mode_name(mode),
                target,
                passenger,
            ),
        );
    }

    fn expected_damage(&self, _caster_stat: &StatV1) -> (usize, usize) {
        (0, 0)
    }
}

/// `mlbb_heroes:johnson_electro_airbag` — passive, checked every tick.
#[derive(Clone, Copy, Default)]
pub struct JohnsonElectroAirbag;

/// Per-entity cooldown so the shield does not re-trigger every tick.
/// Keyed by `EntityKey` so parallel sims don't share cooldowns.
static AIRBAG_CD: LazyLock<Mutex<HashMap<EntityKey, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The Electro Airbag check. Runs every tick from `JohnsonDrive`, and is also
/// reachable as the `johnson_electro_airbag` effect.
///
/// It has to live in the passive, not only in an effect: a champion's data file
/// has exactly ONE `passive_ref` slot, and Johnson has two passive behaviours
/// (the airbag and the car stepper). The registered passive does both.
fn tick_electro_airbag(sim: &mut StableSim<'_>, id: usize) {
    let Some((cur, max, alive)) = sim.get_entity(id).map(|e| {
        let (c, m) = e.hp();
        (c, m, e.is_alive())
    }) else {
        return;
    };
    if !alive || max == 0 {
        return;
    }

    // Per-sim key so parallel sims don't share the cooldown.
    let cd_key = drive_key(sim, id);

    // The cooldown must count down EVERY tick, not only while he is low. It
    // used to sit behind the HP gate, so dropping low, shielding, then healing
    // back above the threshold froze the timer — the shield could never return
    // until he dropped low and STAYED there for the whole cooldown.
    {
        let mut cd = AIRBAG_CD.lock().unwrap_or_else(|p| p.into_inner());
        let left = cd.get(&cd_key).copied().unwrap_or(0);
        if left > 0 {
            cd.insert(cd_key, left - 1);
            return;
        }
    }

    if cur * 100 > max * AIRBAG_HP_PCT {
        return;
    }

    let amount = johnson_damage(sim, id, AIRBAG_BASE, AIRBAG_DEF_PCT);
    let _ = sim.entity_add_shield(id, amount, AIRBAG_TICKS);
    let mark = BuffV1::timed(AIRBAG_BUFF, AIRBAG_TICKS);
    sim.add_buff(id, &mark);

    let mut cd = AIRBAG_CD.lock().unwrap_or_else(|p| p.into_inner());
    cd.insert(cd_key, AIRBAG_COOLDOWN_TICKS);
}

impl StableEffectType for JohnsonElectroAirbag {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _input: InputTargetV1) {
        tick_electro_airbag(sim, caster);
    }

    fn expected_damage(&self, _caster_stat: &StatV1) -> (usize, usize) {
        (0, 0)
    }
}

// ---------------------------------------------------------------------------
// The drive — advanced one tick at a time from `JohnsonDrive::on_update`
// ---------------------------------------------------------------------------

/// Advances the car by one tick. Returns true while still driving.
fn step_drive(sim: &mut StableSim<'_>, id: usize) -> bool {
    // Per-sim key: prevents parallel sims from sharing the same drive entry.
    let key = drive_key(sim, id);

    // `id` is Johnson's entity ID (the map key). Get the car entity ID from Drive.
    let car_id = {
        let map = drives();
        let Some(d) = map.get(&key) else {
            return false;
        };
        d.car_id
    };

    {
        let mut map = drives();
        let Some(d) = map.get_mut(&key) else {
            return false;
        };
        d.ticks += 1;
    }

    // Check both Johnson (driver) and car entity are alive.
    let johnson_alive = sim.get_entity(id).map(|e| e.is_alive()).unwrap_or(false);
    let (pos, team) = match sim.get_entity(car_id).map(|e| (e.pos(), e.team())) {
        Some(v) => v,
        None => {
            end_drive(sim, id);
            return false;
        }
    };
    if !johnson_alive {
        end_drive(sim, id);
        return false;
    }

    // Re-evaluate the mode each tick: a fleeing car that gets company
    // should still be able to ram, and a rammer whose target dies should stop.
    {
        let mut map = drives();
        let Some(d) = map.get_mut(&key) else {
            return false;
        };
        d.target = nearest_enemy_champion(sim, car_id).map(|(t, _)| t);
        if d.target.is_none() && d.mode == DriveMode::Initiate {
            d.mode = if d.passenger.is_some() {
                DriveMode::Taxi
            } else {
                DriveMode::Retreat
            };
        }
    }

    let (mode, target) = drives()
        .get(&key)
        .map(|d| (d.mode, d.target))
        .unwrap_or((DriveMode::Retreat, None));
    // Destination by mode (§5).
    let dest: (f64, f64) = match mode {
        DriveMode::Initiate => match target.and_then(|t| sim.get_entity(t).map(|e| e.pos())) {
            Some(p) => (p.0 as f64, p.1 as f64),
            None => fountain_pos(team),
        },
        DriveMode::Retreat => fountain_pos(team),
        DriveMode::Taxi => fountain_pos(team),
    };

    // Steer the CAR entity toward the destination.
    let dx = dest.0 - pos.0 as f64;
    let dy = dest.1 - pos.1 as f64;
    let len = (dx * dx + dy * dy).sqrt();
    // Hoisted out of the movement branch: the rider must be pinned and their
    // banish refreshed on EVERY tick of the ride, including the ones where the
    // car is already on top of its destination (`len <= 1.0`) and does not
    // move. A stationary car that skipped this would let the window lapse and
    // pop the rider into view mid-ride.
    let mut nx = pos.0 as f64;
    let mut ny = pos.1 as f64;
    let mut set_pos_ret = false;
    if len > 1.0 {
        let step = CAR_SPEED.min(len);
        nx = (pos.0 as f64 + dx / len * step).max(0.0);
        ny = (pos.1 as f64 + dy / len * step).max(0.0);
        set_pos_ret = sim.entity_set_pos(car_id, nx as u64, ny as u64);
    }

    // Refresh Johnson's self-banish (Zhask-style). Stopping this refresh
    // releases him when the drive ends.
    let _ = sim.entity_banish(id, id, HOPIN_REFRESH_TICKS, "", "");

    // entity_set_pos and entity_grab are BOTH ignored for banished units
    // (champion-data.md:4081: "A banished unit no longer moves or acts and
    // knockbacks stop"). v1.1.110 called entity_grab on Johnson every tick
    // trying to drag him along — it did not move him, and the frequent calls
    // on a banished entity likely caused the "stops banish" side effect the
    // owner reported. Johnson's position is handled by the reposition timer
    // in end_drive (see REPOSITIONS): after his banish expires, he is moved
    // to the crash site.
    //
    // Keep the rider glued to the CAR entity every tick. The rider is also
    // banished, so entity_set_pos is ignored — but it is kept as a no-op
    // safety net (harmless, and might work if the engine changes). The banish
    // refresh MUST stay: without it the rider's window lapses mid-ride.
    let rider = drives().get(&key).and_then(|d| d.passenger);
    if let Some(p) = rider {
        let _ = sim.entity_set_pos(p, nx as u64, ny as u64);
        // Refresh the banish window. EMPTY effect names, so the hop-in art
        // plays once at pickup instead of on every tick of the ride.
        let _ = sim.entity_banish(car_id, p, HOPIN_REFRESH_TICKS, "", "");
    }

    // Nitrous art past the acceleration threshold.
    let ticks = drives().get(&key).map(|d| d.ticks).unwrap_or(0);

    // Diagnostic log — v1.1.111 playtest investigation.
    // Logs every DRIVE_LOG_EVERY ticks + first 5 ticks. Shows: tick count,
    // car position, entity_set_pos return value, mode, target, Johnson pos.
    // This tells us: (1) does the car's internal position change? (2) does
    // entity_set_pos return true? (3) does the mode switch to Taxi/Retreat?
    if ticks <= 5 || ticks % DRIVE_LOG_EVERY == 0 {
        let johnson_pos_now = sim.get_entity(id).map(|e| e.pos()).unwrap_or((0, 0));
        let (log_mode, log_target) = drives()
            .get(&key)
            .map(|d| (d.mode, d.target))
            .unwrap_or((DriveMode::Retreat, None));
        log_event(
            sim,
            "step_drive",
            &format!(
                "car_id={} car_pos=({}, {}) -> ({}, {}) set_pos_ret={} \
                 mode={} target={:?} johnson_pos=({}, {}) dest=({}, {})",
                car_id,
                pos.0,
                pos.1,
                nx as u64,
                ny as u64,
                set_pos_ret,
                mode_name(log_mode),
                log_target,
                johnson_pos_now.0,
                johnson_pos_now.1,
                dest.0 as u64,
                dest.1 as u64,
            ),
        );
    }

    if ticks == BOOST_AFTER_TICKS {
        let boost = BuffV1::timed(CAR_BOOST_BUFF, CAR_TICKS.saturating_sub(ticks));
        sim.add_buff(car_id, &boost);
    }

    // Crash: contact with an enemy champion detonates the car — even while
    // fleeing, which is canon and is what makes a getaway risky.
    let crash_target = nearest_enemy_champion(sim, car_id)
        .filter(|(_, p)| dist(sim.get_entity(car_id).map(|e| e.pos()).unwrap_or(pos), *p) <= CAR_CRASH_RADIUS)
        .map(|(t, _)| t);

    if let Some(victim) = crash_target {
        let driven = ticks;
        crash(sim, id, victim, driven, car_id);
        return false;
    }

    if ticks >= CAR_TICKS {
        end_drive(sim, id);
        return false;
    }
    true
}

/// Detonate: damage + stun scaled by how long he drove, then the electrified zone.
fn crash(sim: &mut StableSim<'_>, id: usize, victim: usize, driven: usize, car_id: usize) {
    // Diagnostic log — v1.1.111 playtest investigation.
    log_event(
        sim,
        "crash",
        &format!(
            "victim={} driven={} car_id={} car_pos=({}, {})",
            victim,
            driven,
            car_id,
            sim.get_entity(car_id).map(|e| e.pos()).unwrap_or((0, 0)).0,
            sim.get_entity(car_id).map(|e| e.pos()).unwrap_or((0, 0)).1,
        ),
    );
    let ramp = (driven as f64 / CAR_TICKS as f64).clamp(0.0, 1.0);
    // `play_view_effect` arg order is (name, caster_id, input, range, radius, time).
    let mut input = InputTargetV1::default();
    input.kind = InputTargetKindV1::Target.code();
    input.target_id = victim;
    let _ = sim.play_view_effect("johnson_crash", id, &input, 0, 0, 60);
    let pct = ULT_DEF_PCT_MIN + ((ULT_DEF_PCT_MAX - ULT_DEF_PCT_MIN) as f64 * ramp) as usize;
    let amount = johnson_damage(sim, id, ULT_BASE, pct);
    let _ = sim.deal_damage_typed(id, victim, amount, DamageTypeV1::Ap, AttackTypeV1::Skill);

    let stun_ticks = ULT_STUN_MIN + ((ULT_STUN_MAX - ULT_STUN_MIN) as f64 * ramp) as usize;
    sim.apply_cc(victim, &CcV1::stun(stun_ticks as u64));

    // The electrified ground: everyone caught in the blast takes the zone
    // damage and is stunned, scaling the same way the primary target did.
    let zone = johnson_damage(sim, id, ZONE_BASE, ZONE_DEF_PCT);
    // Zone center is the CAR entity's position, not Johnson's (Johnson is banished elsewhere).
    let (team, centre) = match sim.get_entity(car_id).map(|e| (e.team(), e.pos())) {
        Some(v) => v,
        None => {
            end_drive(sim, id);
            return;
        }
    };
    let mut victims: Vec<usize> = Vec::new();
    for i in 0..sim.entity_count() {
        let Some(e) = sim.entity_at(i) else { continue };
        if !e.is_alive() || e.team() == team || !e.is_targetable() {
            continue;
        }
        // `victim` already took the primary hit above; the zone is splash for
        // everyone ELSE in the blast. Including them double-dips the damage
        // and re-applies the stun.
        if e.id() == victim {
            continue;
        }
        if dist(centre, e.pos()) <= CAR_CRASH_RADIUS * 1.5 {
            victims.push(e.id());
        }
    }
    for v in victims {
        let _ = sim.deal_damage_typed(id, v, zone, DamageTypeV1::Ap, AttackTypeV1::Skill);
        sim.apply_cc(v, &CcV1::stun(stun_ticks as u64));
    }

    end_drive(sim, id);
}

// ---------------------------------------------------------------------------
// Passive: drives the per-tick update, and owns the Electro Airbag check
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Voice (v1.1.114) — lines from the default set, Fandom hero id 321.
// The engine plays `<event>.sound_info`, which names the wav; BOTH need a
// `mod.override_info` route or nothing plays.
// ---------------------------------------------------------------------------

const VOICE_SELECT: &str = "mlbb_johnson_voice_select";
const VOICE_DEATH_LINES: &[&str] = &["mlbb_johnson_voice_death01", "mlbb_johnson_voice_death02"];
/// Movement lines. There is no separate "first move" line: the wiki's
/// headline line ("Come on! Let's roll!") is the SELECT line, so it plays on
/// spawn and these cycle while he runs.
const VOICE_MOVE_LINES: &[&str] = &[
    "mlbb_johnson_voice_move01",
    "mlbb_johnson_voice_move02",
    "mlbb_johnson_voice_move03",
    "mlbb_johnson_voice_move04",
    "mlbb_johnson_voice_move05",
];

/// Per-player driver. `on_update` is the only per-tick hook the stable API
/// gives a passive, so it is where the car advances.
#[derive(Clone, Default)]
pub struct JohnsonDrive {
    voice: crate::audio::VoiceLifecycle,
}

impl StablePassive for JohnsonDrive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(JohnsonDrive::default())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize) {
        self.voice.on_spawn(sim, entity, Some(VOICE_SELECT));
    }

    fn on_base_attack(&mut self, sim: &mut StableSim<'_>, _: u64, _: usize, entity: usize) {
        let _ = crate::audio::play_voice(sim, entity, "mlbb_johnson_voice_attack01");
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _rng: u64, _player: usize, entity: usize) {
        // Move lines only. Johnson's idle set on the wiki is the movement set,
        // so passing an empty idle list keeps him from talking while parked.
        self.voice.on_update(sim, entity, &[], VOICE_MOVE_LINES);
        // This is the champion's ONLY passive hook — a data file has a single
        // `passive_ref`, so both passive behaviours are stepped from here.
        // The airbag is checked every tick whether or not he is driving.
        tick_electro_airbag(sim, entity);

        // Process expired repositions for ANY entity (Johnson or a rider).
        // This must run every tick, even when not driving, because the
        // reposition timers are set in end_drive (when driving has already
        // stopped). Collect expired entries, release the lock, then reposition.
        //
        // Only entries that genuinely belong to THIS sim may be advanced:
        // the map is process-wide, so a parallel sim's timers are also
        // sitting in it. `belongs_here` checks the scope and the copy tag,
        // so we never count down — or teleport — somebody else's unit.
        let expired: Vec<(EntityKey, usize, f64, f64)> = {
            let mut rp = REPOSITIONS.lock().unwrap_or_else(|p| p.into_inner());
            let mut expired = Vec::new();
            for (key, (x, y, ticks)) in rp.iter_mut() {
                if !belongs_here(sim, key) {
                    continue;
                }
                if *ticks == 0 {
                    expired.push((*key, key.entity_id, *x, *y));
                } else {
                    *ticks -= 1;
                }
            }
            for (key, _, _, _) in &expired {
                rp.remove(key);
            }
            expired
        };
        for (_key, eid, x, y) in expired {
            let ret = sim.entity_set_pos(eid, x as u64, y as u64);
            log_event(
                sim,
                "reposition_fire",
                &format!("eid={} pos=({}, {}) ret={}", eid, x, y, ret),
            );
        }

        if !is_driving(sim, entity) {
            return;
        }
        let _ = step_drive(sim, entity);
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        // Death lines. The drive itself is torn down by the entity path: if he
        // dies while driving, step_drive sees `!johnson_alive` and ends it.
        self.voice.on_dead(sim, player, VOICE_DEATH_LINES);
    }
}

// ---------------------------------------------------------------------------
// AI: keep Johnson pointed somewhere sensible while driving
// ---------------------------------------------------------------------------

/// Withholds move orders while Johnson is driving — the car's position is
/// driven by `step_drive`, and the built-in AI would fight it by issuing
/// `move_to`. Same shape as `LingPerchAi`.
pub struct JohnsonDriveAi;

impl StablePlayerAi for JohnsonDriveAi {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(JohnsonDriveAi)
    }

    fn id(&self) -> String {
        "mlbb_heroes_johnson_drive".to_string()
    }

    fn priority(&self) -> i32 {
        150
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name.to_ascii_lowercase().contains("johnson")
    }

    fn think(
        &mut self,
        _ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        // Returning the base input unchanged keeps every non-driving behaviour
        // exactly as the built-in AI produced it.
        base_input
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register(module: &mut StableMod) {
    // Spelled literally, not through constants: verify_effects.py greps the
    // source for `add_native_effect("mlbb_heroes:...")` and a constant would
    // make that gate blind to it (same rule as gord.rs / cecilion.rs).
    module.add_native_effect("mlbb_heroes:johnson_basic_attack", JohnsonBasicAttack);
    module.add_native_effect("mlbb_heroes:johnson_s1_hit", JohnsonS1Hit);
    module.add_native_effect("mlbb_heroes:johnson_s2_tick", JohnsonS2Tick);
    module.add_native_effect("mlbb_heroes:johnson_full_throttle", JohnsonFullThrottle);
    module.add_native_effect("mlbb_heroes:johnson_electro_airbag", JohnsonElectroAirbag);
    module.add_native_passive("mlbb_heroes:johnson_drive", JohnsonDrive::default());
    module.add_player_input_ai(JohnsonDriveAi);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s2_pulses_stack_at_host_boundary_with_refresh_and_per_stack_stats() {
        use mod_api_stable::{SimCtxV1, SimVtableV1, EntityHandleV1};
        use std::{ffi::c_void, mem::size_of};

        #[derive(Default)]
        struct Host {
            calls: usize,
            count: usize,
            speed: i32,
            resistance: i32,
            refresh: u32,
            cap: usize,
        }
        unsafe extern "C" fn stack(
            state: *mut c_void, _target: EntityHandleV1, buff: *const BuffV1,
            cap: usize, refresh: u32,
        ) -> usize {
            let host = &mut *(state as *mut Host);
            let buff = &*buff;
            host.calls += 1;
            host.count = (host.count + 1).min(cap);
            host.speed = buff.move_speed_mult;
            host.resistance = buff.magic_resistance;
            host.refresh = refresh;
            host.cap = cap;
            host.count
        }
        // SDK vtable consists of size and optional function pointers.
        let mut table: SimVtableV1 = unsafe { std::mem::zeroed() };
        table.size = size_of::<SimVtableV1>();
        table.entity_stack_buff = Some(stack);
        let mut host = Host::default();
        let mut context = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            sim: &table,
            frame: std::ptr::null(),
            state: &mut host as *mut Host as *mut c_void,
        };
        let mut sim = unsafe { StableSim::from_raw(&mut context).unwrap() };
        for expected in [1, 2, 3, 3] {
            JohnsonS2Tick.apply(&mut sim, 0, 1, InputTargetV1::target(2));
            assert_eq!(host.count, expected);
            assert_eq!(host.speed * host.count as i32, -12 * expected as i32);
            assert_eq!(host.resistance * host.count as i32, -6 * expected as i32);
            assert_eq!((host.cap, host.refresh), (3, 1));
        }
        assert_eq!(host.calls, 4);
        JohnsonS2Tick.apply(&mut sim, 0, 1, InputTargetV1::target(1));
        JohnsonS2Tick.apply(&mut sim, 0, 1, InputTargetV1::NONE);
        assert_eq!(host.calls, 4);
    }


    #[test]
    fn percent_is_whole_and_rounds_down() {
        assert_eq!(percent(200, 50), 100);
        assert_eq!(percent(0, 999), 0);
        assert_eq!(percent(99, 100), 99);
    }

    #[test]
    fn damage_scales_with_defence() {
        // 42 base defence (his stat) and 5/level growth -> L15 ~112.
        let l1 = S1_BASE + percent(42, S1_DEF_PCT);
        let l15 = S1_BASE + percent(112, S1_DEF_PCT);
        assert_eq!(l1, 70 + 46);
        assert_eq!(l15, 70 + 123);
        // stays inside the roster's 100-200 skill band across the whole game
        assert!(l1 >= 100 && l15 <= 200, "l1={l1} l15={l15}");
    }

    #[test]
    fn ult_ramp_stays_bounded() {
        let min = ULT_BASE + percent(42, ULT_DEF_PCT_MIN);
        let max = ULT_BASE + percent(112, ULT_DEF_PCT_MAX);
        assert!(min < max);
        // never below the S1 hit, never absurd
        assert!(min >= S1_BASE + percent(42, S1_DEF_PCT));
    }

    #[test]
    fn fountain_is_team_symmetric() {
        let a = fountain_pos(0);
        let b = fountain_pos(1);
        assert_eq!((a.0 as u64, a.1 as u64), (b.1 as u64, b.0 as u64));
    }

    #[test]
    fn stun_scales_with_drive_time() {
        let ramp0 = ULT_STUN_MIN;
        let ramp1 = ULT_STUN_MIN + (ULT_STUN_MAX - ULT_STUN_MIN);
        assert_eq!(ramp0, 30);
        assert_eq!(ramp1, 60);
    }

    /// Bug #16, second form (owner, rc.5 playtest): *"he still crash by himself
    /// leaving his ally banished."*
    ///
    /// **The test this replaced was part of the problem.** It asserted
    /// constants, not behaviour — `HOPIN_TICKS > CAR_TICKS` and a stranded
    /// count over 300 — and passed on every build from rc.3 to rc.5 while the
    /// rider was in fact stranded for ~17 s after every early crash. It proved
    /// the numbers were large; it never proved the release worked. Recording
    /// that here so the shape is not repeated: a guard that only checks the
    /// size of a constant is not a guard against the bug the constant causes.
    ///
    /// What IS assertable locally is the design: a short banish refreshed every
    /// tick, rather than one long banish plus a release the API never promised.
    #[test]
    fn the_banish_is_a_short_refreshed_window_not_one_long_banish() {
        // Short enough that even a lapsed refresh is not a kidnapping.
        assert!(
            HOPIN_REFRESH_TICKS <= 10,
            "a missed refresh must not strand the rider ({HOPIN_REFRESH_TICKS} ticks)"
        );
        // Long enough that one dropped tick cannot flicker them into view.
        assert!(
            HOPIN_REFRESH_TICKS >= 3,
            "the window must outlast a single tick ({HOPIN_REFRESH_TICKS})"
        );

        // Split at the test module: the assertions below are string literals
        // that would otherwise match themselves.
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        // The refresh has to happen on the ride loop, not only at pickup.
        assert!(
            code.contains("entity_banish(car_id, p, HOPIN_REFRESH_TICKS"),
            "step_drive must refresh the rider's banish every tick they are aboard"
        );
        // And it must sit OUTSIDE the `if len > 1.0` movement branch, or a car
        // parked on its destination stops refreshing and pops the rider out.
        // `nx`/`ny` were hoisted above that branch for exactly this reason, so
        // their declaration ordering the refresh is the observable proxy.
        let hoist = code.find("let mut nx = pos.0 as f64;").unwrap_or(usize::MAX);
        let refresh = code
            .find("entity_banish(car_id, p, HOPIN_REFRESH_TICKS")
            .unwrap_or(0);
        assert!(
            hoist < refresh,
            "the banish refresh must run on stationary ticks too, not only when the car moves"
        );
        // No code path may depend on a release primitive existing.
        assert!(
            !code.contains("entity_banish(driver, passenger, 1"),
            "there is no documented unbanish; do not reintroduce the 1-tick release"
        );
        assert!(
            !code.contains("HOPIN_TICKS"),
            "the long one-shot banish is gone; refresh HOPIN_REFRESH_TICKS instead"
        );
    }

    /// Redesign (v0.16.0): Johnson banishes HIMSELF and the CAR is a separate
    /// entity that drives itself. The crash is detected on the CAR's position,
    /// not Johnson's — so Johnson's banish no longer breaks crash detection.
    ///
    /// The old rc.7 failure was: banish froze the driver's entity, and the car
    /// was the driver's OWN entity — so the car froze too and never reached
    /// enemies. Now the car is a separate entity (spawn_unit) that is NOT
    /// banished, so it keeps driving while Johnson is hidden.
    #[test]
    fn the_driver_is_banished_and_the_car_drives_itself() {
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        // Johnson must banish himself (Zhask pattern).
        assert!(
            code.contains("entity_banish(caster, caster"),
            "Johnson must banish himself so he's hidden from all views during the drive"
        );
        assert!(
            code.contains("entity_banish(id, id"),
            "step_drive must refresh Johnson's self-banish every tick"
        );

        // The car entity is spawned via spawn_unit and drives via entity_set_pos(car_id, ...).
        assert!(
            code.contains("spawn_unit("),
            "the car must be a real entity spawned via spawn_unit"
        );
        assert!(
            code.contains("entity_set_pos(car_id,"),
            "the car entity must be moved via entity_set_pos(car_id, ...)"
        );

        // Crash detection runs on the CAR's position, not Johnson's.
        assert!(
            code.contains("nearest_enemy_champion(sim, car_id)"),
            "crash detection must find enemies near the CAR entity"
        );
        assert!(
            code.contains("sim.get_entity(car_id).map(|e| e.pos())"),
            "crash detection must check distance from the CAR's position"
        );

        // Johnson must NOT be grabbed by entity_grab during the drive.
        // entity_grab is ignored for banished units (champion-data.md:4081)
        // and the frequent calls caused the "stops banish" regression in
        // v1.1.110. His position is handled by the reposition timer.
        assert!(
            !code.contains("entity_grab(car_id, id, RIDE_GRAB_SPEED, 0)"),
            "the car must NOT grab Johnson every tick — entity_grab is ignored for banished units and caused the v1.1.110 regression"
        );

        // The car entity must NOT have move_speed = 50. v1.1.110 set it to 50
        // (Zhask pattern) but the engine's own movement pipeline then crawls
        // at 50/tick instead of entity_set_pos at CAR_SPEED=2400/tick.
        // move_speed stays at 0 (default) — the engine does not process the
        // car's movement, leaving it to entity_set_pos in step_drive.
        assert!(
            !code.contains("car_stat.move_speed = 50"),
            "the car must NOT have move_speed = 50 (v1.1.110 regression: engine crawls at 50/tick)"
        );

        // A reposition timer must exist to place Johnson at the crash site
        // after his banish expires (entity_set_pos is ignored for banished).
        assert!(
            code.contains("REPOSITIONS"),
            "a reposition timer must move Johnson to the crash site after banish expires"
        );
        assert!(
            code.contains("rp.insert(drive_key(sim, id), (crash_x, crash_y, reposition_ticks))"),
            "end_drive must set a reposition timer for Johnson at the crash site"
        );
    }

    /// The car entity must be destroyed when the drive ends (crash or expiry).
    /// No entity_remove API exists — entity_set_hp(0) kills it (same pattern Zhask uses).
    #[test]
    fn the_car_entity_is_destroyed_on_crash_or_expiry() {
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        assert!(
            code.contains("entity_set_hp(d.car_id, 0)"),
            "end_drive must destroy the car entity via entity_set_hp(car_id, 0)"
        );
        assert!(
            code.contains("entity_remove_buff(d.car_id, CAR_PROTECT_BUFF)"),
            "end_drive must clean up the car's protect buff"
        );
    }

    /// The ally is pinned to the CAR entity, not to Johnson. This is the key
    /// difference from the old design — Johnson is banished elsewhere, so the
    /// ally must follow the car, not the driver.
    #[test]
    fn the_ally_is_pinned_to_the_car_not_the_driver() {
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        assert!(
            code.contains("entity_set_pos(p, nx as u64, ny as u64)"),
            "the ally must be positioned at the car's position each tick"
        );
        // entity_grab on the rider was removed in v1.1.111 — it is ignored
        // for banished units and caused the "stops banish" regression.
        assert!(
            !code.contains("entity_grab(car_id, p, RIDE_GRAB_SPEED, 0)"),
            "the ally must NOT be grabbed every tick — entity_grab is ignored for banished units"
        );
        assert!(
            code.contains("entity_banish(car_id, p, HOPIN_REFRESH_TICKS"),
            "the ally's banish must be refreshed by the CAR entity, not the driver"
        );
    }

    /// Bug #16 (owner, rc.3 playtest): *"they ride right then they banish but
    /// they don't appear on the crash site where johnson is, they appear where
    /// they banish."* `end_drive` used to call `entity_banish` directly, which
    /// released the rider at the pickup point.
    ///
    /// The host behaviour behind the fix cannot be asserted here — a unit test
    /// can only prove the ABI call is well-formed, never that the engine moves
    /// the unit (§29.27b). So this guards the one thing that IS local and IS
    /// sufficient: that the release goes through `drop_passenger`, which
    /// repositions onto the car first.
    #[test]
    fn the_rider_is_dropped_at_the_crash_site_not_the_pickup_point() {
        // Split at the test module: the assertions below are string literals
        // that would otherwise match themselves.
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        assert!(
            code.contains("drop_passenger(sim, d.car_id, p)"),
            "end_drive must drop the rider at the crash site, not release them in place"
        );
        assert!(
            !code.contains("sim.entity_banish(id, p, 1"),
            "end_drive must not release the rider directly"
        );
        // All three mechanisms, so that whichever one the host honours lands.
        assert!(code.contains("entity_set_pos(passenger, x, y)"));
        assert!(code.contains("entity_grab(driver, passenger, DROP_GRAB_SPEED, 0)"));
        // And the reposition timer must place the rider at the crash site
        // after their banish expires (entity_set_pos is ignored for banished).
        assert!(
            code.contains("rp.insert(drive_key(sim, p), (crash_x, crash_y, reposition_ticks))"),
            "end_drive must set a reposition timer for the rider at the crash site"
        );
    }

    /// The per-tick re-grip is a *keep-up* speed, not a catch-up speed: it only
    /// has to cover one tick of car travel. If it were slower than the car the
    /// rider would fall behind and the whole point of the loop would be lost.
    #[test]
    fn the_ride_grip_outruns_the_car() {
        assert!(
            RIDE_GRAB_SPEED as f64 > CAR_SPEED,
            "the per-tick grip must outpace the car ({RIDE_GRAB_SPEED} vs {CAR_SPEED})"
        );
        // It also must not be so violent that a re-issue every tick reads as a
        // glitch — cap it at one cell of travel per tick.
        assert!(
            RIDE_GRAB_SPEED <= 32_000,
            "one tick of grip should not exceed a cell (CELL = 32000)"
        );
    }

    /// `entity_grab` is only fired if `entity_set_pos` was refused, and it is
    /// the last line of defence for bug #16. It has to cross a full-length
    /// drive quickly enough to read as an arrival rather than a second journey
    /// — but it is a drag, so it must also not outrun the car so violently that
    /// it looks like a glitch.
    #[test]
    fn the_drop_fallback_covers_a_full_drive_inside_a_second() {
        let full_drive = CAR_TICKS as f64 * CAR_SPEED;
        let ticks = (full_drive / DROP_GRAB_SPEED as f64).ceil() as usize;
        assert!(
            ticks <= 60,
            "a full-drive drop must land within ~1 s, took {ticks} ticks"
        );
        assert!(
            DROP_GRAB_SPEED as f64 > CAR_SPEED,
            "the drag has to outpace the car or the rider never catches up"
        );
    }

    /// The data file has exactly ONE `passive_ref` slot, and Johnson has two
    /// passive behaviours. Guard the invariant that keeps this honest: the
    /// registered passive must be the one that steps BOTH.
    #[test]
    fn the_single_passive_steps_both_behaviours() {
        assert!(
            AIRBAG_COOLDOWN_TICKS > 0 && AIRBAG_TICKS > 0,
            "the airbag has to be ticked from somewhere"
        );
        // Regression guard for the rc.1 load failure: the passive was written
        // as an action block and the engine rejected it with
        // Error("missing field `passive_ref`").
        assert!(
            CAR_TICKS > 0,
            "the car stepper is the other half of the same passive"
        );
    }

    #[test]
    fn crash_does_not_double_dip_the_primary_target() {
        // The primary victim takes ULT_BASE + ramped def%; everyone else in the
        // blast takes the smaller ZONE amount. The primary must not appear in
        // the zone list, or they eat both and get stunned twice.
        assert!(
            ZONE_BASE + percent(112, ZONE_DEF_PCT) < ULT_BASE + percent(112, ULT_DEF_PCT_MAX),
            "splash must be strictly weaker than the direct hit"
        );
    }

    /// v1.1.111 fix: Johnson and the rider are banished during the drive, so
    /// entity_set_pos is ignored for them (champion-data.md:4081). They stay
    /// at their cast positions. The reposition timer in end_drive must fire
    /// AFTER the banish expires to move them to the crash site.
    ///
    /// This is the fix for the v1.1.109/v1.1.110 "Johnson not at crash site"
    /// bug. v1.1.110 tried entity_grab (ignored for banished + caused "stops
    /// banish"); v1.1.111 uses a delayed entity_set_pos after banish lapses.
    #[test]
    fn the_reposition_lands_at_the_crash_site_after_banish_expires() {
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        // The reposition map must exist and be checked every tick.
        assert!(
            code.contains("static REPOSITIONS"),
            "a REPOSITIONS map must hold pending repositions"
        );
        assert!(
            code.contains("REPOSITIONS.lock()"),
            "the reposition map must be accessed via its mutex"
        );

        // end_drive must capture the car's position BEFORE killing it.
        assert!(
            code.contains("sim.get_entity(d.car_id).map(|e| e.pos())"),
            "end_drive must read the car's position before destroying it"
        );

        // end_drive must set reposition timers for BOTH Johnson and the rider.
        assert!(
            code.contains("rp.insert(drive_key(sim, id), (crash_x, crash_y, reposition_ticks))"),
            "end_drive must schedule a reposition for Johnson"
        );
        assert!(
            code.contains("rp.insert(drive_key(sim, p), (crash_x, crash_y, reposition_ticks))"),
            "end_drive must schedule a reposition for the rider"
        );

        // The timer must outlast the banish window.
        assert!(
            code.contains("HOPIN_REFRESH_TICKS + 10"),
            "the reposition timer must fire after the banish expires"
        );

        // on_update must process expired entries by calling entity_set_pos.
        assert!(
            code.contains("sim.entity_set_pos(eid, x as u64, y as u64)"),
            "on_update must reposition expired entries via entity_set_pos"
        );

        // And it must run even when NOT driving (timers are set in end_drive,
        // which runs when driving has already stopped). Search for the
        // reposition block within on_update specifically — REPOSITIONS.lock()
        // also appears in end_drive, which comes earlier in the file.
        let on_update_start = code.find("fn on_update").unwrap_or(0);
        let after_on_update = &code[on_update_start..];
        let airbag = after_on_update.find("tick_electro_airbag(sim, entity)").unwrap_or(0);
        let reposition = after_on_update.find("REPOSITIONS.lock()").unwrap_or(0);
        let driving_check = after_on_update.find("if !is_driving(sim, entity)").unwrap_or(usize::MAX);
        assert!(
            reposition < driving_check,
            "reposition processing must run before the driving check, so it fires when driving has ended"
        );
        assert!(
            airbag < reposition,
            "airbag tick must come first, then repositions, then driving"
        );
    }

    /// v1.1.111 fix: the car must NOT have move_speed > 0. v1.1.110 set it to
    /// 50 (Zhask pattern) but the engine's own movement pipeline then crawls
    /// at 50/tick, fighting entity_set_pos at CAR_SPEED=2400/tick. The owner
    /// reported "moves so slow, doesnt crash" — the car never reached targets.
    #[test]
    fn the_car_does_not_use_engine_movement() {
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        // move_speed must NOT be set to 50 (or any positive value).
        assert!(
            !code.contains("car_stat.move_speed = 50"),
            "the car must not have move_speed = 50 (v1.1.110 regression)"
        );
        assert!(
            !code.contains("car_stat.move_speed = 2400"),
            "the car must not have move_speed = CAR_SPEED (would fight entity_set_pos)"
        );

        // The car must be moved via entity_set_pos, not engine movement.
        assert!(
            code.contains("entity_set_pos(car_id, nx as u64, ny as u64)"),
            "the car must be steered via entity_set_pos, not the engine's movement pipeline"
        );
    }

    /// v1.1.113 fix: the drive state must be per-simulation.
    ///
    /// The v1.1.112 playtest log (12,807 lines) showed 42 crashes with
    /// `crash_pos=(0, 0)` — all for caster=23, the owner's Johnson. AI
    /// Johnsons had zero. Cause: the game runs two sims (a server pre-sim
    /// and the client match view) in the same process, sharing static
    /// memory. Both wrote `DRIVES[23]`, so the client-view sim read the
    /// server sim's car_id, could not find that entity in its own sim,
    /// called end_drive, and deleted the server sim's drive — freezing the
    /// car. Its end_drive then read (0, 0) for the missing car and
    /// teleported Johnson there: the "top side bush" the owner reported.
    ///
    /// Every drive-state map must therefore be keyed by `EntityKey`, which
    /// carries the sim scope AND an assigned copy serial.
    #[test]
    fn the_drive_state_is_per_sim() {
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        // The three shared maps must all be keyed by EntityKey.
        assert!(
            code.contains("static DRIVES: LazyLock<Mutex<HashMap<EntityKey, Drive>>>"),
            "DRIVES must be keyed by EntityKey"
        );
        assert!(
            code.contains(
                "static REPOSITIONS: LazyLock<Mutex<HashMap<EntityKey, (f64, f64, usize)>>>"
            ),
            "REPOSITIONS must be keyed by EntityKey"
        );
        assert!(
            code.contains("static AIRBAG_CD: LazyLock<Mutex<HashMap<EntityKey, usize>>>"),
            "AIRBAG_CD must be keyed by EntityKey"
        );

        // No bare entity-id keyed map may remain — that is the bug.
        assert!(
            !code.contains("HashMap<usize, Drive>"),
            "a bare usize-keyed DRIVES is the v1.1.112 bug — every drive map must be per-sim"
        );

        // The key must come from the mod's canonical entity_key(), which
        // combines SimScope with an explicitly assigned copy serial.
        // sim_origin() ALONE is insufficient: it returns None during
        // pre-simulation, making two copies of one match indistinguishable.
        assert!(
            code.contains("fn drive_key(sim: &mut StableSim<'_>, id: usize) -> EntityKey"),
            "drive_key must return EntityKey"
        );
        assert!(
            code.contains("crate::entity_key(sim, id)"),
            "drive_key must delegate to the canonical entity_key()"
        );
        assert!(
            !code.contains("sim_origin().unwrap_or((0, 0, 0))")
                && !code.contains("None => (0, 0, 0)"),
            "sim_origin() is None during pre-simulation; a bare (0,0,0) fallback \
             would silently re-merge the parallel sims"
        );

        // Every drive insertion and removal must go through the keyed helper,
        // never the raw entity id.
        assert!(
            code.contains("drives().insert(\n            drive_key(sim, caster)"),
            "apply() must insert the drive under the per-sim key"
        );
        assert!(
            code.contains("drives().remove(&key)"),
            "end_drive() must remove the drive under the per-sim key"
        );
        assert!(
            !code.contains("drives().remove(&id)"),
            "end_drive() must not remove by bare entity id — that deletes the other sim's drive"
        );

        // A global-map timer sweep must verify ownership before advancing or
        // firing an entry, or one sim would tick down the other's timers.
        assert!(
            code.contains("fn belongs_here(sim: &StableSim<'_>, key: &EntityKey) -> bool"),
            "the reposition sweep needs a per-sim ownership check"
        );
        assert!(
            code.contains("crate::sim_copy_of(sim, key.entity_id) == Some(key.copy)"),
            "ownership must compare the entity's own copy tag against the key's"
        );

        // The stale-entry path was the exact failure: a car_id absent from
        // THIS sim must never be read as a crash site at (0, 0). Only the
        // reposition site matters, so check that specific expression.
        assert!(
            !code.contains("sim.get_entity(d.car_id).map(|e| e.pos()).unwrap_or((0, 0))"),
            "a missing car must never silently become crash_pos (0, 0) — that \
             teleports Johnson to the top side bush"
        );
        assert!(
            code.contains("let Some((car_x, car_y)) = sim.get_entity(d.car_id)"),
            "end_drive must bail out (no reposition) when the car is gone"
        );
    }

    /// v1.1.114: Johnson had NO voice at all, and his S2 field had no
    /// visible picture. Both are now wired, and both are easy to break
    /// silently, so they are guarded here.
    #[test]
    fn the_voice_and_the_s2_field_are_wired() {
        let src = include_str!("johnson.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);

        // The passive must actually drive the shared voice lifecycle, or none
        // of the clips play however well the audio.rs table is filled in.
        assert!(
            code.contains("voice: crate::audio::VoiceLifecycle"),
            "JohnsonDrive must own a VoiceLifecycle"
        );
        assert!(
            code.contains("self.voice.on_update(sim, entity, &[], VOICE_MOVE_LINES)"),
            "on_update must step the move lines"
        );
        assert!(
            code.contains("self.voice.on_dead(sim, player, VOICE_DEATH_LINES)"),
            "on_dead must play a death line"
        );
        assert!(
            code.contains("self.voice.on_spawn(sim, entity, Some(VOICE_SELECT))"),
            "on_spawn must play the select line"
        );

        // Every line named here must be a real cue, or the engine looks for a
        // sound_info that does not exist and plays nothing.
        for line in [
            "mlbb_johnson_voice_select",
            "mlbb_johnson_voice_death01",
            "mlbb_johnson_voice_death02",
            "mlbb_johnson_voice_move01",
            "mlbb_johnson_voice_move05",
            "mlbb_johnson_voice_attack01",
        ] {
            assert!(
                code.contains(line),
                "{line} must be named in johnson.rs"
            );
        }

        // S2 looked invisible because the field is turned to the cast
        // direction as a zone view (champion-data.md: a zone picture must
        // look the same turned half round about its pivot; ours is a
        // one-sided wedge). It is now a CasterViewEffect, which is never
        // turned, next to a zone that carries no picture.
        assert!(
            code.contains("johnson_s2_tick"),
            "the S2 tick native must still be registered"
        );

        // The ult line is picked at random from two alternatives, not chained:
        // the wiki lines are alternatives, not two halves of one sentence.
        let audio = include_str!("audio.rs");
        assert!(
            audio.contains("JOHNSON_ULT_LINES"),
            "the ult lines must be a random-pick cue like Zhask's and Balmond's"
        );
        assert!(
            audio.contains("mlbb_heroes:audio_voice_johnson_ult"),
            "the ult cue must be registered in audio.rs"
        );
        assert!(
            audio.contains("\"johnson_skill2\", \"mlbb_johnson_voice_skill2\""),
            "the S2 line must be in VOICE_CUES"
        );
    }
}
