# Lesley Windows candidate 1.1.172

Close the game and move the previous MLBB Heroes folder out of `mods`. Extract the ZIP directly into `mods` and confirm that the Mods menu shows **1.1.172**. Chou candidate **1.1.171** is the previous private build.

- In draft, hover Lesley and check her native idle. In a match, watch her run, attack and die. Draft and combat share the native sprite. Watch the death countdown for any character, name, shadow or lock marker left at the death position.
- Compare a normal attack and Lethal Shot. Wait five seconds after taking damage, or cast a skill, then check the stronger shot and longer reach. Repeat against a jungle unit and a tower. A shot should deal damage once.
- Check the speed and shimmer from Master of Camouflage. The first candidate implements visual camouflage and movement speed. The shimmer ends when Lesley attacks, casts again or takes damage after 0.5s.
- Watch Tactical Grenade from both facings. The cone and victim knockback should go forward while Lesley steps back. Try a backstep into a wall to confirm that it stops on the near side. Check the hit against a jungle unit as well.
- Count four Ultimate Snipe reports and inspect the lock marker. Place a second opponent in the line of fire before one of the rounds. This candidate checks for a blocker at each release. Test a stun, retreat, death and target death during the channel. The marker and pending shots should clear. If the native game accepts a grenade input during the channel, it fires one final round and ends the volley.

No match playtest was performed in the build environment. Check the target, hit position, and aim pose together because a native animation may continue briefly after a mod channel ends. The candidate uses the owner's visual effects as the source for the skill icons. Skill audio and localization beyond English fallbacks are for a separate pass.
