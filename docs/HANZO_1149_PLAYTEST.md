# Hanzo projection test — 1.1.149

18 November 2026, Asia/Manila. First mechanics candidate, built from accepted Granger1.1.148. Static human/Hanekage master sprites; 25 registered heroes. This is a test build, not a release.

## Install

Close TFM2. Keep a backup of the accepted mlbb_heroes folder outside the game's mods directory. Extract the candidate ZIP, open its outer folder, and replace the installed mlbb_heroes folder with the contained mlbb_heroes folder. Keep exactly one enabled copy. Start TFM2, confirm version1.1.149 and enable the mod if a previous loading error left it disabled. Start a fresh match with Hanzo; old saved/replayed simulations are not a fresh mechanics test. Reverting uses the accepted folder backup.

## Observe these six things

1. **Before the match:** Hanzo appears in starting selection, the draft grid, the selected player's portrait and a banned icon. His three skill icons and colored descriptions appear.
2. **At level5:** Hanzo transforms into the distinct Hanekage sprite near an enemy champion. His human body stays at the entry point, linked to the spirit. The body is a protected visual with no combat hitbox; enemies must not stop to attack it. The small lower-left status display shows Hanekage energy and stored body HP.
3. **Normal return:** When the spirit is low on health or has no nearby enemy after two seconds, the AI can recall. Hanzo returns to the parked body's point, the extra body disappears and human HP restores from the stored body pool plus return healing. The normal internal cooldown is30s after exit.
4. **Energy depletion:** If he keeps fighting without recalling, energy reaches zero after roughly10s. The human body catches up to the spirit over1s, then merges. He must not return to the old body point for this exit. Energy recovery is outside this narrow proof, so the duration is predictable.
5. **Defeated spirit:** Let enemies burst Hanekage down. He should return to human form, receive the50% slow for6s and a60s ultimate cooldown. **Hanzo's death count and the enemy's kill count must not increase.** Later, an actual human-form death must still work. Physical and magic matchups are useful; a true-damage matchup is additional evidence if available.
6. **Walls and performance:** Hanekage can cross walls, human form lands on walkable ground on return, and neither form gets stuck. Pause freezes energy/catch-up. Menus and matches should remain responsive. The two art files are static; lack of running/attack frame animation is expected.

If one fails, note which item and whether it happened at transformation, recall, energy depletion or spirit defeat. Send the current mlbb_heroes.log; a screenshot/video is especially useful for body placement or a kill/death counter change.

## Implemented scope

Single real champion and native item/credit pipeline, protected body render snapshot, distinct static forms and facing, saved body HP, native -30% max-HP modifier in spirit, +35% movement/+25% attack speed, wall-ignore,100 energy draining10/s,15s safety cap, bounded body catch-up, short native banish return, restore/heal, failed-return penalty, owner/copy-scoped state and terminal cleanup. Basic attacks build five owner-specific marks. S1 supplies a marked human strike or spirit lunge so the pair can fight; S2 supplies short movement for traversal checks.

This pass intentionally does not certify the full planned hero: creep devour, full five-mark percent-HP payoff/penetration, mist zones/pulses/slow, spirit dash path damage/airborne, combat/death energy recovery, final motions/VFX/audio and final balance are follow-up work after the native projection proof. S1 tooltip describes the smaller strike implemented here. Eight locales currently receive neutral colored English fallback. No new Hanzo voices were added.

## Evidence limits

478 Rust tests pass, including14 Hanzo ABI/control-flow tests. The mock explicitly models an engine that clamps undying HP at1; this is not evidence that the actual engine does that or prevents its real kill/death entry. If the actual champion dies during projection, the code logs NATIVE_PROOF_FAILED, does not resurrect him, and disables further projection for that actor in that match. No live TFM2 client was available during development.

The parked body uses a native-size Game-map sprite with fixed-point position conversion and a thin link; actual coordinate/depth placement is part of this playtest. Master pixels are padded/mirrored, never resized. Status timers advance only with simulation ticks. Existing Julian voice gates and Granger extension are chained, not replaced.

Log markers: [HANZO proof] SPAWN, ENTER, RETURN_BEGIN, RETURN_COMPLETE, BODY_CATCHUP_BEGIN, NATIVE_PROOF_FAILED. Logs are event-based and capped per actor, rather than emitted every tick.
