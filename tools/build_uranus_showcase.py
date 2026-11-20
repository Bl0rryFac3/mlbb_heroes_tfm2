#!/usr/bin/env python3
"""Create an Estes-style staged Uranus showcase GIF from the mod's existing art.

The stage is an illustrative asset preview, not an in-game capture. Original
source sheets are only read; all animation frames remain unchanged in the pack.
"""
from pathlib import Path
import json
import math
from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUTS = [
    Path('/home/user/Uranus_Showcase.gif'),
    ROOT / 'previews/uranus/uranus_estes_style_showcase.gif',
]
W, H = 640, 360
FRAME_MS = 80
BASELINE = 282
HERO_HOME = 222
TARGET_X = 500
HERO_SCALE = 2.38
HERO_PIVOT_X, HERO_PIVOT_Y = 34, 55

FONT_DIR = Path('/usr/share/fonts/truetype/dejavu')
FONT_TITLE = ImageFont.truetype(str(FONT_DIR / 'DejaVuSans-Bold.ttf'), 21)
FONT_STATE = ImageFont.truetype(str(FONT_DIR / 'DejaVuSans-Bold.ttf'), 19)
FONT_SUB = ImageFont.truetype(str(FONT_DIR / 'DejaVuSans.ttf'), 8)
FONT_FOOT = ImageFont.truetype(str(FONT_DIR / 'DejaVuSans-Bold.ttf'), 7)
FONT_TAG = ImageFont.truetype(str(FONT_DIR / 'DejaVuSans-Bold.ttf'), 8)

sprite_sheet = Image.open(ROOT / 'sprite/uranus#sheet.png').convert('RGBA')
sprite_anim = json.loads((ROOT / 'sprite/uranus#anim.fanim').read_text())['anims']


def load_effect(name, anim):
    image = Image.open(ROOT / 'effects' / f'{name}#sheet.png').convert('RGBA')
    frames = json.loads((ROOT / 'effects' / f'{name}#anim.fanim').read_text())['anims'][anim]['frames']
    return [
        image.crop((f['data']['x'], f['data']['y'], f['data']['x'] + f['data']['w'], f['data']['y'] + f['data']['h']))
        for f in frames
    ]


orbit_frames = load_effect('uranus_skill1_orbit', 'orbit')
shield_frames = load_effect('uranus_skill2_shield', 'loop')
burst_frames = load_effect('uranus_skill2_burst', 'burst')
trail_frames = load_effect('uranus_skill2_dash_trail', 'trail')
aura_frames = load_effect('uranus_radiance_aura', 'loop')
flare_frames = load_effect('uranus_ultimate_effect', 'flare')
mark_frames = [load_effect('uranus_skill1_marks', f'mark_{i}') [0] for i in range(1, 5)]


def sprite_frame(anim_name, index):
    anim = sprite_anim[anim_name]
    frame = anim['frames'][index % len(anim['frames'])]['data']
    return sprite_sheet.crop((frame['x'], frame['y'], frame['x'] + frame['w'], frame['y'] + frame['h']))


def resize_nearest(im, scale):
    return im.resize((max(1, round(im.width * scale)), max(1, round(im.height * scale))), Image.Resampling.NEAREST)


