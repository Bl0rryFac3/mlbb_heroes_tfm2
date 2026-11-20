//! Native Obsidia gameplay — Return to Bone (passive + bone-shard counter),
//! Abyssal Bone Needle (S1), Phantom Shadowmeld (S2) and Hunt of Bone (ult).
//!
//! Modelled on `alpha_work/src/alpha.rs`. Every API call here is copied from a
//! verified call site; nothing is invented:
//!   shard / energy counters  <- Aldous's capped stack counters and Alpha's
//!                               `entity_stack_buff` mark stacks
//!   basic attack + hit pair  <- Karrie's `karrie_basic_attack` /
//!                               `karrie_basic_hit` projectile pair
//!   tether beam rendering    <- Estes's `draw_tether_sprites` (12 pre-rotated
//!                               bins tiled along the link)
//!   ForceMove                <- obsidia_work/spike_forcemove
//!   view/voice gating        <- Alpha's `can_render` / `speak`
//!
//! Registration (in candidate/src/lib.rs, next to `alpha::register`):
//!     mod obsidia;
//!     obsidia::register(&mut module);
//!
//! Native entry points:
//!   passive  mlbb_heroes:obsidia_return_to_bone   shard counter + tether leash
//!   effect   mlbb_heroes:obsidia_basic_attack     spawns the shard projectile
//!   effect   mlbb_heroes:obsidia_basic_hit        basic damage + shards + energy
//!   effect   mlbb_heroes:obsidia_needle_hit       S1 damage + skill energy
//!   effect   mlbb_heroes:obsidia_shadowmeld       S2 movement-speed burst
//!   effect   mlbb_heroes:obsidia_hunt             ult cast: clears the lock
//!   effect   mlbb_heroes:obsidia_hunt_hit         ult payload on first hero hit
//!
//! NOT YET VERIFIED IN MATCH. Two behaviours in particular need the owner's
//! eyes and cannot be proven from the SDK alone: (1) the ult's `ForceMove`
//! pull — the spike proved the ABI call is well-formed, not that the engine
//! moves the unit; (2) `CcKindV1::BlockMoveSkill` as the anti-displacement
//! stand-in. See OBSIDIA_SPEC.md §10 and §17.

use mod_api_stable::*;

// ---------------------------------------------------------------------------
// Balance constants (OBSIDIA_SPEC.md §10, priced in §3)
// ---------------------------------------------------------------------------

/// Shard cap. 8 x 5% AD = +40% of total AD at cap (165 AD at L15).
/// Canon's ceiling. The old 8 was reached after ~32 hero hits and then she
/// coasted for the rest of the match -- the wrong shape for a late-game
/// carry. Raised 2026-11-05; the accrual constants below move with it so
/// she does not simply arrive at the new cap sooner.
const SHARD_CAP: usize = 25;
/// Each stored shard strikes the target for this share of total AD.
// Canon: each stored shard strikes for 7% of Total PA. Was 5; raised
// 2026-11-05 alongside the energy slowdown so the slower stack still pays.
const SHARD_DAMAGE_PERCENT: usize = 7;
/// Bone Energy needed to forge one shard.
// Canon: 30 Bone Energy forges one shard. Was 12 -- which made the whole
// economy 2.5x too cheap, so she hit the cap of 25 inside four minutes in
// every scenario, including pure farming. In-game report 2026-11-05.
const ENERGY_PER_SHARD: usize = 30;
/// Energy from a landed basic attack on an enemy **hero**.
const ENERGY_PER_BASIC_HIT: usize = 2;
/// Energy from a skill hit on an enemy hero.
///
/// Canon grants skills 3x a basic. That ratio is deliberately 2x here, and
/// stays 2x after the 2026-11-05 rebalance (4 vs a basic's 2): 3x would pay
/// 1.5 shards per two skill hits, faster than the rate the owner approved.
/// Easy to raise if she reads as too slow.
///
/// At ENERGY_PER_SHARD = 30 that is 15 hero basics, 30 minion hits, or 7.5
/// skill hits, per shard -- canon's own pacing.
const ENERGY_PER_SKILL_HIT: usize = 4;

/// Energy from a basic attack on a **minion or tower**.
///
/// This is the fix for the biggest adaptation error in the kit. The spec
/// dropped canon's non-hero trickle on the reasoning that "there are no creeps
/// in this roster" -- but creeps are not minions. Canon denies Energy only
/// against JUNGLE creeps; lane minions and towers have always fed her. Without
/// this she has no passive at all while farming, which is most of the laning
/// phase. Heroes still pay double (ENERGY_PER_BASIC_HIT), preserving canon's
/// 2:1 hero-to-minion ratio.
///
/// Jungle camps, epic monsters, lords and summons are still excluded: those
/// are the "creeps" the spec meant to exclude.
const ENERGY_PER_NON_HERO_HIT: usize = 1;

/// Basic attack: 40 + 65% AD, plus every stored shard.
const BASIC_DAMAGE_BASE: usize = 40;
const BASIC_ATTACK_RATIO: usize = 65;
/// S1 Abyssal Bone Needle: (90 -> 140) + 60% AD.
const NEEDLE_DAMAGE_LOW: usize = 90;
const NEEDLE_DAMAGE_HIGH: usize = 140;
const NEEDLE_ATTACK_RATIO: usize = 60;
/// Ult Hunt of Bone: 55 + 50% AD.
const ULT_DAMAGE_BASE: usize = 55;
const ULT_ATTACK_RATIO: usize = 50;

// ---------------------------------------------------------------------------
// Timing (60 ticks/sec — OBSIDIA_SPEC.md §14)
// ---------------------------------------------------------------------------

/// Tether duration: canon 3 s.
const ULT_TETHER_TICKS: usize = 180;
/// Leash: canon "cannot move more than 4 units away". 1 canon unit = 10,000,
/// so 4 units = 40,000. She is tethered well inside her own 60,000 basic-attack
/// range, which is exactly what the tether is for. (Built when her range was
/// 46,000; still comfortably inside since v2.0.8 raised it to 60,000.)
const ULT_TETHER_LEASH: f64 = 40_000.0;
/// Where the opening pull parks her. Inside the leash so the pull reads as a
/// yank to the target rather than a stop at the tether's edge.
const ULT_PULL_STANDOFF: f64 = 30_000.0;
/// How long `on_update` keeps snapping her to the standoff. Generous: at
/// PULL_SPEED she covers 120,000 in this window, more than the ult's reach.
const ULT_PULL_TICKS: usize = 20;
/// Units per tick for the opening pull. Alpha's ult `MoveTo` uses 6,000, which
/// is ~5.5x a 1,080 move speed — the same ratio a dash should have.
const ULT_PULL_SPEED: f64 = 6_000.0;
/// "Briefly stuns" — 0.5 s.
const ULT_STUN_TICKS: usize = 30;
/// Temporary shards the tether grants; these may exceed the cap of 8.
const ULT_TEMP_SHARDS: usize = 5;
/// Shield 200 (+25 per shard) at canon 2,320 HP is ~17% of her health bar.
/// Scaled to this roster's 720 HP that is ~122, so 60 + 8 per shard -> 124 at
/// the 8-shard cap. Computed from the shard count **before** the +5 temporary
/// shards land — the conservative reading, and easy to buff.
const ULT_SHIELD_BASE: usize = 60;
const ULT_SHIELD_PER_SHARD: usize = 8;
/// Canon: +30% Attack Speed for the duration of the tether.
const ULT_ATTACK_SPEED_MULT: i32 = 30;
/// S2's cooldown drops to 33% during the tether: 270 -> 90, so 180 off.
const ULT_S2_CD_CUT: usize = 180;

/// S2 Phantom Shadowmeld: canon +500% Movement Speed for 0.3 s.
///
/// This is a *percentage*, so the roster's ~1/3 stat scaling does not apply to
/// it the way it does to HP and AD (whose numbers really are ~1/3 of canon).
/// +500% means she covers in 0.3 s what would normally take 1.8 s: a short
/// repositioning burst, exactly as canon reads. Enhanced to 1 s during the ult.
/// Roster band for `move_speed_mult` is -80..+20 (Karrie's +20 is the current
/// max). Canon's +500% is NOT portable: it compounds with time, and honoured
/// literally it moves her ~117 px in 0.3 s — further than Uranus's tank
/// engage. The dash is carried by the data's `MoveTo` now, so this is only the
/// residual sprint after the dash lands.
const S2_MOVE_SPEED_MULT: i32 = 20;
const S2_TICKS: usize = 18;
const S2_ULT_TICKS: usize = 60;

