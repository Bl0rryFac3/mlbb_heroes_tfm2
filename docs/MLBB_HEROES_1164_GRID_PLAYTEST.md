# MLBB Heroes 1.1.164 draft grid playtest

The Steam screenshot shows the missing sprite inside the selected or hovered tile in the champion pool grid. The 1.1.163 log identifies the screen as InGame / Match, with the pool at `main.champions.contents`. The game exposes its world draw map even in draft. That veto and the previous search for a separate preview pane both prevented a draw.

Build 1.1.164 uses the observed grid and draft header, reads the cursor inside the game's foreground client, and looks for the tile's champion ID or caption. It draws the existing idle loop only for the hovered Popol & Kupa, Hanzo, Granger, or Atlas tile. It reuses the current tile while the identity and geometry still match. The scan and rendering are scoped to this draft grid.

## Steam check

1. Confirm that Mods shows 1.1.164. Enter match ban/pick and hover Atlas, Granger, Hanzo, and Popol & Kupa, both on the artwork and over the name.
2. Confirm the sprite loops inside each tile, the adjacent cards and labels stay clear, and scrolling or changing the filter moves the preview to the right tile.
3. If a tile is still empty, hover its artwork and press F8 once. Send the `[draft grid]` lines from `mlbb_heroes.log`. The lines report whether the host exposes the tile nodes, which target resolved, and whether it accepted drawing.

The screenshots and 1.1.163 log identify the correct grid. The automated cases model its layout and test tile identity, scroll clipping, and looping. Actual tile exposure, mouse mapping, rendering, and performance still require the Steam check.
