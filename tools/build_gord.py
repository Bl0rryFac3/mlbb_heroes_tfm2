#!/usr/bin/env python3
"""Generate `champion/mlbb_gord.data_champion`.

Gord's kit is generated rather than hand-written for two reasons:

1. The ultimate is sixteen waves and the field is sixteen ticks. Writing those
   by hand is 32 copy-pasted blocks that silently disagree the moment a number
   moves.
2. Balance passes change constants, not structure. Edit the table at the top,
   rerun, and every wave moves together.

This follows the sibling project's convention (`tools/kit/build_<hero>.py` in
TFM2-League-Heroes) — the shipped kits there are all generated the same way.

Numbers come from GORD_SPEC.md §6, which re-expresses canon's share of a
2,380 HP pool against this roster's 650-820 HP.

Usage:
    python3 gord_work/build_gord.py                      # writes to build_v2081/champion
    python3 gord_work/build_gord.py --out /tmp/gord.json # elsewhere
    python3 gord_work/build_gord.py --check              # print the balance table
"""

import argparse
import json
import os

# --------------------------------------------------------------------------
# Identity
# --------------------------------------------------------------------------

CHAMP = "mlbb_gord"
MOD = "mlbb_heroes"
SPRITE = f"asset/{MOD}/sprite/gord"
ICON = lambda n: f"asset/{MOD}/icons/gord_{n}"
FX = lambda n: f"asset/{MOD}/effects/gord_{n}"

# Voice. These are the Native wrappers audio.rs registers from VOICE_CUES
# (`mlbb_heroes:audio_voice_gord_<slot>`), so the lines pass through the shared
# actor + global gates instead of firing on every trigger. Slots: skill1,
# skill2, ult — the Conqueror set ("Dark Void!", "Energy Vortex!", "Galaxy
# Beam!"), with the clips installed as sound/sfx/mlbb_gord_sfx_vo_<slot>.wav.
VOICE = lambda n: {"type": "Native",
                   "effect_ref": f"{MOD}:audio_voice_gord_{n}"}

# --------------------------------------------------------------------------
# Stats — GORD_SPEC.md §2. Fitted into the MLBB mage band (Gusion/Zhask/Estes).
# --------------------------------------------------------------------------

STAT = {
    "attack": 48, "magic_power": 64, "hp": 700, "defence": 22,
    "magic_resistance": 24, "move_speed": 1040, "hp_regen": 6,
    "stack": 0, "crit_chance": 0,
}
GROWTH = {
    "attack": 3, "magic_power": 8, "hp": 95, "defence": 3,
    "magic_resistance": 3, "move_speed": 4, "hp_regen": 0,
    "stack": 0, "crit_chance": 0,
}

# --------------------------------------------------------------------------
# Balance table — GORD_SPEC.md §6
#
# canon -> ours. Canon's ratios are per-tick against a 2,380 HP pool; this
# roster's is ~700, so the ratios compress ~3-5x while the *shape* (16 ticks)
# is kept, because the long field and the long beam are the hero's identity.
# --------------------------------------------------------------------------

S1_DAMAGE, S1_AP = 60, 28      # canon 425 + 65% MP
S2_DAMAGE, S2_AP = 3, 5        # canon 60 + 10% MP, per tick
ULT_DAMAGE, ULT_AP = 8, 8      # canon 210 + 32% MP, per tick

# Geometry (world units; ~1,000 units per sprite pixel)
ATTACK_RANGE = 62_000          # long-range poke, as Estes
S1_RANGE, S1_SPEED = 62_000, 7_000
S1_ORB_RADIUS = 12_000         # what stops the orb
S1_SPLASH_RADIUS = 28_000      # what it damages
S2_RADIUS = 35_000             # typical AoE is 25k-40k
ULT_WIDTH, ULT_LENGTH = 12_000, 120_000   # Vel'Koz R is 10k x 120k
ULT_RING_CHAMPION = 40_000     # re-aim: nearest champion first
ULT_RING_ANY = 90_000          # fallback: anything, so waves still clear

# Timing (60 ticks = 1 s)
S1_CD, S1_DUR, S1_START = 480, 24, 8      # canon 8.0 s
S2_CD, S2_DUR, S2_START = 480, 30, 6      # canon 8.0 s
S2_PERIOD, S2_TICKS, S2_LIFE = 15, 16, 240  # 0.25 s x 16 = 4 s
ULT_CD, ULT_DUR = 2_640, 24               # canon 44.0 s
ULT_CAST_TICK, ULT_LOOP_TICK = 16, 192    # wind-up, then the beam loop
ULT_PERIOD, ULT_WAVES, ULT_FIRST = 12, 16, 10  # 0.2 s x 16 = 3.2 s
STUN_TICKS = 60                            # canon 1 s

