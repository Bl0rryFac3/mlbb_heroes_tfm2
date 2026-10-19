# Fanny effect import report

Generated from user-supplied effect PNGs by `tools/build_fanny_effects.py`.
The sources did not include AutoSprite JSON/frame ZIP, so the visible contact-sheet layouts are used.
All source images are converted to RGBA so transparent palette/background pixels remain transparent.

| Asset | Source layout | Output | Frames/tag | Duration/frame |
|---|---:|---:|---:|---:|
| `fanny_tornado` | 4×2 | 128×128 | 8 / `skill` | 0.040s |
| `fanny_cut_throat` | 3×2 | 128×128 | 6 / `ult` | 0.050s |
| `fanny_cable_impact` | 5×1 | 128×128 | 5 / `impact` | 0.040s |
| `fanny_cable_start` | 4×1 cropped launch sparks | 64×64 | 4 / `skill` | 0.030s |
| `fanny_prey_mark` | 2 side-by-side icons | 64×64 | 1 each / `mark_1`, `mark_2` | 0.450s |

Additional art staged: `fanny_cable_wire.png` is a thin cable texture for future route drawing; `fanny_hook.png` is an isolated hook head.
The cable texture is not itself a dynamic in-game trail. The client renderer still needs to place/rotate it along the live route.
The Prey Mark animation tags are short application pulses, not a persistent above-target badge.
