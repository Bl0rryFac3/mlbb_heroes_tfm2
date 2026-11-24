MLBB Heroes 1.1.188 contains 30 heroes and requires Teamfight Manager 2 0.7.3 or newer. The bundled DLL targets Windows x64.

The Rust crate is in sdk/candidate and the matching stable SDK is in sdk/mod-api-stable. Run cargo test --locked --manifest-path sdk/candidate/Cargo.toml.

Install using mlbb_heroes_v3.0.1.zip. See RELEASE_README.md for Sun's playtest stats and the Fanny AI changes.

Original Sun and Fanny PNGs are in source/sun/visuals and source/fanny/visuals. Rebuild their atlases with python tools/compile_sun_visuals.py and python tools/compile_fanny_visuals.py. The compilers use Pillow.

Sun’s approved source icons and voice lines are in source/sun/media. Rebuild the runtime PNGs and voice events with python tools/compile_sun_media.py. The media compiler uses Pillow, FFmpeg, and ffprobe.

Lesley and Valir voice sources and attribution are in source/lesley/media and source/valir/media. Rebuild their voice events with python tools/compile_sun_media.py lesley valir.
