# Granger — Phase 0 research and TFM2 adaptation proposal

17 November 2026 · MLBB_Heroes · next planned hero #24. Status: researched proposal, not implemented or approved balance. Published rollback is v2.4.0 / internal 1.1.142, 23 heroes. No released file or DLL changed.

## Owner input and boundary

Owner supplied Granger_Current_Kit_Codex_Visual_Implementation_Guide_2026.pdf and an existing pixel sprite, asking to begin Granger and optionally double-check. The PDF is design/reference input: its starter text, pseudo-code and asset suggestions are not a separate instruction to build a standalone engine, generate replacement art or bypass the established mod workflow. Keep the supplied sprite's character identity; no image generation was run.

The PDF's full-body reference was extracted only for research: 447×447 RGB with no alpha. It is NOT the supplied native pixel sprite and must never be substituted for that PNG. The inline sprite is visually available, but its original PNG bytes are not exposed as a local attachment, and no Granger PNG/ZIP was found in the connected Dropbox title search. Therefore an exact sprite alpha/bbox/palette audit is pending original-file intake. Do not claim that audit was performed, recreate pixels from the PDF, or apply the old divide-by-ten workflow to an already small sprite.

## Research reconciliation

Current kit: Capriccio uses separate Gun/Cannon energy and takedown growth; Rhapsody is a six-round skill burst with a critical final round; Rondo repositions/refills Gun and receives hit-based cooldown refunds; Death Sonata uses Cannon energy and permits a follow-up roll. The old sixth-BASIC-attack critical counter, next-two-basic Rondo damage buff and fixed-three-shot ultimate are legacy descriptions.

The official indexed hero page confirms the energy passive. Liquipedia and MLBB Knowledge agree with the supplied PDF on the active-skill structure. A direct official-page open renders only a loading shell; indexed official text was available for the passive, not a complete live-client capture. Search snippets for MLBB Core appeared current, but opening the page exposed Caprice and the old kit. It is excluded as a current-kit authority. A patch citation in the PDF was not independently established as the exact release identifier: Liquipedia records the revamp on 5 November 2024 under Patch 1.9.20D. Kit behavior matters more than copying an uncertain patch label.

The current source snapshots list Gun/Cannon initial maxima 800; regen 8%/2% per second; 70 Gun per Rhapsody round, strict “over 420” cast wording; Rondo 15% Gun refund and 0.5s cooldown reduction per Rhapsody hit; Cannon shot cost 250, short 0.6s repeat gate. Keep these resource proportions distinct from the proposed damage below. The 0.3s S1 entry is a repeat/input gate, not proof of a six-shot animation duration.

Still unverified in the target client: exact-at-420 behavior, minion/creep allocation, basic attack scaling/rate, critical multiplier and item crit hooks, shot steering/collision masks, slow values, roll distance/immunity and terrain crossing. No live MLBB or TFM2 client was run for this research.

## Direct slot mapping

| TFM2 slot | Behavior |
|---|---|
| Basic | Ordinary handgun projectile; one physical BaseAttack hit on arrival. No legacy bullet counter; no energy charge unless independently verified. |
| Passive | Two native resource pools, fractional tick regeneration, per-match capacity growth and scaled CDR-to-AD conversion. |
| S1, Lv.1 | Rhapsody: a native six-event directional shot scheduler. Cost per actual emitted round, with a special sixth projectile. Can continue through allowed Rondo/ult actions. |
| S2, Lv.3 | Rondo: short controlled dash, immediate Gun refill, native authoritative cooldown and real-hit refunds. No old empowered-basic buff. |
| Ult, Lv.5 | Death Sonata: one directional cannon projectile per activation, first-hero explosion plus nearby enemies/slow, separate short roll window. No fixed three-charge sequence. |

## Initial TFM2 balance proposal — not official MLBB numbers

