# Hanzo full skill + owner animation candidate —1.1.150

18 November2026, Asia/Manila. Built from accepted projection1.1.149.25 heroes; replace the installed mlbb_heroes folder, keep one enabled copy and start a fresh match. Keep1.1.149 outside the game mods directory as the rollback. This candidate is not a release.

## What changed

All33 owner-uploaded sheets are integrated:18 character/transition clips and15 effect layers. Hanekage uses the supplied floating legless form. Human idle/run/basic/S1/dash/hit/death, spirit idle/glide/attack/Soul Snatch/dash/hit/return/break, parked body, projection and merge are connected to their actual events. Native starting-selection, draft, banned and selected-player portraits retain a visible human frame. Original official skill icons remain; colored dual-form descriptions now match the full kit.

The skill pass adds five-stack jungle devour with four digestion heals; the five-mark Soul Snatch target-max-HP bonus; human five-pulse and spirit four-pulse mist damage/slow; Black Mist path contact/airborne; real-contact and nearby-death energy recovery. Each form retains separate S1/S2 cooldowns. Their clocks follow native cooldown reduction; the accepted projection/return clock remains30s/60s after exit.

## Observe these things

1. **Art and native UI:** Human portraits remain visible in starting selection, draft, selected-player panel and ban icon. Both forms face correctly. Hanekage glides without legs. Body marker stays behind, uses its own animation/ward and disappears on merge. No duplicate attacker or opaque shield hides the character.
2. **Human S1:** Five marks are visible on enemies. A five-mark champion/boss hit produces the bite effect and heal. An ordinary jungle monster can be devoured, followed by four healing ticks over2s. Lane minions, summoned pets and structures must not be instantly eaten.
3. **Human S2:** A short dash leaves mist; its five damage pulses occur0.5s apart and slow targets. The caster gains a short speed boost. Mark stacks can build from the pulses.
4. **Spirit S1:** After a short lunge, the target maw closes once. A five-mark hit is stronger and heals; stacks are consumed. It should track a moving enemy within the lunge's limits, rather than damaging an empty location or hitting twice.
5. **Spirit S2:** Enemies crossed during the dash are struck once and become briefly airborne. The field then pulses four times. Hitting a champion restores20 Energy once per dash, not on every victim or pulse.
6. **Energy, returns and performance:** The HUD can rise from spirit basic hits, champion/boss contact and nearby deaths, but never above100. Fighting may extend projection; the15s cap still forces body catch-up. Normal/failed returns, correct HP restoration, normal human death afterward, legal wall exits and pause behavior must remain as in1.1.149. Menus and fights should remain responsive.

If anything fails, report the item number and send the current mlbb_heroes.log. A short clip is useful for orientation, missing effects, repeated hits, portrait problems or an unexpected actual death/kill count.

## Art limits and implementation choices

17 source character frames exceed the standing48px height limit. They are preserved unchanged under source/hanzo/animations but excluded from playback. Other frames retain native pixels and their order; some attack/projection/hit/idle motions therefore have fewer frames. Nothing was shrunk or clipped. All original33 file hashes match Dropbox. Selected character frames are at most48px tall; cell padding does not change visible size. The supplied VFX have their own larger footprints. Idle/hit empty export cells are omitted. The five supplied mark states are rendered per actual stack count.

The five-mark Soul Snatch bonus currently uses the approved ordinary-armor fallback, combined with the main hit in one physical packet. It does not claim the unproven scoped30% armor ignore. Forced reveal remains omitted because no explicit supported setter was identified. Native BasicAttack damage is used once per mist pulse with scoped zero crit; no additional manual item/on-hit calls or unverified45% proc scaling. Item interactions still need actual gameplay evidence.

Nearby deaths are observed on a bounded15-tick poll, with native credited-kill callbacks and direct-hit receipts deduplicated against that poll. Bodies/marks/ward/link render from simulation snapshots; no render-frame gameplay, UI-tree scans or background voice playback were added. There are no new Hanzo voices in this candidate. English fallback is used for Hanzo in all eight locales.

492 Rust tests pass, including28 Hanzo tests;4 loader regressions and100 champion schema checks pass. The accepted projection tests remain green. A mock is not the game engine: this candidate's full skills, new motion layering, native portrait placement and item effects need this owner playtest.
