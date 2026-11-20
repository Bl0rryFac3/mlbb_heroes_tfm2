# MLBB Heroes 1.1.157 — focused playtest

25 heroes; TFM2 0.7.3+. Before public v2.6.0. Close the game, back up 1.1.156 outside mods, replace the whole mlbb_heroes folder, keep one enabled copy and confirm 1.1.157. Start a fresh match.

1. Hover Popol & Kupa, Granger and Hanzo in starting selection and draft for 10 seconds each. Move the cursor away and back repeatedly. Their previews must stay visible. Check selected-player portraits and banned icons too.
2. In a match, first check ordinary walking, facing and recall for all three. No jitter, freezing, backwards walking or double body should appear. The new named animation state is intended only to select the transparent base sprite. Native engine interaction still needs this playtest; the offline fixture cannot establish it.
3. Check Popol mounting/dismounting and Kupa follow/revival. Check Hanzo human/spirit basics, S1, S2, parked body/link, marks, devour and return. Gameplay and AI files for those heroes are unchanged; the new shared base-animation override is the integration risk.
4. Granger S1 should hit harder with the same six homing rounds and energy use. At 84 Attack the pre-mitigation packet per round rises from 45 to 61; at 104 Attack it rises from 51 to 69. Rondo refill and S1 hit cooldown refunds remain. The ultimate rises from 203 to 239 at 104 Attack, with the same homing, AoE, slow and roll.
5. Optional controlled crit check: use the same target/Attack/armor and compare the sixth S1 impact with and without an item that increases critical damage. The native crit chance scope and Skill attack type remain, with no manual crit multiplier. Automated tests prove submission/scoping, not live engine item scaling. Report actual sixth-shot numbers and item names if checking this.

If any movement/attack or duplicate-sprite regression occurs, restore 1.1.156 and send the current mlbb_heroes.log plus a short clip. A statement that normal skills work does not confirm crit-item scaling. Do not publish v2.6.0 until the new preview integration is accepted.
