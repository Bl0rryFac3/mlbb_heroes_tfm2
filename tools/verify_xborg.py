"""Check the owner input, two standing pivots, and all emitted X.Borg bindings."""
import hashlib
import json
import re
import wave
from pathlib import Path

from PIL import Image
from build_popol_preview import svg


ROOT = Path(__file__).resolve().parents[1]


def load(relative):
    return json.loads((ROOT / relative).read_text())


def main():
    source = ROOT / "source/xborg"
    intake = load("source/xborg/intake-hashes.json")
    for entry in intake:
        data = (source / "received" / entry["file"]).read_bytes()
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["file"]
    assert len(intake) == 33
    art = load("source/xborg/compiled_assets.json")
    assert all(box[3] - box[1] == 45 and box[3] == 62 for box in art["standing"].values())
    data = load("champion/mlbb_xborg.data_champion")
    assert data == load("sdk/candidate/champion/mlbb_xborg.data_champion")
    assert data["stat"]["hp"] == 880
    assert all(data[k]["casting_type"] == "Direction" for k in ("skill", "skill2", "ult"))
    idle = "effects/xborg_body_armored_idle"
    frames = load(idle + "#anim.fanim")["anims"]["main"]["frames"]
    assert len(frames) == 9
    previews = []
    with Image.open(ROOT / (idle + "#sheet.png")) as sheet:
        for i, frame in enumerate(frames):
            x, y, w, h = (frame["data"][axis] for axis in ("x", "y", "w", "h"))
            stem = ROOT / "icons" / f"xborg_draft_idle_{i}"
            assert not stem.with_suffix(".png").exists()
            preview = stem.with_suffix(".svg").read_text()
            assert preview == svg(sheet.crop((x, y, x+w, y+h)))
            previews.append(preview)
    assert len(set(previews)) >= 7
    assert data["skill_icons"] == [
        f"asset/mlbb_heroes/icons/mlbb_xborg_{slot}" for slot in ("skill", "skill2", "ult")
    ]
    media = load("source/xborg/media/SOURCES.json")["assets"]
    assert sum(entry["kind"] == "icon" for entry in media) == 4
    assert sum(entry["kind"] == "voice" for entry in media) == 29
    routes = load("mod.override_info")
    audio = (ROOT / "sdk/candidate/audio.rs").read_text()
    voice = "\n".join((ROOT / "sdk/candidate" / path).read_text()
                      for path in ("xborg.rs", "xborg/combat.rs", "xborg/voice.rs"))
    retained_clips = set()
    for entry in media:
        original = source / "media" / entry["source_file"]
        assert hashlib.sha256(original.read_bytes()).hexdigest() == entry["source_sha256"]
        slot = entry["cue"]
        if entry["kind"] == "icon":
            with Image.open(ROOT / f"icons/mlbb_xborg_{slot}.png") as icon, Image.open(original) as raw:
                assert icon.size == raw.size == (100, 100)
                assert icon.convert("RGBA").tobytes() == raw.convert("RGBA").tobytes()
        else:
            event = f"mlbb_xborg_voice_{slot}"
            plays = load(f"sound/sfx/{event}.sound_info")["plays"]
            assert len(plays) == 1
            clip = plays[0]["clip"]
            assert clip.startswith("mlbb_heroes_clip_mlbb_xborg_voice_")
            retained_clips.add(clip)
            assert f'"{event}"' in voice
            assert f'("xborg_{slot}", "{event}",' in audio and f'"{event}" =>' in audio
            for name in (event, clip):
                assert routes[f"asset/base/sound/sfx/{name}"]["remapping"] == f"asset/mlbb_heroes/sound/sfx/{name}"
            with wave.open(str(ROOT / f"sound/sfx/{clip}.wav")) as wav:
                assert wav.getnchannels() == 1 and wav.getframerate() == 44100 and wav.getnframes() > 0
    assert len(retained_clips) == 11
    for locale in load("text/champion.i18n").values():
        tooltips = locale["description"]["mlbb_xborg"]
        for slot in ("attack", "passive", "skill", "skill2", "ult"):
            colored = re.findall(r"<#[a-f0-9]{8}>[^<>]+<>", tooltips[slot])
            assert colored and tooltips[slot].count("<>") == len(colored)
        assert all(number in tooltips["ult"] for number in ("3s", "50%", "150", "44s"))
    bindings = data["view_effects"] + data["view_projectiles"]
    assert len(set(b["name"] for b in bindings)) == len(bindings)
    paths = {}
    for binding in bindings:
        path = binding["anim"].removeprefix("asset/mlbb_heroes/")
        paths.setdefault(path, set()).add(binding["tag"])
    paths[data["sprite"].removeprefix("asset/mlbb_heroes/")] = {"idle", "run", "attack", "skill", "skill2", "ult", "dead"}
    for mode in ("armor", "recharge"):
        assert {f"f{i}" for i in range(21)} <= paths[f"effects/xborg_firaga_gauge_{mode}"]
    for form, width in (("armored", 48), ("armorless", 64)):
        path = f"effects/xborg_s1_{form}_flame_0"
        animation = load(path + "#anim.fanim")
        assert animation["anchor_x"] == animation["anchor_y"] == 0.5
        with Image.open(ROOT / (path + "#sheet.png")) as png:
            frame = png.crop((0, 0, 160, 160))
            bbox = frame.getbbox()
            assert bbox[0] == 80 and abs(bbox[2] - 80 - width) <= 1, (path, bbox)
    for path, tags in paths.items():
        animation = load(path + "#anim.fanim")
        with Image.open(ROOT / (path + "#sheet.png")) as png:
            assert png.mode == "RGBA"
            for tag in tags:
                for frame in animation["anims"][tag]["frames"]:
                    x, y, w, h = (frame["data"][k] for k in ("x", "y", "w", "h"))
                    assert frame["duration"] > 0 and 0 <= x < x+w <= png.width and 0 <= y < y+h <= png.height, path
    for clip in ("armored_ult_spin", "armored_s1_move", "armorless_s1_move"):
        path = f"effects/xborg_body_{clip}"
        with Image.open(ROOT / (path + "#sheet.png")) as png:
            digests = set()
            for frame in load(path + "#anim.fanim")["anims"]["main"]["frames"]:
                x,y,w,h = (frame["data"][k] for k in ("x","y","w","h"))
                digests.add(hashlib.sha256(png.crop((x,y,x+w,y+h)).tobytes()).digest())
            assert len(digests) >= 7, clip
    print(f"PASS 33 original PNGs, two 45 px masters, {len(paths)} atlases and {len(bindings)} runtime bindings")


if __name__ == "__main__":
    main()
