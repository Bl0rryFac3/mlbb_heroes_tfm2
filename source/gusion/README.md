# Gusion integration notes

The original Gusion atlas, JSON manifest, and seven effect uploads are preserved unchanged in this directory. The split atlas and `asset_manifest.json` are derived staging artifacts.

## Active integration

- Champion data: `champion/mlbb_gusion.data_champion`
- Character frames: `sprite/gusion#sheet.png` and `sprite/gusion#anim.fanim`
- Character animation tags: `idle`, `run`, `attack`, `skill`, `skill_recast`, `skill2`, `skill2_recall`, `ult`, `hit`, `dead`
- Separate projectiles/VFX: `effects/gusion_*`
- Icons: `icons/gusion_*.png`
- Native logic: `src/gusion.rs`, registered in `src/lib.rs`
- Localization: `text/champion.i18n`

Skill 1 throw/dash and Skill 2 throw/recall are separate selectable phases. Skill 2 uses one reusable dagger sprite per projectile and a separate, slower trail projectile behind each dagger. The first Skill 2 recall is delayed by 36 ticks so the AI does not immediately send the daggers back on the next decision. Character frames, projectile art, and effect art are separate assets. After screenshot feedback, the passive rune and Skill 1 reticle were reduced and shifted toward the top of their transparent frames; the Skill 2 streak is 80x10 and dagger projectile 36x15. The user's original Gusion design is retained; Fanny is used only as a pixel-art scale/style reference.

## Current behavior summary

- Dagger Specialist uses four cast-generated rune stacks; an empowered basic attack consumes them, adds 3% target max-HP magic damage per stack, and heals for 50 + 25% AP per stack.
- Sword Spike throws one homing dagger, marks the first enemy hit, refreshes Skill 1, and allows a recast dash behind that marked target.
- Shadowblade Slaughter spawns five separate fan projectiles with separate trails; recast sends them back toward Gusion.
- Incandescence is represented as two selectable target dashes; the first refreshes Skills 1 and 2.

## Validation status

All referenced JSON, frame rectangles, asset paths, view tags, and native effect names passed a static workspace check. `cargo fmt --check`, all 16 Rust tests, and a Linux release build pass. A Windows x64 GNU-target DLL candidate was cross-compiled and its PE format, stable-mod exports, imports, and package CRC were checked. The project's existing MSVC `mlbb_heroes.dll` was left unchanged. The candidate and recast/visual behavior have not been tested in TFM2.


Latest visual follow-up (2026-10-23): the three skill icons now use the default Gusion wiki artwork; the passive rune uses a taller bottom-anchored transparent frame to place it above the head; the Skill 1 mark art is unchanged with a subtle 8 px upward nudge; and the Ultimate arrival VFX is half-size (64×41). Exact in-game positioning and scale have not yet been tested in TFM2.

Basic-attack timing follow-up (2026-10-23): the 17 original poses are retained at 0.04 seconds each (0.68 seconds total). This is an animation-only change; it does not alter attack speed, damage, or cooldown. In-game timing has not been verified.

Rune/mark follow-up (2026-10-23): the passive rune art is now 24×24 and lifted another 16 px in its bottom-anchored transparent frame. Skill 1 mark artwork is unchanged and lifted an additional 8 px. TFM2 placement remains unverified.

Champion-view follow-up (2026-10-23): the shared UI framing map now has explicit entries for all five heroes. Aldous's known-good entry is unchanged; Fanny, Lancelot, Gusion, and Estes use Aldous-referenced framing with per-sprite face offsets. Estes previously relied on the game default because no explicit entry existed. This is not yet runtime-verified.

Basic-attack cadence follow-up (2026-10-23): the Gusion basic-attack cooldown was increased from 52 to 60 ticks after checking the hero-pack timings. The 17-frame, 0.68-second animation and attack power/hit damage remain unchanged; TFM2 cadence is not yet runtime-verified.

Basic-attack follow-up (2026-10-23): Gusion's attack clip was trimmed from 17 to 8 referenced frames to remove repeated follow-up thrusts while preserving a single strike arc. The kept frames are held 0.08 s each (0.64 s total); action duration is 28 ticks, hit timing tick 8, and cooldown 60 ticks. The atlas source remains intact; TFM2 timing still needs validation.
