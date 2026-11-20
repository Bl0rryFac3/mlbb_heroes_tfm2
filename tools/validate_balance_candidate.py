#!/usr/bin/env python3
"""Static checks for the separate v1.1.63 balance source candidate."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
def read(rel): return (ROOT / rel).read_text()
def require(condition, message):
    if not condition:
        raise SystemExit(f"FAIL: {message}")

meta = json.loads(read("mod.mod_info"))
require(meta["version"] == "1.1.63", "wrong mod version")
require(json.loads(read("champion/mlbb_gusion.data_champion"))["id"] == "mlbb_gusion", "Gusion data JSON invalid")
# Parse every champion definition, not just the modified one.
for path in (ROOT / "champion").glob("*.data_champion"):
    json.loads(path.read_text())

gusion = json.loads(read("champion/mlbb_gusion.data_champion"))
recast = gusion["skill"]["effect"]["effect_buff"]["effects"]
anims = [e for e in recast if e.get("type") == "CasterAnimation" and e.get("name") == "skill_recast"]
require(len(anims) == 1 and anims[0]["tick"] == 8, "Gusion S1 recast animation not shortened to 8 ticks")
gs = read("src/gusion.rs")
require("is_unsafe_under_enemy_tower_with_margin" in gs and "is_unsafe_tower_route" in gs, "Gusion recast lacks immediate tower/route guard")
require("let all = crate::units(sim);" in gs, "Gusion recast safety snapshot missing")

lib = read("src/lib.rs")
start = lib.index("fn tower_has_allied_tankers")
end = lib.index("/// Returns true when a move route", start)
require("unit.champion" in lib[start:end], "allied tower tankers are not limited to champions")
require("allied_minions_do_not_count_as_tower_tankers" in gs, "minion tower exception regression test missing")

chou = read("src/chou.rs")
for needle in ["chou_tower_target_is_safe", "unsafe_base_input", "retreat_from_tower", "is_unsafe_tower_route"]:
    require(needle in chou, f"Chou tower-safety change missing: {needle}")

uranus = read("src/uranus.rs")
require("const URANUS_DAMAGE_SCALE_PERCENT: usize = 60;" in uranus, "Uranus damage scale is not 60%")
require(uranus.count("* URANUS_DAMAGE_SCALE_PERCENT / 100") == 4, "all four direct S1/S2 damage components must use the shared scale")
require("100 + 20 * marks.min(MAX_MARKS)" in uranus, "Uranus mark scaling/cap changed unexpectedly")
require("s1_damage(1, 0, 0), 79" in uranus and "s1_damage(12, 0, 0), 131" in uranus, "Uranus S1 endpoints mismatch")
require("s1_damage(1, 0, 8), 205" in uranus and "s1_damage(1, 0, 99), 205" in uranus, "Uranus S1 mark cap mismatch")
require("s2_path_damage(1, 25), 51" in uranus and "s2_path_damage(12, 25), 123" in uranus, "Uranus S2 path samples mismatch")
require("s2_arrival_damage(1, 25), 108" in uranus and "s2_arrival_damage(12, 25), 252" in uranus, "Uranus S2 arrival samples mismatch")
require("s2_burst_damage(1, 25), 175" in uranus and "s2_burst_damage(12, 25), 409" in uranus, "Uranus S2 burst samples mismatch")
require("s2_shield_amount(1, 25), 350" in uranus and "s2_shield_amount(12, 25), 800" in uranus, "Uranus shield values changed unexpectedly")
require("fn chou_tower_target_is_safe" in chou, "Chou tower helper missing")

# Check every skill description and the generator template across all five locales.
locales = json.loads(read("text/champion.i18n"))
expected_locale_tokens = {
    "en": ("79–131 (+18% Total Magic Power)", "48–120 (+12% Total Magic Power)", "96–240 (+48% Total Magic Power)", "156–390 (+78% Total Magic Power)"),
    "zh-hans": ("79–131（+18%总魔法强度）", "48–120（+12%总魔法强度）", "96–240（+48%总魔法强度）", "156–390（+78%总魔法强度）"),
    "zh-hant": ("79–131（+18%總魔法強度）", "48–120（+12%總魔法強度）", "96–240（+48%總魔法強度）", "156–390（+78%總魔法強度）"),
    "ko": ("79–131(+총 마법력의 18%)", "48–120(+총 마법력의 12%)", "96–240(+총 마법력의 48%)", "156–390(+총 마법력의 78%)"),
    "ja": ("79～131（総魔法力の18%加算）", "48～120（総魔法力の12%加算）", "96～240（総魔法力の48%加算）", "156～390（総魔法力の78%加算）"),
}
for locale, tokens in expected_locale_tokens.items():
    require(locale in locales, f"Missing Uranus locale: {locale}")
    desc = locales[locale]["description"]["mlbb_uranus"]
    combined = desc["skill"] + desc["skill2"]
    for token in tokens:
        require(token in combined, f"Uranus {locale} description missing {token}")
    require("20%" in desc["skill"] and "32%" not in desc["skill"], f"Uranus {locale} mark description is stale")
    require(not any(token in combined for token in ("133–219", "133～219", "80–200", "160–400", "260–650")), f"Uranus {locale} contains stale damage values")
    require(desc["skill"] in read("tools/integrate_uranus.py") and desc["skill2"] in read("tools/integrate_uranus.py"), f"Uranus {locale} generator is out of sync")

# Independently verify the formula and integer-rounding sample outputs.
def level_lerp(a, b, level):
    level = max(1, min(12, level))
    span = 11
    step = level - 1
    return (a * (span - step) + b * step + span // 2) // span
s1_raw = (level_lerp(133, 219, 1) + 0 * 30 // 100) * 60 // 100
require(s1_raw == 79 and s1_raw * (100 + 20 * 8) // 100 == 205, "Uranus S1 sample damage mismatch")
require((level_lerp(80, 200, 12) + 25 * 20 // 100) * 60 // 100 == 123, "Uranus S2 path sample mismatch")
require((level_lerp(160, 400, 12) + 25 * 80 // 100) * 60 // 100 == 252, "Uranus S2 arrival sample mismatch")
require((level_lerp(260, 650, 12) + 25 * 130 // 100) * 60 // 100 == 409, "Uranus S2 burst sample mismatch")
print("PASS: v1.1.63 metadata and champion JSON; Gusion timing/tower guards; champion-only tower tankers; Chou tower checks; Uranus 60% S1/S2 formulas, 20% marks, unchanged shields, five descriptions, and generator sync")
