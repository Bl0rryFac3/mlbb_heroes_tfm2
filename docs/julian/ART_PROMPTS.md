# Julian — copyable SpriteCook and PixelLab prompt pack

No images were generated for this handoff and no SpriteCook or PixelLab credits were spent. These are text prompts for the owner to use manually. Use **default Julian / Scarlet Raven**, not Silent Edge, Megumi Fushiguro, or another skin.

## 1. Generate the one character master in SpriteCook

Select **Pixel Engine Image v1**, asset/pixel-art mode, **72 × 72**, transparent background, one variation. The character itself should be about **45 pixels tall**, not 72. This model is text-only: do not attach style/reference images to its generation request. Its exact-size output already has background removal; do not pay for a second removal pass. If the model's name or availability has changed, verify the selector rather than substituting an unannounced model.

Copy this prompt:

> One single full-body pixel-art character sprite of Julian, Scarlet Raven, from Mobile Legends: Bang Bang, wearing his DEFAULT outfit. Final canvas exactly 72 by 72 pixels, transparent background. The living character from the top of his hair to the soles of his boots is approximately 45 pixels tall. Fit the standing body around pixel columns 23–49 and rows 11–55; both feet share a stable ground line at row 55. Keep all cape and chain pixels inside the canvas with safe transparent margins. Chibi tactical auto-battler proportions: slightly enlarged head, compact torso, sturdy readable legs, clean one-pixel dark outline, hard-edged pixel clusters, restrained small palette, two or three clearly separated shades per material. No painterly texture or soft antialiasing. He is turned about 30 degrees toward the viewer's right; his gaze and leading hand point toward the right edge of the image. Do not draw a frontal portrait or a mirrored alternate.
>
> Julian is a slim young human male with pale skin, a stern quiet expression and sharply spiked, swept crimson-red hair with dark roots and deep burgundy shadows. His hair is fully visible; his hood is down. His face has dark, narrow eyes and dark shading around them, without a beard, helmet, animal ears or horns. At this scale, use a few clean face pixels rather than detailed facial lines. His neck has a tall charcoal-black collar. His fitted sleeveless torso armor is steel gray with dark charcoal straps and angular silver edging. A short asymmetric black raven-feather mantle covers his own left shoulder; its large feather groups have subtle deep blue and violet highlights, with a muted red lining. His other upper arm remains visibly bare. He wears burgundy-red gloves or hand wraps and dark forearm wraps. The waist has a small angular silver diamond-shaped clasp and layered dark belt panels. The trousers are deep burgundy above dark charcoal leg armor and simple dark knee-high boots. Keep the footwear simple and readable, without invented bright gold ornaments.
>
> Include one short, controlled section of dark silver chain beside his leading hand, with a small cyan-blue spectral blade tip. The weapon should be identifiable but must not cover the face or turn into a giant glowing prop. The chain is a linked weapon, not a bow, gun, staff or enormous two-handed scythe. His posture is a balanced combat-ready idle with slightly bent knees. The feather mantle hangs behind the shoulder and does not become a pair of wings. No attack burst, ground circle, crown, floating UI icon, lettering, scenery, checkerboard pattern, cast shadow or extra character. Deliver ONE sprite, not a sheet, turnaround, animation strip or collage.

### Visual evidence and deliberate simplification

The official portrait and the default in-game reference were inspected: red swept hair, pale face, high black collar, asymmetric blue-black feather mantle, gray strapped vest, red trousers, dark leg protection, and blue magical weapon are supported by those references. The available large reference crops the lowest footwear; the simple dark boots in this prompt are a conservative pixel-art interpretation, not a claim that a particular buckle/toe pattern was verified. At 45 pixels, silhouette and color placement matter more than decorative micro-detail.

Reference links for the OWNER's visual inspection, not Pixel Engine image inputs:

- Official publisher portrait: https://akmweb.youngjoygame.com/web/svnres/file/test/homepage_2_2_16_1232_1/100_8359a497952c5215b70b47373e168cfe.jpg
- Default in-game model reproduced in the release-era reference: https://tekno.kompas.com/read/2022/05/26/09000067/update-mobile-legends-1.6.84-bawa-hero-baru-julian-ini-kemampuannya
- Official hero spotlight: https://www.youtube.com/watch?v=FAn0Id5_kd4

## 2. Inspect and lock the master

