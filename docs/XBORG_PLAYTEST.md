# X.Borg playtest · 1.1.201

This Windows candidate targets TFM2 0.7.4. Keep build 1.1.200 outside `mods` for rollback. Confirm that the Mods menu displays 1.1.201.

## 1.1.201 owner follow-up

The owner confirmed the 1.1.200 changes in their match and reported that X.Borg did not show his basic attack at towers. They authorized the official skill icons and voices and requested Firaga sustain on S1 and a separate ultimate explosion effect.

1. **Tower and Nexus basics.** The engine action already used `Enemy`. The native X.Borg hit callback applied a second range and targetability check before showing the attack, which can reject the structure the engine chose. A mock attack on the tower beyond the duplicate center check reproduced the missing strike. The new path uses the engine-selected target, faces it, draws the correct pose and applies one ordinary `BaseAttack` through the engine. Armorless structure basics use a direct impact. Units still use the existing armorless projectile. Structures grant no heat. [TFM2 stable API](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/stable-api-reference.md)
2. **Official media.** Four default skill images were obtained from the wiki's media host. The three active slots show the official icons. The passive artwork is included as its own file. The 29 default English cues cover first match spawn, movement, basics, S1, S2, ult, Firaga restoration, champion and epic kills, death, and respawn. Playback uses the shared per-actor and match-wide voice gates. The source media manifest lists the original files, URLs, and SHA-256 hashes. [Ability media](https://mobile-legends.fandom.com/wiki/X.Borg) · [Default English audio](https://mobile-legends.fandom.com/wiki/X.Borg/Audio#Default)
3. **Firaga sustain and item effects.** S1 uses actual target body HP damage. Its existing heal value is 10% against champions or 5% against jungle and epic monsters, capped at 12 per pulse. It additionally restores half that value as reserve while armored or a quarter as recharge energy while armorless, capped at 6 reserve or 3 energy per pulse. Firaga can recover at full body HP. The engine resolves each pulse as typed skill damage, so equipment follows its own applicability rules. Skill-compatible vamp can add body healing. Basic-only lifesteal applies to basic hits. Item healing is excluded from the Firaga calculation. [TFM2 stable API](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/stable-api-reference.md)
4. **Explosion.** The supplied full explosion plays once at the blast center on normal detonation. An armor-break detonation uses the supplied reduced-intensity variant. The effect and damage fire on the same event.

### Live checks

- Push an enemy tower in both forms. Look for a facing-correct basic pose and one HP loss per accepted attack. Repeat at the Nexus. Check whether an armorless attack on a minion still shows the ember projectile.
- Damage Firaga with X.Borg at full body HP, then land S1 on a champion or regular camp and inspect the orange bar. Break the armor and repeat to see the extra blue recharge. Compare body HP gain with and without an item that supports skill vamp.
- Watch a normal Last Insanity explosion, then break Firaga mid-spin. Listen for the official cast and restoration cues. Check Fire Missiles, Fire Stake and Last Insanity icons in the skill panel.

## 1.1.200 baseline

## Owner feedback and follow-up

The owner confirmed that the 1.1.199 changes worked and reported that the Firaga gauge floated too high, Last Insanity ended too soon, and X.Borg lacked a visible idle animation in champion selection. They requested colored tooltips, official skill icons, and voice lines.

