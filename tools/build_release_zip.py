#!/usr/bin/env python3
"""Create an audited, runtime-only GitHub release ZIP without deleting source files."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import tomllib
import zipfile
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = json.loads((ROOT / 'mod.mod_info').read_text())['version']
# These are prior failed/unused Estes tether assets and known raw staging images.
EXCLUDED_EFFECTS = {
    "IMPORT_REPORT.md",
    "estes_link#anim.fanim",
    "estes_link#sheet.png",
    "estes_link_repeat.png",
    "estes_link_pulse#anim.fanim",
    "estes_link_pulse#sheet.png",
    "fanny_cable_wire.png",
    "fanny_hook.png",
}
# The eight old SVG variants belonged only to the removed post-render implementation.
EXCLUDED_PREFIXES = ("estes_tether_",)
RUNTIME_DIRS = (
    "champion",
    "champions",
    "effects",
    "icons",
    "override",
    "sound",
    "sprite",
    "style",
    "text",
    "view",
)
ROOT_FILES = ("mod.mod_info", "mod.override_info", "thumbnail.png")
# The crate lives here, not at ROOT. The old ROOT/"Cargo.toml" and ROOT/"src"
# paths were left over from a flat layout that no longer exists — the shipped
# v2.0.6 tree has no top-level Cargo.toml, and ROOT/src/ holds a stale,
# divergent copy of arlott.rs only. Anything reading those was checking nothing.
SDK_CRATE = Path("sdk/candidate")


def rel_files(library_file: Path) -> list[Path]:
    chosen: list[Path] = []
    for rel in ROOT_FILES:
        path = ROOT / rel
        if not path.is_file():
            raise SystemExit(f"Required release file missing: {path}")
        chosen.append(path)
    readme = ROOT / "RELEASE_README.md"
    if not readme.is_file():
        raise SystemExit("Missing RELEASE_README.md")
    chosen.append(readme)
    # Keep the guide inside the single top-level mod folder.
    guide = ROOT / "Install_Guide.html"
    if not guide.is_file():
        raise SystemExit("Missing Install_Guide.html")
    chosen.append(guide)
    if not library_file.is_file():
        raise SystemExit(f"Required native library missing: {library_file}")
    chosen.append(library_file)

    for dirname in RUNTIME_DIRS:
        base = ROOT / dirname
        if not base.is_dir():
            raise SystemExit(f"Required runtime directory missing: {base}")
        for path in sorted(base.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(base)
            if dirname == "effects":
                if rel.as_posix() in EXCLUDED_EFFECTS or rel.name.startswith(EXCLUDED_PREFIXES):
                    continue
            if dirname == "sprite":
                # Include only the game's atlases/animation metadata, not import notes or raw frames.
                if len(rel.parts) != 1 or not (rel.name.endswith("#sheet.png") or rel.name.endswith("#anim.fanim")):
                    continue
            if dirname == "sound" and not rel.parts[0] == "sfx":
                continue
            chosen.append(path)
    return sorted(set(chosen), key=lambda p: p.relative_to(ROOT).as_posix())


def validate_assets(files: list[Path], library_file: Path, library_name: str, platform: str) -> None:
    included = {p.relative_to(ROOT).as_posix() for p in files if p != library_file}
    included.add(library_name)
    info = json.loads((ROOT / "mod.mod_info").read_text())
    cargo = tomllib.loads((ROOT / SDK_CRATE / "Cargo.toml").read_text())
    if info.get("version") != cargo.get("package", {}).get("version"):
        raise SystemExit("mod.mod_info and Cargo.toml versions do not match")

    fanny = json.loads((ROOT / "champion/fanny.data_champion").read_text())
    fanny_override = json.loads((ROOT / "override/fanny_native.data_champion").read_text())
    for field in ("stat", "growth"):
        if fanny[field] != fanny_override[field]:
            raise SystemExit(f"Fanny's roster and override {field} values do not match")

    # Ensure all Estes's Rust view-effect names resolve to champion bindings and the 12 frames.
    rust = (ROOT / SDK_CRATE / "estes.rs").read_text()
    data = json.loads((ROOT / "champion/mlbb_estes.data_champion").read_text())
    anim = json.loads((ROOT / "effects/estes_link_wire#anim.fanim").read_text())
    for i in range(12):
        name = f"mlbb_heroes_estes_link_wire_{i}"
        if not any(v.get("name") == name and v.get("tag") == f"wire_{i}" for v in data["view_effects"]):
            raise SystemExit(f"Missing Estes champion binding: {name}")
        if f"wire_{i}" not in anim["anims"] or name not in rust:
            raise SystemExit(f"Missing Estes tether animation/code binding: {name}")
    assert "estes_link_pulse" not in json.dumps(data)
    assert "TargetProjectile" not in json.dumps(data["skill"]["effect"])

    # Confirm stale graphics are unreferenced by runtime data or native source before omitting them.
    runtime_text = "\n".join(
        p.read_text(errors="ignore")
        for p in [*ROOT.glob("champion/*.data_champion"), *ROOT.glob("override/*.data_champion"),
                  *(ROOT / SDK_CRATE).glob("*.rs"), ROOT / "mod.override_info"]
    )
    stale = ["estes_link_pulse", "estes_tether_", "fanny_cable_wire.png", "fanny_hook.png"]
    for token in stale:
        if token in runtime_text:
            raise SystemExit(f"Refusing to omit stale candidate still referenced by runtime files: {token}")

    # Resolve each game asset referenced by champion and override data before release.
    refs = set()
    for path in [*ROOT.glob("champion/*.data_champion"), *ROOT.glob("override/*.data_champion")]:
        refs.update(re.findall(r"asset/mlbb_heroes/[^\"?]+", path.read_text()))
    for ref in sorted(refs):
        stem = ref.removeprefix("asset/mlbb_heroes/")
        if stem.startswith(("effects/", "sprite/", "champions/")):
            required = [f"{stem}#sheet.png", f"{stem}#anim.fanim"]
        elif stem.startswith("icons/"):
            required = [f"{stem}.png"]
        elif stem.startswith("text/"):
            required = ["text/champion.i18n"]
        elif stem.startswith("style/"):
            required = ["style/champion_view.champion_view"]
        else:
            continue
        for path in required:
            if path not in included:
                raise SystemExit(f"Referenced runtime asset would be omitted: {ref} -> {path}")

    for req in (
        "effects/estes_link_wire#sheet.png",
        "effects/estes_link_wire#anim.fanim",
        "sprite/estes#sheet.png",
        "champion/mlbb_estes.data_champion",
        library_name,
    ):
        if req not in included:
            raise SystemExit(f"Required package path missing: {req}")
    # Validate every bundled sound event and clip independently. TFM2 can fail
    # to play same-ID event/clip routes, so reject collisions and unresolved WAVs.
    overrides = json.loads((ROOT / "mod.override_info").read_text())
    from validate_override_info import validate as validate_override_info
    validate_override_info(overrides)

    # Native summons resolve an atlas by unit name in the base namespace.
    # Data-driven champion sprites already use an explicit asset path.
    for animation in (ROOT / "champions").glob("*#anim.fanim"):
        stem = "champions/" + animation.name.removesuffix("#anim.fanim")
        if f"asset/mlbb_heroes/{stem}" in refs:
            continue
        for suffix, extension in (("anim", "fanim"), ("sheet", "png")):
            base_key = f"asset/base/aseprite_resources/{stem}#{suffix}"
            mod_key = f"asset/mlbb_heroes/{stem}#{suffix}"
            if overrides.get(base_key) != {"type": "override", "remapping": mod_key}:
                raise SystemExit(f"Missing summoned unit atlas override: {base_key} -> {mod_key}")
            if f"{stem}#{suffix}.{extension}" not in included:
                raise SystemExit(f"Summoned unit atlas would be omitted: {mod_key}")

    sound_dir = ROOT / "sound" / "sfx"
    for sound_info in sorted(sound_dir.glob("*.sound_info")):
        event = sound_info.stem
        event_path = f"asset/base/sound/sfx/{event}"
        if overrides.get(event_path, {}).get("remapping") != f"asset/mlbb_heroes/sound/sfx/{event}":
            raise SystemExit(f"Missing sound-event override: {event}")
        sound_data = json.loads(sound_info.read_text())
        if not sound_data.get("plays"):
            raise SystemExit(f"Sound event has no clips: {sound_info.name}")
        for play in sound_data["plays"]:
            clip = play.get("clip", "")
            if not clip or clip == event:
                raise SystemExit(f"Invalid or same-ID sound clip in {sound_info.name}: {clip!r}")
            if f"sound/sfx/{clip}.wav" not in included:
                raise SystemExit(f"Sound clip WAV is missing from package: {clip}")
            clip_path = f"asset/base/sound/sfx/{clip}"
            if overrides.get(clip_path, {}).get("remapping") != f"asset/mlbb_heroes/sound/sfx/{clip}":
                raise SystemExit(f"Missing sound-clip override: {clip}")

    # Every custom Sfx action in champion data must resolve to a bundled event.
    for champion_path in ROOT.glob("champion/*.data_champion"):
        champion = json.loads(champion_path.read_text())
        def check_sfx(value):
            if isinstance(value, dict):
                if value.get("type") == "Sfx":
                    name = value.get("name", "")
                    if name.startswith("mlbb_") and not (sound_dir / f"{name}.sound_info").is_file():
                        raise SystemExit(f"Unresolved Sfx event in {champion_path.name}: {name}")
                for child in value.values():
                    check_sfx(child)
            elif isinstance(value, list):
                for child in value:
                    check_sfx(child)
        check_sfx(champion)

    library_bytes = library_file.read_bytes()
    if platform == "windows" and not library_bytes.startswith(b"MZ"):
        raise SystemExit("Windows native library does not have a PE signature")
    if platform == "linux" and not library_bytes.startswith(b"\x7fELF"):
        raise SystemExit("Linux native library does not have an ELF signature")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", nargs="?", type=Path)
    parser.add_argument("--platform", choices=("windows", "linux"), default="windows")
    parser.add_argument("--library", type=Path, help="native library file to package")
    parser.add_argument("--force", action="store_true", help="replace an existing output ZIP")
    args = parser.parse_args()
    library_name = "mlbb_heroes.dll" if args.platform == "windows" else "mlbb_heroes.so"
    default_library = (
        ROOT / SDK_CRATE / "target" / "x86_64-pc-windows-gnu" / "release" / "mlbb_heroes.dll"
        if args.platform == "windows"
        else ROOT / SDK_CRATE / "target" / "release" / "libmlbb_heroes.so"
    )
    library_file = (args.library or default_library).resolve()
    out = (args.output or ROOT.parent / f"mlbb_heroes_v{VERSION}_{args.platform}_candidate.zip").resolve()
    if out.exists() and not args.force:
        raise SystemExit(f"Refusing to overwrite existing file without --force: {out}")
    files = rel_files(library_file)
    validate_assets(files, library_file, library_name, args.platform)
    duplicates: dict[str, list[str]] = defaultdict(list)
    for p in files:
        archive_name = library_name if p == library_file else p.relative_to(ROOT).as_posix()
        duplicates[hashlib.sha256(p.read_bytes()).hexdigest()].append(archive_name)
    duplicate_groups = [v for v in duplicates.values() if len(v) > 1]
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for p in files:
            if p == library_file:
                archive_name = f"mlbb_heroes/{library_name}"
            elif p.name == "RELEASE_README.md":
                archive_name = "mlbb_heroes/README.md"
            elif p.name == "Install_Guide.html":
                archive_name = "mlbb_heroes/Install_Guide.html"
            else:
                archive_name = f"mlbb_heroes/{p.relative_to(ROOT).as_posix()}"
            z.write(p, archive_name)

    # Write an external audit, not a file in the install ZIP.
    audit = {
        "archive": str(out),
        "version": json.loads((ROOT / "mod.mod_info").read_text())["version"],
        "platform": args.platform,
        "included_files": len(files),
        "included_paths": [
            f"mlbb_heroes/{library_name}" if p == library_file else
            ("mlbb_heroes/README.md" if p.name == "RELEASE_README.md"
             else "mlbb_heroes/Install_Guide.html" if p.name == "Install_Guide.html"
             else f"mlbb_heroes/{p.relative_to(ROOT).as_posix()}")
            for p in files
        ],
        "excluded_runtime_candidates": sorted(
            [f"effects/{name}" for name in EXCLUDED_EFFECTS if (ROOT / "effects" / name).exists()]
            + [f"effects/{path.name}" for path in (ROOT / "effects").glob("estes_tether_*")]
        ),
        "audio_audit": {
            "sound_events": len(list((ROOT / "sound" / "sfx").glob("*.sound_info"))),
            "clip_references": sum(
                len(json.loads(path.read_text()).get("plays", []))
                for path in (ROOT / "sound" / "sfx").glob("*.sound_info")
            ),
            "event_and_clip_override_routes": sum(
                1 + len(json.loads(path.read_text()).get("plays", []))
                for path in (ROOT / "sound" / "sfx").glob("*.sound_info")
            ),
            "karrie_hawkwatch_wav_clips": len(list((ROOT / "sound" / "sfx").glob("mlbb_karrie_hawkwatch_*.wav"))),
            "runtime_tested": False,
        },
        "exact_duplicate_groups_retained_for_path_compatibility": duplicate_groups,
        "note": "No workspace files were deleted. Source, previews, build outputs, reports, and verified-unused old tether assets are omitted from this install archive.",
    }
    audit_path = out.with_suffix(".audit.json")
    audit_path.write_text(json.dumps(audit, indent=2) + "\n")
    with zipfile.ZipFile(out) as z:
        bad = z.testzip()
        if bad:
            raise SystemExit(f"ZIP CRC verification failed at {bad}")
        size = out.stat().st_size
        uncompressed = sum(i.file_size for i in z.infolist())
    print(f"Created {out}")
    print(f"Platform: {args.platform} | files: {len(files)} | compressed: {size/1e6:.2f} MB | uncompressed: {uncompressed/1e6:.2f} MB")
    print(f"Audit: {audit_path}")
    if duplicate_groups:
        print("Exact duplicate contents retained only where separate referenced asset paths require them:")
        for group in duplicate_groups:
            print("  " + " == ".join(group))


if __name__ == "__main__":
    main()
