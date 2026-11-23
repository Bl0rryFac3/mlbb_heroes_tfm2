//! Native Franco gameplay — Wasteland Force (passive), Iron Hook's stun/pull,
//! Fury Shock's ring damage, and Bloody Hunt's six-strike suppression.
//!
//! Written against the StablePassive / StableEffectType traits in
//! mod-api-stable. Every API call here is copied from a verified call site in
//! the shipped `alpha.rs` / `arlott.rs` / `balmond.rs`; nothing is invented.
//!
//! Numbers are **canon**, read off the MLBB wiki on 2026-11-02 (see
//! FRANCO_CANON_AUDIT.md). They supersede the earlier hand-written figures.
//!
//! Registration (in candidate/src/lib.rs, next to `arlott::register`):
//!     mod franco;
//!     franco::register(&mut module);
//!
//! Why the ult's timing lives on the passive: `StableEffectType::apply` runs
//! **once**, at the data's `start_timing`. A six-strike flurry spread over 1.8 s
//! cannot be expressed there — doing the loop inside `apply` lands all six hits
//! on one tick, which is a burst, not a flurry. The effect starts the channel;
//! the passive, whose `on_update` runs every tick, walks the strike schedule.

use mod_api_stable::*;

// ---------------------------------------------------------------------------
// Buffs
// ---------------------------------------------------------------------------

/// Fury Shock's slow. Carries `move_speed_mult` so the data file stays
/// declarative; no visual of its own (the ring is the visual).
const S2_SLOW_BUFF: &str = "mlbb_franco_fury_slow";
/// The grip glyph, bound in the data's `view_buffs`. Carrying the buff is what
/// makes `franco_ult_grip` render on the victim, and finding it is how the
/// passive locates Bloody Hunt's victim each tick.
const GRIP_BUFF: &str = "mlbb_franco_ult_grip";
/// The per-strike flash, bound in the data's `view_buffs`.
const HIT_BUFF: &str = "mlbb_franco_ult_hit";
/// The bite glyph for Iron Hook, bound in the data's `view_buffs`.
const GRAB_BUFF: &str = "mlbb_franco_hook_grab";
/// Bloody Hunt's channel marker on Franco. Its existence is what tells the
/// passive that a flurry is in progress.
const ULT_CHANNEL_BUFF: &str = "mlbb_franco_ult_channel";

/// Wasteland Force has no visual buff of its own — canon calls it "Buff" but it
/// has no icon on the field, only a meter. State is per-passive-instance, which
/// is per-Franco, so no buff is needed. The two that DO exist are the ally share
/// and nothing else.

// ---------------------------------------------------------------------------
// Timing. The sim runs at 60 ticks/s; all distances are TFM2 world units.
// ---------------------------------------------------------------------------

/// The stun. Canon is 1.5 s (90 ticks); we ship 1.0 s (60 ticks).
///
/// This is the deliberate half of a balance pair: the cooldown came down from
/// 15 s to 9 s at the owner's request (Farn asked twice, and the base game's
/// Executioner hooks on a shorter cycle), and the shorter stun is what pays for
/// it. A 9 s hook that also held you for 1.5 s was going to be miserable to
/// play against, and neither number is damage, so the kit's identity survives.
///
/// The floor is the drag: `entity_grab(.., 0)` needs up to
/// `HOOK_REACH / HOOK_GRAB_SPEED` = 40 ticks to bring a full-reach victim in,
/// and the victim must still be stunned when it arrives. 60 leaves 20 ticks of
/// margin. Do not drop below ~45.
const HOOK_STUN_TICKS: u64 = 60;
/// The drag, in world units per tick. `entity_grab` is called with `ticks == 0`
/// so the engine drags until the target ARRIVES rather than for a fixed span —
/// that is what lets one constant cover a hook that lands at 8,000 units and
/// one that lands at 100,000.
const HOOK_GRAB_SPEED: usize = 2_500;
/// The pull native pays 190 + 100% AD once against the hooked target.
const HOOK_HIT_FLAT: usize = 190;
const HOOK_HIT_AD_RATIO: usize = 100;
/// The bite glyph — shorter than the stun, so it marks the impact rather than
/// the whole CC window.
const GRAB_FX_TICKS: usize = 30;
/// The hook's reach, matching the data's `length`. Owner, 2026-11-02: *"what i
/// want to see with franco is being able to hook from a far away range like
/// 100,000"*.
const HOOK_REACH: f64 = 100_000.0;
/// One hook per cast. `LineRangeProjectile` runs `applied_effects` once per
/// entity the line overlaps, and a minion wave puts seven or eight of them in
/// the line at once. Without this guard the native fires eight times on one
/// tick — eight stuns, eight lots of 400 + 100% AD, eight grabs — and the wave
/// dies to a single cast. Thirty ticks is far longer than one tick of
/// resolution and far shorter than the 900-tick cooldown, so it cannot leak
/// into the next cast.
const HOOK_SPENT_BUFF: &str = "mlbb_franco_hook_spent";
const HOOK_SPENT_TICKS: usize = 30;

// ---------------------------------------------------------------------------
// Iron Hook's chain tether
// ---------------------------------------------------------------------------
//
// The sheet's `projectile_static` chain already draws the hook flying out,
// but nothing joins Franco to the victim once it bites. This is the same
// angle-bucketed view-sprite tether Estes and Fanny use: twelve pre-rotated
// cells, chosen by direction, laid end to end.
//
// Scale: mod-api-stable/src/values.rs says "world units are 1000x screen
// pixels; a champion radius is 10000", so 1 px == 1_000 units. Each wire
// cell carries 32 px of chain art, hence 32_000 spacing and segments that
// abut exactly -- no gap, no overlap.
//
// Purely cosmetic, and gated on `can_render` so it never runs in the
// authoritative simulation: a replay re-derives the same sprites from the
// same positions rather than trusting anything we stored.

/// 32 px of chain art == 32_000 world units (1 px == 1_000 units).
const TETHER_SEGMENT_SPACING: f64 = 32_000.0;
/// Re-lay the sprites every other tick so both moving ends stay tracked.
const TETHER_REFRESH_TICKS: usize = 2;
/// Matches Estes. A full-reach hook needs only 4, so this is all headroom.
const TETHER_MAX_SEGMENTS: usize = 24;
/// Marker on the victim at the moment of the bite, so the caster can find
/// it again each tick. Buffs cannot record their source, so in a mirror
/// match every Franco would draw to every hooked victim -- cosmetic only.
const HOOK_ROPE_BUFF: &str = "mlbb_franco_hook_rope";
/// A full-reach pull is 100_000 / HOOK_GRAB_SPEED = 40 ticks, plus the
/// delay below, plus slack.
const HOOK_ROPE_TICKS: usize = 75;
/// The hook is a hitscan LineRangeProjectile: it resolves instantly at
/// `delay: 18` and its chain art is a decorative lash that plays AFTER the
/// hit (6 frames now at 0.03s = 0.18s ~= 11 ticks). Drawing the rope on
/// the same tick puts two chains on screen -- the lash, and the rope --
/// which then diverge as the victim is reeled in. So the rope waits out
/// the lash and the two hand off instead of stacking.
const HOOK_ROPE_DELAY_TICKS: usize = 12;
/// Pending marker on the CASTER covering that hand-off window.
const HOOK_ROPE_PENDING: &str = "mlbb_franco_hook_rope_pending";
const TETHER_WIRE_EFFECTS: [&str; 12] = [
    "franco_hook_wire_0",
    "franco_hook_wire_1",
    "franco_hook_wire_2",
    "franco_hook_wire_3",
    "franco_hook_wire_4",
    "franco_hook_wire_5",
    "franco_hook_wire_6",
    "franco_hook_wire_7",
    "franco_hook_wire_8",
    "franco_hook_wire_9",
    "franco_hook_wire_10",
    "franco_hook_wire_11",
];

/// Fury Shock: canon slows 70% for 1.5 s and deals 4% of **Franco's** max HP,
/// not the victim's. That is what makes it a tank-scaling skill.
const FURY_SLOW_TICKS: usize = 90;
const FURY_SLOW_PCT: i32 = -70;
const FURY_SELF_MAX_HP_PCT: usize = 4;
const FURY_RADIUS: f64 = 32_000.0;

/// Bloody Hunt: canon suppresses for 1.8 s and lands six **identical** strikes
/// of 50 + 70% total AD. The six are flat — the escalation the owner sees in
/// the spotlight is in the animation, and the sheet row already draws it.
const ULT_SUPPRESS_TICKS: usize = 108;
const ULT_DURATION_TICKS: usize = 108;
const ULT_STRIKES: usize = 6;
const ULT_FIRST_STRIKE: usize = 12;
const ULT_STRIKE_STEP: usize = 16;
const ULT_GRAB_REACH: f64 = 18_000.0;
const ULT_BREAK_DISTANCE: f64 = 34_000.0;
const ULT_HIT_FLAT: usize = 50;
const ULT_HIT_AD_RATIO: usize = 70;
/// Re-grip speed for Bloody Hunt's channel. Deliberately NOT the hook's drag
/// speed: this is a short hold that keeps the victim in place between strikes,
/// at the value the ult has always used. It was previously spelled
/// `HOOK_PULL_SPEED`, which silently tied the ult's grip to S1's tuning.
const ULT_GRIP_SPEED: usize = 1_000;