1. **Firaga gauge.** Lowered the world-space offset from 62,000 to 36,000 units using the supplied screenshot. Check clearance at the top of the head in both forms, through recoil and against other units.
2. **Last Insanity.** The old timer allowed two seconds, and the AI could detonate after 0.6 seconds against a killable or departing target. The published MLBB skill uses a three-second charge and spin window. The TFM2 candidate now ends at three seconds while armor holds. There are six damage pulses at half-second intervals. A cold target in every pulse reaches four stacks at 1.5 seconds. Detonation deals true damage equal to 100 + 6% target max HP. A target already at full heat takes 50% more blast damage, up to the 150 epic-objective cap. The forced blast from an armor break deals half the corresponding damage. [X.Borg skill data](https://www.mlbbcore.com/heroes/xborg/)
3. **Tooltips and icons.** Matched the color tags used in the shipped Sun and Valir descriptions. The active skills and passive have distinct compositions from the owner PNGs. The public wiki identifies the official icons and default voice events for selection, movement, skills, ultimate and death. Those source files have no verified license for bundling in the mod. [Hero and icons reference](https://mobile-legends.fandom.com/wiki/X.Borg) · [Default audio reference](https://mobile-legends.fandom.com/wiki/X.Borg/Audio#Default) · [Moonton terms](https://www.mobilelegends.com/news/articleldetail?newsid=2728920)
4. **Menu idle.** X.Borg already had a draft-hover handler, but it used a renderer that expects SVG and the archive shipped PNGs. The nine new SVG frames use the same exporter as Yu Zhong and play on the supported starting, lineup and database champion cards and the live ban/pick hover. The shared card scanner rechecks caption identity and scroll clipping. [Popol selection fix](https://app.notion.com/p/41a9c01d37c981629807fb4afbf9e61b) · [Yu Zhong draft preview](https://app.notion.com/p/41f9c01d37c9817788bbe8e0883b4ca9)

## First checks in the Windows client

- Watch a full ult into a healthy enemy and a low-health enemy. Watch the heat pips reach four, then the three-second detonation. Repeat with a second, cold enemy on the outer blast fringe to compare damage. Break Firaga during a spin to verify one weaker forced blast. The `[XBORG]` log names the timer or armor-break finish and counts overheated blast hits.
- Inspect the smaller gap between the Firaga bar and head in both forms. Open the descriptions and verify the first, second and ultimate icons.
- Open champion selection, watch the armored idle on X.Borg's card, then filter or scroll away and return. Hover X.Borg in the ban/pick grid. If the selection card stays empty, F8 records `[champion preview]` and `[draft grid]` details.

## 1.1.199 baseline

## Previous live playtest

The owner confirmed Firaga break and restoration, Fire Missiles in both forms, armored Fire Stake and Last Insanity, Firaga supplies, and the heat pips. Armorless Fire Stake was unobserved. Its existing ten-second cooldown can overlap the armorless window, especially if supplies accelerate rearm.

## Research and implementation

Published X.Borg skill data describe moving directional area fire, per-enemy overheat, armorless fire with greater reach, and a separate Firaga resource. The mod uses its established four-stack threshold and native typed damage for the true-damage payoff. The 1.1.198 S1 already enumerated targets on every pulse and compared each unit's center with the cone. It excluded epic monsters from heat and let Fire Stake benefit from existing heat without adding a stack. [Moonton Hero Spotlight](https://www.youtube.com/watch?v=237oBMPCteY) · [November 2026 hero data](https://www.mlbbcore.com/heroes/xborg/) · [TFM2 stable API](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/stable-api-reference.md)

1. **Area and heat.** S1 checks hitbox overlap with the full sector on each of six pulses over a 1.5-second cast. The origin travels with X.Borg while the cast heading stays fixed. Armored fire covers 70° at 50,000 units. Armorless fire covers 36° at 65,000 units and deals 60% damage. Every targetable enemy unit intersecting the sector can be hit, including lane minions and summons. Champions and regular and epic jungle objectives build separate heat. After the fourth hit builds the last stack, subsequent skill hits deal true damage to that target. Towers and the Nexus remain basic-attack targets.
2. **Basic attacks and Fire Stake.** A landed basic or stake recall adds one stack to an eligible target. Five overlapping paths still hit each unit only once. Fire Stake has a 55,000-unit armored reach and a 70,000-unit armorless reach. The shared cooldown is ten seconds. Supplies still come from overheated champions and regular camps.
3. **Sustain.** S1 restores body HP equal to 10% of champion HP lost and 5% of monster HP lost, with a 12-HP cap per pulse. Only actual health loss contributes, so overkill and shield absorption add no base S1 healing. Lane minions take reduced damage. Damage uses the engine's typed skill pipeline for item interactions.
4. **Readability.** The owner-provided flame frames are 48 and 64 pixels wide at their east-facing source angle, up from 36 and 49. Both effects keep their muzzle pivots and all 16 directions. The small orange gauge above X.Borg tracks reserve. It switches to blue to track rearm energy, then returns to orange around 30% when the armor comes back.

Firaga uses the same 20% damage-reduction buff and separate reserve as 1.1.198. The new gauge reads that reserve and the recharge meter. The corresponding native absorb layer remains an engine investigation.

## Live client checklist

| Order | Exercise | Expected result |
| --- | --- | --- |
| 1 | Watch an S1 through a packed wave or several champions. Check both cone edges and both facings while moving. | All enemy hitboxes overlapping the fan take the relevant pulses. Enemies clear of the fan stay untouched. Both right-hand flame effects are larger. |
| 2 | Start one target cold and heat another with a basic or S2 first. Repeat at a jungle camp and an epic objective. | Pips build independently. Later pulses deal true damage to the hot target even while a neighboring target is still warming up. |
| 3 | Test after level 3 with Fire Stake ready before armor breaks. Watch the launch and the recall over an enemy or supply. | Armorless stakes launch farther in a tighter spread. The recall applies one hit and one heat stack per eligible target. Logs distinguish armored and armorless launches. |
| 4 | Watch S1 on injured X.Borg against a champion, then against a minion wave. | Champion damage restores body HP. Lane minions grant no base S1 healing. Try any equipped vamp item separately to observe its own effect. |
| 5 | Damage the armor, break it, collect a supply during recharge, and watch it restore. Then check a Last Insanity ending. | The overhead orange reserve falls, blue recharge rises, and the orange reserve comes back partly filled. The standard HP bar continues to show body HP. |

`[XBORG]` traces report the hit count on each S1 pulse and the form on each S2 launch. The source archive retains the original [integration plan](XBORG_INTEGRATION_PLAN.md).
