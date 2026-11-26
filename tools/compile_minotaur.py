"""Compile Minotaur's original body and separate VFX at fixed world pivots."""
from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path
import subprocess
import wave

from PIL import Image, ImageDraw

from build_popol_preview import svg
from compile_fanny_visuals import ROOT, place

SOURCE = ROOT / 'source/minotaur'
ID = 'mlbb_minotaur'
CELL = (96, 88)
BODY_PIVOT = (48, 66)
WAVE_PIVOT = (44, 25)
WAVE_TICKS = (2, 3, 3, 4, 4, 4, 5, 5)
VOICES = {
    # In and out points trim the source silence while keeping the original takes.
    'basic': ('Minotaur.projnext.skill03.ogg', .085, 1.05, .54),
    'stomp': ('Minotaur.projnext.skill01.ogg', .080, 1.16, .68),
    'roar': ('Minotaur.projnext.skill02.ogg', .080, .99, .68),
    'fury': ('Minotaur.projnext.ult01.ogg', .090, 3.05, .72),
}


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def split(name, cell, count, audit):
    path = SOURCE / 'received' / (name + '.png')
    image = Image.open(path).convert('RGBA')
    w, h = cell
    assert image.width % w == image.height % h == 0, name
    frames = [image.crop((x, y, x + w, y + h))
              for y in range(0, image.height, h) for x in range(0, image.width, w)]
    assert [i for i, frame in enumerate(frames) if frame.getbbox()] == list(range(count)), name
    audit[name] = dict(sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                       sheet=image.size, cell=cell, occupied=count, blank=len(frames) - count)
    return frames[:count]


