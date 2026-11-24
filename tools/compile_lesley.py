"""Build Lesley's TFM2 art from the supplied sheets and official skill icons."""
from copy import deepcopy
from pathlib import Path
import json

from PIL import Image, ImageEnhance, ImageOps

root = Path(__file__).resolve().parents[1]
art = root / 'source/lesley/animations'
audit = {}

def save(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')

def frames(name, size, body=False):
    image = Image.open(art / f'lesley_{name}.png').convert('RGBA')
    w, h = size
    assert image.width % w == 0 and image.height % h == 0, name
    out = []
    for y in range(0, image.height, h):
        for x in range(0, image.width, w):
            cell = image.crop((x, y, x + w, y + h))
            box = cell.getbbox()
            if not box:
                continue
            if body:
                # Preserve body placement and a common ground line. Bound
                # height using nearest neighbor for the few oversized poses.
                cropped = cell.crop(box)
                if cropped.height > 48:
                    width = round(cropped.width * 48 / cropped.height)
                    cropped = cropped.resize((width, 48), Image.Resampling.NEAREST)
                padded = Image.new('RGBA', (80, 72))
                padded.paste(cropped, (box[0] + 4, min(box[3], 62) - cropped.height))
                assert padded.getbbox()[3] - padded.getbbox()[1] <= 48
                cell = padded
            out.append(cell)
    assert out, name
    audit[name] = {'cell': list(out[0].size), 'frames': len(out)}
    return out

def sheet(directory, name, clips, anchor):
    flat = [im for images, _ in clips.values() for im in images]
    width, height = flat[0].size
    columns = min(8, len(flat))
    atlas = Image.new('RGBA', (width * columns, height * ((len(flat) + columns - 1) // columns)))
    anims = {}
    index = 0
    for tag, (images, ticks) in clips.items():
        assert len(images) <= ticks
        track = []
        for i, im in enumerate(images):
            x, y = index % columns * width, index // columns * height
            atlas.paste(im, (x, y))
            frame_ticks = (i + 1) * ticks // len(images) - i * ticks // len(images)
            track.append({'duration': frame_ticks / 60, 'data': {'x': x, 'y': y, 'w': width, 'h': height}})
            index += 1
        anims[tag] = {'frames': track}
    base = root / directory / name
    atlas.save(base.with_name(base.name + '#sheet.png'))
    save(base.with_name(base.name + '#anim.fanim'), {'anchor_x': anchor[0], 'anchor_y': anchor[1], 'anims': anims})

def body(name):
    return frames(name, (72, 72), True)

def camouflage_body():
    poses = body('camouflage_enter')
    steps = max(1, len(poses) - 1)
    for index, pose in enumerate(poses):
        opacity = 255 - 140 * index // steps
        pose.putalpha(pose.getchannel('A').point(lambda value: value * opacity // 255))
    return poses

aim, recoil, hold, recover = (body('ult_' + part) for part in ('aim', 'recoil', 'hold', 'recover'))
body_clips = {
    'idle': (body('idle'), 60),
    'run': (body('run'), 36),
    'attack': (body('attack'), 21),
    'skill': (camouflage_body(), 12),
    'skill2': (body('grenade'), 27),
    'hit': (body('hit'), 12),
    'dead': (body('dead'), 40),
    'lethal': (body('lethal_shot'), 27),
    'camouflage_exit': (body('camouflage_exit'), 12),
    'ult_aim': (aim, 30),
    'ult_hold': (hold, 21),
    'ult_recoil': (recoil, 18),
    'ult_recover': (recover, 18),
}
# Native action tags are sampled against action time. Build one finite
# viewport sequence with onsets at ticks 30, 51, 72, and 93.
aim_poses = [aim[i * len(aim) // 30] for i in range(30)]
kick_poses = [recoil[i * len(recoil) // 18] for i in range(18)]
return_poses = [recover[i * len(recover) // 18] for i in range(18)]
ult = aim_poses + (kick_poses + [hold[0]] * 3) * 3 + kick_poses + return_poses
body_clips['ult'] = (ult, len(ult))
sheet('sprite', 'lesley', body_clips, (0.5, 62 / 72))

view_effects, projectiles, buffs = [], [], []
def effect(name, size, ticks, anchor=(.5, .5), kind='effect', mirrored=False, display=None):
    images = frames(name, size)
    if display is not None:
        images = [
            ImageEnhance.Brightness(im.resize(display, Image.Resampling.NEAREST)).enhance(1.28)
            for im in images
        ]
    for suffix in ('', '_left') if mirrored else ('',):
        basename = 'lesley_' + name + suffix
        selected = [ImageOps.mirror(im) for im in images] if suffix else images
        sheet('effects', basename, {'main': (selected, ticks)},
              (1 - anchor[0] if suffix else anchor[0], anchor[1]))
        common = {'name': 'mlbb_' + basename, 'anim': 'asset/mlbb_heroes/effects/' + basename, 'tag': 'main', 'z': 1}
        if kind == 'projectile':
            projectiles.append({'type': 'Animated', **common, 'repeat': True})
        elif kind == 'buff':
            buffs.append({'type': 'Animated', **common})
        else:
            view_effects.append({'type': 'Animation', **common, 'is_follow': False})

projectile_cell = (48, 16)
grenade_fan_cell = (96, 64)
for prefix, impact, muzzle in [('basic', (24, 24), (24, 24)), ('lethal', (52, 52), (24, 24)), ('ult', (32, 32), (32, 32))]:
    effect(prefix + '_impact', impact, 16)
    if prefix == 'ult':
        effect('ult_muzzle', muzzle, 13, mirrored=True, display=(42, 42))
        effect('ult_projectile', projectile_cell, 9, kind='projectile',
               anchor=(.69, .5), display=(82, 20))
    else:
        effect(prefix + '_muzzle', muzzle, 9, mirrored=True)
        effect(prefix + '_tracer', projectile_cell, 9, kind='projectile')
effect('lethal_ready', (24, 24), 20, (.5, .87), 'buff')
effect('camouflage_edge', (48, 48), 20, (.5, .9), 'buff')
effect('camouflage_ripple', (72, 40), 15)
effect('grenade_fan', grenade_fan_cell, 16, (5 / 96, 58 / 64), mirrored=True)
effect('grenade_impact', (32, 32), 12)
effect('ult_lock', (48, 48), 16, (.5, .8), 'buff')

def action(name, duration, cooldown, timing, reach, cast, target, moving=False):
    return {
        'action_name': name, 'description': f'#asset/base/text/champion?description.mlbb_lesley.{name}',
        'duration': duration, 'cooltime': cooldown, 'start_timing': timing,
        'cancelable': True, 'range': reach, 'casting_type': cast, 'casting_target': target,
        'attack_type': 'BaseAttack' if name == 'attack' else 'Skill', 'can_use_with_move': moving,
        'effect': {'type': 'Native', 'effect_ref': 'mlbb_heroes:lesley_' + name},
    }

icon_slots = ('skill', 'skill2', 'ult')
for index, slot in enumerate(icon_slots, start=1):
    source = root / 'source' / 'lesley' / 'icons' / f'skill-{index}.png'
    with Image.open(source) as image:
        icon = image.convert('RGBA').resize((64, 64), Image.Resampling.LANCZOS)
        icon.save(root / 'icons' / f'lesley_{slot}.png')

original = json.loads((root / 'champion/mlbb_granger.data_champion').read_text())
champion = {
    'id': 'mlbb_lesley', 'category': 'Range', 'tags': ['AD', 'Range'],
    'sprite': 'asset/mlbb_heroes/sprite/lesley', 'anim_prefix': '',
    'skill_icons': [f'asset/mlbb_heroes/icons/lesley_{slot}' for slot in icon_slots],
    'stat': {**original['stat'], 'attack': 104, 'hp': 680},
    'growth': {**original['growth'], 'attack': 6},
    'attack': action('attack', 21, 66, 8, 75000, 'Targeting', 'Enemy'),
    'skill': action('skill', 12, 300, 0, 92000, 'None', 'EnemyWithoutTower', True),
    'skill2': action('skill2', 27, 540, 6, 30000, 'Direction', 'EnemyWithoutTower'),
    'ult': action('ult', len(ult), 1920, 0, 150000, 'Targeting', 'EnemyChampion'),
    'passive': {'passive_ref': 'mlbb_heroes:lesley'},
    'view_effects': view_effects,
    'view_projectiles': projectiles,
    'view_buffs': buffs,
}
champion_path = root / 'champion/mlbb_lesley.data_champion'
if champion_path.is_file():
    # Rebuilding art preserves the gameplay data from the current build.
    existing = json.loads(champion_path.read_text())
    for key in ('sprite', 'anim_prefix', 'skill_icons', 'view_effects', 'view_projectiles', 'view_buffs'):
        existing[key] = champion[key]
    champion = existing
save(champion_path, champion)
styles = root / 'style/champion_view.champion_view'
layout = json.loads(styles.read_text())
for part in layout.values():
    if isinstance(part, dict) and 'mlbb_chou' in part:
        part['mlbb_lesley'] = deepcopy(part['mlbb_chou'])
save(styles, layout)
save(root / 'reports/LESLEY_1172_ART.json', audit)
