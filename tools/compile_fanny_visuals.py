"""Pack the owner's Fanny sheets into the existing TFM2 animation bindings."""
from pathlib import Path
import json

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "source/fanny/visuals"


def split(name, width, height):
    image = Image.open(ART / (name + ".png")).convert("RGBA")
    if image.width % width or image.height % height:
        raise ValueError(f"{name} does not match {width}x{height} cells")
    frames = [
        image.crop((x, y, x + width, y + height))
        for y in range(0, image.height, height)
        for x in range(0, image.width, width)
    ]
    return [frame for frame in frames if frame.getbbox()]


def place(frame, size, offset):
    canvas = Image.new("RGBA", size)
    canvas.paste(frame, offset)
    return canvas


def centered(frame, size):
    return place(frame, size, ((size[0] - frame.width) // 2, (size[1] - frame.height) // 2))


def pack(directory, name, clips, size):
    width, height = size
    columns = max(len(frames) for frames, _ in clips.values())
    atlas = Image.new("RGBA", (width * columns, height * len(clips)))
    animations = {}
    for row, (tag, (frames, seconds)) in enumerate(clips.items()):
        track = []
        for column, frame in enumerate(frames):
            x, y = column * width, row * height
            atlas.paste(frame, (x, y))
            track.append({"duration": seconds / len(frames), "data": {"x": x, "y": y, "w": width, "h": height}})
        animations[tag] = {"frames": track}
    base = ROOT / directory / name
    atlas.save(base.with_name(base.name + "#sheet.png"))
    base.with_name(base.name + "#anim.fanim").write_text(json.dumps({"anims": animations}, indent=2) + "\n")


def main():
    body = {}
    for tag, name, seconds in [
        ("idle", "idle", 1.40), ("run", "run", .48), ("attack", "basic", 20 / 60),
        ("skill", "skill1", .30), ("skill2", "skill2", .30), ("ult", "ult", 32 / 60),
        ("hit", "hit", .375), ("dead", "dead", 1.28),
    ]:
        poses = split(f"fanny-{name}", 72, 72)
        # Keep the pack's default ground line near y=55 and leave room for blades.
        body[tag] = ([place(pose, (88, 72), (16, -6)) for pose in poses], seconds)
    pack("sprite", "fanny", body, (88, 72))

    for asset, source, tag, source_cell, canvas, seconds in [
        ("fanny_tornado", "fanny_tornado_strike", "skill", (100, 100), (128, 128), .32),
        ("fanny_cable_start", "fanny_cable_start", "skill", (64, 52), (64, 64), .12),
        ("fanny_cable_impact", "fanny_cable_anchor_impact", "impact", (24, 24), (48, 48), .20),
    ]:
        frames = [centered(frame, canvas) for frame in split(source, *source_cell)]
        pack("effects", asset, {tag: (frames, seconds)}, canvas)

    wire = centered(split("fanny_cable_wire", 32, 16)[0], (48, 48))
    hook = centered(split("fanny_cable_hook", 24, 16)[0], (48, 48))
    tether = {"hook": ([hook], .20)}
    for direction in range(12):
        angle = direction * 15
        tether[f"wire_{direction}"] = ([wire.rotate(-angle)], .16)
    for direction in range(24):
        angle = direction * 15
        tether[f"hook_{direction}"] = ([hook.rotate(-angle)], .10)
    pack("effects", "fanny_hook", tether, (48, 48))

    frames = split("fanny_prey_mark-animated", 60, 64)
    # The animated sheet is left-aligned. Align its shield with the standalone icon.
    aligned = [place(frame, (64, 64), (13, 0)) for frame in frames]
    icon = centered(split("fanny_prey_mark_1", 24, 24)[0], (64, 64))
    pack("effects", "fanny_prey_mark", {
        "mark_1": ([*aligned[:5], icon], .45),
        "mark_2": (aligned[6:], .45),
    }, (64, 64))


if __name__ == "__main__":
    main()
