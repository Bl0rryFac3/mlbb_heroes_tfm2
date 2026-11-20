MLBB Heroes v2.7.0 uses internal build 1.1.164 and contains 26 heroes. Requires Teamfight Manager 2 0.7.3 or newer.

The canonical Rust crate is in sdk/candidate. The stable SDK used for this build is beside it in sdk/mod-api-stable. From sdk/candidate, run cargo test --offline --locked. The bundled Windows x64 DLL was built using the x86_64-pc-windows-gnu target. See the build report and project context under docs.

The Atlas implementation is in sdk/candidate/atlas.rs and atlas_visual.rs. Original Atlas animation art is in source/atlas/animations. Source icon and audio references are in source/atlas/media. The ban/pick preview is in sdk/candidate/draft_preview.rs.

Release notes, the build report, publication checklist and earlier playtest reports are under docs. To install the mod, use the separate mlbb_heroes_v2.7.0.zip.
