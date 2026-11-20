# Julian implementation specification — source handoff, not a release

Baseline: the verified 21-hero Eudora source archive, internal 1.1.122, SHA-256 in BASELINE.json. Candidate after applying the patch: internal 1.1.123, 22 heroes. Proposed public label only: v2.3.0-rc.1. The owner controls publication. No returned Julian art or actual Julian match was available here.

## Canon identity and source boundary

Julian's identity is three interdependent skills rather than a conventional separate ultimate: two different casts enhance the remaining one. Scythe is a thrown blade with a sustained enhanced phase; Sword is directional movement with a briefly absent/untargetable enhanced form; Chain is a ground area that roots normally and builds toward a launch when enhanced. Skill use also enables stronger pulling basic attacks and skill hits provide sustain.

Primary qualitative reference: [official Julian spotlight](https://www.youtube.com/watch?v=FAn0Id5_kd4), supported by the earlier [Scarlet Raven spotlight](https://www.youtube.com/watch?v=aZcryXrs1rI). Default appearance was visually checked against the publisher portrait and default model in ART_PROMPTS.md. The art is cyan spectral linked weapons, not a fire mage or crown aura.

Secondary ability snapshots: [Liquipedia Julian](https://liquipedia.net/mobilelegends/Julian), [Propublic Julian](https://www.propublic.gg/heroes/116-julian), [MLBB Analytics Julian](https://www.mlbbanalytics.com/heroes?hero=Julian), and [MLBB Wiki Julian](https://mobile-legends.fandom.com/wiki/Julian). Direct wiki retrieval was blocked, so search-index excerpts were cross-checked. Snapshots disagree on modern cooldowns, basic-attack duration/scaling and sustained-hit counts. No claim is made that all mirrors describe the same live patch. In particular, Enhanced Scythe transitions to a slow-moving damaging phase at a non-minion enemy or maximum range; it is not merely a stationary halo. The implementation below freezes an original TFM2 tuning proposal, not an alleged 1:1 port of every current MLBB number.

The remaining prose and numerical proposals below describe original adaptation decisions and the included source, not copied official ability text.

## Source/data ownership

All damage, target filtering, combo state, delayed pulses and shared gates live in `code/julian.rs`. Data actions contain one Native call each. There must be no extra data-layer Attack, ApAttack or area effect that applies the same damage again. The normal-profile expected_damage hints use the same constants as actual hits. Enhanced conditional totals are deliberately not presented as exact engine AI hints because that trait receives only the caster's narrow StatV1, not an entity/sequence state.

`apply_julian.py` adds a module and register call to lib.rs and appends Julian's tick_all to the existing ZhaskMatchHook. Do not call set_match_hook a second time: the builder has one hook slot and replacing it would break existing hero processing. Both passive update and the existing hook may call Julian; the per-entity same-tick guard prevents double payments.

State keys use the project's EntityKey: simulation origin/scope, seed, entity id and assigned simulation-copy tag. A target struck by a traveling blade also uses that identity rather than only its numeric id. Passive spawn creates a fresh owner record. Passive death removes owned queues even if a player-to-living-champion lookup is no longer available. The match hook handles lingering effects while Enhanced Sword banishes the caster. A decreasing simulation tick resets stale runtime state. Abandoned state uses the existing bounded/pruned project pattern; large parallel simulation batches still deserve live testing.

## Hero baseline

Category Assassin. Tags AP, Magic, Melee, CC. Base stats: Attack 82, Magic Power 50, HP 760, Armor 26, Magic Resistance 22, movement 1080, regeneration 6. Per-level growth: Attack 4, Magic Power 7, HP 100, Armor 3, Magic Resistance 3, movement 4. These are original test values, not copied MLBB stats. The 21 existing hero definitions are preserved.

Action rows: attack 24 ticks, skill/skill2 18 ticks, ult 20 ticks, effects at tick 6. Attack cooldown is 60 ticks. Each ordinary engine skill action has 360 ticks of cooldown. Native cycle rules enforce the additional shared lock. Normal attacks have 28,000-unit range. Skills use 70,000-unit targeting; actual Sword displacement caps at 50,000.

## Combo and timing contract

60 simulation ticks = 1 second. A three-bit mask tracks Scythe, Sword and Chain. The same skill cannot progress a sequence twice. With two distinct bits set, the unused skill is enhanced. That cast clears the mask and locks all three native gates for 540 ticks at level 1, falling linearly to 360 at level 15. Formula uses integer tick rounding. An incomplete sequence expires after 240 ticks without a cast, but expiring the mask does not erase already owed individual cooldowns. Death resets the owned sequence and queues; a new spawn starts cleanly.

Every accepted skill starts or refreshes a 120-tick enhanced-basic window. Failed target validation and failed Sword movement do not consume the native sequence. Existing engine action timing can still consume an attempted action before a rejected native effect; the custom AI checks native and engine gates to avoid repeatedly proposing such casts.

### Engine limitation: shared cooldown display and haste

The project's narrow StatV1 does not expose haste, and the stable API can reduce a cooldown but cannot increase all engine cooldowns to a new shared timestamp. Native gates are therefore authoritative; engine icons may appear ready before the shared debt expires. Engine individual cooldowns can still follow their engine-side scaling, but the additional native shared timer is **not reduced by item haste**. This tradeoff is stated in the tooltip rather than concealed. Do not claim perfect cooldown-bar synchronization. Solving it needs a separately verified engine/UI feature, not a fabricated ABI field.

### Engine limitation: Chain before the normal third-slot unlock

EARLY_CHAIN_ASSIST defaults true. When the engine reports Ult explicitly locked, the authoritative update callback performs Chain at the point where the current combo sequence calls for it. This is an automatic adaptation, not an attempt to unlock an unavailable engine button. It can perform a base Chain opener or an enhanced Chain finisher. It uses valid enemies in range, native sequence/cooldown gates, and champion-first deterministic targeting. If the engine cannot report lock state, the code does not assume locked.

After unlock, the player AI requests Chain through the ordinary Ult input and engine validation. Early assisted Chain has its ground VFX and gameplay but does not force an otherwise locked body-action animation. There is no promise that the Ult UI button is usable at level 1. Setting EARLY_CHAIN_ASSIST false retains the normal engine lock and can stall full combinations until unlock; do not silently change this default in the compiler handoff.

## Skill behaviors and original TFM2 values

### Basic attack / Smith's Legacy attack window

Outside the 2-second window: one physical BaseAttack hit for 100% AD. Inside the window against non-structures: replace that hit with one magic BaseAttack hit for 30 + 100% AD + 55% AP, followed by a short pull, speed 700 for 8 ticks. The hit does not also apply an ordinary physical attack. Structures remain ordinary physical attacks. The stable engine performs the pull and may reject it for immune targets; no forced-coordinate drag bypasses immunity. This does not purport to implement an extra Golden Staff interaction with another hero's passive.

### Scythe

A manually simulated, directional traveling blade advances 4,000 units per tick to 70,000 units. Width tolerance is 8,000 units. Intersection candidates are sorted by progress along the segment then entity id; each target-copy identity can take the traveling hit once. It pierces lane minions and transitions/stops on the first other enemy.

Normal: 95 + 45% AP magic skill damage, 30% slow for 60 ticks, then the projectile ends at the first non-minion enemy or maximum distance.

Enhanced initial hit: 125 + 55% AP, 50% slow. At first non-minion contact or range end, it becomes a slow-drifting spin field: 250 units per tick for up to 96 ticks, up to eight pulses spaced 12 ticks apart, each 10 + 5% AP within radius 18,000. A stationary enemy can leave that drifting field; eight pulses are not guaranteed hits. Rendering uses explicit animation phases along the simulated location, so cosmetic playback does not determine collision or payment.

### Sword

Normal: instantaneous 50,000-unit maximum blink adaptation, with one 85 + 35% AP magic skill hit per enemy along the actual traversed segment. The destination is shortened to the input distance when closer. It can cross terrain but searches backward for a non-solid landing, using the project's existing cached/inferred map helper.

Enhanced: move to that destination FIRST, then request 36 ticks of banish. Six blade-path pulses at 6-tick intervals each deal 16 + 8% AP in a widened line tolerance of 12,000. The global hook continues the pulses while the caster is banished; duplicate updates cannot pay again. This represents the brief disappear/reappear blade sequence, not a physically interpolated dash while invisible. The engine ignores position updates for banished units, so reversing those operations is wrong. Banish duration and its interaction with immunity/targetability require live verification. A rejected optional host operation must not be described as proven invulnerability.

Map selection uses the existing project inference/cache, not a newly guaranteed navigation API. Test landing near walls and map edges in the real supported modes. Do not silently rewrite all other heroes' map inference to fix a Julian art problem.

### Chain

Target a location within 70,000 units. A warning lasts 18 ticks. Normal Chain then deals 70 + 35% AP once to enemies within radius 18,000 and applies Bind for 60 ticks. Allies, towers and nexus structures are excluded. The ground bind is a target-area effect, not an ornament attached to Julian.

Enhanced Chain uses the same delay, then four pulses 12 ticks apart, each 5 + 3% AP and a 30% slow. At 48 ticks after the first pulse, enemies STILL inside the area receive one 130 + 55% AP finisher and Airborne for 48 ticks. The finisher checks the current target positions. Leaving the field avoids it. The separate launch effect marks that final upward event.

### Sustain

A skill cast that hits an enemy champion grants one 10% vamp stack, max three stacks / 30%, refreshed for 300 ticks. Lane minions do not grant these stacks. Multiple hits from the same cast grant at most one stack. One enhanced Scythe's traveling hit and its drifting field share that award flag. The field and repeated Sword/Chain hits cannot each create a new stack per pulse. This deliberately reduces the source's hybrid-lifesteal behavior to the engine's unified vamp stat; no second manual healing callback duplicates it. Actual vamp healing and item interactions require an in-game test.

## AI contract

Use exact Julian identity matching, not a broad substring that could attach the controller to another mod's hero. Read-only think() proposes input and never pays damage, moves the hero or changes sequence state. A low-HP target preference breaks ties deterministically and favors champions over farm targets.

Healthy single-target goal: Chain → Sword → Enhanced Scythe. Group goal: Scythe → Sword → Enhanced Chain. Low-health goal: Scythe → Chain → Enhanced Sword, with the enhanced Sword input aimed away from the selected enemy. Once a sequence starts, its native goal is retained instead of switching branches midway because a second enemy appeared. The engine's unlocked/charge/remaining-cooldown and is_valid_input checks still apply. A baseline spell input that would violate native sequence debt is suppressed; ordinary attack/movement fallback is preserved.

This is an initial deterministic AI, not proof of expert tactical movement. Escape destination, enemy tower safety, early jungle clear and combo reliability must be playtested. Do not tune damage to compensate for an AI that never casts the enhanced finisher.

## Asset contract

ART_PROMPTS.md and ASSET_MANIFEST.json define eight 9-frame character rows, nine 9-frame VFX grids, 72-pixel character cells, a 45-pixel standing body, and a 1656 × 576 final sheet with baseline row 55. Character art is never arbitrarily scaled. The compiler applies one vertical translation per ROW from its first frame; it preserves movement within that row. Every input cell is checked before compositing. Directional VFX are rotated with nearest-neighbor into padded 128-pixel cells with clipping assertions.

333 named view bindings: two 16-direction/9-phase flying Scythes, 9 spin phases, two 16-direction Sword clips, and four static Chain stages. Effects are not mirrored by the client, so the compiler authors all directions. The animation phase is cosmetic; time and hit counts remain native. Read counts from the manifest and generated fanim rather than assuming the old Eudora link-frame constants apply.

The three skill icons are intentional crops of accepted returned spell silhouettes with nearest-neighbor fitting and a dark backing. Review them as actual icon designs; they are not blank placeholders. No synthetic voices, borrowed hero voice identity, or nonexistent sound bindings are included. The draft uses silence pending approved audio.

## Text and validation status

English copy is neutral and describes the adapted behavior. Seven other locale drafts are supplied and inserted only under mlbb_julian; all original hero descriptions/names remain unchanged. Their structure and numerical agreement can be tested, but no native-speaker review is claimed. The compiler agent should review translations for the adaptation wording and avoid claiming language quality merely because JSON parsed.

The source patch was compile-checked against the pinned SDK and tested together with the old roster. Final counts and limitations are in VALIDATION.md. No actual Julian sprite, final art, Windows release DLL, playable runtime or live game validation is asserted by this source handoff.