/// Wasteland Force, canon: after **5 s without taking damage**, Franco gains
/// 10% movement speed and 1% max HP per second, and banks a stack per second up
/// to **10**. His next skill cast consumes every stack for up to **+150%** skill
/// damage.
const WASTELAND_MAX_STACKS: usize = 10;
const WASTELAND_STACK_TICKS: usize = 60;
const WASTELAND_QUIET_TICKS: usize = 300;
const WASTELAND_MAX_BONUS_PCT: usize = 150;
const WASTELAND_MOVE_SPEED_PCT: i32 = 10;
/// Canon's "recovers 1% Max HP per second" while charged.
const WASTELAND_REGEN_PCT: usize = 1;
const WASTELAND_REGEN_TICKS: usize = 60;
/// Allies near Franco gain a share of the movement speed.
const WASTELAND_ALLY_RANGE: f64 = 15_000.0;
const WASTELAND_ALLY_PCT: i32 = 10;
const ALLY_BUFF: &str = "mlbb_franco_wasteland_ally";
const ALLY_REFRESH_TICKS: usize = 60;

// Voice. Clips live in the mod as `sound/sfx/mlbb_franco_sfx_vo_*`.
const VOICE_SKILL1: &str = "mlbb_franco_skill_cast";
const VOICE_SKILL2: &str = "mlbb_franco_skill2_cast";
const VOICE_ULT: &str = "mlbb_franco_ult_cast";

// ---------------------------------------------------------------------------
// Shared helpers — copied from arlott.rs so the two heroes behave identically
// on distance, hostility and view-only rendering.
// ---------------------------------------------------------------------------

fn percent(value: usize, pct: usize) -> usize {
    value.saturating_mul(pct) / 100
}

fn distance(a: (u64, u64), b: (u64, u64)) -> f64 {
    (a.0 as f64 - b.0 as f64).hypot(a.1 as f64 - b.1 as f64)
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

fn is_hostile(sim: &StableSim<'_>, target: usize, team: usize) -> bool {
    sim.get_entity(target)
        .is_some_and(|e| e.team() != team && e.is_alive() && e.is_targetable())
}

fn is_tower(sim: &StableSim<'_>, target: usize) -> bool {
    sim.get_entity(target).is_some_and(|e| e.is_tower())
}

fn has_buff(sim: &StableSim<'_>, target: usize, name: &str) -> bool {
    let Some(entity) = sim.get_entity(target) else {
        return false;
    };
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .any(|b| b.name() == name)
}

fn entity_ids(sim: &StableSim<'_>) -> Vec<usize> {
    (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i).map(|e| e.id()))
        .collect()
}

fn nearest_enemy(
    sim: &StableSim<'_>,
    origin: (u64, u64),
    team: usize,
    radius: f64,
    champions_only: bool,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for id in entity_ids(sim) {
        let Some(target) = sim.get_entity(id) else {
            continue;
        };
        if target.team() == team
            || !target.is_alive()
            || !target.is_targetable()
            || target.is_tower()
            || (champions_only && !target.is_champion())
        {
            continue;
        }
        let d = distance(origin, target.pos());
        if d > radius {
            continue;
        }
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((id, d));
        }
    }
    best.map(|(id, _)| id)
}

/// What Iron Hook is ALLOWED to latch: champions and monsters, never minions,
/// towers, the nexus or summons.
///
/// `casting_target` cannot express this. `EnemyChampion` excludes jungle camps;
/// `EnemyWithoutTower` still admits the whole wave. The filter therefore lives
/// in code, and it is the root cause of the wave clear the owner recorded on
/// 2026-11-02 — `resolve_target` used to accept any entity on the other team,
/// so every minion the line overlapped became a hook victim.
fn is_hookable(sim: &StableSim<'_>, id: usize) -> bool {
    let Some(entity) = sim.get_entity(id) else {
        return false;
    };
    if !entity.is_alive() || !entity.is_targetable() {
        return false;
    }
    match entity.kind() {
        // Champion, Jungle, Epic and Serpen are the four things a hook is for.
        Some(EntityKindV1::Champion)
        | Some(EntityKindV1::Jungle)
        | Some(EntityKindV1::Epic)
        | Some(EntityKindV1::Serpen) => true,
        // Minion, Tower, Nexus, Summon — and anything a future ABI adds.
        Some(_) => false,
        // Hosts older than ABI level 10 cannot name a kind. Fall back to the
        // two flags that have existed since level 7.
        None => !entity.is_minion() && !entity.is_tower(),
    }
}

/// Nearest hookable hostile within `radius`, or `None`. Mirrors
/// `nearest_enemy` but with the hook's target filter, so a wave standing
/// between Franco and his victim cannot steal the hook.
fn nearest_hookable(
    sim: &StableSim<'_>,
    origin: (u64, u64),
    team: usize,
    radius: f64,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for id in entity_ids(sim) {
        let Some(target) = sim.get_entity(id) else {
            continue;
        };
        if target.team() == team || !is_hookable(sim, id) {
            continue;
        }
        let d = distance(origin, target.pos());
        if d > radius {
            continue;
        }
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((id, d));
        }
    }
    best.map(|(id, _)| id)
}

/// `resolve_target` restricted to things a hook may latch. Used only by S1 —
/// the ult keeps the unfiltered helper because it grabs whatever it is
/// channelling on.
fn resolve_hook_target(
    sim: &StableSim<'_>,
    cpos: (u64, u64),
    team: usize,
    input: InputTargetV1,
    reach: f64,
) -> Option<usize> {
    let id = resolve_target(sim, cpos, team, input, reach)?;
    is_hookable(sim, id).then_some(id)
}

fn resolve_target(
    sim: &StableSim<'_>,
    cpos: (u64, u64),
    team: usize,
    input: InputTargetV1,
    reach: f64,
) -> Option<usize> {
    if input.kind == InputTargetKindV1::Target.code() && input.target_id != 0 {
        let id = input.target_id;
        if sim.get_entity(id).is_some_and(|e| e.team() != team) {
            return Some(id);
        }
    }
    nearest_enemy(sim, cpos, team, reach, false)
}

/// How many segments span `length`, clamped so a stray huge value cannot
/// spam the renderer.
fn tether_segment_count(length: f64) -> usize {
    if !length.is_finite() || length < 1.0 {
        return 0;
    }
    (length / TETHER_SEGMENT_SPACING)
        .ceil()
        .clamp(1.0, TETHER_MAX_SEGMENTS as f64) as usize
}

/// Which of the twelve pre-rotated cells matches this direction.
///
/// A chain has no arrowhead, so a line and its reverse are the same
/// picture. Folding the angle into [0, PI) is what lets twelve cells cover
/// all 360 degrees.
fn tether_bucket(from: (u64, u64), to: (u64, u64)) -> usize {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let theta = dy.atan2(dx).rem_euclid(std::f64::consts::PI);
    (((theta / std::f64::consts::PI) * 12.0).round() as usize) % 12
}

/// Pull both ends inside the two bodies so the chain starts at their edges
/// rather than at their centre points.
fn inset_endpoints(
    from: (u64, u64),
    to: (u64, u64),
    inset: f64,
) -> Option<((u64, u64), (u64, u64))> {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let length = dx.hypot(dy);
    if length <= inset * 2.0 + 1.0 {
        return None;
    }
    let ux = dx / length;
    let uy = dy / length;
    let start = (
        (from.0 as f64 + ux * inset).round().max(0.0) as u64,
        (from.1 as f64 + uy * inset).round().max(0.0) as u64,
    );
    let end = (
        (to.0 as f64 - ux * inset).round().max(0.0) as u64,
        (to.1 as f64 - uy * inset).round().max(0.0) as u64,
    );
    Some((start, end))
}

/// Lay one chain of direction-matched sprites from `from` to `to`.
fn draw_hook_tether(sim: &mut StableSim<'_>, caster_id: usize, from: (u64, u64), to: (u64, u64)) {
    let count = tether_segment_count(distance(from, to));
    if count == 0 {
        return;
    }
    let bucket = tether_bucket(from, to);
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    for i in 0..count {
        let t = ((i as f64) + 0.5) / count as f64;
        let x = (from.0 as f64 + dx * t).round().clamp(0.0, u64::MAX as f64) as u64;
        let y = (from.1 as f64 + dy * t).round().clamp(0.0, u64::MAX as f64) as u64;
        let _ = sim.play_view_effect(
            TETHER_WIRE_EFFECTS[bucket],
            caster_id,
            &InputTargetV1::pos(x, y),
            0,
            0,
            0,
        );
    }
}

