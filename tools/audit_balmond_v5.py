#!/usr/bin/env python3
"""Read-only structural and formula audit for the Balmond v5 checkpoint."""
from __future__ import annotations
import json
import math
import re
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
champ_path = ROOT / "champion/mlbb_balmond.data_champion"
source_path = ROOT / "src/balmond.rs"
champ = json.loads(champ_path.read_text(encoding="utf-8"))
source = source_path.read_text(encoding="utf-8")
errors: list[str] = []
checks: list[str] = []

def require(condition: bool, message: str) -> None:
    (checks if condition else errors).append(("PASS: " if condition else "FAIL: ") + message)

def walk(obj):
    if isinstance(obj, dict):
        yield obj
        for value in obj.values():
            yield from walk(value)
    elif isinstance(obj, list):
        for value in obj:
            yield from walk(value)

# Champion JSON and v2 Skill 1 movement contract.
s1 = champ["skill"]
require(s1["casting_type"] == "Direction", "Skill 1 accepts direction input, not a selected enemy")
require(s1["range"] == 90000, "Skill 1 action range remains 90,000 TFM2 units")
require("MoveToTarget" not in {x.get("type") for x in walk(s1["effect"])}, "Skill 1 has no target-tracking MoveToTarget effect")
rush = next((x for x in walk(s1["effect"]) if x.get("type") == "RushTime"), None)
require(rush is not None, "Skill 1 uses the schema-supported RushTime movement effect")
if rush:
    require(rush.get("casting_target") == "EnemyChampion" and rush.get("penetrate") is False,
            "Skill 1 rush collides with the first enemy champion")
    require(rush.get("range") == 90000 and rush.get("tick") == 13 and rush.get("speed") == 7000,
            "Skill 1 dash range/ticks/speed are internally consistent with the documented assumption")
    hit_refs = [x.get("effect_ref") for x in walk(rush.get("applied_effects", [])) if x.get("type") == "Native"]
    require("mlbb_heroes:balmond_soul_lock_hit" in hit_refs, "Skill 1 champion collision invokes the registered hit effect")
path_refs = [x.get("effect_ref") for x in walk(s1["effect"]) if x.get("type") == "Native"]
require("mlbb_heroes:balmond_soul_lock_path" in path_refs, "Skill 1 invokes its native non-champion path scan")

# Native references in data all have matching registrations.
registration_sources = source + "\n" + (ROOT / "src/lib_balmond.patch").read_text(encoding="utf-8") + "\n" + (ROOT / "patches/balmond_candidate_v1.patch").read_text(encoding="utf-8")
registration_sources = re.sub(r"(?m)^\+", "", registration_sources)
registered = set(re.findall(r'add_native_(?:effect|passive)\(\s*"([^"]+)"', registration_sources))
refs = {x.get("effect_ref") for x in walk(champ) if x.get("type") == "Native" and x.get("effect_ref")}
passive_refs = {a.get("passive_ref") for a in walk(champ) if a.get("passive_ref")}
missing = (refs | passive_refs) - registered
require(not missing, f"all champion native/passive refs are registered ({len(refs)} effects, {len(passive_refs)} passive refs; missing={sorted(missing)})")

# Skill role/action and native logic invariants.
require(champ["skill2"]["casting_target"] == "EnemyWithoutTower" and champ["skill2"]["range"] == 35000 and champ["skill2"]["duration"] == 180,
        "Skill 2 uses the proven non-tower target filter/range while keeping its 180-tick self-centered spin")
s2_triggers = list(walk(champ["skill2"]["effect"]))
require(any(x.get("type") == "CasterAnimation" and x.get("name") == "skill2" and x.get("tick") == 180 for x in s2_triggers),
        "Skill 2 explicitly plays its body animation tag for the spin duration")
require(any(x.get("type") == "Native" and x.get("effect_ref") == "mlbb_heroes:balmond_cyclone_start" for x in s2_triggers),
        "Skill 2 retains its native spin mechanics")
require(any(x.get("type") == "CasterViewEffect" and x.get("name") == "mlbb_balmond_s1_dash" for x in walk(champ["skill"]["effect"])),
        "Skill 1 trail is triggered at the caster position")
require(any(x.get("type") == "CasterAnimation" and x.get("name") == "ult" and x.get("tick") == 30 for x in walk(champ["ult"]["effect"])),
        "Ultimate explicitly plays the body tag for an approximate 0.5-second mapping (30 ticks; not an exact recovered duration)")
require(any(x.get("type") == "CasterViewEffect" and x.get("name") == "mlbb_balmond_ult_slash" for x in walk(champ["ult"]["effect"])),
        "Ultimate ground fan visual is triggered at Balmond's position")
