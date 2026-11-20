# Granger lock-on candidate — v2.5.0-rc.3 / internal 1.1.145

17 November 2026. The owner confirmed 1.1.144 uses S1/S2/ult without the prior panic, then requested homing projectiles because the AI misses moving enemies. This is an intentional TFM2 adaptation approved by the owner, not a claim about current MLBB aiming.

## Install

Close TFM2. Back up the installed mlbb_heroes folder, then replace it with the inner mlbb_heroes folder from mlbb_heroes_v2.5.0-rc.3_windows_candidate.zip. Do not merge versions. Confirm internal 1.1.145. Published v2.4.0 / 1.1.142 remains the full-release rollback; 1.1.144 remains the owner-confirmed crash-fix candidate.

## Changes

- Rhapsody locks onto the selected enemy and emits six homing projectiles toward that enemy. Later rounds use its current position, not the original firing line. The AI sends a target ID instead of guessing a firing direction.
- A burst retains its lock while the target is alive, targetable, visible and within the 90k acquisition range. When that lock is invalid, remaining rounds select another visible enemy in range, prioritizing heroes. If none exists, remaining rounds stop without charging their energy. Already-emitted missiles keep their own receipts.
- Death Sonata targets an enemy hero within 140k and uses a homing cannon projectile. The hero collision filter, one explosion, nearby damage/slow and retreat roll remain. Rondo stays a directional dash.
- Both ability data and neutral colored tooltips now describe targeted/homing behavior. Granger still uses English fallback in all eight language settings.
- Projectiles retain the Targeting hit callback that fixed the 1.1.143 panic. Target movement controls homing; Direction is never sent as a projectile callback type.
- Expired homing receipts stay reserved while this caster still has live native projectiles. Cleanup of expired slots requires two quiet host observations. This avoids a late old missile stealing a newer shot's receipt and allows lost/ended missiles to be reclaimed within the bounded 64-slot book.

Energy, damage, six-round cadence, hit-only Rondo refunds, critical probe, progression, temporary art, other heroes and Julian audio are unchanged.

## Short retest

1. Watch S1 against an enemy that moves sideways: bullets should follow the target, including later rounds after Granger dashes.
2. If that enemy dies/becomes untargetable/leaves acquisition range during the burst, remaining rounds should switch to another visible enemy in range, or stop when none remains. No energy is charged for stopped rounds.
3. Ult should follow a moving enemy hero, then explode once and slow nearby enemies. Check the short retreat roll still occurs.
4. Watch Gun/Cannon costs and Rondo hit refunds. There must be no repeated Direction-projectile panic, ghost damage or long-match loss of firing.

Static poses and temporary projectile/skill icons remain intentional. No new image generation was run. Send a fresh mlbb_heroes.log and a brief description/clip if the lock-on still misses or firing stops. True host critical/item behavior, final HUD placement, CDR inference and actual respawn lifecycle remain wider Phase 1 checks; this build was compiled/tested, not run in TFM2 here.