# --------------------------------------------------------------------------
# Node builders
# --------------------------------------------------------------------------


def ap_attack(damage: int, ratio: int):
    """Magic damage: `damage` + `ratio`% of Magic Power.

    The field is named `attack_ratio` but inside an ApAttack it means % AP.
    This is the single most misleading thing in the format.
    """
    return {
        "type": "ApAttack", "damage": damage, "attack_ratio": ratio,
        "hp_ratio": 0, "attack_effect_type": "Target",
    }


def mystic_hit():
    """One landed skill hit -> feeds the passive counter (gord.rs)."""
    return {"type": "Native", "effect_ref": f"{MOD}:gord_mystic_hit"}


def applied(*effects):
    return [{"effect": e, "casting_type": "Targeting"} for e in effects]


def splash():
    """The S1 burst: damage, one passive count, the stun, and the picture."""
    return applied(
        ap_attack(S1_DAMAGE, S1_AP),
        mystic_hit(),
        {"type": "Stun", "duration": STUN_TICKS},
        {"type": "ViewEffect", "name": "gord_orb_hit"},
    )


def beam_hits():
    """One ult wave's payload.

    No per-hit impact: `gord_gush_hit` was removed at the owner's call
    (2026-11-06). Its art was a copy of `gord_basic_hit`'s first four frames,
    so every ult tick was drawing the basic attack's impact. The passive
    markers (`gord_mystic_ready` over Gord, `gord_mystic_slow` on the victim)
    already tell the player the ult is landing, so the redundant impact went
    rather than being re-authored.
    """
    return applied(
        ap_attack(ULT_DAMAGE, ULT_AP),
        mystic_hit(),
    )


def gush_wave(i: int):
    """One ult wave, re-aimed at whatever is in front of Gord right now.

    Ring fallback is league_lucian R's idiom (champion-data.md:1949): try
    champions inside 40,000; if no champion was found, fall back to any enemy
    inside 90,000 so the beam still clears a wave.

    The beam must be fired *inside* the RandomTarget's effects — a
    LineRangeProjectile "points from the caster to it", and "it" is the point
    it was started at, i.e. the picked target (champion-data.md:997). Hoisting
    the beam out and remembering the target in a flag would lose the aim.
    """
    beam = {
        "type": "LineRangeProjectile",
        # One shared name for all sixteen waves, not one each. Every distinct
        # projectile name needs its own view_projectiles binding, so naming
        # them per-wave would mean sixteen identical art entries — and the
        # validator flags each as firing invisibly. league_missfortune R ships
        # the same way: one `r_wave` name across all six of its waves.
        "name": "gord_gush_wave",
        "width": ULT_WIDTH,
        "length": ULT_LENGTH,
        # `delay` is the LIFETIME, not a wind-up: per the project's own recorded
        # SDK probe (V15_TEST_BUILD.md) a RangeProjectile / LineRangeProjectile
        # "hits at apply - 1 ticks and is REMOVED at delay - 1 ticks". Shipped
        # as delay 1, which is a ZERO-tick life - the beam was created and
        # deleted inside one tick, so it could never be drawn. Damage still
        # landed (it applies at tick 0), which is exactly the report: passive
        # and damage work, nothing visible.
        #
        # 18 is the value every other visible beam in this roster ships
        # (arlott_s1_slam, franco_hook, obsidia_needle, obsidia_hunt_throw,
        # alpha_s1_wave). Franco had this identical bug - delay 6 against 0.54 s
        # of art - and the owner confirmed the fix in game. `apply` stays at 1
        # so the hit still resolves on tick 0 and damage timing does not move.
        #
        # 17-tick life against waves 12 ticks apart means ~1.4 beams on screen
        # at once, which reads as one continuous beam rather than 16 flashes.
        "delay": 18,
        "apply": 1,
        "applied_target": "EnemyWithoutTower",
        "applied_effects": beam_hits(),
    }
    lock = f"{MOD}_gord_gush_lock_{i}"
    return {
        "type": "Delayed",
        "tick": ULT_FIRST + i * ULT_PERIOD,
        "effects": [
            # Ring 1: a champion. Sets a 1-tick flag so ring 2 stands down.
            {
                "type": "RandomTarget",
                "range": ULT_RING_CHAMPION,
                "casting_target": "EnemyChampion",
                "from_projectile": False,
                "effects": [
                    {
                        "type": "AddCasterBuff",
                        "only_to_enemy": False,
                        "buff_state": {
                            "name": lock,
                            "duration": {"Time": {"tick": 1}},
                        },
                    },
                    beam,
                ],
            },
            # Ring 2: only if ring 1 found nobody.
            {
                "type": "SwitchByBuff",
                "buff_name": lock,
                "effect_buff": {"type": "Combine", "effects": []},
                "effect_none": {
                    "type": "RandomTarget",
                    "range": ULT_RING_ANY,
                    "casting_target": "EnemyWithoutTower",
                    "from_projectile": False,
                    "effects": [dict(beam)],
                },
            },
        ],
    }