def pack(folder, name, clips, pivot):
    frames = [frame for clip, _ in clips.values() for frame in clip]
    w, h = frames[0].size
    assert all(frame.size == (w, h) for frame in frames)
    columns = min(9, len(frames))
    sheet = Image.new('RGBA', (columns * w, (len(frames) + columns - 1) // columns * h))
    animations = {}
    i = 0
    for tag, (images, ticks) in clips.items():
        assert len(images) == len(ticks) and min(ticks) > 0, tag
        track = []
        for image, duration in zip(images, ticks):
            x, y = i % columns * w, i // columns * h
            sheet.paste(image, (x, y))
            track.append(dict(duration=duration / 60, data=dict(x=x, y=y, w=w, h=h)))
            i += 1
        animations[tag] = dict(frames=track)
    base = ROOT / folder / name
    sheet.save(str(base) + '#sheet.png', optimize=True)
    write(Path(str(base) + '#anim.fanim'), dict(anchor_x=pivot[0] / w, anchor_y=pivot[1] / h,
                                                anims=animations))
    return f'asset/mlbb_heroes/{folder}/{name}'


def body_frames(name, count, original_hoof_y, audit):
    frames = split(name, (72, 72), count, audit)
    offset = (BODY_PIVOT[0] - 31, BODY_PIVOT[1] - original_hoof_y)
    for frame in frames:
        left, top, right, bottom = frame.getbbox()
        assert 0 <= left + offset[0] < right + offset[0] <= CELL[0], name
        assert 0 <= top + offset[1] < bottom + offset[1] <= CELL[1], name
    return [place(frame, CELL, offset) for frame in frames]


def ground_layer(frames, source_pivot, scale_x=1, scale_y=None):
    scale_y = scale_x if scale_y is None else scale_y
    w, h = frames[0].size
    # Keep the same registered offset through the presentation, including awkward frames.
    size = (round(w * scale_x), round(h * scale_y))
    pivot = (round(source_pivot[0] * scale_x), round(source_pivot[1] * scale_y))
    return [image.resize(size, Image.Resampling.NEAREST) for image in frames], pivot


def load_body(audit):
    idle = body_frames('minotaur-idle', 9, 60, audit)
    run = body_frames('minotaur-walk', 9, 60, audit)
    basic = body_frames('minotaur-basic', 9, 60, audit)
    hit = body_frames('minotaur-hit', 9, 63, audit)
    dead = body_frames('minotaur-death', 9, 63, audit)
    s1 = body_frames('minotaur_s1_despair_stomp', 13, 60, audit)
    s2 = body_frames('minotaur_s2_motivation_roar', 9, 60, audit)
    ult_1 = body_frames('minotaur_ult_first_two_smashes', 17, 63, audit)
    ult_2 = body_frames('minotaur_ult_final_smash', 13, 62, audit)
    # Remove the misleading downstroke in the roar, and cut the second impact echoed
    # at the start of the finisher. Keep the owner's actual anticipation and strike poses.
    s2 = [s2[i] for i in (0, 1, 2, 5, 6, 7, 8)] + [idle[0]]
    ultimate = ult_1 + ult_2[2:7] + [ult_2[i] for i in (7, 8, 10)]
    clips = {
        'idle': (idle, [8] * 9), 'run': (run, [5] * 9),
        'attack': (basic, [3, 2, 2, 3, 2, 3, 3, 3, 3]),
        'hit': (hit, [2] * 9), 'dead': (dead, [5] * 8 + [22]),
        'skill': (s1, [3, 3, 3, 3, 4, 4, 4, 3, 3, 3, 2, 2, 3]),
        'skill2': (s2, [4, 4, 4, 3, 3, 3, 3, 4]),
        'ult': (ultimate, [3, 2, 2, 3, 3, 3, 2, 4, 4, 4, 4, 4, 4, 3, 3,
                           4, 4, 4, 5, 5, 5, 3, 2, 2, 2]),
    }
    audit['runtime_body'] = {
        'canvas': CELL, 'pivot': BODY_PIVOT, 'standing_height': 48,
        'source_marks': {'attack': 6, 'S1': 7, 'S2': 5, 'R early': [7, 15], 'R final': 7},
        'S2_source_frames': [0, 1, 2, 5, 6, 7, 8, 'idle:0'],
        'R_source_frames': [list(range(17)), [2, 3, 4, 5, 6, 7, 8, 10]],
        'frame_ticks': {name: ticks for name, (_, ticks) in clips.items()},
    }
    assert sum(clips['attack'][1][:6]) == 15
    assert sum(clips['skill'][1][:7]) == 24
    assert sum(clips['skill2'][1][:3]) == 12
    r = clips['ult'][1]
    assert [sum(r[:i]) for i in (7, 15, 22, 25)] == [18, 48, 78, 84]
    return clips


def landing_wave(radius, enraged):
    """Open ground-plane ring with a bright front lip and sparse outward dust."""
    amber = (255, 137, 48) if enraged else (249, 172, 62)
    coal = (152, 66, 49) if enraged else (139, 93, 62)
    ivory = (255, 235, 156) if enraged else (255, 246, 193)
    px, py = WAVE_PIVOT
    frames = []
    for i, (progress, opacity) in enumerate(zip(
        (.53, .72, .86, .95, 1., 1., 1., 1.), (255, 255, 235, 195, 146, 100, 62, 25),
    )):
        frame = Image.new('RGBA', (88, 50))
        draw = ImageDraw.Draw(frame)
        rx = round(radius * progress)
        ry = round(rx * .43)
        outer = (px-rx, py-ry, px+rx, py+ry)
        draw.arc(outer, 185, 355, fill=amber + (opacity * 3 // 5,), width=1)
        draw.arc(outer, 6, 174, fill=coal + (opacity,), width=3)
        lip = (px-rx, py-ry-1, px+rx, py+ry-1)
        for start, end in ((4, 35), (42, 74), (83, 127), (136, 176)):
            draw.arc(lip, start, end, fill=amber + (opacity,), width=2)
            draw.arc(lip, start + 4, end - 4, fill=ivory + (opacity,), width=1)
        if i < 3:
            draw.line((px-4+i, py, px+4-i, py), fill=ivory + (255-i*65,))
            draw.line((px, py-2, px, py+2), fill=amber + (255-i*65,))
        for index, angle in enumerate((9, 51, 116, 163, 209, 246, 308, 345)):
            if i >= 5 and index % 2:
                continue
            radians = math.radians(angle)
            dx = round((rx+2+i//2) * math.cos(radians))
            dy = round((ry+2+i//3) * math.sin(radians))
            x, y = px+dx, py+dy
            draw.point((x, y), fill=ivory + (opacity * 3 // 4,))
            if i < 4:
                draw.line((x, y+1, x+1, y+1), fill=amber + (opacity * 3 // 4,))
        left, top, right, bottom = frame.getbbox()
        assert 0 < left < right < frame.width and 0 < top < bottom < frame.height
        frames.append(frame)
    return frames


def effect_layers(audit):
    def fx(source, cell, count):
        return split(source, cell, count, audit)

    stomp = fx('s1_stomp', (124, 92), 8)
    early = fx('r_slam_early', (124, 124), 9)[:7]  # Last two duplicate the opening.
    final = fx('r_slam_final', (128, 128), 13)
    wave = fx('s2_roar_wave', (96, 96), 7)
    recipient = fx('s2_recipient_heal', (32, 48), 9)
    aura = fx('enraged_aura', (64, 64), 9)[:8]  # Last duplicates frame zero.
    motivation = fx('motivation', (32, 32), 9)
    basic = Image.open(SOURCE / 'basic_impact.png').convert('RGBA')
    basic_frames = []
    for size, strength in ((24, 224), (29, 255), (31, 144), (32, 75)):
        frame = basic.resize((size, size), Image.Resampling.NEAREST)
        frame.putalpha(frame.getchannel('A').point(lambda a: a * strength // 255))
        basic_frames.append(place(frame, (32, 32), ((32 - size) // 2, (32 - size) // 2)))
    gold = []
    for frame in aura:
        yellow = frame.convert('L').point(lambda v: min(255, v + 45))
        gold.append(Image.merge('RGBA', (yellow.point(lambda v: min(255, v + 65)), yellow,
                     yellow.point(lambda v: v // 3), frame.getchannel('A'))))
    layers = {
        'stomp': (stomp, (62, 49), .39, [3] * 8, -1),
        'stomp_enraged': (stomp, (62, 49), .50, [3] * 8, -1),
        'stomp_wave': (landing_wave(22, False), WAVE_PIVOT, 1, WAVE_TICKS, 2),
        'stomp_wave_enraged': (landing_wave(28, True), WAVE_PIVOT, 1, WAVE_TICKS, 2),
        'roar_wave': (wave, (48, 61), 1.28, [3] * 7, -1),
        'heal': (recipient, (16, 41), .8, [3] * 9, 2),
        'slam': (early, (62, 66), .61, [3] * 7, -1),
        'finisher': (final[:11], (64, 90), .62, [2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4], -1),
        'enraged': (aura, (32, 53), .80, [5] * 8, -1),
        'fury': (gold, (32, 53), .80, [5] * 8, -1),
        'motivation': (motivation, (16, 28), .70, [2, 2, 2, 2, 2, 2, 2, 2, 3], 1),
        'basic_bonus': (basic_frames, (16, 18), .65, [2] * 4, 2),
    }
    views, buffs = [], []
    for tag, (source, center, scale, ticks, z) in layers.items():
        frames, pivot = ground_layer(source, center, scale)
        anim = pack('effects', f'minotaur_{tag}', {'play': (frames, ticks)}, pivot)
        binding = dict(name=f'{ID}_{tag}', anim=anim, tag='play', z=z)
        if tag in ('enraged', 'fury'):
            buffs.append(dict(type='Animated', **binding))
        else:
            views.append(dict(type='Animation', is_follow=False, **binding))
    audit['effects'] = {tag: dict(scale=scale, source_pivot=pivot, ticks=ticks)
                        for tag, (_, pivot, scale, ticks, _) in layers.items()}
    audit['basic_impact_source'] = hashlib.sha256((SOURCE / 'basic_impact.png').read_bytes()).hexdigest()
    return views, buffs


def icons(clips):
    idle = clips['idle'][0]
    for i, frame in enumerate(idle):
        (ROOT / 'icons' / f'minotaur_draft_idle_{i}.svg').write_text(svg(frame))
    portrait = idle[0].crop((27, 16, 67, 56))
    portrait.save(ROOT / 'icons' / f'{ID}_portrait.png')
    frame = clips['attack'][0][6]
    image = frame.crop(frame.getbbox())
    image.thumbnail((58, 58), Image.Resampling.NEAREST)
    tile = Image.new('RGBA', (64, 64), '#25191d')
    tile.alpha_composite(image, ((64 - image.width) // 2, (64 - image.height) // 2))
    tile.save(ROOT / 'icons' / f'{ID}_attack.png')
    for slot, filename in {
        'passive': 'Rage_Incarnate.webp', 'skill': 'Despair_Stomp.webp',
        'skill2': 'Motivation_Roar.png', 'ult': 'Minoan_Fury.webp',
    }.items():
        with Image.open(SOURCE / 'media' / filename) as icon:
            assert icon.size == (100, 100), filename
            icon.convert('RGBA').save(ROOT / 'icons' / f'{ID}_{slot}.png')


def voices(audit):
    folder = ROOT / 'sound/sfx'
    override_path = ROOT / 'mod.override_info'
    overrides = json.loads(override_path.read_text())
    audit['voices'] = {}
    for slot, (filename, start, end, volume) in VOICES.items():
        source = SOURCE / 'media' / filename
        event = f'{ID}_voice_{slot}'
        clip = f'mlbb_heroes_clip_{event}'
        destination = folder / f'{clip}.wav'
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', str(start), '-i', str(source),
                        '-t', str(round(end-start, 3)), '-ac', '1', '-ar', '22050',
                        '-c:a', 'pcm_s16le', str(destination)], check=True)
        write(folder / f'{event}.sound_info', {'plays': [{'delay': 0, 'clip': clip, 'volume': volume}]})
        for name in (event, clip):
            overrides[f'asset/base/sound/sfx/{name}'] = {
                'remapping': f'asset/mlbb_heroes/sound/sfx/{name}', 'type': 'override',
            }
        with wave.open(str(destination), 'rb') as audio:
            duration = audio.getnframes() / audio.getframerate()
        audit['voices'][event] = {'source': filename, 'clip': clip, 'duration_seconds': duration,
                                  'lock_ticks': math.ceil(duration * 60) + 2}
    write(override_path, overrides)


def main():
    audit = {}
    clips = load_body(audit)
    sprite = pack('sprite', 'minotaur', clips, BODY_PIVOT)
    views, buffs = effect_layers(audit)
    icons(clips)
    voices(audit)
    path = ROOT / 'champion' / f'{ID}.data_champion'
    data = json.loads(path.read_text())
    data.update(sprite=sprite, view_effects=views, view_buffs=buffs)
    write(path, data)
    write(SOURCE / 'frame_map.json', audit)


if __name__ == '__main__':
    main()
