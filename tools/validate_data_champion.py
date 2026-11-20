#!/usr/bin/env python3
"""Strict enum validation for a .data_champion file.

Born from a real load error:

  unknown variant `Fighter`, expected one of `AD`, `AP`, `Heal`, `Shield`,
  `Dot`, `CC`, `Range`, `Melee`, `Tank`, `Magic`

The engine deserialises these fields as Rust enums, so ANY value outside the
allowed set is a hard load failure. This script checks them BEFORE packaging.

Design rule: the allowed value sets below are taken from the game's own error
messages and from the set actually observed across the shipped roster. Where a
field is only known from observed usage, the script says so and uses a strict
subset check - never a guess.

Usage:  python3 validate_data_champion.py <file.data_champion> [...more]
Exit:   0 clean, 1 any error
"""
from __future__ import annotations

import json
import sys

# ---------------------------------------------------------------- enums
# Confirmed by the engine's own deserialisation error (exhaustive list).
VALID_TAGS = {"AD", "AP", "Heal", "Shield", "Dot", "CC", "Range", "Melee", "Tank", "Magic"}

# Observed across the 11 shipped champions. Not proven exhaustive, so a value
# outside this set is an ERROR worth stopping on rather than shipping blind.
# SOURCE OF TRUTH (2026-11-01): teamsamoyed/TeamfightManager2Mod
# docs/data-champion-schema/enums.md -- the official modding guide.
# View-binding types, from the official visual-bindings schema:
#   view_effects:     Animation, LoopAnimation
#   view_projectiles: Animated, Sprite, ThreePhase
#   view_buffs:       Animated, ThreePhase
# The validator accepts the union and checks the per-block fields; narrowing per
# block would reject shipped shapes (Estes' ult aura is `Animated` in view_buffs).
VIEW_BINDING_TYPES = {"Animation", "LoopAnimation", "Animated", "Sprite", "ThreePhase"}

ACTION_BLOCKS = ("attack", "skill", "skill2", "ult")
# Tank is confirmed by the official0.7.3 patch (18Nov2026); older published enum docs are stale.
OBSERVED_CATEGORY = {"Melee", "Range", "Magician", "Util", "Assassin", "Tank"}
OBSERVED_CASTING_TYPE = {"Targeting", "Position", "Direction", "None"}
OBSERVED_CASTING_TARGET = {
    "Ally", "AllyChampion", "AllyChampionInCC", "AllyNotSelf", "AllyOnlySelf",
    "Enemy", "EnemyWithoutTower", "EnemyChampion", "EnemyChampionInCC",
    "EnemyChampionRecentlyAttacked", "Both", "BothWithoutTower", "BothChampion",
    "None"}
