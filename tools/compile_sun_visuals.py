"""Pack Sun's owner-supplied sheets into TFM2 atlases."""
from PIL import Image

from compile_fanny_visuals import ROOT, pack, place

ART = ROOT / "source/sun/visuals"


def split(name, cell, count):
    with Image.open(ART / f"{name}.png") as source:
        image = source.convert("RGBA")
    width, height = cell
    if image.width % width or image.height % height:
        raise ValueError(f"{name} does not match {cell} cells")
    frames = [
        image.crop((x, y, x + width, y + height))
        for y in range(0, image.height, height)
        for x in range(0, image.width, width)
    ]
    if not all(frame.getbbox() for frame in frames[:count]) or any(
        frame.getbbox() for frame in frames[count:]
    ) or len(frames) < count:
        raise ValueError(f"Unexpected used-frame layout in {name}")
    return frames[:count]


def main():
    body = {}
    for tag, file, count, ticks in [
        ("idle", "idle", 9, 80),
        ("run", "run", 9, 36),
        ("attack", "basic_a", 9, 22),
        ("attack_b", "basic_b", 9, 22),
        ("skill", "endless_variety_cast", 9, 24),
        ("exchange_depart", "swift_exchange_depart", 9, 22),
        ("exchange_arrive", "swift_exchange_arrive", 9, 22),
        ("skill2", "instantaneous_move_dash", 9, 12),
        ("strike", "instantaneous_move_strike", 9, 18),
        ("ult", "clone_techniques_cast", 13, 32),
        ("dead", "death", 13, 54),
    ]:
        frames = split(f"sun_{file}", (72, 72), count)
        # Set the feet near y=55, the roster's existing match-camera pivot.
        if any(frame.getbbox()[1] < 5 for frame in frames):
            raise ValueError(f"Shifting {file} would clip artwork")
        body[tag] = ([place(frame, (72, 72), (0, -5)) for frame in frames], ticks / 60)
    body["hit"] = (body["dead"][0][:5], 18 / 60)
    pack("sprite", "sun", body, (72, 72))
    pack(
        "champions", "mlbb_sun_clone",
        {tag: body[tag] for tag in ("idle", "run", "attack", "strike", "hit", "dead")}, (72, 72),
    )
    for name, cell, count, ticks in [
        ("basic_staff_swing_arc", (60, 60), 9, 15),
        ("staff_hit_spark", (52, 68), 6, 16),
        ("clone_spawn_smoke", (72, 72), 7, 24),
        ("clone_summon", (48, 48), 7, 24),
        ("clone_expire", (48, 48), 8, 24),
        ("simian_heal_pulse", (32, 32), 7, 22),
        ("armor_shred_stacks", (24, 24), 9, 48),
        ("instantaneous_dash_streak", (88, 48), 9, 16),
        ("staff_projectile", (72, 72), 9, 18),
    ]:
        frames = split(f"sun_{name}", cell, count)
        clips = {"play": (frames, ticks / 60)}
        if name == "staff_projectile":
            clips.update({f"step_{i}": ([frame], 2 / 60) for i, frame in enumerate(frames)})
        pack("effects", f"sun_{name}", clips, cell)



if __name__ == "__main__":
    main()