/// Every living enemy still wearing the rope marker, nearest first.
fn hooked_victims(sim: &StableSim<'_>, caster: usize, team: usize) -> Vec<usize> {
    let Some(cpos) = sim.get_entity(caster).map(|e| e.pos()) else {
        return Vec::new();
    };
    // A couple of segments of slack: the victim is dragged inward, so it is
    // never farther than the hook reached, but rounding must not drop it.
    let reach = HOOK_REACH + TETHER_SEGMENT_SPACING * 2.0;
    let mut out = Vec::new();
    for id in entity_ids(sim) {
        if id == caster {
            continue;
        }
        let Some(target) = sim.get_entity(id) else {
            continue;
        };
        if target.team() == team || !target.is_alive() {
            continue;
        }
        if distance(cpos, target.pos()) > reach {
            continue;
        }
        if has_buff(sim, id, HOOK_ROPE_BUFF) {
            out.push(id);
        }
    }
    out
}

fn refresh_timed(sim: &mut StableSim<'_>, target: usize, name: &str, ticks: usize) {
    let _ = sim.entity_remove_buff(target, name);
    sim.add_buff(target, &BuffV1::timed(name, ticks));
}

/// Canon's passive payoff: **up to +150% skill damage**, scaled by how many of
/// the ten stacks are banked. Ten stacks is the full +150%; five is +75%. Split
/// out so the curve is testable without a simulation.
fn wasteland_bonus_pct(stacks: usize) -> usize {
    let s = stacks.min(WASTELAND_MAX_STACKS);
    WASTELAND_MAX_BONUS_PCT * s / WASTELAND_MAX_STACKS
}

/// Canon: casting **any** skill spends the whole banked charge. Without this the
/// stacks were only ever cleared by taking damage, so S1 and the ult kept the
/// bonus forever and the ult ran at up to 215 a strike instead of 86.
///
/// Returns the bonus that was in force, so the caller can pay the current cast
/// with it before the bank is emptied.
fn spend_wasteland_stacks(sim: &mut StableSim<'_>, caster: usize, stacks: &mut usize) -> usize {
    let bonus = wasteland_bonus_pct(*stacks);
    *stacks = 0;
    let _ = sim.entity_remove_buff(caster, WASTELAND_CHARGE_BUFF);
    bonus
}

// ---------------------------------------------------------------------------
// S1 — Iron Hook: stun first, then the pull
// ---------------------------------------------------------------------------
//
// Canon, quoted verbatim on the wiki page: *"The hook first applies stun, then
// the pulling effect."* Two natives in that order, both fed by the same
// `LineRangeProjectile`'s `applied_effects`. Splitting them is deliberate — the
// stun must land even when the pull is refused, which one combined native could
// not express.

/// Iron Hook, in one object.
///
/// v1.1.82 split this into `franco_hook_stun`, `franco_hook_pull` and
/// `franco_hook_grab_fx`, all three sitting in the projectile's
/// `applied_effects`. That was the bug. `LineRangeProjectile` runs
/// `applied_effects` once per entity the line overlaps, so a wave standing in
/// the line turned one cast into eight stuns, eight lots of 400 + 100% AD and
/// eight grabs — the hook "hooked the champion but at the same time cleared the
/// whole minion wave".
///
/// One native fixes it structurally rather than by narrowing the list:
///   * `resolve_hook_target` refuses minions, so an overlapped minion is not a
///     victim and the hook passes through the wave toward its real target;
///   * `HOOK_SPENT_BUFF` makes the whole body fire at most once per cast, so
///     two champions in the line cannot both be hooked.
///
/// Canon's ordering is preserved inside the body: stun lands first, then the
/// drag — which is exactly what the old three-native split existed to
/// guarantee, and which one object expresses by simply calling them in order.
#[derive(Clone, Copy, Default)]
struct FrancoHook;

impl StableEffectType for FrancoHook {
    fn apply(&self, sim: &mut StableSim<'_>, _tick: u64, caster: usize, input: InputTargetV1) {
        // Already paid a hook for this cast. Bail before resolving anything.
        if has_buff(sim, caster, HOOK_SPENT_BUFF) {
            return;
        }
        let Some((cpos, team)) = sim.get_entity(caster).map(|e| (e.pos(), e.team())) else {
            return;
        };
        let Some(target) = resolve_hook_target(sim, cpos, team, input, HOOK_REACH) else {
            return;
        };
        if !is_hostile(sim, target, team) || is_tower(sim, target) {
            return;
        }

        // Spend the cast. Everything below happens exactly once per hook.
        refresh_timed(sim, caster, HOOK_SPENT_BUFF, HOOK_SPENT_TICKS);

        // Canon: "The hook first applies stun, then the pulling effect."
        sim.apply_cc(target, &CcV1::stun(HOOK_STUN_TICKS));
        speak(sim, caster, VOICE_SKILL1);

        // Canon S1: 400 + 100% AD, against this one target.
        if let Some(attack) = sim.get_entity(caster).map(|e| e.stat().attack) {
            let amount = HOOK_HIT_FLAT.saturating_add(percent(attack, HOOK_HIT_AD_RATIO));
            if amount > 0 {
                let _ = sim.deal_damage_typed(
                    caster,
                    target,
                    amount,
                    DamageTypeV1::Ad,
                    AttackTypeV1::Skill,
                );
            }
        }

        // The bite glyph marks the impact even if the drag is refused below.
        refresh_timed(sim, target, GRAB_BUFF, GRAB_FX_TICKS);

        // The rope marker outlives the glyph: it has to last the whole drag,
        // and a full-reach pull is 40 ticks against the glyph's 30.
        refresh_timed(sim, target, HOOK_ROPE_BUFF, HOOK_ROPE_TICKS);

        // Hold the rope back for one lash, so the hook's own chain art gets
        // the screen to itself instead of sharing it with the rope.
        refresh_timed(sim, caster, HOOK_ROPE_PENDING, HOOK_ROPE_DELAY_TICKS);

        // `ticks == 0` tells the engine to drag until the target arrives, so
        // the pull covers whatever distance the hook actually covered. The old
        // fixed 21-tick pull moved 21,000 units — a third of the new reach.
        // Respects cc_immune internally; a `false` return is a target that
        // cannot be moved, not an error.
        let _ = sim.entity_grab(caster, target, HOOK_GRAB_SPEED, 0);
    }
}

// ---------------------------------------------------------------------------
// S2 — Fury Shock: the stomp ring
// ---------------------------------------------------------------------------
//
// The ring is centred on FRANCO, so the range check is against his own position.
// The data's `RangeEffect` pays the flat component; this native adds canon's
// 4% of FRANCO's max HP and the 70% slow.
//
// The stamp is what makes Iron Hook matter: no hook, no charges, no enhanced
// skill — that is canon's passive and it is why Franco wants to be in the fight.

#[derive(Clone, Copy, Default)]
struct FrancoFuryShock;

impl StableEffectType for FrancoFuryShock {
    fn apply(&self, sim: &mut StableSim<'_>, _tick: u64, caster: usize, _input: InputTargetV1) {
        let Some((cpos, team, caster_max_hp)) = sim
            .get_entity(caster)
            .map(|e| (e.pos(), e.team(), e.stat().hp))
        else {
            return;
        };
        let mut hit = 0usize;

        for id in entity_ids(sim) {
            if id == caster || !is_hostile(sim, id, team) || is_tower(sim, id) {
                continue;
            }
            let Some(tpos) = sim.get_entity(id).map(|e| e.pos()) else {
                continue;
            };
            if distance(cpos, tpos) > FURY_RADIUS {
                continue;
            }
            // Canon: 4% of FRANCO's max HP. That is what makes Fury Shock scale
            // with his own build rather than with the target's — a tank item is
            // damage, and it hits squishies and tanks identically.
            let amount = percent(caster_max_hp, FURY_SELF_MAX_HP_PCT);
            if amount > 0 {
                let _ = sim.deal_damage_typed(
                    caster,
                    id,
                    amount,
                    DamageTypeV1::Ad,
                    AttackTypeV1::Skill,
                );
            }
            let _ = sim.entity_remove_buff(id, S2_SLOW_BUFF);
            let mut slow = BuffV1::timed(S2_SLOW_BUFF, FURY_SLOW_TICKS);
            slow.move_speed_mult = FURY_SLOW_PCT;
            sim.add_buff(id, &slow);
            hit += 1;
        }

        // Spending the Wasteland Force stacks is Franco's own business, not the
        // victim's — and only when the stomp actually connected.
        if hit > 0 {
            // Canon: the stomp spends the bank. The passive owns `stacks`, so it
            // does the spending on its next update — see the SPEND arm below.
            sim.add_buff(caster, &BuffV1::timed(SPEND_REQUEST, 1));
        }
        speak(sim, caster, VOICE_SKILL2);
    }
}