/// Basic-attack projectile, matching Karrie's wheel.
const BASIC_PROJECTILE_SPEED: u64 = 6_000;
const BASIC_PROJECTILE_RADIUS: u64 = 6_000;
const BASIC_PROJECTILE: &str = "obsidia_basic_shard";
const BASIC_HIT_EFFECT: &str = "mlbb_heroes:obsidia_basic_hit";
const SHARD_HIT_EFFECT: &str = "mlbb_heroes:obsidia_shard_visual";

/// The stored Bone Shards fly alongside the basic attack — **one projectile per
/// stored shard**, fanned out so the number is legible in the air.
///
/// THIS IS THE STACK COUNTER. The owner corrected this on 2026-11-04: the shard
/// count is read off how many projectiles her basic attack throws, not off her
/// tail. It is also what canon says — "on Basic Attack, ALL stored shards
/// strike the target" — so N shards has to mean N things in the air. See
/// OBSIDIA_SPEC.md §19, which withdraws §16's tail claim.
///
/// The shards are COSMETIC: all shard damage still rides on the main hit, so
/// the damage model is unchanged and every unit test stays valid. Making each
/// shard deal its own 5% is a one-line change if that is wanted instead.
/// Lateral jitter between shards.
///
/// Was 5,000 when the shards flew abreast: 25 of them at 5 px is a 125 px
/// wall, and at the art's 38 px width they overlap ~7x -- wider than the
/// 848 px map at full separation. Now that shards leave one at a time the
/// spread only has to keep consecutive shots from stacking exactly, so 1.2 px
/// is plenty.
const SHARD_FAN_SPACING: f64 = 1_200.0;

// ---------------------------------------------------------------------------
// Cosmetic shard stream (2026-11-05)
// ---------------------------------------------------------------------------
//
// Shards leave her hand one at a time instead of all on the same tick, so a
// full stack reads as sustained fire -- the "machine gun" the owner asked for.
// BoDUD does the same thing for Kai'Sa's Q: "one missile every 2 ticks", and
// warns that a skill tree is copied every tick so node count costs framerate.
// Hers is native (her attack block is 2 nodes), but the same economy applies:
// this spawns AT MOST ONE projectile per tick, so a 16-shard stream costs the
// same per tick as a 1-shard one.
//
// Damage is untouched. The main projectile still pays every shard, so these
// remain decoration and no damage test changes.
//
// State lives in BUFFS, not in a global map. The game runs two simulations of
// one match concurrently with identical entity ids, and global state keyed by
// bare entity id is shared between them and corrupts both -- see
// obsidia_work/apply_simkey_fix.py. Obsidia is otherwise entirely buff-based;
// a global here would be the one place she could desync.

/// Ticks between consecutive shards in the stream.
const SHARD_STREAM_INTERVAL: usize = 2;
/// Hard ceiling on spawned projectiles, DECOUPLED from the shard cap. The cap
/// is now 25, and 25 x 2 ticks is a 50-tick stream that never stops; 16 keeps
/// the pause between volleys that makes a burst read as a burst.
const SHARD_STREAM_MAX: usize = 16;
/// `<prefix><target>` — which enemy the in-flight stream is aimed at.
const SHARD_STREAM_PREFIX: &str = "mlbb_obsidia_shard_stream_";
/// One stack per tick elapsed in the current stream: the pacing clock.
const SHARD_STREAM_TICK_BUFF: &str = "mlbb_obsidia_shard_stream_tick";

// ---------------------------------------------------------------------------
// Buff names
// ---------------------------------------------------------------------------

/// One stack = one Bone Shard. Permanent while held; the cap is enforced by
/// `entity_stack_buff`'s `max_stacks`.
const SHARD_BUFF: &str = "mlbb_obsidia_shard";
/// The ult's five temporary shards, held separately so they can push past the
/// cap of 8 and vanish on their own when the tether ends.
const TEMP_SHARD_BUFF: &str = "mlbb_obsidia_temp_shard";
/// Bone Energy: 1..3 stacks, converted to a shard at ENERGY_PER_SHARD.
const ENERGY_BUFF: &str = "mlbb_obsidia_bone_energy";
/// On Obsidia: the tether is live.
const TETHER_BUFF: &str = "mlbb_obsidia_hunt_tether";
/// On Obsidia: carries which champion she is tethered to, the same way Alpha's
/// ult lock does. A buff rather than struct state so it survives a reload and
/// cannot leak between casts once it expires.
const TETHER_LOCK_PREFIX: &str = "mlbb_obsidia_hunt_lock_";
const TETHER_LOCK_TICKS: usize = 200;
/// On Obsidia: the opening pull is still being enforced.
const PULL_BUFF: &str = "mlbb_obsidia_hunt_pull";
/// On the enemy: tethered, therefore displacement-blocked.
const TETHERED_BUFF: &str = "mlbb_obsidia_hunt_tethered";
/// On Obsidia: +30% attack speed and ignore-wall for the tether.
const HUNT_BUFF: &str = "mlbb_obsidia_hunt";
const SHADOWMELD_BUFF: &str = "mlbb_obsidia_shadowmeld";
/// S1 grants Return to Bone energy on the **first** enemy hero hit only, and a
/// single LineRangeProjectile applies this effect once per champion in the
/// line. This lock makes "first" mean something. It outlives the projectile by
/// a wide margin and cannot survive into the next cast (CD 270).
const NEEDLE_ENERGY_LOCK: &str = "mlbb_obsidia_needle_energy_lock";
const NEEDLE_ENERGY_LOCK_TICKS: usize = 30;

// ---------------------------------------------------------------------------
// Tether beam rendering (Estes's technique, measured on our own art)
// ---------------------------------------------------------------------------

const LINK_BINS: usize = 12;
/// World spacing of the tiled segments. Matches the geometry the beam was
/// built to: 48 px of art at 12 px spacing = 4x overlap, 24 segments reach
/// 288,000 wu. Our 40,000 leash needs 4 segments.
const LINK_SEGMENT_SPACING: f64 = 12_000.0;
const LINK_MAX_SEGMENTS: usize = 24;
const LINK_BIN_NAMES: [&str; LINK_BINS] = [
    "obsidia_bone_link_0",
    "obsidia_bone_link_1",
    "obsidia_bone_link_2",
    "obsidia_bone_link_3",
    "obsidia_bone_link_4",
    "obsidia_bone_link_5",
    "obsidia_bone_link_6",
    "obsidia_bone_link_7",
    "obsidia_bone_link_8",
    "obsidia_bone_link_9",
    "obsidia_bone_link_10",
    "obsidia_bone_link_11",
];

/// Pick the pre-rotated beam cell that lies along `from -> to`.
///
/// MEASURED, not assumed. The 12 cells were run through an alpha-weighted
/// principal-axis fit; bin `k`'s ink lies along image angle **-15k** degrees
/// (image y points down, which is world +y). Every bin measured 25 px along
/// and 4 px across on that axis, and 25/4 inverts if you try the other sign.
///
/// NOTE the sign. Estes's equivalent uses `dy.atan2(dx)`; his art has the
/// opposite handedness to ours, so copying his line would put the beam at
/// right angles to the tether. World y grows downward relative to screen
/// angles, hence the negation.
fn link_bin(from: (u64, u64), to: (u64, u64)) -> &'static str {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let theta = (-dy).atan2(dx).rem_euclid(std::f64::consts::PI);
    let idx = ((theta / std::f64::consts::PI) * LINK_BINS as f64).round() as usize;
    LINK_BIN_NAMES[idx % LINK_BINS]
}

