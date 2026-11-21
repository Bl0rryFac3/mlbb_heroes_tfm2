# MLBB Heroes (`mlbb_heroes`) — Teamfight Manager 2 Mod Pack

![Game Version](https://img.shields.io/badge/TFM2-v0.7.3%2B-orange)
![Mod Type](https://img.shields.io/badge/Mod_Type-Native_Rust_%2B_Data-blue)
![Version](https://img.shields.io/badge/Version-2.7.0-brightgreen)
![Author](https://img.shields.io/badge/Author-Bl0rryFac3-crimson)

A unified **Mobile Legends: Bang Bang (MLBB)** hero expansion pack for **Teamfight Manager 2 (`v0.7.3+`)**, combining custom pixel-art sprites, multi-frame VFX, custom SFX, localization (`EN`, `KO`, `JA`, `ZH-Hans`, `ZH-Hant`, `FR`, `RU`, `VI`), and native Rust (`mod-api-stable`) gameplay mechanics into a single modular package (`mods/mlbb_heroes/`).

---

## Roster (`v2.7.0`)

26 heroes. The Mods menu shows build **1.1.164**.

| Hero | ID | Role / Category | Implementation | Key Mechanics |
| :--- | :--- | :--- | :--- | :--- |
| **Aldous** *(Soul Contractor)* | `mlbb_aldous` | Melee (`AD`, `Tank`, `Shield`, `CC`) | Data-Driven + Native Companion (`v4.3.0`) | **Soul Steal** stacks (max `1,000`) add `+2.5` raw damage each to Skill 1 (`+2,500` at cap), **3D Crimson Shield** passive, AoE stun barrier, and global **Chase Fate** spiral drill dash. |
| **Fanny** *(Blade Dancer)* | `mlbb_fanny` | Assassin (`Melee`, `AD`) | Native Rust Wall-Cable Engine (`v0.2.0`) | Raycasted **Steel Cable** wall-grappling (`165,000` range, 2-hop lookahead chaining, outer lane + jungle walls), **5-Charge Stack System** on Skill 2 badge, mid-flight **Tornado Strike**, **Prey Marks**, and **Cut Throat** finisher. |
| **Lancelot** | `mlbb_lancelot` | Assassin (`Melee`, `AD`) | Data-Driven + Native AI/Effects | **Soul Cutter** timed stacks (`+7.5%` damage each, up to `+30%`); target-following **Puncture** and **Phantom Execution**; three-wave **Thorned Rose**. |
| **Estes** *(Moon Elf King)* | `mlbb_estes` | Support (`AP`, `Heal`, `Shield`, `Range`) | Data-Driven + Native Tether Renderer | **Moonlight Immersion** healing link, **Domain of Moon Goddess** area effect, and **Blessing of Moon Goddess** team healing. |
| **Gusion** *(Dagger Specialist)* | `mlbb_gusion` | Assassin (`AP`, `Melee`) | Data-Driven + Native Rust Passive/Effects | Four-rune empowered hit; **Sword Spike** mark and recast dash; five-dagger **Shadowblade Slaughter**; two-phase **Incandescence**. |

| **Alpha** | `mlbb_alpha` | Melee | Data + native Rust | Beta follows marked targets and skill paths with true damage. |
| **Angela** | `mlbb_angela` | Util | Data + native Rust | Heals and marks enemies with **Love Waves** and attaches to an ally with **Heartguard**. |
| **Arlott** | `mlbb_arlott` | Melee | Data + native Rust | Crowd control marks enemies. **Vengeance** resets and heals against marked targets. |
| **Atlas** | `mlbb_atlas` | Tank | Data + native Rust | **Perfect Match** separates the pilot from his suit. **Fatal Links** pulls and slams nearby enemy champions. |
| **Balmond** | `mlbb_balmond` | Melee | Data + native Rust | Heals on kills and assists. **Cyclone Sweep** shreds armor and **Lethal Counter** scales with missing HP. |
| **Belerick** | `mlbb_belerick` | Tank | Data + native Rust | Returns damage with **Deadly Thorns** and taunts enemies with spreading vines. |
| **Cecilion** | `mlbb_cecilion` | Magician | Data + native Rust | Skill hits build permanent Magic Power. **Sanguine Bats** supports extended fights. |
| **Chou** | `mlbb_chou` | Melee | Data + native Rust | Three-stage **Jeet Kune Do**, a defensive dash, and a targeted kick. |
| **Eudora** | `mlbb_eudora` | Magician | Data + native Rust | Skill hits mark targets for powered-up lightning, control and a delayed ultimate burst. |
| **Franco** | `mlbb_franco` | Tank | Data + native Rust | Hooks enemies, slows nearby targets and suppresses a champion with **Bloody Hunt**. |
| **Gord** | `mlbb_gord` | Magician | Data + native Rust | Repeated skill hits trigger bonus true damage. His ground field and beam hit in pulses. |
| **Granger** | `mlbb_granger` | Range | Data + native Rust | Separate Gun and Cannon Energy pools power a homing barrage and explosive cannon shot. |
| **Hanabi** | `mlbb_hanabi` | Range | Data + native Rust | Bouncing Petal Blades, a defensive shield and a projectile that roots nearby enemies when it blooms. |
| **Hanzo** | `mlbb_hanzo` | Assassin | Data + native Rust | Builds marks and projects **Hanekage**, which can cross terrain and strike with **Soul Snatch**. |
| **Johnson** | `mlbb_johnson` | Tank | Data + native Rust | Armor-scaling attacks and a car transformation that can carry an ally into an enemy. |
| **Julian** | `mlbb_julian` | Melee | Data + native Rust | Casting two different skills enhances the third and starts a shared cooldown. |
| **Karrie** | `mlbb_karrie` | Range | Data + native Rust | Five Lightwheel Marks trigger true damage. **Speedy Lightwheel** boosts attack and movement speed. |
| **Obsidia** | `mlbb_obsidia` | Range | Data + native Rust | Builds Bone Energy and tethers to the first enemy champion caught by **Hunt of Bone**. |
| **Popol & Kupa** | `mlbb_popol_kupa` | Range | Data + native Rust | A spear fighter and companion who coordinate attacks, traps and **Alpha Wolf Form**. |
| **Uranus** | `mlbb_uranus` | Tank | Data + native Rust | Builds **Radiance** when damaged and recovers health through **Consecration**. |
| **Zhask** | `mlbb_zhask` | Magician | Data + native Rust | Places a stationary Spawn and can merge with its empowered form. |

---

## Hero Details

### 1. Aldous — Soul Contractor (`mlbb_aldous`)
- **Passive — `Contract: Transform`**: Every third basic attack, Aldous converts the energy of his gauntlet into a **3D Crimson Shield** that scales with his **Soul Steal** stacks.
- **Skill 1 — `Contract: Soul Steal`**:
  - Casts a targeted molten-force slam (`can_use_with_move: true`, `cancelable: false`) on enemy champions and units; Towers/Base are excluded by the native damage effect.
  - Soul Steal stacks are capped at **1,000** and add **2.5 raw Skill 1 damage per stack** (`+2,500` at cap). Nearby enemies take **40%** of the primary damage.
- **Skill 2 — `Contract: Explosion`**:
  - Gains a **90 shield** and **25% Movement Speed** for `2s`, then detonates to deal **70 + 100% AD** physical damage and **stun** nearby enemies for `1s`.
- **Ultimate — `Contract: Chase Fate`**:
  - Flies to the targeted enemy champion in a golden-crimson spiral, dealing **130 + 160% AD** physical damage and stunning for **1.2s** on impact.

---

### 2. Fanny — Blade Dancer (`mlbb_fanny`)
- **Passive — `Air Superiority` (`mlbb_heroes:air_superiority`)**:
  - While flying via **Steel Cable**, Fanny gains **+10% to +25% bonus damage** based on flight speed and applies up to **2 Prey Marks** (`5s` duration) to enemy champions hit.
- **Skill 1 — `Tornado Strike`**:
  - Whirls twin blades in a `34,000` radius for `35 + 80% AD` physical damage.
  - Colliding with an enemy during Steel Cable triggers a mid-air strike for `40 + 70% AD` physical damage and applies Prey Marks to enemy heroes.
- **Skill 2 — `Steel Cable` (`mlbb_heroes:cable` + `mlbb_heroes:cable_ai`)**:
  - Fires a grappling claw hook (`"hook"`) and directional thin steel wire (`"wire_0".."wire_11"`) into arena obstacles or outer lane walls within `165,000` range and pulls Fanny at `5,200` speed.
  - **Visible 5-Charge Stack System (`stack_skill_index: 1`)**: Holds up to **5 Cable Charges** displayed directly on her Skill 2 icon. Regenerates one charge every 95 ticks while grounded.
  - **2-Hop Lookahead AI**: Evaluates 72 raycast angles and 2-hop wall chains (`Priority 1`: visible Enemy Champions; `Priority 2`: nearby Jungle Monsters when no enemy champions are visible).
- **Ultimate — `Cut Throat` (`mlbb_heroes:cut_throat`)**:
  - Dashes to an enemy for `100 + 125% AD` physical damage. Each Prey Mark adds `35% AD`, up to two marks, and the hit consumes them.

---

### 3. Lancelot (`mlbb_lancelot`)
- **Passive — `Soul Cutter`**:
  - Each **Puncture** or **Phantom Execution** charge grants a timed stack. Each stack increases Lancelot's damage by **7.5%** for **4 seconds**, up to **4 stacks** (**30%** total).
- **Skill 1 — `Puncture`**:
  - Uses a target-following `MoveToTarget` dash and applies damage on arrival. The first unmarked enemy receives the Sword Mark; a one-tick-deferred reset clears the Skill cooldown. A short blue dash streak is played separately from the target mark.
- **Skill 2 — `Thorned Rose`**:
  - Sends three directional line waves at `0`, `7`, and `14` ticks. Each hit deals damage and slows by **20%** for `0.5s`; overlapping enemies receive the configured area-hit damage factor. A brief damage/CC-immunity window is approximated with a timed buff. The triangular effect baked into Lancelot's character animation remains separate from the skill VFX.
- **Ultimate — `Phantom Execution`**:
  - Performs a target-locked dash to an enemy champion, applying its hit effect on arrival. The supplied slash VFX follows the impact point and is mirrored to match the approach direction.
- **Basic attack**:
  - Standard `100% AD` hit with a separate, brief blue-streak VFX.

---

### 4. Estes — Moon Elf King (`mlbb_estes`)
- **Passive — `Scripture of the Moon Elf`**:
  - Builds up to **100 Scripture stacks** over time, charging faster while Estes has an active link. A fully charged basic attack deals bonus damage, can ricochet to nearby enemies, and slows its targets.
- **Skill 1 — `Moonlight Immersion`**:
  - Heals a targeted ally on cast and periodically afterward, grants the ally bonus defenses, and gives Estes a movement-speed bonus.
  - The tether uses Estes-inspired artwork rendered as overlapping directional sprites between the live caster and ally positions. The renderer is included in the release.
- **Skill 2 — `Domain of Moon Goddess`**:
  - Creates a stationary area that periodically damages and slows enemies inside it.
- **Ultimate — `Blessing of Moon Goddess`**:
  - Heals Estes and nearby allies, continues healing over time, grants bonus defenses, and displays the Moon Goddess aura.
- **Art, icons & audio**:
  - Includes custom Estes sprites and skill effects, official skill icons, localized descriptions, and the available default voice/SFX clips.

---

### 5. Gusion — Dagger Specialist (`mlbb_gusion`)
- **Passive — `Dagger Specialist`**:
  - Gusion builds up to **4 Rune stacks** through his skill phases. His next basic attack consumes the stacks. Each Rune adds magic damage equal to **3% of the target’s maximum HP** and heals Gusion for **50 + 25% AP**.
- **Skill 1 — `Sword Spike`**:
  - Throws a target-tracking dagger. On hit it deals magic damage, marks the target, and opens the recast. The recast dashes through the marked enemy and deals additional damage.
- **Skill 2 — `Shadowblade Slaughter`**:
  - Throws **five daggers** in a fan with separate trails. The recast sends them back toward Gusion; each dagger can hit on the outward and return paths.
- **Ultimate — `Incandescence`**:
  - A two-phase targeted dash with an arrival effect on each phase. The first cast also resets Gusion's other skill cooldowns in the mod data.
- **Basic attack**:
  - Uses one strike arc from seven retained poses (0.08 seconds per pose); action duration is 28 ticks, hit timing is tick 8, and cooldown is 60 ticks.
- **Art, icons & audio**:
  - Uses the default Gusion Skill 1, Skill 2, and Ultimate icons, custom skill VFX, four default-English voice clips, original action SFX, and localized descriptions.


---

### 6. Atlas — Ocean Gladiator (`mlbb_atlas`)

- **Passive — `Frigid Breath`**. Skill casts grant armor and magic resistance and create a frost aura. Enemies that stay inside long enough briefly freeze.
- **Skill 1 — `Annihilate`**. Slams three times. The suit also slams while separated, with stronger hits where the two areas overlap.
- **Skill 2 — `Perfect Match`**. Ejects the pilot and gains movement speed. The suit follows and stuns nearby enemies when it rejoins. Cooldown is **10s**.
- **Ultimate — `Fatal Links`**. Chains nearby enemy champions, then pulls and briefly knocks them airborne. Casting while ejected rejoins the suit immediately. Cooldown is **34s**.

---

## Installation

1. Download [the Windows release ZIP](https://github.com/Bl0rryFac3/mlbb_heroes_tfm2/releases/tag/v2.7.0) and extract the `mlbb_heroes` folder into your **Teamfight Manager 2** `mods/` directory so the path is:
   ```text
   Teamfight Manager 2/mods/mlbb_heroes/
   ```
2. *(Important)* If you previously installed standalone Aldous, Fanny, Fanny Wall, Lancelot, Estes, or Gusion mods, remove or disable them so their champion IDs do not conflict with `mlbb_heroes`.
3. Launch **Teamfight Manager 2** and enable **MLBB Heroes** in the in-game **Mod Manager**. Check for build **1.1.164**.
4. See [Install_Guide.html](Install_Guide.html) for the screenshot guide. The optional [MLBB Champ Info companion](https://github.com/Bl0rryFac3/mlbb_champ_info_tfm2) adds basic attack and passive cards.

---

## Repository Layout

```text
mlbb_heroes/
├── mod.mod_info                     # Unified mod metadata
├── mod.override_info                # Text/view merges and audio overrides
├── mlbb_heroes.dll                  # Precompiled Windows x64 native Rust module
├── thumbnail.png                    # Workshop / Mod Manager preview icon
├── champion/
│   ├── mlbb_aldous.data_champion
│   ├── fanny.data_champion
│   ├── mlbb_lancelot.data_champion
│   ├── mlbb_estes.data_champion
│   └── …                            # 26 hero definitions
├── champions/                       # Aldous champion animation
├── sprite/                          # Fanny, Lancelot, Estes, and Gusion atlases
├── effects/                         # Hero skill effects, including Estes tether and Gusion VFX
├── icons/                           # Skill and passive icons
├── sound/sfx/                       # Installed SFX/voice clips and sound_info mappings
├── style/                           # Champion portrait and ban/pick framing
├── text/                            # Localized names and skill tooltips
├── source/                          # Source art, audio, and asset manifests
├── sdk/candidate/                   # Native Rust crate, lockfile and build scripts
├── sdk/mod-api-stable/              # Matching stable SDK
└── tools/                            # Asset generation and clean release ZIP scripts
```

---

## v2.7.0 Notes & Validation

- Atlas joins as hero 26 with pilot and suit animations, three skill icons and cast audio.
- Granger, Hanzo and Eudora received damage updates. Granger honors the game's existing return-to-base orders.
- Animated hover previews for Atlas, Granger, Hanzo and Popol & Kupa are restored in match ban/pick.
- Build 1.1.164 passed 552 Rust tests and 104 champion-data checks. [Release notes](docs/GITHUB_RELEASE_v2.7.0.md) · [Build report](docs/BUILD_REPORT_v2.7.0.md)

### Earlier v1.0.4 notes

- Refreshed Gusion's default skill icons, voice bindings, passive/skill effects, and localized descriptions. Raised the passive rune and Skill 1 mark framing, reduced the Ultimate arrival effect to Gusion's scale, and added a looping asset-staged showcase preview.
- Aldous Skill 1 gains **2.5 raw damage per Soul Steal stack**, capped at **1,000 stacks** (+2,500 raw damage at cap). Aldous's original basic-attack timing was restored: four poses at 0.065 seconds each, 14-tick action, hit at tick 2, and 60-tick cooldown.
- Lancelot's damage coefficients were tuned against Fanny as a burst benchmark; the coefficient tests are included in the Rust suite.
- Updated champion-view framing for Fanny, Lancelot, Gusion, and Estes using Aldous as the reference; Aldous's existing entry remains unchanged.


## Adding More MLBB Heroes

Because `mlbb_heroes` is structured as a multi-hero pack, you can add new MLBB heroes directly into this repository:

1. **Champion Definition**: Add `champion/mlbb_<hero>.data_champion` referencing `"asset/mlbb_heroes/..."` paths.
2. **Sprites & VFX**:
   - Place the hero sprite sheet and animation in `champions/mlbb_<hero>#sheet.png` and `champions/mlbb_<hero>#anim.fanim` (or `sprite/`).
   - Place skill icons in `icons/mlbb_<hero>_{skill,skill2,ult}.png`.
   - Place VFX sheets in `effects/mlbb_<hero>_fx#sheet.png` and `effects/mlbb_<hero>_fx#anim.fanim`.
3. **UI Framing & Localization**:
   - Add an entry for `"mlbb_<hero>"` under `"entries"` in `style/champion_view.champion_view`.
   - Add `"mlbb_<hero>"` under `"description"` and `"skill_name"` in `text/champion.i18n`.
4. **Optional Native Rust Abilities**:
   - Implement `StableEffectType`, `StablePassive`, `StableAction`, or `StablePlayerAi` in `sdk/candidate/` and register them in `sdk/candidate/lib.rs` (`module.add_native_effect("mlbb_heroes:<name>", ...)`).

---

## Building From Source (Optional)

`mlbb_heroes.dll` is included as a precompiled Windows x64 library; v2.7.0 was built with the Windows GNU target. Rebuild from the native crate after editing `sdk/candidate/lib.rs`:

- **Windows (PowerShell)**:
  ```powershell
  .\sdk\candidate\build.ps1
  ```
- **Linux / macOS**:
  ```bash
  sh sdk/candidate/build.sh
  ```

---


Run `cargo test --locked --manifest-path sdk/candidate/Cargo.toml` to test the native crate.

## Credits & Disclaimer

- **Author**: `Bl0rryFac3`
- **Disclaimer**: Non-commercial fan-made mod for *Teamfight Manager 2*. *Mobile Legends: Bang Bang* characters and names are property of Moonton.
