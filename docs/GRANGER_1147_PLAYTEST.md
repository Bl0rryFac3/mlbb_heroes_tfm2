# Granger default voice test — 1.1.147 / v2.5.0-rc.5

The owner accepted 1.1.146 art/VFX as “working fantastically”. This candidate adds voice audio; keep the accepted folder as rollback. Close TFM2 and replace the entire mod folder, then confirm 1.1.147.

Listen for:
1. A single intro opportunity on a real successfully emitted shot after the first second. The selection clip is used as an in-match intro; opening the mod manager, draft screen or loading a match must not dump voice lines.
2. Occasional movement lines after combat begins, with at least 12 simulation seconds between attempts. Standing still and basic attacks must not create repeated chatter.
3. S1's named line, and two alternating named ult lines over several casts. S1 attempts one line per burst, not six. Skill attempts have a six-second spacing; shared actor/global locks can drop a line when another hero is speaking. Missing an occasional line is expected; overlapping chatter is not.
4. Kill/death lines and a respawn line on fresh combat after respawn. These also respect the shared channel. Check Granger alongside Julian and Popol/Kupa.
5. Pause for 15 seconds and resume: no new lines should start during pause and no backlog should drain afterward. A clip already playing may finish: this SDK has no stop-sound handle. Scrubbing/reopening a replay must not replay old voice events or release a pile of lines.
6. Comfortable volume against other heroes and weapon/game sounds; no distortion or severe menu/match lag. Animation, homing, energy, damage and cooldown behavior should match accepted 1.1.146.

Send the mod log if voices are silent, overlap, or begin in menus/pause. The shared log distinguishes frame requests, gate drops, and successful/failed playback. A short clip helps assess volume.

Scope: 19 mapped default-English events (intro, nine movement lines, S1, two ult lines, three kills, one death, two respawns). No skin packs, repeated basic-attack grunts, guessed S2 grunt, invented weapon sounds or showcase music. The wiki's direct page/CDN returned HTTP 402; matching default clips came from mlbb.tools' public audio player, with wiki labels cross-checked through its indexed Default section. Original downloads and source metadata are in the matching source archive. Original automated transcriptions can contain errors; semantic event labels follow the wiki.
