# MLBB Heroes — Popol selection preview candidate / internal 1.1.138

The owner reports 1.1.137 is working well in matches. Screenshots confirm the remaining defect: a blank Popol/Kupa sprite in both the starting-champion grid and ban/pick pool.

## Scope
Only the UI preview reader changed. All gameplay, routing, damage, mounting/recovery, body animation, champion definitions, sprite/effect pixels and audio code remain byte-identical to 1.1.137. Transparent gameplay carriers remain in place so the visible body layers do not duplicate Popol in matches.

The 1.1.137 overlay was restricted to NewGame/DatabaseEdit scenes and only inspected label widgets, even though selectable widgets expose their own caption state. It also searched only ancestors of a matching label, overlooking a caption owned by the card itself. These source limitations are fixed; whether they fully explain the owner's live UI remains unverified without the current log.

The replacement recognizes visible, correctly sized Popol cards from selectable/color-selectable caption state, label children or champion-specific template IDs. It works across scene kinds, including management draft. Before drawing it rechecks identity so recycled card paths after sorting/filtering cannot receive another champion's image. It clips to scroll containers and keeps the preview above the caption. Julian's existing client/audio callbacks are delegated unchanged.

## Install and check — menus only
Close TFM2, back up the current 1.1.137 mod outside mods, replace it with this archive's mlbb_heroes folder, and keep only one installed copy. Verify version 1.1.138.
1. Open the starting-champion pool and wait about one second. Is Popol visible? Change category/scroll away and return; the image must remain on his card only.
2. Open the ban/pick pool. Is Popol visible there too? Search/filter/sort the list and confirm the image follows the correct card.
No repeat full gameplay test is requested for this UI-only change. Keep 1.1.137 as the working gameplay rollback. The overall Popol kit remains unfinished.

## Validation and limitations
384 Rust tests pass, including simulated selectable captions without label children, label-child cards, hidden pools, recycled list identities, custom item IDs and small draft-card geometry. This is not a live game UI test. Windows build and archive verification logs are included. If still blank, return the current mlbb_heroes.log with the screenshot; [POPOL menu] v138 reports the scene/search, matching card paths and draw submission result. No current owner game log was supplied before this build.
