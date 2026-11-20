# Granger technical probe — v2.5.0-rc.1 / internal 1.1.143

17 November 2026. Based on the owner-accepted, published v2.4.0 / 1.1.142. This is a separate mechanics candidate, not a release or animation-complete hero. The owner publishes GitHub releases personally.

## Install

Close TFM2. Keep a backup of the installed mlbb_heroes folder, then replace it with the inner mlbb_heroes folder from mlbb_heroes_v2.5.0-rc.1_windows_candidate.zip. Do not merge two versions. The outer folder and Install_Guide.html remain outside mods. Confirm internal 1.1.143 in the mod manager/startup log. Restore the v2.4.0 folder to roll back; no save conversion is intended or required.

## Included

- The exact owner-supplied 72×48 transparent PNG, padded into a 72×72 cell without resampling. All standard motions deliberately reuse the same still. Sliding during dash/roll is expected at this stage.
- Native selection, face, ban and selected-champion layout entries; three temporary skill slot icons. These are not official MLBB skill icons or finished art.
- Separate 800 Gun/Cannon pools with fractional 8%/2% maximum regeneration per second. A small diagnostic panel at the right shows G/C and Rondo's effective remaining cooldown.
- Rhapsody: six directional rounds at 18-tick intervals, 70 Gun per successfully spawned round, strict >420 start, 12 +18% snapshotted AD per round, +10% movement. Hard control/silence cancels future rounds. Existing projectiles retain their own receipts. Each actual hit refunds Rondo by 30 ticks once.
- Rondo: 35k short dash, 15% maximum Gun refill, 720-tick effective cooldown reduced by hits. Actual wall cells are reused from the existing map hook; Fanny's virtual cable borders are removed. No immunity or deliberate wall crossing.
- Death Sonata: 250 Cannon per emitted shot, 36-tick gate, native EnemyChampion collision filter, 70 +65% AD physical explosion within 18k, 30% slow for 36 ticks. A short retreat roll is queued after firing; Rhapsody keeps running.
- Credited minion CS grows Gun by 6 and restores 5%; jungle CS grows Cannon by 6 and restores 5%. This split is the documented TFM2 interpretation. Epic/Serpen objectives, structures and summons do not grant farm growth in this probe. Personal cumulative kills plus assists award both pools +12/10% once per credit. Capacity state is retained across the passive's death/respawn lifecycle; a cleared copy marker can migrate through passive update.
- Equipment CDR is inferred from Rondo's full native cooldown cycle, not remaining time; 0.5 AD per point, capped at +20 AD. An independent Rondo timer prevents also gaining a shorter effective cooldown. The native icon can become ready earlier with CDR; the diagnostic R counter is authoritative for this probe.
- Neutral colored tooltips in English fallback under all eight existing language settings. Existing roster translations remain intact.

## Observe in one or two matches

1. **Visibility and performance:** Granger appears in starting selection, the draft grid, banned icon, selected champion and match. Menus and matches remain smooth. Static poses are expected.
2. **Resource panel:** G and C are separate. Small handgun rounds spend G in six steps; the larger cannon round spends C. G refills faster. Pause for several seconds: the resource values and firing must stop; the panel may remain visible.
3. **Rhapsody and Rondo:** Watch a barrage at normal/slow speed. There are six small shots, the last red. A dash/refill can happen during the barrage; later rounds start at the new position. A hit reduces R, a miss does not. Stun/silence should stop future shots without refunding earlier costs.
4. **Cannon:** It can fire during Rhapsody, travels through lane minions to the first enemy champion, explodes once and briefly slows nearby enemies. Watch for the short retreat roll. Neither dash nor roll should cross walls or leave Granger stuck.
5. **Growth and death:** After farming/takedowns, maxima can rise above 800. After Granger dies and respawns, those maxima should stay; a new match starts at 800/800. If growth resets, report whether the whole champion entity/portrait also changed.
6. **Final shot:** The sixth red projectile is routed through the real engine damage pipeline with a temporary 100% crit-chance buff only during its impact. Compare its combat number/crit appearance with other rounds, ideally using a crit-related item. The mock tests do NOT establish the real host's Skill-crit multiplier or item hooks. Report if the last round behaves like an ordinary round.

Send the fresh mlbb_heroes.log with any issue, plus which checklist number failed. A short normal-speed clip is useful for concurrency, collision or critical-hit behavior. No further animation/VFX generation is needed until this proof is accepted.

## Limits requiring host evidence

No TFM2 client was available in this environment. Native render placement, host Direction cast acceptance, first-hero collision, true critical/item semantics, equipment-CDR inference and the actual engine's respawn-passive lifecycle remain owner playtest checks. The scoped crit probe does not manually multiply damage, change all basic attacks, or claim a verified host critical multiplier. Host item proc re-entry during that single engine call also requires observation before production. Muzzles, impacts, explosion art, full animations, official skill icons and Granger audio are not supplied in this candidate. The diagnostic resource panel is temporary and is not a final spectator HUD design.

Published Popol/Kupa gameplay and assets, Julian audio implementation, Gusion and Uranus yardsticks are preserved. The extension forwards the existing voice pre/post-update and end callbacks; no UI-tree scanning is added.
