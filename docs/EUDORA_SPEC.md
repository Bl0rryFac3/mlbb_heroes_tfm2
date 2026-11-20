# Eudora — approved kit and integration contract

Status: Phase 0 approved on 14 November 2026; implemented in v2.2.0-rc.2 / internal 1.1.122, roster 21. Returned character/VFX sheets compiled, code integrated and automated checks passed. Owner playtested rc.1 successfully but reported weak damage; rc.2 raises spell base damage and AP scaling. The revised damage needs a new playtest. Historical research and explicit balance choices are retained below; see reports/EUDORA_BUILD_REPORT.md for the implementation and limits.

Prepared 14 November 2026 for TFM2 0.7.2, against the published MLBB Heroes v2.1.0 / internal 1.1.120 source. Proposed hero ID: `mlbb_eudora`; adding her would bring the roster from 20 to 21.

## Recommendation

Adapt the current revamped Eudora: a ranged control/burst mage with Superconductor, a fan attack that links marked enemies, a targeted stun that expands against a marked target, and a ground-targeted ultimate with delayed bursts. Preserve the distinction between opening on an unmarked target and following up on an existing mark.

The older kit is a separate version. Do not combine its bouncing S2 and enemy-targeted ultimate with the newer S1 tether without an explicit design decision.

## Research and version evidence

