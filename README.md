# MLBB Heroes for Teamfight Manager 2

MLBB Heroes v2.7.0 adds 26 heroes, including Atlas. The Mods menu shows build 1.1.164. The Windows build requires Teamfight Manager 2 0.7.3 or newer.

Download `mlbb_heroes_v2.7.0.zip` from the release page and extract it into the game's `mods` folder. The [install guide](Install_Guide.html) includes screenshots. [Windows release](https://github.com/Bl0rryFac3/mlbb_heroes_tfm2/releases/tag/v2.7.0)

## Source

The native Rust crate is in [sdk/candidate](sdk/candidate), with the matching stable SDK in [sdk/mod-api-stable](sdk/mod-api-stable). The release data and runtime assets are at the repository root. Original mod art and compiler inputs are under [source](source).

Run `cargo test --locked --manifest-path sdk/candidate/Cargo.toml` to test. On Windows, build with `sdk/candidate/build.ps1`. See the [source archive guide](SOURCE_ARCHIVE_README.txt) and [asset credits](sdk/candidate/ASSET_ATTRIBUTION.md).

[MLBB Champ Info companion mod](https://github.com/Bl0rryFac3/mlbb_champ_info_tfm2)
