# MLBB Heroes (`mlbb_heroes`) — Teamfight Manager 2 Mod Pack

![Game Version](https://img.shields.io/badge/TFM2-v0.6.4%2B-orange)
![Mod Type](https://img.shields.io/badge/Mod_Type-Native_Rust_%2B_Data-blue)
![Version](https://img.shields.io/badge/Version-1.0.4-brightgreen)
![Author](https://img.shields.io/badge/Author-Bl0rryFac3-crimson)

A unified **Mobile Legends: Bang Bang (MLBB)** hero expansion pack for **Teamfight Manager 2 (`v0.6.4+`)**, combining custom pixel-art sprites, multi-frame VFX, custom SFX, localization (`EN`, `KO`, `JA`, `ZH-Hans`, `ZH-Hant`), and native Rust (`mod-api-stable`) gameplay mechanics into a single modular package (`mods/mlbb_heroes/`).

---

## Roster (`v1.0.4`)

| Hero | ID | Role / Category | Implementation | Key Mechanics |
| :--- | :--- | :--- | :--- | :--- |
| **Aldous** *(Soul Contractor)* | `mlbb_aldous` | Warrior / Assassin (`Melee`, `AD`, `Shield`, `CC`) | Data-Driven + Native Companion (`v4.3.0`) | **Soul Steal** stacks (max `1,000`) add `+2.5` raw damage each to Skill 1 (`+2,500` at cap), **3D Crimson Shield** passive, AoE stun barrier, and global **Chase Fate** spiral drill dash. |
| **Fanny** *(Blade Dancer)* | `mlbb_fanny` | Assassin (`Melee`, `AD`) | Native Rust Wall-Cable Engine (`v0.2.0`) | Raycasted **Steel Cable** wall-grappling (`165,000` range, 2-hop lookahead chaining, outer lane + jungle walls), **4-Charge Stack System** on Skill 2 badge, mid-flight **Tornado Strike**, **Prey Marks** (`+1` charge refund & `5% Max HP` heal), and **Cut Throat** finisher. |
| **Lancelot** | `mlbb_lancelot` | Assassin (`Melee`, `AD`) | Data-Driven + Native AI/Effects | **Soul Cutter** timed stacks (`+7.5%` damage each, up to `+30%`); target-following **Puncture** and **Phantom Execution**; three-wave **Thorned Rose**. |
| **Estes** *(Moon Elf King)* | `mlbb_estes` | Support (`AP`, `Heal`, `Shield`, `Range`) | Data-Driven + Native Tether Renderer | **Moonlight Immersion** healing link, **Domain of Moon Goddess** area effect, and **Blessing of Moon Goddess** team healing. |
| **Gusion** *(Dagger Specialist)* | `mlbb_gusion` | Assassin (`AP`, `Melee`) | Data-Driven + Native Rust Passive/Effects | Four-rune empowered hit; **Sword Spike** mark and recast dash; five-dagger **Shadowblade Slaughter**; two-phase **Incandescence**. |

---

## Hero Details

### 1. Aldous — Soul Contractor (`mlbb_aldous`)
- **Passive — `Contract: Transform`**: Every 2 basic attacks, Aldous converts the energy of his gauntlet into a **3D Crimson Shield** that scales with his **Soul Steal** stacks.
- **Skill 1 — `Contract: Soul Steal`**:
  - Casts a targeted molten-force slam (`can_use_with_move: true`, `cancelable: false`) on enemy champions and units; Towers/Base are excluded by the native damage effect.
  - Soul Steal stacks are capped at **1,000** and add **2.5 raw Skill 1 damage per stack** (`+2,500` at cap). At level 15, the 1,000-stack primary hit is **2,762 raw damage before defense**, enough for the level-15 squishy benchmark under the standard defense model; TFM2 runtime behavior remains unverified.
- **Skill 2 — `Contract: Explosion`**:
  - Assumes a defensive crimson stance for `2s` (`35%` incoming damage reduction + Move Speed boost), then detonates to deal physical AoE damage and **stun** nearby enemies.
- **Ultimate — `Contract: Chase Fate`**:
  - Locks the **Eye of Fate** (`eye_mark`) onto the lowest-HP enemy champion anywhere on the arena and drills across the map as a 7-frame golden-crimson spiral meteor, knocking back and stunning on impact.

---

### 2. Fanny — Blade Dancer (`mlbb_fanny`)
- **Passive — `Air Superiority` (`mlbb_heroes:air_superiority`)**:
  - While flying via **Steel Cable**, Fanny gains **+15% to +30% bonus damage** based on flight speed and applies up to **2 Prey Marks** (`5s` duration) to enemy champions hit.
  - Striking an enemy champion with a Prey Mark **refunds +1 Cable Charge** and **restores 5% Max HP**.
- **Skill 1 — `Tornado Strike`**:
  - Whirls twin blades in a `34,000` radius for `95 + 135% AD` physical damage (`+15%` per Prey Mark).
  - Automatically triggers mid-flight when passing within `28,000` units of an enemy; changing flight direction (`dot < 0.94`) resets the pass-through trigger on the same target.
- **Skill 2 — `Steel Cable` (`mlbb_heroes:cable` + `mlbb_heroes:cable_ai`)**:
  - Fires a grappling claw hook (`"hook"`) and directional thin steel wire (`"wire_0".."wire_11"`) into arena obstacles or outer lane walls within `165,000` range and pulls Fanny at `5,200` speed.
  - **Visible 4-Charge Stack System (`stack_skill_index: 1`)**: Holds up to **4 Cable Charges** displayed directly on her Skill 2 icon (passively regenerates `1` charge every `5s` / `100` ticks).
  - **2-Hop Lookahead AI**: Evaluates 72 raycast angles and 2-hop wall chains (`Priority 1`: visible Enemy Champions; `Priority 2`: nearby Jungle Monsters when no enemy champions are visible).
- **Ultimate — `Cut Throat` (`mlbb_heroes:cut_throat`)**:
  - High-priority (`Priority 0`) lunging execution within `65,000` range dealing `180 + 190% AD` physical damage (`+45% AD` per Prey Mark, consuming marks on hit).

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
  - The tether uses Estes-inspired artwork rendered as overlapping directional sprites between the live caster and ally positions. The new renderer is included in this release; **its visibility and continuity still need in-game testing**.
- **Skill 2 — `Domain of Moon Goddess`**:
  - Creates a stationary area that periodically damages and slows enemies inside it.
- **Ultimate — `Blessing of Moon Goddess`**:
  - Heals Estes and nearby allies, continues healing over time, grants bonus defenses, and displays the Moon Goddess aura.
- **Art, icons & audio**:
  - Includes custom Estes sprites and skill effects, official skill icons, localized descriptions, and the available default voice/SFX clips.

---

### 5. Gusion — Dagger Specialist (`mlbb_gusion`)
- **Passive — `Dagger Specialist`**:
  - Gusion builds up to **4 Rune stacks** through his skill phases. At 4 stacks, his next basic attack is empowered, deals bonus magic damage based on the target's max HP, heals Gusion, and consumes the stacks.
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

## Installation

1. Download the release ZIP and extract the `mlbb_heroes` folder into your **Teamfight Manager 2** `mods/` directory so the path is:
   ```text
   Teamfight Manager 2/mods/mlbb_heroes/
   ```
2. *(Important)* If you previously installed standalone Aldous, Fanny, Fanny Wall, Lancelot, Estes, or Gusion mods, remove or disable them so their champion IDs do not conflict with `mlbb_heroes`.
3. Launch **Teamfight Manager 2** and enable **MLBB Heroes** in the in-game **Mod Manager**.

---

## Repository Layout

```text
mlbb_heroes/
├── mod.mod_info                     # Unified mod metadata
├── mod.override_info                # Text/view merges and audio overrides
├── mlbb_heroes.dll                  # Precompiled Windows x64 native Rust module
├── Cargo.toml / Cargo.lock          # Rust crate manifest and lockfile
├── build.ps1 / build.sh             # Windows PowerShell and Linux/macOS build scripts
├── thumbnail.png                    # Workshop / Mod Manager preview icon
├── champion/
│   ├── mlbb_aldous.data_champion
│   ├── fanny.data_champion
│   ├── mlbb_lancelot.data_champion
│   ├── mlbb_estes.data_champion
│   └── mlbb_gusion.data_champion
├── champions/                       # Aldous champion animation
├── sprite/                          # Fanny, Lancelot, Estes, and Gusion atlases
├── effects/                         # Hero skill effects, including Estes tether and Gusion VFX
├── previews/gusion/                 # Looping asset-staged Gusion showcase
├── icons/                           # Skill and passive icons
├── sound/sfx/                       # Installed SFX/voice clips and sound_info mappings
├── style/                           # Champion portrait and ban/pick framing
├── text/                            # Localized names and skill tooltips
├── source/                          # Source art, audio, and asset manifests
├── src/                             # Native Rust gameplay and registrations
└── tools/                            # Asset generation and clean release ZIP scripts
```

---

## v1.0.4 Notes & Validation

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
   - Implement `StableEffectType`, `StablePassive`, `StableAction`, or `StablePlayerAi` in `src/` and register them in `init()` inside `src/lib.rs` (`module.add_native_effect("mlbb_heroes:<name>", ...)`).

---

## Building From Source (Optional)

`mlbb_heroes.dll` is included as a precompiled Windows x64 library; the current release candidate uses the GNU toolchain build. To rebuild after editing `src/lib.rs`:

- **Windows (PowerShell)**:
  ```powershell
  .\build.ps1
  ```
- **Linux / macOS**:
  ```bash
  ./build.sh
  ```

---


## Credits & Disclaimer

- **Author**: `Bl0rryFac3`
- **Disclaimer**: Non-commercial fan-made mod for *Teamfight Manager 2*. *Mobile Legends: Bang Bang* characters and names are property of Moonton.