def ult_effect():
    """The channel: wind-up, a looped animation, then sixteen waves.

    The waves sit side by side in one Combine, never nested in each other's
    end_effects. Teemo's traps nested that way and hit 119 levels; serde_json
    stops at 128 (champion-data.md:1558).
    """
    return {
        "type": "Combine",
        "effects": [
            VOICE("ult"),
            {"type": "CasterAnimation", "name": "ult", "tick": ULT_CAST_TICK},
            {
                "type": "Delayed",
                "tick": ULT_CAST_TICK,
                "effects": [
                    {"type": "CasterAnimation", "name": "ult_loop", "tick": ULT_LOOP_TICK}
                ],
            },
            {
                "type": "Combine",
                "effects": [gush_wave(i) for i in range(ULT_WAVES)],
            },
        ],
    }


def skill_effect():
    """S1: an orb that stops on the first enemy and bursts where it stopped.

    The stop-then-splash shape is the documented one (champion-data.md:1297,
    "Burst where a skillshot stops"): end_effects run at the stop point, so a
    RangeProjectile placed there is the splash.
    """
    return {
        "type": "Combine",
        "effects": [
            VOICE("skill1"),
            {"type": "CasterAnimation", "name": "skill", "tick": S1_START},
            {
                "type": "Delayed",
                "tick": S1_START,
                "effects": [
                    {
                        "type": "LinearProjectile",
                        "name": "gord_mystic_orb",
                        "speed": S1_SPEED,
                        "range": S1_RANGE,
                        "shape": {"Circle": {"radius": S1_ORB_RADIUS}},
                        "penetrate": False,
                        "applied_target": "EnemyWithoutTower",
                        "applied_effects": [],
                        "end_effects": [
                            {"type": "ViewEffect", "name": "gord_orb_burst"},
                            {
                                "type": "RangeProjectile",
                                "name": "gord_mystic_burst_zone",
                                "delay": 1,
                                "apply": 1,
                                "shape": {"Circle": {"radius": S1_SPLASH_RADIUS}},
                                "applied_target": "EnemyWithoutTower",
                                "applied_effects": splash(),
                            },
                        ],
                    }
                ],
            },
        ],
    }


def skill2_effect():
    """S2: the energy field. Estes' domain, with Gord's numbers.

    A `Position` cast keeps its cast point for every effect it runs, Delayed
    ones included (champion-data.md:1560) — that is what pins the field where
    Gord put it instead of having it follow him.
    """
    return {
        "type": "Combine",
        "effects": [
            VOICE("skill2"),
            {"type": "CasterAnimation", "name": "skill2", "tick": S2_START},
            {
                "type": "Delayed",
                "tick": S2_START,
                "effects": [
                    {
                        "type": "RangePeriodProjectile",
                        "name": "gord_injunction_field",
                        "tick": S2_LIFE,
                        "period": S2_PERIOD,
                        "first_delay": 0,
                        "shape": {"Circle": {"radius": S2_RADIUS}},
                        "applied_target": "EnemyWithoutTower",
                        # No per-tick impact: `gord_injunction_hit` was removed
                        # at the owner's call (2026-11-06). Its art was a copy
                        # of `gord_basic_hit`'s first four frames, so every S2
                        # tick drew the basic attack's impact. The ground field
                        # plus the passive markers carry the feedback instead.
                        "applied_effects": applied(
                            ap_attack(S2_DAMAGE, S2_AP),
                            mystic_hit(),
                        ),
                        "end_effects": [],
                    }
                ],
            },
        ],
    }


