#!/usr/bin/env python3
"""Build the full source+SDK archive for a release.

Layout mirrors the v2.0.6 archive: the whole working tree, flat (paths
relative to the build root), with Cargo build output (``target/``) excluded
and a copy of the built DLL at the root for reference.

Usage:
    python3 tools/build_source_archive.py [--output PATH] [--release v2.0.8-rc.1] [--force]
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Shipped at the archive root. README.md is deliberately absent: it is a
# byte-for-byte duplicate of RELEASE_README.md, and the runtime ZIP installs
# that one as mlbb_heroes/README.md.
TOP_FILES = (
    "RELEASE_README.md",
    "JULIAN_AUDIO_TODO.md",
    "SOURCE_ARCHIVE_README.txt",
    "V1.1.78_CHANGES.md",
    "mlbb_heroes.dll",
    "mod.mod_info",
    "mod.override_info",
    "thumbnail.png",
    "Install_Guide.html",
)

SHIPPED_DIRS = (
    "champion", "champions", "docs", "effects", "icons", "override",
    "patches", "reports", "sdk", "sfx", "sound", "source", "sprite",
    "src", "style", "text", "tools", "view",
)

EXCLUDED_DIR_NAMES = {"target", "__pycache__", ".git", ".venv"}
EXCLUDED_SUFFIXES = {".pyc"}
# 1.1.128: `mlbb_heroes.log` is the MOD'S RUNTIME OUTPUT - `wlog()` writes it into the crate
# directory, so any `cargo build`/`cargo test` run inside the tree drops a residue file there that
# then ships inside the "full source" archive (1.1.127 did exactly that, and a stale log sitting
# next to an installed mod is indistinguishable from a live one). `tools/verify_effects.py` proves
# the name is already reserved by the toolchain, so this is a packaging exclusion, not a deletion:
# nothing under `reports/` is touched. Only a log AT THE RUNTIME PATH is skipped.
EXCLUDED_NAMES = {"mlbb_heroes.log"}

EXPECTED_CHAMPIONS = 27  # Yu Zhong integration candidate


def collect() -> list[Path]:
    chosen: list[Path] = []
    for rel in TOP_FILES:
        p = ROOT / rel
        if not p.is_file():
            raise SystemExit(f"Required file missing: {p}")
        chosen.append(p)
    for d in SHIPPED_DIRS:
        base = ROOT / d
        if not base.is_dir():
            raise SystemExit(f"Required directory missing: {base}")
        for p in base.rglob("*"):
            if not p.is_file():
                continue
            if EXCLUDED_DIR_NAMES & set(p.relative_to(ROOT).parts):
                continue
            if p.suffix in EXCLUDED_SUFFIXES:
                continue
            if p.name in EXCLUDED_NAMES and "reports" not in p.relative_to(ROOT).parts:
                continue
            chosen.append(p)
    return sorted(set(chosen), key=lambda p: p.relative_to(ROOT).as_posix())


def main() -> None:
    from validate_override_info import validate as validate_override_info
    validate_override_info(json.loads((ROOT / "mod.override_info").read_text()))
    info = json.loads((ROOT / "mod.mod_info").read_text())
    build = info["version"]

    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--output", type=Path)
    ap.add_argument("--release", default="v2.3.0-rc.1")
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()

    files = collect()
    rel = {p.relative_to(ROOT).as_posix() for p in files}

    # The crate is the real source of truth; ROOT/src is a vestigial directory
    # (it holds one stale copy of arlott.rs). Assert on sdk/candidate.
    champs = {p for p in rel if p.startswith("champion/") and p.endswith(".data_champion")}
    if len(champs) != EXPECTED_CHAMPIONS:
        raise SystemExit(f"expected {EXPECTED_CHAMPIONS} champions, found {len(champs)}")
    for required in ("sdk/candidate/lib.rs", "sdk/candidate/belerick.rs",
                     "sdk/mod-api-stable/Cargo.toml", "tools/build_release_zip.py"):
        if required not in rel:
            raise SystemExit(f"archive would be missing {required}")

    out = (args.output
           or ROOT.parent / f"MLBB_Heroes_TFM2_v{build}_v{EXPECTED_CHAMPIONS}_full_source_with_SDK_{args.release}.zip"
           ).resolve()
    if out.exists() and not args.force:
        raise SystemExit(f"Refusing to overwrite without --force: {out}")
    out.parent.mkdir(parents=True, exist_ok=True)
    with __import__("zipfile").ZipFile(out, "w", compression=__import__("zipfile").ZIP_DEFLATED,
                                       compresslevel=9) as z:
        for p in files:
            z.write(p, p.relative_to(ROOT).as_posix())

    print(f"wrote {out}")
    print(f"  {out.stat().st_size} bytes, {len(files)} files, {len(champs)} champions")


if __name__ == "__main__":
    main()