def paste_center(canvas, im, cx, cy, scale=1.0, alpha=255):
    im = resize_nearest(im, scale) if scale != 1.0 else im
    if alpha != 255:
        im = im.copy()
        im.putalpha(im.getchannel('A').point(lambda a: a * alpha // 255))
    canvas.alpha_composite(im, (round(cx - im.width / 2), round(cy - im.height / 2)))


def hero_position(cx, ground=BASELINE):
    return round(cx - HERO_PIVOT_X * HERO_SCALE), round(ground - HERO_PIVOT_Y * HERO_SCALE)


def paste_hero(canvas, anim_name, index, cx, ground=BASELINE, alpha=255):
    im = resize_nearest(sprite_frame(anim_name, index), HERO_SCALE)
    if alpha != 255:
        im.putalpha(im.getchannel('A').point(lambda a: a * alpha // 255))
    canvas.alpha_composite(im, hero_position(cx, ground))


def smooth(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3 - 2 * t)


def stage_background():
    im = Image.new('RGBA', (W, H), (5, 13, 24, 255))
    px = im.load()
    for y in range(H):
        t = y / (H - 1)
        col = (5 + int(8 * t), 13 + int(11 * t), 24 + int(17 * t), 255)
        for x in range(W):
            px[x, y] = col
    d = ImageDraw.Draw(im, 'RGBA')
    for x in range(-210, 850, 64):
        d.line((x, 72, x + 178, 284), fill=(31, 52, 74, 28), width=1)
    for y in (117, 164, 211, 258):
        d.line((0, y, W, y), fill=(29, 48, 68, 25), width=1)
    glow = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow, 'RGBA')
    gd.ellipse((65, 239, 592, 335), fill=(16, 37, 62, 95))
    im.alpha_composite(glow.filter(ImageFilter.GaussianBlur(20)))
    d = ImageDraw.Draw(im, 'RGBA')
    d.ellipse((48, 253, 592, 321), fill=(5, 13, 26, 220), outline=(31, 50, 70, 70), width=1)
    d.line((20, BASELINE, 620, BASELINE), fill=(151, 176, 198, 110), width=1)
    d.line((20, BASELINE + 2, 620, BASELINE + 2), fill=(25, 43, 62, 190), width=1)
    return im


def draw_target(canvas, flash=0.0):
    lay = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay, 'RGBA')
    col = (238, 82, 154, 225)
    if flash > 0:
        col = (200 + int(55 * flash), 220 + int(30 * flash), 255, 235)
        d.ellipse((TARGET_X - 37, 156, TARGET_X + 37, 270), fill=(91, 213, 255, int(55 * flash)))
    d.rounded_rectangle((TARGET_X - 24, 185, TARGET_X + 24, 192), radius=3, outline=col, width=1)
    d.ellipse((TARGET_X - 10, 190, TARGET_X + 10, 210), outline=col, width=2)
    d.line((TARGET_X, 210, TARGET_X, 239), fill=col, width=3)
    d.line((TARGET_X, 217, TARGET_X - 21, 235), fill=col, width=3)
    d.line((TARGET_X, 217, TARGET_X + 21, 235), fill=col, width=3)
    d.line((TARGET_X, 239, TARGET_X - 15, 276), fill=col, width=3)
    d.line((TARGET_X, 239, TARGET_X + 15, 276), fill=col, width=3)
    canvas.alpha_composite(lay)


def draw_shadow(canvas, cx, strength=1.0):
    d = ImageDraw.Draw(canvas, 'RGBA')
    d.ellipse((cx - 31, BASELINE - 5, cx + 31, BASELINE + 3), fill=(2, 7, 14, int(92 * strength)))


def draw_tag(canvas, text, x, y, color=(174, 219, 167, 235)):
    d = ImageDraw.Draw(canvas, 'RGBA')
    box = d.textbbox((0, 0), text, font=FONT_TAG)
    tw, th = box[2] - box[0], box[3] - box[1]
    d.rounded_rectangle((x - tw / 2 - 7, y - 3, x + tw / 2 + 7, y + th + 5), radius=5,
                       fill=(5, 12, 23, 218), outline=(color[0], color[1], color[2], 135), width=1)
    d.text((x - tw / 2, y), text, font=FONT_TAG, fill=color)


def draw_header_footer(canvas, state, count, footer, progress):
    d = ImageDraw.Draw(canvas, 'RGBA')
    d.rounded_rectangle((18, 14, 622, 70), radius=14, fill=(5, 12, 23, 220), outline=(102, 122, 143, 95), width=1)
    d.text((32, 20), 'URANUS', font=FONT_TITLE, fill=(240, 190, 112, 255))
    d.text((32, 48), 'TFM2  /  ANIMATION + VFX SHOWCASE', font=FONT_SUB, fill=(184, 198, 213, 235))
    box = d.textbbox((0, 0), state, font=FONT_STATE)
    d.text((606 - (box[2] - box[0]), 19), state, font=FONT_STATE, fill=(230, 237, 246, 255))
    count_text = f'{count:02d} / 07'
    cb = d.textbbox((0, 0), count_text, font=FONT_FOOT)
    d.text((606 - (cb[2] - cb[0]), 48), count_text, font=FONT_FOOT, fill=(184, 198, 213, 235))
    d.rounded_rectangle((20, 334, 620, 341), radius=3, fill=(52, 68, 85, 220))
    d.rounded_rectangle((20, 334, 20 + int(600 * max(0.025, progress)), 341), radius=3, fill=(132, 202, 226, 220))
    d.text((22, 346), footer.upper(), font=FONT_FOOT, fill=(180, 195, 211, 230))
    right = 'STAGED PREVIEW  •  NOT AN IN-GAME CAPTURE'
    rb = d.textbbox((0, 0), right, font=FONT_FOOT)
    d.text((618 - (rb[2] - rb[0]), 346), right, font=FONT_FOOT, fill=(158, 177, 195, 215))


BEATS = [
    ('IDLE', 14), ('BASIC ATTACK', 14), ('IONIC EDGE', 18),
    ('TRANSCENDENT WARD', 18), ('WARD BURST', 14), ('CONSECRATION', 18), ('DEATH', 14),
]
STATE_NO = {name: i + 1 for i, (name, _) in enumerate(BEATS)}
TOTAL = sum(n for _, n in BEATS)


def make_frame(state, local_i, local_n, global_i):
    t = local_i / max(1, local_n - 1)
    canv = stage_background()
    flash = smooth((t - 0.64) / 0.22) if state in ('BASIC ATTACK', 'IONIC EDGE', 'WARD BURST') else 0
    draw_shadow(canv, TARGET_X, 0.8)
    draw_target(canv, flash)

    hx = HERO_HOME
    sprite_anim_name = 'idle'
    sprite_index = local_i
    hero_alpha = 255

    if state == 'IDLE':
        sprite_anim_name, sprite_index = 'idle', local_i
    elif state == 'BASIC ATTACK':
        sprite_anim_name, sprite_index = 'attack', min(len(sprite_anim['attack']['frames']) - 1, round(t * (len(sprite_anim['attack']['frames']) - 1)))
    elif state == 'IONIC EDGE':
        sprite_anim_name, sprite_index = 'skill', min(len(sprite_anim['skill']['frames']) - 1, round(t * (len(sprite_anim['skill']['frames']) - 1)))
    elif state == 'TRANSCENDENT WARD':
        sprite_anim_name, sprite_index = 'skill2', min(len(sprite_anim['skill2']['frames']) - 1, round(t * (len(sprite_anim['skill2']['frames']) - 1)))
        hx = HERO_HOME + (TARGET_X - 55 - HERO_HOME) * smooth(min(1.0, t * 1.25))
    elif state == 'WARD BURST':
        sprite_anim_name, sprite_index = 'idle', (local_i // 2) % len(sprite_anim['idle']['frames'])
    elif state == 'CONSECRATION':
        sprite_anim_name, sprite_index = 'ult', min(len(sprite_anim['ult']['frames']) - 1, round(t * (len(sprite_anim['ult']['frames']) - 1)))
    elif state == 'DEATH':
        sprite_anim_name, sprite_index = 'dead', min(len(sprite_anim['dead']['frames']) - 1, round(t * (len(sprite_anim['dead']['frames']) - 1)))
        if t > 0.86:
            hero_alpha = int(255 * (1 - (t - 0.86) / 0.14))

    draw_shadow(canv, hx, 1.0)

    if state == 'IDLE':
        aura = aura_frames[(local_i // 2) % len(aura_frames)]
        pulse = 1.72 + 0.14 * math.sin(local_i * 0.48)
        paste_center(canv, aura, hx, BASELINE - 54, pulse, 118)
        draw_tag(canv, 'RADIANCE  •  20 STACKS', hx, 119, (190, 225, 152, 235))

    elif state == 'BASIC ATTACK':
        if t > 0.40:
            # A restrained slash glint follows the arm/claw swipe animation.
            lay = Image.new('RGBA', (W, H), (0, 0, 0, 0))
            d = ImageDraw.Draw(lay, 'RGBA')
            a = int(180 * (1 - abs(t - 0.60) / 0.32))
            if a > 0:
                d.arc((hx + 22, 203, hx + 92, 267), start=202, end=328, fill=(204, 241, 163, a), width=3)
                d.arc((hx + 28, 208, hx + 86, 260), start=205, end=326, fill=(244, 224, 148, a // 2), width=1)
            canv.alpha_composite(lay)
        draw_tag(canv, 'ARM / CLAW SWIPE', hx, 119, (221, 231, 182, 230))

    elif state == 'IONIC EDGE':
        orbit = orbit_frames[(local_i * 2) % len(orbit_frames)]
        # Magnify Skill 1 in the staged GIF so its orbit reads clearly around the hero.
        # The runtime sheet and in-game VFX scale remain unchanged.
        paste_center(canv, orbit, hx, BASELINE - 52, 1.5, 255)
        if t > 0.52:
            # The packed mark frame is bottom-padded so its visible emblem sits
            # above the target's head, matching the in-game attachment anchor.
            paste_center(canv, mark_frames[0], TARGET_X, BASELINE - 43, 1.0, 245)
            draw_tag(canv, 'IONIC MARK  •  +1 / CAST', TARGET_X, 151, (250, 205, 111, 245))
        draw_tag(canv, 'TWO BLADES  •  OUTWARD / INWARD', hx, 119, (250, 205, 111, 235))

    elif state == 'TRANSCENDENT WARD':
        # Faint trail stamps are composed along Uranus's selected dash path.
        progress = smooth(min(1.0, t * 1.2))
        for j, fraction in enumerate((0.18, 0.38, 0.58, 0.78, 0.95)):
            if fraction < progress:
                tx = HERO_HOME + (hx - HERO_HOME) * (fraction / max(progress, 0.01))
                tr = trail_frames[(local_i + j * 2) % len(trail_frames)]
                paste_center(canv, tr, tx, BASELINE - 44, HERO_SCALE, 118)
        if t > 0.60:
            sh = shield_frames[local_i % len(shield_frames)]
            paste_center(canv, sh, hx, BASELINE - 45, HERO_SCALE, 245)
        draw_tag(canv, 'DASH  •  4s WARD', hx, 119, (250, 205, 111, 235))

    elif state == 'WARD BURST':
        if t < 0.28:
            sh = shield_frames[local_i % len(shield_frames)]
            paste_center(canv, sh, hx, BASELINE - 45, HERO_SCALE, 210)
        else:
            bi = min(len(burst_frames) - 1, round(((t - 0.28) / 0.72) * (len(burst_frames) - 1)))
            burst = burst_frames[bi]
            pulse = 1.0 * (1.0 + 0.14 * smooth((t - 0.28) / 0.72))
            paste_center(canv, burst, hx, BASELINE - 33, pulse, 255)
        draw_tag(canv, 'SHIELD BREAK / EXPIRY  •  BURST', hx, 119, (250, 205, 111, 235))

    elif state == 'CONSECRATION':
        aura = aura_frames[(local_i // 2) % len(aura_frames)]
        paste_center(canv, aura, hx, BASELINE - 54, 1.9, 145)
        flare = flare_frames[min(len(flare_frames) - 1, round(t * (len(flare_frames) - 1)))]
        paste_center(canv, flare, hx, BASELINE - 67, 0.25, 215)
        draw_tag(canv, 'CLEANSE  •  HEAL  •  SPEED + RADIANCE', hx, 119, (226, 223, 149, 240))
        # Simple motion chevrons indicate the decaying speed buff, not extra game VFX.
        if 0.35 < t < 0.88:
            d = ImageDraw.Draw(canv, 'RGBA')
            aa = int(130 * (1 - abs(t - 0.62) / 0.35))
            if aa > 0:
                for k in range(3):
                    xx = hx + 45 + k * 10
                    d.line((xx, 228, xx + 6, 233, xx, 238), fill=(216, 224, 157, aa), width=1)

    elif state == 'DEATH':
        aura = aura_frames[(local_i // 2) % len(aura_frames)]
        paste_center(canv, aura, hx, BASELINE - 48, 1.65, int(110 * (1 - t)))
        draw_tag(canv, 'DEATH  •  SEPARATE ANIMATION', hx, 119, (191, 214, 178, 230))

    # The true atlas animation is the focal point, layered above the VFX.
    paste_hero(canv, sprite_anim_name, sprite_index, hx, BASELINE, hero_alpha)

    # Keep the shield dome legible as an outline around Uranus in the showcase.
    if state == 'TRANSCENDENT WARD' and t > 0.60:
        sh = shield_frames[local_i % len(shield_frames)]
        paste_center(canv, sh, hx, BASELINE - 45, HERO_SCALE, 255)

    # Re-layer only the outer aura/flare accents so they remain visible around the sprite.
    if state == 'CONSECRATION':
        flare = flare_frames[min(len(flare_frames) - 1, round(t * (len(flare_frames) - 1)))]
        paste_center(canv, flare, hx, BASELINE - 67, 0.23, 105)
    if state == 'WARD BURST' and t > 0.28:
        burst = burst_frames[min(len(burst_frames) - 1, round(((t - 0.28) / 0.72) * (len(burst_frames) - 1)))]
        paste_center(canv, burst, hx, BASELINE - 33, 1.0, 120)

    labels = {
        'IDLE': 'PASSIVE  •  RADIANCE REGEN  •  20 TIMED STACKS',
        'BASIC ATTACK': 'BASIC ATTACK  •  ARM / CLAW SWIPE  •  NO MARK',
        'IONIC EDGE': 'SKILL 1  •  ORBITING BLADES  •  ONE MARK / TARGET / CAST',
        'TRANSCENDENT WARD': 'SKILL 2  •  DASH TRAIL + SHIELD',
        'WARD BURST': 'WARD SHIELD  •  BREAK OR EXPIRY BURST',
        'CONSECRATION': 'ULTIMATE  •  CLEANSE + HEAL + DECAYING SPEED',
        'DEATH': 'DEATH ANIMATION  •  NOT A HIT FRAME',
    }
    draw_header_footer(canv, state, STATE_NO[state], labels[state], (global_i + 1) / TOTAL)
    return canv.convert('RGB')


frames = []
global_i = 0
for state, count in BEATS:
    for local_i in range(count):
        frames.append(make_frame(state, local_i, count, global_i))
        global_i += 1

palette = frames[0].convert('P', palette=Image.Palette.ADAPTIVE, colors=256)
pal_frames = [fr.convert('RGB').quantize(palette=palette, dither=Image.Dither.NONE) for fr in frames]
for out in OUTS:
    out.parent.mkdir(parents=True, exist_ok=True)
    pal_frames[0].save(out, save_all=True, append_images=pal_frames[1:], duration=FRAME_MS,
                       loop=0, disposal=2, optimize=False)
    print(f'Wrote {out} ({out.stat().st_size:,} bytes; {len(frames)} frames; {len(frames) * FRAME_MS / 1000:.1f}s)')
