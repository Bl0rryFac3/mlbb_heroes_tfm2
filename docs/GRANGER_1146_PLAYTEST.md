# Granger art playtest — 1.1.146 / v2.5.0-rc.4

Install with the game closed by replacing the entire mod folder; confirm 1.1.146. Keep the accepted 1.1.145 folder for rollback. This is a presentation candidate, not a published release.

Observe:
1. Selection card, ban icon and selected champion portrait show Granger; three skill icons show the official art.
2. Idle/run scale matches the roster (45px idle; all poses at most 48px). No doubled bodies, disappearing sprite, floating feet or obvious facing reversal. Check both teams.
3. Basic attacks/S1 show short handgun recoil and warm projectiles; the actual sixth S1 shot has a distinct larger projectile/impact. Damage still occurs on impact.
4. Rondo shows a low dash; the cannon shows a heavy discrete shot/explosion, followed by a distinct roll. S1 can continue during either action. Report clipping, sliding or a muzzle flash visibly detached from the gun.
5. Hit/death/respawn work: one death animation, no standing body after death, body returns after respawn. Thirteen death frames are compiled from the 4x4 source grid.
6. Pause the match: body/VFX should freeze with simulation time. Menus and matches should stay smooth. No per-tick body replacement while idle and no UI-tree scanner were added.

Send the mod log plus a short clip/screenshot if a visual problem appears. No new voice audio is included. Host critical/item semantics remain an existing independent validation item; this build changes presentation, not balance.

Integration choices: handgun v1 avoids v2's baked muzzle flash because ordinary shots use the separate supplied flash. Cannon uses the flash already baked into the body sheet, avoiding a second stacked muzzle flash. All FX retain their 3x3 timing, including transparent fade cells. Sprite pixels are not resized; left-facing sheets mirror each cell. Some dash/roll/death frames reach the source cell's right edge and warrant an in-game clipping check.