- [Official hero page](https://www.mobilelegends.com/hero/detail?heroid=15): the search-indexed official text specifies a five-second mark on non-minion units. Its teamfight advice uses S1, S2, ultimate, then S1 again; its pickoff advice opens with S2 and follows with a centered ultimate and a linking S1. This supports the revamped direction and both combo orders.
- [Official revamped spotlight](https://www.youtube.com/watch?v=HD1SAHJRA8U): located, but video fetching was throttled. No claim of having inspected its animation frames, exact particle shapes or timings.
- [Wiki ability page](https://mobile-legends.fandom.com/wiki/Eudora): indexed descriptions support a fan attack with a one-second lightning connection, enhanced S2 area control, and an area ultimate with delayed bursts. They list a 1.5-second S1 refund. Direct fetching was restricted.
- [Liquipedia](https://liquipedia.net/mobilelegends/Eudora): its indexed kit also contains the tether, but its passive duration and refund wording conflict with the official indexed page and other sources. Do not use those fields as definitive.
- [Community dataset](https://www.propublic.gg/heroes/15-eudora): corroborates the modern mechanics, but some displayed damage values differ from other snapshots. It is supplementary evidence.

The official site's public English hero-list CDN was also inspected. Its Eudora record still contains the older kit, despite the current hero page's indexed combo guidance describing the revamp. The downloaded record is retained as `official_eudora_list_record.json` specifically as stale-source evidence, not the selected balance authority. The current CMS response could not be retrieved through this environment (HTTP 405).

Unresolved source details: current S1 refund is reported as either 1.5 seconds or 50%; exact tether break distance and damage pulse cadence were not established; ultimate delayed timing, radii and overlap rules were not established. These are explicitly proposed adaptations below. The final art reference must match the selected revamped default design, not simply the older CDN painting.

No claim that every original-game number or animation has been verified. This proposal fixes a clear TFM2 contract without presenting uncertain original-game details as facts.

## Measured baseline

Canonical archive: `MLBB_Heroes_TFM2_v1.1.120_v20_full_source_with_SDK_v2.1.0.zip`.

- Drive ID: `1Qemq4r5KjUEnJJburf4ORCE2pJqjC0Q5`.
- Downloaded bytes: 69,646,743.
- SHA-256: `a6a892a933a38de5eea3ac10265aad32b7c17b0213d8a78afd986d1756d52da5`, matching PROJECT_CONTEXT.md.
- ZIP entries: 1,351; total uncompressed bytes: 80,636,891.
- Relevant source and SDK files extracted under `baseline/`; the canonical archive remains unchanged.

These are actual shipped data, read from this archive:

| Reference | Base HP / AP | HP / AP growth | Relevant behavior |
|---|---:|---:|---|
| Gord | 700 / 64 | 95 / 8 | S1: 105 + 52% AP, 1 s stun, 8 s cooldown; S2: 16 field pulses at 6 + 12% AP; ult: 32 damage applications at 12 + 17% AP |
| Cecilion | 680 / 60 | 95 / 8 | S1: 30 + 25% AP along the path and 55 + 50% AP at the landing area; S2: 60 + 60% AP, 1 s stun, 12 s cooldown |
| Zhask | 690 / 62 | 92 / 9 | S2: directional line, 0.5 s stun, 8 s cooldown; native summon implementation |
| Angela | 700 / 58 | 90 / 8 | Native cone hit selection, live endpoint tether rendering, per-target state and custom player-input AI |

Gusion and Uranus remain unchanged, per the project rule. No existing hero is modified for this proposal.

## Proposed stats

All values in this section are mod balance choices, not a direct port of MLBB stats. Growth uses the same base-plus-level-gains convention as the roster; confirm engine level indexing during integration.

| Stat | Base | Per level gain |
|---|---:|---:|
| HP | 680 | 95 |
| Attack | 80 | 3 |
| Magic Power | 64 | 8 |
| Physical Defense | 20 | 3 |
| Magic Resistance | 24 | 3 |
| Movement Speed | 1040 | 4 |
| HP Regeneration | 6 | 0 |

Category: `Magician`. Initial tags: `AP`, `Magic`, `Range`, `CC`, following the shipped mage family. Basic attack: ranged projectile, 62,000 range, 100% Attack physical damage; a lightning appearance does not silently change basic-attack damage type or add an AP ratio.

## Proposed kit

All damage below is magic damage except the basic attack. AP means current Magic Power. There are 60 simulation ticks per second. The proposed cooldowns are base cooldowns before item effects.

### Passive — Superconductor

Skill hits apply or refresh a five-second mark (300 ticks) on enemy heroes and jungle monsters. Lane minions and structures do not receive the mark. The same hit cannot benefit from the mark it has just applied: inspect the pre-hit state, resolve the relevant branch, then refresh the mark.

Marks are boolean, not stacks. Use a caster-specific identity so one Eudora does not activate another Eudora's enhanced effects. This ownership rule is an explicit TFM2 multiplayer safeguard pending original-game confirmation. Basic attacks do not apply it. Follow-up damage does not recursively schedule new links or Thunderbursts.

### S1 — Forked Lightning

- A forward fan with a proposed 70,000 reach and 50-degree half-angle.
- Initial hit: **100 + 55% AP** per enemy in the fan.
- A previously marked, eligible target receives a live lightning link lasting up to **1 second**.
- During a maintained link: four damage pulses, each **4 + 2% AP**, at ticks 15, 30, 45 and 60.
- At a successful completion: **100 + 55% AP** additional damage.
- Eudora gains **20% Movement Speed** while at least one link remains; one non-stacking speed buff, never one bonus per victim.
- A successful finishing hit refunds **1.5 seconds** of S1 cooldown, **once per cast**, even if several links finish. This is the selected mod rule amid conflicting source refund descriptions.
- Base cooldown: **7 seconds** (420 ticks).
- Proposed break range: **95,000**. Breaking a link, caster death, target death or loss of targetability ends that link without its finishing hit or refund. Caster CC does not automatically break a connection if both endpoints remain valid.
- Initial fan damage is doubled against lane minions. Minions receive no link or mark.

The pulse schedule, break distance, non-stacking 20% speed and once-per-cast refund are deliberate adaptations. Tick 60 pays exactly one final pulse and one finishing hit. A cancelled connection pays neither remaining pulses nor the finishing hit. Do not create an additional hit through a duplicate data-layer damage effect.

### S2 — Ball Lightning

- Targeted orb, proposed range **75,000**; actual impact resolves the hit.
- Reduce Magic Resistance by **8** for **1.8 seconds** (108 ticks), then deal **115 + 45% AP** and stun for **0.8 seconds** (48 ticks).
- If the primary target was already marked before this impact, apply the same reduction, damage and stun to other eligible enemies in a proposed **22,000-radius** area around it.
- Count the primary target once. Nearby victims do not create additional explosions.
- Base cooldown: **11 seconds** (660 ticks).

This is the revamped area-stun interpretation, not the older three-target bounce. Ordinary minions can take damage and control, but cannot be used as a newly marked trigger for the enhanced branch. Structures remain excluded. The flat reduction refreshes instead of stacking from the same caster; use the host's normal resistance calculation rather than a custom damage multiplier.

### Ultimate — Thunder's Wrath

- Ground-targeted area, proposed cast range **80,000**.
- Inner radius **15,000**: **220 + 100% AP**.
- Outer ring, beyond 15,000 and up to **30,000**: **110 + 50% AP**.
- Inner and outer damage are mutually exclusive for each target. A center victim does not also receive ring damage.
- Snapshot which eligible enemies were marked before the initial hit. Each such enemy seeds one delayed Thunderburst at the position where it was hit.
- After **0.6 seconds** (36 ticks), each seed explodes in a proposed **18,000-radius** area for **135 + 65% AP**. The warning remains on the ground; it does not chase the enemy.
- Proposed overlap safeguard: one victim can receive at most **one delayed Thunderburst hit from this cast**, even when several seed circles overlap. Different Eudora casts remain independent. This is a balance adaptation, not a verified original-game rule.
- Base cooldown: **32 seconds** (1920 ticks).

The new mark applied by this ultimate cannot enable its own delayed branch. The scheduled burst uses a cast-time AP snapshot; initial and delayed phases use normal typed skill damage and original-caster credit. Once the ultimate has been released, scheduled ground bursts may finish after caster death, but all pending state is cleared at match end. The lifetime of the original caster handle must be respected; never redirect credit to a reused entity ID.

## Burst budget

A full single-target marked combo including initial and completed S1, four link pulses, S2, inner ultimate and one delayed burst has a raw maximum of:

`468 + 214% AP`.

| AP scenario | Raw damage, before resistance/shields |
|---|---:|
| 64 AP, proposed starting power | 600 |
| 176 AP, 14 gains of 8 | 839 |
| 400 AP, itemized scenario | 1324 |

The formula is the continuous-coefficient budget; the table floors each individual AP contribution before summing damage, as proposed for the implementation. These figures exclude basic attacks, minion multipliers, item procs and extra S1 casts. They assume an existing mark and a fully maintained link; they are not an estimate of damage from an unmarked three-button sequence. The initial mark's setup damage is separate. Integration must regenerate this table using exactly the same arithmetic as the implemented code.

For context, Gord's complete uninterrupted ultimate alone is `384 + 544% AP` from its 32 data-layer damage applications. Eudora's package is faster, includes control and resistance reduction, and therefore should not inherit that sustained-damage budget. This comparison does not prove balance. Starting-level burst, itemized penetration and repeated refunded S1 casts need actual matches.

## Engine mapping and implementation boundaries

| Requirement | Evidence in this exact baseline | Proposed implementation |
|---|---|---|
| Mark and pre-hit branching | Gord target buffs; Angela per-target state | Native hit resolver with caster-specific mark lookup |
| Fan geometry | `sdk/candidate/angela.rs::wave_contains` | Reuse cone math, but apply an instantaneous fan rather than Angela's traveling wave |
| Live lightning link | `angela.rs::draw_string`, based on Estes | Follow both endpoints with short-lived effect segments; source lightning art remains separate |
| Timed pulses and delayed area damage | Existing hero passive update loops | Per-caster cast/link/seed state with deterministic tick deadlines |
| Resistance reduction | `sdk/mod-api-stable/src/values.rs::BuffV1.magic_resistance` is signed `i32` | Named temporary buff with `magic_resistance = -8` |
| Stun and typed damage | Stable `apply_cc`, `deal_damage_typed`; existing mage code | Host CC and damage APIs, never direct HP subtraction |
| S1 refund | `sim.rs::entity_reduce_cooldown` | Subtract 90 ticks once per successful cast; verify ABI-level-10 availability in the running 0.7.2 host |
| AI combo decisions | `angela.rs::AngelaAi`, registered through `add_player_input_ai` | Eudora-only input wrapper with unlock, cooldown, range and target validation |

Core mechanics have source-backed implementation paths. They are not implemented or game-tested yet. Optional host calls must be checked: if cooldown reduction is unavailable, report it explicitly and withhold the refund claim rather than silently treating the SDK declaration as runtime proof.

No mana system is added. Original mana costs are omitted under the current roster's resource model. No battle spell, bush ambush system or fog-of-war feature is added. Eudora remains usable without MLBB Items installed; item stats and procs must continue through normal host behavior when it is installed.

### AI contract

1. Evaluate live targets, not just the order of skill slots.
2. Against a group, prefer S1 to establish marks, then marked S2 for area control, then ultimate centered on the controlled cluster.
3. Against an isolated priority target, allow S2 first, followed by a centered ultimate and a marked S1.
4. Keep ultimate available against a valid unmarked target when setup is unavailable and a direct cast is worthwhile; never wait forever for a perfect combo.
5. Do not block basic attacks or movement while waiting for skills; do not chase beyond safe range merely to finish a link.
6. Respect ability unlocks, real remaining cooldowns, cast recovery, death/banish and targetability. Issue normal engine inputs rather than free native casts.
7. Use deterministic tie-breaking and bounded scans for multiplayer. Confirm behavior in new careers, continued saves and the relevant match modes.

AI preference is an implementation goal, not a guarantee of a fixed scripted combo in every fight. Decisions must be verified from casts in actual matches.

## Art and animation contract for later phases

After Phase 0 review, gather the revamped default-skin splash/model references. Measure the base against the current roster, facing toward the viewer's right, with true transparency. Character motions require eight rows: idle, run, attack, skill, skill2, ult, hit and dead. Preserve native-size frames; compile and verify alignment here.

Plan these separate visual assets, with exact counts and geometry finalized against the approved mechanics and animation:

1. Basic lightning bolt and impact.
2. A small persistent Superconductor mark on the victim.
3. S1 fan discharge, endpoint-following link segment and finishing spark.
4. S2 traveling orb and enhanced area impact.
5. Ultimate ground warning, central descending strike, outer discharge and delayed Thunderburst.

SpriteCook produces individual base/first frames; the owner animates them in PixelLab. No generated multi-frame strip and no credits spent during Phase 0. A caster animation should not duplicate a world effect drawn by the runtime. Bind persistent marks to victims, moving connections to live endpoints, and ultimate warnings to fixed ground coordinates.

## Description contract

Use neutral third-person descriptions: Eudora, the target, enemies and allies. No “you” or “your.” Keep concise sentences and the roster's supported color/stat-icon markup. Names, damage, duration, range and conditions must match the actual implementation in all eight locales. If a passive/basic-attack companion card is required, add it to that separate mod rather than assuming the main ability panel has a passive slot.

Example English wording to regenerate from implemented constants later:

- **Superconductor:** Skill hits mark enemy heroes and jungle monsters for 5 seconds. Eudora's skills gain additional effects against enemies already marked.
- **Forked Lightning:** Lightning strikes enemies in a fan. Marked enemies are linked to Eudora for up to 1 second, taking damage over time and a final hit if the link holds. Eudora moves faster while linked. A completed link reduces this skill's cooldown once per cast.
- **Ball Lightning:** An orb strikes an enemy, reducing Magic Resistance and stunning the target. Against a marked target, the effect also hits nearby enemies.
- **Thunder's Wrath:** Lightning strikes an area, dealing more damage at its center. Enemies already marked create delayed explosions at their hit locations. Overlapping explosions from the same cast damage each enemy once.

## Acceptance checks for implementation

- An unmarked first hit only applies the mark; the next skill gains its enhanced branch. Marks refresh without stacking and do not transfer between casters.
- Lane minions never gain the mark; jungle monsters do. No structure is marked, stunned or included in area damage.
- S1 hits only its fan, handles multiple links, grants one speed bonus, and pays exactly four pulses plus one finishing hit on completion. An interrupted link pays no finishing hit. Five completed links still refund only 90 ticks.
- S2 checks the pre-impact mark and hits the primary once. Resistance reduction expires/refreshes correctly, respects host math, and is not converted to penetration or a generic vulnerability multiplier.
- Ultimate tests just inside, on and outside both radius boundaries. No double center/ring hit. A moving target can leave a ground warning. Unmarked victims do not seed a burst from the mark this cast just applied. Overlap applies the documented cap.
- Native hits count as skill magic damage with the correct attacker; item on-hit behavior and stasis are checked with and without MLBB Items. No direct-health damage bypass.
- Death, banish, target invalidation, despawn, respawn, reused entity IDs, match restart and two Eudoras do not retain stale connections or pending effects across matches.
- AI casts all three skills, uses both sensible opening orders, and never bypasses locks or permanently waits for a marked target.
- Sheet/animation bindings, actual PNG formats, skill icons, view positions, voice routing and all eight locales pass the current strict gates. The runtime and full-source archives contain the same DLL and matching source.

## Phase 0 review

Recommended decisions: use the revamped kit; accept the proposed reduced damage/CC/movement values; use a fixed 1.5-second S1 refund once per cast; cap same-cast overlapping Thunderbursts at one hit per victim. Keep the source uncertainties visible in the research record.

The owner approved the proposed kit and balance choices before Phase 1 artwork. Nothing in this document is a shipped feature or an in-game verification result.
