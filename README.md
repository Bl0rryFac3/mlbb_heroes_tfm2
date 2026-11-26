# MLBB Heroes — Teamfight Manager 2

Adds 32 Mobile Legends heroes to Teamfight Manager 2, with pixel-art sprites, skill effects, and voice lines.

**New in v3.2.0 — Minotaur.** Minotaur is a tank support with a landing shockwave, a healing roar, and a three-hit ultimate. Minoan Fury triggers 12 seconds of Enraged. His champion-select idle, combat animations, official ability icons, and four English voice cues are included.

The compact voice bank keeps one recording per retained cue and preserves combat sound effects. The Windows package was playtested on Teamfight Manager 2 0.7.4. The Mods menu shows build **1.1.204**.

---

## Roster

| Hero | ID | Role / Category | Implementation | Key Mechanics |
| :--- | :--- | :--- | :--- | :--- |
| **Aldous** *(Soul Contractor)* | `mlbb_aldous` | Melee (`AD`, `Tank`, `Shield`, `CC`) | Data-Driven + Native Companion (`v4.3.0`) | **Soul Steal** stacks (max `1,000`) add `+2.5` raw damage each to Skill 1 (`+2,500` at cap), **3D Crimson Shield** passive, AoE stun barrier, and global **Chase Fate** spiral drill dash. |
| **Fanny** *(Blade Dancer)* | `mlbb_fanny` | Assassin (`Melee`, `AD`) | Native Rust Wall-Cable Engine (`v0.2.0`) | **Steel Cable** hooks reach 165,000 units and can chain with three-anchor lookahead. Fanny has 10 charges, mid-flight **Tornado Strike**, **Prey Marks**, and **Cut Throat**. |
| **Lancelot** | `mlbb_lancelot` | Assassin (`Melee`, `AD`) | Data-Driven + Native AI/Effects | **Soul Cutter** timed stacks (`+7.5%` damage each, up to `+30%`); target-following **Puncture** and **Phantom Execution**; three-wave **Thorned Rose**. |
| **Estes** *(Moon Elf King)* | `mlbb_estes` | Support (`AP`, `Heal`, `Shield`, `Range`) | Data-Driven + Native Tether Renderer | **Moonlight Immersion** healing link, **Domain of Moon Goddess** area effect, and **Blessing of Moon Goddess** team healing. |
| **Gusion** *(Dagger Specialist)* | `mlbb_gusion` | Assassin (`AP`, `Melee`) | Data-Driven + Native Rust Passive/Effects | Four-rune empowered hit; **Sword Spike** mark and recast dash; five-dagger **Shadowblade Slaughter**; two-phase **Incandescence**. |
| **Alpha** | `mlbb_alpha` | Melee | Data + native Rust | Beta follows Rotary Impact and Force Swing with true damage. His passive laser and ultimate volley deal physical damage. |
| **Angela** | `mlbb_angela` | Util | Data + native Rust | Heals and marks enemies with **Love Waves** and attaches to an ally with **Heartguard**. |
| **Arlott** | `mlbb_arlott` | Melee | Data + native Rust | Crowd control marks enemies. **Vengeance** resets and heals against marked targets. |
| **Atlas** | `mlbb_atlas` | Tank | Data + native Rust | **Perfect Match** separates the pilot from his suit. **Fatal Links** pulls and slams nearby enemy champions. |
| **Balmond** | `mlbb_balmond` | Melee | Data + native Rust | Heals on kills and assists. **Cyclone Sweep** shreds armor and **Lethal Counter** scales with missing HP. |
| **Belerick** | `mlbb_belerick` | Tank | Data + native Rust | Returns damage with **Deadly Thorns** and taunts enemies with spreading vines. |
| **Cecilion** | `mlbb_cecilion` | Magician | Data + native Rust | Skill hits build permanent Magic Power. **Bats Feast** damages nearby enemies and restores HP. |
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
| **Yu Zhong** *(Black Dragon)* | `mlbb_yu_zhong` | Melee | Data + native Rust | **Sha Residue** damages marked targets and heals Yu Zhong. **Black Dragon Form** lets him fly and extends his skill reach after landing. |
| **Lesley** *(Deadly Sniper)* | `mlbb_lesley` | Range | Data + native Rust | **Lethal Shot** primes after every skill. **Master of Camouflage** conceals her, **Tactical Grenade** creates space, and **Ultimate Snipe** fires four rounds. |
| **Valir** *(Son of Flames)* | `mlbb_valir` | Magician | Data + native Rust | **Ashing** detonates at three stacks. **Searing Torrent** pushes back enemies, and **Vengeance Flame** cleanses crowd control and empowers four casts. |
| **Sun** *(Monkey King)* | `mlbb_sun` | Melee Fighter | Data + native Rust | **Simian God** reduces Defense. **Endless Variety / Swift Exchange** and **Clone Techniques** each summon a clone. **Instantaneous Move** brings all three together. |
| **X.Borg** *(Firaga Armor)* | `mlbb_xborg` | Melee Fighter | Data + native Rust | A separate Firaga reserve controls armor. **Fire Missiles** builds heat while moving, **Fire Stake** pulls targets, and **Last Insanity** has a stronger blast against overheated enemies. |
| **Minotaur** *(Son of Minos)* | `mlbb_minotaur` | Tank / Support | Data + native Rust | **Despair Stomp** leaps and empowers two basics. **Motivation Roar** heals allies. **Minoan Fury** lands three slams and triggers Enraged. |

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
  - **Steel Cable charges**. Holds up to **10 charges**, visible on the skill icon. Charges return one at a time while grounded.
  - **Cable routing.** Fanny can plan up to three anchors ahead, including sideways hooks. Steel Cable can target movement waypoints during rotations and retreats.
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

