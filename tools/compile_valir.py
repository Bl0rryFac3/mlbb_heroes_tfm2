"""Build TFM2 assets from the owner's original Valir sheets."""
from pathlib import Path
import json
import math

from PIL import Image

root = Path(__file__).resolve().parents[1]
art = root / 'source/valir/animations'
audit = {}

def save(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')

def frames(name, size, body=False):
    image = Image.open(art / f'valir_{name}.png').convert('RGBA')
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
                # The idle feet straddle x=23 on the source's y=48 ground line.
                # Keep that same pivot throughout the run and cast poses.
                padded = Image.new('RGBA', (80, 72))
                padded.paste(cell, (40 - 23, 0))
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
    images = frames(name, (48, 48), True)
    if name == 'vengeance_flame':
        # The native effect supplies the flash even when the ult reacts to CC.
        # Drop the two poses with a baked-in flash so it cannot play twice.
        return images[:4] + images[6:]
    return images

body_clips = {tag: (body(name), ticks) for tag, name, ticks in [
    ('idle','idle',60),('run','run',36),('attack','attack',21),
    ('skill','fireball',24),('skill2','torrent',28),('ult','vengeance_flame',30),
    ('hit','hit',12),('dead','dead',40)
]}
sheet('sprite', 'valir', body_clips, (.5, 48 / 72))

effects, projectiles, buffs = [], [], []
def binding(name, images, ticks, anchor=(.5,.5), mode='fx'):
    asset = 'valir_' + name
    sheet('effects', asset, {'main': (images,ticks)}, anchor)
    common={'name':'mlbb_'+asset,'anim':'asset/mlbb_heroes/effects/'+asset,'tag':'main','z':1}
    if mode == 'projectile':
        projectiles.append({'type':'Animated', **common,'repeat':True})
    elif mode == 'buff':
        buffs.append({'type':'Animated', **common})
    else:
        effects.append({'type':'Animation', **common,'is_follow':False})

for n,cell in [('basic_ember',(44,24)),('fireball_projectile',(52,28)),('fireball_empowered',(56,24))]:
    binding(n,frames(n,cell),15,mode='projectile')
for n,cell,ticks,anchor in [
    ('fireball_impact',(32,32),15,(.5,.5)),
    ('fireball_waiting_patch',(48,24),45,(.5,.5)),
    ('fireball_delayed_blast',(56,48),21,(.5,.77)),
    ('ablaze_burn',(24,24),15,(.5,.8)),
    ('ablaze_detonation',(52,44),18,(.5,.58)),
]:
    binding(n,frames(n,cell),ticks,anchor)
binding('cleanse_flash', [
    im.resize((36,25),Image.Resampling.NEAREST)
    for im in frames('cleanse_flash',(52,36))
], 16, (.5,.62))
for n,cell,pivot,zero_rotation,scale,duration in [
    ('torrent_wave',(92,76),(9,63),-27.5,.82,20),
    ('torrent_empowered',(72,52),(4,47),-27.5,.88,20),
    ('torrent_firewall',(116,84),(58,62),90,.64,180)
]:
    scaled_size=tuple(round(d * scale) for d in cell)
    scaled_pivot=tuple(round(d * scale) for d in pivot)
    images=[im.resize(scaled_size,Image.Resampling.NEAREST) for im in frames(n,cell)]
    if n=='torrent_firewall':
        # Hold the bright central section between rise and expiry.
        images=images[:3]+images[3:6]*12+images[6:]
    for direction in range(8):
        rotated=[]
        for im in images:
            canvas=Image.new('RGBA',(160,160))
            canvas.paste(im,(80-scaled_pivot[0],80-scaled_pivot[1]))
            angle=zero_rotation-direction*45
            rotated.append(canvas.rotate(angle,resample=Image.Resampling.NEAREST,center=(80,80)))
        binding(n+'_'+str(direction),rotated,duration)
marker=frames('ablaze_marker',(72,72))
for count,index in [(1,0),(2,4),(3,8)]:
    binding('mark_'+str(count),[marker[index]],20,(.5,.97),'buff')
binding('idle_palm',frames('idle_palm',(24,24)),20,(.0,.8))

# A single attached animation shows the remaining charges near the feet.
ember_frames=[]
for im in frames('vengeance_ember',(32,28)):
    ember=im.crop(im.getbbox())
    ember.thumbnail((7,7),Image.Resampling.NEAREST)
    ember_frames.append(ember)
orbit_frames=16
for count in range(1,5):
    orbit=[]
    for frame in range(orbit_frames):
        canvas=Image.new('RGBA',(64,50))
        for index in range(count):
            angle=math.tau * (frame/orbit_frames + index/count)
            ember=ember_frames[(frame+index)%len(ember_frames)]
            x=round(32+22*math.cos(angle)-ember.width/2)
            y=round(37+8*math.sin(angle)-ember.height/2)
            canvas.alpha_composite(ember,(x,y))
        orbit.append(canvas)
    binding(f'vengeance_{count}',orbit,72,(.5,48/50),'buff')

# Default Mobile Legends icons, archived alongside the original artwork.
for name, slot in [('skill','skill-1'),('skill2','skill-2'),('ult','skill-3')]:
    with Image.open(root/'source/valir/icons'/f'{slot}.webp') as im:
        icon=im.convert('RGBA').resize((64,64),Image.Resampling.LANCZOS)
        icon.save(root/'icons'/f'valir_{name}.png')
def action(name,duration,cooldown,start,reach,cast,target):
    return {'action_name':name,'description':f'#asset/base/text/champion?description.mlbb_valir.{name}',
      'duration':duration,'cooltime':cooldown,'start_timing':start,'cancelable':True,
      'range':reach,'casting_type':cast,'casting_target':target,
      'attack_type':'BaseAttack' if name=='attack' else 'Skill','can_use_with_move':False,
      'effect':{'type':'Native','effect_ref':'mlbb_heroes:valir_'+name}}
base={'attack':80,'magic_power':35,'hp':880,'defence':20,'magic_resistance':20,'move_speed':900,'hp_regen':6}
growth={'attack':6,'magic_power':18,'hp':100,'defence':7,'magic_resistance':4,'move_speed':9,'hp_regen':0}
save(root/'champion/mlbb_valir.data_champion',{
 'id':'mlbb_valir','category':'Magician','tags':['AP','Range','CC'],
 'sprite':'asset/mlbb_heroes/sprite/valir','anim_prefix':'','skill_icons':[f'asset/mlbb_heroes/icons/valir_{n}' for n in ('skill','skill2','ult')],
 'stat':base,'growth':growth,
 'attack':action('attack',21,90,8,60000,'Targeting','Enemy'),
 'skill':action('skill',24,540,10,82000,'Direction','EnemyWithoutTower'),
 'skill2':action('skill2',28,600,10,38000,'Direction','EnemyWithoutTower'),
 'ult':action('ult',30,2520,0,82000,'None','EnemyWithoutTower'),
 'view_effects':effects,'view_projectiles':projectiles,'view_buffs':buffs,
 'passive':{'passive_ref':'mlbb_heroes:valir'}})
report = root/'reports/VALIR_ART_AUDIT.json'
report.parent.mkdir(exist_ok=True)
save(report,audit)
