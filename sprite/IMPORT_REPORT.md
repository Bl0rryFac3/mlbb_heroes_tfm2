# Fanny sprite import report

Generated from the approved AutoSprite PNG sheets by `tools/build_fanny_sprite_sheet.py`.
Frames are read left-to-right, top-to-bottom; fully transparent trailing cells are skipped.
No AutoSprite JSON/individual-frame ZIP was included, so row-major order is assumed.

- Source grid cells: 128×128
- Output frame canvas: 72×72
- Uniform cell transform: nearest-neighbour 128→68, offset (2, -3)
- Transparent pixels preserved; indexed `dead_fanny.png` transparency is converted to RGBA.
- Animation frame rectangles are pixel coordinates in `fanny#sheet.png`.

| Tag | Source | Source grid | Frames used | Seconds/frame | Total seconds |
|---|---|---:|---:|---:|---:|
| `idle` | `idle_fanny.png` | 4×4 | 14 | 0.100000 | 1.400 |
| `run` | `run_fanny.png` | 4×2 | 8 | 0.060000 | 0.480 |
| `attack` | `attack_fanny.png` | 4×4 | 15 | 0.022222 | 0.333 |
| `skill` | `skill_fanny.png` | 4×4 | 15 | 0.020000 | 0.300 |
| `skill2` | `skill2_fanny.png` | 5×5 | 23 | 0.013043 | 0.300 |
| `ult` | `ultimate_fanny.png` | 4×4 | 15 | 0.035556 | 0.533 |
| `hit` | `hit_fanny.png` | 4×4 | 15 | 0.025000 | 0.375 |
| `dead` | `dead_fanny.png` | 4×4 | 16 | 0.080000 | 1.280 |
