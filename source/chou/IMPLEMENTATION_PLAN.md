# Chou for TFM2 — Internal Implementation & Validation Plan

**Status (2026-10-26):** v1.1.40 is a cautious damage-only follow-up to v1.1.39. S1 is now 100–165 + 45% total physical attack; Ultimate first phase 275–420 + 110% extra attack; recast 330–500 + 132%. This models roughly +10% early and up to ~16–20% late versus v1.1.39, without restoring published MLBB values. Event-level raw-damage logs and regression tests were added. Shunpo and v1.1.39 range/knockback/combo behaviors are unchanged. New runtime trace review: 10 recast events were 336–397 raw damage (median 367), only two match-origin; no numeric S1 damage was logged. This is a test hypothesis, not a validated balance result.

v1.1.39 follows the new v1.1.38 runtime report: the 145,000 action range let first-phase target input appear to cast at distant targets before the native 30,000 guard rejected the effect. Base Ultimate range is now 20,000 (15,000 basic-attack range), with a +125,000 native dynamic range bonus only while the recast buff is active; AI trigger is 18,000. The user then requested a 100,000-unit first-phase knockback experiment (10,000 × 10), up from the observed ~60,000. TFM2 needs to confirm both phase-specific range and actual displacement. v1.1.38 follows the v1.1.37 combo trace with a one-retry/short-wait transition guard, match-correlated AI logs, 45-tick S1 timing, 20,000 S1 dash, and 24,000 first-Ult AI trigger (30,000 native hard cap, 145,000 recast range). Knockback strength remains unchanged. v1.1.37 is a separate experimental Chou Insec-combo candidate based on v1.1.36: AI sequences S1 x2, optional S2 approach, S1 third airborne, refreshed S2 to the far side of the target, and Ultimate phases; first-phase knockback stays unchanged and positional diagnostics were added. v1.1.36 is a separate experimental S1 speed candidate: all three S1 clips/action duration/AI lock are 48 ticks (20% faster), with finisher impact delay 10 ticks after the 2-tick start. v1.1.35 remains the user-confirmed rollback. No other mechanics are intended to change. v1.1.35 applies the requested cautious behavior changes while retaining v1.1.34 animation speeds: S1 dash 15,000 units; third-cast airborne via a delayed data-driven 30,000-radius RangeEffect at slam impact; Ultimate recast airborne for 60 ticks; and a 30,000-unit first-phase Ultimate range gate in both AI and native effect, with the shared slot range still 145,000 for recast chase. Chou event logs remain enabled. Formatting, 34 Rust tests, Linux/Windows x64 release builds, JSON/static integration validation and ZIP CRC checks pass. No TFM2 match test has been performed; dash visibility, CC rendering, and phase-specific range still need in-game verification. After the user authorized storage cleanup for irrelevant old candidates, the v1.1.27/.28 Linux/Windows ZIPs were removed; their audit JSONs remain. The v1.1.35/.36/.37/.38 rollback ZIPs remain untouched.

## 1. Scope and confirmed user decisions

- Add Chou to the existing `mlbb_heroes` pack using the Uranus/Gusion/Estes integration conventions.
- Use the submitted sprite/manifest; retain the second supplied design as the style reference. **No nunchaku, weapon, or prop.** The selected sprite shows empty hands.
- Skill effects are baked into the character frames. No separate effect sheet or effect assets were created.
- Use Fandom's Chou page as the requested mechanics baseline, adjusted for later patch information and TFM2 conventions.
- Use default Chou voice clips for Skill 1 and Ultimate; use the Iori Yagami clip for Skill 2. No extra idle/death/move voices are assumed.
- S1: cast 1 high/rising kick; cast 2 dash uppercut; cast 3 leap/downward slam with baked circular shockwave and airborne target.
- S2: directional dash with baked glow/trail.
- Ultimate: roundhouse knockback, then recast chase and aerial **kick flurry** (not a fist-focused flurry).
- The user prefers several smaller Ultimate recast events if proc/CC behavior proves safe; otherwise one total event with the flurry animation.

## 2. Mechanics sources and provisional tuning