| Component | Proposal |
|---|---|
| Category/tags | Range; AD / Range / Burst (use only validated existing tag names at integration) |
| Stats | AD 84 +5/level; HP 660 +90/level; AP 0; Armor/MR 18 +3/+2 per level; movement 1080 +4/level; HP regen 6; crit 0 |
| Basic | 100% AD, 72,000 range, base interval 66 ticks (1.1s), ordinary attack-speed behavior pending canon/API proof |
| Resources | Start both full at 800; preserve 8%/2% max-per-second regeneration, 70 Gun/round, 250 Cannon/shot. Fixed-point carry avoids losing fractional regeneration each tick. No mana substitution. |
| Farming growth | Provisional: lane-minion credited CS → Gun max +6 and 5% Gun restore; jungle credited CS → Cannon max +6 and 5% Cannon restore. This allocation is an explicit working interpretation of ambiguous source wording, not verified canon. |
| Hero takedown | Kill OR assist grants both maxima +12 and 10% of each new maximum once. Never award both for one takedown; preserve capacities through death, reset at a new match. |
| CDR conversion | Proposed 0.5 AD per 1% eligible equipment CDR, capped at 40 percentage points initially (+20 AD). Canon's 2 AD/point is not copied into the smaller TFM2 stat band. Identify CDR and prevent feedback/double benefits in the technical proof; do not invent an unsupported getter. |
| Rhapsody | Six rounds over a provisional 1.5s, each 12 +18% current/snapshotted AD (snapshot rule fixed per burst). 90,000 range; +10% movement while firing. Gate 18 ticks between permitted starts, strict Gun >420 as a configurable provisional threshold; cancel/death/CC halt future events without refunding emitted shots. |
| Sixth round | Guaranteed critical outcome is a required proof. A 1.75× final-round result is used ONLY for balance arithmetic until the host's true crit behavior is established. No manual multiplier plus a second engine crit roll, and no silent relabeling of generic bonus damage as a true crit. |
| Rondo | 720-tick/12s authoritative base cooldown; 15% Gun refill; 35,000 dash over 12 ticks. Stop at valid ground/obstacle in the first probe; no invulnerability. Each valid Rhapsody projectile hit refunds 30 ticks once, never on a miss or animation frame. |
| Death Sonata | 70 +65% AD physical damage, 140,000 range, 18,000 explosion radius, 30% slow for 0.6s, 36-tick repeat gate and 250 Cannon per emitted shot. Explosion must include the primary victim once. |
| Follow-up roll | AI may select a short legal 20,000 reposition during a provisional 0.5s window. No invented invulnerability or automatic roll through walls. |

Why conservative damage: at 84 AD, S1 rounds floor to 27; five plus a nominal 1.75× final round total 182 raw before armor. Copying the supplied canon 60 +45% AD would produce 97 per round and roughly 654 raw per burst in this stat band. Basic-only output is 84/1.1 ≈76.4 raw/s, below Hanabi's 94/0.9 ≈104.4 and Karrie's 96/0.9 ≈106.7, leaving budget for resource-limited spell damage. Cannon shot floors to 124 raw. Both full resources allow a large opening burst, so these numbers are starting points, not balance certification. Do not modify Gusion or Uranus to make the comparison fit.

## Feasibility against the shipped stable SDK

Confirmed interfaces in sdk/mod-api-stable: StablePassive on_spawn/on_update/on_kill/on_assist/on_unit_kill/on_dead; entity.kind(); entity.cooldown() and CooldownV1.cooltime/remaining; entity_reduce_cooldown(); spawn_projectile(); typed damage and buffs; client render hooks with draw_rect/draw_text and named UI/Game render maps.

