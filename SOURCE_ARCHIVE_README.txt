MLBB Heroes v3.2.0 contains 32 heroes and uses Windows x64 build 1.1.204, playtested on Teamfight Manager 2 0.7.4. Install with mlbb_heroes_v3.2.0.zip.

The Rust crate is in sdk/candidate and the matching stable SDK is in sdk/mod-api-stable. Run cargo test --locked --manifest-path sdk/candidate/Cargo.toml.

Minotaur's original sprites, voice takes, skill icons, and source hashes are under source/minotaur. Run python tools/compile_minotaur.py with Pillow and FFmpeg to rebuild the atlases, icons, and voice events. See docs/MINOTAUR_1204_PLAYTEST.md for the tuning and in-game checks.

The compact bank keeps a single recording per retained cue. Run python tools/compact_voice_bank.py to audit the WAVs and sound event routes. If rebuilding other heroes from original media, reapply the compact bank before packaging.

Release notes and the build report are in docs/release/v3.2.0.
