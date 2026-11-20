# Eudora — VFX base frames and PixelLab motion prompts

These are first-frame assets and animation instructions, not finished animations or installed game effects. Animate them in PixelLab and return the outputs for compilation and binding.

The palette follows the inspected revamped references: white-hot lightning, cyan/blue energy and restrained violet accents. These are readable pixel adaptations; exact original particle geometry and frame timing were not certified from a full official video.

## Source art and reuse

Four generated VFX designs cover the kit. A free 90-degree SpriteCook rotation provides the vertical bolt. A crop of the horizontal thread provides a link segment without its enlarged endpoint. All source images remain available; crop/padding derivatives do not repaint pixels.

| Source | Original canvas | Animation input | Use |
|---|---|---|---|
| `eudora_lightning_thread.png` | 48×16 | `eudora_lightning_thread_padded.png`, 72×32 | Basic projectile |
| `eudora_link_segment_crop.png` | 24×16 | `eudora_link_segment_padded.png`, 48×32 | Repeated live tether segments |
| `eudora_forked_discharge.png` | 48×32 | `eudora_forked_discharge_padded.png`, 72×56 | S1 forward fan |
| `eudora_conductive_orb.png` | 32×32 | `eudora_conductive_orb_padded.png`, 48×48 | S2 orb; smaller mark and impact variants |
| `eudora_thunder_strike.png` | 16×48 | `eudora_thunder_strike_padded.png`, 48×72 | Ultimate descending strike; free rotated thread copy |
| `eudora_ground_ring.png` | 48×24 | `eudora_ground_ring_padded.png`, 72×48 | Ground warning and expanding discharge |

The generated orb occupies its 32×32 source rather than the smaller requested center. It is suitable for an ability projectile. The passive uses the same motif at a much smaller display size (roughly 10–12 screen pixels), not a full-sized orb covering the hero. The source ring's actual measured bounds are recorded in `art/asset_checks.json`; the compiler must bind its ground center rather than assume a filled rectangular image.

Use the native/padded files, not the enlarged inspection previews. Leave the added space available for sparks. If PixelLab adds padding, return it intact with its metadata. Maintain a consistent white/cyan/blue/violet palette and hard transparent edges. No character, crown, ice, water, clouds, landscape, cast text or weapon should appear in any VFX output.

Character motion already contains the casting gesture. These VFX must not add a second Eudora silhouette.

## 1. Basic bolt — `eudora_basic_bolt`, 4 frames, loop while traveling

Source: `eudora_lightning_thread_padded.png`.

**Motion prompt:** The horizontal white-and-cyan lightning bolt flickers internally while keeping the same full left-to-right length and fixed endpoints. Tiny violet branches alternate along the edge, and a brief brighter pulse travels through the white core. Keep the bolt centered, its main silhouette stable and all sparks inside the transparent canvas. Finish with a clean looping state. Do not move the whole bolt across the image, grow it from one side, rotate it or create an impact explosion.

Runtime: a traveling projectile moves the sprite. Its speed, collision and damage are engine/native behavior; the animation does not simulate travel. If a separate impact flash is desired, use variant 5 below.

## 2. Superconductor mark — `eudora_superconductor_mark`, 6 frames, loop

Source: `eudora_conductive_orb_padded.png`; shared visual motif at reduced display size.

**Motion prompt:** The small round electric emblem stays centered in place, with its central lightning symbol clearly recognizable. Its cyan rim brightens and dims gently, and a few small violet sparks flicker at alternating points around the perimeter. Keep the circle, lightning symbol and overall dimensions stable. Return smoothly to the starting brightness for a quiet loop. No forward travel, swelling, explosion, character or scene background.

Runtime: a small marker above the affected victim, not a world projectile or a halo over Eudora. Visible only while that caster's mark exists; expire/refresh with the five-second gameplay mark. Its loop length does not extend the buff lifetime. This orb-based marker is a deliberate shared-art adaptation, not a claim that the original MLBB marker is identical.

## 3. Forked Lightning fan — `eudora_forked_lightning`, 6 frames, one shot

Source: `eudora_forked_discharge_padded.png`.

