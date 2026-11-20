#!/usr/bin/env python3
"""Build a seven-beat Karrie animation/VFX showcase GIF in the supplied Estes style.

The character and effect frames are cropped from the current project atlases. The
stage and timings are illustrative; this is a staged preview, not an in-game capture.
"""
from __future__ import annotations

import json
import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUTS = [Path('/home/user/Karrie_Showcase.gif'), ROOT / 'previews/karrie/karrie_showcase.gif']
W, H = 640, 360
BASELINE = 282
HERO_HOME = 210
TARGET_X = 500
HERO_SCALE = 2.28
FRAME_MS = 80

FONT_REG = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
FONT_BOLD = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
FONT_TITLE = ImageFont.truetype(FONT_BOLD, 21)
FONT_STATE = ImageFont.truetype(FONT_BOLD, 19)
FONT_SUB = ImageFont.truetype(FONT_REG, 8)
FONT_FOOT = ImageFont.truetype(FONT_BOLD, 7)


def load_anim(sheet_name: str, anim_name: str, tag: str) -> list[Image.Image]:
    sheet = Image.open(ROOT / sheet_name).convert('RGBA')
    data = json.loads((ROOT / anim_name).read_text())['anims'][tag]['frames']
    return [sheet.crop((f['data']['x'], f['data']['y'], f['data']['x'] + f['data']['w'], f['data']['y'] + f['data']['h'])) for f in data]


def trim(im: Image.Image) -> Image.Image:
    box = im.getchannel('A').getbbox()
    return im.crop(box) if box else im


