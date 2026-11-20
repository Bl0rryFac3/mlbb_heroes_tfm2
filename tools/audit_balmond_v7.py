#!/usr/bin/env python3
"""Static checks for the v7 directional Ult visual and smaller S2 art."""
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image

ROOT=Path(__file__).resolve().parents[1]
checks=[]
def check(ok: bool, message: str) -> None:
    checks.append((bool(ok),message))
    if not ok: raise SystemExit(f'FAIL: {message}')

def walk(value):
    if isinstance(value,dict):
        yield value
        for child in value.values():
            if isinstance(child,(dict,list)): yield from walk(child)
    elif isinstance(value,list):
        for child in value:
            if isinstance(child,(dict,list)): yield from walk(child)

def main() -> None:
    data=json.loads((ROOT/'champion/mlbb_balmond.data_champion').read_text())
    check(data.get('id')=='mlbb_balmond','Balmond data loaded')
    ult=data['ult']
    check(ult.get('casting_type')=='Direction' and ult.get('range')==110000,'Ultimate remains directional with 110,000 range')
    nodes=list(walk(ult['effect']))
    lines=[n for n in nodes if n.get('type')=='LineRangeProjectile' and n.get('name')=='mlbb_balmond_ult_line']
    check(len(lines)==1,'exactly one directional Ult line visual exists')
    line=lines[0]
    check(line.get('length')==110000 and line.get('width')==60000,'visual line uses 110,000 length and 60,000 width')
    check(line.get('delay')==0 and line.get('apply')==30,'visual line begins immediately and lasts 30 ticks')
    check(line.get('applied_effects')==[],'directional line has no gameplay effects')
    fan_nodes=[n for n in nodes if n.get('type')=='RangeEffect' and 'DirDot' in n.get('shape',{})]
    check(len(fan_nodes)==1,'original DirDot hit fan is preserved exactly once')
    fan=fan_nodes[0]
    check(fan['shape']['DirDot']=={'radius':110000,'range':707},'existing DirDot geometry unchanged')
    fan_children=fan.get('effects',[])
    check(len(fan_children)==1 and fan_children[0].get('effect_ref')=='mlbb_heroes:balmond_lethal_counter','existing execute remains the sole fan child')
    check(not any(n.get('type')=='ViewEffect' and n.get('name')=='mlbb_balmond_ult_slash' for n in nodes),'no per-target Ultimate ViewEffect remains')
    binding=[x for x in data.get('view_projectiles',[]) if x.get('name')=='mlbb_balmond_ult_line']
    check(len(binding)==1 and binding[0].get('type')=='Animated','line visual name is registered once as an Animated projectile')
    bind=binding[0]
    check(bind.get('anim')=='asset/mlbb_heroes/effects/balmond_ult_fx' and bind.get('tag')=='slash','line binding resolves to the Balmond slash animation')
    check((ROOT/'effects/balmond_ult_fx#sheet.png').is_file() and Image.open(ROOT/'effects/balmond_ult_fx#sheet.png').size==(201,122),'directional Ult art is the restored original 201x122 sheet')
    check(not any(x.get('name')=='mlbb_balmond_ult_slash' for x in data.get('view_effects',[])),'legacy per-target slash binding removed')

    # Preserve the S2 duration/timing and only shrink visible pixels in its existing 8 frames.
    s2=data['skill2']
    body=[n for n in walk(s2.get('effect',{})) if n.get('type')=='CasterAnimation' and n.get('name')=='skill2']
    check(len(body)==1 and body[0].get('tick')==12,'v6 12-tick S2 entry animation retained')
    check(s2.get('duration')==180,'S2 native action duration remains 180 ticks')
    s2path=ROOT/'effects/balmond_s2_fx#sheet.png'
    image=Image.open(s2path).convert('RGBA')
    check(image.size==(1086,362),'S2 sheet dimensions unchanged')
    anim=json.loads((ROOT/'effects/balmond_s2_fx#anim.fanim').read_text())
    frames=anim.get('anims',{}).get('spin',{}).get('frames',[])
    check(len(frames)==8 and all(f.get('duration')==0.09375 for f in frames),'S2 8-frame timing metadata unchanged')
    expected_sizes=[(114,67),(110,71),(112,68),(112,68),(112,68),(116,65),(114,66),(109,66)]
    actual=[]
    for i,frame in enumerate(frames):
        d=frame['data']; bounds=(round(d['x']),round(d['y']),round(d['x']+d['w']),round(d['y']+d['h']))
        crop=image.crop(bounds); box=crop.getchannel('A').getbbox()
        check(box is not None,f'S2 frame {i} remains visible')
        actual.append((box[2]-box[0],box[3]-box[1]))
    check(actual==expected_sizes,'S2 visible alpha footprint equals the audited 10%-smaller frame sizes')
    print('\n'.join(f'PASS: {msg}' for ok,msg in checks))
    print(f'\n{len(checks)} static checks passed, 0 failed')

if __name__=='__main__': main()