**Motion prompt:** The branching lightning fan flashes sharply from its fixed narrow origin on the left toward its wide branches on the right. Keep every frame's overall reach and fan angle unchanged. The white cores flare, the cyan branches split into brief smaller zigzags, and the violet edge sparks blink out as the discharge fades to transparency. Keep empty transparent gaps between the branches. No solid cone, traveling wave, growing wedge, rotating fan or whole-image movement.

Runtime: place its narrow origin at Eudora's casting position and orient it with the cast direction. Fan hit selection uses the approved cone; the PNG's colored pixels are not the collision boundary. The initial fan is quick and instantaneous, not a one-second moving projectile. Prepare required direction variants during compilation; the client does not automatically mirror all effect pictures.

## 4. Maintained lightning link — `eudora_lightning_link`, 8 frames, loop

Source: `eudora_link_segment_padded.png`. This crop removes the enlarged tip from the projectile design; do not use a whole bulb-shaped projectile for every segment.

**Motion prompt:** A short horizontal lightning connection stays anchored at both ends. Its white core trembles in small angular zigzags, cyan highlights alternate between its branches, and a few violet edge sparks briefly appear and vanish. Keep the two attachment points and overall segment length fixed in every frame, with enough transparent space around the lightning. Loop cleanly. No drifting, growing from one end, orb, impact flash or camera movement.

Runtime: use the Estes/Angela technique to place short-lived, pre-rotated segments between the live caster and victim positions. Recompute endpoints as either moves. The loop itself remains horizontal; do not bake a particular pair of hero positions into it. Stop drawing immediately when the link breaks or an endpoint becomes invalid. A valid connection can last up to 60 ticks. Damage pulses at 15/30/45/60 ticks are a separate mechanic.

## 5. Small impact / completed link — `eudora_lightning_impact`, 6 frames, one shot

Source: `eudora_conductive_orb_padded.png`.

**Motion prompt:** The compact electric orb flashes white once, then opens into a short burst of thin cyan and violet lightning sparks radiating from its fixed center. The center quickly loses brightness as the separated sparks shorten and disappear, ending on a completely transparent frame. Keep the burst small and contained in the existing canvas. No lingering ring, smoke, stone fragments, ice, character or moving projectile.

Runtime: small impact on the struck victim; also the S1 finishing flash only when a link completes successfully. A broken link does not play the completion flash. Size this below the full S2 area burst. Reuse is intentional; it does not add another damage event.

## 6. Ball Lightning projectile — `eudora_ball_lightning`, 8 frames, loop while traveling

Source: `eudora_conductive_orb_padded.png`.

**Motion prompt:** The electric orb holds the same round silhouette and fixed center. The white lightning core pulses, cyan light circulates around the inner edge, and small violet sparks flicker around the broken outer ring. Keep its center symbol recognizable and preserve its diameter. Cycle smoothly back to the first frame. Do not move the orb across the image, change its size, explode it or add a character, hand, staff or background.

Runtime: a targeted orb follows the selected enemy until impact or invalidation. The stun and resistance reduction happen on the actual impact, not at the first animation frame. The projectile animation never includes the enhanced area impact.

## 7. Enhanced Ball Lightning area burst — `eudora_ball_lightning_burst`, 8 frames, one shot

Source: `eudora_conductive_orb_padded.png`. A larger padded burst preparation is supplied so sparks can spread without rescaling the starting orb.

**Motion prompt:** The round electric core flashes and breaks open at its fixed center. Thin cyan lightning forks spread outward in a circular burst with short white highlights and a few violet tips. The fragments separate and rapidly fade until the final frame is fully transparent. Keep the starting orb at the source pixel size and let only its emitted forks expand into the surrounding empty space. No shockwave made of smoke, ice shards, rock, character or camera movement.

Runtime: play around the impact position only when S2 hit a previously marked primary target. Bind its display extent to the approved 22,000-radius area. Unmarked S2 receives a small hit flash instead. Do not hit the primary twice merely because it lies inside its own explosion.

## 8. Ground warning — `eudora_thunder_warning`, 8 frames, loop

Source: `eudora_ground_ring_padded.png`.