OBSERVED_ATTACK_TYPE = {"BaseAttack", "Skill", "Dot", "DotIgnoreShield", "Item", "Well"}
# Confirmed 2026-11-01: "attack_effect_type" appears 14x across all 17
# .data_champion files in the V11 archive and is ONLY ever "Target", including
# inside Aldous's AoE ult crash (damage 110). "Skill" is NOT a valid variant and
# fails deserialisation with: expected one of `Target`, `EnemyTarget`, `EnemyAll`.
# The engine's message is the source of truth for the exhaustive set.
VALID_ATTACK_EFFECT_TYPE = {"Target", "EnemyTarget", "EnemyAll"}
# NOTE: EnemyAll / AllyAll are OBJECT forms in the schema, e.g.
#   { "EnemyAll": { "range": 40000 } }  -- not bare strings.
OBSERVED_ATTACK_EFFECT_TYPE = {"Target", "EnemyTarget", "EnemyAll"}
OBSERVED_EFFECT_TYPE = {
    "Native", "Animation", "Combine", "Animated", "CasterAnimation",
    "AddCasterBuff", "RangeEffect", "SwitchByBuff", "Attack", "ViewEffect",
    "ApplyInProjectile", "CasterViewEffect", "Heal", "LineRangeProjectile",
    "MoveToTarget", "AddBuff", "MoveTo", "Stun", "Sprite", "AddCasted",
    "Shield", "Delayed", "RushMoveToBack", "Airborne", "TargetSfx",
    "ThreePhase", "TargetProjectile", "RangePeriodProjectile", "ApAttack",
    "WithSelf", "RemoveCasterBuff", "RandomTarget", "BlockSkill",
    "FixedAttack", "RushTime", "Pull", "Sfx",
    # Added for Gord. Absent only because no MLBB hero had used them yet --
    # both are in this file's own PROJECTILE_TYPES (below) and both are
    # documented and shipped in the reference project:
    #   LinearProjectile  champion-data.md:397, "skillshot" (Lucian R fires 20)
    #   RangeProjectile   champion-data.md:401, the burst where a skillshot
    #                     stops (Jinx R, Fizz R, Ashe R; Gord's S1 splash)
    "LinearProjectile", "RangeProjectile",
    # CC effects are spelled with the CcKindV1 variant name (values.rs), paired
    # with a "duration" in ticks -- Belerick's ult taunt is the first Taunt in
    # the roster, so the type had never been observed here before. Taunt = 7 in
    # CcKindV1, right alongside the Stun/Airborne already listed above.
    "Taunt",
    # --- CORRECTED 2026-11-09, after reading the OFFICIAL guide -------------
    # https://github.com/teamsamoyed/TeamfightManager2Mod/blob/main/docs/
    #   data-champion-schema/effects.md
    # The full documented vocabulary is saved in this directory as
    #   TFM2_OFFICIAL_EFFECT_TYPES.md (55 types). Use that file, not guesswork.
    #
    # What went wrong in v2.0.9-rc.1, and what is now known:
    #   * CasterInvisible -- STILL WITHDRAWN, and it was the real culprit. It is
    #     not in the official vocabulary. I took it from the League reference
    #     repo (tfm2_league_ref), which describes a DIFFERENT mod.
    #   * Invisible -- RESTORED. It IS documented ("Makes the target invisible
    #     for `tick` ticks", {"type":"Invisible","tick":120}). I struck it out
    #     only because no shipped hero used it. That reasoning was wrong.
    #   * RemoveCasterBuff -- ALWAYS REAL, and it takes `name`, NOT `buff_name`.
    #     It is documented: "Removes a named normal buff from the caster."
    #     My rc.1 node used the wrong field name, which is what produced
    #     Error("missing field `name`", line: 243, column: 2). I also told the
    #     owner it did not exist at all. That was false.
    #
    # THE RULE, restated correctly in both directions:
    #   The official docs are the authority on what EXISTS. The shipped roster
    #   is only evidence of what we have USED. Absence from the roster does NOT
    #   make a type invalid (RemoveCasterBuff, Invisible). Absence from the docs
    #   is not proof either -- `Taunt` is shipped and undocumented. Prefer
    #   shipped-and-working, then documented; never neither.
    "Invisible", "Banish", "Teleport", "Rush", "Grab", "Knockback",
    "RemoveCasterAnimation", "ShrinkingBarrier",
}
OBSERVED_APPLY_TYPE = {"AroundCaster", "Forward"}
OBSERVED_SHAPE = {"Circle", "Line", "Rect", "DirDot"}
OBSERVED_HEAL_TYPE = {"Ally", "Caster", "Any", "AllyAll"}
OBSERVED_DAMAGE_TYPE_HINT = {"Fixed", "Physical", "Magic", "True"}

REQUIRED_RAW_ACTIONS = ("attack", "skill", "skill2", "ult")
OPTIONAL_RAW_ACTIONS = ("skill_recast", "skill2_recast", "ult_recast",
                        "skill_finisher", "basic", "walk")  # animation tags, not data keys

errors: list[str] = []
warns: list[str] = []
oks: list[str] = []


def err(m):
    errors.append(m)


def warn(m):
    warns.append(m)


def ok(m):
    oks.append(m)


# Plain (non-effect) enum-valued fields, checked anywhere they appear.
# This table exists because the FIRST version of this script checked only the
# fields the author remembered, and "attack_effect_type" was missed -> a real
# load error shipped. Anything enum-like belongs here.
PLAIN_ENUM_FIELDS = {
    "category":              OBSERVED_CATEGORY,
    "casting_type":          OBSERVED_CASTING_TYPE,
    "casting_target":        OBSERVED_CASTING_TARGET,
    "attack_type":           OBSERVED_ATTACK_TYPE,
    "attack_effect_type":    OBSERVED_ATTACK_EFFECT_TYPE,
    "apply_type":            OBSERVED_APPLY_TYPE,
    "heal_type":             OBSERVED_HEAL_TYPE,
    "damage_type":           OBSERVED_DAMAGE_TYPE_HINT,
}


