# Julian audio — 1.1.134 hotfix candidate

The current implementation is sdk/candidate/audio.rs. Julian keeps the working path-based asset call, wrapped in client pre_update/post_update and viewer tick observations from the existing match hook. Legacy silence/guard proposals are historical, not current instructions.

Validate in TFM2: no pre-match chatter; pause for at least 10 seconds (an already-started clip may finish, but no new line); resume for fresh movement and skill cues; fast-forward and scrub without a burst; test another voiced hero in the same match and two Julians. Read [VOICE frame-request] and [VOICE frame-play] rows in mlbb_heroes.log. Check the loaded banner says 1.1.134.

The SDK has no arbitrary sound pause/stop API. This change controls new starts; immediate silencing of an already-playing clip would require host support or a validated engine sound-event route.