### 7. Alpha (`mlbb_alpha`)

- **Passive — `Beta, Advance!`**. Every third eligible hit on an enemy champion calls Beta’s physical-damage laser and slows the target by 30% for 1s.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Rotary Impact`**. Sends an energy wave forward, dealing 70 + 80% Attack physical damage and slowing by 40% for 1s. Beta strafes the same path for true damage.
- **Skill 2 — `Force Swing`**. Sweeps forward for 100 + 80% Attack physical damage. Beta follows with true damage. Alpha heals for 100 + 20% Attack once per cast.
- **Ultimate — `Spear of Alpha`**. Beta’s dive deals 70 + 20% Attack physical damage and stuns for 1s. Alpha lands for 130 + 80% Attack physical damage with knock-up and slow. Beta then fires five physical hits of 8 + 10% Attack.

---

### 8. Angela (`mlbb_angela`)

- **Passive — `Smart Heart`**. Casting a skill grants Movement Speed for 4s, stacking up to 4 times for 7%, 15%, 22% or 30% total. While attached, her ally receives the bonus. Charges and cooldowns carry over when attaching or leaving.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Love Waves`**. Sends a wave that deals 80 + 30% AP magic damage to enemies and heals allies for 46 + 40% AP. Each enemy hit gains a Lover’s Mark for 3s, up to 5 stacks. Each adds 12% damage to later waves and Puppet-on-a-String’s final hit, and a 6% slow. Holds 5 charges. The last cast starts a 5s cooldown.
- **Skill 2 — `Puppet-on-a-String`**. Links to an enemy for 3s, dealing 95 + 25% AP magic damage and gradually slowing by up to 45%. If the link holds, deals another 140 + 35% AP magic damage, increased by Lover’s Marks, and roots for 0.7s. Moving too far apart or dying breaks the link without a final hit.
- **Ultimate — `Heartguard`**. Shields an allied hero anywhere on the map for 220 + 180% AP for 6s. Angela attaches to them for 12s, disappears from the battlefield and automatically casts Love Waves and Puppet-on-a-String from their position. A doll marks her ally. Recasting, the ally’s death or untargetability ends the attachment. Cooldown: 40s.

---

### 9. Arlott (`mlbb_arlott`)

- **Passive — `Demon Gaze`**. Marks the nearest enemy hero every 7s. His crowd-control effects also apply a mark. Marks last 7s and empower Vengeance.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Dauntless Strike`**. Slams his lance in a line, dealing 110 + 55% AD physical damage and applying Demon Gaze. Stuns for 0.5s nearby, increasing to 1s at the far edge.
- **Skill 2 — `Vengeance`**. Dashes to an enemy with interruption immunity, dealing 100 + 55% AD physical damage. Against a marked target, deals double damage, resets its cooldown and restores 30% AD HP, halved against non-heroes. Also marks the target.
- **Ultimate — `Final Slash`**. Sweeps his lances across a wide area, dealing 160 + 75% AD physical damage and dragging enemies to his right. Applies Demon Gaze to displaced enemies.

---

### 10. Balmond (`mlbb_balmond`)

- **Passive — `Bloodthirst`**. Restores 5% maximum HP after killing a minion or jungle monster, and 10% after a hero kill or assist.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Soul Lock`**. Charges toward the chosen position, dealing 150–275 + 60% AD physical damage along the path. Hits an enemy hero near the endpoint, briefly knocking them back and slowing by 40% for 3s.
- **Skill 2 — `Cyclone Sweep`**. Spins for 3s, dealing 14 hits of 24–44 + 1.3% maximum HP + 25% AD physical damage. Each hit reduces the target’s Armor by 5%, stacking up to 8 times. Gains 15% Movement Speed while spinning.
- **Ultimate — `Lethal Counter`**. Strikes a wide area in front of him for 150 + 70% AD + 30–45% of each target’s missing HP as true damage. Damage against non-heroes is capped at 1,000 / 1,500 / 2,000, based on level.

