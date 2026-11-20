# MLBB Heroes — current handoff, 19 November 2026

- Prepared public v2.6.0 / internal1.1.158, 25 heroes, TFM2>=0.7.3. Public v2.5.0 is still latest. Owner publishes GitHub/Discord; nothing was published by the assistant.
- Owner rejected1.1.157: Popol/Granger/Hanzo stayed at base. Its continuously refreshed self-Animation CC helper is removed completely. All three visual modules/carriers are restored byte-for-byte from delivered1.1.156. Hanzo/Popol gameplay/AI and the frozen SDK are unchanged.
- Cecilion comparison: native nine-frame/1.600002s idle shared by UI and matches. The three layered heroes cannot simply use that visible base without duplicate bodies. Animated hover previews remain a disclosed known issue; no new UI scanner or state-based workaround.
- Granger retained buffs: S1 28 +40% Attack per round; ult130 +105%. Costs/regen/cooldowns/homing/crit route unchanged. Actual crit-damage-item scaling remains unverified. Hanzo 1.1.156 mark/ward/mouth/text/buff and four Tank categories retained.
- 537 Rust pass, one inherited ignored; four loader tests;100 root schema checks;Windows x64 built. Regression tests now assert zero self-animation CC submission. Preservation audit:1,449 existing runtime asset/data files unchanged, original source art unchanged, only two Granger production damage formulas differ from1.1.156.
- Before owner publishes: confirm1.1.158 and fresh-match base departure, walking/recall/attacks, mounting and both Hanzo forms. No native engine run occurred here. 1.1.156 remains gameplay rollback with known hover defect. Never use1.1.157. The old isolated hanzo_release_v260 tree holds1.1.156 and is superseded; current canonical release source is hanzo_repair.
- Dropbox /Shien Ken/MLBB_Heroes:01_windows_candidates/mlbb_heroes_v2.6.0.zip;02_full_source/MLBB_Heroes_TFM2_v1.1.158_v25_full_source_with_SDK_v2.6.0.zip;04_docs GitHub/Discord/build report/publication checklist/context/manifest.
- Notion is durable project memory; Dropbox holds artifacts with standing upload permission. No Drive writes. Preserve native Hanzo Move/Return/None; no custom chase/recall/hold. No per-frame UI-tree scanners. Neutral third-person colored tooltips; sprite target45px/max48px without resampling. Gusion/Uranus gameplay benchmarks preserved.

Release documents are embedded below for continuity. Owner publication checklist is a separate file.

---

# v2.6.0 — New hero Hanzo, now 25 heroes

## New hero: Hanzo

- **The Akuma Ninja.** An assassin with human and Hanekage forms, animated sprites and effects, official skill icons, colored descriptions and native champion portraits.
- **Ame no Habakiri (passive).** Basic attacks and mist hits build up to *5* Dark Ninjutsu marks. Marks strengthen Soul Snatch and unlock human-form Demon Feast.
- **Demon Feast / Soul Snatch (S1).** At five marks, human Hanzo bites an enemy for physical damage and healing, or devours an ordinary jungle monster and restores *20% maximum Health* over time. Hanekage lunges and strikes with a large demonic maw; five marks add maximum-Health damage and healing. Spirit S1 deals burst damage rather than instantly devouring a camp.
- **Dark Mist / Black Mist (S2).** Dashes and leaves a damaging mist trail that applies marks and slows enemies by *35%*. Hanekage can dash through walls and knock enemies airborne for *0.4s*.
- **Pinnacle Ninja (ultimate).** Leaves a protected body behind and projects Hanekage, gaining *35% Movement Speed*, *25% Attack Speed* and wall traversal with *30% less maximum Health*. Recast to return, or let depleted Energy draw the body toward Hanekage. Returning restores body Health and heals; defeated Hanekage returns with a temporary slow and doubled cooldown.

## Also in this release

- **Granger damage increased.** Rhapsody deals *28 + 40% Attack* per round, up from *20 + 30%*. Death Sonata deals *130 + 105% Attack*, up from *110 + 90%*. Energy costs, regeneration, cooldowns and homing remain unchanged.

