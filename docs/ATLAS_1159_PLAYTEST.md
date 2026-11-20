# Atlas Windows playtest · build 1.1.159

Requires Teamfight Manager 2 0.7.3 or newer. The ZIP opens directly to `mlbb_heroes`. Close the game and back up the existing folder outside `mods` before extracting the candidate.

1. Confirm build 1.1.159 in Mods. In a fresh draft, find Atlas under Tank. Verify his portrait and three skill icons. Hover for ten seconds to check the layered sprite preview.
2. Confirm he leaves base normally, walks, lands basic attacks and recalls. Check both facings and an idle-to-run transition. The pilot keeps the same health pool through ejection and reunion.
3. Watch Frigid Breath around a moving Atlas. Verify slow, departure from the aura, and a 0.5-second freeze only after continuous exposure. Verify the freeze cannot immediately repeat.
4. Watch Annihilate on normal and ejected casts. Each cast pulses three times. The pilot and suit each act as an origin while separated. The overlap should pay one stronger hit per pulse. Check the precise Annihilate ice overlay in the docked slam.
5. Watch Perfect Match as Atlas pursues a champion. The suit should follow his route, then reunite once. Test a normal reunion, Fatal Links during ejection and death while separated. Normal reunion stuns. Early reunion shows the separate slow ring and applies slow in place of the hit and stun.
6. Test Fatal Links on a solo target with nearby allies, then on multiple enemies. Check the windup, chain positions, pull and slam. An interrupted windup or a failed pull should produce no slam for that target. Try an immune target, retreat past the leash and a fight by a wall.
7. Confirm existing Hanzo, Granger and Popol & Kupa still leave base, attack and use their form or mount transitions.

The toolchains validate data and native registration. Steam is required to validate the actual renderer, pathfinding, recall channel, damage reduction and displacement. This first build uses a caster-relative native pull. It also uses short-lived visual effects for the separate suit. The stable SDK has no dedicated recall-channel hook, so the integration leaves native recall input alone and discards pending local actions when it sees abrupt relocation. Slow-only immunity remains deferred. Atlas tooltips use English in all eight locales. The supplied art includes baked ejection frames, so two alternate pilot poses are retained in the source and ready for future binding. Atlas audio assets are pending.
