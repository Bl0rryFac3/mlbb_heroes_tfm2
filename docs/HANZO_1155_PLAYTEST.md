# Hanzo1.1.155 — mark/ward/trail polish

Owner reports all gameplay and skills work fantastically in1.1.154. That build is the accepted rollback. This candidate keeps its movement, recall, facing, S1/devour, projection/return and energy intact.

Replace the installed mlbb_heroes folder, keep one enabled copy, and start a fresh match.

1. **Mark lights:** Follow one target as stacks rise. The glyph should show1,2,3,4,5 illuminated symbols in that order, retaining the same head position. Previously the source frame order displayed1,5,4,2,3. The existing pixels are remapped; they were not repainted.
2. **Parked body ward:** During ult, the44px hollow ward should be centered around the body left behind. The middle is transparent and the outline is55% opacity, so the body should remain clear. Its bottom is anchored at the body's feet, centering it22 sprite pixels above them; it is no longer displaced north in map coordinates. Check both facing directions and return cleanup.
3. **S2 trail marks:** Watch an enemy remain in the horizontal mist trail. Each valid positive pulse should add one mark, up to five, without requiring an immediate HP read. Human trail has five pulses; spirit trail has four, with basic/dash contact able to supply other stacks. There must be no double stacks for a single pulse. Allies, towers/nexus and targets outside the existing trail area do not gain marks from it.
4. **Preservation:** The already-working movement/recall, attacks, S1/devour, ult/body/link/return, portraits and skill text should behave as in1.1.154.

531 automated tests pass, including delayed-damage trail marking in both forms, immediate single-pulse marking, ineligible-target exclusion and ward world placement. Static audits check the mark-state mapping,44px ward, alpha ceiling and transparent center. Actual in-game appearance still needs confirmation. If a check fails, provide the current log and the affected item; avoid changing balance while assessing these visuals.