def check_tagged_enum(value, path, allowed):
    """Some enum fields are either a bare string or a one-key object.

    Docs (`enums.md`): `DataRangeApplyType` is `"AroundCaster"` OR
    `{"Forward": {"offset": 24000}}`; `DataHealType` likewise has
    `{"AllyAll": {"range": ...}}`. Treating them as always-strings crashes.
    """
    if isinstance(value, str):
        if value not in allowed:
            err(f"{path}: '{value}' not in {sorted(allowed)}")
        return
    if isinstance(value, dict):
        if len(value) != 1:
            err(f"{path}: tagged enum object must have exactly one key, got {sorted(value)}")
            return
        key = next(iter(value))
        if key not in allowed:
            err(f"{path}: '{key}' not in {sorted(allowed)}")
        return
    err(f"{path}: expected string or single-key object, got {type(value).__name__}")


def walk_plain_enums(node, path):
    """Check every plain enum field, wherever it occurs in the tree."""
    if isinstance(node, dict):
        for k, v in node.items():
            allowed = PLAIN_ENUM_FIELDS.get(k)
            if allowed is not None and isinstance(v, dict):
                check_tagged_enum(v, f"{path}.{k}", allowed)
            elif allowed is not None and isinstance(v, str):
                if v not in allowed:
                    engine = ""
                    if k == "attack_effect_type":
                        engine = (f"  ENGINE SAYS: expected one of "
                                  f"{', '.join(sorted(VALID_ATTACK_EFFECT_TYPE))}")
                    err(f"{path}.{k}: '{v}' not in observed set "
                        f"{sorted(allowed)}{engine}")
            walk_plain_enums(v, f"{path}.{k}")
    elif isinstance(node, list):
        for i, v in enumerate(node):
            walk_plain_enums(v, f"{path}[{i}]")


# --- applied_effects: the ONE container with two legal entry shapes ---------
# Discovered 2026-11-01 from a real load error: "missing field `effect`".
#   bare:    {"type": "ViewEffect", "name": ...}          (Gusion only)
#   wrapper: {"effect": {...}, "casting_type": "Targeting"} (Estes/Lancelot/
#            Uranus/Zhask - i.e. every non-ViewEffect payload, incl. Stun)
# Writing an Attack/Stun bare makes serde try the wrapper variant, fail, and
# report the wrapper's mandatory field as missing. Exactly the error seen.
BARE_OK_IN_APPLIED = {"ViewEffect"}


def check_applied_effects(node, path):
    """Validate applied_effects entry shape wherever it appears."""
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "applied_effects" and isinstance(v, list):
                for i, entry in enumerate(v):
                    ep = f"{path}.{k}[{i}]"
                    if not isinstance(entry, dict):
                        err(f"{ep}: not an object")
                        continue
                    if "effect" in entry:
                        if "casting_type" not in entry:
                            err(f"{ep}: wrapper entry missing 'casting_type'")
                        elif entry["casting_type"] not in OBSERVED_CASTING_TYPE:
                            err(f"{ep}: bad casting_type '{entry['casting_type']}'")
                        if not isinstance(entry["effect"], dict):
                            err(f"{ep}: 'effect' must be an object")
                    elif "type" in entry:
                        if entry["type"] not in BARE_OK_IN_APPLIED:
                            err(f"{ep}: bare '{entry['type']}' is not a valid "
                                f"applied_effects entry. Only "
                                f"{sorted(BARE_OK_IN_APPLIED)} may be bare; wrap it: "
                                f'{{\"effect\": {{...}}, \"casting_type\": \"Targeting\"}}')
                    else:
                        err(f"{ep}: entry has neither 'type' nor 'effect'")
            check_applied_effects(v, f"{path}.{k}")
    elif isinstance(node, list):
        for i, v in enumerate(node):
            check_applied_effects(v, f"{path}[{i}]")


DAMAGE_TYPES = {"Attack", "ApAttack", "FixedAttack"}
CC_TYPES = {"Stun", "Airborne", "Knockback", "Grab", "Pull", "Fear", "Charm",
            "Bind", "Taunt", "BlockAttack", "BlockSkill", "BlockMoveSkill",
            "Invisible", "Banish"}
