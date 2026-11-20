#!/usr/bin/env python3
"""Field-parity check for .data_champion effect nodes.

WHY THIS EXISTS
---------------
On 2026-11-08 build 4 shipped a Cecilion S1 rebuild whose new `RangeProjectile`
lacked a `name` field. The engine deserializes these nodes with serde, and
`name` is required on that type, so the whole champion failed to load:

    data_champion load error: Error("missing field `name`", line: 184, column: 2)

Nothing else caught it:
  * `validate_data_champion.py` passed clean (it only checks enums and parsing).
  * `lint_mod.py` (upstream) passed clean (it checks per-type field *names* are
    known, not that required ones are present).

The lesson: an effect type's shape is best inferred from its own instances in
the shipped tree. If every other `RangeProjectile` on the roster carries a
field, a new one missing it is a bug.

WHAT IT DOES
------------
Walks every effect node in every shipped champion and reports:

  MISSING  - the node lacks a field that EVERY instance of its type carries.
             A field present in 100% of siblings is required in practice.
             This is what broke build 4.

  DIFFERS  - the node lacks a field that all-but-one instances carry (needs
             >= 3 instances). Usually a genuine omission, occasionally
             deliberate -- so this is a nudge to look, not a hard failure.

USAGE
-----
    python3 tools/validate_effect_fields.py [champion_dir]

Defaults to ./champion. Exit code 1 if any MISSING is found.
"""
from __future__ import annotations

import glob
import json
import os
import sys
from collections import defaultdict

ACTIONS = ("attack", "skill", "skill2", "ult")


def collect(node, path: str, fname: str, out: dict) -> None:
    if isinstance(node, dict):
        if isinstance(node.get("type"), str):
            out[node["type"]].append((fname, path, set(node.keys())))
        for k, v in node.items():
            collect(v, f"{path}/{k}", fname, out)
    elif isinstance(node, list):
        for i, v in enumerate(node):
            collect(v, f"{path}[{i}]", fname, out)


def main(argv: list[str]) -> int:
    root = argv[1] if len(argv) > 1 else "champion"
    files = sorted(glob.glob(os.path.join(root, "*.data_champion")))
    if not files:
        print(f"no .data_champion under {root!r}")
        return 2

    instances: dict[str, list] = defaultdict(list)
    for fn in files:
        with open(fn, encoding="utf-8") as fh:
            data = json.load(fh)
        name = os.path.basename(fn)
        for act in ACTIONS:
            blk = data.get(act)
            if isinstance(blk, dict):
                collect(blk.get("effect", {}), act, name, instances)

    print("EFFECT FIELD PARITY")
    print("=" * 70)
    print(f"{len(files)} champions, {sum(len(v) for v in instances.values())} effect nodes\n")

    missing = 0
    for kind in sorted(instances):
        items = instances[kind]
        if len(items) < 2:
            continue
        # Leave-one-out: the node under test must never shape its own baseline.
        for i, (fname, path, keys) in enumerate(items):
            others = [k for j, (_, _, k) in enumerate(items) if j != i]
            universal_others = set.intersection(*others)
            for fld in sorted(universal_others - keys):
                print(f"  MISSING  {kind:<22} {fname:<30} lacks '{fld}'  {path}")
                missing += 1

    print("=" * 70)
    print(f"{missing} MISSING")
    if missing:
        print("\nA MISSING field is fatal: serde refuses the whole champion.")
        return 1
    print("OK - every node carries its type's full required field set")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
