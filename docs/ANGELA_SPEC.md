# Angela — Phase 0 spec (hero 20, the Bunnylove)

**Status updated 12 November 2026. Phase 0 was approved previously. Owner reports character motions complete. VFX prompts reviewed and corrected; visual production is pending. Angela implementation remains pending.**

Written 2026-11-11 against the canon kit read verbatim from the MLBB wiki (Phase 0, step 1),
the shipped v2.0.10 roster for balance bands, and `mod-api-stable` for feasibility.
Anything I could **not** confirm is marked **UNVERIFIED** rather than asserted.

---

## 1. Canon kit (verbatim, mobile-legends.fandom.com/wiki/Angela)

> **Angela, the Bunnylove** — Support, Guard/Support specialty, Roaming, Mana-based, Magic damage.

| slot | name | canon text |
|---|---|---|
| **Passive** | **Smart Heart** (Speed Up) | Angela gains **7.5% extra Movement Speed for 4 s each time she casts a skill (up to 30%)**. When she's attached to an allied hero, the Movement Speed boost is transferred to the allied hero. |
| **Skill 1** | **Love Waves** (Heal, AOE · Spell Vamp 50%) | Sends Love Waves in the target direction, dealing **170–320 (+80% Total Magic Power)** magic damage to enemies hit, revealing their positions briefly, and applying a stack of **Lover's Mark** (up to **5**). Each stack increases the damage by **20%** and **slows** by **8%** for **3 s**. The Waves also **restore 150–200 (+75% TMP) HP to allied heroes hit**. Angela stores up to **5 charges**; recharge time affected by CDR. |
| **Skill 2** | **Puppet-on-a-String** (Slow, CC) | Launches a Puppet String at the target enemy, dealing **300–450 (+40% TMP)** magic damage and **gradually slowing them up to 80%**. If the enemy is **still connected after 3 s**, they are **immobilized for 1.5 s** and take **450–675 (+60% TMP)**. Each Lover's Mark stack increases the final damage by 20%. |
| **Ultimate** | **Heartguard** (Attach, Shield) | Channels **2 s**, applying a **1000–2000 (+200–300% TMP) shield** lasting **6 s** to the target allied hero **regardless of distance**. When the channel completes she **attaches herself to the ally for 12 s**, during which she can cast skills at **zero Mana cost**. Recas to cancel/detach; the state ends early if the ally dies. |

Canon stats (L1 → L15): HP 2300→4206 · Phys Atk 115→216 · Phys Def 15→69 · Magic Def 15→50 ·
Move Spd **240** · Attack Range **4.9** (ranged).

---

## 2. Feasibility — data vs native

| ability | route | why |
|---|---|---|
| **Passive Smart Heart** | **native** (`mlbb_heroes:angela_smart_heart`) | Needs a call-site on *every* skill cast and a buff that migrates to a host while attached. Purely stateful; no data expression. |
| **S1 Love Waves** | **data** for the cone + damage + heal; **native** for the mark counter | `RangeEffect`/`LineRangeProjectile` + `Heal` (heal_type `Any`, so the same wave heals allies and damages enemies) + `AddBuff` for the slow. **5 charges = `cooltime_use_count: 5`** — confirmed real, the field exists and Chou's S1 already uses `3`. |
| **S1 Lover's Mark stacks** | **native** — and it must use `entity_stack_buff` | The 20%-per-stack damage and 8%-per-stack slow are a counter on the *victim*, capped at 5. **This is exactly the Johnson S2 bug**: `add_buff` does not stack. Use `entity_stack_buff(target, &mark, 5, ?)`. |
| **S2 Puppet String** | **data** for the damage + progressive slow; **native timer** for the 3 s detonation | `AddBuff` carries the ramp; a native tick checks "still connected at 180 ticks" → `apply_cc` immobilize + second damage chunk. Immobilize is a real CC type in the vocabulary (stun/immobilize). |
| **Ult Heartguard** | **native**; banish-launch **UNVERIFIED** | Shield is trivially data (`Shield`, ally-targeted, 6 s). The attach is the problem — see §3. |
| **Mana cost** | **DROPPED** | This roster has no mana economy on any hero. Said out loud per Phase 0 step 3. |
| **CDR interaction** | **DROPPED** | No CDR in the roster; charge recharge is fixed. |

### What is impossible, said out loud

- **None of the 19 shipped heroes has mana.** Angela's "zero Mana cost while attached" is
  therefore meaningless — it is dropped, not approximated.
- **"Reveals their positions briefly"** is dropped: there is no fog of war for mods
  (HERO_PIPELINE stop list).
- **The 2 s channel** likely cannot be a real channel — a `CasterAnimation` roots the caster,
  and we learned from Cecilion's ult that a rooted caster is unusable. Proposal: treat canon's
  2 s channel as **instant** and keep only the 6 s shield + 12 s attach. Flagged as a
  deviation, not a silent change.

