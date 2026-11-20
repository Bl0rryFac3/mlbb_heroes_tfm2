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
