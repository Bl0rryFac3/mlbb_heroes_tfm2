# MLBB Heroes 1.1.163 draft idle preview

The owner reported that the previous balance and media changes worked and the idle animation still failed when hovering a champion during match ban/pick. The 1.1.162 Popol overlay only ran in New Game, Lineup and Stadium Entrance. This candidate adds a preview route for a visible draft layout in the match client.

The client tries to resolve the hero and the art area in the same detail pane. It can also resolve a shared champion tooltip when an active draft layout exists. It uses the shipped idle sprite sequences for Popol & Kupa, Granger, Hanzo and Atlas. The match simulation, roster, carrier animations, skill descriptions, and Atlas audio match 1.1.162. The preview uses the stable client drawing API and suppresses itself when the match-world draw map is visible.

## Verify in Steam

1. Close the game. Move the old mod folder outside `mods`. Extract the ZIP into `mods` and confirm Mods displays 1.1.163.
2. Enter a fresh match and reach the ban/pick screen. Hover Popol & Kupa, Granger, Hanzo and Atlas for a few seconds each. Check that the detail pane loops while the cursor remains on a hero. Check that its art follows a new hero and disappears when the pane closes.
3. Confirm the compact ban icons and the confirmed pick slots look normal. Start the match and check that the champions still move and attack.
4. If the idle remains absent, keep the cursor over the affected champion and press F8. Send a screenshot and the lines starting with `[draft preview]` from `mlbb_heroes.log`. The F8 entry records the screen and visible UI paths. The render entry records which draw maps the host exposed.

The build passed 554 Rust tests, with one existing ignored case, plus 104 champion-data checks, four loader tests and 192 Atlas view bindings. The actual host UI paths and draw maps still need Steam verification. Rollback for the current balance and Atlas media is 1.1.161.

[Team Samoyed stable client API](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/064c2facbd725c71a4d1fae17f4dc75c1f1ea7a0/docs/stable-api-reference.md)
