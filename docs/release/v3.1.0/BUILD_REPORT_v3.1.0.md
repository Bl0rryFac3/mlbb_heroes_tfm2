# MLBB Heroes v3.1.0 build report

| | |
| --- | --- |
| Internal build | 1.1.201 |
| Roster | 31 heroes |
| Platform | Windows x64 |
| Playtest | Owner confirmed 1.1.201 in TFM2 0.7.4 |
| Prior public release | v3.0.1 |

The release reuses the owner-tested 1.1.201 DLL. The changes from the playtest package are the public README, install guide, and Mods menu description. The full source archive also has the release documents.

## Verification

- The accepted build passed 597 Rust tests with one inherited case ignored. The tests covered basic attacks on structures, S1 Firaga recovery and the ultimate explosion event.
- The accepted DLL is Windows x64 and exports both stable module entry points.
- Four original skill images and 29 default English voice clips were verified during the build. The originals, sources, and hashes are recorded in the asset manifest.
- Both release archives passed CRC checks. All 2,404 install files match the source archive, with the bundled README stored as `RELEASE_README.md` in source.
- The release archives differ from the accepted playtest runtime only in its three documentation and metadata files.

The owner reported the build working as a whole. The report carries forward the automated results and the owner's general confirmation.
