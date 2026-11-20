# MLBB Heroes — Popol automatic trap candidate / internal 1.1.141

Built on owner-confirmed 1.1.140. This adds the approved automatic-trap adaptation using the five supplied trap assets. Existing spear/Alpha behavior, native UI preview, direct Julian audio extension and performance structure are retained. No UI tree scanning reintroduced.

## Trap behavior — TFM2 proposals
During recent combat, with living Kupa and a visible valid opponent nearby, Popol places one legal ground trap. The automatic placement replaces MLBB's fourth active within TFM2's three-skill layout; it is not manual trap control.
- 10-second placement cooldown; at most 2 waiting traps; minimum 0.5-second arming; 15-second untriggered lifetime.
- Placement pauses during casts, recall, riding, hard movement CC and Kupa recovery. Visible-opponent positions only; no targeting through fog. Failed/blocked placement retries at most twice per simulation second without consuming the cooldown.
- Each armed trap chooses one valid enemy within 12000 world units, deals 25 +20% current AD physical/Skill damage once and roots for 36 ticks/0.6 seconds. Allies, towers/nexuses, dead and untargetable units do not trigger it.
- Trigger leaves a 20000-radius frost field for 3 seconds, slowing 20% and reducing physical defense 12% while inside. Enemy minions and other valid enemy units are eligible. Same-owner fields use one modifier, not stacking copies. Separate owners maintain independent fields/modifiers.
- Maximum 2 fields per owner. Ground checks refresh every 8 ticks; arming/entry/exit can therefore be observed up to about 0.13 seconds after the boundary. Expiry/owner death clear tracked modifiers. An existing trap triggering counts as combat and interrupts prayer.
These are the approved mod's scaled numbers, not independently certified live MLBB coefficients. Cooperative passive stacks, complete item inheritance and final balance/audio remain unfinished.

## Art and performance
All five original PNGs are retained unchanged under source/popol_kupa/traps_received. Original 96px cells filled most of their canvases, so the compiler makes nearest-neighbor runtime derivatives: 32px cells for trap/trigger, 48px for frost. Anchors remain centered; all nine frames retained. The supplied trigger returns to a trap-like last pose, so it is played once and removed, never looped as a fresh trap.
No summon entities or render-time damage. At most 2 waiting traps +2 fields; inactive traps perform no entity scan. Active ground logic runs every 8 simulation ticks. Headless ServerPresim computes effects but emits no VFX. World visual frames use short 8-tick leases instead of long unremovable loops; after cleanup the last purely visual frame may linger up to about 0.13 seconds. Original animations, shaders and UI are unchanged.
Rebuild order: compile_popol_animations.py, compile_popol_traps.py, compile_popol_shared_preview.py. Never run the historical diagnostic compiler as the final carrier build.

## Install and focused test
Close game, back up 1.1.140 outside mods, replace the mlbb_heroes folder, keep one installed copy, verify 1.1.141.
1. During fighting, watch for small fang-shaped traps appearing automatically near a visible opponent after Popol finishes an action. No extra button is needed.
2. An enemy stepping onto an armed trap should take one hit and briefly stop, then a frost ring remains for about 3 seconds. It must not repeatedly deal trap damage.
3. Enemies leaving the frost should regain normal movement/defense; allies should be unaffected. Traps should disappear after expiry/owner death, and placement should pause during mount/prayer.
4. Confirm menus and normal/fast match playback remain smooth, icons stay visible, and no duplicate mounted body returns. Stop and roll back to 1.1.140 if performance regresses.
If a check fails, return the CURRENT mlbb_heroes.log and describe the event. [POPOL traps] viewer traces show placement/arming/trigger/frost times.

## Verification
412 active Rust tests pass, 0 fail; one ignored manual performance probe run separately. Tests cover arming, one-hit/root, target filters, frost expiry/exit, same-owner nonstacking, owner death, placement cap/cooldown/action gates, fog visibility, separate owners/sim copies, zero inactive entity scans and bounded viewer requests with no presim VFX. The previous combat and performance tests remain. Windows and package checks are recorded in reports. No actual TFM2 launch here; owner trap/performance verification pending. Keep 1.1.140 as the accepted rollback. No new art is requested.
