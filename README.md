# MLBB Heroes (`mlbb_heroes`) — Teamfight Manager 2 Mod Pack

![Game Version](https://img.shields.io/badge/TFM2-v0.6.4%2B-orange)
![Mod Type](https://img.shields.io/badge/Mod_Type-Native_Rust_%2B_Data-blue)
![Version](https://img.shields.io/badge/Version-1.0.2-brightgreen)
![Author](https://img.shields.io/badge/Author-Bl0rryFac3-crimson)

A unified **Mobile Legends: Bang Bang (MLBB)** hero expansion pack for **Teamfight Manager 2 (`v0.6.4+`)**, combining custom pixel-art sprites, multi-frame VFX, custom SFX, localization (`EN`, `KO`, `JA`, `ZH-Hans`, `ZH-Hant`), and native Rust (`mod-api-stable`) gameplay mechanics into a single modular package (`mods/mlbb_heroes/`).

---

## Roster (`v1.0.2`)

| Hero | ID | Role / Category | Implementation | Key Mechanics |
| :--- | :--- | :--- | :--- | :--- |
| **Aldous** *(Soul Contractor)* | `mlbb_aldous` | Warrior / Assassin (`Melee`, `AD`, `Shield`, `CC`) | Data-Driven + Native Companion (`v4.3.0`) | Permanent **Soul Steal** stacking (`+2` on hit, `+9` on kill, max `1,000`, hits Towers), **3D Crimson Shield** passive, AoE stun barrier, and global **Chase Fate** spiral drill dash. |
| **Fanny** *(Blade Dancer)* | `mlbb_fanny` | Assassin (`Melee`, `AD`) | Native Rust Wall-Cable Engine (`v0.2.0`) | Raycasted **Steel Cable** wall-grappling (`165,000` range, 2-hop lookahead chaining, outer lane + jungle walls), **4-Charge Stack System** on Skill 2 badge, mid-flight **Tornado Strike**, **Prey Marks** (`+1` charge refund & `5% Max HP` heal), and **Cut Throat** finisher. |
| **Lancelot** | `mlbb_lancelot` | Assassin (`Melee`, `AD`) | Data-Driven + Native AI/Effects | **Soul Cutter** timed stacks (`+7.5%` damage each, up to `+30%`); target-following **Puncture** and **Phantom Execution**; three-wave **Thorned Rose**. |

---

## Hero Details

### 1. Aldous — Soul Contractor (`mlbb_aldous`)
- **Passive — `Contract: Transform`**: Every 2 basic attacks, Aldous converts the energy of his gauntlet into a **3D Crimson Shield** that scales with his **Soul Steal** stacks.
- **Skill 1 — `Contract: Soul Steal`**:
  - Enhances his next strike (`can_use_with_move: true`, `cancelable: false`, targets enemies **including Towers**) to slam the ground with molten force.
  - Grants permanent **Soul Steal** stacks (`+2` per hit, `+9` bonus on kill), capped at **1,000**, that permanently boost his Attack Damage.
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

## Installation

1. Download or clone this repository into your **Teamfight Manager 2** `mods/` directory so the folder path is:
   ```text
   Teamfight Manager 2/mods/mlbb_heroes/
   ```
2. *(Important)* If you previously installed standalone Aldous, Fanny, Fanny Wall, or Lancelot mods, remove or disable them so their champion IDs do not conflict with `mlbb_heroes`.
3. Launch **Teamfight Manager 2** and enable **MLBB Heroes** in the in-game **Mod Manager**.

---

## Repository Layout

```text
mlbb_heroes/
├── mod.mod_info                     # Unified mod metadata (mod_id: "mlbb_heroes", mod_type: "native")
├── mod.override_info                # Merges champion text/view data and package audio overrides
├── mlbb_heroes.dll                  # Precompiled Windows x64 native Rust module (TFM2 0.6.4+)
├── Cargo.toml                       # Rust crate manifest for mlbb_heroes.dll
├── build.ps1 / build.sh             # Windows PowerShell & Linux/macOS build scripts
├── thumbnail.png                    # Workshop / Mod Manager preview icon
├── champion/
│   ├── mlbb_aldous.data_champion    # Aldous champion definition
│   ├── fanny.data_champion          # Fanny champion definition (id: "mlbb_fanny")
│   └── mlbb_lancelot.data_champion  # Lancelot champion definition
├── champions/
│   └── mlbb_aldous#{sheet.png,anim.fanim}
├── sprite/
│   ├── fanny#{sheet.png,anim.fanim}
│   ├── lancelot#{sheet.png,anim.fanim}
│   └── source/                      # Raw action strips for Fanny
├── effects/
│   ├── mlbb_aldous_fx#{sheet.png,anim.fanim}
│   ├── fanny_hook#{sheet.png,anim.fanim}       # 416x32 Claw Hook + 12-angle Thin Steel Wire
│   ├── fanny_cable_wire#{sheet.png,anim.fanim}
│   ├── fanny_cable_start#{sheet.png,anim.fanim}
│   ├── fanny_cable_impact#{sheet.png,anim.fanim}
│   ├── fanny_tornado#{sheet.png,anim.fanim}
│   ├── fanny_cut_throat#{sheet.png,anim.fanim}
│   ├── fanny_prey_mark#{sheet.png,anim.fanim}
│   └── lancelot_*#{sheet.png,anim.fanim} # Puncture, Skill 2, Ultimate, and attack VFX
├── icons/
│   ├── mlbb_aldous_{skill,skill2,ult}.png
│   ├── fanny_{skill,skill2,ult}.png
│   └── lancelot_{passive,skill,skill2,ult}.png
├── sound/sfx/
│   └── MLBB action SFX and available voice clips + sound_info mappings
├── style/
│   └── champion_view.champion_view  # Portrait & ban/pick framing for the roster
├── text/
│   └── champion.i18n                # Localized names & skill tooltips (en, ko, ja, zh-hans, zh-hant)
├── source/
│   ├── audio/                       # Voice source clips, manifest, and mappings
│   └── lancelot/                    # Lancelot source art and frame manifests
└── src/
    ├── lib.rs                       # Native Rust gameplay logic and registrations
    ├── lancelot.rs                  # Lancelot native AI/effect callbacks
    ├── maps.rs                      # Arena & MOBA wall obstacle grids
    ├── aldous_companion.rs          # Aldous companion passive source
    └── crt_shim.rs                  # Windows x64 CRT / PE TLS directory shim
```

---

## v1.0.2 Notes & Validation

- Includes Lancelot alongside Aldous and Fanny; Aldous's Soul Steal cap is **1,000**.
- Bundles the available default voice clips for Lancelot, Fanny, and Aldous. Missing source lines are not fabricated; see `source/audio/README.md` and `source/audio/audio_manifest.json` for the mappings and gaps.

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

`mlbb_heroes.dll` comes precompiled for Windows x64 (`x86_64-pc-windows-msvc`). To rebuild after editing `src/lib.rs`:

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
