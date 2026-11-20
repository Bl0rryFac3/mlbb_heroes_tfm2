# Granger loading fix + damage buff — 1.1.148 / v2.5.0-rc.6

1. Close TFM2. Back up the existing folder, then replace the entire mlbb_heroes folder with this candidate. Do not merge old/new files.
2. The 1.1.147 failure caused TFM2 to SAVE the mod as disabled. Re-enable MLBB Heroes in the mod manager after replacement, and restart if requested. Confirm internal version 1.1.148.
3. Confirm the missing-field-type loading error is gone. If loading still fails, send the first loading error and the mod log.

Damage changes (physical, before armor):
- S1 per round: 12 + 18% AD -> 20 + 30% AD. At level 5 without items (104 AD), six rounds total 180 -> 306 before the sixth-round critical effect.
- Ult per impact/AoE victim: 70 + 65% AD -> 110 + 90% AD. At 104 AD: 137 -> 203.
- Basic attacks, energy costs/regen/growth, six-shot timing, cooldowns, Rondo, homing, slow and critical handling retain their previous behavior. This is a first balance candidate, not a proven final balance.

Observe in a few comparable matches:
- S1 should remove a noticeably larger amount of enemy health when all six shots land. Check its sixth shot still looks different and resources drain normally.
- Ult should hit harder, including nearby enemies, while remaining one impact per victim. No damage should occur before impact.
- Check performance and accepted animations/portraits still work.
- This is also the first usable test of the 19 default voice events from 1.1.147: intro on fresh combat, sparse movement, S1, alternating ult, kills/death/respawn. Check volume alongside Julian/Popol and no new menu/pause voice starts or backlog after resuming. A currently playing clip may finish after pause because the SDK has no stop-sound handle.

Report whether loading succeeds, whether S1/ult now feel useful or excessive, and whether voices are audible/comfortable. A post-match damage screenshot and mod log help guide the next balance adjustment. Owner publishes releases.