require(champ["ult"]["casting_type"] == "Direction", "Ultimate remains direction-cast")
require("S2_HITS: usize = 14" in source and "S2_RADIUS: f64 = 36_000.0" in source,
        "Skill 2 retains its explicitly assumed 14-hit/36,000-radius TFM2 values")
require("const S2_MAX_SHRED_STACKS: usize = 8" in source and "shred.defence_mult = -(5 * next as i32)" in source,
        "Skill 2 retains the 8-stack, 5%-per-stack defense shred")
require("missing_hp_damage(" in source and "lethal_counter_damage(attack, max_hp, hp, level, is_champion)" in source,
        "Ultimate uses live current/max HP in a per-target missing-health calculation")
require("input.kind != InputTargetKindV1::Target.code()" in source[source.index("struct BalmondLethalCounter"):source.index("pub fn register")],
        "Ultimate native execute handler requires the target supplied by the data fan")
require("DamageTypeV1::Fixed" in source and "amount.min(ult_nonhero_cap(level))" in source,
        "Ultimate uses fixed damage and applies its non-hero cap")
ult_ranges = [x for x in walk(champ["ult"]["effect"]) if x.get("type") == "RangeEffect"]
ult_fan = next((x for x in ult_ranges if "DirDot" in x.get("shape", {})), None)
require(ult_fan is not None, "Ultimate fan geometry is selected by a data-driven RangeEffect DirDot")
if ult_fan:
    shape = ult_fan["shape"]["DirDot"]
    require(shape.get("radius") == 110000 and shape.get("range") == 707,
            "Ultimate fan data uses 110,000 radius and the inferred cosine-threshold approximation 707")
    require(ult_fan.get("target") == "EnemyWithoutTower", "Ultimate fan selects enemies and excludes towers")
    native_children = [x.get("effect_ref") for x in walk(ult_fan.get("effects", [])) if x.get("type") == "Native"]
    require(native_children == ["mlbb_heroes:balmond_lethal_counter"],
            "Ultimate fan delegates only per-target execute calculation to native code")
require("min_cos" not in source and "input.dir_x as f64" not in source[source.index("struct BalmondLethalCounter"):source.index("pub fn register")],
        "Ultimate native handler no longer computes fan geometry")
require("let mut stop_at = 90_000.0_f64" in source and "along <= stop_at && across <= 20_000.0" in source,
        "Skill 1 path scan is clipped to first projected hero and the documented 20,000 half-width")
require("|| target.is_champion()" in source, "Skill 1 path scan excludes champions handled by the collision effect")

# Data names, animation tags and VFX source files stay synchronized.
def read_fanim(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))
body_anim = read_fanim(ROOT / "sprite/balmond#anim.fanim")
body_tags = set(body_anim.get("anims", {}))
for slot in ("attack", "skill", "skill2", "ult"):
    require(slot in body_tags, f"body animation tag exists for {slot}")
effect_defs = {x.get("name"): x for x in champ.get("view_effects", [])}
for name in ("mlbb_balmond_s1_dash", "mlbb_balmond_ult_slash"):
    require(name in effect_defs, f"view effect name is defined: {name}")
    if name in effect_defs:
        rel = effect_defs[name]["anim"].split("asset/mlbb_heroes/", 1)[-1]
        base = ROOT / rel
        require(base.with_name(base.name + "#anim.fanim").is_file(), f"effect animation exists for {name}")
        require(base.with_name(base.name + "#sheet.png").is_file(), f"effect sheet exists for {name}")
buffs = {x.get("name"): x for x in champ.get("view_buffs", [])}
require("mlbb_heroes_balmond_s2_active" in buffs, "Skill 2 loop visual remains bound to its active buff")
if "mlbb_heroes_balmond_s2_active" in buffs:
    rel = buffs["mlbb_heroes_balmond_s2_active"]["anim"].split("asset/mlbb_heroes/", 1)[-1]
    base = ROOT / rel
    anim_path = base.with_name(base.name + "#anim.fanim")
    require(anim_path.is_file(), "Skill 2 buff animation asset exists")
    if anim_path.is_file():
        require(buffs["mlbb_heroes_balmond_s2_active"].get("tag") in read_fanim(anim_path).get("anims", {}),
                "Skill 2 buff animation tag is present in its FAnim")

# Independent arithmetic regression checks mirror Rust helper expectations.
def level_linear(low: int, high: int, level: int) -> int:
    return low + (high-low) * min(max(level-1, 0), 14) // 14
def s1_damage(attack: int, level: int) -> int:
    return level_linear(150,275,level) + attack*60//100
def missing_hp_damage(max_hp: int, hp: int, ratio: int) -> int:
    return max(max_hp-hp,0)*ratio//1000