/// Draws the tether as a row of overlapping bone segments. Render-only.
fn draw_tether(sim: &mut StableSim<'_>, caster: usize, from: (u64, u64), to: (u64, u64)) {
    if !can_render(sim) {
        return;
    }
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let length = dx.hypot(dy);
    if !length.is_finite() || length < 1.0 {
        return;
    }
    let name = link_bin(from, to);
    let count = (length / LINK_SEGMENT_SPACING)
        .ceil()
        .clamp(1.0, LINK_MAX_SEGMENTS as f64) as usize;
    for i in 0..count {
        let t = (i as f64 + 0.5) / count as f64;
        let x = (from.0 as f64 + dx * t).round().clamp(0.0, u64::MAX as f64) as u64;
        let y = (from.1 as f64 + dy * t).round().clamp(0.0, u64::MAX as f64) as u64;
        let _ = sim.play_view_effect(name, caster, &InputTargetV1::pos(x, y), 0, 0, 0);
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Rounds to nearest instead of truncating.
///
/// Truncation was quietly costing her ~0.76% per shard: at 118 AD, 5% is 5.9
/// and she was paid 5, so her headline "5% per shard" was really 4.24% at L15.
///
/// This is NOT a shard-only fix. All 17 call sites route through here -- the
/// 65% basic, the 60% needle and the 50% ult among them -- so rounding nudges
/// every one of them up by at most 1. That is a small across-the-board buff,
/// which is exactly why it ships alongside the accrual rebalance rather than
/// on its own: measured in isolation it would look like a damage buff with no
/// stated cause.
fn percent(value: usize, pct: usize) -> usize {
    let scaled = value.saturating_mul(pct);
    (scaled + 50) / 100
}

fn level_linear(low: usize, high: usize, level: usize) -> usize {
    low + (high - low).saturating_mul(level.saturating_sub(1).min(14)) / 14
}

fn is_hostile(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target)
        .is_some_and(|e| e.team() != team && e.is_alive() && e.is_targetable() && !e.is_tower())
}

/// The same rule for BASIC ATTACKS, except towers are legal targets.
///
/// Why this exists separately: `is_hostile` excludes towers, and that is right
/// for SKILLS -- alpha, arlott, franco and balmond all filter towers out of
/// their skill effects. But it is wrong for a basic attack. Hanabi and Karrie,
/// the two marksmen in this roster that also drive their basics from a native,
/// both gate on `team / alive / targetable` and exclude ONLY the Nexus:
///
///     && !matches!(target.kind(), Some(EntityKindV1::Nexus))
///
/// Obsidia was the odd one out, and because the gate sits in front of
/// `spawn_projectile` AND in front of `deal_damage_typed`, it produced two
/// symptoms from one cause: no projectile art and no damage when hitting a
/// tower. Reported by the owner in-game, 2026-11-04.
///
/// Bone Energy: as of 2026-11-05 a TOWER DOES grant, at the reduced
/// ENERGY_PER_NON_HERO_HIT rate. Canon denies Energy only against jungle
/// creeps; towers and lane minions have always fed her. The old hero-only gate
/// was an adaptation error -- the spec read "no creeps in this roster" as "no
/// non-heroes", but creeps are not minions. `grant_energy` therefore gates on
/// THIS rule (towers legal, Nexus excluded), not on `is_hostile`, which was
/// silently swallowing tower hits before they could ever be scored.
fn is_hostile_basic(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| {
        e.team() != team
            && e.is_alive()
            && e.is_targetable()
            && !matches!(e.kind(), Some(EntityKindV1::Nexus))
    })
}

fn is_hero(sim: &StableSim<'_>, target: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| e.is_champion())
}

