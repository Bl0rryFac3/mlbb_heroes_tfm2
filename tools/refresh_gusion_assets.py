#!/usr/bin/env python3
"""Deterministically refresh Gusion UI icons and original gameplay SFX."""
from __future__ import annotations

import json
import math
import random
import wave
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
SAMPLE_RATE = 22_050
SCALE = 4
SIZE = 64


def scaled_xy(points):
    return [(round(x * SCALE), round(y * SCALE)) for x, y in points]


def glow_layer(draw_fn, canvas_size):
    layer = Image.new("RGBA", canvas_size, (0, 0, 0, 0))
    draw_fn(ImageDraw.Draw(layer))
    return layer


def make_icon(name: str, draw_art) -> None:
    s = SIZE * SCALE
    glow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    # Soft cyan outer bloom behind the crisp, transparent icon art.
    g = ImageDraw.Draw(glow)
    draw_art(g, SCALE, glow=True)
    glow = glow.filter(ImageFilter.GaussianBlur(4 * SCALE))
    art = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw_art(ImageDraw.Draw(art), SCALE, glow=False)
    image = Image.alpha_composite(glow, art).resize((SIZE, SIZE), Image.Resampling.LANCZOS)
    image.save(ROOT / "icons" / name, optimize=True)


def dagger(draw: ImageDraw.ImageDraw, scale: int, cx: float, cy: float, length: float,
           angle: float, glow: bool = False, thickness: float = 1.0) -> None:
    # A compact silver-white blade with pale-cyan edge and angular silver hilt.
    ca, sa = math.cos(angle), math.sin(angle)
    def p(x, y):
        return ((cx + ca*x - sa*y) * scale, (cy + sa*x + ca*y) * scale)
    tip = p(length * .58, 0)
    blade_base = p(-length * .24, 0)
    blade_top = p(length * .22, -thickness * 2.0)
    blade_bottom = p(length * .22, thickness * 2.0)
    if glow:
        draw.polygon([tip, blade_top, blade_bottom], fill=(71, 220, 255, 220))
        draw.line([p(-length*.30, -thickness*2), p(-length*.30, thickness*2)], fill=(150,245,255,210), width=max(2, round(scale*2)))
        return
    draw.polygon([tip, blade_top, blade_bottom], fill=(202, 252, 255, 255))
    draw.line([blade_base, tip], fill=(74, 231, 255, 255), width=max(1, round(scale*1.15)))
    draw.polygon([p(-length*.30, -thickness*2), p(-length*.30, thickness*2), p(-length*.44, thickness*1.1), p(-length*.44, -thickness*1.1)], fill=(226, 239, 247, 255))
    draw.line([p(-length*.43, 0), p(-length*.61, 0)], fill=(250, 219, 146, 255), width=max(1, round(scale*1.2)))
    bx, by = p(-length*.625, 0)
    br = max(1, thickness * scale * .9)
    draw.ellipse((bx-br, by-br, bx+br, by+br), fill=(104,223,244,255))


def icon_s1(draw, scale, glow=False):
    dagger(draw, scale, 31.4, 32.0, 36, -math.pi/4, glow=glow, thickness=1.0)
    if glow:
        draw.ellipse((38*scale, 7*scale, 60*scale, 29*scale), outline=(50,215,255,240), width=2*scale)
        return
    # Compact FPS target reticle around the blade tip.
    color=(100,232,255,245)
    w=max(1,scale)
    draw.arc((39*scale,8*scale,59*scale,28*scale), 205, 335, fill=color, width=w)
    draw.arc((39*scale,8*scale,59*scale,28*scale), 25, 155, fill=color, width=w)
    for x,y,dx,dy in [(43,9,1,3),(56,9,-1,3),(43,27,1,-3),(56,27,-1,-3)]:
        draw.line((x*scale,y*scale,(x+dx)*scale,(y+dy)*scale), fill=color, width=w)
    draw.ellipse((48*scale,16*scale,50*scale,18*scale), fill=(230,255,255,255))


def icon_s2(draw, scale, glow=False):
    angles = [-0.64, -0.32, 0, 0.32, 0.64]
    for i, a in enumerate(angles):
        dagger(draw, scale, 32 + (i-2)*2.2, 39 - abs(i-2)*1.3, 24 if i != 2 else 29,
               -math.pi/2 + a, glow=glow, thickness=.8 if i else 1.0)
    if glow:
        draw.arc((10*scale,9*scale,54*scale,56*scale),205,335,fill=(61,218,255,220),width=2*scale)
        return
    draw.arc((9*scale,8*scale,55*scale,58*scale), 205, 335, fill=(89,221,255,220), width=max(1,scale))
    for x in (17, 47):
        draw.ellipse((x*scale-1, 48*scale-1, x*scale+1, 48*scale+1), fill=(240,250,255,245))


def icon_ult(draw, scale, glow=False):
    if glow:
        draw.arc((8*scale, 5*scale, 57*scale, 59*scale), 220, 130, fill=(61,220,255,245), width=5*scale)
        draw.arc((14*scale, 12*scale, 51*scale, 52*scale), 35, 285, fill=(179,244,255,210), width=3*scale)
        return
    # Twin blink slashes form a clean, luminous loop; no character or weapon duplication.
    draw.arc((8*scale, 5*scale, 57*scale, 59*scale), 215, 45, fill=(67,207,241,255), width=2*scale)
    draw.arc((12*scale, 9*scale, 53*scale, 55*scale), 35, 225, fill=(225,251,255,255), width=2*scale)
    draw.line(scaled_xy([(17,47),(26,38),(34,31),(44,20),(51,13)]), fill=(124,238,255,255), width=2*scale)
    draw.line(scaled_xy([(14,24),(20,19),(28,14)]), fill=(235,250,255,240), width=max(1,scale))
    for x,y,r in [(14,24,1.5),(51,13,1.6),(31,31,1.1)]:
        draw.ellipse(((x-r)*scale,(y-r)*scale,(x+r)*scale,(y+r)*scale), fill=(255,246,198,255))


