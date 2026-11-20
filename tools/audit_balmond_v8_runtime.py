#!/usr/bin/env python3
"""Audit V8 runtime packaging against the preserved V7 runtime ZIP."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
import zipfile
from pathlib import Path

EXPECTED_CHANGES = {
    "mlbb_heroes/README.md",
    "mlbb_heroes/champion/mlbb_balmond.data_champion",
    "mlbb_heroes/mod.mod_info",
    "mlbb_heroes/mlbb_heroes.dll",
}
FORBIDDEN = ("mod-api-stable/", "candidate/src/", ".rs", "Cargo.toml", "Cargo.lock", "target/")

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def filemap(path: Path) -> dict[str, bytes]:
    with zipfile.ZipFile(path) as zf:
        bad = zf.testzip()
        if bad:
            raise ValueError(f"corrupt ZIP member: {bad}")
        return {i.filename: zf.read(i) for i in zf.infolist() if not i.is_dir()}

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--v7", type=Path, required=True)
    ap.add_argument("--v8", type=Path, required=True)
    ap.add_argument("--report", type=Path)
    args = ap.parse_args()
    old, new = filemap(args.v7), filemap(args.v8)
    assert set(old) == set(new), "runtime path set differs from V7"
    changed = {p for p in old if old[p] != new[p]}
    assert changed == EXPECTED_CHANGES, f"unexpected runtime file changes: {sorted(changed ^ EXPECTED_CHANGES)}"
    assert len(new) == 490, f"expected 490 runtime files, found {len(new)}"
    assert all(not any(token in p for token in FORBIDDEN) for p in new), "source/SDK/build file leaked into runtime"
    info = json.loads(new["mlbb_heroes/mod.mod_info"])
    assert info["version"] == "1.1.63"
    assert "v8" in info["description"].lower()
    champion = new["mlbb_heroes/champion/mlbb_balmond.data_champion"].decode()
    assert '"delay": 30' in champion and '"apply": 1' in champion
    assert "entity_knockback(caster, target, 1_000, 3)" in Path(__file__).parents[1].joinpath("src/balmond.rs").read_text()
    dll = new["mlbb_heroes/mlbb_heroes.dll"]
    assert dll[:2] == b"MZ", "Windows DLL missing PE signature"
    result = {
        "baseline": args.v7.name,
        "archive": args.v8.name,
        "mod_version": info["version"],
        "target": "TFM2 0.7.0 (in-game unverified)",
        "platform": "Windows x86-64",
        "runtime_files": len(new),
        "runtime_paths_identical_to_v7": True,
        "changed_runtime_files": sorted(changed),
        "source_or_sdk_in_runtime": False,
        "windows_pe_signature": "MZ",
        "sha256": sha256(args.v8.read_bytes()),
        "bytes": args.v8.stat().st_size,
    }
    text = json.dumps(result, indent=2) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(text)
    print(text, end="")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