fn can_render(sim: &StableSim<'_>) -> bool {
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

fn speak(sim: &mut StableSim<'_>, caster: usize, line: &str) {
    if !can_render(sim) {
        return;
    }
    let _ = crate::audio::play_voice(sim, caster, line);
}

fn named_buff_count(sim: &StableSim<'_>, entity: usize, name: &str) -> usize {
    let Some(e) = sim.get_entity(entity) else {
        return 0;
    };
    (0..e.buff_count())
        .filter_map(|i| e.buff_at(i))
        .filter(|b| b.name() == name)
        .count()
}

fn has_buff(sim: &StableSim<'_>, entity: usize, name: &str) -> bool {
    named_buff_count(sim, entity, name) > 0
}

// ---------------------------------------------------------------------------
// Bone shards and Bone Energy
// ---------------------------------------------------------------------------

/// Stored Bone Shards, including the ult's temporary ones.
fn shard_count(sim: &StableSim<'_>, owner: usize) -> usize {
    named_buff_count(sim, owner, SHARD_BUFF) + named_buff_count(sim, owner, TEMP_SHARD_BUFF)
}

/// Every stored shard strikes the target for SHARD_DAMAGE_PERCENT of total AD.
fn shard_damage(attack: usize, shards: usize) -> usize {
    percent(attack, SHARD_DAMAGE_PERCENT).saturating_mul(shards)
}

fn add_shard(sim: &mut StableSim<'_>, owner: usize) {
    // PERMANENT, not `timed(.., 0)`. `BuffV1::timed` sets the duration kind to
    // `Time`, and a Time buff with 0 ticks is already expired — the shard would
    // be gone before it could be spent. Shards persist for the life.
    let stack = BuffV1::named(SHARD_BUFF);
    let _ = sim.entity_stack_buff(owner, &stack, SHARD_CAP, true);
}

/// Adds Bone Energy, forging a shard whenever ENERGY_PER_SHARD is reached.
///
/// Energy carries over, so two skill hits (2 + 2) plus two basic attacks
/// (1 + 1) forge two shards between them.
fn add_energy(sim: &mut StableSim<'_>, owner: usize, amount: usize) {
    // Permanent for the same reason as the shards: energy must not tick away
    // between fights.
    let stack = BuffV1::named(ENERGY_BUFF);
    for _ in 0..amount {
        let _ = sim.entity_stack_buff(owner, &stack, ENERGY_PER_SHARD, true);
        if named_buff_count(sim, owner, ENERGY_BUFF) >= ENERGY_PER_SHARD {
            let _ = sim.entity_remove_buff(owner, ENERGY_BUFF);
            add_shard(sim, owner);
        }
    }
}

/// Grants energy for a landed hit.
///
/// Heroes pay the full rate. Minions and towers pay the reduced
/// ENERGY_PER_NON_HERO_HIT rate -- canon denies Energy only against jungle
/// creeps, never against lane minions or towers, and excluding them left her
/// with no passive during laning. Jungle camps, epic monsters, lords, the
/// nexus and summons are excluded: those are the "creeps" §10 meant to drop.
fn grant_energy(sim: &mut StableSim<'_>, caster: usize, target: usize, skill: bool) {
    let Some(team) = sim.get_entity(caster).map(|e| e.team()) else {
        return;
    };
    // is_hostile_basic, NOT is_hostile: the latter excludes towers, which
    // would make the Tower arm of `energy_for_kind` dead code.
    if !is_hostile_basic(sim, target, team) {
        return;
    }
    let Some(kind) = sim.get_entity(target).and_then(|e| e.kind()) else {
        return;
    };
    if let Some(amount) = energy_for_kind(kind, skill) {
        add_energy(sim, caster, amount);
    }
}

/// Bone Energy granted by hitting an entity of `kind`, or `None` when the kind
/// feeds nothing at all.
///
/// Pure, so the rule is unit-testable without a sim. Canon denies Energy only
/// against JUNGLE creeps -- lane minions and towers have always fed her. The
/// old hero-only gate was an adaptation error: the spec's "there are no creeps
/// in this roster" was about creeps, and `EntityKindV1::Minion` exists.
fn energy_for_kind(kind: EntityKindV1, skill: bool) -> Option<usize> {
    match kind {
        EntityKindV1::Champion => Some(if skill {
            ENERGY_PER_SKILL_HIT
        } else {
            ENERGY_PER_BASIC_HIT
        }),
        EntityKindV1::Minion | EntityKindV1::Tower => Some(ENERGY_PER_NON_HERO_HIT),
        // Jungle, Epic, Serpen, Nexus and Summon are canon's "creeps".
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Cosmetic shard stream
// ---------------------------------------------------------------------------

/// The `(target, total)` an in-flight stream was armed with, if any.
fn shard_stream_state(sim: &StableSim<'_>, caster: usize) -> Option<(usize, usize)> {
    let entity = sim.get_entity(caster)?;
    for i in 0..entity.buff_count() {
        let Some(buff) = entity.buff_at(i) else {
            continue;
        };
        let Some(rest) = buff.name().strip_prefix(SHARD_STREAM_PREFIX) else {
            continue;
        };
        let mut parts = rest.split('_');
        let target = parts.next()?.parse().ok()?;
        let total = parts.next()?.parse().ok()?;
        return Some((target, total));
    }
    None
}

/// Arms a stream. The first shard leaves on the next `on_update`, not here, so
/// the whole thing stays inside the per-tick drain path.
fn start_shard_stream(sim: &mut StableSim<'_>, caster: usize, target: usize, total: usize) {
    clear_shard_stream(sim, caster);
    if total == 0 {
        return;
    }
    let name = format!("{SHARD_STREAM_PREFIX}{target}_{total}");
    let _ = sim.add_buff(caster, &BuffV1::named(&name));
}

/// Drops all stream state. Called on spawn (death) so a stream never outlives
/// the champion who fired it -- a dead caster with projectiles still in flight
/// is exactly the case BoDUD measured at -40% CPU.
fn clear_shard_stream(sim: &mut StableSim<'_>, caster: usize) {
    let Some(entity) = sim.get_entity(caster) else {
        return;
    };
    let names: Vec<String> = (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .map(|b| b.name().to_string())
        .filter(|n| n.starts_with(SHARD_STREAM_PREFIX) || n == SHARD_STREAM_TICK_BUFF)
        .collect();
    for name in names {
        let _ = sim.entity_remove_buff(caster, &name);
    }
}

/// Spawns one cosmetic shard, offset perpendicular to the flight line so
/// consecutive shots do not stack exactly on top of each other.
fn spawn_stream_shard(sim: &mut StableSim<'_>, caster: usize, target: usize, index: usize) {
    let Some(team) = sim.get_entity(caster).map(|e| e.team()) else {
        return;
    };
    let Some((start, alive)) = sim.get_entity(caster).map(|e| (e.pos(), e.is_alive())) else {
        return;
    };
    if !alive || !is_hostile_basic(sim, target, team) {
        return;
    }
    let Some(end) = sim.get_entity(target).map(|e| e.pos()) else {
        return;
    };
    let dx = end.0 as f64 - start.0 as f64;
    let dy = end.1 as f64 - start.1 as f64;
    let len = dx.hypot(dy);
    if !len.is_finite() || len < 1.0 {
        return;
    }
    let (px, py) = (-dy / len, dx / len);
    let off = if index % 2 == 0 {
        -SHARD_FAN_SPACING
    } else {
        SHARD_FAN_SPACING
    };
    let sx = (start.0 as f64 + px * off)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64;
    let sy = (start.1 as f64 + py * off)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64;
    let spec = ProjectileSpawnV1 {
        caster_id: caster,
        team,
        x: sx,
        y: sy,
        radius: BASIC_PROJECTILE_RADIUS,
        speed: BASIC_PROJECTILE_SPEED,
        move_kind: ProjectileMoveKindV1::Target.code(),
        target_id: target,
        attack_type: AttackTypeV1::BaseAttack.code(),
        casting_type: CastingTypeV1::Targeting.code(),
        casting_target: CastingTargetV1::Enemy.code(),
        target_x: end.0,
        target_y: end.1,
        penetrate: false,
    };
    let _ = sim.spawn_projectile(BASIC_PROJECTILE, SHARD_HIT_EFFECT, &spec);
}

/// Advances the stream by one tick. Called from the passive's `on_update`.
///
/// Spawns at most one projectile, and does no work at all when no stream is
/// armed, so the idle cost is a single buff scan per tick.
fn drain_shard_stream(sim: &mut StableSim<'_>, caster: usize) {
    let Some((target, total)) = shard_stream_state(sim, caster) else {
        return;
    };
    let span = total.saturating_mul(SHARD_STREAM_INTERVAL);
    let ticks = named_buff_count(sim, caster, SHARD_STREAM_TICK_BUFF);

    if ticks < span && ticks % SHARD_STREAM_INTERVAL == 0 {
        spawn_stream_shard(sim, caster, target, ticks / SHARD_STREAM_INTERVAL);
    }

    if ticks + 1 >= span {
        clear_shard_stream(sim, caster);
    } else {
        let tick = BuffV1::named(SHARD_STREAM_TICK_BUFF);
        let _ = sim.entity_stack_buff(caster, &tick, usize::MAX, true);
    }
}

// ---------------------------------------------------------------------------
// Tether state
// ---------------------------------------------------------------------------

fn tether_lock_name(target: usize) -> String {
    format!("{TETHER_LOCK_PREFIX}{target}")
}

fn set_tether_lock(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    clear_tether_lock(sim, caster);
    let name = tether_lock_name(target);
    let _ = sim.add_buff(caster, &BuffV1::timed(&name, TETHER_LOCK_TICKS));
}

fn clear_tether_lock(sim: &mut StableSim<'_>, caster: usize) {
    let Some(entity) = sim.get_entity(caster) else {
        return;
    };
    let names: Vec<String> = (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .map(|b| b.name().to_string())
        .filter(|n| n.starts_with(TETHER_LOCK_PREFIX))
        .collect();
    for name in names {
        let _ = sim.entity_remove_buff(caster, &name);
    }
}

/// The champion Obsidia is currently tethered to.
fn tether_target(sim: &StableSim<'_>, caster: usize) -> Option<usize> {
    let entity = sim.get_entity(caster)?;
    for i in 0..entity.buff_count() {
        let Some(buff) = entity.buff_at(i) else {
            continue;
        };
        if let Some(rest) = buff.name().strip_prefix(TETHER_LOCK_PREFIX) {
            return rest.parse().ok();
        }
    }
    None
}

/// ForceMove, ported from obsidia_work/spike_forcemove. Returns the number of
/// ticks the CC lasts, or 0 when no movement was needed.
///
/// The spike proved the ABI call is reachable and the arithmetic is sound. It
/// could not prove the engine moves the unit — `enforce_tether` covers that
/// case with `entity_set_pos`.
fn force_move(sim: &mut StableSim<'_>, id: usize, from: (f64, f64), to: (f64, f64)) -> usize {
    let d = (to.0 - from.0).hypot(to.1 - from.1);
    if d < 1.0 {
        return 0;
    }
    let ticks = f64::ceil(d / ULT_PULL_SPEED).max(1.0);
    let mut cc = CcV1::of_kind(CcKindV1::ForceMove, ticks as u64);
    cc.dx = (to.0 - from.0).round() as i64;
    cc.dy = (to.1 - from.1).round() as i64;
    cc.speed = f64::ceil(d / ticks) as u64;
    sim.apply_cc(id, &cc);
    ticks as usize
}

/// Keeps Obsidia within `limit` of the tethered champion.
///
/// This is one rule doing two jobs, which is what canon asks for: if she walks
/// away she is snapped back, and if the enemy walks she is dragged along. Both
/// are "she may not be farther than the leash".
fn hold_within(sim: &mut StableSim<'_>, caster: usize, target: usize, limit: f64) {
    let Some(from) = sim.get_entity(caster).map(|e| e.pos()) else {
        return;
    };
    let Some(to) = sim.get_entity(target).map(|e| e.pos()) else {
        return;
    };
    let dx = from.0 as f64 - to.0 as f64;
    let dy = from.1 as f64 - to.1 as f64;
    let d = dx.hypot(dy);
    if !d.is_finite() || d <= limit || d < 1.0 {
        return;
    }
    let ux = dx / d;
    let uy = dy / d;
    let x = (to.0 as f64 + ux * limit)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64;
    let y = (to.1 as f64 + uy * limit)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64;
    let _ = sim.entity_set_pos(caster, x, y);
}

/// Per-tick tether upkeep: hold the leash and redraw the beam.
fn enforce_tether(sim: &mut StableSim<'_>, caster: usize) {
    if !has_buff(sim, caster, TETHER_BUFF) {
        return;
    }
    let Some(target) = tether_target(sim, caster) else {
        return;
    };
    let alive = sim
        .get_entity(target)
        .is_some_and(|e| e.is_alive() && e.is_targetable());
    if !alive {
        return;
    }
    // During the opening pull, hold her to the closer standoff; afterwards the
    // full leash. This is what actually guarantees she reaches the target if
    // the engine ignores ForceMove.
    let limit = if has_buff(sim, caster, PULL_BUFF) {
        ULT_PULL_STANDOFF
    } else {
        ULT_TETHER_LEASH
    };
    hold_within(sim, caster, target, limit);
    let Some(from) = sim.get_entity(caster).map(|e| e.pos()) else {
        return;
    };
    let Some(to) = sim.get_entity(target).map(|e| e.pos()) else {
        return;
    };
    draw_tether(sim, caster, from, to);
}

// ---------------------------------------------------------------------------
// Passive — Return to Bone
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct ObsidiaReturnToBone;

impl StablePassive for ObsidiaReturnToBone {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize) {
        // Bone Shards are EARNED and persist across death — owner's call,
        // 2026-11-04. They are a resource she builds over the match, not a
        // per-life buff. Without this the ~21 s ramp restarts every life and
        // she never reaches the +40% the kit is priced around.
        //
        // Still cleared: the ult's temporary shards (they belong to the tether
        // and must not outlive it) and partial Bone Energy (progress toward the
        // NEXT shard — keeping it would hand her a free shard on respawn).
        let _ = sim.entity_remove_buff(entity, TEMP_SHARD_BUFF);
        let _ = sim.entity_remove_buff(entity, ENERGY_BUFF);
        clear_tether_lock(sim, entity);
        clear_shard_stream(sim, entity);
    }

    /// The tether's per-tick work lives here. `on_update` is the roster's
    /// standard per-tick hook (Balmond, Zhask, Karrie and Uranus all drive
    /// movement or pulsing effects from it).
    fn on_update(
        &mut self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        entity: usize,
    ) {
        enforce_tether(sim, entity);
        drain_shard_stream(sim, entity);
    }

    /// Fires for every hit she lands, skills included, so it is the single
    /// charge path — the same reason Alpha charges Beta's mark here.
    fn on_attack(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity: usize,
        target: usize,
        _damage: &mut usize,
    ) {
        grant_energy(sim, entity, target, false);
    }

    /// Canon: killing an enemy hero instantly grants a Bone Shard.
    fn on_kill(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize, victim: usize) {
        if is_hero(sim, victim) {
            add_shard(sim, entity);
        }
    }
}

// ---------------------------------------------------------------------------
// Basic attack
// ---------------------------------------------------------------------------

/// The data routes her basic attack through native code because a flat
/// `attack_ratio` cannot express "plus every stored shard" (Karrie and Zhask
/// route their basics the same way, for the same reason).
#[derive(Clone, Copy, Default)]
struct ObsidiaBasicAttack;

impl StableEffectType for ObsidiaBasicAttack {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let Some((start, team, alive)) = sim
            .get_entity(caster)
            .map(|e| (e.pos(), e.team(), e.is_alive()))
        else {
            return;
        };
        if !alive || !is_hostile_basic(sim, input.target_id, team) {
            return;
        }
        let Some(end) = sim.get_entity(input.target_id).map(|e| e.pos()) else {
            return;
        };
        let spec = ProjectileSpawnV1 {
            caster_id: caster,
            team,
            x: start.0,
            y: start.1,
            radius: BASIC_PROJECTILE_RADIUS,
            speed: BASIC_PROJECTILE_SPEED,
            move_kind: ProjectileMoveKindV1::Target.code(),
            target_id: input.target_id,
            attack_type: AttackTypeV1::BaseAttack.code(),
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: CastingTargetV1::Enemy.code(),
            target_x: end.0,
            target_y: end.1,
            penetrate: false,
        };
        let _ = sim.spawn_projectile(BASIC_PROJECTILE, BASIC_HIT_EFFECT, &spec);

        // One extra projectile per stored shard, released ONE PER TICK instead
        // of all at once, so a full stack reads as sustained fire. Cosmetic
        // only: all shard damage still rides on the main hit above.
        //
        // Capped by SHARD_STREAM_MAX, not by the shard cap: at a cap of 25 the
        // stream would run 50 ticks and never stop. The count still reads as
        // "many"; what is capped is how long she keeps firing.
        let shards = shard_count(sim, caster).min(SHARD_STREAM_MAX);
        if shards > 0 {
            start_shard_stream(sim, caster, input.target_id, shards);
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            BASIC_DAMAGE_BASE + percent(stat.attack, BASIC_ATTACK_RATIO),
            0,
        )
    }
}

/// Cosmetic-only landing effect for the extra shard projectiles. It must NEVER
/// deal damage — the main hit already paid every shard. Same reason Alpha's
/// `alpha_beta_strike_once` is an empty effect.
#[derive(Clone, Copy, Default)]
struct ObsidiaShardVisual;

impl StableEffectType for ObsidiaShardVisual {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if !can_render(sim) || input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let _ = sim.play_view_effect(
            "obsidia_shard_flash",
            caster,
            &InputTargetV1::target(input.target_id),
            0,
            0,
            0,
        );
    }
}

#[derive(Clone, Copy, Default)]
struct ObsidiaBasicHit;

impl StableEffectType for ObsidiaBasicHit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let Some((team, attack)) = sim.get_entity(caster).map(|e| (e.team(), e.stat().attack))
        else {
            return;
        };
        let target = input.target_id;
        if !is_hostile_basic(sim, target, team) {
            return;
        }
        let amount = BASIC_DAMAGE_BASE
            + percent(attack, BASIC_ATTACK_RATIO)
            + shard_damage(attack, shard_count(sim, caster));
        let _ = sim.deal_damage_typed(
            caster,
            target,
            amount,
            DamageTypeV1::Ad,
            AttackTypeV1::BaseAttack,
        );
        if can_render(sim) {
            let _ = sim.play_view_effect(
                "obsidia_shard_flash",
                caster,
                &InputTargetV1::target(target),
                0,
                0,
                0,
            );
        }
        // Charged here rather than in the passive, because the passive's
        // `on_attack` cannot tell this hit from a skill hit.
        grant_energy(sim, caster, target, false);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            BASIC_DAMAGE_BASE + percent(stat.attack, BASIC_ATTACK_RATIO),
            0,
        )
    }
}

