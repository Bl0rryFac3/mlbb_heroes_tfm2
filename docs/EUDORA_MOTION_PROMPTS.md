# Eudora — PixelLab character motion prompts

Use the selected base `art/eudora_base_72x72.png`. It is the same native 45-pixel-tall sprite padded onto a transparent 72×72 cell. The tight `eudora_master_22x45.png` and untouched 40×45 source are also included. Do not animate a zoomed preview.

## Shared settings and constraints

- Keep native pixel size. Do not divide by 10: that instruction applied to earlier large masters, not this 45-pixel source.
- Fixed camera, transparent background, same three-quarter view toward the viewer's right. Character translation through the world belongs to the engine.
- Preserve the pale face, violet eyes, pointed ear, silver swept hair, silver collar, white split-tail coat, navy body, cyan chest detail, blue legs and cyan-accented boots. Empty hands throughout.
- Hard pixel edges, one-pixel dark outline and the original palette. No smooth anti-aliasing, painted shadows, extra accessories or changing facial proportions.
- The floor/contact line is row 55. Keep the reference placement stable; allow intentional limb and coat motion inside the 72×72 cell. No per-frame automatic centering, cropping or resizing.
- The eight engine row names are `idle`, `run`, `attack`, `skill`, `skill2`, `ult`, `hit`, `dead`. There is no separate walk row.
- Character sheets contain the body motion only. Projectiles, marks, links and lightning strikes are separate world effects, so do not bake them into these frames.
- Suggested frame counts below are production targets. Actual returned frame counts must be measured and the `.fanim` generated from them. The engine timing in the approved spec remains authoritative; frame count alone does not set damage timing.

If PixelLab cannot start a continuous run from this standing source without inserting a pause, use its pose preparation tools to make a matching mid-stride starting pose. Preserve the same design and pixel scale; do not independently regenerate another Eudora.

## 1. Idle — `idle`, 8 frames, loop

**Motion prompt:**

The silver-haired elf mage holds the same calm ready stance, facing slightly toward the viewer's right. Her chest rises and falls by one pixel with a slow breath, her open hands relax slightly, and the two white coat tails make a small delayed sway. Both boots stay planted on the same floor line. Her pointed ear, violet eyes, hair shape and cyan chest detail remain stable. Finish in the exact starting pose for a seamless quiet loop. No camera movement, lightning, new objects or body travel.

## 2. Movement — `run`, 12 frames, loop

**Motion prompt:**

The same mage runs in place toward the viewer's right while retaining the source's three-quarter angle. Her blue legs alternate through clear planted and lifted steps, her arms make restrained opposite swings, and the white coat tails trail and swing behind her with a slight delay. Keep her head and torso height steady, with only a small natural bounce. Preserve the silver hair, face and clothing shapes. Cycle continuously through the starting stride without stopping in an idle pose. The camera and character's world position stay fixed. No particles or weapon.

## 3. Basic attack — `attack`, 8 frames, one shot

**Motion prompt:**

From the source ready pose, the mage draws her near open hand slightly toward her chest, then snaps that hand forward toward the right edge of the image with fingers spread. Her shoulder follows with a small controlled lean while both feet stay planted. Hold the forward palm briefly, then smoothly return to the exact starting pose. Keep the other hand relaxed and maintain her face and outfit. Animate only the casting gesture; leave the projectile, flash and impact to the game. No staff, punch, sword or camera motion.

Suggested release pose: middle of the motion; mark its actual frame for the later attack event binding.

## 4. Forked Lightning — `skill`, 10 frames, one shot

**Motion prompt:**

The mage gathers both open hands close to her chest, then pushes both palms forward and slightly apart toward the viewer's right in one broad fan-casting gesture. Her upper body leans forward slightly, the pointed shoulder pieces remain attached, and the white coat tails flare briefly before settling. Her boots stay on the same floor line. Hold the spread-hand release pose for one clear beat, then return to the source ready stance. Preserve the same head, violet eyes, silver hair and costume. Do not draw lightning, beams, a fan shape or any world effect in the character frames.

This is a quick fan release, not an uninterrupted one-second channel. The maintained link is a separate native mechanic; ordinary movement must still be possible after cast recovery.

## 5. Ball Lightning — `skill2`, 10 frames, one shot

**Motion prompt:**

The mage pulls her near hand back beside her shoulder, with the open palm angled forward as if gathering a small charge. She makes a precise forward casting flick toward the right edge, extending the fingers and following through with the wrist. Her far hand remains close to her torso for balance. Keep both feet planted and use only a small upper-body twist. Her white coat tails react gently and settle as she returns to the original ready pose. Keep the source face, hair, ears and clothing unchanged. Do not include an orb, trail, explosion or lightning in these body frames.

## 6. Thunder's Wrath — `ult`, 12 frames, one shot

**Motion prompt:**

The mage braces in the source stance and opens both hands outward. She raises her forearms and hands above shoulder level in a deliberate summoning gesture, keeping all fingers and hair inside the canvas. She then lowers both palms decisively toward the ground in front of her, as if calling a strike down onto a distant area. The white coat tails lift slightly with the gesture and settle afterward. Her boots remain planted; no jump or levitation. Hold the downward command pose briefly and return to the exact ready stance. Preserve her identity and palette. No sky, clouds, ground circle, lightning column or impact flash belongs in the character frames.

The lightning lands on the selected ground position, not automatically on Eudora's body. The cast is not a long beam channel.

## 7. Taking damage — `hit`, 4 frames, one shot

**Motion prompt:**

The same mage flinches briefly from a hit coming from the viewer's right. Her torso recoils slightly toward the left of the canvas, her near arm draws inward, and her white coat tails lag by one frame. Keep her face recognizable and both feet close to the original placement. She quickly regains the exact ready pose. No spinning, knockback travel, changing costume, blood, particles, extra weapons or camera shake.

## 8. Death — `dead`, 10 frames, hold final frame

**Motion prompt:**

The mage loses strength from the source stance. Her shoulders drop, her knees bend, and she lowers onto the same floor line before collapsing onto her side in a compact pose. Her silver-haired head, pointed ear, hands and both white coat tails remain recognizable and completely inside the canvas. The coat settles naturally around her blue legs. End with a still fallen body resting on the floor, with no return to standing and no loop back to the start. Preserve her colors and costume; no disappearance, explosion, blood or particle effects.

Prepare enough horizontal space for the fallen pose; 72×72 is the intended shared cell. Do not shrink only the dead row to make it fit. If the pose overflows, revise the motion to fold more compactly.

## Return/export checklist

Return separate named PNG sheets or frame sequences for all eight motions with transparent backgrounds. Include frame order, actual count, intended loop behavior and any timing metadata PixelLab provides. Keep the original frame canvases and pixel scale. Do not flatten the character onto a colored background, combine unrelated motions into one undocumented sheet, or trim every frame independently.

We will measure every row, compare the starting/ending poses, inspect the loop seams and align with one uniform translation per row. That preserves actual motion rather than cancelling it through per-frame recentering. The final sprite files will be `eudora#sheet.png` and `eudora#anim.fanim`, never a bare `eudora.png` reference.