def build() -> dict:
    return {
        "id": CHAMP,
        "category": "Magician",
        # AP mage, ranged, the roster's only mage with hard CC.
        "tags": ["AP", "Magic", "Range", "CC"],
        "sprite": SPRITE,
        "anim_prefix": "",
        "skill_icons": [ICON("skill"), ICON("skill2"), ICON("ult")],

        "stat": dict(STAT),
        "growth": dict(GROWTH),

        # Basic attack. Routed through native code (Karrie/Hanabi/Obsidia/Zhask
        # all do the same) because a bare ViewEffect has no travel: the spark
        # appeared on the target the instant the swing started.
        #
        # The native spawns `gord_basic_bolt` and pays the damage on arrival via
        # `mlbb_heroes:gord_basic_hit`. Damage is 100% AD, physical — the same
        # number the roster's plain `Attack` node produces. The previous version
        # carried no damage node at all, so the attack hit for nothing.
        #
        # Mystic Favor is deliberately NOT charged here: canon counts skill
        # hits, and the field plus the beam already land 32 ticks a rotation.
        "attack": {
            "action_name": "attack",
            "description": f"#asset/base/text/champion?description.{CHAMP}.attack",
            "duration": 20,
            "cooltime": 60,
            "start_timing": 6,
            "cancelable": False,
            "range": ATTACK_RANGE,
            "casting_type": "Targeting",
            "casting_target": "Enemy",
            "attack_type": "BaseAttack",
            "effect": {"type": "Native", "effect_ref": f"{MOD}:gord_basic_attack"},
        },

        "skill": {
            "action_name": "skill",
            "description": f"#asset/base/text/champion?description.{CHAMP}.skill",
            "duration": S1_DUR,
            "cooltime": S1_CD,
            "start_timing": S1_START,
            "cancelable": False,
            "range": S1_RANGE,
            "casting_type": "Targeting",
            # EnemyWithoutTower so the AI fires it at waves and camps too; on
            # EnemyChampion a hero clears with auto-attacks only (SKILL.md).
            "casting_target": "EnemyWithoutTower",
            "attack_type": "Skill",
            "can_use_with_move": False,
            "effect": skill_effect(),
        },

        "skill2": {
            "action_name": "skill2",
            "description": f"#asset/base/text/champion?description.{CHAMP}.skill2",
            "duration": S2_DUR,
            "cooltime": S2_CD,
            "start_timing": S2_START,
            "cancelable": False,
            "range": ATTACK_RANGE,
            # Position, not Targeting: the field goes where he aims, not on a unit.
            "casting_type": "Position",
            "casting_target": "EnemyWithoutTower",
            "attack_type": "Skill",
            "can_use_with_move": True,
            "effect": skill2_effect(),
        },

        "ult": {
            "action_name": "ult",
            "description": f"#asset/base/text/champion?description.{CHAMP}.ult",
            "duration": ULT_DUR,
            "cooltime": ULT_CD,
            "start_timing": 1,
            "cancelable": False,
            "range": ULT_LENGTH,
            "casting_type": "Targeting",
            "casting_target": "EnemyChampion",
            "attack_type": "Skill",
            "can_use_with_move": False,
            "effect": ult_effect(),
        },

        # ---- view bindings -------------------------------------------------
        # Names must match what the effect trees above and gord.rs ask for:
        # a projectile / effect / buff name with no view entry draws nothing,
        # and typos there compile fine and fail silently.
        "view_projectiles": [
            {"type": "Animated", "name": "gord_mystic_orb",
             "anim": FX("orb"), "tag": "orb", "z": 1, "repeat": True},
            {"type": "Animated", "name": "gord_injunction_field",
             "anim": FX("injunction_field"), "tag": "field", "z": -1, "repeat": True},
            # Must match the projectile name in gush_wave() exactly.
            {"type": "Animated", "name": "gord_gush_wave",
             "anim": FX("gush_wave"), "tag": "wave", "z": 1, "repeat": True},
            # The basic attack's travelling bolt. Must match BASIC_PROJECTILE
            # in gord.rs. `repeat` is true so the bolt keeps pulsing for the
            # ~10 ticks it is in flight, as Karrie's lightwheel does; Obsidia's
            # shard sets it false only because that art is a thin streak.
            {"type": "Animated", "name": "gord_basic_bolt",
             "anim": FX("basic_bolt"), "tag": "bolt", "z": 1, "repeat": True},
        ],
        "view_effects": [
            {"type": "Animation", "name": "gord_orb_burst",
             "anim": FX("orb_burst"), "tag": "burst", "z": 2, "is_follow": False},
            {"type": "Animation", "name": "gord_orb_hit",
             "anim": FX("orb_hit"), "tag": "hit", "z": 2, "is_follow": False},
            # `gord_injunction_hit` and `gord_gush_hit` are deliberately ABSENT.
            # Both were removed 2026-11-06 at the owner's call: their art was a
            # copy of gord_basic_hit's first four frames, so S2 and the ult were
            # drawing the basic attack's impact on every tick. The passive
            # markers carry the feedback instead. Do not re-add these without
            # art of their own - re-pointing them at the basic hit reproduces
            # exactly the bug that was just removed.
            {"type": "Animation", "name": "gord_basic_hit",
             "anim": FX("basic_hit"), "tag": "hit", "z": 2, "is_follow": False},
            # Played by the native in gord.rs (play_view_effect), so it must be
            # a view_effects entry, never view_projectiles.
            {"type": "Animation", "name": "gord_mystic_burst",
             "anim": FX("mystic_burst"), "tag": "passive_proc", "z": 2, "is_follow": False},
        ],
        "view_buffs": [
            # z 3, not 2: every over-the-head marker in the roster draws at 3
            # (karrie_passive_marks_head_smaller, uranus_skill1_marks,
            # gusion_passive_rune, arlott_mark, alpha_lockon). At 2 the armed
            # indicator sat under the unit and never read in game.
            {"type": "Animated", "name": "mlbb_gord_mystic_ready",
             "anim": FX("mystic_ready"), "tag": "ready", "z": 3},
            {"type": "Animated", "name": "mlbb_gord_mystic_slow",
             "anim": FX("mystic_slow"), "tag": "slow", "z": 1},
        ],

        "passive": {"passive_ref": f"{MOD}:gord_mystic_favor", "params": {}},
        "stack_skill_index": 0,
    }