Check the actual PNG before paying for animation. It must be RGBA with real transparent pixels, approximately 45-pixel standing height, a visible face, correct red/gray/burgundy/blue-black palette, no clipped cape/chain, and the gaze aimed toward the viewer's right. Do not silently mirror it. If the master is wrong, fix the master first; one incorrect silhouette multiplied over eight animations is expensive to repair.

Save the accepted original as `julian_master.png`. Keep it unchanged once motion work starts. Every character motion below uses this SAME accepted PNG as its reference. Do not regenerate the character separately for each motion.

## 3. PixelLab character animation settings

Request **9 frames per motion**, exported as a transparent RGBA PNG with **72 × 72 cells**, preferably a 3 × 3 grid; a horizontal nine-cell strip also works. All eight requested motions are required. The integration compiler assembles the game's eight rows and leaves unused columns transparent.

The source here is already native-size. Preserve its size or use the current UI's equivalent of 1× output. **Do not blindly use ÷10 on a 72-pixel source.** Historical project notes used ÷10 only for a source that had first been enlarged tenfold. If PixelLab insists on a larger reference, enlarge using nearest-neighbor exactly, record that factor, and require a lossless return to the native grid before this compiler. Never use arbitrary fractional scaling or smooth interpolation.

If PixelLab exposes them, Faithful mode ON and reference initialization around 86 are established project starting points. Do not invent unsupported API fields; the owner is using the current UI manually. Ask for one motion at a time, not all eight motions in one image.

Paste this shared instruction before each motion-specific prompt:

> Animate the supplied accepted Julian sprite without redesigning it. Preserve his exact hair silhouette, pale face, asymmetric feather mantle, gray strapped vest, red gloves and trousers, dark boots, chain, palette, outline thickness, 45-pixel body scale and gaze toward the viewer's right. Keep the character's world position centered in the same 72 by 72 cell. Preserve transparent alpha and hard pixel edges. Do not add a second character, scenery, text, ground shadows or detached world-space spell effects. Keep every frame inside the cell. Keep both foot planting and cape asymmetry consistent; never mirror or turn him to face the viewer's left. Return exactly nine frames. The runtime creates moving projectiles and ground VFX separately.

### idle → `character/idle.png` — loop, 1.2 seconds

> A quiet, restrained combat idle. Julian breathes subtly, shifts his weight by at most one pixel, and lightly flexes the chain hand. Two or three large feather groups settle on his shoulder with a very small secondary sway. The red hair tips move minimally. Both boots stay planted at the same ground line. No casting, walking, hopping, weapon orbit, large glow or large silhouette change. The ninth frame reconnects cleanly to the first.

### run → `character/run.png` — loop, 0.6 seconds

> A brisk forward run in place toward the right edge of the canvas. Use a compact alternating leg cycle, a slight forward torso lean and natural opposing arm motion. Julian keeps control of the short chain instead of letting it whip outside the cell. The feather mantle trails behind him and follows the run with a small delay; keep the shoulder silhouette identifiable. No net travel across the sheet, no sliding floor, no extra walk animation. End in a pose that loops smoothly into frame one.

### attack → `character/attack.png` — one action, 0.4 seconds

> One short, forceful chain-blade basic attack aimed toward the right edge of the canvas. Start from the accepted combat stance, draw the leading arm back, flick the short chain forward in a controlled diagonal strike, then recover toward idle. Emphasize a crisp contact pose without rotating the whole torso away from the target. Keep the chain and blade inside the cell. Do not draw an enormous magical slash or a projectile traveling independently across the screen; the weapon motion itself should carry the attack.

### Scythe / skill → `character/skill.png` — 0.3 seconds

> Julian primes a spectral weapon with a quick hand rotation and thrusts his casting arm toward the right edge of the canvas as if releasing a flying scythe. A very small cyan glint at the fingertips is acceptable, but do not draw the released scythe moving through the world. His weight moves onto the leading foot, the mantle responds to the arm movement, and the body returns toward a usable combat stance. The same body animation serves normal and enhanced Scythe; the separate VFX carries the difference.

### Sword / skill2 → `character/skill2.png` — 0.3 seconds

> Julian lowers his center of gravity, draws his chain hand close, and makes a sharp forward dash preparation and recovery in place toward the right edge of the canvas. Show a readable low streamlined pose with the mantle trailing. Keep the sprite anchored inside its cell: do not translate him across multiple cells, teleport him within the sheet or add a whole path of ghost copies. The game performs the movement, banish and separate blade trail. Normal and enhanced Sword reuse this body motion.

