# Hanzo attack/S1 and body/link/mark repair —1.1.151

18 November2026, Asia/Manila. Replaces the faulty1.1.150 candidate. Owner confirmed transformation, spirit S2 and forced human return, but reported neither form using basic attacks/S1 and missing human body, link, marks and devour.

Close TFM2, replace the installed mlbb_heroes folder with the1.1.151 candidate, keep exactly one enabled copy and start a fresh match.1.1.149 remains the accepted rollback outside the mods directory. No release publication.

## Focus this test on four things

1. **Basic attacks:** Before level5, human Hanzo should repeatedly strike enemies. After transforming, Hanekage should also use basic attacks between skills. Look for both the attack motion and actual enemy HP loss.
2. **Marks and S1:** Marks should appear over the target and build toward five. Human S1 should bite/heal at five marks. Spirit S1 should lunge/use the maw, preferring five marks or a useful gap-close/finisher. Targets the engine considers within melee reach must not be ignored.
3. **Devour:** Watch an ordinary jungle monster after five marks. It can be devoured, followed by four healing ticks over2s. Lane minions, summons, towers and bosses are not instant-devour targets in this adaptation; a lane fight alone may not demonstrate devour.
4. **Parked human and link:** On projection, the human body should remain at the entry point with a magenta link to Hanekage. It should merge on return. These now use native view effects like the visible skill effects. Body/link effects stop refreshing immediately on exit and any last stamp lasts at most0.1s; marks last at most0.2s after consumption/expiry.

Also confirm the already working transformation, spirit S2/airborne and defeated-spirit return remain intact. Pause should freeze the simulation; no new sound or voice playback was added. Menus and fights should remain responsive.

If any item still fails, send the current mlbb_heroes.log and the item number. New capped diagnostics include BASIC (with actual_contact and center/edge distance), S1 cast and NATIVE_OVERLAY, alongside the existing ENTER/RETURN markers. A short clip helps distinguish an animation problem from missing damage.

## What changed

- Basic native effect now advertises positive expected damage to the engine and custom AI explicitly requests validated Attack inputs. Invalid higher-priority skill proposals fall through to other valid candidates instead of suppressing attacks. If the host rejects a predicted attack range, the AI can move closer.
- Removed the extra26k center-distance veto from a basic attack already accepted by the native engine. S1 reach uses collision-edge distance, with native input validation still authoritative. An accepted target is not rejected a second time by a conflicting visibility read. AI target searches retain visibility checks.
- Human S1 searches eligible marked targets independently, so an unmarked farther champion cannot hide a marked nearby camp. Spirit S1 can use a useful gap-close/finisher without deadlocking while waiting for marks that basics were never building.
- Replaced client Game-map drawing of body/link/marks with bounded native positional view effects. Per-frame body/ward states refresh at6-tick intervals; marks at12 ticks; at most16 native link segments. Unknown simulation origin follows the existing skill-VFX policy, while positively identified server presims skip overlays. No UI-tree scanner, new targetable body or gameplay render clock.
- Projection entry, stored-health restoration/penalty and begin-return functions are identical to1.1.150. S2 damage, timing, airborne, energy refund and mist geometry are retained. Existing owner art,48px filtering, official icons and colored descriptions remain.

505 Rust tests pass,41 Hanzo tests total including13 new repair tests through the AI ABI and native-effect dispatch; one inherited test ignored.4 loader tests and100 schema checks pass. These local tests do not replace actual game-engine rendering/AI confirmation. No new paid art or audio.