---

### 11. Belerick (`mlbb_belerick`)

- **Passive — `Deadly Thorns`**. Every fourth hit of at least 10 damage fires a thorn at the nearest enemy, dealing 10 + level + 1% maximum HP magic damage. Can trigger once every 1.2s. Tower attacks do not count.
- **Basic attack**. Basic attacks deal 100% AD physical damage. While empowered by Nature’s Strike, instead deals 60 magic damage, restores 150 + 12% missing HP and slows by 60% for 1.4s.
- **Skill 1 — `Ancient Seed`**. Sends vines forward for 115 magic damage and a 25% slow lasting 2s. After 1.1s, seeds burst along the same path for another 115 magic damage and taunt for 1.2s.
- **Skill 2 — `Nature's Strike`**. Gains 60% Movement Speed for 1s, then 20% for 1s, and empowers his next basic attack for 3s. Each Deadly Thorns activation reduces this skill’s cooldown by 1s.
- **Ultimate — `Wrath of Dryad`**. Releases spreading vines in 4 waves, each dealing 55 magic damage and taunting enemies for 1.5s.

---

### 12. Cecilion (`mlbb_cecilion`)

- **Passive — `Overflowing`**. Skill hits grant a permanent stack, up to once per second. Each grants 3 Magic Power. Stacks remain after death.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Bat Impact`**. Sends a giant bat forward, dealing 30 + 25% AP magic damage along its path. It dives at its destination for 55 + 50% AP magic damage nearby.
- **Skill 2 — `Sanguine Claws`**. Marks a location. After 0.6s, blood claws close in, dealing 60 + 60% AP magic damage, stunning for 1s and pulling enemies to the center.
- **Ultimate — `Bats Feast`**. Releases 40 blood bolts over 3s, each dealing 5 + 8% AP magic damage to a nearby enemy hero and restoring 8 HP. Cecilion can move while channeling and gains 60% Movement Speed for 1.5s.

---

### 13. Chou (`mlbb_chou`)

- **Passive — `Only Fast`**. Moving charges his next basic attack. When ready, it deals 180% AD physical damage, slows by 80% for 1s and reduces the target’s Armor by 10 for 2s, stacking up to 2 times.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Jeet Kune Do`**. Performs a three-part combo, each hit dealing 100–165 + 45% AD physical damage. The first two casts refresh the skill. The third knocks enemies airborne and starts its cooldown.
- **Skill 2 — `Shunpo`**. Dashes in the chosen direction with crowd-control immunity. Gains a 200–300 + 100% bonus Attack shield and 15 Physical Penetration for 2.5s on arrival.
- **Ultimate — `The Way of Dragon`**. Kicks an enemy hero away, dealing 275–420 + 110% bonus Attack physical damage. Recast within 4s to chase them and deliver an aerial flurry for 330–500 + 132% bonus Attack physical damage.

---

### 14. Eudora (`mlbb_eudora`)

- **Passive — `Superconductor`**. Skill hits mark enemy heroes and jungle monsters for 5s. Skills gain additional effects against enemies already marked. Basic attacks do not apply the mark. Marks do not stack.
- **Basic attack**. Basic attacks launch a lightning bolt that deals 100% AD physical damage.
- **Skill 1 — `Forked Lightning`**. Strikes non-champions in a fan for 100 + 55% AP magic damage, doubled against lane minions. Enemies already marked are linked for up to 1s, taking 4 pulses of 4 + 2% AP and a final hit of 100 + 55% AP if the link holds. Eudora gains 20% Movement Speed while linked. A final hit reduces this skill’s cooldown by 1.5s, once per cast. The link breaks beyond 95,000 range. Cooldown: 7s. Enemy champions take 120 + 65% AP magic damage from the first hit.
- **Skill 2 — `Ball Lightning`**. Launches an orb that reduces the target’s Magic Resistance by 8 for 1.8s, deals 115 + 45% AP magic damage and stuns for 0.8s. Against an enemy already marked, the same effects also hit nearby enemies. Each enemy is hit once. Cooldown: 11s.
- **Ultimate — `Thunder’s Wrath`**. Strikes a ground area for 260 + 110% AP magic damage at the center or 130 + 55% AP in the outer ring. Enemies already marked leave warnings at their hit locations. After 0.6s, those areas explode for 160 + 72% AP magic damage. Overlapping explosions from this cast hit each enemy once. Cooldown: 32s.

