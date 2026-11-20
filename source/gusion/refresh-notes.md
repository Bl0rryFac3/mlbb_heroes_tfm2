# Gusion refresh notes

- The passive rune and Skill 1 target reticle preserve their existing pixel artwork. Their transparent frames now use a bottom anchor (`anchor_y: 1.0`) and extra transparent space so the visible mark is shifted above the unit origin instead of covering the face.
- The basic attack now uses one strike arc from the supplied animation atlas: source frames 1–5 and 16–17 are retained; repeated thrust phases are omitted. The 7 poses last 0.08 seconds each (0.56 seconds total), with a 28-tick action duration and impact at tick 8. Cooldown is 60 ticks; damage is unchanged.
- Skill icons in `icons/gusion_skill.png`, `icons/gusion_skill2.png`, and `icons/gusion_ult.png` use the actual default Gusion Skill 1, Skill 2, and Ultimate icons from the supplied Mobile Legends Wiki page. Original CDN assets and their URLs are preserved under `source/gusion/wiki_skill_icons/`.
- Seven original mono PCM16/22.05 kHz SFX clips are wired into the action data. Four default-English Gusion voice clips from the supplied Audio page are also converted to mono PCM16/22.05 kHz WAV and wired to Skill 1, Skill 2, Ultimate, and Ultimate recast. The `.sound_info` format follows the Drive Twitch example; Twitch audio is not copied.
- Lancelot's damage has been tuned against the current Fanny build as a burst reference. At level 15, Lancelot's current raw coefficients are: Puncture `80 + 7/level + 40% AD`; each Thorned Rose wave `85 + 5/level + 30% AD` (three scheduled waves); Phantom Execution `100 + 4/level + 100% AD`. With the two Soul Cutter stacks generated in a Puncture-to-Ultimate sequence, that one Puncture + all three Rose waves + Ultimate is approximately 1,260 raw damage at level 15 before enemy mitigation. These values are covered by Rust unit tests. At level 15, Lancelot's two-stack Phantom Execution is 356 raw damage, versus Fanny's two-Prey-Mark Cut Throat at 357 raw damage.
- These are static asset/data/build checks only. The positions, sound playback, animation feel, and combat balance still require a TFM2 runtime test.


## Visual follow-up (2026-10-23)

- Repositioned the passive rune higher with a 40×96 transparent frame and bottom anchor, aiming to clear Gusion's face. This is an asset-level adjustment and still needs an in-game TFM2 check.
- Kept the Skill 1 mark artwork unchanged and added 8 px of transparent padding below it (40×72 frame) for a small upward nudge.
- Reduced the Ultimate arrival effect from 128×82 to 64×41 (50% dimensions), centered on the effect anchor.
- Saved the prior runtime icons and affected effect sheets/animation metadata under `source/gusion/visual_before_followup_2026-10-23/`; no earlier candidate/source archives were overwritten.


## Basic-attack timing follow-up (2026-10-23)

- Preserved all 17 original attack poses and changed each frame from 0.01960784 s to 0.04 s, for a 0.68 s animation cycle. No attack-speed, cooldown, or damage values were changed.
- The previous animation metadata is backed up at `source/gusion/visual_before_followup_2026-10-23/gusion_anim_before_attack_slowdown.fanim`.
- The updated timing has not been tested in a TFM2 match.


## Rune and Skill 1 mark follow-up (2026-10-23)

- Reduced the passive rune art from 28×28 to 24×24 pixels (about 14% smaller) and added 16 px of transparent-bottom frame height to lift it further.
- Kept the Skill 1 mark artwork and size unchanged; added another 8 px of transparent-bottom frame height (16 px total upward shift from its original frame).
- Saved the prior runtime sheets and animation metadata under `source/gusion/visual_before_second_adjustment_2026-10-23/`.
- These placement adjustments still require a TFM2 visual check.


## Champion-view framing follow-up (2026-10-23)

- Kept Aldous's known-good `champion_view` entry exactly unchanged.
- Adjusted Fanny, Lancelot, and Gusion's face/center/ban-pick offsets toward Aldous's framing, with face X positions tailored to each idle sprite's visible placement.
- Added an explicit Estes entry; he previously had no custom `champion_view` entry and therefore inherited the game's default framing.
- The adjusted framing is based on sprite-sheet geometry and has not been confirmed in a TFM2 UI/runtime test. The previous shared map is preserved at `source/ui_view/champion_view_before_aldous_reference_2026-10-23.champion_view`.


## Basic-attack cadence follow-up (2026-10-23)

- Checked the attack animation and action data: the 17-frame visual is 0.68 seconds, but the basic-attack cooldown was only 52 simulation ticks, the shortest in this hero pack (Lancelot 55, Aldous/Estes 60, Fanny 65).
- Increased only Gusion's basic-attack `cooltime` from 52 to 60 ticks. The attack animation remains 17×0.04 s; damage, attack value, action duration, and hit timing are unchanged.
- The previous champion data and animation metadata are backed up in `source/gusion/attack_cadence_before_cooltime_fix_2026-10-23/`. Runtime cadence still needs TFM2 verification.


## Basic-attack frame-trim follow-up (2026-10-23)

- Inspection found 17 unique cells, but the sequence contains repeated thrust phases (around source frames 3–5, 8–10, and 14–15). No exact duplicate cells exist.
- Kept source frames 1–5 and 16–17 to form one wind-up/strike/recovery sequence; the atlas sheet itself is unchanged. The remaining seven poses are retimed to 0.08 s each (0.56 s total).
- The Gusion attack action now lasts 28 ticks so the complete clip can play. Hit timing remains tick 8, which lands at the start of retained frame 3. Cooldown stays at 60 ticks.
- The previous 17-frame version is backed up at `source/gusion/attack_frame_trim_before_2026-10-23/`. Visual timing needs TFM2 confirmation.
