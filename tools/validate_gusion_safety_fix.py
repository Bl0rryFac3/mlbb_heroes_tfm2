#!/usr/bin/env python3
"""Static scope/regression checks for the Gusion S1/tower-safety candidate."""
from __future__ import annotations
import hashlib
import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
gusion = (ROOT / "src/gusion.rs").read_text()
zhask = (ROOT / "src/zhask.rs").read_text()
report = (ROOT / "GUSION_SAFETY_FIX_VALIDATION_REPORT.md").read_text()
release = (ROOT / "RELEASE_README.md").read_text()
mod_info = json.loads((ROOT / "mod.mod_info").read_text())
data = json.loads((ROOT / "champion/mlbb_gusion.data_champion").read_text())

checks = {
    "native S1 recasts are route-checked": "s1_recast: bool" in gusion and "gusion_s1_dash_safety(all, caster, target)" in gusion,
    "unsafe native S1 regression is covered": "native_s1_recast_with_safe_target_but_unsafe_dash_route_is_blocked" in gusion,
    "effect-time S1 safety remains": "S1 recast effect rejected after cast input" in gusion,
    "escape waits for consecutive safe-route ticks": "TOWER_ESCAPE_RELEASE_CONFIRM_TICKS: u8 = 8" in gusion and "safe_route_ticks" in gusion,
    "release-race regression is covered": "unsafe_route_returning_during_release_confirmation_keeps_escape_latched" in gusion,
    "existing outward extension and support tests remain": "tower_escape_extends_and_stays_latched_if_native_route_is_still_unsafe_at_exit" in gusion and "tower_escape_respects_existing_two_champion_tank_support" in gusion,
    "safe tower attack remains allowed": data["attack"].get("casting_target") == "Enemy" and "normal_attack_targeting_enemy_tower_is_not_mistaken_for_a_champion_dive" in gusion,
    "no Return-home input is issued": "InputV1::return_home()" not in gusion,
    "six-entry Zhask S2 follow-up cap and test remain": "const S2_FOLLOWUP_CAP: usize = 6;" in zhask and "skill2_follow_up_queue_honors_and_reports_the_explicit_cap" in zhask,
    "candidate report disclaims gameplay verification": "No in-game match has been run" in report and "in-game behavior remains unverified" in release,
    "candidate description excludes Chou patch": "excludes the Chou native-movement patch" in mod_info["description"],
}
for label, ok in checks.items():
    if not ok:
        raise SystemExit(f"FAIL: {label}")

baseline = json.loads((ROOT / "GUSION_BASELINE_SOURCE_HASHES.json").read_text())
for rel, expected in baseline.items():
    actual = hashlib.sha256((ROOT / rel).read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"FAIL: non-Gusion source changed from clean baseline: {rel}")

cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
assert mod_info["version"] == cargo["package"]["version"]
assert mod_info["mod_id"] == "mlbb_heroes"
print(f"PASS: Gusion safety scope; {len(baseline)} other Rust modules match clean baseline")
