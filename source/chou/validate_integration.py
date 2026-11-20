#!/usr/bin/env python3
"""Static consistency checks for the Chou integration; not a TFM2 runtime test."""
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
errors: list[str] = []

def read_json(path: Path):
    try:
        return json.loads(path.read_text())
    except Exception as exc:
        errors.append(f"invalid JSON {path.relative_to(ROOT)}: {exc}")
        return None

# Parse all user-facing registry files and champion data.
champion = read_json(ROOT / "champion/mlbb_chou.data_champion") or {}
overrides = read_json(ROOT / "mod.override_info") or {}
style = read_json(ROOT / "style/champion_view.champion_view") or {}
text = read_json(ROOT / "text/champion.i18n") or {}
for path in (ROOT / "champion").glob("*.data_champion"):
    read_json(path)

if champion.get("sprite") != "asset/mlbb_heroes/sprite/chou":
    errors.append("Chou sprite root is unexpected")
for suffix in ("#sheet.png", "#anim.fanim"):
    if not (ROOT / "sprite" / ("chou" + suffix)).is_file():
        errors.append(f"missing active sprite asset: chou{suffix}")
for icon in champion.get("skill_icons", []):
    path = ROOT / icon.replace("asset/mlbb_heroes/", "")
    if not path.suffix:
        path = path.with_suffix(".png")
    if not path.is_file():
        errors.append(f"missing icon: {icon}")

atlas = Image.open(ROOT / "sprite/chou#sheet.png").convert("RGBA")
fanim = read_json(ROOT / "sprite/chou#anim.fanim") or {}
for tag, anim in fanim.get("anims", {}).items():
    for i, frame in enumerate(anim.get("frames", [])):
        rect = frame.get("data", {})
        if frame.get("duration", 0) <= 0:
            errors.append(f"non-positive duration {tag}[{i}]")
        if rect.get("x", -1) < 0 or rect.get("y", -1) < 0 or rect.get("x", 0) + rect.get("w", 0) > atlas.width or rect.get("y", 0) + rect.get("h", 0) > atlas.height:
            errors.append(f"frame outside sheet {tag}[{i}]")

# Ensure all animation overrides referenced by Chou's data are present.
def walk(value):
    if isinstance(value, dict):
        if value.get("type") == "CasterAnimation":
            yield value.get("name")
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)
for tag in walk(champion):
    if tag not in fanim.get("anims", {}):
        errors.append(f"missing animation tag: {tag}")

# Timed CasterAnimation selectors must cover the referenced sprite clip, not a
# one-tick flash. Frame durations are seconds; the runtime clock is 60 ticks/s.
def check_animation_durations(value):
    if isinstance(value, dict):
        if value.get("type") == "CasterAnimation":
            tag = value.get("name")
            anim = fanim.get("anims", {}).get(tag, {})
            frames = anim.get("frames", [])
            clip_ticks = sum(frame.get("duration", 0) for frame in frames) * 60
            expected_ticks = int(clip_ticks + 0.999999)
            if value.get("tick") != expected_ticks:
                errors.append(
                    f"CasterAnimation {tag!r} lasts {value.get('tick')} ticks; "
                    f"sprite clip requires {expected_ticks}"
                )
        for child in value.values():
            check_animation_durations(child)
    elif isinstance(value, list):
        for child in value:
            check_animation_durations(child)
check_animation_durations(champion)
if champion.get("skill", {}).get("cooltime_use_count") != 3:
    errors.append("Chou S1 must retain its three-charge Riven-style action")
if champion.get("skill", {}).get("duration") != 45:
    errors.append("Experimental Chou S1 action must be 45 ticks")
for tag in ("skill", "skill_recast", "skill_finisher"):
    total_ticks = sum(frame.get("duration", 0) for frame in fanim.get("anims", {}).get(tag, {}).get("frames", [])) * 60
    if abs(total_ticks - 45) > 1e-5:
        errors.append(f"experimental S1 animation {tag!r} must total 45 ticks; got {total_ticks}")

# Verify S1's third-cast landing CC is a data-driven delayed area effect, and
# Ultimate recast has airborne distinct from first-phase knockback.
def nodes_of_type(value, type_name):
    found = []
    if isinstance(value, dict):
        if value.get("type") == type_name:
            found.append(value)
        for child in value.values():
            found.extend(nodes_of_type(child, type_name))
    elif isinstance(value, list):
        for child in value:
            found.extend(nodes_of_type(child, type_name))
    return found

s1_delayed = [n for n in nodes_of_type(champion.get("skill", {}).get("effect", {}), "Delayed") if n.get("tick") == 9]
if not any(
    any(
        rng.get("shape", {}).get("Circle", {}).get("radius") == 30000
        and rng.get("apply_type") == "AroundCaster"
        and any(a.get("type") == "Airborne" and a.get("duration") == 30 for a in nodes_of_type(rng.get("effects", []), "Airborne"))
        for rng in nodes_of_type(delay.get("effects", []), "RangeEffect")
    )
    for delay in s1_delayed
):
    errors.append("S1 finisher needs a 30,000-radius delayed data Airborne at tick 9")

ult_recast_switch = next((n for n in nodes_of_type(champion.get("ult", {}).get("effect", {}), "SwitchByBuff") if n.get("buff_name") == "mlbb_chou_ult_recast"), None)
if not ult_recast_switch or not any(n.get("duration") == 60 for n in nodes_of_type(ult_recast_switch.get("effect_buff", {}), "Airborne")):
    errors.append("Ultimate recast must include 60-tick data-driven Airborne")
if champion.get("ult", {}).get("range") != 20000:
    errors.append("Ultimate base range must keep first-phase targeting close (20,000)")
