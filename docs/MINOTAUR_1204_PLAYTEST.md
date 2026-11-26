# Minotaur 1.1.204 · Windows playtest

The owner reported that 1.1.203 works in game. This follow-up comes from that build and targets Teamfight Manager 2 0.7.4. Use 1.1.203 as the rollback.

## Landing and balance

Despair Stomp still resolves on tick 24 using the legal landing point. The supplied rock burst remains behind Minotaur. An eight-frame gold shockwave now plays above it for 30 ticks. The Enraged wave has a larger radius. The foreground effect is independent of the landing's hit count and is not emitted on an interrupted leap. Normal and Enraged damage radii remain 22,000 and 28,000 world units.

| Skill | 1.1.203 | 1.1.204 |
| --- | --- | --- |
| S2 each recipient | 110 + 40% AP | 220 + 60% AP |
| S2 extra self heal | 6% missing HP | 8% missing HP |
| R opening slams, each | 44 + 30% AD physical | 65 + 40% AD physical |
| R final slam | 62 + 40% AD true | 100 + 55% AD true |
| Enraged | 540 ticks from activation | 720 ticks from activation |

At level 6 with stock stats and no items, the ally heal rises from 130 to 250. Minotaur's heal at half health rises from 176 to 312. Three ult hits rise from 267 to 389 raw damage before mitigation. A target needs to remain in the 36,000 radius for all three hits to receive that total. The 18 / 48 / 78 pulse ticks and 84-tick cast immunity continue to use the original timing.

The heal and 12-second Enraged window match the current Minotaur entry in MLBB Knowledge and the MLBB Analytics skill sheet. The damage uses the mod's closer-range and stat scale. The same shared formulas drive runtime and AI estimates.

## Icons and audio

The four official ability images are installed as 100 × 100 PNGs. The source manifest records Fandom's CDN for Rage Incarnate, Despair Stomp, and Minoan Fury, and the MLBB Knowledge mirror for Motivation Roar. `source/minotaur/media/SOURCES.json` records all original sources and hashes.

Exactly four original English takes are compiled to mono PCM WAVs. S1, S2, and R use the source-listed default skill takes. The default voice listing has no dedicated basic grunt. The basic uses the shorter alternate default effort take, `Minotaur.projnext.skill03.ogg`, as its own cue. Each event resolves to one WAV. The three skills use the shared priority voice policy. The basic uses the existing routine voice gate. The ult requests one voice at activation and its three slam effects do not trigger additional voices.

## Live checks

1. Compare the normal and Enraged landing waves in an area with nearby units. The bright ring expands from the landing spot and the body remains readable. Check an interrupted jump.
2. Check the larger S2 heal on a wounded ally and then on several wounded allies during Enraged. Try one standard heal-reduction case.
3. Compare the three R damage numbers on the same target. Verify the final true-damage hit and the 12-second state, including a late S2 near expiry.
4. Check each of the four official icons. Listen for one distinct clip per cue when the shared voice channel is free. Check that the ult voice does not play again on each slam.