- **Tank roles for TFM2 0.7.3+.** Franco, Johnson, Belerick and Uranus now appear as Tanks. Their skills and stats are unchanged.
- **25 heroes · 8 language settings.** Hanzo, Granger and Popol & Kupa currently use English descriptions in every language setting; the other heroes retain their existing localized text.

## Known issue

Animated hover previews for Popol & Kupa, Granger and Hanzo may disappear. The issue remains under investigation.

**Requires Teamfight Manager 2 version 0.7.3 or newer.** Download **`mlbb_heroes_v2.6.0.zip`** from Assets below. With the game closed, extract it, open the outer `mlbb_heroes_v2.6.0` folder and copy the inner `mlbb_heroes` folder into `Teamfight Manager 2\mods\`. Replace the previous folder completely and keep one enabled copy. `Install_Guide.html` is beside the mod folder.

The mod manager shows **1.1.158**; **v2.6.0** is the public release label. GitHub's automatic “Source code” downloads are repository snapshots, not the installable mod. The matching development archive is **`MLBB_Heroes_TFM2_v1.1.158_v25_full_source_with_SDK_v2.6.0.zip`**.


---

# Discord announcement — v2.6.0

## Long form

```
# 🗡️ MLBB Heroes — v2.6.0 · 25 heroes

**New hero: Hanzo, the Akuma Ninja.** Human and Hanekage forms, animated sprites/VFX, skill icons and colored descriptions.
**Passive:** Basic attacks and mist build up to 5 Dark Ninjutsu marks.
**S1 — Demon Feast / Soul Snatch:** Bite marked enemies or devour ordinary camps. Hanekage lunges with a demonic maw; five marks add maximum-Health damage and healing.
**S2 — Dark Mist / Black Mist:** Dash and leave a damaging, slowing trail that builds marks. Hanekage crosses walls and knocks enemies airborne.
**Ult — Pinnacle Ninja:** Project Hanekage, leave a protected body behind, then return and heal. Defeated Hanekage returns with a slow and longer cooldown.

**Granger buff:** S1 28 + 40% Attack per round; ult 130 + 105%. Energy costs and homing unchanged.
**Tank roles:** Franco, Johnson, Belerick and Uranus. **Requires TFM2 0.7.3+.** Internal build 1.1.158.
**Known issue:** Popol, Granger and Hanzo hover previews may disappear.

⬇️ **Download:** https://github.com/Bl0rryFac3/mlbb_heroes_tfm2/releases/tag/v2.6.0
```

## Short form

```
# 🗡️ MLBB Heroes — v2.6.0 · 25 heroes

**Hanzo joins:** Five-stack marks, jungle devour, mist trails and Hanekage spirit combat with a protected body and return/heal.
**Granger buffed:** S1 28 + 40% Attack; ult 130 + 105%.
**New Tank roles:** Franco, Johnson, Belerick and Uranus.
**TFM2 0.7.3+ required · internal 1.1.158.** Popol/Granger/Hanzo animated hover previews remain a known issue.