### Chain / ult → `character/ult.png` — about 0.333 seconds

> Julian braces his legs, raises the casting hand briefly, then snaps the hand down and forward as if anchoring magical chains in the ground ahead of him. Make the hand plant and the downward command crisp and readable. Keep his head and red hair visible above the collar. Do not put a crown above his head, wrap his entire body in an ice block, or cover him with the target's chain prison. The ground chains, warning circle and launch are separate effects at the target location. Normal and enhanced Chain reuse this body motion.

### hit → `character/hit.png` — 0.2 seconds

> One quick damage reaction: a small backward shoulder recoil, brief knee compression, slight mantle jolt and recovery. Keep both the character identity and original orientation. No full spin, extended stun pose, white full-body flash, blood cloud or displacement outside the cell. The motion should read quickly without interrupting the silhouette for long.

### dead → `character/dead.png` — 0.9 seconds, non-looping

> A controlled defeat animation. Julian's shoulders drop, one knee buckles, the chain hand lowers and he collapses into a small readable fallen or kneeling body pose. Keep the lowest visible body/boot pixels on the established ground baseline rather than sinking through the floor. The mantle settles after the body. End with a persistent clear corpse pose, not resurrection, disappearance, a victory pose or a return to idle. Preserve the red hair and burgundy clothing so the defeated character remains recognizable.

## 4. SpriteCook VFX first frames

Generate each FIRST FRAME separately with **Pixel Engine Image v1**, pixel-art asset mode, transparent background, one variation. These are **effects, not character portraits or item emblems**. Keep the same spectral cyan/icy blue magic, pale blue highlights, dark navy outline and chunky pixel clusters throughout. Julian's magic is not red fire simply because his hair is red.

The dimensions below are the native CELL dimensions, not the size of the final animation sheet. Keep substantial transparent margins. For directional effects, their orientation in the first frame is toward the right edge of the canvas; the compiler creates the other 15 directions. Do not request 16 directions from the image model.

### A. Normal Scythe — 72 × 48 → `vfx/scythe.png`

> One isolated flying spectral scythe-blade effect belonging to Julian, default Scarlet Raven. Exactly 72 by 48 pixels, transparent. A compact crescent-hook weapon with a dark blue steel-like spine, vivid cyan inner edge and a few pale blue facet highlights. Its travel axis points toward the right edge of the canvas. Center the weapon around pixel 36,24; the visible blade is approximately 28 by 22 pixels, leaving transparent space on all sides. The silhouette must read as a hooked cutting blade, not a circular shield, feather, crown, bow or ice cube. Short restrained energy wisps may sit directly behind the blade, but no long expanding trail or full character. Hard pixels, clean outline, one effect, no text.

### B. Enhanced Scythe — 72 × 48 → `vfx/scythe_enhanced.png`

> One stronger Julian spectral scythe projectile. Exactly 72 by 48 pixels, transparent. Use two interlocking or opposing curved cyan-blue cutting edges around a compact dark central spine, larger and more energetic than the normal scythe but still centered near pixel 36,24. Keep the visible art within roughly 40 by 30 pixels. Pale blue highlights mark the cutting edges and small cyan fragments cluster close to the weapon. Aim the travel axis toward the right edge of the canvas. This is a flying magical weapon, not an entire spinning ground storm or a character holding a prop. No aura crown, red flame, full-screen flash or opaque glow background.

### C. Endpoint spin field — 72 × 48 → `vfx/scythe_spin.png`

> A compact Julian scythe spinning field seen from the game's elevated side perspective. Exactly 72 by 48 pixels, transparent. A low horizontal ellipse about 36 pixels wide and 20 pixels tall, made of two curved cyan-blue blade streaks circling an empty transparent center near pixel 36,24. Show a crisp rotating-weapon silhouette with several small blue fragments, not a filled disk. Keep most visual energy near the rim, with enough empty center that a character remains readable. No hero body, pedestal, crown, smoke, fire or giant tornado. Hard pixel clusters and pale cyan edge highlights.

### D. Normal Sword trail — 96 × 48 → `vfx/sword.png`