---

## 3. 🔴 The ult — the one real design decision

Canon: Angela **attaches to an ally** for 12 s. She is not a separate unit on the field; she
rides.

**What the engine gives us (checked in `mod-api-stable/src/sim.rs`, not assumed):**

| primitive | present? | what it does |
|---|---|---|
| `entity_banish` | ✅ | untargetable + invisible + input-locked. **Proven both ways on the caster** (Johnson self-banishes every tick and the owner has now played it). |
| `entity_set_pos` | ✅ | ignored on banished units (§29.27b) |
| `entity_set_invisible` | ✅ | **enemy vision only** — not a real hide |
| `entity_remove` / despawn | ❌ **absent from the stable ABI** | — |
| `entity_set_hp(0)` | ✅ | the Zhask/Johnson way to remove an entity |

**Therefore the mechanism is:** Angela self-banishes (invisible, untargetable, input-locked)
for 12 s and her **position is slaved to the host's** every tick. That is the Johnson
passenger pattern with Angela as the passenger.

**Two sub-decisions this forces, both needing your call:**

1. **The camera.** With Johnson the *player* drives the car, so banishing Johnson is fine.
   With Angela, *Angela is the player*. If she banishes herself for 12 s, what does the
   player look at? **UNVERIFIED** — I have not confirmed the host camera follows a banished
   player unit. This is the single biggest risk in the whole hero and it must be answered by
   one build, not argued.
2. **Can she still cast?** Banish is documented as *input-locked*. Canon Angela casts from
   the host's position the whole time. If input is locked she cannot — which guts the hero.
   Mitigation to test: cast the ult's payload **before** the banish lands, then again from
   the host's position on each skill. If input-lock is enforced by the engine's AI path
   rather than the sim, a native `JohnsonDriveAi`-style **pass-through player AI** may bypass
   it — Johnson already proves a pass-through AI exists and works.

**Fallback if both fail (§48-style, shelved-not-forced):** Angela becomes a strong
long-range shield-and-heal support with S1 charges and the Puppet String, and the attach is
dropped and documented. **She is still a complete hero without it** — which is the whole
reason this is worth attempting rather than blocking on.

---

## 4. Stat block + balance proposal

Roster HP band is **650–880** and canon pools are ~2300+, so everything scales down hard.

| | canon L1 | proposal | reference |
|---|---|---|---|
| category | Support | **Util** | Estes is `Util` — the game's only support slot |
| tags | Support, Roaming, Magic | `AP, Heal, Shield, Range, CC` | matches Estes `AP,Heal,Shield,Range` + CC from S2 |
| HP | 2300 | **700** | Estes 690 · band 650–880 |
| attack | 115 | **46** | Estes 48 · mages 45–48 |
| defence | 15 | **22** | Estes 26 |
| magic resistance | 15 | **28** | Estes 30 |
| move speed | 240 | **1040** | the roster's slowest bracket; she is ranged and must be catchable |
| magic_power | 0 | **58** | Estes 55 · Gord 64 |
| attack range | 4.9 | **88000** | Estes 92000 on S1; ranged but shorter than a mage |

### Damage / heal (canon re-priced, then fitted to the band)

| | canon | proposal | working |
|---|---|---|---|
| **S1 hit** | 170–320 (+80% TMP) | **80 + 30% TMP** per wave | ÷~3; a 5-charge nuke must stay small per hit |
| **S1 per mark** | +20% dmg, +8% slow | **+12% dmg, +6% MS** | floored so 5 stacks = +60% / −30% MS, not +100% |
| **S1 heal** | 150–200 (+75% TMP) | **46 + 40% TMP** | just under Estes' heal, because S1 has 5 charges |
| **S2 first hit** | 300–450 (+40% TMP) | **95 + 25% TMP** | ÷~3.2 |
| **S2 detonate** | 450–675 (+60% TMP) | **140 + 35% TMP** | the "you stayed in it" punishment |
| **S2 slow ramp** | up to 80% | **up to 45%, over 180 ticks** | 80% is a root on this map scale |
| **S2 immobilize** | 1.5 s | **0.7 s (42 ticks)** | ÷~2 — 1.5 s at 60 tps is a death sentence here |
| **Ult shield** | 1000–2000 (+200–300% TMP) | **220 + 180% TMP** | compare Johnson's 221 airbag at TPD 42 |

Sanity check against the roster: at level 1 with 58 magic power, S1 heals **69** and hits for
**97**; S2 detonate is **160**; the ult shield is **324** on a 700 HP ally. Nothing
one-shots, and the support cannot solo-kill — which is the point.

---

## 5. Open questions for you (answer before Phase 1)