# --------------------------------------------------------------------------
# Balance report
# --------------------------------------------------------------------------

def pct(value: int, percent: int) -> int:
    """The engine's integer percentage — obsidia.rs's rounding convention.

    The engine evaluates each tick as an integer, so the rounding happens
    sixteen times over a field or a beam and the error compounds. Working out
    a per-tick figure as a float and multiplying by 16 (as GORD_SPEC.md first
    did) overstates L1 by ~3 and understates L15 by ~3.
    """
    return (value * percent + 50) // 100


def report():
    """Print the rotation table from GORD_SPEC.md §6 for a sanity check."""
    print(f"Gord — AP {STAT['magic_power']} +{GROWTH['magic_power']}/level\n")
    print(f"{'':10s}{'L1':>8s}{'L8':>8s}{'L15':>8s}")
    rows = []
    for level in (1, 8, 15):
        ap = STAT["magic_power"] + GROWTH["magic_power"] * (level - 1)
        s1 = S1_DAMAGE + pct(ap, S1_AP)
        s2 = S2_TICKS * (S2_DAMAGE + pct(ap, S2_AP))
        ult = ULT_WAVES * (ULT_DAMAGE + pct(ap, ULT_AP))
        # The passive needs 5 skill hits per proc (4 arm, the 5th pays).
        procs = (1 + S2_TICKS + ULT_WAVES) // 5
        proc = 12 + 2 * level + pct(ap, 8)
        rows.append((s1, s2, ult, procs * proc, ap))

    for label, idx in (("S1", 0), ("S2", 1), ("ult", 2), ("passive", 3)):
        print(f"{label:10s}" + "".join(f"{r[idx]:>8d}" for r in rows))
    print(f"{'TOTAL':10s}" + "".join(
        f"{sum(r[i] for i in range(4)):>8d}" for r in rows))
    print(f"\nAP:      " + "".join(f"{r[4]:>8d}" for r in rows))
    print(f"procs:   " + "".join(
        f"{(1 + S2_TICKS + ULT_WAVES) // 5:>8d}" for _ in rows))
    print("\nEngine-exact (integer per tick). Alpha v17: 810 / 1016 / 1229.")


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    default = os.path.normpath(
        os.path.join(here, "..", "build_v2081", "champion", f"{CHAMP}.data_champion"))

    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=default)
    ap.add_argument("--check", action="store_true", help="print the balance table and exit")
    a = ap.parse_args()

    if a.check:
        report()
        return

    data = build()
    with open(a.out, "w", encoding="utf-8") as fh:
        json.dump(data, fh, indent=1)
        fh.write("\n")
    print(f"wrote {a.out} ({os.path.getsize(a.out):,} bytes)")
    report()


if __name__ == "__main__":
    main()