// ---------------------------------------------------------------------------
// S1 — Abyssal Bone Needle
// ---------------------------------------------------------------------------

/// Runs once per champion the needle passes through. Damage for everyone;
/// Return to Bone energy for the first hero only, which is what canon says.
#[derive(Clone, Copy, Default)]
struct ObsidiaNeedleHit;

impl StableEffectType for ObsidiaNeedleHit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let Some((team, attack, level)) = sim
            .get_entity(caster)
            .map(|e| (e.team(), e.stat().attack, e.level().max(1)))
        else {
            return;
        };
        let target = input.target_id;
        if !is_hostile(sim, target, team) {
            return;
        }
        let amount = level_linear(NEEDLE_DAMAGE_LOW, NEEDLE_DAMAGE_HIGH, level)
            + percent(attack, NEEDLE_ATTACK_RATIO);
        let _ = sim.deal_damage_typed(
            caster,
            target,
            amount,
            DamageTypeV1::Ad,
            AttackTypeV1::Skill,
        );
        speak(sim, caster, VOICE_SKILL1);
        if !has_buff(sim, caster, NEEDLE_ENERGY_LOCK) {
            let _ = sim.add_buff(
                caster,
                &BuffV1::timed(NEEDLE_ENERGY_LOCK, NEEDLE_ENERGY_LOCK_TICKS),
            );
            grant_energy(sim, caster, target, true);
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            NEEDLE_DAMAGE_LOW + percent(stat.attack, NEEDLE_ATTACK_RATIO),
            0,
        )
    }
}

// ---------------------------------------------------------------------------
// S2 — Phantom Shadowmeld
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct ObsidiaShadowmeld;

impl StableEffectType for ObsidiaShadowmeld {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _: InputTargetV1) {
        if !sim.get_entity(caster).is_some_and(|e| e.is_alive()) {
            return;
        }
        // Enhanced during the tether: 0.3 s -> 1 s, and the cooldown drops to
        // 33% of normal (270 -> 90), so 180 ticks come off the running timer.
        let enhanced = has_buff(sim, caster, TETHER_BUFF);
        let ticks = if enhanced { S2_ULT_TICKS } else { S2_TICKS };
        let _ = sim.entity_remove_buff(caster, SHADOWMELD_BUFF);
        let mut buff = BuffV1::timed(SHADOWMELD_BUFF, ticks);
        buff.move_speed_mult = S2_MOVE_SPEED_MULT;
        let _ = sim.add_buff(caster, &buff);
        if enhanced {
            let _ = sim.entity_reduce_cooldown(caster, SkillSlotV1::Skill2, ULT_S2_CD_CUT);
        }
        speak(sim, caster, VOICE_SKILL2);
    }

    fn expected_damage(&self, _stat: &StatV1) -> (usize, usize) {
        (0, 0)
    }
}

// ---------------------------------------------------------------------------
// Ult — Hunt of Bone
// ---------------------------------------------------------------------------

/// Cast entry. The payload waits for the projectile, because "the first enemy
/// hero hit" is not known until the bones are in flight.
#[derive(Clone, Copy, Default)]
struct ObsidiaHunt;

impl StableEffectType for ObsidiaHunt {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, _input: InputTargetV1) {
        // A fresh ult always re-locks. Everything else waits for the bones to
        // land, which is where "the first enemy hero hit" becomes knowable.
        clear_tether_lock(sim, caster);
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (ULT_DAMAGE_BASE + percent(stat.attack, ULT_ATTACK_RATIO), 0)
    }
}

/// The ult's payload, applied per champion the bones pass through. Only the
/// first one is hunted.
#[derive(Clone, Copy, Default)]
struct ObsidiaHuntHit;

