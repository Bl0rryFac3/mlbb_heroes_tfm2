"""Pack the supplied Aldous sprite sheets into the existing champion atlas."""

import json
from PIL import Image

from compile_fanny_visuals import ROOT, pack, place

ART = ROOT / "source/aldous/visuals"
SIZE = (72, 72)


def read_frames(name, count, vertical_offset):
    with Image.open(ART / f"{name}.png") as source:
        image = source.convert("RGBA")
    width, height = SIZE
    if image.width % width or image.height % height:
        raise ValueError(f"{name} does not use {width}x{height} cells")
    cells = [
        image.crop((x, y, x + width, y + height))
        for y in range(0, image.height, height)
        for x in range(0, image.width, width)
    ]
    used = [i for i, cell in enumerate(cells) if cell.getchannel("A").getbbox()]
    if used != list(range(count)):
        raise ValueError(f"{name} has unexpected occupied cells: {used}")
    return [place(cell, SIZE, (0, vertical_offset)) for cell in cells[:count]]


def main():
    clips = {}
    for tag, source, frames, seconds, offset in [
        ("idle", "idle", 9, 0.78, -4),
        ("run", "walk", 9, 0.57, -4),
        ("attack", "basic", 13, 0.26, -4),
        ("skill", "skill1", 13, 0.36, -4),
        ("skill2", "skill2", 9, 0.34, -4),
        ("hit", "hit", 9, 0.21, -7),
        ("dead", "death", 9, 0.78, -7),
    ]:
        clips[tag] = (read_frames(source, frames, offset), seconds)

    acquire = read_frames("ultimate_acquire", 9, -4)
    transform = read_frames("ultimate_transform", 13, -4)
    flight = read_frames("ultimate_flight", 13, -4)
    recover = read_frames("ultimate_recover", 13, -4)
    # The impact event ends the outbound animation, including on a short charge.
    # Repeat the drill poses past the maximum normal travel time so a long
    # charge never cycles back to the opening pose before the impact.
    opening = [acquire[0], acquire[2], acquire[4], *transform[1:5]]
    clips["ult"] = ([*opening, *flight], 0.4)
    clips["ult_recover"] = (recover[7:13], 12 / 60)
    pack("champions", "mlbb_aldous", clips, SIZE)
    path = ROOT / "champions/mlbb_aldous#anim.fanim"
    atlas = json.loads(path.read_text())
    track = atlas["anims"]["ult"]["frames"]
    atlas["anims"]["ult"]["frames"] = [*track[:len(opening)], *(track[len(opening):] * 8)]
    path.write_text(json.dumps(atlas, indent=2) + "\n")


if __name__ == "__main__":
    main()