---

### 15. Franco (`mlbb_franco`)

- **Passive — `Wasteland Force`**. After 5s without taking damage, gains 10% Movement Speed, restores 1% maximum HP per second and builds up to 10 stacks. His next skill consumes them for up to 150% bonus skill damage.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Iron Hook`**. Throws a hook at the first enemy hit, dealing 190 + 100% AD physical damage and pulling them toward Franco. Stuns before pulling. Enemies that cannot be pulled can still be stunned.
- **Skill 2 — `Fury Shock`**. Deals 160 + 4% maximum HP physical damage to nearby enemies and slows by 70% for 1.5s. Can target jungle monsters and objectives.
- **Ultimate — `Bloody Hunt`**. Suppresses an enemy hero for 1.8s and strikes 6 times, each dealing 50 + 70% AD physical damage. Ends early if Franco is crowd controlled.

---

### 16. Gord (`mlbb_gord`)

- **Passive — `Mystic Favor`**. After 3 skill hits on an enemy, the next skill hit deals 12 + 2 × level + 8% AP bonus true damage and slows by 20% for 1s. The slow stacks twice, up to 40%.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Mystic Projectile`**. Fires an energy orb that bursts on the first enemy hit, dealing 105 + 52% AP magic damage and stunning for 1s.
- **Skill 2 — `Mystic Injunction`**. Creates an energy field for 4s, dealing 6 + 12% AP magic damage every 0.25s, up to 16 hits.
- **Ultimate — `Mystic Gush`**. Channels an energy beam for 3.2s, dealing 12 + 17% AP magic damage every 0.2s, up to 16 hits. Each wave aims toward the nearest enemy hero ahead of Gord.

---

### 17. Granger (`mlbb_granger`)

- **Passive — `Capriccio`**. Uses separate Gun and Cannon Energy pools, each starting at 800. Gun Energy regenerates at 8% of its maximum per second; Cannon Energy regenerates at 2%. Credited lane-minion kills increase maximum Gun Energy by 6 and restore 5%. Jungle kills grant the same benefit to Cannon Energy. Champion kills and assists increase both maxima by 12 and restore 10%. Capacity growth persists through death. Equipment cooldown reduction grants 0.5 Attack per percentage point, up to 20 Attack.
- **Basic attack**. Fires a handgun round that deals 100% AD physical damage on impact.
- **Skill 1 — `Rhapsody`**. Locks onto an enemy and fires 6 homing rounds over 1.5s, each dealing 36 + 45% AD physical damage and consuming 70 Gun Energy. Requires more than 420 Gun Energy. Movement Speed increases by 10% while firing. Each hit reduces Rondo’s cooldown by 0.5s. The final round gains 100% Critical Chance for its impact. Rondo and Death Sonata can be used during the barrage.
- **Skill 2 — `Rondo`**. Dashes a short distance and restores 15% of maximum Gun Energy. Walls and movement-disabling effects stop the dash. Equipment cooldown reduction increases Attack instead of shortening Rondo’s effective cooldown.
- **Ultimate — `Death Sonata`**. Consumes 250 Cannon Energy to fire a homing cannon round at an enemy champion. Impact triggers an explosion, dealing 155 + 110% AD physical damage to nearby enemies and slowing them by 30% for 0.6s. Granger then rolls a short distance away from the firing direction.

---

### 18. Hanabi (`mlbb_hanabi`)

