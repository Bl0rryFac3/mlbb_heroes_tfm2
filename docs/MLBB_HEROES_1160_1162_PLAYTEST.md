# MLBB Heroes Windows playtest for builds 1.1.160 through 1.1.162

Teamfight Manager 2 0.7.3 or newer. Build 1.1.159 is the owner-confirmed Atlas rollback. Keep one installed copy of the mod. Close the game, move the current mod folder outside `mods`, then extract the chosen ZIP directly into `mods`. The root should be `mods/mlbb_heroes/mod.mod_info`.

## 1.1.160 balance

- Granger. Rhapsody rises from 28 + 40% Attack to 36 + 45% Attack per round. Death Sonata rises from 130 + 105% to 155 + 110%. Check the sixth burst round with and without a crit-damage item, then watch what happens after first blood and when the native AI initiates recall near a valid target. The AI now preserves native Return commands. Walking to base and channeling recall still need observation in Steam.
- Hanzo. At five marks, Demon Feast hits champions for 200 + 7 per level after level 1 + 115% bonus Attack. In spirit form, Soul Snatch starts at 135 + 6 per level after level 1 + 95% bonus Attack. Each mark adds 24% versus champions and 20% versus other targets. The existing five-mark Max Health hit still has its epic cap. Compare a squishy hero at zero and five marks, then repeat on a tank. At level 5 with no bonus Attack and a 1,000 HP target, the five-mark strike delivers 409 raw physical damage before defenses. Verify 70% spirit Max Health, normal/recast recovery of 26 seconds and defeat recovery of 52 seconds.
- Eudora. Forked Lightning's initial hero hit becomes 120 + 65% AP. At 96 AP it rises from 152 to 182 raw magic damage. Test against a champion, a lane minion and a pre-marked target. Check that the four tether pulses, final hit and once-per-cast cooldown refund still occur. Thunder's Wrath becomes 260 + 110% AP at the center, 130 + 55% AP in the outer ring and 160 + 72% AP in the marked follow-up. A marked center target at 96 AP takes 594 raw magic damage from those two hits.
- Atlas. Check 10 seconds for Perfect Match and 34 seconds for Fatal Links before cooldown reduction. Annihilate remains at 7 seconds. Recheck normal reunion, early reunion and the delayed pull/slam on multiple champions.

## 1.1.161 Atlas icons and audio

This build includes the 1.1.160 balance changes. Verify the three new icons in the draft and the tooltips. Annihilate has a default English cast line. Perfect Match has distinct original sounds for ejection and reassembly. Fatal Links requests its default English line when it catches at least one enemy. Test normal and early reunion, two Atlas copies, pause/resume, and replay seeking. Voice lines use the existing shared gates, so competing lines can be dropped. The mod schema exposes three active icon slots. Frigid Breath artwork is included in full source as a reference.

Atlas artwork and English cast lines come from the Mobile Legends Wiki; the rights to the original artwork and recordings remain with their owners. [Atlas abilities](https://mobile-legends.fandom.com/wiki/Atlas) · [Atlas voice index](https://mobile-legends.fandom.com/wiki/Atlas/Audio)

## 1.1.162 Popol menu preview experiment

This build includes 1.1.161. It draws a Popol & Kupa idle loop over recognized cards using the client UI. In starting champion selection, wait a second and hover for ten seconds. Check that the art loops, stays within Popol's card and does not appear on another champion after scrolling or sorting. F8 requests one fresh scan in the same supported screen. Try draft and record the screen where it works or fails. The runtime currently allows New Game, Lineup and Stadium Entrance. The actual draft screen and the Granger, Hanzo and Atlas previews still need live confirmation and follow-up. Send the `[POPOL preview]` lines in `mlbb_heroes.log` if the art is absent, misplaced or doubled. In a fresh match, confirm that Popol still leaves base and attacks.

All three are Windows playtest candidates. The build machine checks damage, data, packaging and ABI wiring. Steam is required to verify audio playback, actual recall, and the hover UI.