def make_sfx(filename: str, duration: float, components, seed: int) -> None:
    count = int(SAMPLE_RATE * duration)
    rng = random.Random(seed)
    samples = []
    noise = 0.0
    for i in range(count):
        t = i / SAMPLE_RATE
        x = t / duration
        value = 0.0
        for amp, f0, f1, phase, exponent in components:
            env = max(0.0, 1.0 - x) ** exponent
            freq = f0 + (f1 - f0) * x
            value += amp * env * math.sin(2 * math.pi * freq * t + phase)
        # Smooth, filtered air/noise adds a soft blade-whoosh rather than harsh static.
        noise = noise * 0.76 + rng.uniform(-1.0, 1.0) * 0.24
        value += 0.035 * noise * (max(0.0, 1.0 - x) ** 1.5)
        # short attack click, very low amplitude
        if x < 0.025:
            value += 0.045 * (1.0 - x / 0.025) * math.sin(2 * math.pi * 2600 * t)
        value = max(-0.92, min(0.92, value))
        samples.append(int(value * 32767))
    peak = max((abs(v) for v in samples), default=1)
    gain = min(4.0, 27_000 / peak) if peak else 1.0
    samples = [max(-30_000, min(30_000, round(v * gain))) for v in samples]
    path = ROOT / "sound" / "sfx" / filename
    with wave.open(str(path), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(SAMPLE_RATE)
        f.writeframes(b"".join(int(v).to_bytes(2, "little", signed=True) for v in samples))


def make_sound_info(stem: str, volume: float) -> None:
    data = {"plays": [{"delay": 0.0, "clip": stem, "volume": volume}]}
    (ROOT / "sound" / "sfx" / f"{stem}.sound_info").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    (ROOT / "icons").mkdir(exist_ok=True)
    make_icon("gusion_skill.png", icon_s1)
    make_icon("gusion_skill2.png", icon_s2)
    make_icon("gusion_ult.png", icon_ult)

    specs = {
        "mlbb_gusion_sfx_attack": (.20, [(0.21, 1800, 520, .2, .7), (0.08, 410, 170, 0, 1.2)], .55, 11),
        "mlbb_gusion_sfx_s1_throw": (.28, [(0.20, 1550, 420, .7, .8), (0.07, 2300, 950, 0, 2.0)], .64, 13),
        "mlbb_gusion_sfx_s1_recast": (.30, [(0.18, 550, 1750, .1, .8), (0.10, 980, 320, .4, 1.2)], .62, 17),
        "mlbb_gusion_sfx_s2_throw": (.38, [(0.13, 1650, 620, .4, .7), (0.10, 1250, 470, 1.1, .9), (0.06, 820, 300, 2.0, 1.2)], .58, 19),
        "mlbb_gusion_sfx_s2_recall": (.36, [(0.14, 520, 1450, .3, .8), (0.08, 720, 1650, 1.4, 1.0), (0.06, 930, 1900, 2.1, 1.2)], .58, 23),
        "mlbb_gusion_sfx_ult": (.42, [(0.13, 980, 280, .4, .8), (0.08, 1450, 500, 1.2, .8), (0.06, 180, 80, 0, 1.0)], .68, 29),
        "mlbb_gusion_sfx_ult_recast": (.34, [(0.16, 2100, 650, .6, .7), (0.09, 400, 1100, 1.1, .8)], .62, 31),
    }
    for stem, (duration, components, volume, seed) in specs.items():
        make_sfx(stem + ".wav", duration, components, seed)
        make_sound_info(stem, volume)

    # Preserve the existing glyph artwork exactly while changing the frame's
    # origin so the visible mark/rune sits above the head rather than the face.
    for base in ("gusion_passive_rune", "gusion_s1_mark"):
        path = ROOT / "effects" / f"{base}#sheet.png"
        with Image.open(path) as src:
            rgba = src.convert("RGBA")
            canvas = Image.new("RGBA", (rgba.width, 64), (0, 0, 0, 0))
            canvas.alpha_composite(rgba, (0, 0))
            canvas.save(path, optimize=True)
        anim_path = ROOT / "effects" / f"{base}#anim.fanim"
        anim = json.loads(anim_path.read_text(encoding="utf-8"))
        anim["anims"]["effect"]["frames"][0]["data"]["w"] = rgba.width
        anim["anims"]["effect"]["frames"][0]["data"]["h"] = 64
        anim["anchor_x"] = 0.5
        anim["anchor_y"] = 1.0
        anim_path.write_text(json.dumps(anim, indent=2) + "\n", encoding="utf-8")

    # Uniform frame cadence removes the rapid 13 ms cadence changes that made
    # this 17-frame, 1/3-second attack sequence visibly stutter.
    anim_path = ROOT / "sprite" / "gusion#anim.fanim"
    anim = json.loads(anim_path.read_text(encoding="utf-8"))
    frames = anim["anims"]["attack"]["frames"]
    frame_duration = (1 / 3) / len(frames)
    for frame in frames:
        frame["duration"] = round(frame_duration, 8)
    anim_path.write_text(json.dumps(anim, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
