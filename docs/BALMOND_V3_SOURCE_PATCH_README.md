# Balmond v3 incremental source patch

This revision follows the approved hybrid Ultimate recommendation. It is **not** a full source-with-SDK archive and is not a runtime ZIP.

## Apply order

1. Start from the preserved full v1 source-with-SDK archive in Drive.
2. Ensure the v1 Balmond candidate patch is applied.
3. Apply `patches/balmond_candidate_v2_incremental.patch` with `patch -p1`.
4. Apply `patches/balmond_candidate_v3_incremental.patch` with `patch -p1`.
5. Run `python tools/audit_balmond_v3.py`, then the complete project's Rust tests, Windows release build, and source/runtime package audits.

V3 changes the Ultimate from a fully native cone scan to a data-driven `RangeEffect`/`DirDot` fan whose per-target Native child computes only missing-health damage and the non-hero cap. V3 carries forward v2's Direction + RushTime Skill 1. Other kit effects and supplied assets are retained.

`DirDot.range = 707` is an inferred encoding for the existing 45-degree-half-angle dot threshold. Public schema docs omit the units, so its actual width is **not yet verified in-game**. Nested per-target Native delivery must also be verified in a runtime match.

The v1 runtime ZIP remains v1. No v3 runtime DLL/ZIP, full project build, or TFM2 0.7.0 match test exists yet. Do not label the old runtime as v3 and do not put source/SDK files in a runtime ZIP.
