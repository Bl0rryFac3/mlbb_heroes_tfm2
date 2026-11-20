# Validation evidence and limits

## Verified baseline

Google Drive canonical source folder contained internal 1.1.122 / 21 heroes as its newest source. The existing local source archive matched SHA-256 fe308acad904a8f95bb465b21a06fe2073e9ad329e0b6da37e8aed02b4adc948. All copied baseline Rust files and Cargo files were byte-compared against that archive before adding Julian. Baseline art/audio/locale data were used as read-only references; the canonical source and delivered Eudora archives were not rewritten.

## Source verification actually performed

The module was compiled and tested with the baseline's Linux Rust 1.98.0-nightly toolchain and exact supplied SDK. `cargo check --offline` passes with the 17 inherited baseline warnings. The final combined `cargo test --offline --lib` run passes **316/316**, including 16 new Julian tests. The final source/test SHA-256 values are in VERIFICATION.json.

New tests cover: all six enhancement permutations and shared debt boundaries; duplicate skill rejection and expired partial sequences; traveling Scythe through a lane minion into a hero; Enhanced Scythe's initial hit and eight slow-phase opportunities with a following victim; same-tick update deduplication; delayed normal Chain; four enhanced pulses then one launch; move-before-banish and six Enhanced Sword path hits while banished; one enhanced magic basic attack and expiry; structure/ally exclusions; hero-only sustain without one-stack-per-pulse inflation; death and cross-simulation isolation; early locked-slot Chain assistance and read-only AI; group-combo ordering/engine input validation; ordinary baseline action fallback; shared base damage profile hints; and direct passive-death cleanup even without living-player lookup.

The reused deterministic ABI host derives from the project's Eudora fixture. It records typed damage and control calls rather than running the engine's actual resistance, vamp, movement/banish rendering, native cooldown UI or item reactions. The tests therefore establish callback logic and counts, not live damage dealt or in-game balance.

## Tool verification actually performed

- All handoff Python scripts pass syntax compilation.
- `tools/test_tools.py`: two tests pass. Native synthetic 3×3 grids compile into all eight body rows and nine VFX sheets, with expected phase/direction tags. A missing-art inbox deliberately fails.
- `apply_julian.py` was exercised on a temporary baseline metadata/source subset. All old heroes' description/name values remained unchanged across eight locales. A second application is rejected instead of overwriting a later candidate.
- The generated Julian definition passes the existing strict champion schema validator: 4 pass, 0 warn, 0 fail.
- Source integration validator confirms 333 unique view-effect names, exact native refs, eight locale structures, the mirrored SDK test-data definition and both source hooks.

## Not performed / not claimed

No Julian artwork or animation was generated, returned or visually accepted. No SpriteCook/PixelLab generation credits were used. The two supplied JPGs are downloaded visual references, not generated base sprites. No true asset-complete release validation has passed because those inputs are absent. No Windows Julian DLL or runtime ZIP was built, no actual Julian match was played, no exact in-game tooltip/cooldown-display synchronization was verified, and no multilingual native-speaker review occurred.

Synthetic compilation fixtures were temporary test data and were removed; do not ship them as character art. The handoff ZIP contains original source and compilation tools, not a fake playable runtime. The Arena.ai agent must finish art import, actual Windows compilation/loading, full preservation checks, packaging and owner playtest handoff.