Use the [Fandom Chou page](https://mobile-legends.fandom.com/wiki/Chou) as the requested baseline, but not as the sole numeric source: it is marked outdated. Its [patch history](https://mobile-legends.fandom.com/wiki/Chou/Patch_history) does not cover the latest tuning. [Patch 2.1.67a](https://liquipedia.net/mobilelegends/Patch_2.1.67a) is used for the later S1 backswing removal, increased basic-attack range, Ultimate cooldown/damage changes, and shortened recast knockback. Newer secondary sources differ on the passive slow and ultimate coefficient; treat those as tuning questions, not settled facts.

Current native implementation uses the following TFM2 approximations:

- **Only Fast:** charge after 42,000 world units of movement. Next basic attack deals 180% AD and is temporarily protected from crit via a negative crit-chance buff; it slows 80% for 1 second and reduces physical defense by 10 for 2 seconds, up to 2 stacks. This approximates movement distance at TFM2 scale and does not apply the Fandom per-level increase. Verify the crit modifier path and slow/shred values in-game.
- **Jeet Kune Do:** three casts; each deals line damage of 100–165 + 45% total physical attack (v1.1.40; previous v1.1.39: 90–140 + 40%). First two reset the skill cooldown so the sequence can continue; third starts the normal 540-tick cooldown and applies 0.5s airborne. A cast-3 enemy-hero hit fully resets Shunpo. The recast state lasts 6 seconds. A cast is locked for 60 ticks to let its 1-second animation play before the AI recasts.
- **Shunpo:** `MoveTo` dash capped at 30,000 units at 3,000 units/tick, 12 ticks of CC immunity around the dash, then 200–300 + 100% extra physical attack shield and +15 physical penetration for 2.5 seconds. Confirm exact dash collision/CC lifetime in TFM2.
- **The Way of Dragon:** first phase base 275–420 + 110% extra physical attack; 4-second recast window. Recast base 330–500 + 132% extra physical attack and chases the saved target. The code uses **one damage event** for proc safety; the full multi-kick animation is retained. Chase travel is capped at 30,000 units at 3,000 units/tick. Cooldown is fixed at 1,980 ticks (33 seconds), the midpoint of the 36–30 second reference range.

## 3. Assets and active animation

Original atlas/manifest are preserved at `source/chou/chou_atlas.png` and `source/chou/chou_atlas.json`. Atlas dimensions are 3672×576 RGBA: 51 columns × 8 rows of 72×72 cells, feet pivot x=34, ground y=55. The active sprite sheet and `.fanim` were generated with the reproducible script `source/chou/prepare_assets.py`:

- `sprite/chou#sheet.png`
- `sprite/chou#anim.fanim`

Fanim tags: `idle`, `run`, `attack`, `skill`, `skill_recast`, `skill_finisher`, `skill2`, `ult`, `ult_recast`, `hit`, `dead`. The original `skill` row is split into three 17-cell ranges; the Ultimate row is split into 10-cell first phase and 25-cell kick flurry. `CasterAnimation` is used to select recast tags. The active skill timelines now run about 0.72–1.0s; action durations are aligned to those clips and S1 recasts were gated for 60 ticks in v1.1.35; the separate v1.1.36 experiment retimes all three clips and the action/AI lock to 48 ticks. Durations and retargeted animation timing still need a live visual check.

The downloaded original icons are under `source/chou/icons/`; active 100×100 PNGs are in `icons/` (`chou_passive.png`, `chou_skill.png`, `chou_skill2.png`, `chou_ult.png`). The hero action icon array uses S1/S2/Ultimate; the passive icon is retained for future UI use.

Original OGGs are in `source/audio/chou/`. Converted mono PCM16 22.05kHz WAV/event pairs and `.sound_info` files are in `sound/sfx/`; IDs: `mlbb_chou_voice_skill1_a`, `mlbb_chou_voice_skill1_b`, `mlbb_chou_voice_skill2`, `mlbb_chou_voice_ult`. Skill 1 uses two default-voice alternatives across its stages; Skill 2 uses Iori; Ultimate uses default Chou. Cues are registered in `src/audio.rs` and event+clip overrides are in `mod.override_info`.

## 4. Current implementation map

- `champion/mlbb_chou.data_champion`: champion stats, action slots, icons, data-driven `SwitchByBuff`, `MoveTo`, and animation cues.
- `src/chou.rs`: native `StablePassive`, base-attack effect, S1 line geometry/stages, Shunpo shield/penetration, Ultimate target effects, and Chou-specific combo AI.
- `src/lib.rs`: registers the Chou module.
- `text/champion.i18n`: Chou names and descriptions in English, Simplified Chinese, Traditional Chinese, Korean, and Japanese.
- `style/champion_view.champion_view`: provisional portrait/selection offsets.
- `source/chou/prepare_assets.py` and `source/chou/integrate_metadata.py`: reproducible atlas and metadata integration helpers.

The native AI matches Chou, advances S1 recasts when the action reports an available charge, checks each skill slot’s engine `unlocked` state and charges, ignores ordinary base-AI suggestions when checking its combo state, approaches with the shorter/slower Shunpo dash, and stores the first Ultimate target until its cooldown is ready for recast. It does not change other heroes' AI. S1 direction is derived from a position input and hits a forward line; structures are excluded. Its half-width is 30,000 units, and the AI only attempts a staged recast when the selected target is within the 67,000-unit S1 range. Basic attack uses the engine's `deal_damage` pipeline. Passive anti-crit uses a short-lived negative crit modifier immediately around that damage call; this is **not yet proven in runtime**.

## 5. Required validation before release

### Static and build

- **Passed:** `cargo fmt --check`; `cargo test` (33 passed, 0 failed); optimized Linux x64 build; optimized `x86_64-pc-windows-gnu` DLL build using MinGW-w64; release ZIP builder asset checks; and ZIP CRC checks. The Windows artifact is confirmed PE32+ x86-64; Linux is ELF x86-64.
- Validate all `.data_champion`, `.i18n`, `.champion_view`, `.override_info`, `.sound_info` JSON, all sprite rectangles, all icon/audio references, and no duplicate IDs. JSON parsing, frame-bound checks, and staged audio file checks passed.
- Confirm TFM2 accepts the nested `SwitchByBuff`/`CasterAnimation` combination, plays the selected animation for its configured duration, and honors queued third-stage impact effects.

### Mechanics/runtime

- Passive movement accumulation, one-hit consumption, crit suppression, defense stack cap/refresh, and slow duration.
- S1 charge consumption/recharge, staged animation playback with the 15,000-unit dash, delayed third-cast landing hit/airborne and 30,000-unit radius, normal cooldown after cast 3, and S2 reset only on a cast-3 champion hit. Check that movement/cast cancellation cannot desynchronize visual and buff stages.
- Shunpo dash collision, valid directional behavior, CC immunity limited to the dash, and shield/penetration arriving only at dash end.
- Ultimate first-phase close-range rejection, 60,000-unit knockback followed by explicitly targeted queued 0.5s airborne, 60-tick recast airborne, post-action cooldown reset, target lock, 4-second recast expiry, target death/despawn, wall/collision handling, and cooldown behavior. Queued target delivery and timing still need a live runtime check. Verify the chase truly moves Chou rather than creating a teleport-like snap.
- Confirm the Ultimate recast's single damage event avoids unwanted extra procs. If safe, switch to several per-kick events whose sum equals the displayed total; if not, keep the current single-event fallback.
- AI should not lose recasts, retarget mid-Ultimate, repeatedly waste a stale stage, or outrange its action target.

### Visual/package

- Check actual sprite scale/pivot, baked effect visibility, action start/hit timing, portrait framing, and target delivery in TFM2.
- Record any remaining API approximation in the release README. The v1.1.27 and v1.1.28 candidate archives remain untouched; v1.1.32 should not be called a final release until in-game validation succeeds.


## v1.1.36 experimental timing test

- Keep `/home/user/mlbb_heroes_work/mlbb_heroes_v1.1.35_windows_candidate.zip` untouched as the user-confirmed rollback.
- S1 clips preserve all 17 frames and scale each frame duration uniformly to 80%; each totals 0.8 seconds / 48 ticks. Skill action duration and `S1_ACTION_LOCK` are 48; the existing AI +2-tick guard remains.
- Finisher `Delayed` tick is 10 (plus action `start_timing: 2` ≈ 12 ticks total), matching the frame-3 slam onset after retiming. Damage, 30-tick airborne, radius, dash, 3 charges, cooldown, and recast window are unchanged.
- Static/build validation proves only configuration/source consistency. Compare v1.1.36 against v1.1.35 in TFM2 and roll back if the clip is truncated, impact desynchronizes, or the increased cadence is undesirable. This change does not introduce S2 weaving or change AI action priority.


## v1.1.37 experimental Insec combo test

- Preserve the v1.1.35 user-confirmed candidate and v1.1.36 faster-S1 candidate ZIPs untouched.
- AI locks the chosen enemy hero through the S1 chain; at long range it may S2 approach between cast 2 and cast 3. On a confirmed hero hit by the S1 finisher, the native effect marks the hit and the AI uses the refunded S2 to aim 20,000 units onto the opposite side from the nearest allied champion/structure. It checks side and 30,000-unit first-Ultimate range before kicking.
- First-phase knockback speed/ticks remain exactly 6,000/10. The new trace records caster and target start coordinates and target coordinates at the delayed post-knockback airborne event.
- Runtime unknowns: whether MoveTo reaches the calculated far-side point under TFM2 movement/collision rules; whether the non-cancelable S2 action lock allows a sufficiently fluid follow-up; whether the target remains correctly selected; and actual kick travel/direction. Review the uploaded match log and visible gameplay before changing distance.
- Knockback tuning is intentionally a later, separate candidate.


## v1.1.38 combo follow-up test

- Preserve the v1.1.35/.36/.37 candidate ZIPs untouched.
- S1 clips/action/AI lock: 48→45 ticks; third-cast delayed event: 10→9 ticks. Dash: 15,000→20,000 units. Knockback remains 6,000×10; shared recast range remains 145,000. AI first-Ult trigger is 24,000; native first-phase hard cap remains 30,000.
- Combo adds a single initial S1 retry and up to five short waits for action-charge propagation; combo events use the normal origin/match/caster log prefix. Failed stages now include stage flags/readiness in the trace.
- v1.1.37 log review: 20 combo starts, no completed `combo kick target`; only one S2-behind attempt and it cancelled (`behind=false`, 46,375 target distance). First-phase attempts beyond 30,000 were rejected. Successful knockback positions show about 53,400 units in one match-origin sample and about 49,000–60,000 in presim. These measures are not a visual calibration.
- Verify the combo sequence in a real TFM2 match before any further tuning; evaluate knockback as its own later test.


## v1.1.39 first-phase Ultimate range correction

- Runtime trace: the action data exposed 145,000 range for both Ultimate phases. The native first-phase callback rejected targets beyond 30,000 only after the action had started, explaining visible/empty distant casts.
- New data range: 20,000, modestly above the Chou basic attack range of 15,000. The native first-phase hard guard matches 20,000.
- Recast: keep the same action slot and target chase, but apply +125,000 `StableEffectType::range_adjust` only while `mlbb_chou_ult_recast` is present, for a 145,000 total. The adjustment marker is outside `SwitchByBuff` so both phase range checks can read it. Verify in a live TFM2 match that range checks traverse the marker in this combined effect.
- AI first-phase trigger: 18,000.
- User requested the first kick travel farther: test 10,000 speed × 10 ticks = 100,000 units. Keep the 145,000 recast reach so it can chase the displaced target.
- This is an experimental phase-specific range and knockback implementation. Do not call it confirmed until live target-selection testing establishes: first cast fails beyond 20,000; recast succeeds around 60,000–100,000; recast fails beyond 145,000; and first-phase knockback still occurs on close casts.


## v1.1.40 conservative Chou damage test

- Changed only Jeet Kune Do and the two damaging Ultimate phases: S1 100–165 + 45% total attack, first Ultimate 275–420 + 110% extra attack, recast 330–500 + 132% extra attack. The progression intentionally lifts low-level base values by about 10%, with a larger 16–20% late-level increase; it does not copy published MLBB coefficients.
- Shunpo remains non-damaging and unchanged. The 20,000 first-phase range, 145,000 recast reach, 100,000 knockback, S1/S2/Ultimate timing and combo actions are preserved.
- Added sparse logs for S1 raw damage per target and first-phase Ultimate damage, plus unit tests that guard early/late damage against unintended jumps. Recast remains one damage event.
- Validation still required in live TFM2; compare match-origin damage at comparable hero levels/gear and observe the full combo’s kill pressure before tuning further.
