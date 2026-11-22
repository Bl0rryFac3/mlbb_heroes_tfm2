MLBB Heroes v2.10.0 contains 29 heroes and uses build 1.1.178. Requires Teamfight Manager 2 0.7.3 or newer.

The Rust crate is in sdk/candidate and the matching stable SDK is in sdk/mod-api-stable. Run cargo test --locked --manifest-path sdk/candidate/Cargo.toml. The bundled DLL targets Windows x64.

Valir's Rust implementation is in sdk/candidate/valir.rs. The original sheets and icons are under source/valir. Run python tools/compile_valir.py with Pillow to regenerate his runtime sprites, VFX, icon PNGs, and champion data. Install the mod using mlbb_heroes_v2.10.0.zip.