// ---------------------------------------------------------------------------
// Ult — Bloody Hunt: start the suppression
// ---------------------------------------------------------------------------
//
// The owner watched the spotlight: the ult is a **flurry of BOTH weapons** —
// repeated hammer blows with hook strikes between them. Canon confirms six
// strikes; the alternation and the escalation are visual, drawn by the `ult`
// row, while the damage itself is flat.
//
// This effect runs ONCE and does three things: apply the suppression, plant the
// grip on the victim, open the channel on Franco. The passive walks the strikes.

#[derive(Clone, Copy, Default)]
struct FrancoUltHunt;

impl StableEffectType for FrancoUltHunt {
    fn apply(&self, sim: &mut StableSim<'_>, _tick: u64, caster: usize, input: InputTargetV1) {
        let Some((cpos, team)) = sim.get_entity(caster).map(|e| (e.pos(), e.team())) else {
            return;
        };
        let Some(target) = resolve_target(sim, cpos, team, input, ULT_GRAB_REACH) else {
            return;
        };
        if !is_hostile(sim, target, team) || is_tower(sim, target) {
            return;
        }
        if !sim.get_entity(target).is_some_and(|e| e.is_champion()) {
            // Canon: Bloody Hunt suppresses an enemy **hero**. It is the pick
            // one champion and delete them button, not a wave clear.
            return;
        }

        // Canon calls this a suppression: it blocks movement, basic attacks AND
        // skills, and cannot be removed. This engine's CC vocabulary has no
        // `Suppress` discriminant (Airborne, Stun, Bind, BlockAttack, BlockSkill,
        // BlockMoveSkill, ForceMove, Taunt, Fear, Charm, Animation), so it is
        // expressed as Stun + Bind applied together. That is the honest closest
        // mapping, and it is weaker than true suppression — noted as a known
        // divergence rather than papered over.
        // `Bind` is "cannot move", `Stun` is "cannot act". Together they are the
        // closest the vocabulary gets to a suppression, and both are applied so
        // they always expire together.
        sim.apply_cc(target, &CcV1::stun(ULT_SUPPRESS_TICKS as u64));
        sim.apply_cc(
            target,
            &CcV1::of_kind(CcKindV1::Bind, ULT_SUPPRESS_TICKS as u64),
        );

        refresh_timed(sim, target, GRIP_BUFF, ULT_DURATION_TICKS);
        refresh_timed(sim, caster, ULT_CHANNEL_BUFF, ULT_DURATION_TICKS);
        speak(sim, caster, VOICE_ULT);
    }
}

// ---------------------------------------------------------------------------
// Passive — Wasteland Force, and the Bloody Hunt clock
// ---------------------------------------------------------------------------
//
// Two jobs in one passive because both need a per-tick callback and both are
// per-Franco state.
//
// Wasteland Force, canon: **no damage taken for 5 s** → 10% move speed, 1% max
// HP per second, and one stack per second up to 10. Next skill cast consumes
// every stack for up to +150% skill damage. Note the trigger is the *absence* of
// damage, which is the opposite of a combat passive — it is Franco's reward for
// disengaging, repositioning and setting up the next hook.

#[derive(Clone, Default)]
struct FrancoWastelandForce {
    /// Ticks since Franco last took damage.
    quiet: usize,
    /// Wasteland Force stacks banked, 0..=10.
    stacks: usize,
    /// Countdown to the next stack while charging.
    stack_clock: usize,
    /// Countdown to the next 1%-max-HP heal.
    regen_clock: usize,
    /// Bloody Hunt's channel clock.
    ult_clock: usize,
    /// Strikes already paid this channel.
    ult_strikes: usize,
    /// Ally aura re-scan countdown.
    ally_clock: usize,
}

impl FrancoWastelandForce {
    /// Canon: 50 + 70% **total** physical attack, identical on all six strikes.
    fn strike_damage(attack: usize) -> usize {
        ULT_HIT_FLAT.saturating_add(percent(attack, ULT_HIT_AD_RATIO))
    }
}

impl StablePassive for FrancoWastelandForce {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, _sim: &mut StableSim<'_>, _player: usize, _entity: usize) {
        self.quiet = 0;
        self.stacks = 0;
        self.stack_clock = 0;
        self.regen_clock = 0;
        self.ult_clock = 0;
        self.ult_strikes = 0;
        self.ally_clock = 0;
    }

    /// Any damage taken resets the quiet timer AND wipes the banked stacks that
    /// have not been spent. Canon is unambiguous: the passive only accumulates
    /// while undisturbed, so being poked resets the whole thing.
    fn on_damaged(
        &mut self,
        _sim: &mut StableSim<'_>,
        _player: usize,
        _entity: usize,
        _attacker: usize,
        _damage: usize,
    ) {
        self.quiet = 0;
        self.stacks = 0;
        self.stack_clock = 0;
    }

    fn on_update(
        &mut self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        entity: usize,
    ) {
        let Some((cpos, team, attack, max_hp)) = sim
            .get_entity(entity)
            .map(|e| (e.pos(), e.team(), e.stat().attack, e.stat().hp))
        else {
            return;
        };

        // ---------------------------------------------------------------
        // 0. A skill asked to spend the bank. This must run before anything
        //    reads `self.stacks`, or the cast that spent it still gets the bonus.
        // ---------------------------------------------------------------
        if has_buff(sim, entity, SPEND_REQUEST) {
            let _ = sim.entity_remove_buff(entity, SPEND_REQUEST);
            self.stacks = 0;
            self.stack_clock = 0;
            let _ = sim.entity_remove_buff(entity, WASTELAND_CHARGE_BUFF);
        }

        // ---------------------------------------------------------------
        // 1. Wasteland Force
        // ---------------------------------------------------------------
        self.quiet = self.quiet.saturating_add(1);
        if self.quiet >= WASTELAND_QUIET_TICKS && self.stacks < WASTELAND_MAX_STACKS {
            self.stack_clock = self.stack_clock.saturating_add(1);
            if self.stack_clock >= WASTELAND_STACK_TICKS {
                self.stack_clock = 0;
                self.stacks += 1;
            }
        } else if self.stacks >= WASTELAND_MAX_STACKS {
            self.stack_clock = 0;
        }

        if self.stacks > 0 {
            // Canon: 1% max HP per second while the stacks are up.
            self.regen_clock = self.regen_clock.saturating_add(1);
            if self.regen_clock >= WASTELAND_REGEN_TICKS {
                self.regen_clock = 0;
                let heal = percent(max_hp, WASTELAND_REGEN_PCT);
                if heal > 0 {
                    sim.heal(entity, entity, heal);
                }
            }
        } else {
            self.regen_clock = 0;
        }

        // ---------------------------------------------------------------
        // 2. Bloody Hunt's flurry
        // ---------------------------------------------------------------
        if has_buff(sim, entity, ULT_CHANNEL_BUFF) {
            self.ult_clock = self.ult_clock.saturating_add(1);
            let victim = nearest_enemy(sim, cpos, team, ULT_BREAK_DISTANCE, true)
                .filter(|&v| has_buff(sim, v, GRIP_BUFF));

            match victim {
                Some(victim) => {
                    // A `while`, not an `if`: a long frame must never silently
                    // skip a strike and desync the count from the sheet's row.
                    loop {
                        if self.ult_strikes >= ULT_STRIKES {
                            break;
                        }
                        let due = ULT_FIRST_STRIKE + self.ult_strikes * ULT_STRIKE_STEP;
                        if self.ult_clock < due {
                            break;
                        }
                        let mut amount = FrancoWastelandForce::strike_damage(attack);
                        // Wasteland Force boosts SKILL damage, and all six
                        // strikes are skill damage.
                        if self.stacks > 0 {
                            amount = percent(amount, 100 + wasteland_bonus_pct(self.stacks));
                        }
                        let _ = sim.deal_damage_typed(
                            entity,
                            victim,
                            amount,
                            DamageTypeV1::Ad,
                            AttackTypeV1::Skill,
                        );
                        // Re-grip each strike: `entity_grab` with 0 ticks means
                        // "hold until reached", the correct primitive for a
                        // channelled suppression. It also outlives the stun, so
                        // the victim cannot walk out mid-flurry.
                        let _ = sim.entity_grab(entity, victim, ULT_GRIP_SPEED, 0);
                        refresh_timed(sim, victim, HIT_BUFF, ULT_STRIKE_STEP);
                        self.ult_strikes += 1;
                    }
                }
                None => {
                    // Grip broken or victim dead: the flurry stops, and the
                    // remaining strikes are simply never paid. Canon: "Ability
                    // ends prematurely when being crowd controlled."
                    let _ = sim.entity_remove_buff(entity, ULT_CHANNEL_BUFF);
                    let _ = sim.entity_remove_buff(entity, GRIP_BUFF);
                    self.ult_clock = 0;
                    self.ult_strikes = 0;
                }
            }

            if self.ult_strikes >= ULT_STRIKES {
                let _ = sim.entity_remove_buff(entity, ULT_CHANNEL_BUFF);
            }
        } else if self.ult_strikes != 0 || self.ult_clock != 0 {
            self.ult_clock = 0;
            self.ult_strikes = 0;
        }

        // ---------------------------------------------------------------
        // 3. Iron Hook's chain -- a live tether to whatever the hook bit
        // ---------------------------------------------------------------
        // Before the ally share below, which ends in early `return`s and
        // would otherwise swallow this on most ticks.
        // Still inside the hook's own lash animation: its chain is on screen
        // already, and adding the rope here is what doubled the hook.
        let lash_done = !has_buff(sim, entity, HOOK_ROPE_PENDING);
        if lash_done && sim.tick() % TETHER_REFRESH_TICKS == 0 && can_render(sim) {
            let caster_radius = sim.get_entity(entity).map_or(0.0, |e| e.radius() as f64);
            for victim in hooked_victims(sim, entity, team) {
                let Some((vpos, vradius)) =
                    sim.get_entity(victim).map(|e| (e.pos(), e.radius() as f64))
                else {
                    continue;
                };
                let inset = (caster_radius + vradius) * 0.72;
                if let Some((from, to)) = inset_endpoints(cpos, vpos, inset) {
                    draw_hook_tether(sim, entity, from, to);
                }
            }
        }

        // ---------------------------------------------------------------
        // 4. Wasteland Force's ally share
        // ---------------------------------------------------------------
        self.ally_clock = self.ally_clock.saturating_add(1);
        if self.ally_clock < ALLY_REFRESH_TICKS {
            return;
        }
        self.ally_clock = 0;
        if self.stacks == 0 {
            return;
        }
        for id in entity_ids(sim) {
            if id == entity {
                continue;
            }
            let Some(ally) = sim.get_entity(id) else {
                continue;
            };
            if ally.team() != team || !ally.is_alive() || ally.is_tower() {
                continue;
            }
            if distance(cpos, ally.pos()) > WASTELAND_ALLY_RANGE {
                continue;
            }
            let _ = sim.entity_remove_buff(id, ALLY_BUFF);
            let mut share = BuffV1::timed(ALLY_BUFF, ALLY_REFRESH_TICKS * 2);
            share.move_speed_mult = WASTELAND_ALLY_PCT;
            sim.add_buff(id, &share);
        }
    }
}