impl StableEffectType for ObsidiaHuntHit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, caster: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let target = input.target_id;
        // Already hunting someone: the tether is single-target, and the lock is
        // only cleared by a fresh cast.
        if tether_target(sim, caster).is_some() {
            return;
        }
        let Some((team, attack, from)) = sim
            .get_entity(caster)
            .map(|e| (e.team(), e.stat().attack, e.pos()))
        else {
            return;
        };
        if !is_hostile(sim, target, team) || !is_hero(sim, target) {
            return;
        }
        let Some(to) = sim.get_entity(target).map(|e| e.pos()) else {
            return;
        };

        speak(sim, caster, VOICE_ULT);

        let amount = ULT_DAMAGE_BASE + percent(attack, ULT_ATTACK_RATIO);
        let _ = sim.deal_damage_typed(
            caster,
            target,
            amount,
            DamageTypeV1::Ad,
            AttackTypeV1::Skill,
        );

        // Stun, then haul herself along the chain. Canon is explicit that she
        // is the one who travels, not the enemy.
        sim.apply_cc(target, &CcV1::stun(ULT_STUN_TICKS as u64));

        // Shield is priced on the shards she held at cast time, before the
        // temporary ones land — the conservative reading of the official text.
        let shards = shard_count(sim, caster);
        let _ = sim.entity_add_shield(
            caster,
            ULT_SHIELD_BASE + ULT_SHIELD_PER_SHARD * shards,
            ULT_TETHER_TICKS,
        );

        // Five temporary shards, past the cap of 8, gone when the tether ends.
        let temp = BuffV1::timed(TEMP_SHARD_BUFF, ULT_TETHER_TICKS);
        for _ in 0..ULT_TEMP_SHARDS {
            let _ = sim.entity_stack_buff(caster, &temp, ULT_TEMP_SHARDS, true);
        }

        // +30% attack speed and move-through-obstacles for the tether.
        let mut hunt = BuffV1::timed(HUNT_BUFF, ULT_TETHER_TICKS);
        hunt.attack_speed_mult = ULT_ATTACK_SPEED_MULT;
        hunt.ignore_wall = true;
        let _ = sim.add_buff(caster, &hunt);

        // The tether itself, and the leash's per-tick handle on its target.
        let _ = sim.add_buff(caster, &BuffV1::timed(TETHER_BUFF, ULT_TETHER_TICKS));
        let _ = sim.add_buff(caster, &BuffV1::timed(PULL_BUFF, ULT_PULL_TICKS));
        set_tether_lock(sim, caster, target);

        // Anti-displacement. The engine has no "no dash or blink" CC, so this
        // uses BlockMoveSkill, which is the closest thing and blocks *all*
        // movement skills — broader than canon. The proper cross-module flag
        // every hero would have to honour is still an open design item
        // (OBSIDIA_SPEC.md §10). Walking is left alone: canon says the enemy's
        // movement drags Obsidia along, so they must be able to move.
        let _ = sim.add_buff(target, &BuffV1::timed(TETHERED_BUFF, ULT_TETHER_TICKS));
        sim.apply_cc(
            target,
            &CcV1::of_kind(CcKindV1::BlockMoveSkill, ULT_TETHER_TICKS as u64),
        );

        // The pull. ForceMove for the smooth ride; `enforce_tether` snaps her
        // in with `entity_set_pos` if the engine does not honour it.
        let _ = force_move(
            sim,
            caster,
            (from.0 as f64, from.1 as f64),
            (to.0 as f64, to.1 as f64),
        );

        if can_render(sim) {
            let _ = sim.play_view_effect(
                "obsidia_hunt_ring",
                caster,
                &InputTargetV1::target(target),
                0,
                0,
                0,
            );
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (ULT_DAMAGE_BASE + percent(stat.attack, ULT_ATTACK_RATIO), 0)
    }
}

// ---------------------------------------------------------------------------
// Voice — S1 / S2 / ult only, per the standing rule. No hit or cast SFX.
// ---------------------------------------------------------------------------

const VOICE_SKILL1: &str = "mlbb_obsidia_skill1_cast";
const VOICE_SKILL2: &str = "mlbb_obsidia_skill2_cast";
const VOICE_ULT: &str = "mlbb_obsidia_ult_cast";

// ---------------------------------------------------------------------------
// Registration. Spelled out with literal names, not constants: the effect
// verifier greps the source for `add_native_effect("mlbb_heroes:...")`.
// ---------------------------------------------------------------------------

pub fn register(module: &mut StableMod) {
    module.add_native_passive(
        "mlbb_heroes:obsidia_return_to_bone",
        ObsidiaReturnToBone::default(),
    );
    module.add_native_effect("mlbb_heroes:obsidia_basic_attack", ObsidiaBasicAttack);
    module.add_native_effect("mlbb_heroes:obsidia_basic_hit", ObsidiaBasicHit);
    module.add_native_effect("mlbb_heroes:obsidia_shard_visual", ObsidiaShardVisual);
    module.add_native_effect("mlbb_heroes:obsidia_needle_hit", ObsidiaNeedleHit);
    module.add_native_effect("mlbb_heroes:obsidia_shadowmeld", ObsidiaShadowmeld);
    module.add_native_effect("mlbb_heroes:obsidia_hunt", ObsidiaHunt);
    module.add_native_effect("mlbb_heroes:obsidia_hunt_hit", ObsidiaHuntHit);
}