1. **Approve the kit and the numbers above?** Phase 0's gate is exactly this.
2. **The attach is the risk.** Do you want me to *attempt* it (needs a build + your playtest
   to settle the camera question), or ship Angela as a pure support first and add the attach
   in a later release? My recommendation: **attempt it in the same rc, with the drop-in
   fallback already written**, so a failed playtest costs one build, not a re-scope.
3. **Fix Johnson's S2 stacking in the same release?** I'd say yes — Angela's Lover's Mark
   needs the *identical* `entity_stack_buff` mechanism, so writing her mark correctly
   essentially produces Johnson's fix for free. Both should land together.
4. **Facing/art is untouched until you approve.** No pixels yet.

---

## 6. Process notes

- Hero 20 · `mlbb_angela` · namespace `mlbb_heroes:angela_*`.
- New source file `src/angela.rs`; register in `lib.rs`.
- i18n: `description.mlbb_angela.*` + `skill_name.mlbb_angela.*` in **8 locales**, merged via
  `mod.override_info` type **`merge`, never `override`**.
- Charges: `cooltime_use_count: 5` on `skill`, `cooltime` 300 (= 6 s total recharge, 1.2 s
  per charge) — mirrors Chou's `540 / 3`.
- Every claimed validation must show real command output; nothing is "in-game verified"
  without your playtest.

---

## 7. Research pass — what the kit text alone did not tell us

Checked 2026-11-11 against the wiki **Trivia**, the **Audio** page (all 13 default lines) and
two independent guides. Three findings change the design.

### 7a. Her basic attack is a heart projectile, not a swing

> *"She used to toss the puppet as her basic attack when she was still in development, but the
> animation was eventually altered to simple **heart-shaped projectiles**."* — wiki, Trivia

She is a **ranged caster with a 4.9 attack range**. So `attack` is a `LineRangeProjectile` /
`view_projectiles` heart, and she never makes contact. This also means her `attack` row must
**not** be a melee punch — an easy mistake given the 9-frame budget.

### 7b. She cannot fly — and says so

> *"Doctor calls me his little angel! But — [giggles], **I can't fly**…"* — Angela.move06

The pixelly look invites a floating idle. **She walks.** Her `run` row is a genuine two-step
walk cycle and her `idle` keeps feet planted. Only `ult` lifts her, and only 11 rows.

### 7c. Confirmed: she is the first attach hero, so there is no canon tether visual

> *"She is the first hero who can be attached to a hero by using her Ultimate, the second is
> Gloo."* — wiki, Trivia

In MLBB she simply **rides** the ally — no line is drawn. So the attach needs no tether art
in canon, which is fortunate, because the engine has no primitive for it. `angela_attach_link`
is optional polish only, and is **gated** on the attach mechanism being proven first.

### 7d. Voice lines — this is the S1/S2/ult set

| trigger | canon line |
|---|---|
| select | "Love and hope are the two greatest inventions." |
| move 01–09 | "Abracadabra! Keep smiling!" · "Kiki kaka! The strings of my puppet got tangled up…" · "Stay cheerful, everyday!" · … · "It's my job to help people." |
| **skill 1** | "Pipa Pipa!" |
| skill 1 (alt) | "[giggles]" |
| **ultimate** | "Papipapi Poom!" |
| death 01–03 | "Doctor, calls me… angel." · "Where's… my daddy?" · "[grunts]" |

That is **13 default lines** — the same count Johnson shipped. Consistent with the roster
rule *"voice on S1/S2/ult only"*; she gets S1, ult, and the move/select/death set.

### 7e. Stat sanity check against the roster

Her canon **move speed is 240** — the second-slowest hero in MLBB. The proposal keeps her at
**1040**, the roster's slowest bracket, which is the correct analogue of "240, and she can't
fly" without making her unplayably slow on a 60-tick clock.

---

## 8. Deliverables produced so far (Phase 0–1 only)

| file | what |
|---|---|
| `angela_work/ANGELA_SPEC.md` | this document — kit, feasibility, balance |
| `angela_work/ANGELA_SPRITE_CHECK.md` | the master measured with PIL, palette, landmarks, the 32×45 size problem |
| `prompts/ANGELA_MOTION_PROMPTS.md` | the eight `PixelLab` motion prompts + rejection checklist |
| `prompts/ANGELA_VFX_MOTION_PROMPTS.md` | seven effect packages + binding rules |

**Nothing has been sent to a build. No pixels generated. No code written.**



