# MLBB Heroes 1.1.24 — Linux x86-64 candidate

This candidate retains the v1.1.21 passive-mark atlas repair and v1.1.22 Karrie rich-text localization. It adds a lingering Skill 1 field: after the original impact, the matching violet ring remains for 1.5 seconds, refreshes a 35% slow, and deals two modest pulses of 10 + 10% AD physical damage at 0.5-second intervals. The pulses do not add extra Lightwheel Marks. Karrie’s Ultimate cooldown remains 20 seconds; its six-second self-buff now grants +20% move speed and +30% attack speed.

Audio routing now covers all bundled voice/SFX clips for Fanny, Lancelot, Estes, Aldous, and Gusion, using distinct event/clip IDs with explicit override mappings. Native passive hooks trigger available selection/idle and movement lines, plus available death clips. Karrie Hawkwatch selection/idle, eight movement lines, two death clips, Skill 1, Skill 2, and both sequential Ultimate lines are included. No replacement recordings were fabricated: the source set lacks Fanny Skill 1/death, Lancelot death, Aldous basic/Skill 1/Skill 2/death, and Gusion selection/movement/death voices. The selection line plays on spawn rather than in the pre-match hero-select screen; passive-based idle/run timing is heuristic and not runtime-verified.

## Install

Extract the `mlbb_heroes` folder into the Teamfight Manager 2 `mods/` directory, then enable **MLBB Heroes** in the in-game Mod Manager. Disable older standalone copies of the same heroes to avoid duplicate champion IDs.

## Build and validation status

- Linux `mlbb_heroes.so` built from the included Karrie-enabled Rust source.
- `cargo test --release`: 21 tests passed; Rust formatting check passed.
- Static champion JSON, rich-text markup, audio event/clip routing, asset references, passive-mark and slow-zone frame checks, and ZIP CRC checks passed.
- **This package has not been launched in Teamfight Manager 2 or tested in a match.** Skill 1 zone behavior and voice playback remain in-game unverified.
- The Windows `mlbb_heroes.dll` is not included in this Linux ZIP; use the separately built Windows x64 candidate on Windows.

Karrie's values are TFM2 balance choices, not verified live-server values. See `source/karrie/ASSET_IMPORT.md` in the project source for implementation notes; that development file is not part of this runtime ZIP.