> One horizontal Julian dash blade trail, exactly 96 by 48 pixels on transparent alpha. The casting origin is pixel 12,24. A narrow spectral blue sword-shaped streak extends from that origin toward the right edge, with visible art contained between approximately x=12 and x=62 and y=14 to y=34. Use an angular bright cyan leading blade and two slim trailing blue energy ribbons. The full trail has a fixed 50-pixel extent and never fills the whole canvas. Leave the left side and outer margins transparent. No character, footwear, crown, wings, ground circle or scenery. The effect represents a short path of magical cutting force, not a separate enormous sword item.

### E. Enhanced Sword swarm — 96 × 48 → `vfx/sword_enhanced.png`

> One Julian enhanced-dash field of several small flying spectral swords, exactly 96 by 48 pixels, transparent. Use the casting origin at pixel 12,24 and a fixed path extending toward pixel 62,24. Arrange three to five small cyan-blue blade streaks at slightly different heights along the same narrow 50-pixel horizontal lane, leaving clear transparent gaps between them. Each sword has a dark navy edge and pale blue cutting highlight. This should read as a brief coordinated blade swarm covering the dash path, not a single oversized sword, a clone of Julian, a crown or an explosion. Keep all art within x=12..62 and y=10..38 so rotated frames will fit safely.

### F. Chain warning — 72 × 48 → `vfx/chain_warning.png`

> One restrained ground-target warning for Julian's Chain. Exactly 72 by 48 pixels, transparent. Center at pixel 36,24 a low perspective ellipse about 36 by 18 pixels, formed from a thin broken cyan-blue ring and two short angular chain-link marks. Keep the center transparent and the glow modest, so it reads as an incoming spell warning rather than an already solid prison. Do not draw a character, giant rune text, a halo above a head, a crown, fire or an opaque floor disk. Crisp small pixel clusters, blue/navy outline, no letters.

### G. Normal chain bind — 72 × 48 → `vfx/chain_bind.png`

> Julian's ground-anchored binding chains, one effect on transparent alpha, exactly 72 by 48 pixels. Center a small perspective ellipse around pixel 36,26, about 36 pixels wide. Two or three chunky linked cyan-blue chains cross just above the ground and curl upward only slightly, leaving the central body space mostly empty. Emphasize recognizable individual rectangular or hexagonal links with dark navy edges and pale cyan highlights. This is a root at the target location, not armor worn by the caster and not a crown hovering above him. No target character, full cage wall, smoke, red flame or text.

### H. Enhanced chain field — 72 × 48 → `vfx/chain_enhanced.png`

> A charged Julian chain field just before a launch. Exactly 72 by 48 pixels, transparent. A compact perspective ground ellipse about 36 by 20 pixels, centered at pixel 36,26. Three or four blue spectral chains form a tense crossing pattern with small links beginning to lift from the ground. Add pale cyan light at the intersections, while keeping the center mostly transparent and the outside edge clean. More forceful than the normal bind, but not yet a tall eruption. No hero, crown, ice, fire, dense smoke or giant luminous disk. Match the same cyan/blue linked-metal visual language as the normal Chain.

### I. Chain launch — 72 × 72 → `vfx/chain_launch.png`

> One upward chain eruption for Julian's enhanced Chain finisher. Exactly 72 by 72 pixels, transparent. The ground center is pixel 36,36. A low blue chain ellipse about 36 pixels wide anchors three narrow spectral linked chains that snap upward into a short open column, reaching roughly 20 pixels above the ground center. Keep the column open so the target sprite will not be buried behind a solid wall. Use cyan-blue links, dark navy edges and pale edge highlights, with a few small broken sparks at the top. The direction is visibly UP from the target's ground point. No character, crown, tornado, sword, smoke puff, fire pillar or text.

## 5. PixelLab VFX motion prompts

Reference each accepted VFX first frame separately. Export exactly **nine frames**, RGBA PNG, with the SAME cell size specified above. Keep alpha transparent, fixed cell registration and hard pixel edges. Source filenames must match `ASSET_MANIFEST.json`. A 3 × 3 grid is recommended. Do not bake a checkerboard into the background.

Shared VFX instruction:

> Animate this single supplied effect without changing its palette, linked-weapon identity, pixel scale, canvas, anchor or travel axis. Preserve transparent alpha. Keep every frame inside the original cell. No camera movement, scaling, scene, character, text or added icon. The game positions and rotates the effect; the animation should show the effect's internal motion rather than moving the entire object out of its cell.

### scythe — loop, 0.3 seconds