- **Passive — `Ninjutsu: Petal Barrage`**. Basic attacks and skill hits release Petal Blades that bounce between nearby enemies. While shielded, Hanabi is immune to crowd control. Gaining a shield does not remove control already affecting her.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Ninjutsu: Equinox`**. Gains a 120 + 35% AD shield for 5s, plus 12% Movement Speed and 20% AD Speed. While this shield lasts, 20% of damage dealt to heroes and 10% against other enemies becomes additional shielding. Damage stops adding shields once her total shielding reaches 50% maximum HP.
- **Skill 2 — `Ninjutsu: Soul Scroll`**. Throws a piercing scroll, dealing 65 + 75% AD physical damage and slowing by 30% for 1s. Marks enemies for 5s. Her next basic-attack Petal bounce on a marked enemy deals an extra 20 + 50% AD damage without bounce reduction, consuming the mark.
- **Ultimate — `Forbidden Jutsu: Higanbana`**. Throws Higanbana at the first enemy hero hit, dealing 150 + 80% AD physical damage and rooting for 0.8s. It then blooms, dealing 105 + 55% AD physical damage nearby. Other enemies hit are slowed by 20% for 0.6s.

---

### 19. Hanzo (`mlbb_hanzo`)

- **Passive — `Ame no Habakiri`**. Basic attacks and mist hits apply up to 5 marks for 6s. New hits refresh them. Hanekage starts with 100 Energy and spends 10 per second. Hits and nearby deaths restore Energy.
- **Basic attack**. Strikes for 100% Attack physical damage and applies a mark.
- **Skill 1 — `Demon Feast`**. At 5 marks, bites champions for 200 + 7 per level after Lv.1 + 115% bonus Attack physical damage, heals and refunds 60% cooldown. Ordinary camps are devoured, healing 20% maximum Health.
  - Soul Snatch: Lunges for 135 + 6 per level after Lv.1 + 95% bonus Attack physical damage, increased 24% per mark against champions and 20% per mark against other targets. At 5 marks, adds damage based on maximum Health and heals.
  - Cooldown: 12s / 10s.
- **Skill 2 — `Dark Mist`**. Dashes and gains 15% Movement Speed for 2s. Leaves 5 mist pulses.
  - Black Mist: Dashes through walls for 110 + 4 per level + 55% bonus Attack physical damage and 0.4s airborne. Leaves 4 pulses.
  - Mist deals physical damage every 0.5s, applies marks and slows 35%. It cannot critically strike. Cooldown: 10s per form.
- **Ultimate — `Pinnacle Ninja`**. Projects Hanekage for up to 15s, leaving an invulnerable body. Gains 35% Movement Speed, 25% Attack Speed and wall traversal, with 30% less maximum Health.
  - Recast returns and heals 90 + 50% bonus Attack. Empty Energy draws the body to Hanekage. Defeat forces a return to the body, slows Hanzo 50% for 6s and sets a 52s cooldown (normally 26s).

---

### 20. Johnson (`mlbb_johnson`)

- **Passive — `Electro Airbag`**. Dropping below 30% HP grants a 120 + 240% Armor shield for 10s. Cooldown: 100s.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Impact Wrench`**. Throws his wrench, dealing 80 + 110% Armor magic damage along its path and stunning enemies near the landing point for 0.8s.
- **Skill 2 — `Electromagnetic Waves`**. Raises his shield, dealing 40 + 45% Armor magic damage in front of him and reducing enemy Movement Speed and Magic Resist, stacking up to 3 times.
- **Ultimate — `Full Throttle`**. Transforms into a car and picks up a nearby allied hero. Crashing into an enemy hero deals 90 + 150–220% Armor magic damage, stuns for 0.5–1s based on driving time and leaves an electrified zone. Can also drive his passenger to safety.

---

### 21. Julian (`mlbb_julian`)

- **Passive — `Smith’s Legacy`**. After casting two different skills, Julian enhances the remaining skill. The enhanced cast puts all three skills on a shared 9–6s cooldown that decreases with level. A skill can be used once per sequence. An unfinished sequence expires after 4s. Hitting an enemy hero with a skill grants 10% hybrid lifesteal for 5s, up to 30%. Repeated hits from one cast grant only one stack. Skills and basic attacks also heal Julian for 12% of the damage dealt, increased by 6% per stack, up to 30%.
- **Basic attack**. Basic attacks deal 100% AD physical damage. For 2s after casting a skill, basic attacks instead deal 100% AD + 30 + 55% AP magic damage and pull the target toward Julian. Every hit also heals Julian for a share of the damage dealt.
- **Skill 1 — `Scythe`**. Hurls a scythe in the target direction, dealing 155 + 72% AP magic damage to enemies along the way and slowing them by 30% for 1s. It pierces lane minions and stops at the first other enemy. Enhanced: deals 215 + 100% AP and slows by 50%. At the first non-minion enemy or maximum range the scythes keep flying at reduced speed, dealing 26 + 11% AP magic damage every 0.1s in an area for 1.6s, hitting each enemy repeatedly.
- **Skill 2 — `Sword`**. Summons a flying sword and dashes in the target direction, dealing 130 + 52% AP magic damage to enemies along the path. Enhanced: summons a storm of swords, dealing 30 + 14% AP magic damage every 0.1s along the dash, up to 6 times. Julian is untargetable and invisible during the dash, then reappears at the farthest end.
- **Ultimate — `Chain`**. Casts chains at the target location. After 0.3s they deal 112 + 50% AP magic damage in an 18,000-unit radius and immobilize enemies for 1s. Enhanced: deals 12 + 6% AP every 0.2s and slows by 30%, then enemies still inside take 205 + 88% AP and are knocked airborne for 0.8s. Before Chain is unlocked, Julian casts it automatically during a combo.

---

### 22. Karrie (`mlbb_karrie`)

