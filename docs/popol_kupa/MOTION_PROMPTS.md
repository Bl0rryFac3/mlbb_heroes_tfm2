# Copyable PixelLab motion prompts

Use the shared text plus ONE motion paragraph per generation. These are original authoring instructions for the supplied project sprites, not official extracted MLBB animation frames. Counts/timings below are output targets for review and can be adjusted after we inspect the returned frames. Gameplay event timing is owned by the simulation, not by repeatedly evaluating an animation frame.

## Shared — Popol

> Animate only the supplied Popol reference. Keep his existing pale face, silver-white hair, dark horns, brown leather hunter clothing, pale fur collar and ankle cuffs, boots and cyan-tipped spear. Preserve the exact native pixel scale, palette, clean outlines and hard pixel clusters. The cell is 72×72 transparent RGBA. His feet remain on the established ground line around row 55. Preserve his original three-quarter orientation toward the viewer's right. Keep the whole spear inside the cell. No redesign, wolf, rider, scenery, ground shadow, lettering, UI, motion blur, smoothed pixels or detached spell effects. Return exactly nine frames, preferably a 3×3 grid, with identical cell sizes and one character in each cell. Animate the motion in place; the game supplies world movement.

### BATCH 1 — popol_idle.png: loop, about 1.2 seconds

> A quiet hunter's ready stance: subtle breathing, a one-pixel shoulder settle and slight fur/hair movement. Both feet stay planted. He keeps control of the spear; the icy tip remains readable and does not drift into another cell. No attack, hop or weapon flourish. End close to the initial pose for a smooth loop.

### BATCH 1 — popol_run.png: loop, about 0.6 seconds

> A brisk compact run in place toward the right edge of the canvas. Alternate foot plants and opposing arm motion, with a small forward lean. Carry the spear securely beside the body so the tip stays visible without crossing the face. Fur cuffs and hair follow with restrained secondary movement. Do not translate the character across the sheet; the loop should reconnect naturally.

### Later — popol_attack.png: one attack, about 0.35 seconds

> One ranged spear throw toward the right edge: ready, compact arm draw-back, a clear forward release pose, then recovery toward idle. Release around the third frame. Keep the torso and face readable. Do not draw a separate traveling spear, damage burst or target; those are separate runtime assets. Do not invent a second spear, change the weapon design, or alternate facing. This represents one attack, not multiple throws.

### Later — popol_skill.png: one command, about 0.3 seconds

> One decisive command for Kupa to attack: Popol points or thrusts the spear forward with a sharp shoulder/arm motion, shifts weight to the leading foot and recovers. Keep his feet grounded and the spear inside the cell. No wolf or detached projectile. The companion lunge and spear projectile will be separate simulation events.

### Later — popol_skill2.png: one recall gesture, about 0.3 seconds

> Popol turns his upper body slightly without changing his established facing, gives a short inward beckoning signal with the free hand, then returns to ready. Keep the spear controlled, feet on the ground and original silhouette readable. Do not draw Popol suddenly seated or create a wolf under him; this clip is the recall gesture before the mounting transition.

### Later — popol_ult.png: one activation, about 0.5 seconds

> Popol plants both feet, braces, lifts the spear briefly and gives a forceful forward command. A tiny cyan highlight at the spear tip is acceptable. Keep the full body visible and preserve the existing outfit. Do not transform Popol into a wolf, enlarge him, add a full-screen aura or draw Kupa. The Alpha swap and burst are separate assets and runtime events.

### Later — popol_hit.png: one reaction, about 0.2 seconds

> A short readable damage recoil: shoulders jerk back slightly, knees compress, then recover. Keep the body and spear within the cell. No full spin, prolonged stun, blood, explosion or large movement. Preserve identity and facing.

### Later — popol_dead.png: non-looping, about 0.9 seconds

> A controlled defeat: Popol falters, lowers the spear and collapses into a compact kneeling or fallen pose. The fur and hair settle after the body. Keep the lowest ground contact on the established baseline and the weapon within the cell. End with a stable recognizable defeated pose. No disappearance, resurrection or return to idle.

## Shared — normal Kupa

> Animate only the supplied normal Kupa reference. Preserve his exact broad four-legged blue-gray wolf design, angular spiky mane, cyan markings and eyes, dark ears, muzzle, claws and bushy tail. Keep his native pixel dimensions and clean outlines; do not make him humanoid or redesign him as a different wolf. The transparent RGBA cell is 96×72; paws use the established ground line around row 55. Keep him facing toward the viewer's right, with all muzzle, mane and tail pixels inside the cell. No Popol, rider, ground shadow, scenery, text, smooth blur or detached spell effects. Return nine frames, preferably a 3×3 grid (288×216 total), with identical cells. Animate in place, preserving world position and scale.

### BATCH 1 — kupa_idle.png: loop, about 1.2 seconds

> A steady alert wolf idle: restrained breathing through the rib cage, a small mane settle, slight ear attention and a minimal tail movement. Keep all four paws planted. No bouncing, snarling attack, new glow effect or change in body size. The last frame reconnects cleanly to the first.

### BATCH 1 — kupa_run.png: loop, about 0.6 seconds

> A readable four-legged run in place toward the right edge. Use a grounded quadruped gait with forelegs and hind legs coordinated, controlled body rise/fall and a trailing tail. Keep the head aimed forward and mane identifiable. No net travel, skating, spinning, giant bounding leap or clipping the muzzle/tail at cell edges. This same cycle can serve ordinary following and recall travel; do not generate a duplicate return run.

### Later — kupa_attack.png: one bite, about 0.3 seconds

> One short melee bite: slight head draw-back, clear jaw opening, quick forward snap toward the right edge and recovery. Keep the paws mostly planted and the body inside the cell. Show one bite, not a repeating chew. Do not draw the victim or impact burst. The actual hit is a separate simulation event checked at contact range.

### Later — kupa_skill.png: commanded lunge, about 0.4 seconds

> One compact attack lunge in place: crouched anticipation, forelegs extend, shoulders and jaws drive forward, then a grounded recovery. Keep the wolf's world anchor stable; the game moves him toward the target. Use the extra transparent side margins for the pose without enlarging the character. No teleport copies, long traveling trail, victim or full-body icy explosion.

### Later — kupa_hit.png: one reaction, about 0.2 seconds

> A small impact reaction through the shoulders and front legs, head recoils slightly, mane jolts and then settles. Keep the wolf on four legs with the same orientation, scale and anatomy. No knockback travel across the sheet or full collapse.

### Later — kupa_dead.png: non-looping, about 0.8 seconds

> Kupa loses strength, forelegs fold and the torso lowers into a compact defeated resting pose. The mane and tail settle last. Keep his body in the 96×72 cell and grounded at the existing contact line. End still and recognizable. No disappearance, new rider, revival circle or magical dissolve.

## Hold until the technical mount test passes

- A properly seated Popol-on-Kupa canonical pose, with a coherent single combined silhouette. The diagnostic build merely composes the existing standing master over Kupa and labels it MOUNT TEST; it is not an animation reference.
- Mount start/end transitions and a mounted run loop derived from that seated design.
- A distinct Alpha Kupa master, then Alpha idle/run/attack/lunge/hit/defeat, plus any Alpha-mounted variant. Preserve the normal wolf's design identity; don't just magnify him smoothly or replace his palette.
- Prayer/revival and automatic trap gestures after their final control and interruption rules are implemented. Do not commission animations that the approved runtime plan cannot play.

After batch 1 returns, inspect actual alpha, canvas/cell counts, foot/paw drift, edge overflow, palette consistency and loop continuity before requesting all remaining clips.