if not any(n.get("effect_ref") == "mlbb_heroes:chou_ult_range" for n in nodes_of_type(champion.get("ult", {}).get("effect", {}), "Native")):
    errors.append("Ultimate needs the recast-buff-gated dynamic range marker")
source_chou = (ROOT / "src/chou.rs").read_text()
if "const ULT_FIRST_RANGE: f64 = 20_000.0;" not in source_chou or "const ULT_FIRST_AI_RANGE: f64 = 18_000.0;" not in source_chou:
    errors.append("Ultimate first phase must be gated at 20,000 with an 18,000 AI trigger")
# Guard the deliberate v1.1.40 damage test against accidental scope creep.
for expected in (
    "const S1_DAMAGE_MIN: usize = 100;",
    "const S1_DAMAGE_MAX: usize = 165;",
    "const S1_DAMAGE_AD_RATIO: usize = 45;",
    "const ULT_FIRST_DAMAGE_MIN: usize = 275;",
    "const ULT_FIRST_DAMAGE_MAX: usize = 420;",
    "const ULT_FIRST_EXTRA_AD_RATIO: usize = 110;",
    "const ULT_RECAST_DAMAGE_MIN: usize = 330;",
    "const ULT_RECAST_DAMAGE_MAX: usize = 500;",
    "const ULT_RECAST_EXTRA_AD_RATIO: usize = 132;",
):
    if expected not in source_chou:
        errors.append(f"missing v1.1.40 conservative damage setting: {expected}")
for lang in ("en", "zh-hans", "zh-hant", "ko", "ja"):
    chou_text = text.get(lang, {}).get("description", {}).get("mlbb_chou", {})
    joined = " ".join(value for value in chou_text.values() if isinstance(value, str))
    for value in ("100–165", "275–420", "330–500"):
        if value not in joined:
            errors.append(f"Chou {lang} localization is missing updated damage value {value}")
if "const ULT_RECAST_RANGE_BONUS: u64 = 125_000;" not in source_chou or "fn may_adjust_range(&self)" not in source_chou or "caster.has_buff(ULT_RECAST)" not in source_chou:
    errors.append("Ultimate must add 125,000 range only while the recast buff is present")
if "const ULT_TARGET_RANGE: f64 = 145_000.0;" not in source_chou:
    errors.append("Ultimate AI recast chase target limit must remain 145,000")
if "const S1_DASH_DISTANCE: f64 = 20_000.0;" not in source_chou:
    errors.append("Experimental S1 dash should be 20,000 units")
if "S1_FINISHER_HIT" not in source_chou or "combo S2 behind" not in source_chou:
    errors.append("Chou combo candidate must gate the backside S2 on a confirmed S1 hero hit")
if "const ULT_KNOCKBACK_SPEED: usize = 10_000;" not in source_chou or "const ULT_KNOCKBACK_TICKS: usize = 10;" not in source_chou:
    errors.append("v1.1.39 kick-distance experiment must use 10,000 × 10 = 100,000 knockback")
if "caster_start=" not in source_chou or "at_position={:?}" not in source_chou:
    errors.append("Ultimate knockback trace must include start/end position diagnostics")

# Verify all effect references used in Chou's data are registered in src/audio.rs or src/chou.rs.
refs = set()
def collect_refs(value):
    if isinstance(value, dict):
        if value.get("type") == "Native":
            refs.add(value.get("effect_ref"))
        for child in value.values(): collect_refs(child)
    elif isinstance(value, list):
        for child in value: collect_refs(child)
collect_refs(champion)
source_text = (ROOT / "src/chou.rs").read_text() + (ROOT / "src/audio.rs").read_text()
for ref in sorted(filter(None, refs)):
    tail = ref.split(":", 1)[-1]
    search_key = tail.removeprefix("audio_voice_") if tail.startswith("audio_voice_") else tail
    if search_key not in source_text:
        errors.append(f"native effect not found in registration/source: {ref}")

# Audio event and clip entries must exist, parse, and have an override.
for event in ("mlbb_chou_voice_skill1_a", "mlbb_chou_voice_skill1_b", "mlbb_chou_voice_skill2", "mlbb_chou_voice_ult"):
    info_path = ROOT / "sound/sfx" / f"{event}.sound_info"
    info = read_json(info_path) or {}
    wav_path = ROOT / "sound/sfx" / f"{event}.wav"
    if not wav_path.is_file(): errors.append(f"missing audio WAV: {wav_path.name}")
    clips = [play.get("clip") for play in info.get("plays", [])]
    expected = f"mlbb_heroes_clip_{event}"
    if expected not in clips: errors.append(f"sound_info does not reference {expected}")
    for asset in (event, expected):
        key = f"asset/base/sound/sfx/{asset}"
        if key not in overrides: errors.append(f"missing override route: {key}")

# Localization and selection metadata must have all five supported locales.
for lang in ("en", "zh-hans", "zh-hant", "ko", "ja"):
    try:
        assert "mlbb_chou" in text[lang]["description"]
        assert "mlbb_chou" in text[lang]["skill_name"]
    except Exception:
        errors.append(f"missing Chou localization for {lang}")
if "mlbb_chou" not in style.get("entries", {}):
    errors.append("missing Chou selection framing")

if errors:
    print("FAIL")
    for error in errors: print(" -", error)
    raise SystemExit(1)
print("PASS: Chou JSON, sprite bounds, animation timings, staged S1 CC, Ultimate recast CC/range, native refs, audio, icons, localization, and style metadata are consistent.")
print("NOTE: this does not compile Rust or validate TFM2 runtime behavior.")