- **Passive — `Lightwheel Mark`**. Basic attacks and Skills 1 and 2 apply a Lightwheel Mark. At 5 marks, they burst for 4% of the target’s maximum HP + 10% AD as true damage. Damage is capped at 250 against heroes and 300 against jungle and epic monsters.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Spinning Lightwheel`**. Throws a lightwheel that deals 60 + 80% AD physical damage to its target and 20 + 35% AD to nearby enemies except towers, slowing by 35% for 1s. Leaves a slowing zone for 1.5s, with two pulses of 10 + 10% AD physical damage, 0.5s apart. The pulses do not apply Lightwheel Marks.
- **Skill 2 — `Phantom Step`**. Dashes toward the chosen position and fires at an enemy near the landing point, dealing 75 + 70% AD physical damage and applying a Lightwheel Mark. The Attack ratio increases to 100% during Speedy Lightwheel.
- **Ultimate — `Speedy Lightwheel`**. Gains 20% Movement Speed and 35% AD Speed for 6s. During this time, Phantom Step deals 75 + 100% AD physical damage.

---

### 23. Obsidia (`mlbb_obsidia`)

- **Passive — `Return to Bone`**. Gains Bone Energy when dealing damage: 2 per basic attack on a hero, 4 per skill hit, or 1 per hit on a minion or turret. Jungle monsters grant none. Every 30 Energy creates a Bone Shard, up to 25. Each adds 7% AD to her basic attacks. Hero kills grant a shard immediately.
- **Basic attack**. Basic attacks deal 40 + 65% AD physical damage.
- **Skill 1 — `Abyssal Bone Needle`**. Fires a bone needle in a line, dealing 90–140 + 60% AD physical damage. The first enemy hero hit grants Bone Energy.
- **Skill 2 — `Phantom Shadowmeld`**. Dashes in the chosen direction and gains 20% Movement Speed for 0.3s.
- **Ultimate — `Hunt of Bone`**. Fires a bone shot for 55 + 50% AD physical damage, briefly stunning the first enemy hero hit and pulling Obsidia toward them. Tethers her to the target for 3s: they cannot dash, and their movement pulls her along. Grants a shield, 30% AD Speed and 5 temporary Bone Shards beyond the cap. She can cross obstacles and attack while moving; Phantom Shadowmeld lasts 1s.

---

### 24. Popol & Kupa (`mlbb_popol_kupa`)

- **Passive — `We Are Friends`**. When Popol and Kupa damage the same enemy within 2s, both gain 5% Attack Speed and 2% Movement Speed for 4s, stacking up to 3 times. Each paired exchange grants one stack.
  - Kupa has 560 + 80 per level after Lv.1 + 50% of Popol’s bonus Max HP and inherits Popol’s current Armor, Magic Resist and Attack Speed. After 5s out of combat, Kupa recovers 8% Max HP per second. If Kupa falls, Popol begins a stationary 3s prayer after 3s without combat. Movement, damage or another action interrupts it. Successful revival has a 30s cooldown.
  - Popol’s Surprise: During combat, Popol automatically places a trap every 10s, up to 2. Traps arm after 0.5s and last 15s. An enemy triggers 25 + 20% AD physical damage and 0.6s of immobilization, leaving frost for 3s that slows enemies by 20% and reduces Armor by 12%. Frost from the same Popol does not stack. Placement pauses during casting, recall, riding and prayer.
- **Basic attack**. Popol throws a spear that deals 100% AD physical damage on impact. Kupa bites nearby enemies for 45% AD physical damage, increased to 75% AD in Alpha Wolf Form. Kupa’s ordinary bites use Popol’s basic-attack item effects.
- **Skill 1 — `Bite ’em, Kupa!`**. Popol throws a spear for 50 + 50% AD physical damage and commands Kupa to bite the target for 60 + 60% AD physical damage, slowing it by 50% for 1s. Alpha Wolf Form: the bite instead deals 90 + 75% AD physical damage to nearby enemies and knocks them airborne for 0.6s.
- **Skill 2 — `Kupa, Help!`**. Popol gains a 90 + 50% AD shield for 3s and calls Kupa back. When Kupa arrives, Popol mounts him for up to 1.5s, gaining 50% Movement Speed while Kupa recovers 25% Max HP over the ride. Attacks, other skills and hard crowd control end the ride. Alpha Wolf Form: the shield increases to 140 + 70% AD and Kupa fully recovers HP at mount start.
- **Ultimate — `We Are Angry!`**. Kupa fully recovers HP and enters Alpha Wolf Form for 9s, gaining 35% Max HP and enhancing Bite ’em, Kupa! and Kupa, Help! Cooperative stack bonuses are doubled. Requires a living Kupa.

---

### 25. Uranus (`mlbb_uranus`)

- **Passive — `Radiance`**. Taking damage grants a Radiance stack for 10s, up to 12. Gains one stack per second. At 12 stacks, restores 28–134 HP every 0.8s, based on level.
- **Basic attack**. Basic attacks deal 100% AD physical damage. Does not apply Ionic Marks.
- **Skill 1 — `Ionic Edge`**. Sends two blades outward and back. Each hits a target once for 79–131 + 18% AP magic damage and slows by 30% for 2s. Each cast applies one Ionic Mark per target for 6s. Marks increase later Ionic Edge damage by 20% each, up to 8 marks. Buildings cannot be marked.
- **Skill 2 — `Transcendent Ward`**. Dashes to the chosen position, dealing 48–120 + 12% AP magic damage along the path and 96–240 + 48% AP near the endpoint, slowing nearby enemies by 25%. Gains a 300–750 + 200% AP shield for 4s. When it breaks or expires, it bursts for 156–390 + 78% AP magic damage.
- **Ultimate — `Consecration`**. Removes all crowd control, restores 200–400 HP and grants 60% Movement Speed, fading over 8s. Adds 5–12 Radiance stacks. For 8s, increases HP regeneration and his own Transcendent Ward shield by 20%.

---

### 26. Zhask (`mlbb_zhask`)

- **Passive — `Decimation`**. On death, summons a frenzied Spawn at his last position. It lasts up to 6s and loses 2% of its maximum HP each second.
- **Basic attack**. Basic attacks deal 100% AD physical damage.
- **Skill 1 — `Nightmaric Spawn`**. Summons a stationary Spawn for up to 12s, inheriting 55–100% of his HP, defenses and Magic Power, based on level. Its Death Ray deals 30 + 25% AP magic damage every 0.25s and slows by 30% for 0.6s. Aim again to reposition it. The ray pauses if Zhask moves too far away.
- **Skill 2 — `Mind Eater`**. Fires a piercing projectile, dealing 87 + 6 × levels above 1 + 60% AP magic damage and stunning for 0.5s. His Spawn follows up on each enemy hit for 35 + 50% AP bonus magic damage.
- **Ultimate — `Dominator's Descent`**. Teleports to the chosen position and merges with an empowered Spawn for up to 10s. Zhask becomes untargetable and cannot act separately while merged. The Spawn inherits all his stats, and its Death Ray deals 1.8 times its normal damage.

