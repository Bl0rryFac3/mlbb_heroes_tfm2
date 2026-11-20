# Angela — corrected VFX bases and PixelLab motion prompts

Reviewed 12 November 2026. Character motions are complete according to the owner. VFX production remains pending.

## Production contract

Use SpriteCook Pixel Engine Image v1 when the owner requests generation. Each generation produces ONE isolated base frame on a genuinely transparent RGBA background. Animate that base separately in PixelLab. The frame counts below are animation targets, not instructions to generate a strip.

Keep the existing PixelLab settings as project presets — Reference Init Strength 86 and Faithful mode on. Output Scale ÷10 applies only to the previously used enlarged input workflow. A native-resolution image must keep one image pixel per art pixel. Inspect input dimensions before rescaling.

Canvas sizes below retain the previous guide's budgets. They are proposed production cells, awaiting final effect-scale verification in the mod. Keep transparent margins and a fixed origin. Use crisp pixel clusters, a limited palette and intentional alpha. A faint blank spark is a poor animation reference; the base should contain the recognizable effect silhouette.

The palette is a project adaptation. The source sprite's pink, turquoise and gold do not prove that every default-skin effect uses all three colors. Pink and pale highlights suit the proposed love effects. Detailed default-skin hue matching remains pending a clear close-up.

## Evidence and confidence

The official 2024 Angela spotlight was located. Its first storyboard page was inspected and shows directional S1 targeting and gameplay. The full video stream was unavailable, so exact frame timing and fine particle shapes remain unverified. A targeting reticle is UI and must be excluded from the effect art.

Ability references support a directional damaging/healing wave, a persistent S2 connection followed by a conditional root, a protective ally shield, and a doll above the attached ally. The precise crescent contour, tether colors, root ornament and marker layout below are explicit pixel-art proposals. They should not be described as traced default-skin assets.

Sources are listed at the end. Skin-specific clocks, hourglasses, Hello Kitty symbols and cyber patterns belong to their respective skins.

## 1. angela_love_wave — S1 moving wave

Cell 72×72. Animation target 8 frames. Proposed silhouette: a broad curved wave front with a short trailing glow. This replaces the old cloud of overlapping hearts and compulsory turquoise ribbons.

**Base-frame prompt**

> One isolated pixel-art magic wave for Angela's Love Waves, viewed from the game's elevated three-quarter perspective and facing canvas-right. A broad gently curved pink wave front, bowed toward the right, with a pale rose-white leading rim and a short translucent rose trail on its left. One restrained heart motif integrated into the center of the wave. Clear, coherent silhouette with open transparent space around it. Keep the entire wave inside a 72×72 transparent canvas. Crisp native-resolution pixel clusters and a limited pink palette.

**PixelLab motion prompt**

> Animate the supplied wave in place over 8 frames. Let the pale leading rim brighten, send a short shimmer across the curved front, and let the small trailing pixels dissolve and renew. Hold the base orientation, origin and travel footprint steady. Preserve its recognizable contour and transparency. The game supplies forward movement.

**Integration**

The visible wave must advance from the cast origin through the affected area. A caster-bound effect alone can only work if its animation explicitly contains that travel. Prefer a travelling effect/projectile binding, with width, collision and visual timing checked together. Damage enemies and heal eligible allies through the same pass, using explicit team filtering. Confirm the engine route before final export. The exact native MLBB hitbox is not established by the ability text.

## 2. angela_heart_bolt — basic attack

Cell 40×40. Animation target 5 looping frames.

**Base-frame prompt**

> One compact pink heart-shaped magic projectile facing canvas-right, with a pale center and a very short rose light trail extending left. Keep its heart outline clearly readable at tiny game scale. Center it with generous transparent padding inside a 40×40 canvas. Crisp pixel art, limited pink and pale-white palette, transparent RGBA background.

**PixelLab motion prompt**

> Gently pulse the heart's brightness across 5 frames and let the short trail flicker. Keep the heart upright, the direction fixed and the total projectile length stable. Return smoothly to the initial state.

Bind to the basic-attack projectile. Review the completed attack motion for a baked-in projectile so the moving heart appears once.

## 3. angela_puppet_string — S2 connection

Cell 40×40. Animation target 8 looping frames. A horizontal reusable segment is easier to rotate or repeat than the former diagonal cord. Endpoint alignment and seam handling need engine verification.

**Base-frame prompt**