// ---------------------------------------------------------------------------
// Tests — pure arithmetic and the measured beam bins, both runnable offline.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_math_matches_the_decision() {
        // percent() ROUNDS as of 2026-11-05. Truncating paid 4.24% per shard
        // at L15, not the advertised 5%: 118 x 5% is 5.9, and she got 5.
        assert_eq!(percent(62, 7), 4, "4.34 rounds down");
        assert_eq!(percent(118, 7), 8, "8.26 rounds down - was 6 at 5%");
        assert_eq!(shard_damage(62, 8), 32, "4 x 8, was 24 at 5%");
        assert_eq!(shard_damage(118, 8), 64, "8 x 8, was 48 at 5%");
        // A L15 basic carrying 8 shards: 40 + 77 + 64 = 181.
        let l15 = 118;
        let total = BASIC_DAMAGE_BASE + percent(l15, BASIC_ATTACK_RATIO) + shard_damage(l15, 8);
        assert_eq!(total, 181, "L15, eight shards");
        // The cap is canon's 25, not 8. Eight is now a mid-game stack.
        assert_eq!(SHARD_CAP, 25);
        assert_eq!(shard_damage(l15, SHARD_CAP), 200, "25 x 8, was 150 at 5%");
    }

    #[test]
    fn rotation_sits_between_arlott_and_alpha() {
        // L1, 62 AD, EIGHT shards: S1 + 5 basics + ult. Eight is a mid-game
        // stack under the new cap of 25, not the ceiling -- the band below is
        // about her mid-game placement, and a full 25-stack rotation is
        // deliberately allowed to exceed Alpha (see the ceiling test below).
        let ad = 62;
        let basic = BASIC_DAMAGE_BASE + percent(ad, BASIC_ATTACK_RATIO) + shard_damage(ad, 8);
        let s1 = NEEDLE_DAMAGE_LOW + percent(ad, NEEDLE_ATTACK_RATIO);
        let ult = ULT_DAMAGE_BASE + percent(ad, ULT_ATTACK_RATIO);
        let rotation = s1 + 5 * basic + ult;
        // Arlott 437 marked, Alpha 810. Unstacked she is below Alpha; stacked
        // she approaches him. Both ends must stay inside that band.
        let unstacked = s1 + 5 * (BASIC_DAMAGE_BASE + percent(ad, BASIC_ATTACK_RATIO)) + ult;
        assert!(rotation > 700 && rotation < 810, "stacked = {rotation}");
        assert!(unstacked > 437 && unstacked < rotation, "raw = {unstacked}");
    }

    #[test]
    fn energy_forges_a_shard_every_fifteen_basics() {
        // Rebalanced 2026-11-05, then corrected the same day after an in-game
        // report that she stacked "way too fast". The first pass set the cost
        // to 12 while keeping canon's per-hit values, which made the economy
        // 2.5x too cheap: she reached the cap of 25 in under four minutes in
        // every scenario, including pure farming, because minions pay 1 and
        // laning is mostly minions. Canon's cost is 30.
        assert_eq!(ENERGY_PER_SHARD, 30, "canon");
        // 30 / 2 = 15 hero basics per shard.
        assert_eq!(ENERGY_PER_SHARD, ENERGY_PER_BASIC_HIT * 15);
        // A skill hit is worth two hero basics. Note 30 / 4 is 7.5, so skill
        // hits do not divide the shard cost evenly -- energy carries over,
        // which is fine and is how canon behaves too.
        assert_eq!(ENERGY_PER_SKILL_HIT, ENERGY_PER_BASIC_HIT * 2);
        // Minions feed her at half the hero rate: canon's 2:1.
        assert_eq!(ENERGY_PER_BASIC_HIT, ENERGY_PER_NON_HERO_HIT * 2);
    }

    #[test]
    fn energy_follows_canon_not_a_hero_only_gate() {
        use EntityKindV1::*;
        // Heroes pay the full rate, skills double that.
        assert_eq!(energy_for_kind(Champion, false), Some(ENERGY_PER_BASIC_HIT));
        assert_eq!(energy_for_kind(Champion, true), Some(ENERGY_PER_SKILL_HIT));
        // Minions and towers DO feed her. This is the fix for the biggest
        // adaptation error in the kit: the old gate returned nothing here, so
        // she had no passive at all while farming.
        assert_eq!(
            energy_for_kind(Minion, false),
            Some(ENERGY_PER_NON_HERO_HIT)
        );
        assert_eq!(energy_for_kind(Tower, false), Some(ENERGY_PER_NON_HERO_HIT));
        // Jungle creeps are what canon actually excludes. They stay excluded.
        assert_eq!(energy_for_kind(Jungle, false), None);
        assert_eq!(energy_for_kind(Epic, false), None);
        assert_eq!(energy_for_kind(Serpen, false), None);
        assert_eq!(energy_for_kind(Nexus, false), None);
        assert_eq!(energy_for_kind(Summon, false), None);
    }

    #[test]
    fn leash_is_inside_her_own_attack_range() {
        // 4 canon units at 10,000/unit.
        assert_eq!(ULT_TETHER_LEASH, 40_000.0);
        assert!(ULT_PULL_STANDOFF < ULT_TETHER_LEASH);
        // Read the range from the DATA instead of hardcoding it. This guard
        // previously asserted `< 46_000.0`, a literal copy of the value it
        // was supposed to be checking -- so when v2.0.8 moved her to 60,000
        // the guard still passed and told us nothing. A guard that repeats
        // the value cannot catch the value changing.
        let data = std::fs::read_to_string(find_file("champion/mlbb_obsidia.data_champion"))
            .expect("read champion data");
        let at = data
            .find("\"action_name\": \"attack\"")
            .or_else(|| data.find("\"action_name\":\"attack\""))
            .expect("no attack block");
        let range = num_after(&data[at..], "\"range\":");
        assert!(
            (ULT_TETHER_LEASH as usize) < range,
            "leash {ULT_TETHER_LEASH} is not inside her {range} attack range"
        );
    }

    #[test]
    fn hunt_throw_art_plays_out_before_the_projectile_is_gone() {
        // The S1 defect, on the ult. `obsidia_hunt_throw` is 6 frames x 0.05 s
        // = 0.300 s of art, but the ult's LineRangeProjectile shipped with
        // `delay: 6` -- 0.1 s of life, so the Bones of Wrath vanished a third
        // of the way through. The ult is hitscan (`apply: 1`), so the damage
        // always landed; this is purely the art being cut short.
        let data = std::fs::read_to_string(find_file("champion/mlbb_obsidia.data_champion"))
            .expect("read champion data");
        let at = data
            .find("\"obsidia_hunt_throw\"")
            .expect("ult projectile missing from the data");
        let delay = num_after(&data[at..], "\"delay\":");

        let fanim = std::fs::read_to_string(find_file("effects/obsidia_hunt_throw#anim.fanim"))
            .expect("read hunt_throw fanim");
        let frames = fanim.matches("\"duration\"").count();
        let per_frame = (fnum_after(&fanim, "\"duration\":") * 60.0).ceil() as usize;
        let art_ticks = per_frame * frames;

        assert!(
            delay >= art_ticks,
            "hunt throw lives {delay} ticks but the art needs {art_ticks} \
             ({frames} frames x {per_frame} ticks) to play once"
        );
    }

    #[test]
    fn pull_covers_the_ult_reach_inside_the_pull_window() {
        // 80,000 wu at 6,000/tick is 14 ticks; the window is 20.
        let ticks = (80_000.0 / ULT_PULL_SPEED).ceil();
        assert!(ticks <= ULT_PULL_TICKS as f64, "needs {ticks} ticks");
    }

    #[test]
    fn beam_bins_follow_the_measured_handedness() {
        // Measured: bin k's ink lies along image angle -15k deg, i.e. screen
        // angle +15k deg. Check the four cardinal bins and one diagonal.
        let origin = (100_000u64, 100_000u64);
        let at = |dx: f64, dy: f64| ((origin.0 as f64 + dx) as u64, (origin.1 as f64 + dy) as u64);
        // Screen angle 0 (world +x): bin 0.
        assert_eq!(link_bin(origin, at(10_000.0, 0.0)), "obsidia_bone_link_0");
        // Screen angle 90 (world -y, because world y grows downward): bin 6.
        assert_eq!(link_bin(origin, at(0.0, -10_000.0)), "obsidia_bone_link_6");
        // Screen angle 45 (world +x, -y): bin 3.
        assert_eq!(
            link_bin(origin, at(10_000.0, -10_000.0)),
            "obsidia_bone_link_3"
        );
        // Screen angle 135 (world -x, -y): bin 9.
        assert_eq!(
            link_bin(origin, at(-10_000.0, -10_000.0)),
            "obsidia_bone_link_9"
        );
        // The beam is symmetric: reversing the pair must not change the bin.
        for k in 0..LINK_BINS {
            let a = (0u64, 0u64);
            let b = (
                (10_000.0 * (k as f64 * 15.0).to_radians().cos()) as u64,
                (10_000.0 * (k as f64 * 15.0).to_radians().sin()) as u64,
            );
            assert_eq!(link_bin(a, b), link_bin(b, a));
        }
    }

    #[test]
    fn beam_tiling_stays_within_the_art_reach() {
        // 24 segments x 12,000 spacing = 288,000; the leash is 40,000.
        let segs = (ULT_TETHER_LEASH / LINK_SEGMENT_SPACING).ceil() as usize;
        assert!(segs <= LINK_MAX_SEGMENTS, "needs {segs} segments");
        assert_eq!(segs, 4);
    }

    #[test]
    fn the_shard_stream_is_the_stack_counter() {
        // §19: the count is read off how many projectiles the basic attack
        // throws. One main attack (which carries all the damage) plus one
        // cosmetic projectile per stored shard, released one at a time.
        let streamed = |shards: usize| shards.min(SHARD_STREAM_MAX);
        assert_eq!(streamed(0), 0, "no shards, no stream");
        assert_eq!(streamed(5), 5);
        // The cap is 25 but the STREAM stops at 16: a 25 x 2-tick stream runs
        // 50 ticks and never stops, so the volley stops reading as a volley.
        assert_eq!(SHARD_STREAM_MAX, 16);
        assert_eq!(streamed(SHARD_CAP), 16);
        assert_eq!(streamed(SHARD_CAP + ULT_TEMP_SHARDS), 16);
        assert_eq!(streamed(99), 16, "the stream is capped");
        // A full stream spans (16 - 1) x 2 = 30 ticks and stops, leaving a
        // 24-tick pause before the 54-tick cooldown expires. That pause is
        // what makes a burst read as a burst.
        let span = (streamed(SHARD_CAP) - 1) * SHARD_STREAM_INTERVAL;
        assert_eq!(span, 30);
        assert!(span < 54, "stream must finish before the next attack");
    }

    #[test]
    fn the_stream_fits_inside_the_attack_cooldown() {
        // The stream staggers DEPARTURE, not flight: every shard travels at
        // the base speed. So the last shard lands at (stream span) + (flight
        // time), and that total must beat the 54-tick attack cooldown or
        // consecutive volleys overlap into one continuous line.
        let span = (SHARD_STREAM_MAX - 1) * SHARD_STREAM_INTERVAL;
        let flight = (60_000.0 / BASIC_PROJECTILE_SPEED as f64).ceil();
        let last_lands = span as f64 + flight;
        assert!(last_lands < 54.0, "last shard lands at {last_lands} ticks");
    }

    #[test]
    fn level_one_and_fifteen_needle_damage() {
        assert_eq!(level_linear(NEEDLE_DAMAGE_LOW, NEEDLE_DAMAGE_HIGH, 1), 90);
        assert_eq!(level_linear(NEEDLE_DAMAGE_LOW, NEEDLE_DAMAGE_HIGH, 15), 140);
    }

    // -----------------------------------------------------------------------
    // Guards added 2026-11-04 after the owner's first Obsidia playtest.
    // -----------------------------------------------------------------------

    /// Walk up from the crate root looking for a FILE, not a directory.
    /// `sdk/candidate/champion/` exists but is a stale partial copy (only
    /// arlott and franco), so matching on the directory stops in the wrong
    /// place — the same trap the context records for arlott's `delay: 6`.
    fn find_file(rel: &str) -> std::path::PathBuf {
        use std::path::{Path, PathBuf};
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            let p: PathBuf = cur.join(rel);
            if p.is_file() {
                return p;
            }
            if !cur.pop() {
                break;
            }
        }
        panic!(
            "could not locate `{rel}` near {}",
            env!("CARGO_MANIFEST_DIR")
        );
    }

    fn num_after(hay: &str, key: &str) -> usize {
        let k = hay.find(key).unwrap_or_else(|| panic!("{key} not found"));
        let rest = &hay[k + key.len()..];
        let s = rest.find(|c: char| c.is_ascii_digit()).expect("no digits");
        let e = rest[s..]
            .find(|c: char| !c.is_ascii_digit())
            .map(|i| s + i)
            .unwrap_or(rest.len());
        rest[s..e].parse().expect("parse int")
    }

    fn fnum_after(hay: &str, key: &str) -> f64 {
        let k = hay.find(key).unwrap_or_else(|| panic!("{key} not found"));
        let rest = &hay[k + key.len()..];
        let s = rest
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .expect("no number");
        let e = rest[s..]
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .map(|i| s + i)
            .unwrap_or(rest.len());
        rest[s..e].parse().expect("parse float")
    }

    #[test]
    fn the_needle_stays_on_screen_long_enough_to_read() {
        // The Franco/Arlott/Alpha defect, a fourth time. A LineRangeProjectile
        // lives `delay` ticks and is removed at `delay - 1`, so `delay` IS the
        // whole on-screen budget. Shipped at 4 it lived 3 ticks (0.05 s)
        // against 0.30 s of art — the owner saw one frame of six.
        let data = std::fs::read_to_string(find_file("champion/mlbb_obsidia.data_champion"))
            .expect("read champion data");
        let at = data
            .find("\"obsidia_needle\"")
            .expect("needle not in the data");
        let delay = num_after(&data[at..], "\"delay\":");

        let fanim = std::fs::read_to_string(find_file("effects/obsidia_bone_needle#anim.fanim"))
            .expect("read needle fanim");
        let frames = fanim.matches("\"duration\"").count();
        let per_frame = (fnum_after(&fanim, "\"duration\":") * 60.0).ceil() as usize;
        let art_ticks = per_frame * frames;

        assert!(
            delay >= art_ticks,
            "needle lives {delay} ticks but the art needs {art_ticks} to play once \
             ({frames} frames x {per_frame} ticks)"
        );
        // And it must not be a token gesture: Alpha and Franco both sit at 18.
        assert!(
            delay >= 18,
            "delay {delay} is below the 18-tick roster floor"
        );
    }

    #[test]
    fn shards_survive_death() {
        // Owner's call, 2026-11-04. `on_spawn` is the only place that could
        // wipe them, so guard the source of that one function.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("obsidia.rs");
        let src = std::fs::read_to_string(path).expect("read obsidia.rs");
        let at = src.find("fn on_spawn").expect("no on_spawn");
        let body = &src[at..(at + 1400).min(src.len())];
        // Match the whole call, not the bare constant: "SHARD_BUFF" is a
        // substring of "TEMP_SHARD_BUFF", so a bare `contains` fails on the
        // very line that is supposed to be there.
        assert!(
            !body.contains("entity_remove_buff(entity, SHARD_BUFF)"),
            "on_spawn must not clear SHARD_BUFF — earned Bone Shards persist through death"
        );
        assert!(
            body.contains("TEMP_SHARD_BUFF"),
            "the ult's temporary shards must still be cleared on spawn"
        );
        assert!(
            body.contains("ENERGY_BUFF"),
            "partial Bone Energy must still be cleared on spawn"
        );
    }

    #[test]
    fn the_volley_stays_smaller_than_the_champion() {
        // Her body is ~46 px. Shards no longer fan abreast -- they leave one
        // at a time, jittered to either side of the flight line -- so the
        // lateral span is one jitter step plus the art, not CAP * spacing.
        // At the old 5,000 spacing and a 25 cap the fan would have spanned
        // 125 px, and 25 sprites at 38 px wide is 950 px: wider than the map.
        let fanim = std::fs::read_to_string(find_file("effects/obsidia_bone_shard#anim.fanim"))
            .expect("read shard fanim");
        let art_w = num_after(&fanim, "\"w\":");
        let span = 2.0 * SHARD_FAN_SPACING / 1_000.0;
        let total = span + art_w as f64;
        assert!(
            total <= 52.0,
            "volley spans {total} px (span {span} + art {art_w}); her body is ~46"
        );
    }

    #[test]
    fn s2_burst_stays_inside_the_roster_band() {
        // `move_speed_mult` across the whole roster runs -80..+20. Canon's
        // +500% compounds with time and, honoured, threw her ~117 px in 0.3 s —
        // further than Uranus's tank engage. The dash is the data's `MoveTo`.
        assert!(
            S2_MOVE_SPEED_MULT <= 20,
            "S2 burst {S2_MOVE_SPEED_MULT}% is outside the roster's -80..+20 band"
        );
        assert!(
            S2_MOVE_SPEED_MULT > 0,
            "S2 must still be a burst, not a slow"
        );
    }

    #[test]
    fn basic_attack_hits_towers() {
        // Owner's in-game report, 2026-11-04: no damage on tower AND no
        // projectile art. One cause -- `is_hostile` excluded towers and sat in
        // front of both `spawn_projectile` and `deal_damage_typed`.
        //
        // Hanabi and Karrie are the precedent: both drive their basic attack
        // from a native and both exclude ONLY the Nexus.
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("obsidia.rs"),
        )
        .expect("read obsidia.rs");

        // 1. the basic-attack predicate must exist and must not filter towers
        assert!(
            src.contains("fn is_hostile_basic("),
            "is_hostile_basic is missing - towers are being filtered out of basics"
        );
        let at = src.find("fn is_hostile_basic(").expect("checked above");
        let body = &src[at..(at + 700).min(src.len())];
        assert!(
            !body.contains("is_tower"),
            "is_hostile_basic excludes towers; the basic attack cannot hit them"
        );
        assert!(
            body.contains("EntityKindV1::Nexus"),
            "is_hostile_basic must still exclude the Nexus, as Hanabi/Karrie do"
        );

        // 2. both basic-attack call sites must use it
        let atk = src.find("struct ObsidiaBasicAttack").expect("basic attack");
        let atk_body = &src[atk..(atk + 3000).min(src.len())];
        assert!(
            atk_body.contains("is_hostile_basic(sim, input.target_id, team)"),
            "ObsidiaBasicAttack still gates on is_hostile - no projectile vs towers"
        );
        let hit = src.find("struct ObsidiaBasicHit").expect("basic hit");
        let hit_body = &src[hit..(hit + 2000).min(src.len())];
        assert!(
            hit_body.contains("is_hostile_basic(sim, target, team)"),
            "ObsidiaBasicHit still gates on is_hostile - no damage vs towers"
        );

        // 3. Towers and minions now DO grant, at the reduced
        //    ENERGY_PER_NON_HERO_HIT rate. This REVERSES the rule this guard
        //    was written to protect: canon denies Bone Energy only against
        //    jungle creeps, and an `is_hero` gate was blocking minions and
        //    towers alike. The policy itself is pinned by
        //    energy_follows_canon_not_a_hero_only_gate.
        //
        //    What this guard still protects is the PLUMBING, which is the part
        //    that silently broke: `is_hostile` excludes towers, so gating on it
        //    makes the Tower arm of `energy_for_kind` unreachable dead code --
        //    the rule looks implemented and never fires.
        let ge = src.find("fn grant_energy(").expect("grant_energy");
        let ge_body = &src[ge..(ge + 700).min(src.len())];
        assert!(
            ge_body.contains("is_hostile_basic(sim, target, team)"),
            "grant_energy must gate on is_hostile_basic; is_hostile excludes towers"
        );
        assert!(
            !ge_body.contains("is_hostile(sim, target, team)"),
            "grant_energy gates on is_hostile, so tower hits are dropped before scoring"
        );

        // 4. skills keep the stricter tower-excluding rule
        assert!(
            src.contains("fn is_hostile("),
            "is_hostile is missing; skills still need the tower-excluding rule"
        );
        let ih = src.find("fn is_hostile(").expect("checked above");
        let ih_body = &src[ih..(ih + 300).min(src.len())];
        assert!(
            ih_body.contains("!e.is_tower()"),
            "is_hostile no longer excludes towers - skills would start hitting them"
        );
    }

    #[test]
    fn basic_attack_tower_rule_matches_the_marksman_peers() {
        // The rule is not my invention: check the two marksmen that also drive
        // their basics from a native gate on Nexus-only, not on towers.
        for peer in ["hanabi.rs", "karrie.rs"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(peer);
            let Ok(src) = std::fs::read_to_string(path) else {
                continue;
            };
            for marker in ["struct HanabiBasicAttack", "struct KarrieBasicAttack"] {
                let Some(at) = src.find(marker) else {
                    continue;
                };
                let body = &src[at..(at + 2500).min(src.len())];
                assert!(
                    !body.contains("is_tower"),
                    "{peer} basic attack excludes towers, so the precedent moved - \
                     re-check is_hostile_basic"
                );
            }
        }
    }
}