---

### 27. Yu Zhong — Black Dragon (`mlbb_yu_zhong`)

- **Cursing Touch.** Three marks trigger five damaging pulses and restore health. Marks also work on jungle monsters, including Morgard and Serpen.
- **Dragon Tail.** Sweeps nearby enemies. The outer edge hits harder and applies two marks.
- **Soul Grip.** Slows enemies and empowers the next basic attack, which can lunge at the champion hit. Furious Dive deals damage on landing, allows a short second dash, and knocks champions airborne at the landing point.
- **Black Dragon Form.** Flies through terrain and knocks enemies back on contact. Yu Zhong lands in Dragonoid Form with wider, longer-reaching skills for 15 seconds.

---

### 28. Lesley — Deadly Sniper (`mlbb_lesley`)

- **Lethal Shot.** Each skill, or five seconds without taking damage, readies an attack with 17 extra range for 75 + 110% Attack true damage to champions. Lane and jungle enemies take 75 + 125% Attack physical damage.
- **Master of Camouflage.** Conceals Lesley and grants movement speed for up to 2.5s.
- **Tactical Grenade.** Blasts a cone and knocks enemy champions away as Lesley steps back.
- **Ultimate Snipe.** Fires four shots for 70 + 32% Attack + 6% missing HP each. Other enemy champions can intercept.

---

### 29. Valir — Son of Flames (`mlbb_valir`)

- **Ashing.** Direct spells apply a burn. At three stacks, it detonates for **4% max HP + 25% Magic Power** and briefly stuns eligible targets.
- **Burst Fireball.** Stores two charges. A shot deals **70 + 65% Magic Power** on impact and leaves a delayed explosion for **42 + 30% Magic Power**. A direct hit on a champion or boss can refund a charge.
- **Searing Torrent.** Deals **95 + 65% Magic Power** and pushes back enemy champions, lane minions and regular jungle monsters. The empowered cast leaves a damaging firewall. Cooldown is **10s**.
- **Vengeance Flame.** Can break a stun when ready. It grants a short movement speed boost and empowers up to four Fireball or Torrent casts within **9s**. Cooldown is **42s**.

---

### X.Borg — Firaga Armor (`mlbb_xborg`)