> A fine horizontal luminous puppet thread centered in a transparent 40×40 canvas, running left to right. A warm pale-gold core with a restrained rose fringe, a clean continuous line and a tiny travelling glint. Keep both ends straight and compatible with adjoining segments. Crisp pixel-art thread with transparent space above and below.

**PixelLab motion prompt**

> Move a faint glint along the thread over 8 frames while keeping the cord continuous, endpoints fixed and thickness stable. Make the loop seamless.

Warm gold/rose is an adaptation pending a default-skin color close-up. Render the connection between the current caster or attached host and the target. Update its length and direction as they move. The proposed 180-tick connection is controlled by gameplay, not one 8-frame playback. Hide it on break, death, cancellation or successful completion. Specify and test the break-distance rule in implementation.

## 4. angela_immobilize — S2 successful root

Cell 72×72. Animation target 6 frames for onset, with a separately held state if needed.

**Base-frame prompt**

> One compact rose-pink binding effect centered around an empty target space, viewed from an elevated three-quarter angle. A thin elliptical light ring near the ground and two fine puppet-thread arcs curving upward toward the target's lower body. A small pale highlight at each meeting point. Preserve a clear center so the victim remains readable. Crisp pixel art on a transparent 72×72 canvas.

**PixelLab motion prompt**

> Tighten the two light-thread arcs toward the center, brighten their meeting points briefly, then settle into a restrained binding glow. Keep the center clear and the ground ellipse fixed.

This is a simplified mod root cue. The old turquoise cage, four gold pins and white-hot opening flash were unsupported embellishments. Trigger only if the connection succeeds. Root duration follows the approved 42-tick mod value. A short onset animation does not set the gameplay duration.

## 5. angela_lovers_mark — S1 stack indicator

Cell 36×36. Animation target 5 looping frames.

**Base-frame prompt**

> One small upright pink heart status marker with a pale central highlight and a clean dark-rose edge. Center it in a transparent 36×36 canvas with generous empty space. Crisp pixels, simple readable shape.

**PixelLab motion prompt**

> Pulse the heart very gently over 5 frames. Keep its size, orientation and center fixed, with a smooth transition back to the first frame.

Use one icon with the game's verified stack display, or explicitly position repeated icons if the implementation supports that layout. The former statement that the engine automatically draws N separate hearts is unverified and must not guide asset generation. Stacks and damage/slow behavior require their own implementation tests. Orbiting rings are optional adaptation, not established default-skin detail.

## 6. angela_heartguard_shield — ally protection

Cell 72×72. Animation target 6 looping frames. Use a full enclosing bubble silhouette, with an open center for sprite readability.

**Base-frame prompt**

> One softly rounded pink magical shield bubble viewed from an elevated three-quarter angle, centered around empty space for an allied hero. A thin luminous rose rim with pale highlights around the upper sides and a subtle lower elliptical rim. The interior is almost fully transparent. Use a restrained soft-pink tint and crisp pixel clusters inside a transparent 72×72 canvas.

**PixelLab motion prompt**

> Gently circulate a highlight around the bubble rim and pulse its brightness over 6 frames. Keep its shape and center steady, with the final frame transitioning smoothly into the first.

Bind to the actual shield state on the host. Show an arrival flash separately if desired. Fade or remove when the shield expires or is exhausted. Attach indication must follow attachment state independently, because shield and attach lifetimes can differ in the approved mod design. Decorative heart tiling and a gold halo are optional art choices, not verified canon requirements.

## 7. angela_attached_doll — Heartguard rider indicator

Cell 40×40. Proposed doll content height 18–24 px, subject to game-scale review. Animation target 3 subtle looping frames.

This replaces the former optional angela_attach_link. The canon reference explicitly describes a doll above the allied hero. The previously proposed line between distant allies is unnecessary for the baseline design.

**Base-frame prompt**

> One tiny seated Angela puppet indicator for her default Bunnylove appearance, matching the supplied Angela master as a style reference. Preserve recognizable rabbit ears, brown bob, pink accents, white outfit and compact doll proportions. Arms relaxed, legs tucked, clearly readable silhouette. Center it inside a transparent 40×40 canvas with generous padding. Crisp native-resolution pixel art.

**PixelLab motion prompt**

> Animate a very small idle sway and one-pixel ear movement over 3 frames. Keep the doll centered, recognizable and consistently sized. Make the loop gentle.

