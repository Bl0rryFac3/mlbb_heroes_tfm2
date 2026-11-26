"""Build Balmond's runtime atlas from the supplied 72-pixel sprite sheets."""

import json

from PIL import Image

from compile_fanny_visuals import ROOT, pack, place

ART = ROOT / "source/balmond/visuals"
SOURCE_CELL = 72
ATLAS_CELL = (82, 92)
TICKS_PER_SECOND = 60
SWEEP_TICKS = 180
SPIN_FRAME_TICKS = 3


def read_poses(name, count):
    with Image.open(ART / f"{name}.png") as source:
        image = source.convert("RGBA")
    if image.width % SOURCE_CELL or image.height % SOURCE_CELL:
        raise ValueError(f"{name} must use {SOURCE_CELL}-pixel cells")
    cells = [
        image.crop((x, y, x + SOURCE_CELL, y + SOURCE_CELL))
        for y in range(0, image.height, SOURCE_CELL)
        for x in range(0, image.width, SOURCE_CELL)
    ]
    populated = [index for index, cell in enumerate(cells) if cell.getbbox()]
    if populated != list(range(count)):
        raise ValueError(f"{name} has unexpected occupied cells: {populated}")
    # Keep the feet 17 pixels below center without cropping the raised axe.
    return [place(cell, ATLAS_CELL, (5, 0)) for cell in cells[:count]]


def main():
    clips = {}
    for tag, source, frames, ticks in (
        ("idle", "idle", 9, 58),
        ("run", "walk", 9, 25),
        ("attack", "basic", 9, 20),
        ("skill", "skill1", 13, 30),
        ("ult", "ult", 13, 30),
        ("hit", "hit", 9, 14),
        ("dead", "death", 9, 53),
    ):
        clips[tag] = (read_poses(source, frames), ticks / TICKS_PER_SECOND)

    activation = read_poses("spin_activation", 13)[:4]
    # The last five source cells begin a second turn. Reuse the first complete
    # turn so both the stationary and moving cast share the same playback.
    turn = read_poses("spin_continuous", 13)[:8]
    poses = activation + turn
    clips["skill2"] = (poses, len(poses) * SPIN_FRAME_TICKS / TICKS_PER_SECOND)
    pack("sprite", "balmond", clips, ATLAS_CELL)

    animation_path = ROOT / "sprite/balmond#anim.fanim"
    atlas = json.loads(animation_path.read_text())
    track = atlas["anims"]["skill2"]["frames"]
    opening, loop = track[:len(activation)], track[len(activation):]
    turns, remainder = divmod(SWEEP_TICKS // SPIN_FRAME_TICKS - len(opening), len(loop))
    if remainder or SWEEP_TICKS % SPIN_FRAME_TICKS:
        raise ValueError("The spin must play full turns through the entire damage window")
    atlas["anims"]["skill2"]["frames"] = opening + loop * turns
    animation_path.write_text(json.dumps(atlas, indent=2) + "\n")


if __name__ == "__main__":
    main()