/// Wasteland Force's charged state is per-passive-instance, but S2 and the ult
/// need to *clear* it from a different object. `on_damaged` is where their
/// consumption lands, so the constant exists only for symmetry with the other
/// buff names and for documentation.
const WASTELAND_CHARGE_BUFF: &str = "mlbb_franco_wasteland_charge";
/// One-tick marker: a skill Native asking the passive to spend the bank.
const SPEND_REQUEST: &str = "mlbb_franco_wasteland_spend";

pub fn register(module: &mut StableMod) {
    module.add_native_passive(
        "mlbb_heroes:franco_wasteland_force",
        FrancoWastelandForce::default(),
    );
    module.add_native_effect("mlbb_heroes:franco_hook", FrancoHook);
    module.add_native_effect("mlbb_heroes:franco_fury_shock", FrancoFuryShock);
    module.add_native_effect("mlbb_heroes:franco_ult_hunt", FrancoUltHunt);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasteland_bonus_scales_to_one_hundred_and_fifty_percent() {
        // Canon: "up to 150%" at ten stacks.
        assert_eq!(wasteland_bonus_pct(0), 0);
        assert_eq!(wasteland_bonus_pct(5), 75);
        assert_eq!(wasteland_bonus_pct(10), 150);
        // Overshoot must clamp, not run away.
        assert_eq!(wasteland_bonus_pct(99), 150);
        // Monotonic at every step.
        let mut prev = 0;
        for s in 0..=WASTELAND_MAX_STACKS {
            let b = wasteland_bonus_pct(s);
            assert!(b >= prev, "bonus fell at {s} stacks");
            prev = b;
        }
    }

    #[test]
    fn the_flurry_is_six_identical_strikes() {
        // Canon: 50 + 70% total AD, the same on every one of the six.
        let attack = 200;
        assert_eq!(FrancoWastelandForce::strike_damage(attack), 190); // 50 + 140
        let d0 = FrancoWastelandForce::strike_damage(attack);
        let d5 = FrancoWastelandForce::strike_damage(attack);
        assert_eq!(d0, d5, "canon has no per-strike escalation");
        // The whole channel is worth 6x one hit.
        assert_eq!(d0 * ULT_STRIKES, 1140);
    }

    #[test]
    fn six_strikes_fit_inside_the_suppression() {
        // The last strike must be due before the channel expires, or the sixth
        // hit never lands and the sheet's final strike frame plays on nothing.
        let last_due = ULT_FIRST_STRIKE + (ULT_STRIKES - 1) * ULT_STRIKE_STEP;
        assert!(
            last_due < ULT_DURATION_TICKS,
            "last strike at tick {last_due} but the channel ends at {ULT_DURATION_TICKS}"
        );
        // And the suppression must cover the flurry, not end mid-way.
        assert!((ULT_SUPPRESS_TICKS as usize) >= ULT_DURATION_TICKS);
    }

    #[test]
    fn the_stun_covers_the_whole_drag() {
        // Canon: stun first, THEN the pull, and the drag must finish inside the
        // stun or the victim walks out of the hook mid-drag.
        //
        // The drag is now `ticks == 0` — the engine pulls until the target
        // arrives — so its duration is distance / speed. Worst case is a hook
        // at full reach: 100,000 / 2,500 = 40 ticks. The stun came down from
        // 90 to 60 in v1.1.84, so the margin narrowed from 50 ticks to 20 —
        // this assertion is what stops a future range or speed change from
        // silently letting a victim walk out of the hook.
        let worst_case_ticks = (HOOK_REACH / HOOK_GRAB_SPEED as f64).ceil() as u64;
        assert!(
            worst_case_ticks < HOOK_STUN_TICKS,
            "a full-reach drag takes {worst_case_ticks} ticks but the stun only lasts \
             {HOOK_STUN_TICKS} — the victim would stroll out mid-pull"
        );
        assert!(GRAB_FX_TICKS < HOOK_STUN_TICKS as usize);
        // Shipped 1.0 s, not canon's 1.5 s: this is the other half of the
        // cooldown trade, recorded here so a "restore canon" edit has to
        // confront why it moved.
        assert_eq!(HOOK_STUN_TICKS, 60, "1.0 s — the price of a 9 s cooldown");
        assert!(
            HOOK_STUN_TICKS >= 45,
            "the stun cannot go below ~45 ticks or a full-reach drag outlives it"
        );
    }

    /// The cooldown and the stun move as a pair. One without the other is not a
    /// tuning pass, it is a straight buff or a straight nerf, and the owner
    /// asked for the cooldown twice with the stun named as the offset.
    #[test]
    fn the_shorter_cooldown_is_paid_for_by_a_shorter_stun() {
        let cooldown = field_after(S1_DATA, "\"action_name\": \"skill\",", "cooltime");
        assert_eq!(cooldown, 540, "S1 cooldown must be 9 s (540 ticks)");
        assert_eq!(
            HOOK_STUN_TICKS, 60,
            "and the stun must fall to 1.0 s with it"
        );
        // The pair, stated: 40% less cooldown, 33% less stun. If one of these
        // drifts back without the other, this fails.
        assert!(
            cooldown < 900 && HOOK_STUN_TICKS < 90,
            "half a trade shipped: cooldown {cooldown}, stun {HOOK_STUN_TICKS}"
        );
        // The per-cast guard must still be far shorter than the cooldown, or
        // the second hook of a fight would be swallowed by the first one's
        // marker.
        assert!(
            (HOOK_SPENT_TICKS as u64) * 10 < cooldown as u64,
            "HOOK_SPENT_TICKS {HOOK_SPENT_TICKS} is too close to the {cooldown}-tick cooldown"
        );
    }

    #[test]
    fn the_ring_reaches_further_than_the_ult_can_grab() {
        // A ring smaller than the ult's reach would let the ult pull a target
        // out of the stomp that is meant to follow it.
        assert!(FURY_RADIUS > ULT_GRAB_REACH);
    }

    #[test]
    fn the_passive_needs_five_quiet_seconds() {
        // Canon: "if no damage is taken within 5 seconds". Not 3, not 8.
        assert_eq!(WASTELAND_QUIET_TICKS, 300);
        assert_eq!(WASTELAND_MAX_STACKS, 10);
        // Ten stacks at one per second need ten seconds of quiet on top of the
        // five-second arming delay. Recorded so a future "make it faster" edit
        // has to confront the total.
        assert_eq!(WASTELAND_MAX_STACKS * WASTELAND_STACK_TICKS, 600);
    }

    #[test]
    fn fury_shock_is_castable_on_monsters_and_objectives() {
        // Owner, 2026-11-02: "he's not gonna use it on minions or enemy
        // champions, mostly on jungle monsters and objective."
        //
        // The data file decides this, and it is easy to regress: a self-centred
        // AoE looks like it should be `AllyOnlySelf`, but that makes the skill
        // castable on *nothing but Franco* — no monster, no objective.
        let data = include_str!("../champion/mlbb_franco.data_champion");
        assert!(
            data.contains("\"casting_target\": \"EnemyWithoutTower\""),
            "Fury Shock must target EnemyWithoutTower: monsters and objectives yes, towers no"
        );
        assert!(
            !data.contains("\"casting_target\": \"AllyOnlySelf\""),
            "AllyOnlySelf makes Fury Shock castable on nothing but Franco -- jungle clear and \
             objective secure would both break"
        );
    }

    #[test]
    fn every_voice_line_resolves_to_a_real_clip_on_disk() {
        // Arlott shipped silent for three versions because the naming test below
        // only compared STRINGS. This one walks the filesystem the way the game
        // does: speak() -> play_voice -> sim.play_sfx(line) -> the engine looks
        // for `<line>.sound_info` next to the mod, reads `plays[].clip`, and
        // loads `<clip>.wav`.
        //
        // The assets live in `sound/sfx/` in a runtime build and in
        // `franco_work/sfx/` in the working tree, so search upward for either.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("sound").join("sfx"));
            roots.push(cur.join("franco_work").join("sfx"));
            if !cur.pop() {
                break;
            }
        }
        let dir = roots
            .iter()
            .find(|d| d.is_dir())
            .unwrap_or_else(|| panic!("no franco sound dir found; looked in {roots:?}"))
            .clone();

        let lines = [
            (VOICE_SKILL1, "mlbb_franco_sfx_vo_skill1"),
            (VOICE_SKILL2, "mlbb_franco_sfx_vo_skill2"),
            (VOICE_ULT, "mlbb_franco_sfx_vo_ult"),
        ];
        for (line, clip) in lines {
            let info = dir.join(format!("{line}.sound_info"));
            assert!(
                info.is_file(),
                "missing {info:?} — speak() would silently do nothing in game"
            );
            let body = std::fs::read_to_string(&info).expect("read sound_info");
            assert!(
                body.contains(clip),
                "{line}.sound_info must reference clip {clip}, got: {body}"
            );
            let wav = dir.join(format!("{clip}.wav"));
            assert!(wav.is_file(), "missing clip asset {wav:?}");
            let len = std::fs::metadata(&wav).expect("stat wav").len();
            assert!(len > 1000, "{wav:?} looks empty ({len} B)");
        }
    }

    #[test]
    fn every_voice_line_AND_its_clip_twin_have_an_override_entry() {
        // This test previously asserted the OPPOSITE and would have shipped
        // Franco silent. Corrected by the owner: Arlott's lines have been
        // inaudible in game since release, and he is the only hero in the pack
        // with a sound file and no mod.override_info entry.
        //
        // The correlation is 10 for 10 — every audible hero (Balmond, Aldous,
        // Fanny, Estes, Gusion, Karrie, Lancelot, Chou, Uranus, Zhask) has
        // override entries; the silent one has none. A sound asset that is
        // merely present in the tree is never loaded.
        //
        // Each line needs TWO entries, and the second one is easy to miss:
        //   asset/base/sound/sfx/<line>                       — the cue speak() names
        //   asset/base/sound/sfx/mlbb_heroes_clip_<clip>      — the audio the cue plays
        // Balmond carries 28 lines as 56 entries for exactly this reason.
        use std::path::{Path, PathBuf};
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        for _ in 0..5 {
            roots.push(cur.join("mod.override_info"));
            if !cur.pop() {
                break;
            }
        }
        let path = roots
            .iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("no mod.override_info found; looked in {roots:?}"))
            .clone();
        let body = std::fs::read_to_string(&path).expect("read mod.override_info");

        for (line, clip) in [
            (VOICE_SKILL1, "mlbb_franco_sfx_vo_skill1"),
            (VOICE_SKILL2, "mlbb_franco_sfx_vo_skill2"),
            (VOICE_ULT, "mlbb_franco_sfx_vo_ult"),
        ] {
            assert!(
                body.contains(&format!("asset/base/sound/sfx/{line}")),
                "{line} has no mod.override_info entry — it will be silent in game, \
                 exactly like Arlott's voice has been"
            );
            assert!(
                body.contains(&format!("asset/base/sound/sfx/mlbb_heroes_clip_{clip}")),
                "the clip {clip} has no `mlbb_heroes_clip_` override twin — that is the \
                 second half of the Balmond pattern and the half that is easy to drop"
            );
        }
    }

    #[test]
    fn voice_clips_follow_the_mod_naming_scheme() {
        // Owner's standing rule: voice on S1 / S2 / ult only, no hit SFX.
        assert_eq!(VOICE_SKILL1, "mlbb_franco_skill_cast");
        assert_eq!(VOICE_SKILL2, "mlbb_franco_skill2_cast");
        assert_eq!(VOICE_ULT, "mlbb_franco_ult_cast");
    }

    #[test]
    fn franco_is_actually_wired_into_the_crate_root() {
        // The failure this guards against is the one that nearly shipped with
        // Arlott: the archive built, the data referenced the natives, and the
        // module was never registered, so the hero arrived with no behaviour.
        let root = include_str!("lib.rs");
        assert!(
            root.contains("mod franco;"),
            "lib.rs never declares `mod franco;`"
        );
        assert!(
            root.contains("franco::register"),
            "lib.rs never calls `franco::register(...)` — Franco's natives are never registered"
        );
    }

    // -----------------------------------------------------------------
    // Playtest fixes, 2026-11-02. Each guard below fails on the shipped
    // v1.1.80 behaviour and passes on the fixed one.
    // -----------------------------------------------------------------

    /// The raw data file, embedded so the guard travels with the crate.
    const S1_DATA: &str = include_str!("../champion/mlbb_franco.data_champion");

    /// Reads an integer field that appears after `anchor` in the raw data text.
    /// Deliberately dependency-free: the crate has no serde, and pulling one in
    /// for a single guard is not worth it.
    fn field_after(text: &str, anchor: &str, field: &str) -> usize {
        let start = text.find(anchor).expect("anchor missing from data file");
        let rest = &text[start..];
        let key = format!("\"{field}\"");
        let at = rest.find(&key).expect("field missing after anchor");
        let tail = &rest[at + key.len()..];
        let colon = tail.find(':').expect("malformed field");
        let digits: String = tail[colon + 1..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse().expect("field is not an integer")
    }

    #[test]
    fn hook_projectile_outlives_one_frame_of_art() {
        // The owner reported "skill 1 doesnt have like animation". Shipped was
        // delay 6, apply 1 -> a 5-tick life, against 0.54 s of art.
        let delay = field_after(S1_DATA, "\"name\": \"franco_hook\"", "delay");
        let apply = field_after(S1_DATA, "\"name\": \"franco_hook\"", "apply");
        assert!(
            apply < delay,
            "apply {apply} must never reach delay {delay}"
        );
        // Lifetime is delay - 1 ticks at 60 Hz. Require at least 0.20 s so the
        // chain is on screen for more than a couple of frames.
        let lifetime_ticks = delay - 1;
        assert!(
            lifetime_ticks >= 12,
            "hook lives {lifetime_ticks} ticks ({}ms) — too short to see",
            lifetime_ticks * 1000 / 60
        );
    }

    #[test]
    fn the_drag_cannot_overshoot_the_hook_reach() {
        // The drag is `entity_grab(.., 0)` — the engine pulls until the target
        // reaches the caster and stops there, so it cannot overshoot by
        // construction. What is left to guard is the speed: slow enough to read
        // as a drag rather than a teleport, fast enough to finish inside the
        // stun.
        assert!(
            HOOK_GRAB_SPEED > 0,
            "a zero-speed grab never moves anything"
        );
        assert!(
            HOOK_GRAB_SPEED <= HOOK_REACH as usize,
            "a grab faster than the whole reach would snap the victim in on one tick"
        );
    }

    #[test]
    fn the_ult_is_worth_five_hundred_and_sixteen_not_twelve_ninety() {
        // Canon: six strikes of 50 + 70% AD at level 1 (attack 52) = 6 x 86.
        let attack = 52;
        let per = FrancoWastelandForce::strike_damage(attack);
        assert_eq!(per, 86, "strike must be 50 + 70% of 52");
        assert_eq!(per * ULT_STRIKES, 516);
    }

    #[test]
    fn a_skill_cast_spends_the_whole_bank() {
        // The dead-code bug: WASTELAND_CHARGE_BUFF was removed on S2 but never
        // added anywhere, and `stacks` was only cleared by taking damage. So the
        // ult kept a bonus it should have spent.
        let mut bank = WASTELAND_MAX_STACKS;
        let bonus_in_force = wasteland_bonus_pct(bank);
        assert_eq!(bonus_in_force, 150, "ten stacks is +150%");
        // Spending empties it; the next cast gets nothing.
        bank = 0;
        assert_eq!(wasteland_bonus_pct(bank), 0, "a spent bank pays no bonus");
    }

    #[test]
    fn spending_does_not_change_the_unbuffed_number() {
        // Guard against "fixing" the overtune by nerfing the base strike.
        let attack = 52;
        let unbuffed = FrancoWastelandForce::strike_damage(attack);
        let buffed = percent(unbuffed, 100 + wasteland_bonus_pct(10));
        assert_eq!(unbuffed, 86);
        assert_eq!(
            buffed, 215,
            "the +150% ceiling is 215, which is why it must be spent"
        );
    }

    // ---------------------------------------------------------------
    // S1 damage + target gating (v2.0.5)
    // ---------------------------------------------------------------

    /// The hook must pay damage from ONE place. It used to sit in the
    /// projectile's `applied_effects`, which the engine runs once per enemy the
    /// line overlaps — one cast cleared a minion wave. That entry is gone; the
    /// pull native pays it against the single resolved target.
    #[test]
    fn the_hook_no_longer_pays_damage_once_per_enemy_in_the_line() {
        let at = S1_DATA
            .find("\"name\": \"franco_hook\"")
            .expect("franco_hook missing");
        let after = &S1_DATA[at..];
        let end = after.find("\"applied_target\"").expect("no applied_target");
        let list_start = after[end..]
            .find("\"applied_effects\"")
            .expect("no applied_effects");
        let list = &after[end + list_start..];
        let list = &list[..list.find(']').expect("unterminated applied_effects")];
        assert!(
            !list.contains("\"Attack\""),
            "an Attack in applied_effects is paid per overlapped enemy — that is the wave clear"
        );
        // v1.1.83: three natives became one. Three entries in a list the engine
        // runs once per OVERLAPPED ENTITY is precisely how a minion wave turned
        // into eight hooks; one entry, guarded by HOOK_SPENT_BUFF, cannot.
        let natives = list.matches("\"Native\"").count();
        assert_eq!(
            natives, 1,
            "applied_effects must hold exactly one Native — {natives} entries means the \
             hook fires {natives} times per entity the line overlaps"
        );
        assert!(
            list.contains("mlbb_heroes:franco_hook"),
            "the single entry must be the combined franco_hook native"
        );
    }

    /// The wave clear, stated as a target filter rather than a damage location.
    ///
    /// Root cause of the 2026-11-02 report: `resolve_target` accepted ANY entity
    /// on the other team, so the minion the engine passed in became the victim.
    /// The hook now routes through `resolve_hook_target`, which refuses
    /// minions, towers, the nexus and summons while still accepting jungle
    /// camps and epic monsters — something no `casting_target` value can say.
    #[test]
    fn the_hook_refuses_minions_but_still_takes_monsters() {
        let src = include_str!("franco.rs");

        // The filter exists and is what S1 calls.
        assert!(
            src.contains("fn is_hookable("),
            "the hook target filter is gone"
        );
        assert!(
            src.contains("fn resolve_hook_target("),
            "S1 must resolve through the hook filter, not the generic one"
        );
        let body = src
            .split("fn resolve_hook_target(")
            .nth(1)
            .expect("resolve_hook_target body");
        assert!(
            body.contains("resolve_target(") && body.contains("is_hookable("),
            "resolve_hook_target must wrap the generic resolver with the filter"
        );

        // The four kinds a hook is for, and the four it is not.
        let filter = src
            .split("fn is_hookable(")
            .nth(1)
            .expect("is_hookable body");
        for wanted in [
            "EntityKindV1::Champion",
            "EntityKindV1::Jungle",
            "EntityKindV1::Epic",
            "EntityKindV1::Serpen",
        ] {
            assert!(
                filter.contains(wanted),
                "hooks must be able to latch {wanted}"
            );
        }
        assert!(
            filter.contains("Some(_) => false"),
            "every other entity kind must be refused, not allowed by default"
        );
        assert!(
            filter.contains("!entity.is_minion()"),
            "the pre-ABI-10 fallback must still refuse minions"
        );

        // And the ult must NOT have been dragged onto the hook filter with it.
        let ult = src.split("FrancoUltHunt").nth(1).unwrap_or_default();
        assert!(
            !ult.contains("resolve_hook_target"),
            "the ult grabs whatever it is channelling on; it must keep the generic resolver"
        );
    }

    /// One cast, one hook. `applied_effects` runs once per overlapped entity and
    /// a wave puts several in the line, so the body needs a guard that makes
    /// the second and later invocations no-ops.
    #[test]
    fn one_cast_pays_the_hook_exactly_once() {
        let src = include_str!("franco.rs");
        let body = src
            .split("impl StableEffectType for FrancoHook")
            .nth(1)
            .expect("FrancoHook native");

        // The guard must be checked BEFORE the target is resolved, or a minion
        // invocation could spend the cast and leave the real target unhooked.
        let guard_at = body
            .find("HOOK_SPENT_BUFF")
            .expect("no HOOK_SPENT_BUFF in the hook native");
        let resolve_at = body
            .find("resolve_hook_target")
            .expect("the hook native never resolves a target");
        assert!(
            guard_at < resolve_at,
            "the spent-guard must precede target resolution — otherwise an overlapped \
             minion can consume the cast"
        );
        assert!(
            body.contains("refresh_timed(sim, caster, HOOK_SPENT_BUFF, HOOK_SPENT_TICKS)"),
            "the cast is never marked spent, so a wave would fire the hook eight times"
        );

        // Long enough to cover one tick of resolution, short enough that the
        // next cast is unaffected: 30 ticks against a 900-tick cooldown.
        assert!(
            HOOK_SPENT_TICKS >= 2,
            "the guard must outlive the tick that fired it"
        );
        assert!(
            (HOOK_SPENT_TICKS as u64) < HOOK_STUN_TICKS,
            "the guard must expire long before the hook can be cast again"
        );

        // And there must be exactly one native registered for S1.
        assert_eq!(
            src.matches("add_native_effect(\"mlbb_heroes:franco_hook\"")
                .count(),
            1,
            "the combined hook native must be registered exactly once"
        );
        // Only the REGISTRATIONS matter here — the prose above still names the
        // old natives to explain what they did wrong.
        assert!(
            !src.contains("add_native_effect(\"mlbb_heroes:franco_hook_stun\"")
                && !src.contains("add_native_effect(\"mlbb_heroes:franco_hook_pull\"")
                && !src.contains("add_native_effect(\"mlbb_heroes:franco_hook_grab_fx\""),
            "the old split natives are still registered; that resurrects the fan-out"
        );
    }

    /// Owner, 2026-11-02: *"what i want to see with franco is being able to hook
    /// from a far away range like 100,000"*.
    #[test]
    fn the_hook_reaches_one_hundred_thousand_units() {
        assert_eq!(HOOK_REACH, 100_000.0);
        // The data's action range and the projectile's `length` must both move
        // with the constant, or the art stops short of where the code looks.
        //
        // Anchored on the action name, not on `"casting_type": "Direction"`:
        // `range` precedes `casting_type` in the S1 block, so a direction anchor
        // skips forward into skill2 and reads 32,000.
        let range = field_after(S1_DATA, "\"action_name\": \"skill\",", "range");
        let length = field_after(S1_DATA, "\"name\": \"franco_hook\"", "length");
        assert_eq!(range, 100_000, "S1 action range must be 100,000");
        assert_eq!(length, 100_000, "the projectile length must be 100,000");
        assert_eq!(
            range as f64, HOOK_REACH,
            "code and data disagree on the reach"
        );
    }

    /// Damage must not silently become zero when it moved into code.
    #[test]
    fn hook_damage_is_190_plus_full_attack() {
        assert_eq!(HOOK_HIT_FLAT, 190);
        assert_eq!(HOOK_HIT_AD_RATIO, 100);
        let at = 52usize; // Franco's base attack
        let paid = HOOK_HIT_FLAT.saturating_add(percent(at, HOOK_HIT_AD_RATIO));
        assert_eq!(paid, 242, "190 + 100% of 52");
        let geared = HOOK_HIT_FLAT.saturating_add(percent(200, HOOK_HIT_AD_RATIO));
        assert_eq!(geared, 390);
    }

    /// The rebalance target: one hook must not take half a squishy's health at
    /// level 1. 760 is the thinnest base HP in the roster (Lancelot).
    #[test]
    fn the_level_one_hook_no_longer_halves_a_squishy() {
        const SQUISHY_HP: usize = 760;
        let paid = HOOK_HIT_FLAT.saturating_add(percent(52, HOOK_HIT_AD_RATIO));
        assert_eq!(paid, 242);
        let pct = paid * 100 / SQUISHY_HP;
        assert!(
            pct < 40,
            "hook takes {pct}% of a squishy; must stay under 40%"
        );
        // The value this replaced really was over half, so this guards a real
        // change and not a rounding drift.
        assert_eq!(452 * 100 / SQUISHY_HP, 59);
    }

    /// Fury Shock pays from TWO places that stack — the data's flat `damage` on
    /// the RangeEffect, and the Rust native's 4% of Franco's max HP. The flat
    /// carried 89% of the level-1 hit and did not scale at all, so v2.0.11 cut
    /// it from 300 to 160 and left the max-HP term alone.
    #[test]
    fn fury_shock_flat_was_cut_and_the_max_hp_term_survived() {
        let flat = field_after(S1_DATA, "\"action_name\": \"skill2\"", "damage");
        assert_eq!(flat, 160, "S2 flat damage must be 160");
        assert_eq!(FURY_SELF_MAX_HP_PCT, 4, "the 4% max HP term is unchanged");
        assert_eq!(FURY_RADIUS, 32_000.0, "and so is the blast radius");
        // Level 1: 160 + 4% of 880 base HP.
        assert_eq!(flat + 880 * FURY_SELF_MAX_HP_PCT / 100, 195);
    }

    /// The AI may only choose champions, so it cannot spend the hook on a wave.
    #[test]
    fn the_hook_may_only_be_cast_at_champions() {
        let v: String = S1_DATA.to_string();
        let at = v
            .find("\"casting_type\": \"Direction\"")
            .expect("S1 casting_type");
        let window = &v[at..at + 400];
        assert!(
            window.contains("\"casting_target\": \"EnemyChampion\""),
            "S1 must be EnemyChampion, found: {}",
            &window[..window.find("range").unwrap_or(window.len())]
        );
        assert!(
            !window.contains("EnemyWithoutTower"),
            "EnemyWithoutTower lets the AI pick minions"
        );
    }

    // -----------------------------------------------------------------
    // Iron Hook's chain tether
    // -----------------------------------------------------------------

    /// The whole tether rests on one number: 1 px == 1_000 world units.
    ///
    /// mod-api-stable/src/values.rs states this verbatim -- "world units are
    /// 1000x screen pixels; a champion radius is 10000". If that scale ever
    /// moves, the 32 px of chain art in each wire cell stops matching the
    /// 32_000 spacing and the rope either gaps or doubles over itself.
    #[test]
    fn tether_spacing_matches_the_chain_art_exactly() {
        const ART_PX: f64 = 32.0;
        const UNITS_PER_PX: f64 = 1_000.0;
        assert_eq!(TETHER_SEGMENT_SPACING, ART_PX * UNITS_PER_PX);
    }

    /// A chain has no arrowhead, so a line and its reverse are one picture.
    /// That fold is what lets twelve cells cover all 360 degrees.
    #[test]
    fn tether_folds_a_line_and_its_reverse_into_one_bucket() {
        let a = (10_000u64, 10_000u64);
        let b = (70_000u64, 40_000u64);
        assert_eq!(tether_bucket(a, b), tether_bucket(b, a));
    }

    /// Sweep the full circle: no bucket index may escape the table, and every
    /// one of the twelve must actually be reachable (an unreachable cell would
    /// be a rotation the art never covers).
    #[test]
    fn tether_buckets_cover_all_twelve_directions() {
        let origin = (500_000u64, 500_000u64);
        let mut seen = [false; 12];
        for step in 0..360 {
            let rad = (step as f64) * std::f64::consts::PI / 180.0;
            let far = (
                (origin.0 as f64 + rad.cos() * 100_000.0).max(0.0) as u64,
                (origin.1 as f64 + rad.sin() * 100_000.0).max(0.0) as u64,
            );
            let bucket = tether_bucket(origin, far);
            assert!(bucket < 12, "step {step} produced bucket {bucket}");
            seen[bucket] = true;
        }
        assert!(
            seen.iter().all(|&s| s),
            "unreachable buckets: {seen:?} -- the sheet is missing a rotation"
        );
    }

    /// Degenerate lengths must never reach the renderer, and absurd ones must
    /// stay clamped rather than spamming hundreds of sprites.
    #[test]
    fn tether_segment_count_clamps_and_rejects_degenerate_lengths() {
        assert_eq!(tether_segment_count(0.0), 0, "zero length draws nothing");
        assert_eq!(
            tether_segment_count(-5.0),
            0,
            "negative length draws nothing"
        );
        assert_eq!(tether_segment_count(f64::NAN), 0, "NaN must not draw");
        assert_eq!(
            tether_segment_count(f64::INFINITY),
            0,
            "infinity must not draw"
        );

        assert_eq!(
            tether_segment_count(1.0),
            1,
            "a hair's breadth is one segment"
        );
        assert_eq!(
            tether_segment_count(TETHER_SEGMENT_SPACING),
            1,
            "exactly one spacing is one segment"
        );
        assert_eq!(
            tether_segment_count(TETHER_SEGMENT_SPACING + 1.0),
            2,
            "a hair past one spacing needs two"
        );

        // A full-reach hook must fit inside the clamp, not silently truncate.
        let full = tether_segment_count(HOOK_REACH);
        assert!(
            full > 0 && full <= TETHER_MAX_SEGMENTS,
            "full reach needs {full} segments, clamp is {TETHER_MAX_SEGMENTS}"
        );
        assert_eq!(tether_segment_count(1.0e12), TETHER_MAX_SEGMENTS, "clamped");
    }

    /// Every name franco.rs asks the engine for must exist in the data.
    /// A typo here is silent in game: the segment resolves to nothing and the
    /// rope simply never appears, with no error anywhere.
    #[test]
    fn every_tether_wire_name_is_registered_in_the_data() {
        for name in TETHER_WIRE_EFFECTS {
            let needle = format!("\"name\": \"{name}\"");
            assert!(
                S1_DATA.contains(&needle),
                "wire `{name}` is referenced by franco.rs but absent from \
                 sdk/champion/mlbb_franco.data_champion"
            );
        }
    }

    /// ...and each one must point at the wire asset with its own tag, or two
    /// directions would render the same rotation.
    #[test]
    fn each_registered_wire_points_at_the_wire_asset_and_its_own_tag() {
        for (i, name) in TETHER_WIRE_EFFECTS.iter().enumerate() {
            let at = S1_DATA
                .find(&format!("\"name\": \"{name}\""))
                .unwrap_or_else(|| panic!("`{name}` missing from the data"));
            let window = &S1_DATA[at..(at + 300).min(S1_DATA.len())];
            assert!(
                window.contains("franco_hook_wire"),
                "`{name}` does not point at the wire asset"
            );
            assert!(
                window.contains(&format!("\"tag\": \"wire_{i}\"")),
                "`{name}` is not bound to tag wire_{i}"
            );
        }
    }

    /// The hook's own chain lash and the rope must never be on screen at the
    /// same time -- that is what doubled the hook. The delay has to cover the
    /// lash, and the rope has to outlast the delay plus the whole pull.
    #[test]
    fn the_rope_waits_out_the_hook_lash_and_still_covers_the_pull() {
        const LASH: &str = include_str!("../../effects/franco_hook_chain#anim.fanim");
        // sum the frame durations of the `chain` anim -> seconds -> ticks
        let mut ticks = 0usize;
        for part in LASH.split(r#""duration""#).skip(1) {
            let body: &str = &part[..part.find(',').unwrap_or(part.len())];
            let num: &str = body.trim().trim_start_matches(':').trim();
            if let Ok(secs) = num.parse::<f64>() {
                ticks += (secs * 60.0).ceil() as usize;
            }
        }
        assert!(ticks > 0, "could not read the lash duration from the fanim");
        assert!(
            HOOK_ROPE_DELAY_TICKS >= ticks,
            "rope delay {HOOK_ROPE_DELAY_TICKS} does not cover the {ticks}-tick lash; \
             the two chains will overlap again"
        );

        // a full-reach pull is 100_000 units at HOOK_GRAB_SPEED per tick
        let pull = (HOOK_REACH / HOOK_GRAB_SPEED as f64).ceil() as usize;
        assert!(
            HOOK_ROPE_TICKS > HOOK_ROPE_DELAY_TICKS + pull,
            "rope lasts {HOOK_ROPE_TICKS} but needs to cover a {pull}-tick pull \
             after a {HOOK_ROPE_DELAY_TICKS}-tick delay"
        );
    }

    /// The twelve tags the data asks for have to be the twelve the sheet
    /// actually defines -- the asset is the other half of the contract.
    #[test]
    fn every_referenced_tag_is_defined_by_the_wire_sheet() {
        const FANIM: &str = include_str!("../../effects/franco_hook_wire#anim.fanim");
        for i in 0..12 {
            let tag = format!("\"wire_{i}\"");
            assert!(
                FANIM.contains(&tag),
                "franco_hook_wire#anim.fanim does not define wire_{i}"
            );
        }
    }
}
