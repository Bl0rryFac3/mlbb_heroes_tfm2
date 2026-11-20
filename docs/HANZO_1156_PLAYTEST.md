# MLBB Heroes1.1.156 — TFM2 0.7.3+ retest

Requires0.7.3 or newer: this build uses the native Tank category. Replace the mod folder, keep one enabled copy and start fresh. Accepted gameplay rollback remains1.1.154;1.1.155 has owner-reported mark-light/mouth issues.

1. **Lights:** One target should show1,2,3,4,5 bright symbols. Source clusters are now counted: the original atlas is1,3,5,2,4; display mapping is0,3,1,4,2. No glyph pixels changed.
2. **Descriptions:** Single skill headers;360/323/333 plain characters for S1/S2/ult, preserving neutral wording, colored values and both forms. Check all three fit the actual draft panel. The included tooltip preview is representative layout evidence, not a game screenshot.
3. **Spirit S1 mouth:** Watch Soul Snatch's lunge and large mouth at impact, especially at five marks. Existing mouth pixels are centered on the target torso and play for0.6s, even when damage is processed later. A successful five-mark hit should consume stacks and heal once. Spirit S1 does physical burst/max-Health bonus damage; it does not instantly devour a camp. Human S1 still devours ordinary jungle camps.
4. **Timing:** Spirit S1 can be used earlier as a gap closer, then has a10s cooldown. Five marks do not bypass cooldown. Human S1 retains12s/60% successful-hit refund. Returning before pending spirit confirmation must not heal the restored human again.
5. **Tank role:** In0.7.3 starting-selection/draft/champion screens, Franco, Johnson, Belerick and Uranus should appear under Tank. Their skills/stats are unchanged. Alpha, Aldous, Balmond and Chou remain Melee bruisers. Verify portraits/selectability.
6. **Buff:** Base Attack90→100, growth5→6; base HP700→770, growth90→95. S1/mist coefficients increase; cooldowns/energy unchanged. Assess comparable matches before balance judgment: items, opponents and save auto-patches vary. First pass, not a measured win-rate claim.
7. **Regression:** Movement/recall, S2 path/airborne, trail marks, ward/body/link, projection/return, portraits and other heroes remain intact.

535 Rust tests pass, including a five-mark AI-selected spirit strike with deferred damage, visible mouth before damage, one-time heal/mark confirmation, no zero-stack heal/human refund, return cleanup and correct bonus-Attack baseline. Pixel audit counts mark lights; role/text/data/package checks pass. No live0.7.3 client here. If a check fails, send current log plus item/form/target; S1_COMPLETE includes spirit=true/false. Native Tank parsing/filter and real tooltip/mouth rendering require owner confirmation.