def fade(im: Image.Image, alpha: int) -> Image.Image:
    im = im.copy()
    im.putalpha(im.getchannel('A').point(lambda a: a * max(0, min(255, alpha)) // 255))
    return im


def paste_center(canvas: Image.Image, im: Image.Image, cx: float, cy: float, scale: float = 1.0, alpha: int = 255, flip: bool = False) -> None:
    if flip:
        im = im.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    if scale != 1.0:
        im = im.resize((max(1, round(im.width * scale)), max(1, round(im.height * scale))), Image.Resampling.NEAREST)
    if alpha != 255:
        im = fade(im, alpha)
    canvas.alpha_composite(im, (round(cx - im.width / 2), round(cy - im.height / 2)))


def smooth(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def stage_background() -> Image.Image:
    im = Image.new('RGBA', (W, H), (6, 14, 27, 255))
    px = im.load()
    for y in range(H):
        t = y / (H - 1)
        col = (6 + int(7 * t), 14 + int(11 * t), 27 + int(18 * t), 255)
        for x in range(W):
            px[x, y] = col
    d = ImageDraw.Draw(im, 'RGBA')
    for x in range(-210, 850, 64):
        d.line((x, 72, x + 178, 284), fill=(31, 52, 74, 28), width=1)
    for y in (117, 164, 211, 258):
        d.line((0, y, W, y), fill=(29, 48, 68, 25), width=1)
    glow = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow, 'RGBA')
    gd.ellipse((65, 239, 592, 335), fill=(22, 30, 73, 105))
    im.alpha_composite(glow.filter(ImageFilter.GaussianBlur(20)))
    d = ImageDraw.Draw(im, 'RGBA')
    d.ellipse((48, 253, 592, 321), fill=(5, 13, 26, 225), outline=(49, 55, 86, 82), width=1)
    d.line((20, BASELINE, 620, BASELINE), fill=(151, 176, 198, 110), width=1)
    d.line((20, BASELINE + 2, 620, BASELINE + 2), fill=(25, 43, 62, 190), width=1)
    return im


BACKGROUND = stage_background()
hero_anims = {name: load_anim('sprite/karrie#sheet.png', 'sprite/karrie#anim.fanim', name)
              for name in ('idle', 'run', 'attack', 'skill', 'skill2', 'ult', 'hit', 'dead')}
wheel_frames = [trim(f) for f in load_anim('effects/karrie_basic_lightwheel#sheet.png', 'effects/karrie_basic_lightwheel#anim.fanim', 'basic_projectile')]
s1_projectile = [trim(f) for f in load_anim('effects/karrie_skill1_projectile#sheet.png', 'effects/karrie_skill1_projectile#anim.fanim', 'effect')]
s1_impact = [trim(f) for f in load_anim('effects/karrie_skill1_impact#sheet.png', 'effects/karrie_skill1_impact#anim.fanim', 'effect')]
ult_pulse = [trim(f) for f in load_anim('effects/karrie_ult_activation_pulse#sheet.png', 'effects/karrie_ult_activation_pulse#anim.fanim', 'effect')]
passive_proc = [trim(f) for f in load_anim('effects/karrie_passive_proc#sheet.png', 'effects/karrie_passive_proc#anim.fanim', 'effect')]
mark_frames = [trim(f) for i in range(1, 7) for f in load_anim('effects/karrie_passive_marks_head_smaller#sheet.png', 'effects/karrie_passive_marks_head_smaller#anim.fanim', f'mark_{i}')]
slow_zone = trim(load_anim('effects/karrie_skill1_slow_zone_prominent#sheet.png', 'effects/karrie_skill1_slow_zone_prominent#anim.fanim', 'zone')[0])
# Flatten the orb-like source art into a ground-plane field so the GIF reads as lingering area control.
slow_zone_flat = slow_zone.resize((144, 48), Image.Resampling.LANCZOS)


def draw_shadow(canvas: Image.Image, cx: float, strength: float = 1.0) -> None:
    d = ImageDraw.Draw(canvas, 'RGBA')
    d.ellipse((cx - 37, BASELINE - 5, cx + 37, BASELINE + 4), fill=(2, 7, 14, int(95 * strength)))


def draw_slow_zone(canvas: Image.Image, t: float) -> None:
    """Draw a stable, ground-plane slow field that remains after the impact flash."""
    progress = max(0.0, min(1.0, (t - .42) / .58))
    pulse = .76 + .24 * math.sin(progress * math.pi * 4.0)
    cx, cy = TARGET_X, BASELINE - 7

    glow = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow, 'RGBA')
    gd.ellipse((cx - 82, cy - 19, cx + 82, cy + 11), fill=(127, 77, 255, int(58 * pulse)))
    canvas.alpha_composite(glow.filter(ImageFilter.GaussianBlur(9)))

    field = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(field, 'RGBA')
    d.ellipse((cx - 75, cy - 14, cx + 75, cy + 9), fill=(98, 55, 203, int(48 * pulse)), outline=(173, 132, 255, int(220 * pulse)), width=2)
    d.ellipse((cx - 57, cy - 10, cx + 57, cy + 6), outline=(206, 180, 255, int(165 * pulse)), width=1)
    d.arc((cx - 68, cy - 12, cx + 68, cy + 8), start=190, end=350, fill=(229, 213, 255, int(190 * pulse)), width=2)
    canvas.alpha_composite(field)

    # The enlarged game zone is laid flat on the arena; slow motes keep it visibly active.
    art = fade(slow_zone_flat, int(128 * pulse))
    canvas.alpha_composite(art, (round(cx - art.width / 2), round(cy - art.height / 2)))
    motes = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    md = ImageDraw.Draw(motes, 'RGBA')
    phase = progress * math.pi * 2.0
    for j in range(4):
        angle = phase + j * math.pi / 2
        x = cx + math.cos(angle) * 47
        y = cy - 2 + math.sin(angle) * 8
        r = 2 if j % 2 else 3
        md.ellipse((x-r, y-r, x+r, y+r), fill=(232, 214, 255, int(225 * pulse)), outline=(154, 111, 255, 220))
    canvas.alpha_composite(motes)


def draw_target(canvas: Image.Image, flash: float = 0.0, marks: int = 0, slow: bool = False) -> None:
    # Pink training silhouette and overhead marker, matching the reference preview.
    lay = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay, 'RGBA')
    col = (239, 83, 157, 225)
    if flash > 0:
        col = (210, 225, 255, 245)
        d.ellipse((TARGET_X - 42, 158, TARGET_X + 42, 272), fill=(118, 72, 255, int(55 * flash)))
    d.rounded_rectangle((TARGET_X - 24, 185, TARGET_X + 24, 192), radius=3, outline=col, width=1)
    d.ellipse((TARGET_X - 10, 190, TARGET_X + 10, 210), outline=col, width=2)
    d.line((TARGET_X, 210, TARGET_X, 239), fill=col, width=3)
    d.line((TARGET_X, 217, TARGET_X - 21, 235), fill=col, width=3)
    d.line((TARGET_X, 217, TARGET_X + 21, 235), fill=col, width=3)
    d.line((TARGET_X, 239, TARGET_X - 15, 276), fill=col, width=3)
    d.line((TARGET_X, 239, TARGET_X + 15, 276), fill=col, width=3)
    if slow:
        d.ellipse((TARGET_X - 38, 258, TARGET_X + 38, 269), outline=(171, 111, 255, 220), width=2)
        d.ellipse((TARGET_X - 49, 254, TARGET_X + 49, 273), outline=(132, 103, 255, 115), width=1)
    canvas.alpha_composite(lay)
    if marks:
        # Five pips suggest the Lightwheel mark stack without obscuring the sprite.
        d = ImageDraw.Draw(canvas, 'RGBA')
        for i in range(5):
            x = TARGET_X - 18 + i * 9
            active = i < marks
            d.ellipse((x - 2, 172, x + 3, 177), fill=(211, 176, 255, 245 if active else 58), outline=(222, 203, 255, 200 if active else 90), width=1)


