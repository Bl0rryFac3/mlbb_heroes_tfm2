#!/usr/bin/env python3
"""Build an Estes-style, single-stage Gusion animation/VFX showcase GIF.

All hero/VFX artwork is cropped from the active mod project assets. Stage motion
and timing are illustrative; this is not an in-game capture.
"""
from pathlib import Path
import math
from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUTS = [
    Path('/home/user/Gusion_Estes_Style_Showcase.gif'),
    ROOT / 'previews/gusion/gusion_estes_style_showcase.gif',
]
W, H = 640, 360
FPS_MS = 70
BASELINE = 282
HERO_HOME = 222
TARGET_X = 500
SCALE = 2.38

# Fonts
FONT_REG = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
FONT_BOLD = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
FONT_TITLE = ImageFont.truetype(FONT_BOLD, 21)
FONT_STATE = ImageFont.truetype(FONT_BOLD, 20)
FONT_SUB = ImageFont.truetype(FONT_REG, 8)
FONT_FOOT = ImageFont.truetype(FONT_BOLD, 7)

# Current project sprite sheets (transparent PNGs)
hero_sheet = Image.open(ROOT / 'sprite/gusion#sheet.png').convert('RGBA')

def effect(name):
    return Image.open(ROOT / 'effects' / name).convert('RGBA')

rune = effect('gusion_passive_rune#sheet.png')
mark = effect('gusion_s1_mark#sheet.png')
projectile = effect('gusion_s1_projectile#sheet.png')
s2_streak = effect('gusion_s2_streak#sheet.png')
dash_trail = effect('gusion_dash_trail#sheet.png')
ult_arrival = effect('gusion_ult_arrival#sheet.png')
empowered_hit = effect('gusion_empowered_hit#sheet.png')

# Crop non-transparent artwork out of the effect frames while preserving alpha.
def trim(im):
    box = im.getchannel('A').getbbox()
    return im.crop(box) if box else im

rune, mark, projectile = map(trim, (rune, mark, projectile))
s2_streak, dash_trail = map(trim, (s2_streak, dash_trail))
ult_arrival, empowered_hit = map(trim, (ult_arrival, empowered_hit))

# Atlas rows: idle=0, run=1, attack=2, S1=3, S1 recast=4, S2=5,
# S2 return=6, Ultimate=7, hit=8, death=9.
def hero_cell(row, col):
    return hero_sheet.crop((col * 72, row * 72, (col + 1) * 72, (row + 1) * 72))