- **Firaga Guard.** Armor reduces incoming damage by 20% and uses a separate reserve. A bar above X.Borg shows the reserve or recharge. Attacks and skills build heat on champions and monsters. Skill hits deal true damage to targets already at four stacks.
- **Fire Missiles.** Fires up to six pulses in a moving cone. The armorless cone is narrower, reaches farther, and deals 60% damage. Hits on champions and monsters restore body HP and Firaga reserve or recharge.
- **Fire Stake.** Launches five stakes that return after 1.2 seconds. The returning stakes pull enemies and collect supplies.
- **Last Insanity.** Charges and spins for up to 3 seconds. Detonation deals 100 + 6% target max HP as true damage, with a 50% bonus against overheated targets and a cap of 150 against epic monsters. An armor break triggers a weaker blast.
- X.Borg has official skill icons, default English voice clips, and a nine-frame selection idle.

---

### Minotaur — Son of Minos (`mlbb_minotaur`)

- **Rage Incarnate.** Skill casts grant three seconds of Motivation. Minotaur recovers a portion of HP lost to enemy champions, doubled while Enraged.
- **Despair Stomp.** Leaps to a landing point, knocking enemies airborne and slowing them. The next two eligible basic hits deal bonus physical damage. Enraged widens the landing area. Structure attacks preserve the bonus charges.
- **Motivation Roar.** Heals Minotaur and the nearby ally with the lowest HP percentage for 220 + 60% Magic Power. Minotaur also recovers 8% of his missing HP. While Enraged, the heal reaches every nearby allied champion.
- **Minoan Fury.** Slams three times. The first two hits each deal 65 + 40% Attack physical damage and slow. The final blow deals 100 + 55% Attack true damage and knocks enemies airborne. Enraged lasts 12 seconds.

## Installation

1. Move any existing `mlbb_heroes` folder out of `mods/`, then download the Windows ZIP (`mlbb_heroes_v3.2.0.zip`) from the release assets and extract the `mlbb_heroes` folder into the **Teamfight Manager 2** `mods/` directory.
   ```text
   Teamfight Manager 2/mods/mlbb_heroes/
   ```
2. *(Important)* If you previously installed standalone Aldous, Fanny, Fanny Wall, Lancelot, Estes, or Gusion mods, remove or disable them so their champion IDs do not conflict with `mlbb_heroes`.
3. Launch **Teamfight Manager 2** and enable **MLBB Heroes** in the in-game **Mod Manager**. Check for build **1.1.204**.
4. See `Install_Guide.html` for the screenshot guide. The optional MLBB Champ Info companion adds basic attack and passive cards.

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
│   └── …                            # 32 hero definitions
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

## v3.2.0

- Minotaur joins as hero 32 with a landing shockwave, team healing, and a three-hit ultimate.
- The Windows ZIP is about 38 MB, down from about 84 MB in v3.1.0. The voice bank keeps one recording per retained cue.

## v3.1.0

- X.Borg joins as hero 31 with both forms, the Firaga gauge, an animated selection idle, official skill icons, and default English voices.
- Fanny's cable routing can plan up to three anchors ahead and use movement waypoints.
- Aldous has new combat animations. Balmond has updated sprites and Cyclone Sweep visuals.

## v3.0.1

- Sun joins as hero 30, with two doppelgangers, official skill icons, and default English voice lines.
- **Fanny revamp.** Updated sprites and effects, 10 Steel Cable charges, midair redirects, and retreat cables.
- Ultimate Snipe shots are easier to see. Lesley and Valir have their default English voice lines.

## v3.0.0

- Increased starting HP for 24 heroes and slowed basic attacks for five tanks, two supports, and four mages.
- Reduced excess damage on Alpha, Balmond, Franco, and Lesley. Increased Johnson’s Impact Wrench damage.
- Adjusted Angela’s basic reach and improved Eudora’s Forked Lightning targeting against minions.
- Updated Lesley’s skill descriptions and original Mobile Legends skill icons. The balance notes and checklist are in `docs/V3.0.0_BALANCE.md`.

## v2.10.0

- Valir joins as hero 29 with charged fireballs, a knockback wave, and an ultimate that breaks supported crowd control and empowers four casts.

## v2.9.0

- Lesley joins as hero 28 with skill icons, custom sound effects, and a four-round ultimate.
- Yu Zhong gains the empowered basic lunge and Furious Dive follow-through. Chou’s combo transitions are faster.

## v2.8.0

- Yu Zhong joins the roster as the 27th hero, with human, Black Dragon, and Dragonoid forms.
- Includes Yu Zhong's animated draft preview, ability icons, and cast sound effects.

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

`mlbb_heroes.dll` is included as a precompiled Windows x64 library. Rebuild from the native crate after editing the source:

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
- **Disclaimer**: Non-commercial fan-made mod for *Teamfight Manager 2*. *Mobile Legends: Bang Bang* characters, names, icons, and voice recordings belong to Moonton.