def paste_hero(canvas: Image.Image, anim: str, frame_i: int, cx: float, ground: float = BASELINE, alpha: int = 255) -> None:
    frame = hero_anims[anim][frame_i % len(hero_anims[anim])]
    im = frame.resize((round(72 * HERO_SCALE), round(72 * HERO_SCALE)), Image.Resampling.NEAREST)
    if alpha != 255:
        im = fade(im, alpha)
    canvas.alpha_composite(im, (round(cx - 36 * HERO_SCALE), round(ground - 55 * HERO_SCALE)))


def draw_header_footer(canvas: Image.Image, state: str, count: int, footer: str, progress: float) -> None:
    d = ImageDraw.Draw(canvas, 'RGBA')
    d.rounded_rectangle((18, 14, 622, 70), radius=14, fill=(5, 12, 23, 225), outline=(102, 122, 143, 95), width=1)
    d.text((32, 20), 'KARRIE', font=FONT_TITLE, fill=(241, 191, 116, 255))
    d.text((32, 48), 'TFM2  /  ANIMATION + VFX SHOWCASE', font=FONT_SUB, fill=(184, 198, 213, 235))
    box = d.textbbox((0, 0), state, font=FONT_STATE)
    d.text((606 - (box[2] - box[0]), 19), state, font=FONT_STATE, fill=(230, 237, 246, 255))
    count_text = f'{count:02d} / 08'
    cb = d.textbbox((0, 0), count_text, font=FONT_FOOT)
    d.text((606 - (cb[2] - cb[0]), 48), count_text, font=FONT_FOOT, fill=(184, 198, 213, 235))
    d.rounded_rectangle((20, 334, 620, 341), radius=3, fill=(52, 68, 85, 220))
    d.rounded_rectangle((20, 334, 20 + int(600 * max(0.025, progress)), 341), radius=3, fill=(171, 134, 239, 235))
    d.text((22, 346), footer.upper(), font=FONT_FOOT, fill=(180, 195, 211, 230))
    right = 'STAGED PREVIEW  •  NOT AN IN-GAME CAPTURE'
    rb = d.textbbox((0, 0), right, font=FONT_FOOT)
    d.text((618 - (rb[2] - rb[0]), 346), right, font=FONT_FOOT, fill=(158, 177, 195, 215))