require((s1_damage(100,1), s1_damage(100,15)) == (210,335), "Skill 1 damage endpoint arithmetic")
require((missing_hp_damage(2000,1000,300), missing_hp_damage(2000,1000,450)) == (300,450),
        "Ultimate missing-health ratio arithmetic")
require((1000 + 500*min((11-1)//5,2)) == 2000, "Ultimate non-hero cap top tier arithmetic")
def lethal_counter_damage(attack: int, max_hp: int, current_hp: int, level: int, is_champion: bool) -> int:
    base=150+attack*70//100
    ratio=300+75*min((max(level,1)-1)//5,2)
    raw=base+max(max_hp-current_hp,0)*ratio//1000
    return raw if is_champion else min(raw,1000+500*min((max(level,1)-1)//5,2))
require(lethal_counter_damage(100,2000,1000,1,True)==520, "per-target Ultimate hero damage arithmetic")
require(lethal_counter_damage(1000,10000,0,11,False)==2000, "per-target Ultimate non-hero cap arithmetic")
require(lethal_counter_damage(100,2000,2000,1,True)==220, "per-target Ultimate full-health baseline arithmetic")

def point_projection(origin, point, ux, uy):
    px, py = point[0]-origin[0], point[1]-origin[1]
    return px*ux+py*uy, abs(px*uy-py*ux)
require(point_projection((0,0),(30000,4000),1.0,0.0) == (30000.0,4000.0),
        "forward dash lane projection")
require(point_projection((0,0),(30000,4000),0.0,1.0) == (4000.0,30000.0),
        "perpendicular dash lane projection")

# FAnim and PNG dimensions, crops, and frame timing.
root_body_anim = ROOT / "balmond#anim.fanim"
anim_files = ([root_body_anim] if root_body_anim.is_file() else []) + sorted((ROOT / "sprite").glob("*.fanim")) + sorted((ROOT / "effects").glob("*.fanim"))
require(len(anim_files) >= 4, "body and three effect FAnim files are present")
for anim_path in anim_files:
    try:
        anim = json.loads(anim_path.read_text(encoding="utf-8"))
        sheet_path = anim_path.with_name(anim_path.name.replace("#anim.fanim", "#sheet.png"))
        require(sheet_path.is_file(), f"sheet exists for {anim_path.relative_to(ROOT)}")
        if not sheet_path.is_file():
            continue
        with Image.open(sheet_path) as image:
            sw, sh = image.size
        frame_count = 0
        bounded = True
        positive = True
        for animation in anim.get("anims", {}).values():
            for frame in animation.get("frames", []):
                frame_count += 1
                data = frame.get("data", {})
                x,y,w,h = (float(data.get(k, 0)) for k in ("x","y","w","h"))
                bounded &= x >= -0.01 and y >= -0.01 and w > 0 and h > 0 and x+w <= sw+0.05 and y+h <= sh+0.05
                positive &= float(frame.get("duration", 0)) > 0
        require(frame_count > 0, f"frames exist in {anim_path.relative_to(ROOT)}")
        require(bounded, f"all atlas crops are inside {sheet_path.name} ({sw}x{sh})")
        require(positive, f"all frame durations are positive in {anim_path.relative_to(ROOT)}")
    except Exception as exc:
        errors.append(f"FAIL: animation parse/audit {anim_path}: {exc}")

for name in ("balmond_passive.png","balmond_skill1.png","balmond_skill2.png","balmond_ult.png"):
    path = ROOT / "icons" / name
    require(path.is_file(), f"runtime icon present: {name}")
    if path.is_file():
        with Image.open(path) as im:
            require(im.width > 0 and im.height > 0, f"icon decodes: {name} ({im.width}x{im.height})")

# The existing S1 trail artwork is downscaled to approximately 65% without altering frame timing.
s1_sheet = ROOT / "effects/balmond_s1_fx#sheet.png"
with Image.open(s1_sheet) as im:
    require(im.size == (354, 118), "S1 trail sheet is downscaled to 354x118 (~65.2% of original)")
s1_anim = read_fanim(ROOT / "effects/balmond_s1_fx#anim.fanim")
s1_frames = s1_anim.get("anims", {}).get("dash", {}).get("frames", [])
require(len(s1_frames) == 6 and all(f.get("data", {}).get("w") == 59 and f.get("data", {}).get("h") == 118 for f in s1_frames),
        "all six S1 trail frames use 59x118 crops after scaling")
print("Balmond v5 static audit")
for line in checks + errors:
    print(line)
print(f"\n{len(checks)} checks passed; {len(errors)} failures")
raise SystemExit(1 if errors else 0)
