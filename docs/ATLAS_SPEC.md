# Atlas — Phase 0 kit and TFM2 proposal

**Hero 26 · Draft for owner review · Baseline v2.6.0 / internal 1.1.158**

Atlas is a melee Tank and teamfight initiator. His job is to close distance, pull a group into allied damage, and survive the engage. His crowd control budget sits in Fatal Links. The basic attack and Annihilate give him something to do between engages.

## Current Mobile Legends kit

I checked all four abilities in Moonton’s rendered Atlas page. The November live patch updates Frigid Breath to a 4-second aura, a 4-second freeze cooldown, and 20 + 0.7% extra max HP in bonus physical and magic defense. The hero page agrees on aura duration and still displays 0.5% scaling. The page supplies the 2-second exposure and 0.5-second freeze. [Moonton Atlas page](https://www.mobilelegends.com/hero/detail?channelid=2678815&heroid=93) · [Original Server patch 2.2.23](https://www.reddit.com/r/MobileLegendsGame/comments/1xtm2m3/patch_notes_2223_org_server/) · [Patch mirror](https://liquipedia.net/mobilelegends/Patch_2.2.23)

| Ability | Geometry | Canon behavior |
| --- | --- | --- |
| Frigid Breath | Moving aura around Atlas | Casting a skill grants temporary defenses. Enemies slow as they remain nearby and briefly freeze after sustained exposure. |
| Annihilate | Three ground explosions around the caster | While ejected, both Atlas and the Mecha Sentry emit the explosions. |
| Perfect Match | Pilot ejection, pursuing suit, reunion burst | The pilot gains speed and slow immunity. The two forms share health and damage reduction. Reunion damages and stuns nearby enemies. |
| Fatal Links | Close-range hero chains, then directional landing | Atlas damages and slows chained enemy heroes, then pulls and slams them at a chosen location. Casting while ejected immediately reunites the suit and replaces the reunion stun with a slow. |

[Moonton Atlas ability text](https://www.mobilelegends.com/hero/detail?channelid=2678815&heroid=93) · [Atlas wiki](https://liquipedia.net/mobilelegends/Atlas)

## Proposed TFM2 numbers

60 ticks per second. Distances are TFM2 world units. AP means current total Magic Power. Cooldowns are base values.

| Component | Starting proposal | Data or native |
| --- | --- | --- |
| Stats and basic | Tank with AD, AP, Melee and CC tags. HP **870 +115/level**, Attack **90 +4/level**, AP **20 +4/level**, armor **40 +4/level**, MR **27 +3/level**, move speed **1040**, HP regeneration **9**, crit **0**. Basic is physical, 100% Attack, 24,000 range, 56-tick interval. | Champion data and standard `Attack`. |
| Passive · Frigid Breath | An accepted active cast refreshes a **4 s** moving aura of radius **35,000** and grants **+10 armor and MR**. Enemy champions inside slow progressively up to **25%**. **2 s** continuous exposure applies a **0.5 s** stun as the freeze. Leaving resets exposure. A per-target **4 s** freeze cooldown survives re-entry and aura refreshes. A skill’s delayed payoff counts as part of its original cast. | Native exposure and cooldown bookkeeping. Timed stat and movement buffs plus the existing stun API. |
| S1 · Annihilate | **7 s** cooldown. Three pulses at **0, 12 and 24 ticks** from the skill’s first impact. Each pulse checks **30,000** around Atlas and deals **30 +10% AP** magic damage to enemy champions and ordinary creeps. While ejected, the suit contributes a second origin. Single-origin contact deals normal damage. Overlap deals **45 +15% AP** as one damage packet per pulse. | Data defines the action and cooldown. Native pulse scheduling, dual origins and target deduplication. |
| S2 · Perfect Match | **12 s** cooldown. Eject for up to **2 s**, with **+25% move speed** and provisional **20% damage reduction**. The suit follows and reunites on contact after a short separation window, or at the time cap. A normal reunion deals **65 +20% AP** magic damage and **0.6 s** stun to hostile champions within **30,000**. | Data defines the action and cooldown. Native state, follow position, buff lifetime and reunion. |
| Ult · Fatal Links | **45 s** cooldown. Link visible enemy champions within **42,000**. Link hit deals **35 +10% AP** magic damage and **30% slow** during the **0.4 s** windup. Automatically release at Atlas’s current position. Eligible links pull toward Atlas for up to **0.3 s**, then take **105 +25% AP** magic damage and **0.35 s** airborne. Links break if a target dies, leaves **55,000**, or becomes invalid before release. Casting while ejected ends ejection immediately and replaces the normal reunion payoff with a **40% slow for 0.6 s** in the reunion radius. The stronger slow takes precedence where the effects overlap. | Data defines the action and cooldown. Native champion filter, target snapshots, pull receipts, timed slam and ejection interaction. |

The HP and defensive stats sit beside existing Tanks. Franco has 880 HP, 40 armor and 26 MR. Belerick has 870, 36 and 28. Johnson has 860, 42 and 24. Atlas starts close to Uranus’s 25 AP and 5 AP growth and well below Gusion’s 78 AP and 8 growth. A full S1 has 30% AP scaling, or 45% at full overlap. Fatal Links has 35% across both hits. The 45-second cooldown sits between Johnson’s 36 seconds and Franco’s 62 seconds. These comparisons use the shipped `champion/*.data_champion` files from the Hanzo release.

## Engine decisions and limits

- **One combat actor.** Atlas owns health, items, ability cooldowns, and kill credit throughout ejection. The suit follows as a simulated position with match visuals and a second S1 origin. Separate targetable suit collision is outside this design. Read the presentation from the same per-actor state as the skill geometry. Native movement and recall keep control of the pilot.
- **Hanzo precedent.** Hanzo’s Pinnacle Ninja keeps the active form on the existing champion and draws the body at stored world coordinates through positional view effects. Atlas can use the same approach for the suit. On ejection, save the suit’s starting position and advance it toward the active pilot on simulation ticks. Use that position for both the suit sprite and the second Annihilate origin. The pilot keeps one continuous HP pool and native movement and recall. The suit catches up to the pilot, whose position stays under native control. Handle contact, timeout and Fatal Links through one reunion path so each ejection pays out once. Hanzo’s parked-body return, HP restoration, wall phasing and temporary banish are specific to his ult. [Hanzo Phase 0](https://app.notion.com/p/41c9c01d37c981098d11d9b0763c3942)
- **Bounded native pull.** The stable SDK documents `entity_pull` as pulling toward the caster for a fixed number of ticks and respecting CC immunity. Franco supplies an existing in-match displacement pattern using `entity_grab`. The first Atlas probe uses the caster-relative pull and a single bounded release. Targeted landing away from Atlas requires separate movement and terrain proof. Atlas can be interrupted by hard CC during the windup in the first probe, so the MLBB suppression-only exception is deferred.
- **Controls and mitigation.** The SDK exposes timed movement and mitigation buffs, plus a general CC-immunity flag. Slow-only immunity needs a host proof. Use the speed bonus in the first probe and keep ordinary CC rules. Verify the proposed mitigation buff before exposing its percentage in tooltips. Use the stun effect with freeze visuals. Use Hanzo’s existing airborne primitive for the slam only after the pull test passes.
- **Target filters and settlement.** Pulls, freezes and reunion crowd control are champion-only. Structures, epic objectives, minions and summons stay outside those effects. The initial Fatal Links hit can land independently. A refused pull grants no slam or airborne. A target that dies or becomes invalid before the delayed impact receives no follow-up. Clear transient state on death and match end. Recall clears the suit presentation without a reunion hit.
- **AI.** Prefer Fatal Links on two valid nearby enemies, or one when an ally can follow up. Let Perfect Match support an existing local engage. Preserve native walking and retreat. Atlas should keep enough time to cast Annihilate while the suit is still out, without forcing an attack, path change or recall cancellation.
- **Resource and text.** Mana costs are omitted from the first TFM2 adaptation. At integration, neutral third-person colored tooltips will describe the actual numbers and timing across all eight locale entries.

Engine references in the 1.1.158 source are `sdk/mod-api-stable/src/sim.rs`, `sdk/mod-api-stable/src/values.rs`, `sdk/candidate/franco.rs` and `sdk/candidate/hanzo_combat.rs`. [Project handoff](https://www.dropbox.com/preview/Shien%20Ken/MLBB_Heroes/04_docs/PROJECT_CONTEXT_v2.6.0_1158.md)

## What the technical probe must establish

1. Atlas leaves base, walks, attacks and recalls normally before and after ejection. Reunion fires once, and a second Atlas or replay copy has independent state.
2. Passive exposure clears on exit and cannot freeze twice through refresh or immediate re-entry. S1 deals the expected amount at either origin and in overlap.
3. Fatal Links pulls multiple valid targets with one release. Test CC-immune targets, an interrupted windup, a death before release, and enemies near terrain or towers. The ejected cast produces one early reunion and its slow.
4. Game-side checks cover readability and the S2 → ult → S1 engage with allies present. Automated checks cover receipt counts and state transitions.

## Owner review

The review choices are the 870-HP Tank stat line, the reduced damage on overlapping S1 pulses, and automatic Fatal Links toward Atlas for the first probe. Phase 1 starts after the kit and numbers are approved. [Hero Pipeline](https://drive.google.com/file/d/1ujSv5Sk_FAC3CfzE41iy_4ESAh9M68LA/view)
