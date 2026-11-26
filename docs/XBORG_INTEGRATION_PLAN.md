# X.Borg integration plan

Start from the owner-confirmed **MLBB Heroes 1.1.197** source and Windows package. Build the X.Borg candidate separately as **1.1.198**, hero 31. The previous archive is the rollback and regression reference. Use the Phase 0 numbers and rules, the owner’s animation brief, and the actual Dropbox sheets. Run Windows acceptance on TFM2 0.7.4.

[Phase 0 kit and balance](https://app.notion.com/p/4239c01d37c98167b6eece635040d087) · [Owner motion brief](https://app.notion.com/p/4239c01d37c981f388a6d97b805c9a6a) · [Owner asset folder](https://www.dropbox.com/scl/fo/bmuwdhxqfujknvfeow2fo/AP-JqKJJRiV7iVVsZgJ3qF8?rlkey=jtmdq7zo4xjhtyrbvz7yxe9c8&dl=0) · [1.1.197 source and SDK](https://www.dropbox.com/preview/Shien%20Ken/MLBB_Heroes/02_full_source/MLBB_Heroes_TFM2_v1.1.197_v30_full_source_with_SDK_Balmond.zip)

## 1. Establish the art source and frame map

Downloaded and hash-checked **33 transparent PNGs**, comprising **22 character sheets and 11 VFX sheets**. The character files contain the red armored and dark armorless forms described in the owner brief. These PNGs are the hero's visual source. Keep their originals with names, dimensions and SHA-256 hashes so every compiled clip can be rebuilt.

The body grids use 72×72 source cells. Count occupied cells in row-major order and record the actual ranges. The character groups below account for all 22 sheets. Ignore empty grid padding and preserve intentional hold frames.

| Owner files | Occupied frames | Runtime use |
|---|---:|---|
| Armored idle, run, basic | 9 each | Primary idle, movement and contact attack. Use a slower cadence of the run cycle for armored walking as needed. |
| Armored hurt | 5 | Brief hit reaction. The final grid cell is blank. |
| Armorless idle, walk, basic | 9 each | Slim form's idle, gait and ranged presentation. Use the walk cycle at faster cadence for running. |
| Armorless hurt and death | 5 / 9 | Slim hit reaction and `dead` animation. Use the provided armor breakup cue for an armored lethal hit and the supplied death sequence after it only if the transition remains visually coherent. |
| Armored S1 start and loop | 9 each | Short ignition followed by sustained body loop. Select the sustained portion of the loop after inspecting the first frames. |
| Armorless S1 start and loop | 9 / 5 | Shorter ignition and repeatable slim-form firing motion. The final cell of the loop grid is empty. |
| Armored and armorless S2 cast and recall | 9 each | Four body clips. Play recall only if it can be shown without interrupting a higher-priority action. The five stakes still return on time. |
| R charge, spin and explosion | 9 / 9 / 13 | Charge into spin, continue turning after dash contact, then show the combined blast/ejection clip. Trim the opening spin poses from the explosion sheet at the actual cut so the body does not visibly spin again after detonation. |
| Firaga break and restore | 13 each | Full-to-slim and slim-to-full transitions. Align the last body pose to the respective form's idle and keep the actual swap tied to the gameplay event. |

Set one foot pivot per form, preserve displacement inside each action, and align both forms to the same in-game ground point. The idle silhouettes in the supplied sheets occupy roughly 48 and 50 pixels. Normalize each form once from its idle master toward 45 pixels with nearest-neighbor scaling, then keep that transform throughout the form's clips. Check the transform endpoints against both standing poses. Give overshooting arms and debris additional transparent canvas room. Never crop visible pixels to force the old idle-cell size. Preserve the burner side and orientation in the source turn sequence. Verify both screen directions with the model's asymmetry visible.

### VFX map

| Owner VFX sheets | Use and origin |
|---|---|
| Basic projectile and projectile impact | Attach the projectile to the armorless basic's launch socket and the impact to the struck target. Preserve the normal basic hit/proc path. The outgoing art follows the chosen target direction. |
| Armored and armorless S1 flames | Use separate muzzle-anchored, direction-aware streams. Keep the broad armored stream and long narrow armorless stream. The armored sheet progresses from muzzle sparks to wide fire. Select a contiguous sustained subset for the loop. The armorless sheet already offers nine full streams. |
| S2 fire stake and recall trail | Instantiate one stake per path, five in all. Orient the bright head with travel, plant at the end point, then attach the trail on return. Use the projectile impact sparingly on real contacts. |
| R fire loop, overheat and explosion | Follow the current actor with the perimeter, synchronize the arc with spin, use overheat only during buildup, and fire one explosion at the actual detonation position. Shorten or dim the effect for forced break while showing the true hit radius. |
| Armor break and Firaga supply | Burst plates at the armor break location. Anchor each supply at its actual ground position, then remove it in the same tick it is collected or expires. |

Several body sheets already contain bright effects. S1 contains baked flames, the R spin contains a baked ring, and the R ending and passive break contain built-in flashes or debris. Inspect each overlap from the source cells. Use one main rendering layer for each flame, ring or blast. If a clean layer split is possible without removing body pixels, use the separate effect for directional alignment. If the parts overlap, keep the owner’s composite and use the separate asset only for a compatible accent or a diagnostic preview. The art audit records each choice. In particular use the genuine rotating character poses throughout R, so turning never depends on the perimeter effect.

**Gate.** A contact sheet shows every retained frame at native scale and each paired form. All emitted frame rectangles fit the resulting PNGs. Neither form jitters around a moving anchor. Only intentional blank or hold frames survive. All source files map to a runtime clip, layered effect or a documented alternate.

## 2. Connect the character to the game

Add one `mlbb_xborg` native champion entry, localization and skill labels, draft portrait and original icons derived from the supplied work. Register a dedicated Rust module while keeping the normal TFM2 player AI, lane, target selection, farming and recall. Use the same `.fanim`, `#sheet.png`, `.data_champion` and asset paths that ship for the other heroes. Keep generated assets and bindings in one compiler so frame names, pivots and durations stay in sync.

Try the existing Hanzo-style actor-bound body binding for armored, armorless and active clip transitions. A single body layer should follow live X.Borg during movement, dashes and client replay. Keep a valid draft sprite. Check the client-side lifetime on removal or death. If the base/view layer cannot express a pose correctly, the existing Yu Zhong positional sampler gives a proven loader pattern to test in isolation. Use phase and facing to choose the clip and use the active sim tick to choose its elapsed frame. Establish idle/run, basic/hurt and a short triggered form swap before wiring more abilities.

**Gate.** Champion schema, icon/asset lookup and mirrored frames validate. One visible actor stands at the correct world location in either form and is visible in draft and match. No frozen idle layer remains under a skill animation.

## 3. Prove Firaga state and shield behavior

Keep per-life state for form, shell budget and energy. Expose clean events for break, recoil and rearm, and reset pending events, heat and supplies on death/respawn. Implement four heat stacks per target, three-second decay, true skill damage after the fourth hit, and capped owner-only supplies on champion and regular camp targets. Pickup restores the correct resource once. The shell itself is 30% of body base HP with level growth, and rearming restores 30% capacity. A deterministic transition test covers full and partial absorption, stacked ally/item shields, lethal overflow, repeated hits in one tick, and R spending only Firaga.

The installed stable SDK exposes the combined shield value and a clear-all operation. It does not document a per-layer read/remove handle. Audit `WithShield` and exhaustion callbacks in 0.7.4 before using either as ownership proof. Probe the ordering on physical and true hits and on damage fully absorbed by a shield. No use of `entity_clear_shield`, post-lethal healing, global immunity or a fake revival. Develop and test the ordinary combat, pose transition and separate-resource logic independently. If ownership cannot be established, keep the build explicitly experimental and report the precise missing engine behavior. A named damage-reduction fallback with a different absorption model is a design change requiring its own test and approval. [Team Samoyed stable SDK](https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/stable-api-reference.md)

**Gate.** Firaga’s charge, break and spend are deterministic. Angela and item shields persist according to the engine's normal rules, and one lethal hit yields one correct death entry.

## 4. Connect Fire Missiles

Bind a directional moving cast with X.Borg's normal AI, use a short anticipation pose, lock aim, and run the owner-supplied start and body loop for the active phase. Sample the actor position on each of six pulses across 90 ticks. Use the correct 50,000 / 70° or 65,000 / 36° shape and armorless 60% damage. Route physical and hot true damage through the native pipeline. Keep the muzzle alive for the timed buff rather than the short cast animation. Break transfers the active loop to armorless art and geometry while preserving elapsed time. Stun, airborne, suppression and death remove remaining pulses and flames.

**Gate.** The AI casts when a target overlaps its forward cone, it can walk and keep spraying in either facing, and hit position and rendered flame agree. The fourth hit adds the last heat stack and later eligible pulses convert once each.

## 5. Connect Fire Stake

Implement launch and return as distinct events. Use five explicit rays and wide/narrow spread based on the form at cast. Associate each shot with the launching life and cast. At 72 ticks, sample the caster position and resolve the return paths with deterministic deduplication. Gameplay applies one hit, one capped pull, and at most one collection per supply across the five paths. Launch and return graphics follow real positions. When return overlaps R, resolve damage and pull on time and defer supply collection. Body recall never resets S1 or R.

**Gate.** Five stakes appear in both forms, reverse at the same simulation time, and overlap cannot multiply damage, pulls or supply recovery. Moving before recall changes the convergence point. Neither enemy towers nor allied units trigger the attack.

## 6. Connect Last Insanity

Use an armored-only AI and execution gate. Begin turning as the short terrain-checked dash moves the actor. Stop dash translation on first champion contact and keep the spin clock and owner-supplied body loop running. Apply six pulses and slows across 120 ticks. The ring follows the actor. Test ordinary movement during the spin after contact and keep its visual clock independent of the facing and gait decisions. Timer, early trigger and enemy armor break enter one resolver. Break wins a same-tick tie and gives a half-strength detonation. Normal and early detonation spend Firaga, apply one fixed-damage blast with the epic cap and finish in armorless form. Use the separate blast or the compatible composite from the audit. End events and death stop spin and flame cleanly.

**Gate.** The body visibly rotates while moving and after contact. Frame index changes throughout the live spin. Normal, early and forced ending each show one blast and one armorless transition. A live client check is mandatory because a compile or sim event cannot catch a missing body loop like Balmond’s previous regression.

## 7. Audit and package

Run source and asset checks, the complete Rust suite plus X.Borg state regressions, and the existing Windows GNU toolchain. Verify the package manifest, matching champion definitions, Windows DLL export and archive CRCs. Compare all pre-existing installed assets, champion definitions and mod overrides against 1.1.197. Spot-check Balmond's working cyclone VFX and movement. Prepare matching Windows and source + SDK archives with hashes and a short acceptance checklist. The source archive includes the intake hashes and art mapping. Keep an experimental candidate clearly labeled until the real Windows match validates AI casts, render lifetime and the shield overlap test. Publish only after the usual owner acceptance route.