def put_glow(canvas: Image.Image, x: float, y: float, radius: int, color: tuple[int, int, int], alpha: int) -> None:
    lay = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay, 'RGBA')
    d.ellipse((x - radius, y - radius, x + radius, y + radius), fill=(*color, alpha))
    canvas.alpha_composite(lay.filter(ImageFilter.GaussianBlur(max(2, radius // 2))))


def draw_dash_trail(canvas: Image.Image, x1: float, y1: float, x2: float, y2: float, t: float = 1.0) -> None:
    lay = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay, 'RGBA')
    color = (154, 108, 255, int(110 * t))
    d.line((x1, y1, x2, y2), fill=color, width=8)
    d.line((x1, y1, x2, y2), fill=(216, 194, 255, int(210 * t)), width=2)
    canvas.alpha_composite(lay.filter(ImageFilter.GaussianBlur(5)))
    canvas.alpha_composite(lay)


def make_frame(state: str, local_i: int, local_n: int, global_i: int, total_n: int) -> Image.Image:
    t = local_i / max(1, local_n - 1)
    canv = BACKGROUND.copy()
    hero_x = HERO_HOME
    target_flash = 0.0
    marks = 0
    slow = False

    # Persistent arena and target, with stage-specific hit/slow feedback.
    if state == 'BASIC':
        target_flash = smooth(max(0.0, min(1.0, (t - .58) / .2)))
        marks = 1 if t > .62 else 0
    elif state == 'S1':
        # Brief impact flash; the slow state remains visible after the burst fades.
        target_flash = max(0.0, 1.0 - abs(t - .46) / .14)
        slow = t >= .42
    elif state == 'S2':
        target_flash = smooth(max(0.0, min(1.0, (t - .68) / .18)))
        marks = 1 if t > .70 else 0
    elif state == 'ULT':
        target_flash = .55 * max(0.0, math.sin(t * math.pi * 2.3))
        marks = min(5, max(0, int(t * 7)))
    elif state == 'PASSIVE':
        target_flash = smooth(max(0.0, min(1.0, (t - .45) / .18)))
        marks = 5 if t < .47 else 0
    elif state == 'DEATH':
        target_flash = .12 * (1.0 - t)

    draw_shadow(canv, TARGET_X, .82)
    if state == 'S1' and t >= .42:
        draw_slow_zone(canv, t)
    draw_target(canv, target_flash, marks, slow)

    if state == 'IDLE':
        hero_frame = local_i % len(hero_anims['idle'])
        paste_hero(canv, 'idle', hero_frame, hero_x)
        # Gentle violet breathing aura.
        aura = .15 + .08 * math.sin(local_i * .65)
        put_glow(canv, hero_x, BASELINE - 65, 26, (142, 74, 255), int(28 + 35 * aura))
        paste_hero(canv, 'idle', hero_frame, hero_x)

    elif state == 'RUN':
        hero_x = HERO_HOME + 50 * smooth(t)
        hero_frame = local_i % len(hero_anims['run'])
        # Small pixel-like dust motes trail behind her feet.
        d = ImageDraw.Draw(canv, 'RGBA')
        for j in range(4):
            px = int(hero_x - 29 - ((local_i * 7 + j * 17) % 42))
            py = BASELINE - 3 - ((j * 7 + local_i * 3) % 13)
            d.rectangle((px, py, px + 3, py + 3), fill=(170, 139, 233, max(25, 130 - j * 22)))
        paste_hero(canv, 'run', hero_frame, hero_x)

    elif state == 'BASIC':
        hero_frame = min(len(hero_anims['attack']) - 1, round(t * (len(hero_anims['attack']) - 1)))
        paste_hero(canv, 'attack', hero_frame, hero_x)
        proj_t = smooth(max(0.0, min(1.0, (t - .16) / .60)))
        if .12 <= t <= .84:
            px = HERO_HOME + 40 + (TARGET_X - HERO_HOME - 52) * proj_t
            py = BASELINE - 65 - 4 * math.sin(t * math.pi * 2)
            wheel = wheel_frames[local_i % len(wheel_frames)]
            paste_center(canv, wheel, px, py, .42, 255)
        if t > .60:
            paste_center(canv, mark_frames[0], TARGET_X, 165, .26, 235)

    elif state == 'S1':
        cast_end = .40
        if t < cast_end:
            # Show Karrie clearly casting before the wheel reaches the target.
            hero_frame = min(len(hero_anims['skill']) - 1, round((t / cast_end) * (len(hero_anims['skill']) - 1)))
            put_glow(canv, HERO_HOME + 18, BASELINE - 80, 21, (173, 101, 255), int(45 + 70 * smooth(t / cast_end)))
            paste_hero(canv, 'skill', hero_frame, hero_x)
        else:
            # After impact, let the camera read the lingering field rather than stretching the cast pose.
            idle_frame = local_i % len(hero_anims['idle'])
            paste_hero(canv, 'idle', idle_frame, hero_x)

        if .10 <= t < cast_end:
            proj_t = smooth((t - .10) / (cast_end - .10))
            px = HERO_HOME + 42 + (TARGET_X - HERO_HOME - 45) * proj_t
            py = BASELINE - 72 - 5 * math.sin(proj_t * math.pi)
            proj = s1_projectile[local_i % len(s1_projectile)]
            paste_center(canv, proj, px, py, .34, 250)

        # A short burst marks the collision; the flatter, separate ground field persists afterward.
        if .40 <= t <= .56:
            impact_t = (t - .40) / .16
            impact = s1_impact[min(len(s1_impact) - 1, round(impact_t * (len(s1_impact) - 1)))]
            impact_alpha = int(235 * max(0.0, 1.0 - abs(t - .46) / .10))
            paste_center(canv, impact, TARGET_X - 4, BASELINE - 48, .34, impact_alpha)

    elif state == 'S2':
        hero_frame = min(len(hero_anims['skill2']) - 1, round(t * (len(hero_anims['skill2']) - 1)))
        dash_t = smooth(max(0.0, min(1.0, (t - .13) / .49)))
        if t < .72:
            hero_x = HERO_HOME + (TARGET_X - HERO_HOME - 88) * dash_t
            draw_dash_trail(canv, HERO_HOME + 15, BASELINE - 42, hero_x - 22, BASELINE - 42, max(.18, 1 - t * .75))
        else:
            hero_x = HERO_HOME + (TARGET_X - HERO_HOME - 88) * (1 - smooth((t - .72) / .28))
        paste_hero(canv, 'skill2', hero_frame, hero_x)
        if .56 < t < .90:
            wheel = wheel_frames[local_i % len(wheel_frames)]
            wx = hero_x + 46 + (TARGET_X - hero_x - 30) * smooth((t - .56) / .34)
            paste_center(canv, wheel, wx, BASELINE - 63, .36, 255)
        if t > .72:
            paste_center(canv, mark_frames[min(1, len(mark_frames) - 1)], TARGET_X, 164, .25, 225)

    elif state == 'ULT':
        hero_frame = min(len(hero_anims['ult']) - 1, round(t * (len(hero_anims['ult']) - 1)))
        paste_hero(canv, 'ult', hero_frame, hero_x)
        # The activation pulse blooms around Karrie; the mod retains a single lightwheel.
        pulse = ult_pulse[min(len(ult_pulse) - 1, round(t * (len(ult_pulse) - 1)))]
        if t < .64:
            paste_center(canv, pulse, hero_x + 12, BASELINE - 62, .45 + .15 * math.sin(t * math.pi), int(210 * min(1, t * 5 + .3)))
        if .35 < t < .88:
            wheel = wheel_frames[local_i % len(wheel_frames)]
            wx = hero_x + 55 + (TARGET_X - hero_x - 58) * smooth((t - .35) / .53)
            paste_center(canv, wheel, wx, BASELINE - 65 + 5 * math.sin(t * 8), .40, 255)
        # Three subtle afterimages convey the attack-speed surge, not extra projectiles.
        if .55 < t < .98:
            for j, dx in enumerate((-18, 0, 18)):
                paste_hero(canv, 'ult', max(0, hero_frame - (j + 1) * 2), hero_x + dx, alpha=42 - j * 8)

    elif state == 'PASSIVE':
        hero_frame = min(len(hero_anims['hit']) - 1, round(t * (len(hero_anims['hit']) - 1)))
        paste_hero(canv, 'hit', hero_frame, hero_x)
        # Five marks appear, then Karrie's Lightwheel proc bursts on the dummy.
        if t < .52:
            for j in range(5):
                if t > j * .075:
                    paste_center(canv, mark_frames[j], TARGET_X - 18 + j * 9, 164, .24, 235)
        if t > .34:
            proc = passive_proc[min(len(passive_proc) - 1, round((t - .34) / .66 * (len(passive_proc) - 1)))]
            pulse_alpha = int(240 * max(.25, 1 - max(0, t - .78) * 1.4))
            paste_center(canv, proc, TARGET_X - 10, BASELINE - 56, .40, pulse_alpha)

    elif state == 'DEATH':
        hero_frame = min(len(hero_anims['dead']) - 1, round(t * (len(hero_anims['dead']) - 1)))
        paste_hero(canv, 'dead', hero_frame, hero_x, alpha=max(130, int(255 - t * 70)))
        if t > .45:
            # Faint violet energy dissipates as the death animation settles.
            put_glow(canv, hero_x, BASELINE - 36, int(28 + 32 * t), (129, 80, 202), int(28 * (1 - t)))

    labels = {
        'IDLE': ('IDLE', 'LIGHTWHEEL MARKS  •  READY'),
        'RUN': ('RUN', 'MOBILITY  •  RUN ANIMATION'),
        'BASIC': ('BASIC ATTACK', 'LIGHTWHEEL  •  SINGLE PROJECTILE'),
        'S1': ('SKILL 1', 'SLOW ZONE  •  VISIBLE 1.5S LINGER'),
        'S2': ('SKILL 2', 'PHANTOM STEP  •  DASH + LIGHTWHEEL'),
        'ULT': ('ULTIMATE', 'SPEEDY LIGHTWHEEL  •  ATTACK-SPEED SURGE'),
        'PASSIVE': ('PASSIVE PROC', '5 MARKS  •  8–12% MAX-HP TRUE DAMAGE'),
        'DEATH': ('DEATH', 'KARRIE  •  DEATH ANIMATION'),
    }
    title, footer = labels[state]
    draw_header_footer(canv, title, state_index[state], footer, (global_i + 1) / total_n)
    return canv.convert('RGB')


state_index = {'IDLE': 1, 'RUN': 2, 'BASIC': 3, 'S1': 4, 'S2': 5, 'ULT': 6, 'PASSIVE': 7, 'DEATH': 8}
beats = [('IDLE', 10), ('RUN', 10), ('BASIC', 12), ('S1', 32), ('S2', 13), ('ULT', 14), ('PASSIVE', 13), ('DEATH', 14)]
total_frames = sum(count for _, count in beats)
frames: list[Image.Image] = []
idx = 0
for state, count in beats:
    for local in range(count):
        frames.append(make_frame(state, local, count, idx, total_frames))
        idx += 1

# Shared adaptive palette keeps the purple VFX and HUD stable throughout the loop.
palette = frames[0].convert('P', palette=Image.Palette.ADAPTIVE, colors=256)
pal_frames = [frame.quantize(palette=palette, dither=Image.Dither.NONE) for frame in frames]
for out in OUTS:
    out.parent.mkdir(parents=True, exist_ok=True)
    pal_frames[0].save(out, save_all=True, append_images=pal_frames[1:], duration=FRAME_MS,
                       loop=0, disposal=2, optimize=True)
    print(f'Wrote {out} ({out.stat().st_size:,} bytes; {total_frames} frames; {W}x{H})')
