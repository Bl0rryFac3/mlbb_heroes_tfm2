"""Compile approved wiki media. Sun is the default when no heroes are supplied."""
from pathlib import Path
import argparse
import hashlib
import json
import math
import re
import subprocess

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOUNDS = ROOT / "sound/sfx"


def replace_section(text, marker, content):
    pattern = rf"(?m)^( *)(// {marker}_BEGIN).*?^ *// {marker}_END\n"
    updated, count = re.subn(
        pattern,
        lambda match: (
            f"{match[1]}{match[2]}\n"
            + "".join(f"{match[1]}{line}\n" for line in content)
            + f"{match[1]}// {marker}_END\n"
        ),
        text,
        count=1,
        flags=re.S,
    )
    if count != 1:
        raise ValueError(f"Missing section {marker}")
    return updated


def compile_hero(hero, overrides, code):
    media = ROOT / f"source/{hero}/media"
    cue_rows = []
    duration_rows = []
    entries = json.loads((media / "SOURCES.json").read_text())["assets"]
    for entry in entries:
        source = media / entry["source_file"]
        if hashlib.sha256(source.read_bytes()).hexdigest() != entry["source_sha256"]:
            raise ValueError(f"Source hash mismatch for {source.name}")
        cue = entry["cue"]
        if entry["kind"] == "icon":
            with Image.open(source) as image:
                image.save(ROOT / f"icons/{hero}_{cue}.png")
            continue
        event = f"mlbb_{hero}_voice_{cue}"
        clip = f"mlbb_heroes_clip_{event}"
        wav = SOUNDS / f"{clip}.wav"
        subprocess.run([
            "ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(source),
            "-af", "loudnorm=I=-20:TP=-2:LRA=7", "-ac", "1", "-ar", "44100", "-c:a",
            "pcm_s16le", str(wav),
        ], check=True)
        (SOUNDS / f"{event}.sound_info").write_text(json.dumps({
            "plays": [{"delay": 0, "clip": clip, "volume": 0.68}],
        }, indent=2) + "\n")
        for name in (event, clip):
            overrides[f"asset/base/sound/sfx/{name}"] = {
                "remapping": f"asset/mlbb_heroes/sound/sfx/{name}",
                "type": "override",
            }
        seconds = float(subprocess.check_output([
            "ffprobe", "-v", "error", "-show_entries", "format=duration",
            "-of", "default=nw=1:nk=1", str(wav),
        ]))
        priority = str(cue.startswith(("ult", "death"))).lower()
        cue_rows.append(f'("{hero}_{cue}", "{event}", {priority}, false),')
        duration_rows.append(f'"{event}" => {math.ceil(seconds * 60) + 12},')

    code = replace_section(code, f"{hero.upper()}_CUES", cue_rows)
    return replace_section(code, f"{hero.upper()}_DURATIONS", duration_rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("heroes", nargs="*", default=["sun"])
    args = parser.parse_args()
    overrides_path = ROOT / "mod.override_info"
    overrides = json.loads(overrides_path.read_text())
    rust = ROOT / "sdk/candidate/audio.rs"
    code = rust.read_text()
    for hero in args.heroes:
        if hero not in ("sun", "lesley", "valir"):
            parser.error(f"unsupported hero: {hero}")
        code = compile_hero(hero, overrides, code)
    overrides_path.write_text(json.dumps(overrides, indent=2) + "\n")
    rust.write_text(code)


if __name__ == "__main__":
    main()