PROJECTILE_TYPES = {
    "LinearProjectile", "BackToCasterLinearProjectile", "TargetProjectile",
    "TargetProjectileFromProjectile", "TargetSplashProjectile",
    "AutoTargetProjectile", "RangeProjectile", "LineRangeProjectile",
    "RangePeriodProjectile", "ApplyInProjectile", "ParabolicProjectile",
    "ShrinkingBarrier"}


def check_target_defaults(effect, action, path, action_target=None):
    """Docs warn 3x: the Rust default for target enums is `Ally`.

    A damage/CC effect with no explicit target anywhere above it will silently
    hit allies. The `attack` slot is exempt: its target comes from the action.
    """
    if action == "attack":
        return
    def walk(node, p, inherited):
        if isinstance(node, dict):
            t = node.get("type")
            ctx = dict(inherited)
            if t == "RangeEffect":
                if "target" in node:
                    ctx["target"] = node["target"]
                else:
                    err(f"{p}: RangeEffect with no explicit `target` "
                        f"(Rust default is Ally)")
            if t in PROJECTILE_TYPES:
                if "applied_target" in node:
                    ctx["target"] = node["applied_target"]
                elif t in ("ShrinkingBarrier",):
                    pass
                else:
                    err(f"{p}: {t} with no `applied_target` (Rust default is Ally)")
            if t in DAMAGE_TYPES or t in CC_TYPES:
                if not (ctx.get("target") or node.get("target")):
                    err(f"{p}: {t} has no explicit target above it -> would hit allies")
            for k, v in node.items():
                walk(v, f"{p}.{k}", ctx)
        elif isinstance(node, list):
            for i, v in enumerate(node):
                walk(v, f"{p}[{i}]", inherited)
    # The action's own casting_target seeds the context for the whole tree
    # (Chou's shipped ult relies on this: its Airborne sits inside a
    # SwitchByBuff with no local target). Only nodes that RE-OWN targeting
    # -- RangeEffect.target / projectile.applied_target -- must be explicit.
    seed = {"target": action_target} if action_target else {}
    walk(effect, path, seed)


def walk_effects(node, path, root):
    """Recursively check every effect object."""
    if isinstance(node, dict):
        t = node.get("type")
        if isinstance(t, str):
            if t not in OBSERVED_EFFECT_TYPE:
                err(f"{path}: unknown effect type '{t}'")
            if t == "RangeEffect":
                shape = node.get("shape")
                if not isinstance(shape, dict) or not shape:
                    err(f"{path}: RangeEffect with no shape")
                else:
                    for k in shape:
                        if k not in OBSERVED_SHAPE:
                            err(f"{path}: unknown shape '{k}'")
                if "apply_type" in node:
                    check_tagged_enum(node["apply_type"], f"{path}.apply_type",
                                      {"AroundCaster", "Forward"})
            if t == "Heal" and "heal_type" in node:
                if node["heal_type"] not in OBSERVED_HEAL_TYPE:
                    warn(f"{path}: heal_type '{node['heal_type']}' unverified")
        for k, v in node.items():
            walk_effects(v, f"{path}.{k}", root)
    elif isinstance(node, list):
        for i, v in enumerate(node):
            walk_effects(v, f"{path}[{i}]", root)


def walk_collect(node, type_name, out):
    """Collect the `name` of every effect node whose "type" == type_name."""
    if isinstance(node, dict):
        if node.get("type") == type_name and node.get("name"):
            out.append(node["name"])
        for v in node.values():
            walk_collect(v, type_name, out)
    elif isinstance(node, list):
        for v in node:
            walk_collect(v, type_name, out)


