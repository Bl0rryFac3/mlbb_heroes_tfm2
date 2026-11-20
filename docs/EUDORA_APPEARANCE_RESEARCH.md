# Eudora — appearance research and selected base

14 November 2026. Phase 0 was approved in chat. This pass covers the revamped default Eudora, not a paid skin or the older default portrait.

## References actually inspected

1. [January 2026 in-game model capture, reproduced by GGWP](https://www.ggwp.id/esports/moba/build-eudora-revamp-2026-terbaru-dan-tersakit-00-rxg23-lwjg05): useful frontal head, face, shoulders, chest and arms. Local image: `references/ggwp_2026_revamp.jpg`.
2. [January 2026 splash reproduced by IDN Times](https://www.idntimes.com/tech/games/hero-eudora-dapat-revamp-patch-terbaru-mlbb-c1c2-01-drfp5-n9bpv0): useful three-quarter face, ears, hair, sleeve construction and chest emblem. Local: `references/idntimes_2026_revamp.jpg`.
3. [November 2025 advanced-server model preview, esports.gg](https://esports.gg/news/mobile-legends-bang-bang/mlbb-eudora-revamp-advanced-server-preview-release-date/): full-body view, including coat tails, leg panels and pointed footwear. Local: `references/esports_2025_preview.jpg`. The release-period upper-body references agree with its main silhouette, but this is still a preview; unseen release-model rear details are not certified.
4. [Official hero page](https://www.mobilelegends.com/hero/detail?heroid=15) and its older public CDN painting: inspected to distinguish versions. `references/official_cdn_painting.png` has the older swept hairstyle and cyan waist ornaments. Do not treat it as the authoritative revamped costume sheet.

Image-search captions were not used as visual proof. In particular, references described by search as icy do not make Eudora an ice mage. Her element is lightning. The official spotlight video was found during Phase 0 but could not be inspected frame by frame in this environment.

## Head to toe

| Region | Observed features to preserve | Treatment at 45 pixels |
|---|---|---|
| Hair | Short silver/white swept crest with curled locks, darker silver roots/shadows and a small longer nape section | Three to four silver shades, readable crest; avoid a long curtain covering the face |
| Face | Pale skin, violet-lit eyes, dark eye definition, dark plum lips, focused expression | Pale peach face cluster, violet eye pixels and a small mouth; do not turn her into a masked knight |
| Ears | Long pointed elf ears exposed beside the hair | One strongly readable pointed near ear; far ear may be occluded by the three-quarter view |
| Neck/shoulders | High dark collar and sharp, layered silver shoulder/collar blades | Silver outward points around a dark neckline; these are costume elements, not a crown |
| Chest | Dark navy central panel with a bright cyan branching lightning emblem | Small cyan chest symbol contrasting with the white lapels |
| Coat | White/light-gray crossing front panels, long split tails, dark inner lining and angular trim | White front framing the navy torso, two distinct tail shapes; preserve gaps between coat and legs |
| Arms/hands | Pale upper sleeve sections, dark fitted forearm pieces, blue/cyan accents; hands cast without a staff | Dark-blue forearm shapes with cyan accents; empty open hands |
| Waist/hips | Angular white panels over the dark fitted suit | Simple overlapping white panels; no invented belt pouches or weapon holster |
| Legs | Fitted royal/navy blue leg panels, long cyan accents | Two distinct blue legs with a few cyan highlights |
| Feet | Narrow pointed blue footwear with cyan edges, visible in the preview | Pointed blue boots, cyan toe/edge pixels; retain both feet in the base |

No staff, wand, sword, shield or separate crown is present in these default-design references. No halo, ice block or permanent lightning cloud belongs in the character base. World VFX are separate assets.

## Selected base and measurements

Model: SpriteCook `pixel-engine-image-v1`, exactly as requested. This model takes text only; the above visual observations were translated into its prompt. No external reference image was uploaded to SpriteCook.

- First take: `b0d80fbd-b828-476a-a0b0-cde78939fafe`, 48×48 PNG, visible height 48, 27 colors, binary alpha. Rejected for the 45-pixel requirement and heavy face-framing hair. Retained separately for the record, not the animation source.
- Selected take: `139bd6da-7e21-406e-9d14-255c99d456f3`, 40×45 PNG, occupied box `(9, 0, 31, 45)`, therefore **22×45 visible pixels**. 26 RGBA colors including transparency, alpha extrema 0/255, **zero semitransparent pixels**.
- `art/eudora_base_45px.png`: untouched selected SpriteCook output.
- `art/eudora_master_22x45.png`: transparent margins cropped only; no resampling or redrawing.
- `art/eudora_base_72x72.png`: selected output padded only, positioned at `(16,11)`. Visible box `(25,11,47,56)`, so the feet end on **row 55**, matching the roster contract. Anchor x=0.5; engine anchor y follows the measured 0.7639 convention.
- `art/eudora_base_45px_zoom.png`: nearest-neighbor inspection preview, not an animation input.

The selected sprite reads as the intended silver-haired elf mage and retains the white coat, navy suit, cyan chest and boot accents. Fine shoulder layers, earrings and individual hair locks are simplified at this scale. This is a pixel adaptation, not a claim of an exact model reproduction. Facing is slightly toward the viewer's right; do not flip it silently.

The head is intentionally enlarged for the roster's chibi presentation. No further resizing is needed before PixelLab. In particular, do not use the older large-master “divide by 10” settings on this already-native-size base.

## Credit record

Primary had 2 credits; no generation was charged there. The previously authorized qwerty account started with 36 credits. Each Pixel Engine still costs 6 credits. Two character takes used 12; four shared VFX bases used the remaining 24. The vertical strike was a free rotation of the horizontal thread; it used no generation credits. No SpriteCook animation jobs were requested: animation remains the owner's PixelLab step.

Important asset IDs and local source hashes are in `art/spritecook-assets.json`. Signed download URLs are intentionally excluded from that manifest.