⬇️ **Download:** https://github.com/Bl0rryFac3/mlbb_heroes_tfm2/releases/tag/v2.6.0
```

## Posting notes

Publish the GitHub release first so the labelled download link resolves. Paste only the chosen code-block text, without fences. One ability per source line; no hard wrapping. Hanzo/Granger/Popol descriptions currently use English in all eight language settings. The assistant prepared this copy; no announcement was sent.


---

# Build report — v2.6.0 / internal 1.1.158

19 November 2026, Asia/Manila. 25 heroes. Requires TFM2 0.7.3+. Prepared for the owner to publish, with a fresh-match movement check still pending. Previous published release: v2.5.0 (verified read-only). No GitHub tag/release or Discord message was created.

## Freeze repair

Owner reports Popol, Granger and Hanzo remain at base in 1.1.157. The new shared match_carrier helper applied CcKindV1::Animation to each owner for 3,600 ticks, refreshed at 1,800 ticks. At 60 ticks/second this maintained a simulation CC state continuously. The earlier claim that it was cosmetic was not established by native-engine evidence. Mock tests stored CC and called the mod's own gates/direct skill dispatch; they did not execute the engine pathfinder/input restrictions.

This build removes that helper, registration and all three call sites. The three visual modules, carrier metadata and compiler outputs are restored exactly from delivered 1.1.156. The new 1.1.157 preview compiler and tests based on its assumptions are removed. Removing only the helper would have left the visible match base drawing beneath body layers; restoring the matching carrier metadata prevents that incomplete rollback. No energy removal or AI/pathfinding rewrites.

## Cecilion comparison and remaining limitation

Cecilion's native idle has nine frames over 1.600002 seconds. His native sprite supplies both UI and match body animations. Popol, Granger and Hanzo use separate pose layers for mounted/action/human/spirit states on transparent gameplay carriers. Copying only Cecilion's visible native idle would draw an extra body under those layers. His animation and gameplay assets are unchanged. No new replacement UI renderer or UI-tree scanner was introduced.

Animated hover previews for these three heroes remain unresolved. Static first-frame portraits and the previous 1.1.156 presentation are retained. Both public GitHub and Discord text disclose this limitation; the package does not claim a hover fix. A UI-only fix needs independent native validation before it can be added.

## Release content and retained balance

Hanzo is hero25: human/Hanekage forms, five-stack Dark Ninjutsu, ordinary-jungle devour, Soul Snatch maw, mist trails/marks/airborne, parked protected body/link and return. Retains the 1.1.156 compact neutral colored text, corrected mark lights, hollow translucent ward, spirit-mouth receipt and damage/survival pass. Tank categories remain Franco, Johnson, Belerick and Uranus; their stats/skills unchanged. Eight language settings; Hanzo/Granger/Popol descriptions use English across all settings.

Granger damage only: Rhapsody 20 +30% Attack →28 +40% per round; Death Sonata 110 +90%→130 +105%. At104 Attack, pre-mitigation S1 packet51→69 and ultimate203→239. Costs, energy regeneration, cooldowns, homing, collisions, crit routing, audio and other mechanics are unchanged. Sixth-shot crit-item scaling remains unverified in the real game; no force-crit multiplier or attack-type reclassification was added.

## Validation and preservation

537 Rust tests pass, zero fail, one inherited manual test ignored. Two new regression tests exercise Granger idle/barrage/dash and Hanzo human/spirit presentation while asserting no self animation-CC submission; unlike the rejected tests, they do not assume Animation is harmless. Existing native Move/Return/None passthrough, both-form skills and resource tests pass. Four loader tests and100 root champion-schema checks pass. Windows x64 GNU release build passes with17 inherited warnings; test build has16 inherited warnings. Both stable exports and system-only imports verified.

Audit against delivered1.1.156 preserves1,449 existing runtime asset/data files, every original source-art file and the frozen SDK. All existing production Rust is byte-identical except Granger's exact two damage formulas. Hanzo/Popol AI/gameplay/visual modules and shared lib are unchanged from1.1.156. The only extra Rust differences are the new/updated tests. All root champion definitions and override routes are identical. DLL SHA256: `9cd8cdd10d142707e14ec3d374ef2a654759519a93cd53f189323f11b9d77f67`.

The final runtime/source ZIP CRCs, wrapper layout, all packaged mod-entry/source comparisons and remote Dropbox content hashes are verified in the delivery manifest. A clean-source build can use the included SDK; historical reports retain their original version context.

## Owner pre-publication check

Install the supplied release ZIP with the game closed, replace the entire previous folder and keep one enabled copy. Confirm internal1.1.158, start a fresh match and verify all three leave base, walk/recall/attack normally, with no duplicate bodies. Check Popol mount/dismount/Kupa recovery and Hanzo body/link/both-form skills/return, then Granger's damage. This environment did not run the TFM2 engine, so offline checks are not live movement/renderer proof. Restore1.1.156 and stop publication if this behavior regresses. The 1.1.157 candidate is rejected; never publish it.

Installable asset: `mlbb_heroes_v2.6.0.zip`. Matching source: `MLBB_Heroes_TFM2_v1.1.158_v25_full_source_with_SDK_v2.6.0.zip`. Mod manager/Cargo version remains1.1.158; public release labelv2.6.0. Owner publishes GitHub/Discord personally. No Drive writes, new paid art, audio downloads or messages to others.

