# Julian — source + art prompt + Arena.ai handoff

Prepared 15 November 2026 (Asia/Manila). This is **not a playable mod release**. No paid images/animations were generated. It is the researched design, native source/tests, eight-locale text drafts, additive installer, returned-asset compiler and detailed receiving-agent instructions.

## For the owner

1. Open **ART_PROMPTS.md**. Generate ONE Julian master in SpriteCook using Pixel Engine Image v1, 72×72 transparent canvas and an approximately 45-pixel-tall character.
2. Use the eight character motion prompts in PixelLab, then generate/animate the nine VFX first frames. Keep native cells and real transparent PNGs. Filenames and sizes are listed in ASSET_MANIFEST.json.
3. Give Arena.ai **this entire handoff ZIP**, the **existing canonical source ZIP linked below**, and the returned PNG folder. Paste **ARENA_AI_MASTER_PROMPT.md** into that session. It is over 3,000 words of concrete project-specific instructions; the art prompt pack is approximately 3,900 words.
4. Arena.ai applies the tested patch, compiles the returned assets, builds a real Windows DLL, runs the release gates and prepares runtime/source packages. The owner then playtests and publishes personally.

Pinned source: [21-hero Eudora baseline, internal 1.1.122](https://drive.google.com/file/d/1lx7pDQGqdCs2ga7jznuC6YKU5VcJd_sB/view). Its exact SHA-256 is recorded in BASELINE.json. This handoff does not duplicate that 74.7 MB source archive. The new module is code/julian.rs; it is not pseudocode. The source installer adds only Julian, preserves older heroes and sets candidate metadata to 1.1.123. The proposed public label v2.3.0-rc.1 is not a published release.

## What is already done

- Default appearance and three-skill/enhancement identity researched; official portrait and default model visually inspected.
- Base-sprite prompt, eight body-motion prompts, nine VFX base prompts and nine VFX motion prompts written.
- Native three-skill sequence, six enhancement orders, traveling/slow-drifting Scythe, Sword path/banish adaptation, Chain bind/launch, boosted basic attack/pull, capped sustain, AI and simulation-copy ownership implemented.
- Native source compile check passes; full source suite passes **316 tests** (300 existing + 16 Julian).
- Additive source/data installer validated against the pinned baseline; all old description/name values are preserved in all eight locale sets. Julian schema check: 4 pass, 0 warnings, 0 failures; 333 unique view bindings.
- Two Python tool tests pass, including a synthetic native-grid compilation and a deliberate missing-art rejection. Synthetic fixtures are NOT artwork for the hero.
- Eight-locale draft skill descriptions supplied. English copy uses neutral wording. Non-English drafts are not claimed as native-speaker reviewed.

## Important deliberate adaptations

- TFM2 normally locks the third skill slot early. The supplied authoritative Chain assist supplies that part of the combo while the engine explicitly reports the slot locked; it does not claim to unlock the UI button or force its body animation.
- Native shared cooldown debt cannot be perfectly synchronized to all engine icons with this SDK. It lasts 9–6 seconds by level and is not shortened by item haste. Ordinary engine action timers can appear ready before native debt expires.
- Enhanced Sword is a destination blink followed by 0.6 seconds of banish and repeated path damage, not movement applied while already banished.
- Map landing checks use the existing project's grid inference/cache and still need live tests in supported modes.
- Audio is silent pending approved clips. Skill icons are intentional derived crops of accepted returned effects, not blank stand-ins.

These limitations are explicit so the receiving agent does not “fix” them by inventing SDK fields or silently weakening the kit. See JULIAN_SPEC.md for the full contract and VALIDATION.md for evidence boundaries.

## Receiving-agent quick start

With HANDOFF, ROOT and INBOX replaced by actual paths:

```sh
python HANDOFF/tools/apply_julian.py ROOT
python HANDOFF/tools/validate_julian.py ROOT
python HANDOFF/tools/compile_assets.py ROOT INBOX
cd ROOT/sdk/candidate
cargo test --offline --lib
cargo build --release --target x86_64-pc-windows-msvc
```

The Windows command needs a configured target/linker/toolchain. Copy the actual built DLL to ROOT/mlbb_heroes.dll, verify its stable entry export/load, then run validate_julian.py ROOT --release and the existing baseline gates. The previous agent did **not** perform these final art-dependent Windows release steps. A Linux artifact renamed to .dll is not acceptable.