**Motion prompt:** The thin cyan electric ground ellipse remains in exactly the same location and at exactly the same diameter. Short violet sparks crawl around its rim while white highlights pass along the outline. Keep its center completely transparent in every frame and keep the ellipse flat on the ground plane. Loop at a restrained brightness. Do not fill the center, add a vertical beam, expand the ring, tilt the camera or create an explosion.

Runtime: a ground-position effect with a clear center, not a buff glued to the enemy's current position. For delayed Thunderbursts, keep the warning at each seed's snapshotted hit position for the approved **36-tick delay**. Moving enemies can leave it. Any initial cast telegraph must fit the cast windup; do not silently add another gameplay delay.

The ring expresses a warning. The main and delayed strikes use the next two outputs; a warning loop is not proof of damage.

## 9. Descending strike — `eudora_thunder_strike`, 8 frames, one shot

Source: `eudora_thunder_strike_padded.png`, the free rotated filament. Its lower tip is the ground contact; the top is the incoming direction.

**Motion prompt:** The vertical jagged lightning bolt flares downward into its fixed lower contact point. Keep its top-to-bottom length and overall placement stable while the white core flashes, cyan side branches briefly widen and a few violet sparks scatter near the lower end. After the bright contact beat, the bolt quickly breaks into smaller electric fragments and disappears into complete transparency. Keep the impact point fixed. No rising beam, character, ground disc, smoke, ice, cloud, text or camera motion.

Runtime: align the lower contact point with the selected ground center, not with Eudora's center or the sprite's geometric midpoint. The vertical bolt is shared art derived from the basic bolt; the separate descending animation is what distinguishes its use. The initial strike and delayed Thunderburst reuse this asset at different event times; neither should remain drawn as a persistent laser.

## 10. Ground discharge / outer ring — `eudora_thunder_discharge`, 8 frames, one shot

Source: `eudora_ground_ring_padded.png`. A larger padded preparation is included for outward sparks, retaining the original source pixels.

**Motion prompt:** The hollow cyan ground ring flashes with brief white highlights, then sends short jagged electric branches outward along the ground. Its rim fragments into violet and cyan sparks that fade away, ending fully transparent. Keep the ground center fixed and keep the interior mostly open so a hero beneath the effect remains readable. Any expansion happens within the provided transparent margins. No filled disc, smoke wave, ice, stone, crown or character.

Runtime: use as the brief ground component under the initial strike, and optionally under a delayed Thunderburst. The initial inner and outer damage zones remain mutually exclusive even if the pictures overlap. Multiple seed visuals may overlap, but the approved same-cast delayed-hit cap still applies. Artwork must not trigger damage itself.

## Timing, export and binding checks

- VFX frame counts are suggestions. The compiler will use measured frame counts and deliberate per-frame timings. For example, an eight-frame warning loop does not mean an eight-tick warning.
- Projectile sprites and link segments keep constant full extent. Do not animate them growing across the sheet to imitate world travel.
- One-shot impacts fade to transparent; loops have a clean last-to-first seam. Return transparent PNG sequences or sheets plus actual count and loop metadata.
- Keep each output separate with the names above. The same source may intentionally produce several animations, but those outputs are different packages with different playback behavior.
- Preserve alpha, all provided margins, frame dimensions and source pixel scale. Do not trim/recenter individual frames, add a matte, resample the base, or bake the heroine into an effect.
- The eventual renderer must use the correct schema for the exact effect/buff kind, matching `.fanim` animation names. Verify a known-good roster binding before introducing a new one. A valid-looking texture file alone does not guarantee it appears in game.
- Mark follows the victim; link follows both endpoints; orb follows its target; warning and strike remain at fixed ground positions. These four attachment rules must be tested explicitly in motion.
- Dedicated stun/confusion stars are not necessary to show Ball Lightning's mechanics; native CC and the existing game's CC presentation remain authoritative. No extra unresearched crowd-control icon is added to the character sheet.

## What remains after animation

Receive and inspect all character rows and VFX outputs, compile `.fanim` packages, implement the approved data/native kit and AI, add real skill icons and voice where available, write neutral colored descriptions in eight locales, build a candidate and verify it in game. The art handoff does not install any of those mechanics yet.