> Keep the scythe centered with constant length and silhouette extent in all nine frames. Rotate or shimmer the cutting facets slightly around the same central weapon, and let two or three nearby cyan pixels pulse along the edge. Do not translate the projectile from one side of the canvas to the other, grow a trail from zero length, expand the weapon, dissolve it or create an empty final frame. The game's projectile simulation supplies travel and the runtime chooses the moving effect's animation phase.

### scythe_enhanced — loop, 0.3 seconds

> The opposing spectral cutting edges rotate in a compact interlocked cycle around the same center. Keep their maximum extent constant and emphasize a stronger bright edge than the normal scythe. Small cyan fragments orbit close to the blades, not far outside the silhouette. No whole-canvas travel, expanding shockwave, size growth, empty final frame or detached character. The ninth frame must reconnect to the first.

### scythe_spin — loop, 0.6 seconds

> Spin the two low blade arcs around the same small ground ellipse. Alternate their highlights and fragment positions so rotation is readable, keeping the same width, height and empty center throughout. Do not turn the low field into a standing tornado or a giant sphere. No traveling across the canvas and no filled center disk. Make the loop seamless; the game controls the field's 1.6-second lifetime.

### sword — one shot, 0.3 seconds

> A short light sweep races along the existing fixed horizontal sword trail from the casting origin toward the right edge. The overall 50-pixel footprint stays fixed; only the highlight and small fragments move. The trail brightens quickly, cuts once, and fades cleanly by the last frame. Do not draw Julian or translate a second hero across the canvas. Keep the origin exactly aligned at pixel 12,24.

### sword_enhanced — one shot, 0.6 seconds

> The small spectral blades make staggered rapid cutting passes within the same narrow fixed path. Alternate which blade is brightest, let blue fragments sweep forward, then let the entire swarm fade by the final frame. The effect should suggest several rapid cuts while Julian is briefly absent, not clones of him standing in a line. Keep the 50-pixel path footprint constant and the casting origin at pixel 12,24. No expansion, new large blade, smoke or crown.

### chain_warning — one shot, 0.3 seconds

> The thin broken ground ellipse and small link marks become briefly clearer, make one controlled pulse, then hand off into a near-transparent final frame. The center stays transparent. Do not raise a prison, launch chains or create a damaging explosion at the warning stage. Keep the same ground center and perspective ellipse.

### chain_bind — one shot, 1 second

> The ground chains rise a little, snap across the same small area and hold tension with a subtle two-pixel tremble. Toward the end, the links loosen and fade, leaving a transparent final frame. Keep the center mostly open, the links low, and the whole field registered to the same ground point. It should communicate immobilization, not a full-body costume or a hovering emblem.

### chain_enhanced — one shot, 0.8 seconds

> The crossing ground chains tighten in four short rhythmic pulses. Each pulse sends a restrained cyan highlight along the links toward the central intersection. The links lift slightly as tension builds, but do not perform the final upward launch here. Keep the footprint fixed and the middle readable. Fade into a clear handoff at the end; the separate chain_launch effect provides the eruption.

### chain_launch — one shot, 0.4 seconds

> The linked chains snap rapidly upward from the fixed ground center, making a narrow open vertical eruption. Emphasize one crisp upward whip, a short bright peak, then broken blue sparks and fading links. Keep the lower anchor at pixel 36,36 and leave the column center open. End transparent. Do not move the whole effect vertically across the cell or add a target character, impact smoke, crown or fire pillar.

## 6. Return checklist

Return **8 character grids + 9 VFX grids**. Put them in `character/` and `vfx/` using the filenames above. Include the accepted `julian_master.png` as reference. Keep originals; do not use a chat screenshot of a sheet as the source. Send lossless PNG files, not JPEG, video screenshots or a compressed Discord preview.

If PixelLab returns a different frame count or cell size, tell the compiler agent the actual output. Do not have it silently crop frames or rescale native 45-pixel art. The supplied compiler deliberately fails when the nine-frame contract is broken.

Three skill icons can be derived intentionally from the accepted Scythe, Sword and Chain first-frame silhouettes by the included compiler, with nearest-neighbor scaling and dark icon backing. Inspect their readability; these are a deliberate derived-icon design, not blank placeholders. Official licensed references or separately approved icons may replace them. Audio is intentionally not synthesized or borrowed: Julian is silent until approved clips are supplied and bound.
