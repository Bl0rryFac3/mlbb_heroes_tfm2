# Balmond V10 original-sprite scale test — 2026-10-31

V10 restores Balmond's original 201×122 Ult slash art and `balmond_ult_fx` binding. It keeps the user's requested 110,000-unit directional line and uses a test width of 36,000 as an intermediate value between V8's 60,000 (reported too broad) and V9's 16,000 (reported too thin).

The provided mockup is a 2D fit of the original sprite to a 110,000×36,000 rectangle. It is not rendered by TFM2 and does not establish the client's actual size, orientation, alpha, or animation behavior.

No charge or airborne effect is added. The visual-only line uses `apply: 1`, `delay: 30`. The existing Ult fan/native execute/range/targeting/damage and V8 S1 reduced knockback are unchanged.

Only an in-game test can confirm whether 36,000 looks right. V10 is an intermediate runtime candidate; no visual success is claimed until the client screenshot is reviewed.
