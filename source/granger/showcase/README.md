# Granger showcase — MLBB Heroes v2.5.0

800x450, 20fps, 32.2 seconds, 644 frames, infinite loop. Rendered from the unchanged sprite/VFX assets shipped with internal build 1.1.148. The GIF is about 2.07 MB; a silent H.264 MP4 is also provided.

This is a staged training-target asset preview, not a game recording or engine simulation. The permanent footer identifies it as staged. Dummy health bars provide illustrative contact feedback, not measured native damage/critical results. Positions, projectile travel and action clips are choreographed/retimed for readability; each chapter uses a reset training preset. Defeat/return timing is condensed and is explicitly labeled.

## Chapters

| Start | Seconds | Demonstration |
|---|---:|---|
| 0.0 | 3.0 | Granger arrival, locomotion, two energy pools and target dummy |
| 3.0 | 4.8 | Basic handgun fire, six homing S1 rounds, distinct final projectile and impact |
| 7.8 | 3.5 | Rondo dash and 15% maximum Gun Energy restore |
| 11.3 | 4.8 | Two homing cannon shots, one area explosion per shot, slow cue and separate retreat roll |
| 16.1 | 5.3 | Rhapsody continues while Rondo/cannon/roll body poses take priority |
| 21.4 | 5.0 | Minion, jungle and champion training targets; +6/+6/+12 capacity growth |
| 26.4 | 3.4 | Incoming hit, hit reaction, death and condensed return |
| 29.8 | 2.4 | Release end card and seamless repeat to the opening setup |

## Asset handling and QA

No character painting, invented animation frames or paid image generation. Sprite frames are sampled from shipped FANIM rectangles/anchors, nearest-neighbor scaled, and composited onto an original geometric concert-hall stage. The passive/active icons use the recovered official artwork. Basic fire shows the full clip; barrage recoil reuses its existing aimed/recovery poses to maintain aim through repeated shots. Cannon uses its baked body flash, avoiding a second external muzzle flash.

All 15 used PNG/FANIM animation pairs retain every source frame. Frame-use QA confirms full coverage, including all 13 death frames. Source asset hashes, dimensions, anchors and coverage appear in showcase_manifest.json. GIF decoded frame count/timing/loop/size checks appear in QA.json. Every source asset remains byte-identical to the release tree. Resource display uses 60-tick energy clocks, real 70/250 costs, 15% Rondo restore and capacity growth; chapter resets and retimed choreography are intentional. It is not evidence of live AI, hit detection, critical damage or balance.

Both GIF and MP4 are silent. Voices remain in the installable mod; no synthetic speech or soundtrack was generated.

## Files and rebuilding

- Granger_Showcase.gif — looping release/Discord media.
- Granger_Showcase.mp4 — silent H.264 copy.
- Granger_Preview.png — selected encoded GIF frame.
- Granger_Storyboard.png — one decoded GIF sample per chapter.
- Granger_Showcase_Source.zip — editable script, exact used assets, fonts, manifest, QA and preview material.

Source ZIP layout: granger_showcase/build_showcase.py plus assets/effects, assets/icons, passive_icon.png and fonts. Requires Python 3, Pillow and FFmpeg. Run:

```
python build_showcase.py --preview
python build_showcase.py --encode
```

The script finds bundled assets/fonts first. Preserve the staged-preview footer if reusing this render. The owner publishes the GitHub release and announcement. References: previous Popol_Kupa_Showcase.gif and the owner's Angela example in Dropbox; the Granger composition uses a new red/gold concert-hall palette and the current accepted Granger assets.