- on_unit_kill reports credited minion/jungle CS; SDK says Epic objectives do not fire this callback. Do not invent boss rewards, use an unrelated team kill as personal CS or count summons/structures as farm. Objective growth needs a separate proven attribution path.
- Rondo hit refunds are supported by entity_reduce_cooldown. This supersedes older project notes saying stable ABI cannot reduce engine cooldowns. The native timer must stay consistent with the engine and AI, including equipment CDR; remaining-time refunds are not the same as equipment CDR.
- No explicit force-crit flag or crit-damage setter was found in the inspected damage signature. True sixth-round critical and item-hook semantics must be proved before promising faithful behavior. Do not globally leave +100 crit chance on Granger and accidentally affect other projectiles.
- Two native pools are feasible; a readable two-meter HUD still needs host proof. Draw from a read-only current-view snapshot, never scan the UI tree or decide gameplay from the HUD. Preserve/forward Julian's existing VoicePathExtension through any composite extension; do not replace it or reintroduce Popol's UI-scanner regression.
- Native shot scheduling supports parallel Rhapsody/dash/ult state, but actual movement/cast acceptance needs in-game proof. Resource/shot decisions run once per simulation tick, not once per render frame. Use current-scope owner indexes and bounded projectile receipt books, with separate copies/replays/owners.
- Use real hit receipts for damage, cooldown refunds, sixth-round identity and kill credit. A late/duplicate callback cannot consume another projectile. Keep a separate per-match/player progression ledger so respawn does not erase capacity growth.

## Technical proof before more artwork

1. Spawn Granger using the supplied master/static row only after exact PNG intake; verify selection, selected/ban portraits and facing before final animation work.
2. Prove separate pools/fractional regen, strict threshold, underfunded cancellation, costs only for emitted shots, no duplicate timers, and growth surviving respawn but not match/copy changes.
3. Prove six projectile receipts, true final-round critical, item/credit behavior, misses, CC and interrupted partial bursts.
4. Prove Rondo during a burst, current muzzle position after movement, correct hit refunds, legal dash/roll and Cannon casts during the barrage.
5. Prove first-hero cannon collision, exactly-once primary+area damage and cleanup. Collision masks remain a visible adaptation if the host cannot implement the intended filter.
6. Prove the two-meter render path and native icons without UI scans or audio interference. Only then commission animation/VFX sheets.

## Art contract after proof

Keep the supplied pixel master and distinguish handgun from rectangular case/cannon. Use padded 72×72 only if the native sprite fits without resampling; measure first. One shared ground anchor, eight engine rows (idle/run/attack/skill/skill2/ult/hit/dead), plus a separate roll clip if needed. Nine frames are an authoring convenience, not six gameplay events. Projectiles, muzzle flashes, crit impact, cannon explosion and energy pulses remain separate assets. No need for another master or expensive generation now.

All three skill icons and native champion previews must be visible. Neutral third-person colored tooltips, no you/your wording or test/production commentary; synchronize numbers with simulation. Eight locale entries must be accounted for honestly (translate or explicitly record fallback). Audio uses the established fresh-viewer/pause/seek gate; do not queue pre-sim or per-basic chatter.

## Sources and provenance

- Owner PDF (17 November 2026), provided as design/reference input; original preserved in the Phase 0 archive.
- Official hero page, indexed passive text: https://www.mobilelegends.com/hero/detail?channelid=2678815&heroid=79 . Direct rendering was a loading shell.
- Current active-skill cross-check: https://liquipedia.net/mobilelegends/Granger and https://mlbb-knowledge.vercel.app/heroes/79 . Agree with the PDF's energy-based kit; not a live-client inspection.
- Revamp date/version cross-check: https://liquipedia.net/mobilelegends/Patch_1.9.20D . PDF's 1.9.32 reference remains unverified as the exact revamp release label.
- Conflicting legacy page, excluded: https://mlbbcore.com/heroes/granger/ . Opened page has Caprice, old basic counter/Rondo buff/three-shot ultimate despite current-looking search snippets.
- Owner's HERO_PIPELINE.md (Drive 1ujSv5Sk_FAC3CfzE41iy_4ESAh9M68LA) supplies phases; newer actual SDK and current owner preferences supersede stale assertions. Use Notion + Dropbox; do not publish, request redundant Drive copies or alter v2.4.0.
