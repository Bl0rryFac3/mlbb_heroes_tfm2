# Hanzo 1.1.153 — native movement and S1 retest

This is a candidate, not a confirmed fix in the game. Owner rejected1.1.152 after standing, objective/recall jitter and backwards walking; head marks were visible but bite/devour unseen. The supplied log proves five basic requests then human S1 at tick680, not successful devour completion.1.1.149 remains the accepted rollback.

Close TFM2, replace the installed mlbb_heroes folder with this candidate's folder, keep one enabled copy and start a fresh match.

1. **Native movement and recall first.** Before level5, watch Hanzo leave base, move between ordinary camps/lane, and recall. He should continue the normal engine's task/recall behavior instead of repeated stationary moves or custom escapes. Watch again when the Turtle/Morgard/Lord objective appears. The hook no longer rejects objective movement or requires two nearby allies; the native AI makes those decisions. Judge the comparison against ordinary native behavior, not a promise that the native AI never takes a bad fight.
2. **Facing.** Watch ordinary left/right walking at normal speed in both forms. The run/move sprite should face the travel direction. Attack/skill motion can retain its cast-facing briefly. Check actual basics still animate and damage.
3. **Human S1 and devour.** On the native AI's current ordinary jungle target, observe five marks, then the bite. The bite now plays on an accepted cast; digestion begins only after a later death read or credited jungle-kill receipt. A confirmed devour should produce four5%-max-HP digestion heals at0.5s intervals. Ordinary camps only: lane minions, summons/pets and bosses are not instant-devour targets. Human S1 on a marked champion/boss is the existing burst/heal/refund, not an execution. S1 never resubmits its damage while waiting for confirmation.
4. **Preserved behavior.** Confirm ult transformation, spirit basics/S1/S2, parked human body/link, visible head marks, normal return and defeated-spirit return. Energy remains as before. Check selection/draft/ban portraits still display.

If a check fails, send the current mlbb_heroes.log and the form/target/event. In the viewed match, bounded Hanzo traces distinguish S1 cast, S1_COMPLETE (ordinary/credited/digest), S1_UNCONFIRMED and S1_SUBMIT_FAILED. BASIC inline_hp_delta is only a snapshot observation and can be false even when the host processes damage later.

Automated evidence is limited: it verifies no Move/Return/hold/escape output from this hook, left/right facing at1084 movement units, queued S1 completion and one-time rewards, preservation and package integrity. It does not run the real pathfinder, recall channel, renderer or a full TFM2 match. Retest the movement/recall item before assessing combat balance.
