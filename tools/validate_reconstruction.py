#!/usr/bin/env python3
"""Fast static integrity checks for the 1.1.63 reconstructed candidate."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def fail(message: str) -> None:
    raise SystemExit(f"FAIL: {message}")


try:
    info = json.loads((ROOT / "mod.mod_info").read_text(encoding="utf-8"))
    json.loads((ROOT / "mod.override_info").read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    fail(f"invalid mod metadata: {exc}")
if info.get("version") != "1.1.63":
    fail("mod.mod_info version is not 1.1.63")

champion_path = ROOT / "champion/mlbb_zhask.data_champion"
try:
    champion = json.loads(champion_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    fail(f"invalid Zhask champion data: {exc}")
if champion.get("id") != "mlbb_zhask":
    fail("unexpected champion id")

beam_names = [e.get("name") for e in champion.get("view_effects", [])]
expected = [f"mlbb_heroes_zhask_spawn_beam_{i}" for i in range(12)]
if beam_names[:12] != expected:
    fail("Zhask view_effects must begin with the ordered 12 Spawn beam bindings")
if "mlbb_heroes_zhask_mind_eater_impact" not in beam_names:
    fail("Skill 2 impact visual binding is missing")
if any("zhask_death_ray" in str(e) for e in champion.get("view_effects", [])):
    fail("obsolete directional Death Ray binding remains")
for suffix in ("#sheet.png", "#anim.fanim"):
    if not (ROOT / f"effects/zhask_spawn_beam{suffix}").exists():
        fail(f"missing Spawn beam asset suffix {suffix}")
beam_sheet = (ROOT / "effects/zhask_spawn_beam#sheet.png").read_bytes()
if beam_sheet[:8] != b"\x89PNG\r\n\x1a\n" or int.from_bytes(beam_sheet[16:20], "big") != 384 or int.from_bytes(beam_sheet[20:24], "big") != 32:
    fail("Spawn beam sheet must be a valid 384x32 PNG containing twelve 32px directional frames")
beam_anim = json.loads((ROOT / "effects/zhask_spawn_beam#anim.fanim").read_text(encoding="utf-8"))
for angle in range(12):
    tag = beam_anim.get("anims", {}).get(f"beam_{angle}", {}).get("frames", [])
    if len(tag) != 1 or tag[0].get("data") != {"x": angle * 32, "y": 0, "w": 32, "h": 32} or tag[0].get("duration", 0) < 0.079:
        fail(f"Spawn beam directional frame {angle} has an invalid rect or short visual lifetime")

source = (ROOT / "src/zhask.rs").read_text(encoding="utf-8")
if not re.search(r"entity_banish\(\s*caster\s*,\s*caster\s*,\s*FUSION_BANISH_STEP", source):
    fail("fusion is not using short-step true entity_banish")
if "entity_clear_cc(caster)" not in source:
    fail("fusion cleanup does not attempt entity_clear_cc")
s2_effect = champion.get("skill2", {}).get("effect", {})
if s2_effect.get("type") != "LineRangeProjectile":
    fail("Skill 2 line collision is not data-defined as LineRangeProjectile")
if (s2_effect.get("width"), s2_effect.get("length"), s2_effect.get("delay"), s2_effect.get("apply")) != (18000, 78000, 0, 1):
    fail("Skill 2 data line dimensions/timing changed unexpectedly")
applied = s2_effect.get("applied_effects", [])
if not any(a.get("effect", {}).get("type") == "Stun" and a.get("effect", {}).get("duration") == 30 for a in applied):
    fail("Skill 2 does not contain the declarative 30-tick Stun effect")
if not any(a.get("effect", {}).get("type") == "Native" and a.get("effect", {}).get("effect_ref") == "mlbb_heroes:zhask_skill2_hit" for a in applied):
    fail("Skill 2 data hit path is missing the native damage/follow-up sub-effect")
if "sim.apply_cc(target, &CcV1::stun(S2_STUN_TICKS))" in source:
    fail("Rust hit sub-effect must not own the primary Skill 2 Stun")
if "delayed Spawn bonus damage requested" not in source or "delayed Spawn follow-up stun requested" in source:
    fail("Spawn follow-up must be bonus damage only, not the Skill 2 stun owner")
if "FUSION_BANISH_REFRESH" not in source or "one_tick_exit_banish" not in source:
    fail("short-step Banish renewal/exit diagnostics are incomplete")
if "entity_set_invisible(caster" in source:
    fail("invisibility must not replace Banish on Zhask")
if "FUSED_SPAWN_NAME" not in source or "normal Spawn restored as distinct entity" not in source:
    fail("fusion does not use a distinct fused Spawn and normal-form restoration")
if "fn take_fusion_marker" not in source or "take_fusion_marker(st)" not in source:
    fail("fusion end marker is not consumed idempotently")
if re.search(r"st\.fused_until\s*=\s*tick\s*;", source):
    fail("finished fusion retains a sticky expired deadline")
if source.count('"duration_expired"') != 1:
    fail("fusion timeout must have exactly one cleanup call site")
if "inset_beam_endpoints" not in source or "can_render_spawn_visual" not in source or "&InputTargetV1::pos(x, y), 0, 0, 0" not in source:
    fail("Spawn beam is missing the Estes-style view-only, inset, refreshed renderer")
if "BEAM_SEGMENT_SPACING: f64 = 750.0" not in source or "BEAM_MAX_SEGMENTS: usize = 96" not in source:
    fail("basic attack beam does not enforce tight overlapping segment spacing")
if "view_owner" not in source or "sim.play_view_effect(&name, view_owner" not in source:
    fail("Spawn beam view effects are not attributed to Zhask, their registered owner")
if "basic_attack_beam_segments_use_tight_overlap_and_are_bounded" not in source:
    fail("basic attack beam density regression test is missing")
if "rem_euclid(std::f64::consts::PI)" not in source or "PI / 12.0" not in source or "beam_directions_match_twelve_15_degree_undirected_frames" not in source:
    fail("beam direction mapping does not match the 12 half-turn sprite frames")
if "S2_FOLLOWUP_CAP: usize = 6" not in source or "dropped_at_cap" not in source:
    fail("Skill 2 follow-up cap is not explicit and logged")
if "data line hit target=" not in source or "data_stun_expected_ticks" not in source:
    fail("Skill 2 data-hit diagnostics are missing")
if "fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize)" not in source or "sim.player_at(player)" not in source:
    fail("Decimation does not resolve the dead champion from the on_dead player index")
if "death callback resolved player=" not in source or "Decimation Spawn creation failed at=" not in source:
    fail("Decimation callback/creation diagnostics are incomplete")
if "fn tick_active_frenzy_spawns" not in source or "tick_active_frenzy_spawns(sim);" not in source:
    fail("Decimation scheduler is still coupled to the dead champion tick")
if "Decimation scheduler active spawn=" not in source or "no valid enemy within ray_range=" not in source:
    fail("Decimation scheduler diagnostics do not distinguish scheduler/no-target state")
if "queueing Spawn for regular match tick" not in source or "Decimation Spawn verified in match simulation" not in source:
    fail("Decimation creation is not deferred/verified in the match context")
if "Decimation Spawn create returned id=" not in source or "retrying creation in match context" not in source:
    fail("Decimation Spawn lacks missing-entity recovery diagnostics")
if "sim.add_buff(fid, &BuffV1::named(BUFF_SPAWN))" not in source:
    fail("Decimation Spawn is missing the standard S1 Spawn marker")
if "sim.deal_damage(fid, target" not in source:
    fail("Decimation Spawn does not own its independent attack path")
if "zhask_death_ray" in source:
    fail("obsolete Death Ray source binding remains")

# v1.1.63 icon, voice, and impact-visual integrity checks.
if champion.get("skill_icons") != ["asset/mlbb_heroes/icons/mlbb_zhask_skill", "asset/mlbb_heroes/icons/mlbb_zhask_skill2", "asset/mlbb_heroes/icons/mlbb_zhask_ult"]:
    fail("Zhask action icon slots changed unexpectedly")
for icon in ("mlbb_zhask_skill.png", "mlbb_zhask_skill2.png", "mlbb_zhask_ult.png"):
    icon_path = ROOT / "icons" / icon
    if not icon_path.is_file():
        fail(f"missing action icon {icon}")
    if icon_path.read_bytes()[:8] != b"\x89PNG\r\n\x1a\n":
        fail(f"action icon {icon} is not encoded as PNG")
if "mlbb_heroes:audio_voice_zhask_ult" not in str(champion.get("ult", {})):
    fail("Zhask initial Ultimate is missing its randomized voice cue")
for suffix in ("#sheet.png", "#anim.fanim"):
    if not (ROOT / f"effects/zhask_mind_eater{suffix}").is_file():
        fail(f"missing enhanced Mind Eater travel asset {suffix}")
    if not (ROOT / f"effects/zhask_mind_eater_impact{suffix}").is_file():
        fail(f"missing Mind Eater impact asset {suffix}")
for cue in ("ult01", "ult02", "ult03", "death01", "death02", "death03", "death04"):
    name = f"mlbb_zhask_voice_{cue}"
    clip = f"mlbb_heroes_clip_{name}"
    if not (ROOT / "sound/sfx" / f"{name}.sound_info").is_file() or not (ROOT / "sound/sfx" / f"{clip}.wav").is_file():
        fail(f"missing Zhask voice clip asset/metadata for {cue}")
    if f"{name}" not in (ROOT / "src/audio.rs").read_text(encoding="utf-8"):
        fail(f"missing shared voice-gating registration for {cue}")

registered = set(re.findall(r'add_native_(?:passive|effect)\(\s*format!\(\s*\"\{MOD_ID\}:(\w+)\"', source))
audio_source = (ROOT / "src/audio.rs").read_text(encoding="utf-8")
registered.update(re.findall(r'add_native_effect\(\s*\"mlbb_heroes:(\w+)\"', audio_source))
refs = set()

def collect_refs(value):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in {"effect_ref", "passive_ref"} and isinstance(child, str) and child.startswith("mlbb_heroes:"):
                refs.add(child.split(":", 1)[1])
            collect_refs(child)
    elif isinstance(value, list):
        for child in value:
            collect_refs(child)

collect_refs(champion)
missing = refs - registered
if missing:
    fail(f"unregistered Zhask native references: {sorted(missing)}")

# Every native Spawn attack must be inert so the custom scheduler remains the sole damage path.
attack_blocks = re.findall(r"let atk = UnitAttackV1 \{(.*?)\n\s*\};", source, re.S)
if len(attack_blocks) != 4:
    fail(f"expected four native Spawn attack definitions, found {len(attack_blocks)}")
for index, block in enumerate(attack_blocks):
    for field in ("attack_ratio", "attack", "range"):
        if not re.search(rf"\b{field}:\s*0\s*,", block):
            fail(f"Spawn UnitAttackV1 #{index + 1} has nonzero {field}")

print("PASS: v1.1.63 metadata, action icons, gated voice clips, enhanced S2 travel/impact assets, data-defined S2 line/Stun with capped native bonus queue, deferred and verified Decimation creation plus owner-independent scheduler, beam mapping, and inert Spawn attacks")
