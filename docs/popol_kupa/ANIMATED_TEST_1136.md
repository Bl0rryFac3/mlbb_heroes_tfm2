# MLBB Heroes — Popol & Kupa animated candidate / internal 1.1.136

This is a test candidate, not a completed hero release. The accepted rollback remains 1.1.134. Existing heroes and Julian audio behavior are preserved.

## Install and test
Close TFM2. Back up the installed mlbb_heroes folder outside mods, replace it with this archive's mlbb_heroes folder, and keep only one installed copy. Verify internal version 1.1.136. Choose Popol & Kupa [ANIM TEST].

1. At match start, watch Kupa leave the nexus and follow Popol without needing Popol to return. Record any stall and the location.
2. After level 3 with Popol below about 85% HP and Kupa alive, observe recall, translucent shield, mounted movement and dismount. Popol must still take shield/HP damage. Hard CC or companion death must immediately restore separate poses. Ordinary dismount must leave him at the current position.
3. If Kupa dies, observe Popol wait until 3 seconds out of combat, then pray in place for 3 seconds and spawn a fresh companion. Moving or incoming damage interrupts the prayer; repeat revival has a 30-second cooldown. These durations are TFM2 adaptation choices, not verified MLBB coefficients.
4. At level 5, observe Alpha transformation/movement/bite and mounted Alpha art; Alpha expires after 9 seconds on the same living Kupa.

Return the current mlbb_heroes.log plus observations/video if anything looks wrong. No need for further artwork before this test.

## Changed
Supplied character/mount/Alpha animations and recall, prayer, revival and shield effects are integrated. Wider mount-start canvases are retained. Left-facing variants mirror each frame individually. Shield centers use 18% opacity, luminous rims/sparks 60%, darker rim pixels 35%; original RGB and all frames remain unchanged.

Navigation now plans bounded routes around walls and can leave an obstructed spawn edge. Popol waits during recall; an already committed basic callback does not cancel the return. Added interruptible prayer/fresh companion recovery. These pass automated tests but require actual host verification.

## Limits
Traps, cooperative stacks, complete item inheritance, Alpha AoE knock-up and final audio remain unfinished. Alpha hit falls back to its idle pose; normal Kupa death art represents companion defeat. Animation timing and ground alignment need live review, especially the wider mount-start scenes. The spear is cosmetic: its damage still occurs on cast, not projectile arrival. No voice lines were added. This package has not been launched in TFM2 here.
