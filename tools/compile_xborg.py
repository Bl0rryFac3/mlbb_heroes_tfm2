"""Compile the owner's X.Borg sheets. Re-run from any working directory."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps
from build_popol_preview import svg


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "source/xborg"
PREFIX = "mlbb_xborg_"
NATIVE = "mlbb_heroes:xborg_"
FRAME_TICKS = 4
DIRECTIONS = 16
GAUGE_STEPS = 20
PIVOT = (48, 62)
BODY_CELL = (96, 80)
FLAME_WIDTH = {"s1_armored_flame": 48, "s1_armorless_flame": 64}

# A real source frame can contain an intentional hold. Blank padding cannot.
# Cutoffs for the S1 sheets are measured at the muzzle in source coordinates.
BODY = {
    "armored_idle": (9, 72, None), "armored_run": (9, 42, None),
    "armored_basic_attack": (9, 24, (0, 1, 2, 3, 4, 5, 6, 7)),
    "armored_hurt": (5, 15, None),
    "armored_s1_start": (9, 12, (0, 1, 2, 3)),
    "armored_s1_loop": (9, 32, (2, 3, 4, 5, 6, 7, 8)),
    "armored_s2_cast": (9, 22, None), "armored_s2_recall": (9, 18, None),
    "armored_ult_charge": (9, 13, (3, 4, 5, 6, 7, 8)),
    "armored_ult_spin": (9, 36, None),
    "armored_ult_explosion": (13, 30, tuple(range(4, 13))),
    "armorless_idle": (9, 72, None), "armorless_walk": (9, 36, None),
    "armorless_basic_attack": (9, 24, None), "armorless_hurt": (5, 15, None),
    "armorless_death": (9, 44, None),
    "armorless_s1_start": (9, 12, (0, 1, 2, 3)),
    "armorless_s1_loop": (5, 24, (0, 1, 2, 3, 4)),
    "armorless_s2_cast": (9, 22, None), "armorless_s2_recall": (9, 18, None),
    "armor_break": (13, 33, (0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 12)),
    "armor_restore": (13, 33, (0, 1, 2, 3, 4, 5, 6, 9, 10, 11, 12)),
}
MASKS = {
    "armored_s1_loop": {2: 49, 3: 49, 4: 50, 5: 49, 6: 49, 7: 50, 8: 49},
    "armorless_s1_start": {1: 59, 2: 55, 3: 60},
    "armorless_s1_loop": {1: 55, 2: 53, 3: 56, 4: 60},
    "armorless_basic_attack": {2: 59, 3: 59, 4: 59, 5: 62},
}
FX = {
    "basic_projectile": ((24, 16), 5, 20),
    "projectile_impact": ((24, 24), 5, 15),
    "firaga_supply": ((16, 16), 9, 54),
    "armor_break": ((60, 60), 15, 28),
    "s1_armored_flame": ((128, 128), 13, 32),
    "s1_armorless_flame": ((128, 48), 9, 32),
    "s2_fire_stake": ((72, 72), 9, 36),
    "s2_recall_trail": ((72, 72), 9, 24),
    "ult_fire_loop": ((128, 128), 9, 36),
    "ult_overheat": ((64, 64), 9, 18),
    "ult_explosion": ((128, 128), 9, 26),
}


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def read(name, cell, count, audit):
    path = SOURCE / "received" / f"xborg_{name}.png"
    image = Image.open(path).convert("RGBA")
    w, h = cell
    assert image.width % w == image.height % h == 0, name
    frames = [image.crop((x, y, x + w, y + h))
              for y in range(0, image.height, h) for x in range(0, image.width, w)]
    used = [i for i, frame in enumerate(frames) if frame.getbbox()]
    assert used == list(range(count)), (name, used)
    audit[name] = {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                   "source_dimensions": image.size, "source_cell": cell,
                   "source_frames": count, "empty_padding": len(frames) - count}
    return frames[:count]


def put_frame(frame, scale, top, baseline, center):
    out = Image.new("RGBA", BODY_CELL)
    width = round(frame.width * scale)
    above, below = round(top * scale), round((frame.height - baseline) * scale)
    resized = Image.new("RGBA", (width, above + 45 + below))
    for start, end, height, y in [(0, top, above, 0), (top, baseline, 45, above),
                                  (baseline, frame.height, below, above + 45)]:
        if height:
            band = frame.crop((0, start, frame.width, end))
            resized.paste(band.resize((width, height), Image.Resampling.NEAREST), (0, y))
    xy = (PIVOT[0] - round(center * scale), PIVOT[1] - above - 45)
    box = resized.getbbox()
    assert box and 0 <= box[0] + xy[0] < box[2] + xy[0] <= BODY_CELL[0]
    assert 0 <= box[1] + xy[1] < box[3] + xy[1] <= BODY_CELL[1]
    out.paste(resized, xy)
    return out


def atlas(folder, name, frames, pivot, ticks, tags=None, sampled=False):
    base = ROOT / folder / ("xborg_" + name)
    w, h = frames[0].size
    n = len(frames)
    assert n and all(f.size == (w, h) for f in frames)
    rects = []
    if sampled:
        frames = [*frames, Image.new("RGBA", (w, h))]
    cols = min(6, len(frames))
    sheet = Image.new("RGBA", (cols * w, ((len(frames) + cols - 1) // cols) * h))
    for i, frame in enumerate(frames):
        x, y = i % cols * w, i // cols * h
        sheet.paste(frame, (x, y))
        rects.append({"x": x, "y": y, "w": w, "h": h})
    def frame(i, duration):
        return {"duration": duration, "data": rects[i]}
    if tags is None:
        tags = {"main": [(i, ticks/n/60) for i in range(n)]}
    if sampled:
        tags.update({f"f{i}": [(i, FRAME_TICKS/60), (n, 1/60)] for i in range(n)})
    sheet.save(str(base) + "#sheet.png")
    write(Path(str(base) + "#anim.fanim"), {"anchor_x": pivot[0]/w, "anchor_y": pivot[1]/h,
          "anims": {tag: {"frames": [frame(i, duration) for i, duration in seq]}
                    for tag, seq in tags.items()}})
    return f"asset/mlbb_heroes/{folder}/xborg_{name}"


def bind(views, name, anim, tag="main", z=1, follow=False):
    views.append({"type": "Animation", "name": PREFIX + name, "anim": anim,
                  "tag": tag, "z": z, "is_follow": follow})


def samples(views, name, frames, pivot, ticks, z):
    anim = atlas("effects", name, frames, pivot, ticks, sampled=True)
    for i in range(len(frames)):
        bind(views, f"{name}_{i}", anim, f"f{i}", z)
    return anim


def body_samples(views, name, frames, ticks):
    for side, seq in [("", frames), ("_left", [ImageOps.mirror(f) for f in frames])]:
        anim = samples(views, "body_" + name + side, seq, PIVOT, ticks, 1)
        if name == "armorless_death":
            bind(views, "death" + side, anim)


def directed_frame(frame, angle, size, muzzle=False):
    # A square centered on the socket allows identical rotation anchors in all buckets.
    if muzzle:
        box = frame.getbbox()
        left = box[0]
        core = [y for y in range(frame.height)
                if any(frame.getpixel((x, y))[3] for x in range(left, min(left + 4, box[2])))]
        x0, y0 = left, round((core[0] + core[-1])/2)
        scale = size / (box[2] - box[0])
    else:
        box = frame.getbbox()
        x0, y0 = (box[0] + box[2])//2, (box[1] + box[3])//2
        scale = size / (box[2] - box[0])
    cropped = frame.crop(box)
    resized = cropped.resize(tuple(max(1, round(n * scale)) for n in cropped.size), Image.Resampling.NEAREST)
    side = 160 if muzzle else 64
    canvas = Image.new("RGBA", (side, side))
    xy = (side//2 + round((box[0] - x0) * scale), side//2 + round((box[1] - y0) * scale))
    assert xy[0] >= 0 and xy[1] >= 0 and xy[0]+resized.width <= side and xy[1]+resized.height <= side
    canvas.paste(resized, xy)
    rotated = canvas.rotate(-angle, Image.Resampling.NEAREST)
    assert rotated.getbbox()
    return rotated


def icon(name, frame):
    image = frame.crop(frame.getbbox())
    image.thumbnail((88, 88), Image.Resampling.NEAREST)
    bg = Image.new("RGBA", (96, 96), (35, 29, 39, 255))
    bg.alpha_composite(image, ((96 - image.width)//2, (96 - image.height)//2))
    bg.save(ROOT / "icons" / f"{PREFIX}{name}.png")


def firaga_gauge(views):
    for mode, (track, fill, highlight) in {
        "armor": ((61, 43, 32), (244, 141, 50), (255, 219, 132)),
        "recharge": ((31, 48, 62), (76, 181, 208), (170, 238, 244)),
    }.items():
        frames = []
        for step in range(GAUGE_STEPS + 1):
            frame = Image.new("RGBA", (36, 7))
            draw = ImageDraw.Draw(frame)
            draw.rounded_rectangle((1, 0, 34, 6), radius=2, fill=(21, 19, 27),
                                   outline=(113, 109, 118))
            draw.rectangle((3, 2, 32, 4), fill=track)
            width = round(step * 30 / GAUGE_STEPS)
            if width:
                draw.rectangle((3, 2, 2 + width, 4), fill=fill)
                draw.line((3, 2, 2 + width, 2), fill=highlight)
            frames.append(frame)
        samples(views, f"firaga_gauge_{mode}", frames, (18, 3), FRAME_TICKS, 3)


def main():
    audit, views, bodies, effects = {}, [], {}, {}
    body_ticks = {name: row[1] for name, row in BODY.items()}
    raw = {name: read(name, (72, 72), row[0], audit) for name, row in BODY.items()}
    keys = {}
    for form in ("armored", "armorless"):
        idle = raw[f"{form}_idle"][0]
        top, bottom = idle.getbbox()[1::2]
        boots = idle.getchannel("A").crop((0, bottom-6, 72, bottom)).getbbox()
        keys[form] = (45/(bottom-top), top, bottom, (boots[0]+boots[2])/2)
    for name, (_, ticks, indices) in BODY.items():
        selected = list(range(len(raw[name]))) if indices is None else list(indices)
        frames = []
        for i in selected:
            frame = raw[name][i].copy()
            cutoff = MASKS.get(name, {}).get(i)
            if cutoff is not None:
                frame.paste((0, 0, 0, 0), (cutoff, 0, 72, 72))
            slim = name.startswith("armorless") or (name == "armor_restore" and i <= 1) \
                or (name == "armor_break" and i >= 7) or (name == "armored_ult_explosion" and i >= 7)
            frames.append(put_frame(frame, *keys["armorless" if slim else "armored"]))
        audit[name].update(selected=selected, output_cell=BODY_CELL, ticks=ticks,
                           muzzle_cuts=MASKS.get(name, {}))
        bodies[name] = frames
        body_samples(views, name, frames, ticks)

    for form, gait in [("armored", "armored_run"), ("armorless", "armorless_walk")]:
        loop = f"{form}_s1_loop"
        moving = f"{form}_s1_move"
        recovering = f"{form}_s1_recover"
        frames = []
        for i, gait_frame in enumerate(bodies[gait]):
            frame = bodies[loop][i * len(bodies[loop]) // len(bodies[gait])].copy()
            # Preserve the burner and sleeves, using the authored stride below the hips.
            frame.paste(gait_frame.crop((0, 50, BODY_CELL[0], BODY_CELL[1])), (0, 50))
            frames.append(frame)
        bodies[moving] = frames
        body_ticks[moving] = body_ticks[gait]
        bodies[recovering] = [*reversed(bodies[f"{form}_s1_start"][:3])]
        body_ticks[recovering] = 12
        for name in (moving, recovering):
            body_samples(views, name, bodies[name], body_ticks[name])
        audit[moving] = {"derived_from": [loop, gait], "leg_splice_row": 50, "frames": len(frames)}
        audit[recovering] = {"derived_from": [f"{form}_s1_start"], "selected_reverse": [2, 1, 0]}

    for name, (size, count, ticks) in FX.items():
        frames = read("vfx_" + name, size, count, audit)
        effects[name] = frames
        if name.startswith("s1_") or name.startswith("s2_"):
            is_flame = name.startswith("s1_")
            use = frames[8:] if name == "s1_armored_flame" else frames
            if name == "s2_recall_trail":
                use = [ImageOps.mirror(f) for f in use]
            width = {**FLAME_WIDTH,
                     "s2_fire_stake": 16, "s2_recall_trail": 28}[name]
            audit["vfx_" + name]["output_width"] = width
            for angle in range(DIRECTIONS):
                draw = [directed_frame(f, angle * (360/DIRECTIONS), width, is_flame) for f in use]
                pivot = (draw[0].width//2, draw[0].height//2)
                samples(views, f"{name}_{angle}", draw, pivot, ticks, 2 if is_flame else 0)
            continue
        pivot = (size[0]//2, size[1]//2)
        if name == "firaga_supply":
            samples(views, name, frames, pivot, ticks, 0)
            continue
        if name == "armor_break":
            # The body already provides the initial flash and major fragments.
            frames = frames[9:]
            pivot = (30, 45)
        anim = atlas("effects", name, frames, pivot, ticks)
        if name != "basic_projectile":
            bind(views, name, anim, z=0 if name in ("ult_fire_loop", "ult_explosion", "armor_break") else 2,
                 follow=name == "ult_overheat")
        if name == "ult_explosion":
            forced = []
            for frame in frames[:5]:
                frame = frame.copy()
                frame.putalpha(frame.getchannel("A").point(lambda value: value * 3 // 5))
                forced.append(frame)
            bind(views, "ult_explosion_forced", atlas("effects", "ult_explosion_forced", forced, pivot, 15), z=0)

    for count in range(1, 5):
        marker = Image.new("RGBA", (24, 10))
        draw = ImageDraw.Draw(marker)
        for i in range(4):
            color = (255, 229, 121) if count == 4 else ((247, 134, 52) if i < count else (41, 33, 42))
            draw.rectangle((3 + i*5, 3, 6 + i*5, 6), fill=color, outline=(28, 22, 32))
        bind(views, f"heat_{count}", atlas("effects", f"heat_{count}", [marker], (12, 5), 4), z=3)
    firaga_gauge(views)

    blank = Image.new("RGBA", BODY_CELL)
    first = bodies["armored_idle"][0]
    tags = {key: [(1, 1/60)] for key in ("main", "run", "attack", "hit", "skill", "skill2", "ult", "dead")}
    tags["idle"] = [(0, .00006), (1, 86_400.)]
    carrier = atlas("sprite", "carrier", [first, blank], PIVOT, 1, tags)
    icon("portrait", first)
    for i, frame in enumerate(bodies["armored_idle"]):
        (ROOT / "icons" / f"xborg_draft_idle_{i}.svg").write_text(svg(frame))
    icon("attack", bodies["armored_basic_attack"][3])
    install_data(carrier, views)

    rust = ["// Generated by tools/compile_xborg.py.", f"const FRAME_TICKS: usize = {FRAME_TICKS};",
            f"const GAUGE_STEPS: usize = {GAUGE_STEPS};",
            "fn clip(name: &str) -> (usize, usize) {", "    match name {"]
    rust.extend(f'        "{name}" => ({len(frames)}, {body_ticks[name]}),' for name, frames in bodies.items())
    rust.extend(['        _ => unreachable!("unknown X.Borg animation"),', "    }", "}"])
    (ROOT / "sdk/candidate/xborg_body.rs").write_text("\n".join(rust) + "\n")
    standing = {form: bodies[f"{form}_idle"][0].getbbox() for form in keys}
    assert all(box[3] - box[1] == 45 and box[3] == PIVOT[1] for box in standing.values()), standing
    audit.update(standing=standing,
                 effect_layers={"spin": "owner body ring only", "detonation": "owner blast at damage event plus body recovery",
                                "alternate_ring_and_overheat": "registered for inspection",
                                "firaga_break": "owner body with late debris only"})
    assert len(raw) + len(effects) == 33
    write(SOURCE / "compiled_assets.json", audit)
    print(f"Verified 33 source sheets and {len(views)} compiled view bindings")


def install_data(carrier, views):
    stat = dict(hp=880, attack=96, magic_power=15, defence=33, magic_resistance=25,
                move_speed=1040, hp_regen=7, stack=0, crit_chance=0)
    growth = dict(hp=100, attack=6, magic_power=5, defence=5, magic_resistance=3,
                  move_speed=4, hp_regen=0, stack=0, crit_chance=0)
    d = dict(id="mlbb_xborg", category="Melee", tags=["AD", "Melee", "CC", "Dot"],
             sprite=carrier, anim_prefix="", stat=stat, growth=growth,
             skill_icons=[f"asset/mlbb_heroes/icons/{PREFIX}{key}" for key in ("skill", "skill2", "ult")],
             passive={"passive_ref": NATIVE + "passive"}, view_buffs=[], view_effects=views,
             view_projectiles=[dict(name=PREFIX + "basic_projectile", type="Animated",
                                    anim="asset/mlbb_heroes/effects/xborg_basic_projectile", tag="main", z=1, repeat=True)])
    for slot, cooldown, reach, casting, target in [
        ("attack", 66, 24000, "Targeting", "Enemy"),
        ("skill", 300, 50000, "Direction", "EnemyWithoutTower"),
        ("skill2", 600, 55000, "Direction", "EnemyWithoutTower"),
        ("ult", 2640, 62000, "Direction", "EnemyChampion"),
    ]:
        basic = slot == "attack"
        d[slot] = dict(action_name=slot, description=f"#asset/base/text/champion?description.mlbb_xborg.{slot}",
                       duration=18 if basic else 1, start_timing=5 if basic else 0,
                       cooltime=cooldown, cancelable=True, range=reach, casting_type=casting,
                       casting_target=target, attack_type="BaseAttack" if basic else "Skill",
                       can_use_with_move=not basic, effect={"type": "Native", "effect_ref": NATIVE + slot})
    for folder in (ROOT / "champion", ROOT / "sdk/candidate/champion"):
        write(folder / "mlbb_xborg.data_champion", d)


if __name__ == "__main__":
    main()
