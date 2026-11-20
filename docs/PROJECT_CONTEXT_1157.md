# MLBB Heroes — current project context, 19 November 2026

- Active owner-test candidate: internal 1.1.157; 25 heroes; TFM2 0.7.3+. Gameplay rollback 1.1.156, known hover defect.
- Public v2.5.0 Granger is published. Next planned public release is v2.6.0 — New hero Hanzo, now 25 heroes. Owner publishes personally. The isolated v2.6.0 preparation tree still contains 1.1.156 and must be refreshed only after this new candidate is accepted.
- Feedback: Popol and Granger vanish on hover; owner confirms Hanzo too. Native visible preview tags plus explicit transparent match Animation state implemented. No UI scanners. Test hover, duplicate bodies and normal movement/recall first; exact native engine behavior not proven by mocks.
- Granger buff: S1 28 + 40% Attack; ultimate 130 + 105%. Energy costs/regen/cooldowns/homing unchanged. Sixth shot retains native crit pipeline; crit-damage-item scaling requires a controlled live check. No manual crit multiplier.
- Hanzo gameplay/AI/combat remains byte-identical to delivered 1.1.156. Preserve native Move/Return/None passthrough. Its accepted S1/body/link/marks/ward/return fixes and buffs are retained. Tank categories remain Franco, Johnson, Belerick and Uranus; their gameplay stats unchanged.
- Canonical Rust: sdk/candidate. New shared helper match_carrier.rs. Finish legacy sprite compiler runs with tools/compile_native_previews.py. No character PNG edits or paid generation.
- Validation: 537 Rust pass, one inherited ignored; four loader tests; 100 root schema checks; Windows x64 build. Exact scope, limitations and pending tests: SEAWAY_1157_BUILD_REPORT.md and SEAWAY_1157_PLAYTEST.md.
- Dropbox /Shien Ken/MLBB_Heroes: 01_windows_candidates/mlbb_heroes_v1.1.157_windows_candidate.zip; 02_full_source/MLBB_Heroes_TFM2_v1.1.157_full_source_with_SDK.zip; 04_docs/SEAWAY_1157_* and PROJECT_CONTEXT_1157.md.
- Notion is project memory, Dropbox holds artifacts. Standing permission to upload to Dropbox. No Drive writes. No publishing/messages to others.
- Neutral third-person skill descriptions with existing color/icon syntax. Sprite target45px, maximum48px; preserve owner pixels. Never restore per-frame UI-tree scanning or custom Hanzo movement/chase/recall logic. Gusion/Uranus remain gameplay benchmarks.
