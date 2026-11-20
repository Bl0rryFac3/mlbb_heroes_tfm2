# Zhask v1.1.59 asset sources and rights note

## Action icon images

Fetched from the Mobile Legends Fandom wiki's default Zhask artwork. For packaging, the static CDN URLs used `&format=original`; each response was checked for `Content-Type: image/png` and the PNG signature:

- Nightmaric Spawn (Skill 1): https://mobile-legends.fandom.com/wiki/File:Nightmaric_Spawn.png
- Mind Eater (Skill 2): https://mobile-legends.fandom.com/wiki/File:Mind_Eater.png
- Dominator's Descent (Ultimate): https://mobile-legends.fandom.com/wiki/File:Dominator%27s_Descent.png
- Reference page: https://mobile-legends.fandom.com/wiki/Zhask

The mod format exposes only the three action icon slots; the Decimation passive image is not wired into the unsupported slot.

## Default English voice clips

Original OGG sources are retained under `source/zhask_fandom_audio_originals/`; packaged WAV versions use mono 22.05 kHz PCM to match the existing voice asset convention. Fandom's Zhask audio index: https://mobile-legends.fandom.com/wiki/Zhask/Audio

- Ultimate: https://mobile-legends.fandom.com/wiki/File:Zhask.new.ult01.ogg
- Ultimate: https://mobile-legends.fandom.com/wiki/File:Zhask.new.ult02.ogg
- Ultimate: https://mobile-legends.fandom.com/wiki/File:Zhask.new.ult03.ogg
- Death: https://mobile-legends.fandom.com/wiki/File:Zhask.new.death01.ogg
- Death: https://mobile-legends.fandom.com/wiki/File:Zhask.new.death02.ogg
- Death: https://mobile-legends.fandom.com/wiki/File:Zhask.new.death03.ogg
- Death: https://mobile-legends.fandom.com/wiki/File:Zhask.new.death04.ogg

**Rights warning:** the wiki/source pages identify the files and lines but do not establish permission to reuse or redistribute the underlying Mobile Legends art/audio. The audio is bundled in this candidate for testing; no licensing claim is made. Confirm rights before public redistribution.

## Custom effect art

The revised Mind Eater travel sprite and impact burst are custom procedural pixel-art assets created for this candidate, not downloaded from Fandom.

## Hanabi candidate assets

- The Hanabi character atlas and six VFX sprite sheets under `source/hanabi/import_2026-10-28/` were supplied by the user, who identified the VFX workflow as GPT Image 2.5 Sunburst. The candidate preserves those original files unchanged and records derived crops/scales in `HANABI_IMPLEMENTATION_NOTES.md`.
- The four 100×100 classic/default Hanabi ability icons in the same import folder were staged as transparent PNGs. Three active-skill icons are wired into the champion data; the passive icon is retained in `icons/` for reference because the project data schema exposes three active action-icon slots.
- No license or redistribution permission is asserted for Mobile Legends character/icon art or user-supplied generated assets. Confirm rights before public redistribution.

## Balmond candidate assets

- The Balmond body atlas and the `Balmond-spin.png`, Skill 1, Skill 2 ring, and Ultimate art were supplied by the user and are preserved in `source/balmond/` and `source/balmond/reference_effects/`. The generated runtime sprite/VFX animation metadata references these assets without replacing or regenerating the art.
- The classic/default action icons were fetched from the Mobile Legends Fandom Balmond page. Exact source URLs, byte sizes, and hashes are in `source/balmond/ASSET_SOURCES.json`.
- The default English Balmond voice set (selection, movement, basic exertion, Ultimate, death) is sourced from https://mobile-legends.fandom.com/wiki/Balmond/Audio. Exact source URLs and mappings are in `source/audio/audio_manifest.json`; original OGG files are retained under `source/audio/balmond/default/` and playback files are converted to mono 22.05 kHz PCM16 WAV.
- The page supplies voice/grunt clips, not distinct non-voice Skill 1/Skill 2 ability SFX. No voice lines were fabricated.

**Rights warning:** the source references identify provenance only and do not establish permission to redistribute user-supplied images, game art, icons, or game voice recordings. Confirm rights before public redistribution.
