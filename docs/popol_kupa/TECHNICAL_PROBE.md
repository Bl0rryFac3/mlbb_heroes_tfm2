# Popol & Kupa — 1.1.135 technical probe

This candidate tests companion and cosmetic mount plumbing on the owner-confirmed 1.1.134 baseline. It is not a finished hero release. The owner approved the three-slot adaptation and asked to proceed; all proposed balance remains subject to real matches.

## Implemented for the test

A separately targetable Kupa summon is tracked with the project's per-simulation/copy EntityKey. Its engine attack and movement are disabled, and native ticks perform short swept movement, target selection, attack wind-up/contact checks and one damage payout. The movement test slides around local obstructions but does not provide a full long-distance pathfinder. There is a bounded recall timeout rather than an infinite stuck recall.

The render experiment uses transparent base atlases for Popol and Kupa, plus ordinary named visual buffs carrying the supplied still sprites. During a mount, Popol's normal body layer switches to a labeled composite, and Kupa's separate body layer is removed. Both actual combat entities remain alive and targetable; no banish, invisibility, undying, damage reduction or CC immunity is applied. Popol remains the movement root; Kupa's position follows him. Dismount never moves Popol. The combined picture is standing Popol over Kupa with MOUNT TEST written underneath, not a designed seated pose.

Alpha is a timed +35% max-HP buff on the same companion entity. It heals once on activation, uses an ALPHA TEST body layer, and reverts/clamps the extra HP without respawning or emitting a fake kill. It expires correctly even while mounted. A skill-1 wind-up is distinct from ordinary companion bites; duplicate same-tick updates cannot pay twice. Defensive AI can trigger recall/mount below 85% owner HP once Skill 2 unlocks. When no enemy is nearby, ordinary lane movement passes through unchanged.

## Intentionally unfinished

The rig uses one still for every motion. Normal spear damage is immediate; a traveling spear, final hit/attack animations, proper seated rider art, a distinct Alpha silhouette and correct movement-facing variants are pending. Alpha command currently tests single-target knock-up, not its final area hit. Automatic traps, prayer/revival, cooperative stacks, out-of-combat healing, jungle bonus, full stat/item inheritance, final cooldown/AI tuning, voice clips and finished icons/translations are not implemented. The temporary icons and all eight locale entries are clearly labeled English technical-test content; existing hero translations are preserved.

This is a deliberately narrow checkpoint before animation costs. Do not use the diagnostic mounted or Alpha images as canonical animation references.

## Verification and limits

The callback suite covers spawn/disabled engine attack, contact-only commanded damage, missed contact, same-tick dedupe, ordinary-vs-command bite damage, rooted movement, bounded swept steps/wall collision, mount damage/targetability, a single owner movement root, safe dismount, CC/death cancellation, Alpha identity/one-time heal/expiry (including mid-mount), owner respawn cleanup, two owners, isolated simulation copies, recall timeout and read-only AI/baseline movement. The ABI host records operations and models basic shield/HP/buff behavior. It is not the TFM2 engine or renderer and does not establish actual mitigation, item procs, body-layer rendering, live movement acceptance or live targetability.

All prior roster tests are retained. Windows release DLL, asset/schema/packager checks and exact preservation results are recorded with this build. The real render/targetability proof is still the owner's next match test, not something a green suite substitutes for.

## Owner's short test

1. Close TFM2 and back up the installed 1.1.134 mod folder outside mods. Install only this candidate's mlbb_heroes folder. Verify the loaded banner/version is 1.1.135. Do not publish it.
2. Pick **Popol & Kupa [TECH TEST]** in a practice/test match. Confirm both normal bodies are visible and Kupa follows/attacks without teleporting or doing a flurry of duplicate hits. Blank bodies would indicate visual-buff registration/rendering has failed.
3. After Skill 2 unlocks (level 3), let Popol take some damage (below about 85% HP) with Kupa alive. The AI should recall him, then display MOUNT TEST for about 1.5 seconds while moving. A shield is expected. Incoming hits should consume the shield and/or HP; shield absorption is not invulnerability. Pause to inspect the body if needed.
4. Confirm dismount returns the ordinary pair at the same location, without Popol jumping back. Observe whether Kupa is stranded by walls or repeatedly fails recall.
5. After ultimate unlocks (level 5), Alpha should show ALPHA TEST, gain/heal HP once, and return after 9 seconds without a duplicate Kupa. If Kupa dies, revival is not available in this probe; restart to repeat the test.
6. Return the current mlbb_heroes.log and say whether normal bodies, MOUNT TEST, movement, incoming damage and dismount worked. Logs use [POPOL probe]. Keep 1.1.134 for immediate rollback.

## Animation work that can proceed now

Use POPOL_KUPA_ANIMATION_HANDOFF_v1.zip. First batch: Popol idle/run, Kupa idle/run. The padded references preserve every original pixel and align the ground line. No SpriteCook or PixelLab generation credits were spent. Hold mounted and Alpha animation work until the real render test passes and proper masters are approved.
