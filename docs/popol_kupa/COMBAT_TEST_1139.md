# MLBB Heroes — Popol spear/Alpha combat candidate / internal 1.1.139

Baseline: owner-working 1.1.137 match behavior plus the 1.1.138 starting-selection preview confirmed by screenshot. This build implements the authorized next two combat changes. Original art, translucent shields, selection handling, routing, mounting, recovery and existing heroes are preserved.

## Implemented
- Basic spear: no cast-time damage; a real homing projectile applies 100% cast-time AD at arrival through physical/BaseAttack damage handling.
- S1 spear: no cast-time damage; its projectile applies 50 +50% cast-time AD at arrival through physical/Skill damage handling. The commanded Kupa bite remains a separate contact event.
- Each outstanding spear has a separate hit receipt/native callback slot. It consumes only its own receipt; duplicates do not consume another in-flight spear, including when hits arrive out of order. Target copy identity and simulation ownership are checked. Same-frame duplicate basics cannot launch twice. Failed spawns do not fall back to instant damage.
- Alpha S1 replaces the single-target commanded bite with one physical/Skill area payoff centered on Kupa at bite contact: 90 +75% owner AD, 18000-world-unit radius (about 18 displayed range units), 36-tick/0.6-second airborne on surviving valid victims. The primary target is included once, not paid an extra separate bite. Allies, structures, dead/untargetable and out-of-radius entities are excluded. Valid enemy minions/summons/jungle units can be hit.
- Normal S1 remains a single-target bite and slow. Alpha state is checked at bite contact; expiring before contact selects the normal variant, not both.
- Popol attack/S1 tooltip text updated in all eight locale entries. Coefficients and area size remain the approved TFM2 adaptation proposals, not certified MLBB live-client numbers.

## Install and focused test
Close the game, back up 1.1.138 outside mods, replace the installed folder with this ZIP's mlbb_heroes folder, keep one installed copy, and verify version 1.1.139.
1. Spear timing: watch a ranged shot while Kupa is away from that target. Enemy HP should change when the spear reaches it, not when Popol throws it. Kupa's independent bites can still cause other HP changes. Normal armor, shields and crit/procs affect the final number; this is not a raw-HP-damage test.
2. Normal S1: the spear lands, then Kupa's separate contact bite/slow applies when he reaches the target. These are two intended damage components.
3. After level 5 and Alpha activation, watch S1 against enemies packed close around Kupa. Nearby enemies should take the area bite and rise together; a unit clearly outside the small radius should not. The original spear still hits only its intended target.
4. Confirm the working selection sprite, following, mounted poses and prayer recovery remain intact if encountered; no exhaustive repeat animation test is requested.
If anything fails, send the CURRENT mlbb_heroes.log and describe which event. [POPOL combat] logs launch/no-launch-damage, arrival and Alpha area victim IDs. Successful traces are limited to viewer/replay/spectate origins; this does not add sounds or presimulation voice lines.

## Validation and known limits
400 Rust tests pass: launch/arrival separation, damage classification, duplicate/out-of-order flights, target copies, separate simulations, failed spawns, owner death/respawn, area radius/filtering, one primary bite and normal/Alpha expiry behavior. The projectile host fixture simulates callbacks; the actual game was not launched here. Windows build and package verification are recorded in reports.
Spear damage snapshots AD at cast. Flights cancel on owner death and expire after 10 seconds; up to 64 outstanding receipts per owner, with refusal logged rather than instant fallback. The engine owns real projectile movement/collision. No new AoE art was generated; existing Kupa skill art and engine airborne feedback show the contact.
Traps, cooperative passive stacks, complete item inheritance, final audio and balance remain unfinished. Keep 1.1.138 as rollback. Ban/pick preview was not separately owner-confirmed before this combat build.
