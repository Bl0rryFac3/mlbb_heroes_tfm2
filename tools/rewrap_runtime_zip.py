#!/usr/bin/env python3
"""Rewrap a flat runtime ZIP into the shape §7c-ii requires.

`build_release_zip.py` emits entries as `mlbb_heroes/…` with `Install_Guide.html`
beside them and **no outer folder**. §7c-ii forbids shipping that:

    mlbb_heroes_v2.0.7-rc.1/
    ├── mlbb_heroes/          the mod
    └── Install_Guide.html    beside it, not inside

A flattened zip also makes the guide's step 3 wrong ("open the new
`mlbb_heroes_v2.0.7-rc.1` folder. Inside it, locate the folder named
`mlbb_heroes`"), and flattening puts the mod straight under `mods/`.

Usage:
    python3 rewrap_runtime_zip.py <flat.zip> [--root NAME] [--output PATH] [--force]

`--root` defaults to the input's stem (so `mlbb_heroes_v2.0.7-rc.1.zip` wraps
under `mlbb_heroes_v2.0.7-rc.1/`), which matches the rc-zip convention.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import zipfile
from pathlib import Path

EXPECTED_CHAMPIONS = 25


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("zippath", type=Path)
    ap.add_argument("--root")
    ap.add_argument("--output", type=Path)
    ap.add_argument("--guide", type=Path,
                    help="replacement Install_Guide.html. Use this when the guide was "
                         "re-versioned after the zip was built, so the copy inside the "
                         "archive does not contradict the folder it ships in.")
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()

    src: Path = args.zippath
    if not src.is_file():
        sys.exit(f"No such file: {src}")
    root = args.root or src.stem
    out = args.output or src.with_name(f"{src.stem}.rewrapped.zip")
    if out.exists() and not args.force:
        sys.exit(f"Refusing to overwrite without --force: {out}")

    with zipfile.ZipFile(src) as zin:
        names = zin.namelist()
        tops = {n.split("/")[0] for n in names}
        if tops != {"mlbb_heroes", "Install_Guide.html"}:
            sys.exit(f"Input is not a flat runtime zip; top-level entries: {sorted(tops)}")
        champs = [n for n in names if n.startswith("mlbb_heroes/champion/")
                  and n.endswith(".data_champion")]
        if len(champs) != EXPECTED_CHAMPIONS:
            sys.exit(f"Expected {EXPECTED_CHAMPIONS} champions, found {len(champs)}")

        # The guide must have been re-versioned to match this root, or step 3 lies.
        guide = (args.guide.read_text(encoding="utf-8") if args.guide
                 else zin.read("Install_Guide.html").decode("utf-8", "ignore"))
        versions = set(re.findall(r"v2\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.]+)?", guide))
        stem_ver = re.match(r"mlbb_heroes_(v2\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.]+)?)", src.stem)
        if stem_ver and stem_ver.group(1) not in versions:
            sys.exit(f"Guide inside the zip does not mention {stem_ver.group(1)}; "
                     f"it mentions {sorted(versions)}. Step 3 would be wrong.")

        with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED,
                             compresslevel=9) as zout:
            for n in sorted(names):
                payload = (args.guide.read_bytes() if args.guide and n == "Install_Guide.html"
                           else zin.read(n))
                zout.writestr(f"{root}/{n}", payload)

    data = out.read_bytes()
    print(f"wrote {out}")
    print(f"  {len(data)} bytes, {len(names)} entries, root '{root}/'")
    print(f"  sha256 {hashlib.sha256(data).hexdigest()}")
    print("\n  audit written to", out.with_suffix(".audit.json"))
    out.with_suffix(".audit.json").write_text(json.dumps({
        "source_zip": str(src),
        "archive": str(out),
        "root": root,
        "entries": len(names),
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "champions": len(champs),
        "top_level": sorted({f"{root}/{t}" for t in tops}),
        "runtime_tested": False,
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
