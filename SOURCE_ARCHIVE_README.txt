MLBB Heroes v3.1.0 contains 31 heroes and uses the Windows x64 build 1.1.201. It was playtested on Teamfight Manager 2 0.7.4.

The Rust crate is in sdk/candidate and the matching stable SDK is in sdk/mod-api-stable. Run cargo test --locked --manifest-path sdk/candidate/Cargo.toml.

Install using mlbb_heroes_v3.1.0.zip. See docs/XBORG_PLAYTEST.md for the X.Borg changes and live test steps.

Original X.Borg sheets and their hashes are in source/xborg. Rebuild the atlases, Firaga gauge and preview frames with python tools/compile_xborg.py (Pillow). Rebuild the official skill icons and default English voice events with python tools/compile_sun_media.py xborg (Pillow, FFmpeg and ffprobe). Original media and provenance are in source/xborg/media.

Original Balmond sprite sheets are in source/balmond/visuals. Rebuild the Balmond atlas with python tools/compile_balmond_visuals.py (Pillow).

Original Aldous sprite sheets are in source/aldous/visuals. Rebuild the Aldous champion atlas with python tools/compile_aldous_visuals.py.

Original Sun and Fanny PNGs are in source/sun/visuals and source/fanny/visuals. Rebuild their atlases with python tools/compile_sun_visuals.py and python tools/compile_fanny_visuals.py. The compilers use Pillow.

Sun’s approved source icons and voice lines are in source/sun/media. Rebuild the runtime PNGs and voice events with python tools/compile_sun_media.py. The media compiler uses Pillow, FFmpeg, and ffprobe.

Lesley and Valir voice sources and attribution are in source/lesley/media and source/valir/media. Rebuild their voice events with python tools/compile_sun_media.py lesley valir.
