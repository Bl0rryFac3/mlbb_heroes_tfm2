# MLBB Heroes v2.7.0 handoff

Public label v2.7.0. Mod manager version 1.1.164. Windows x64 and Teamfight Manager 2 0.7.3 or newer. The roster has 26 heroes. The previous published tag is v2.6.0.

Atlas, his skill icons and audio, the Granger/Hanzo/Eudora tuning, and match draft idle previews are included. The owner reported that 1.1.164 worked and authorized the release handoff. This confirmation covers the reported hover bug. No live FPS measurement accompanied it.

The final preview uses `main.champions.contents` from the owner’s Steam log and resolves the hovered tile in the game’s foreground window. It reuses the tile while its geometry and identity match. The world draw map can be present during draft.

The canonical code lives in `sdk/candidate` of the matching full source ZIP, with the frozen SDK in `sdk/mod-api-stable`. See the 1.1.164 build reports and playtest notes in that archive.

[1.1.164 owner playtest record](https://app.notion.com/p/41e9c01d37c9810ead0ff6650bab284d)
