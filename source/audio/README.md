# Voice clip sources and mapping

Original `.ogg` files are retained under `source/audio/<hero>/`; playback clips are converted to mono 22.05 kHz PCM16 WAV under `sound/sfx/`. The Estes clips come from the Default section of [Estes/Audio](https://mobile-legends.fandom.com/wiki/Estes/Audio). Karrie's Hawkwatch clips come from the user-provided [Karrie/Audio#Hawkwatch](https://mobile-legends.fandom.com/wiki/Karrie/Audio#Hawkwatch) page. These are source/provenance notes, not rights claims; game voice recordings may have separate rights.

## Runtime routing

Each `.sound_info` event uses a clip ID distinct from its event ID. Both are routed in `mod.override_info`, avoiding TFM2's reported same-event/same-clip ID failure mode. Action voices use champion-data `Sfx` effects. Idle/selection, movement/run, and available death voices use native passive lifecycle callbacks via `StableSim::play_sfx`.

The TFM2 API has no pre-match hero-selection-screen callback. Accordingly, the selection line plays on champion spawn and is reused as an occasional stationary idle bark. Movement is inferred from position changes: one voice on movement start, with an 8-second repeat throttle; idle barks begin after 12 seconds stationary and repeat no more often than every 20 seconds. Estes death clips and Karrie Hawkwatch death clips cycle on owner death. These heuristics and all playback still need in-game verification.

## Existing heroes

- **Lancelot:** selection/idle, movement/run, basic attack, Skill 1, Skill 2, and Ultimate audio are routed. Basic exertion clips are used as Skill 1/2 effort-grunt proxies because the page does not list dedicated skill clips. No death voice is present in the bundled source set.
- **Fanny:** selection/idle, movement/run, Skill 2, and Ultimate audio are routed. The Skill 2 voice is throttled by the existing cable-cast guard. No hero-specific Skill 1 or death voice is present in the bundled source set; existing action SFX remain intact.
- **Estes:** selection/idle, six movement/run lines, the shared Skill 1/Skill 2 voice, Ultimate, and three death lines are routed.
- **Aldous:** selection/idle, movement/run, Ultimate start, and Ultimate impact audio are routed. No bundled basic/Skill 1/Skill 2/death voice clips were found; existing action SFX remain intact.
- **Gusion:** the four available English voice lines are routed to Skill 1, Skill 2, and both Ultimate phases; seven original synthesized SFX remain routed to attacks, recasts, recalls, and action cues. No selection/movement/death clips are included in this source set.
- **Karrie Hawkwatch:** selection/idle, eight movement lines, two death lines, Skill 1, Skill 2, and both Ultimate lines are routed. The Ultimate lines play sequentially from one event.

No replacement voice lines were fabricated. See `audio_manifest.json` for exact event IDs, clip IDs, source filenames, and playback paths. Static/package checks do not prove audible behavior in TFM2; a match test is still required.