def check(path: str) -> None:
    try:
        d = json.loads(open(path, encoding="utf-8").read())
    except json.JSONDecodeError as exc:
        err(f"{path}: not valid JSON ({exc})")
        return

    name = path.rsplit("/", 1)[-1]

    # --- generic sweep: every plain enum field, everywhere ---
    walk_plain_enums(d, name)
    check_applied_effects(d, name)

    # --- target-default audit, per action slot ---
    for action in ("attack", "skill", "skill2", "ult"):
        act = d.get(action)
        if isinstance(act, dict) and isinstance(act.get("effect"), dict):
            check_target_defaults(act["effect"], action, f"{name}.{action}.effect",
                                  act.get("casting_target"))

    # --- tags: the field that actually broke ---
    tags = d.get("tags")
    if not isinstance(tags, list) or not tags:
        err(f"tags: missing or empty")
    else:
        for t in tags:
            if t not in VALID_TAGS:
                err(f"tags: '{t}' is not a valid TagV1. Allowed: "
                    f"{', '.join(sorted(VALID_TAGS))}")
        if len(set(tags)) != len(tags):
            warn("tags: duplicate entries")
        if tags and all(t in VALID_TAGS for t in tags):
            ok(f"tags {tags}")

    # --- passive block ---------------------------------------------------
    # ADDED 2026-11-09 after Johnson rc.1 failed to load with
    #   data_champion load error: Error("missing field `passive_ref`", ...)
    # The engine deserialises `passive` into a struct that REQUIRES
    # `passive_ref`; a passive written as an action/effect block (with
    # `action_name` / `description` / `effect`) deserialises to a missing
    # field and the whole champion fails to load. There was no check for
    # this at all, so the file validated clean while being unloadable.
    passive = d.get("passive")
    if not isinstance(passive, dict):
        err("passive: missing — the engine requires a `passive` object with a "
            "`passive_ref` (the champion will not load without it)")
    else:
        pref = passive.get("passive_ref")
        if not isinstance(pref, str) or not pref:
            err("passive.passive_ref: missing or empty — required by the engine; "
                "it must name a native registered with add_native_passive")
        else:
            ok(f"passive.passive_ref {pref}")
        if "params" in passive and not isinstance(passive["params"], dict):
            err("passive.params: must be an object")
        # Keys the engine does NOT accept here. Action-shaped keys mean the
        # passive was written like a skill, which is exactly the rc.1 bug.
        ACTION_SHAPED = ("effect", "action_name", "description", "casting_target",
                         "cooltime", "range", "target")
        for k in ACTION_SHAPED:
            if k in passive:
                err(f"passive.{k}: not valid on a passive — the engine reads only "
                    f"`passive_ref` and `params`; a passive is not an action")
        for k in passive:
            if k not in ("passive_ref", "params"):
                warn(f"passive.{k}: unknown key — the engine will ignore it")

    # --- category ---
    cat = d.get("category")
    if cat is None:
        err("category: missing")
    elif cat not in OBSERVED_CATEGORY:
        err(f"category: '{cat}' not in observed set {sorted(OBSERVED_CATEGORY)}")
    else:
        ok(f"category {cat}")

    # --- required top-level keys ---
    for k in ("id", "sprite", "stat", "growth", "attack", "skill", "skill2", "ult"):
        if k not in d:
            err(f"missing required key '{k}'")
    if d.get("anim_prefix") is None:
        warn("anim_prefix missing (roster uses an empty string)")

    # --- per-action enums ---
    for act in REQUIRED_RAW_ACTIONS:
        body = d.get(act)
        if not isinstance(body, dict):
            continue
        ct = body.get("casting_type")
        if ct is not None and ct not in OBSERVED_CASTING_TYPE:
            err(f"{act}.casting_type: '{ct}' invalid "
                f"{sorted(OBSERVED_CASTING_TYPE)}")
        cg = body.get("casting_target")
        if cg is not None and cg not in OBSERVED_CASTING_TARGET:
            err(f"{act}.casting_target: '{cg}' invalid "
                f"{sorted(OBSERVED_CASTING_TARGET)}")
        at = body.get("attack_type")
        if at is not None and at not in OBSERVED_ATTACK_TYPE:
            err(f"{act}.attack_type: '{at}' invalid {sorted(OBSERVED_ATTACK_TYPE)}")
        if "effect" in body:
            walk_effects(body["effect"], act, d)
        for req in ("action_name", "duration", "cooltime", "range"):
            if req not in body:
                err(f"{act}: missing '{req}'")

    # --- skill icons: exactly 3, engine path form ---
    icons = d.get("skill_icons")
    if not isinstance(icons, list) or len(icons) != 3:
        err("skill_icons: expected exactly 3 entries")
    else:
        for i in icons:
            if not i.startswith("asset/"):
                err(f"skill_icons: '{i}' is not an asset/ path")

    # --- view effect blocks: type + required fields ---
    for blk in ("view_effects", "view_buffs", "view_projectiles"):
        for i, e in enumerate(d.get(blk) or []):
            p = f"{blk}[{i}]"
            et = e.get("type")
            if et not in VIEW_BINDING_TYPES:
                err(f"{p}.type: '{et}' invalid {sorted(VIEW_BINDING_TYPES)}")
            if not e.get("name"):
                err(f"{p}: missing 'name'")
            # Each binding type uses a DIFFERENT set of required fields.
            # Verified against aldous (ThreePhase) and gusion (Sprite).
            if et in ("Animation", "Animated", "LoopAnimation"):
                if not e.get("anim"):
                    err(f"{p}: missing 'anim'")
                if not e.get("tag"):
                    err(f"{p}: missing 'tag'")
            elif et == "Sprite":
                if not e.get("sprite"):
                    err(f"{p}: Sprite binding needs 'sprite'")
            elif et == "ThreePhase":
                for req in ("pre_tag", "loop_tag", "remove_tag"):
                    if not e.get(req):
                        err(f"{p}: ThreePhase binding needs '{req}'")
            for key in ("anim", "sprite"):
                if key in e and not str(e[key]).startswith("asset/"):
                    err(f"{p}.{key}: not an asset/ path")
            if "z" in e and not isinstance(e["z"], int):
                err(f"{p}.z: must be an integer")

    # --- CasterViewEffect vs view_projectiles: the v1.1.77 defect ----------
    # A `view_projectiles` binding draws its animation AT THE PROJECTILE, wherever
    # it has flown to. A `CasterViewEffect` draws its animation AT THE CASTER.
    # If both resolve to the SAME anim asset, the engine draws that asset twice:
    # once out along the line, once standing on Arlott.
    #
    # Comparing NAMES is not enough — that was the trap. Arlott's caster effects
    # were `arlott_s1_wave` / `arlott_s1_far` while the projectile was
    # `arlott_s1_slam`, so the names never collided; only the underlying asset
    # did (`arlott_s1_wave#sheet.png`). Resolve through view_effects and compare
    # assets.
    eff_asset = {}
    for e in (d.get("view_effects") or []):
        if e.get("name") and e.get("anim"):
            eff_asset[e["name"]] = e["anim"]
    proj_assets = {}
    for e in (d.get("view_projectiles") or []):
        if e.get("anim"):
            proj_assets.setdefault(e["anim"], e.get("name"))

    if proj_assets:
        for act in ACTION_BLOCKS:
            body = d.get(act)
            if not isinstance(body, dict):
                continue
            seen = []
            walk_collect(body.get("effect"), "CasterViewEffect", seen)
            for nm in seen:
                asset = eff_asset.get(nm)
                if asset and asset in proj_assets:
                    err(f"{act}: CasterViewEffect '{nm}' and projectile "
                        f"'{proj_assets[asset]}' both draw {asset.split('/')[-1]} — "
                        f"that asset renders at the caster AND on the projectile "
                        f"(the Arlott S1 on-him bug)")
        # a LineRangeProjectile with no binding fires invisibly
        for act in ACTION_BLOCKS:
            body = d.get(act)
            if not isinstance(body, dict):
                continue
            refs = []
            walk_collect(body.get("effect"), "LineRangeProjectile", refs)
            for nm in refs:
                if nm not in {e.get("name") for e in (d.get("view_projectiles") or [])}:
                    err(f"{act}: LineRangeProjectile '{nm}' has no view_projectiles "
                        f"binding — the damage fires invisibly")

    ok(f"{name}: parsed")
    print(f"  {name}: {len(d.get('tags') or [])} tags, "
          f"{sum(len(d.get(b) or []) for b in ('view_effects','view_buffs','view_projectiles'))} "
          f"view bindings")


if __name__ == "__main__":
    paths = sys.argv[1:]
    if not paths:
        print("usage: validate_data_champion.py <file> [...]", file=sys.stderr)
        sys.exit(2)
    print("DATA_CHAMPION ENUM VALIDATION")
    print("=" * 70)
    for p in paths:
        check(p)
    for m in oks:
        print("PASS", m)
    for m in warns:
        print("WARN", m)
    for m in errors:
        print("FAIL", m)
    print("=" * 70)
    print(f"{len(oks)} pass, {len(warns)} warn, {len(errors)} fail")
    sys.exit(1 if errors else 0)
