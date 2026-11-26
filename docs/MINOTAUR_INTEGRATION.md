# Minotaur integration · 1.1.203

This candidate adds hero 32 on top of the 1.1.202 compact-voice baseline for Teamfight Manager 2 0.7.4. The Windows install ZIP and full source + SDK ZIP use the same DLL. Keep 1.1.202 as the rollback.

## Art and runtime bindings

Sixteen owner-supplied PNGs are preserved under `source/minotaur/received`. `source/minotaur/frame_map.json` records their SHA-256 hashes, grids, selections and output timings. Run `python tools/compile_minotaur.py` to rebuild the native fanim atlases, draft frames, icons and view bindings. The single body atlas uses a 96 × 88 canvas with a common (48, 66) ground pivot. It preserves the approximately 48 px standing sprite and uses normal engine facing. Nine idle frames also drive the draft preview. The walk provides the movement loop. The icon set uses the supplied art.

| Clip | Runtime connection |
| --- | --- |
| Basic | Contact on frame 6 at tick 15. Normal melee basic reaches towers and the Nexus through the engine-selected target and base attack damage path. A small gold target spark distinguishes the two S1-enhanced basics. |
| S1 | Source frame 7 lands at tick 24. The leap moves toward a wall-clipped point, aborts on displacement or hard CC, and resolves all eligible enemies in the landing circle. The same ground sheet covers 22,000 and 28,000 radii. |
| S2 | Source frames 3 and 4 depict a downward weapon strike and were omitted. Frame 5 gives the open-jaw pose at tick 12. Play the green wave at Minotaur and the recipient burst at each selected wounded champion. |
| R | The early sheet uses contact frames 7 and 15. The final sheet skips its two opening echoes of the previous impact and uses frame 7 for the last slam. The stitched cast lands on ticks 18, 48 and 78 and ends on 84. The early impact plays twice. The final impact has its own brighter earth fracture. |
| Enraged | A warm-gold variation of the supplied aura accompanies the 84-tick cast. The red loop attaches afterwards for the rest of the nine seconds, measured from activation. The original body clips serve both states. |
| Motivation | One small gold pulse at skill activation and a green recovery cue when the passive pays. |

All body and cast sheets are from the owner. Only selection, padding, scaling, PvE-safe event binding, and minor separate effects were needed. The enhanced spark was animated from the previously generated basic-hit starter. Existing voices were left at 136 cues and 136 WAVs. This candidate has no new Minotaur voice clips. Native ABI offers a Melee category and Tank tag, with a Support lane prior. The data roster uses Tank.

## Verification and live checks

Automated checks exercise the S1 movement and impact timing, out-of-range and wall handling, normal and Enraged hit radii, two enhanced basics with ordinary structure damage, S2 target ordering and radius, three independently sampled R impacts, and the healing/peel/group AI decisions. Static checks resolve all eight body tags, ten effect tags and nine draft SVGs. The Windows DLL and both ZIPs are built for handoff. The staged asset preview uses the compiled clips and contact ticks.

The stable damage callback supplies source and amount but leaves shield accounting and exact callback order unspecified. The candidate caps eligible recovery at observed body HP loss and defers a notification that arrives before the current tick's HP change. A sample with mixed eligible and ineligible receipts receives zero passive recovery if their shares cannot be proved. This can undercount simultaneous incoming damage or damage offset by healing between samples. To inspect the callback order in a Windows match, set the process environment variable `MLBB_MINOTAUR_RECEIPTS=1` before launching the game. The mod log records the attacker kind, amount, callback HP, previous sampled HP and visible shield.

1. Open draft and hover Minotaur in both pick states. Check the idle changes frames and the hero's five icons resolve in the expected views.
2. Check a short in-place S1, a long leap, and an initiated leap that meets a wall or receives hard CC. Confirm the rock pulse stays at the actual landing. Check several enemies at different angles and compare ordinary and Enraged reach.
3. After S1, check two ordinary-unit enhanced basics and an intervening tower or Nexus basic. Confirm that the hammer swing plays for both directions and the structure does not consume an enhanced hit.
4. With different max HP values on two allies, confirm normal S2 heals the lower HP percentage. During Enraged, check two wounded allies and one just outside the radius. Verify that heal reduction and engine health limits still apply.
5. Check R with two enemies inside and then with one enemy pressuring a wounded ally. Move a target out of the circle after the first pulse. Confirm two early flashes and one final launch, control immunity until the cast ends, and post-cast aura expiry. Probe suppression separately on the host.
6. With Motivation active, compare unshielded hits, fully absorbed hits, partial shields, same-tick hits with mixed sources, a lethal hit, and respawn/replay cleanup. Pay special attention to any engine callback delay or shield HP behavior.
