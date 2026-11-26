# MLBB Heroes v3.2.0 build report

| Item | Value |
| --- | --- |
| Internal build | 1.1.204 |
| Roster | 32 heroes |
| Platform | Windows x64 |
| Playtest | Owner confirmed 1.1.204 on TFM2 0.7.4 |
| Previous public release | v3.1.0 |

The release uses the approved 1.1.204 DLL. The install package updates the public README, screenshot guide, and Mods menu description. The full source and SDK archive also includes the release notes and handoff documents.

- 607 Rust tests passed with one inherited ignored test. The Minotaur tests cover the landing coordinates, stronger heal, separated ult impacts, and 12-second Enraged boundary.
- The Windows DLL exports both stable module entry points.
- The four Minotaur icon images and four single-take voice routes were verified. The compact roster bank has 140 voice WAVs with zero redundant takes.
- Both ZIPs passed CRC checks. The release installer entries match the full source archive. The manifest records the source commit and checksums.
