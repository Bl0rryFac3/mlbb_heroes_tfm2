#!/usr/bin/env python3
"""Render Chou's authored animation tags and a closing S1/S2/Ultimate combo to GIF."""
from __future__ import annotations
import json
import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
SHEET = ROOT / "sprite" / "chou#sheet.png"
ANIM = ROOT / "sprite" / "chou#anim.fanim"
OUT = ROOT.parent / "chou_showcase.gif"
W, H = 640, 360
FLOOR = 286
SCALE = 2.15

FONT_DIR = Path("/usr/share/fonts/truetype/dejavu")
def font(name: str, size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    p = FONT_DIR / name
    return ImageFont.truetype(str(p), size) if p.exists() else ImageFont.load_default()

F_REG = font("DejaVuSans.ttf", 11)
F_BOLD = font("DejaVuSans-Bold.ttf", 20)
F_SMALL = font("DejaVuSans.ttf", 8)
F_MICRO = font("DejaVuSansMono.ttf", 7)

sheet = Image.open(SHEET).convert("RGBA")
anims = json.loads(ANIM.read_text())["anims"]

# Stable, dark arena background in the same compact showcase spirit as the reference.
_bg = Image.new("RGB", (W, H))
pix = _bg.load()
for y in range(H):
    t = y / H
    col = (int(7 + 4*t), int(14 + 9*t), int(27 + 16*t))
    for x in range(W):
        pix[x, y] = col

def background() -> Image.Image:
    im = _bg.copy().convert("RGBA")
    d = ImageDraw.Draw(im, "RGBA")
    # Very subtle arena grid and horizon.
    for x in range(-260, W + 260, 54):
        d.line((x, 76, x + 150, FLOOR), fill=(49, 70, 99, 32), width=1)
    for y in (112, 160, 210, 250):
        d.line((0, y, W, y), fill=(43, 61, 85, 18), width=1)
    d.ellipse((42, 260, 596, 322), fill=(2, 7, 18, 155))
    d.line((20, FLOOR, 620, FLOOR), fill=(135, 158, 185, 150), width=1)
    # Header plate.
    d.rounded_rectangle((18, 13, 622, 70), radius=13, fill=(5, 12, 25, 218), outline=(101, 123, 155, 115), width=1)
    d.text((32, 19), "CHOU", font=F_BOLD, fill=(255, 203, 104, 255), stroke_width=0)
    d.text((34, 46), "TFM2  /  ANIMATION + COMBO SHOWCASE", font=F_SMALL, fill=(181, 194, 212, 255))
    # Footer / timeline.
    d.line((20, 337, 620, 337), fill=(76, 93, 117, 255), width=4)
    d.line((20, 337, 20, 345), fill=(122, 214, 231, 255), width=1)
    d.text((22, 347), "ONLY FAST  •  JEET KUNE DO  •  SHUNPO  •  THE WAY OF DRAGON", font=F_MICRO, fill=(179, 193, 211, 255))
    return im

def draw_dummy(im: Image.Image, x: int = 466, state: str = "idle", progress: float = 0.0) -> None:
    d = ImageDraw.Draw(im, "RGBA")
    # Training target outline, with a small health bar.
    ybase = FLOOR
    if state == "airborne":
        ybase -= 44 * math.sin(math.pi * progress)
    lean = 0
    if state == "hit":
        lean = int(13 * math.sin(progress * math.pi))
    xx = x - (int(28 * progress) if state == "knockback" else 0)
    head_x, head_y = xx, ybase - 77
    pink = (231, 91, 159, 235)
    # capsule top bar
    d.rounded_rectangle((xx-24, head_y-13, xx+24, head_y-7), radius=3, outline=pink, width=1)
    d.ellipse((head_x-9, head_y, head_x+9, head_y+18), outline=pink, width=2)
    neck = (xx, head_y+18)
    shoulder = (xx + lean, head_y+32)
    hip = (xx, head_y+53)
    d.line((neck, shoulder), fill=pink, width=2)
    d.line((shoulder[0]-17, shoulder[1]+10, shoulder[0]+17, shoulder[1]+10), fill=pink, width=2)
    d.line((shoulder[0]-17, shoulder[1]+10, shoulder[0]-28, shoulder[1]+25), fill=pink, width=2)
    d.line((shoulder[0]+17, shoulder[1]+10, shoulder[0]+29, shoulder[1]+23), fill=pink, width=2)
    d.line((shoulder, hip), fill=pink, width=2)
    d.line((hip, (xx-16, ybase-3)), fill=pink, width=2)
    d.line((hip, (xx+18, ybase-3)), fill=pink, width=2)
    # clean, readable health bar
    d.rounded_rectangle((xx-31, ybase-104, xx+31, ybase-98), radius=3, fill=(28, 36, 54, 235), outline=(241, 107, 168, 225), width=1)
    hp = 1.0 if state == "idle" else max(0.12, 1.0 - 0.78 * progress)
    d.rounded_rectangle((xx-29, ybase-102, xx-29+int(58*hp), ybase-100), radius=1, fill=(255, 102, 164, 255))
    if state == "airborne":
        d.text((xx-29, ybase-123), "AIRBORNE", font=F_MICRO, fill=(255, 196, 222, 255))

def paste_chou(im: Image.Image, tag: str, frame_obj: dict, x: float, yoffset: float = 0.0, flip: bool = False, hit_glow: bool = False) -> None:
    rect = frame_obj["data"]
    crop = sheet.crop((rect["x"], rect["y"], rect["x"] + rect["w"], rect["y"] + rect["h"]))
    crop = crop.resize((round(rect["w"]*SCALE), round(rect["h"]*SCALE)), Image.Resampling.NEAREST)
    if flip:
        crop = crop.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    px = round(x - 34*SCALE)
    py = round(FLOOR - 55*SCALE + yoffset)
    if hit_glow:
        glow = Image.new("RGBA", crop.size, (255, 199, 76, 0))
        glow.putalpha(crop.getchannel("A").point(lambda a: int(a*0.20)))
        im.alpha_composite(glow, (px, py))
    im.alpha_composite(crop, (px, py))

def label_frame(im: Image.Image, section: str, counter: str, note: str = "") -> None:
    d = ImageDraw.Draw(im, "RGBA")
    d.text((550, 20), section.upper(), font=F_REG, fill=(235, 242, 252, 255), anchor="ra")
    d.text((588, 47), counter, font=F_SMALL, fill=(190, 203, 221, 255), anchor="ra")
    if note:
        d.rounded_rectangle((24, 83, 24 + max(105, int(d.textlength(note, font=F_REG))+18), 105), radius=6, fill=(7, 15, 29, 210), outline=(63, 89, 120, 125), width=1)
        d.text((33, 88), note, font=F_REG, fill=(228, 235, 246, 255))

def add_clip(frames: list[Image.Image], durations: list[int], tag: str, section: str, count: int, total: int, *, x: float = 245, dummy_state: str = "idle", note: str | None = None, flip: bool = False) -> None:
    clip = anims[tag]["frames"]
    for idx, fr in enumerate(clip):
        im = background()
        progress = idx / max(1, len(clip)-1)
        draw_dummy(im, state=dummy_state, progress=progress)
        label_frame(im, section, f"{count:02d} / {total:02d}", note or tag.replace("_", " ").upper())
        # Small stage marker ties the authored clips together.
        d = ImageDraw.Draw(im, "RGBA")
        d.text((24, 317), "CHOU  •  TRAINING GROUND", font=F_MICRO, fill=(135, 155, 181, 255))
        paste_chou(im, tag, fr, x=x, flip=flip, hit_glow=tag in ("skill", "skill_recast", "skill_finisher", "ult", "ult_recast"))
        frames.append(im.convert("RGB"))
        durations.append(max(55, round(float(fr.get("duration", 0.1))*1000)))

def add_combo_segment(frames: list[Image.Image], durations: list[int], tag: str, x: float, phase: str, counter: str, note: str, target_state: str = "idle", progress_shift: float = 0.0, flip: bool = False) -> None:
    clip = anims[tag]["frames"]
    for idx, fr in enumerate(clip):
        prog = min(1.0, progress_shift + idx / max(1, len(clip)-1))
        im = background()
        draw_dummy(im, state=target_state, progress=prog)
        label_frame(im, "COMBO FINISHER", counter, note)
        d = ImageDraw.Draw(im, "RGBA")
        # hit flash at the target during impact-heavy parts
        if phase in ("AIRBORNE", "KNOCKBACK", "RECAST") and idx % 3 == 0:
            d.ellipse((429, 200, 503, 274), outline=(255, 206, 101, 105), width=2)
        d.text((24, 317), "S1-1  →  S1-2 CLOSE  →  S1-3 AIRBORNE  →  S2 BEHIND  →  ULT  →  RECAST", font=F_MICRO, fill=(135, 155, 181, 255))
        paste_chou(im, tag, fr, x=x, flip=flip, hit_glow=True)
        # phase chip at right of the lower middle
        chip_w = int(ImageDraw.Draw(im).textlength(phase, font=F_SMALL))+18
        d.rounded_rectangle((W-chip_w-21, 307, W-20, 327), radius=5, fill=(42, 20, 45, 220), outline=(229, 91, 159, 160), width=1)
        d.text((W-chip_w-12, 312), phase, font=F_SMALL, fill=(255, 199, 222, 255))
        frames.append(im.convert("RGB"))
        durations.append(max(55, round(float(fr.get("duration", 0.1))*1000)))

# Display every authored Chou animation tag, including hit and death frames,
# then finish with the complete approved S1/S2/Ultimate kick sequence.
frames: list[Image.Image] = []
durations: list[int] = []
ordered = [
    ("idle", "IDLE", "READY STANCE"),
    ("run", "RUN", "MOBILITY"),
    ("attack", "BASIC ATTACK", "ONLY FAST"),
    ("skill", "SKILL 1", "JEET KUNE DO  •  CAST 1"),
    ("skill_recast", "SKILL 1", "JEET KUNE DO  •  CAST 2"),
    ("skill_finisher", "SKILL 1", "JEET KUNE DO  •  FINISHER"),
    ("skill2", "SKILL 2", "SHUNPO"),
    ("ult", "ULTIMATE", "THE WAY OF DRAGON  •  ROUNDHOUSE"),
    ("ult_recast", "ULTIMATE", "AERIAL KICK FLURRY"),
    ("hit", "REACTION", "HIT"),
    ("dead", "REACTION", "DEFEAT"),
]
for i, (tag, title, note) in enumerate(ordered, 1):
    add_clip(frames, durations, tag, title, i, len(ordered), x=245, note=note)
# Brief title beat, then the continuous skill sequence.
for _ in range(2):
    im = background()
    draw_dummy(im)
    label_frame(im, "FINAL SEQUENCE", "12 / 12", "THE CHOU INSEC COMBO")
    d = ImageDraw.Draw(im, "RGBA")
    d.text((24, 317), "S1 CASTS 1–3  →  S2 BEHIND  →  ULT  →  RECAST", font=F_MICRO, fill=(135, 155, 181, 255))
    paste_chou(im, "idle", anims["idle"]["frames"][0], x=245)
    frames.append(im.convert("RGB")); durations.append(100)
# Continuous combo: S1 cast 1, S1 cast 2 dashes in, S1 cast 3 launches
# airborne, Shunpo moves behind the target, then roundhouse and recast.
combo = [
    ("skill", 245, "CAST 1/3", "RISING KICK", "idle", 0.00, False),
    ("skill_recast", 292, "CAST 2/3", "DASH UPPERCUT  •  CLOSE IN", "hit", 0.15, False),
    ("skill_finisher", 354, "CAST 3/3", "SLAM  •  AIRBORNE", "airborne", 0.20, False),
    ("skill2", 535, "SHUNPO", "DASH BEHIND TARGET", "airborne", 0.65, True),
    ("ult", 535, "ULTIMATE", "ROUNDHOUSE  •  100K KNOCKBACK", "knockback", 0.10, True),
    ("ult_recast", 490, "RECAST", "CHASE  •  AERIAL KICK FLURRY", "airborne", 0.45, True),
]
for tag, x, phase, note, target_state, shift, flip in combo:
    add_combo_segment(frames, durations, tag, x, phase, "12 / 12", note, target_state, shift, flip)
# Quiet end card after the final combo, retaining the dark showcase stage.
for i in range(10):
    im = background()
    draw_dummy(im, state="airborne", progress=max(0.1, 1-i/10))
    label_frame(im, "COMBO COMPLETE", "12 / 12", "CHOU  •  THE WAY OF DRAGON")
    d = ImageDraw.Draw(im, "RGBA")
    d.text((24, 317), "JEET KUNE DO  →  SHUNPO  →  ROUNDHOUSE  →  KICK FLURRY", font=F_MICRO, fill=(135, 155, 181, 255))
    paste_chou(im, "idle", anims["idle"]["frames"][i % len(anims["idle"]["frames"])], x=433, flip=True)
    frames.append(im.convert("RGB")); durations.append(100)

OUT.parent.mkdir(parents=True, exist_ok=True)
frames[0].save(OUT, save_all=True, append_images=frames[1:], duration=durations, loop=0, optimize=True, disposal=2)
print(f"Wrote {OUT}: {len(frames)} frames, {sum(durations)/1000:.1f}s, {OUT.stat().st_size/1024/1024:.2f} MiB")