Anchor above the host through an explicit overlay offset. Its existence does not prove that banish, camera behavior or casting while attached works. Those remain implementation/playtest questions. The doll can be prepared as art while that mechanism is tested.

## Optional reuse

A small rose pulse from Love Waves can be reused as an ally-heal confirmation if the engine needs one. Any healing flash should follow a successful heal. Avoid adding another required package without an actual readability need.

## Review against completed motions

The owner has completed the motions. Their actual animation sheets were not supplied for this review, so animation quality and alignment remain uninspected. Before integration, check attack/S1 for travelling art already baked into the character frames and S2 for a fixed-length drawn string. Bind the world-space VFX once. A short hand glow can stay in the character animation.

## Export checks

- One base frame per generation, Pixel Engine Image v1, genuine alpha.
- Inspect native pixel resolution before applying the historical ÷10 preset.
- Fixed cell size and origin across each animation; complete bounds with padding.
- Continuous S2 connection with correctly updated endpoints.
- Clear shield center and host-following doll.
- S1 travels consistently with its hit/heal timing.
- Frame-count proposals do not override gameplay duration.
- Verify effect tags against the actual fanim animation keys during integration.
- In-game preview at final size on both bright and dark arena areas.

## Sources and remaining visual work

- Official Mobile Legends spotlight, 11 March 2024. Skills begin at 0:05. Storyboard observations are limited-resolution evidence. https://www.youtube.com/watch?v=eM6SWvfJfEo
- Angela ability text and doll/basic-attack trivia. https://mobile-legends.fandom.com/wiki/Angela
- Recent ability values for comparison. https://liquipedia.net/mobilelegends/Angela
- Ability summary and gameplay screenshot. The screenshot mixes targeting UI with gameplay and is insufficient to establish default-skin color details. https://esports.gg/news/mobile-legends-bang-bang/mlbb-angela-build-guide-best-items-skills-emblem-combos-and-counters/
- Skin-specific effects differ substantially. https://www.oneesports.gg/mobile-legends/avatar-of-time-angela-skin/

A clear default-skin S1/S2 close-up is still needed to certify exact particle silhouettes and hues. The prompts above are usable corrected adaptations with that limitation explicitly recorded.


## Integration checkpoint — 13 November 2026, v2.1.0-rc.2 / 1.1.118
Owner supplied replacement S1 art in Drive (1rn_ifbsUuJ9gwhbsJp5QkU7mPS5Fdm-G), 300×300 RGBA, 3×3 row-major cells. This supersedes the old S1 visual, not the completed character motions. Original PNG is retained in source/angela/new_love_wave_original.png.
The importer preserves each cell's common origin and uses nearest-neighbor scaling/rotation. Source motion goes upper-right toward lower-left; 16 travel directions use that orientation. Nine short frames advance with the native wave over 27 ticks.
S2 uses Angela's supplied thread normalized into twelve directional tiles and refreshed every two ticks from live endpoints, following the shipped Estes approach. It is not a fixed-length character-frame tether.
The supplied doll is padded into an above-host overlay and refreshed separately from the six-second shield. Assets alone do not prove attachment; see ANGELA_SPEC.md's current implementation checkpoint for banish, cooldown-display and playtest limits.



## rc.3 playtest repair — 13 November 2026 / internal 1.1.119
Owner tested rc.2: S1 and S2 function, speed is excessive, and Angela never uses Heartguard. This is partial in-game evidence, not attachment verification.
The rc.2 repair misread BuffV1 percentage units. Smart Heart's raw 75 × four meant +300%, not +30%. Fixed with 7/8/7/8 whole-percent contributions; cumulative bonus is 7/15/22/30%. Mark slow is -6 per stack; S2 slow ramp is capped at 45, not 450. Other damage/timing/art are retained.
Heartguard's Native effect incorrectly reported shield as expected_damage, while expected_shield defaulted to zero. It now reports expected_shield = 220 + 180% AP. Targeting/AllyChampion is valid (Estes uses the same pair). A proactive AI rule selects a living targetable allied champion, excludes self, prefers low health, and requires enemies within 150,000 (or within 300,000 when the ally is at or below 65% HP). Engine level lock, charges, native cooldown debt and input validity are respected. Already protected hosts and quiet allies are skipped.
New tests exercise the actual AI callback with no base action, self-target redirection, engine/native gates, rejected inputs and percentage sums; they do not establish live Heartguard/attachment behavior. Retest speed and ult activation at level 5+, then validate attachment, host-origin skills and return position.