def paste_center(canvas, im, cx, cy, scale=1.0, alpha=255):
    if scale != 1.0:
        im = im.resize((max(1, round(im.width * scale)), max(1, round(im.height * scale))), Image.Resampling.NEAREST)
    if alpha != 255:
        im = im.copy()
        im.putalpha(im.getchannel('A').point(lambda a: a * alpha // 255))
    canvas.alpha_composite(im, (round(cx - im.width / 2), round(cy - im.height / 2)))


def paste_hero(canvas, row, col, cx, ground=BASELINE, alpha=255):
    im = hero_cell(row, col).resize((round(72 * SCALE), round(72 * SCALE)), Image.Resampling.NEAREST)
    if alpha != 255:
        im.putalpha(im.getchannel('A').point(lambda a: a * alpha // 255))
    x = round(cx - 36 * SCALE)
    y = round(ground - 55 * SCALE)
    canvas.alpha_composite(im, (x, y))


def smooth(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def stage_background():
    im = Image.new('RGBA', (W, H), (5, 13, 24, 255))
    px = im.load()
    for y in range(H):
        t = y / (H - 1)
        col = (5 + int(8 * t), 13 + int(11 * t), 24 + int(17 * t), 255)
        for x in range(W):
            px[x, y] = col
    d = ImageDraw.Draw(im, 'RGBA')
    # Restrained diagonal arena grid
    for x in range(-210, 850, 64):
        d.line((x, 72, x + 178, 284), fill=(31, 52, 74, 28), width=1)
    for y in (117, 164, 211, 258):
        d.line((0, y, W, y), fill=(29, 48, 68, 25), width=1)
    # Stage glow and floor plane
    glow = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow, 'RGBA')
    gd.ellipse((65, 239, 592, 335), fill=(16, 37, 62, 95))
    glow = glow.filter(ImageFilter.GaussianBlur(20))
    im.alpha_composite(glow)
    d = ImageDraw.Draw(im, 'RGBA')
    d.ellipse((48, 253, 592, 321), fill=(5, 13, 26, 220), outline=(31, 50, 70, 70), width=1)
    d.line((20, BASELINE, 620, BASELINE), fill=(151, 176, 198, 110), width=1)
    d.line((20, BASELINE + 2, 620, BASELINE + 2), fill=(25, 43, 62, 190), width=1)
    return im


def draw_target(canvas, flash=0.0, marked=False):
    # Target dummy inspired by the pink training-marker silhouette in the Estes reference.
    lay = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay, 'RGBA')
    col = (238, 82, 154, 225)
    if flash > 0:
        col = (180 + int(75 * flash), 220 + int(30 * flash), 255, 235)
        d.ellipse((TARGET_X - 35, 159, TARGET_X + 35, 267), fill=(91, 213, 255, int(60 * flash)))
    # overhead sensor/bar, circular head and thin stick body
    d.rounded_rectangle((TARGET_X - 24, 185, TARGET_X + 24, 192), radius=3, outline=col, width=1)
    d.ellipse((TARGET_X - 10, 190, TARGET_X + 10, 210), outline=col, width=2)
    d.line((TARGET_X, 210, TARGET_X, 239), fill=col, width=3)
    d.line((TARGET_X, 217, TARGET_X - 21, 235), fill=col, width=3)
    d.line((TARGET_X, 217, TARGET_X + 21, 235), fill=col, width=3)
    d.line((TARGET_X, 239, TARGET_X - 15, 276), fill=col, width=3)
    d.line((TARGET_X, 239, TARGET_X + 15, 276), fill=col, width=3)
    if marked:
        d.ellipse((TARGET_X - 19, 210, TARGET_X + 19, 248), outline=(117, 223, 255, 220), width=2)
        d.ellipse((TARGET_X - 25, 204, TARGET_X + 25, 254), outline=(117, 223, 255, 115), width=1)
    canvas.alpha_composite(lay)


def draw_header_footer(canvas, state, count, footer, progress):
    d = ImageDraw.Draw(canvas, 'RGBA')
    # Main inset title bar, matching reference layout
    d.rounded_rectangle((18, 14, 622, 70), radius=14, fill=(5, 12, 23, 220), outline=(102, 122, 143, 95), width=1)
    d.text((32, 20), 'GUSION', font=FONT_TITLE, fill=(240, 190, 112, 255), stroke_width=0)
    d.text((32, 48), 'MOD ASSETS  /  ANIMATION + VFX SHOWCASE', font=FONT_SUB, fill=(184, 198, 213, 235))
    # top-right state/count
    box = d.textbbox((0, 0), state, font=FONT_STATE)
    d.text((606 - (box[2] - box[0]), 19), state, font=FONT_STATE, fill=(230, 237, 246, 255))
    count_text = f'{count:02d} / 07'
    cb = d.textbbox((0, 0), count_text, font=FONT_FOOT)
    d.text((606 - (cb[2] - cb[0]), 48), count_text, font=FONT_FOOT, fill=(184, 198, 213, 235))
    # lower timeline and footnotes
    d.rounded_rectangle((20, 334, 620, 341), radius=3, fill=(52, 68, 85, 220))
    d.rounded_rectangle((20, 334, 20 + int(600 * max(0.025, progress)), 341), radius=3, fill=(132, 202, 226, 220))
    d.text((22, 346), footer.upper(), font=FONT_FOOT, fill=(180, 195, 211, 230))
    right = 'STAGED PREVIEW • NOT AN IN-GAME CAPTURE'
    rb = d.textbbox((0, 0), right, font=FONT_FOOT)
    d.text((618 - (rb[2] - rb[0]), 346), right, font=FONT_FOOT, fill=(158, 177, 195, 215))


def draw_shadow(canvas, cx, strength=1.0):
    d = ImageDraw.Draw(canvas, 'RGBA')
    a = int(95 * strength)
    d.ellipse((cx - 27, BASELINE - 5, cx + 27, BASELINE + 3), fill=(2, 7, 14, a))


def make_frame(state, local_i, local_n, global_i, total_n):
    t = local_i / max(1, local_n - 1)
    canv = stage_background()
    # target/arena props under the hero and effects
    draw_shadow(canv, TARGET_X, 0.8)
    mark_active = state in ('S1', 'RECAST') and (state == 'RECAST' or t > 0.58)
    hit = smooth(max(0.0, min(1.0, (t - 0.52) / 0.2))) if state == 'EMPOWERED' else 0.0
    draw_target(canv, flash=hit, marked=mark_active)

    # Positions and animation rows for each beat.
    hx = HERO_HOME
    row, col = 0, int(local_i * 0.45) % 6
    if state == 'IDLE':
        row, col = 0, (local_i // 2) % 6
    elif state == 'S1':
        row, col = 3, min(11, round(t * 11))
    elif state == 'RECAST':
        row, col = 4, min(12, round(t * 12))
        hx = HERO_HOME + (TARGET_X - HERO_HOME) * smooth(t)
    elif state == 'S2':
        row, col = 5 if t < .68 else 6, min(15 if t < .68 else 11, round((t / .68 if t < .68 else (t - .68) / .32) * (15 if t < .68 else 11)))
    elif state == 'ULT':
        row, col = 7, min(17, round(t * 17))
        if t < 0.47:
            hx = HERO_HOME + (TARGET_X - HERO_HOME) * smooth(t / 0.47)
        else:
            hx = TARGET_X - (TARGET_X - HERO_HOME) * smooth((t - 0.47) / 0.53)
    elif state == 'EMPOWERED':
        row, col = (2 if t < .82 else 8), min(16 if t < .82 else 8, round((t / .82 if t < .82 else (t - .82) / .18) * (16 if t < .82 else 8)))
        hx = HERO_HOME + (TARGET_X - 92 - HERO_HOME) * smooth(max(0.0, min(1.0, (t - .08) / .45)))
    elif state == 'RESET':
        row, col = 0, (local_i // 2) % 6
        hx = TARGET_X - (TARGET_X - HERO_HOME) * smooth(t)

    draw_shadow(canv, hx, 1.0)

    # Passive rune floats over Gusion in idle and empowers; subtle in reset.
    if state in ('IDLE', 'EMPOWERED', 'RESET'):
        bob = math.sin(local_i * 0.75) * 2.0
        paste_center(canv, rune, hx, BASELINE - 112 + bob, 1.42, 230 if state != 'RESET' else 145)

    if state == 'S1':
        # A single bright dagger travels to the target; target mark activates at impact.
        proj_x = HERO_HOME + 37 + (TARGET_X - HERO_HOME - 54) * smooth(t)
        proj_y = BASELINE - 72 - 5 * math.sin(t * math.pi)
        paste_center(canv, projectile, proj_x, proj_y, 1.95, 250)
        if t > .60:
            pulse = 1.0 + 0.08 * math.sin(local_i * 1.1)
            paste_center(canv, mark, TARGET_X, 225, 1.18 * pulse, 235)
    elif state == 'RECAST':
        # Leave a cyan streak behind the recast dash; marked ring dissolves at arrival.
        move = smooth(t)
        trail = dash_trail if TARGET_X >= HERO_HOME else dash_trail.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        paste_center(canv, trail, hx - 64, BASELINE - 58, 0.72, int(215 * (1.0 - .28 * t)))
        if t < .76:
            paste_center(canv, mark, TARGET_X, 225, 1.2, int(235 * (1.0 - .35 * move)))
        else:
            paste_center(canv, ult_arrival, TARGET_X, BASELINE - 7, 1.2, int(210 * (1.0 - t) * 2.0))
    elif state == 'S2':
        # Five distinct daggers with separately drawn streaks; outward, then return.
        out_phase = t < .65
        u = smooth(t / .65) if out_phase else smooth((t - .65) / .35)
        offsets = [-28, -14, 0, 14, 28]
        for j, oy in enumerate(offsets):
            startx = HERO_HOME + 30
            endx = TARGET_X - 18
            x = startx + (endx - startx) * u if out_phase else endx - (endx - startx) * u
            y = BASELINE - 67 + oy + (j - 2) * 5 * math.sin(u * math.pi)
            ang = [-18, -9, 0, 9, 18][j]
            dagger = projectile.resize((round(projectile.width * 1.32), round(projectile.height * 1.32)), Image.Resampling.NEAREST)
            streak = s2_streak.resize((round(s2_streak.width * 1.22), round(s2_streak.height * 1.22)), Image.Resampling.NEAREST)
            if not out_phase:
                dagger = dagger.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
                streak = streak.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
                ang = -ang
            dagger = dagger.rotate(ang, resample=Image.Resampling.NEAREST, expand=True)
            streak = streak.rotate(ang, resample=Image.Resampling.NEAREST, expand=True)
            trail_cx = x - 29 if out_phase else x + 29
            paste_center(canv, streak, trail_cx, y, 1.0, 190)
            paste_center(canv, dagger, x, y, 1.0, 250)
    elif state == 'ULT':
        # Two readable blink/dash arrivals, one on each side of the dummy.
        if local_i in (3, 4, 10, 11, 12):
            trail_cx = hx - 62 if (local_i < 8) else hx + 62
            tr = dash_trail if local_i < 8 else dash_trail.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
            paste_center(canv, tr, trail_cx, BASELINE - 53, 0.7, 175)
        arrival = (local_i in (5, 6, 13, 14, 15))
        if arrival:
            paste_center(canv, ult_arrival, hx, BASELINE - 7, 1.45, 225)
    elif state == 'EMPOWERED':
        if t > .50:
            pulse = 1.0 + .12 * math.sin(local_i * 1.25)
            paste_center(canv, empowered_hit, TARGET_X - 18, BASELINE - 62, 1.5 * pulse, int(235 * min(1.0, hit + .25)))
        if .36 < t < .82:
            paste_center(canv, dash_trail, hx - 40, BASELINE - 53, .48, 150)

    # The actual Gusion sprite remains the focal point, anchored to the floor.
    paste_hero(canv, row, col, hx, BASELINE)

    # Re-layer impact VFX over the hero if the arrival crosses the target.
    if state == 'ULT' and local_i in (5, 6, 13, 14, 15):
        paste_center(canv, ult_arrival, hx, BASELINE - 7, 1.32, 120)
    if state == 'EMPOWERED' and t > .53:
        paste_center(canv, empowered_hit, TARGET_X - 18, BASELINE - 62, 1.28, int(160 * hit))

    state_names = {
        'IDLE': ('IDLE', 'PASSIVE RUNE  •  DAGGER SPECIALIST'),
        'S1': ('SWORD SPIKE', 'SKILL 1  •  HOMING DAGGER + TARGET MARK'),
        'RECAST': ('RECAST', 'SWORD SPIKE  •  MARKED-TARGET DASH'),
        'S2': ('SHADOWBLADE', 'SKILL 2  •  FIVE DAGGERS OUT / RETURN'),
        'ULT': ('INCANDESCENCE', 'ULTIMATE  •  TWO BLINKS + ARRIVAL VFX'),
        'EMPOWERED': ('PASSIVE PROC', 'EMPOWERED HIT  •  RUNE-CHARGED STRIKE'),
        'RESET': ('IDLE', 'PASSIVE RUNE  •  RESET TO LOOP'),
    }
    title, footer = state_names[state]
    draw_header_footer(canv, title, state_index[state], footer, (global_i + 1) / total_n)
    return canv.convert('RGB')


state_index = {'IDLE': 1, 'S1': 2, 'RECAST': 3, 'S2': 4, 'ULT': 5, 'EMPOWERED': 6, 'RESET': 7}
beats = [
    ('IDLE', 12), ('S1', 12), ('RECAST', 12), ('S2', 16),
    ('ULT', 16), ('EMPOWERED', 14), ('RESET', 8),
]
total_frames = sum(n for _, n in beats)
frames = []
idx = 0
for name, count in beats:
    for local in range(count):
        frames.append(make_frame(name, local, count, idx, total_frames))
        idx += 1

# Quantize to one shared palette for stable looping colors and compact file size.
palette = frames[0].convert('P', palette=Image.Palette.ADAPTIVE, colors=256)
pal_frames = [fr.convert('RGB').quantize(palette=palette, dither=Image.Dither.NONE) for fr in frames]
for out in OUTS:
    out.parent.mkdir(parents=True, exist_ok=True)
    pal_frames[0].save(
        out, save_all=True, append_images=pal_frames[1:], duration=FPS_MS,
        loop=0, disposal=2, optimize=False,
    )
    print(f'Wrote {out} ({out.stat().st_size:,} bytes; {total_frames} frames)')