## Review addendum — 12 November 2026
This addendum supersedes conflicting historical statements above.
- Character motions are complete by owner report. Actual sheets were not available for visual inspection.
- Use ANGELA_VFX_MOTION_PROMPTS.md for the corrected alpha-background, single-base-frame workflow.
- Heartguard includes a doll above the ally. The earlier inference that being attached needs no indicator was incomplete. Replace angela_attach_link with angela_attached_doll.
- S1 needs visible forward travel and explicit ally/enemy filtering. A caster-bound effect alone does not establish travel.
- The banish proposal contradicts its own table: entity_set_pos is recorded as ignored while banished. Host-origin casting and overlay placement require a verified implementation; do not assume position slaving works.
- Historical canon values vary by patch. Recent Liquipedia and METAMMO references list passive 5% per cast up to 20%, and a 20-second Heartguard attachment with revised shield values. The approved mod timings and balance remain the implementation proposal pending a separate balance decision.
- 300 ticks at 60 ticks/second is 5 seconds. The earlier claim of 6 seconds is arithmetically wrong. Charge recharge semantics must be checked against the SDK and working examples before assigning per-charge timing.
- The 46 attack proposal predates the roster-wide basic-attack rescale. Re-evaluate against the current 1.1.116 roster before implementation.
- Treat the exact VFX palette, crescent shape and root ornament as adaptations until clear default-skin visual references are available.
Sources: https://liquipedia.net/mobilelegends/Angela ; https://metammo.com/mobile-legends-guide/heroes/angela ; https://mobile-legends.fandom.com/wiki/Angela


## Implementation checkpoint — 13 November 2026, v2.1.0-rc.2 / 1.1.118
This checkpoint supersedes historical pending/fallback statements above. Owner requested repair of the incomplete rc.1 kit and supplied a new S1 sheet (Drive 1rn_ifbsUuJ9gwhbsJp5QkU7mPS5Fdm-G).
- S1 is a travelling native wave, using the supplied 3×3 RGBA grid, with one hit/heal per unit. Mark slow is 6% per stack, maximum five; bonus applies only to later S1 hits and S2 final damage.
- S2 is source-owned, draws a live Estes-style directional tether, ramps slow to 45%, and applies real Bind for 42 ticks plus 140 + 35% AP at tick 180. Break/death/purge cancels. Its runtime is called by the selected passive, with a composed match-hook fallback.
- Heartguard now applies the shield once, then banishes Angela for a 720-tick attachment. Native simulation callbacks automatically cast S1/S2 from the host position; normal banished input is not used. Doll overlay follows host. Expiry/recast/host death or banish ends it; wait 3 ticks before repositioning the released Angela. The original 2-second channel remains the previously proposed instant adaptation.
- Smart Heart restored to this approved spec: 7.5% per cast, four stacks (30%), 240 ticks; transfer to host on attach and back on detach. The rc.1 1.5% × five cap was inconsistent with this spec.
- Charges follow the SDK contract, not the old grouped-recharge claim: full 300-tick cycle / five uses = 60 ticks per use before CDR.
- Engine cooldown debt cannot be increased/consumed through the stable ABI. Shared native cooldown debt and an AI guard prevent free extra casts after detachment; engine-displayed counters can show ready early. This display discrepancy remains unresolved.
- Banish still prevents moving the original unit: neither SetPos nor Grab is used to move it during attachment. All attached targeting/visual origins use host coordinates. Position is restored only after banish expires.
- Shield exhaustion while another shield remains cannot be identified individually: ABI exposes total shield. Visual ends at zero total shield or six seconds.
- Unit/callback and asset checks are separate from in-game verification. Camera, engine callback delivery during banish and banished-caster damage attribution require the owner's playtest. Do not label the attachment proven in game or publish a bare v2.1.0 release on these checks alone.



## rc.3 playtest repair — 13 November 2026 / internal 1.1.119
Owner tested rc.2: S1 and S2 function, speed is excessive, and Angela never uses Heartguard. This is partial in-game evidence, not attachment verification.
The rc.2 repair misread BuffV1 percentage units. Smart Heart's raw 75 × four meant +300%, not +30%. Fixed with 7/8/7/8 whole-percent contributions; cumulative bonus is 7/15/22/30%. Mark slow is -6 per stack; S2 slow ramp is capped at 45, not 450. Other damage/timing/art are retained.
Heartguard's Native effect incorrectly reported shield as expected_damage, while expected_shield defaulted to zero. It now reports expected_shield = 220 + 180% AP. Targeting/AllyChampion is valid (Estes uses the same pair). A proactive AI rule selects a living targetable allied champion, excludes self, prefers low health, and requires enemies within 150,000 (or within 300,000 when the ally is at or below 65% HP). Engine level lock, charges, native cooldown debt and input validity are respected. Already protected hosts and quiet allies are skipped.
New tests exercise the actual AI callback with no base action, self-target redirection, engine/native gates, rejected inputs and percentage sums; they do not establish live Heartguard/attachment behavior. Retest speed and ult activation at level 5+, then validate attachment, host-origin skills and return position.
