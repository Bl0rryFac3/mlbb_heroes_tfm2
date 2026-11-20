#!/usr/bin/env python3
"""Static checks for Balmond v8 S1 impact displacement and immediate Ult line VFX."""
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image

ROOT=Path(__file__).resolve().parents[1]
checks=[]
def check(ok: bool, msg: str) -> None:
    checks.append(msg)
    if not ok: raise SystemExit(f'FAIL: {msg}')
def walk(v):
    if isinstance(v,dict):
        yield v
        for child in v.values():
            if isinstance(child,(dict,list)): yield from walk(child)
    elif isinstance(v,list):
        for child in v:
            if isinstance(child,(dict,list)): yield from walk(child)

def main():
    data=json.loads((ROOT/'champion/mlbb_balmond.data_champion').read_text())
    check(data.get('id')=='mlbb_balmond','Balmond champion data parses')
    # S1 movement and target-hit behavior remain; only the native impact displacement is tuned.
    s1=data['skill']; nodes=list(walk(s1.get('effect',{})))
    rush=[n for n in nodes if n.get('type')=='RushTime']
    check(s1.get('casting_type')=='Direction' and s1.get('range')==90000,'S1 remains directional with 90,000 cast range')
    check(len(rush)==1 and rush[0].get('speed')==7000 and rush[0].get('tick')==13 and rush[0].get('range')==90000 and rush[0].get('penetrate') is False,'S1 RushTime path and first-champion stop behavior unchanged')
    refs=[n.get('effect_ref') for n in nodes if n.get('type')=='Native']
    check('mlbb_heroes:balmond_soul_lock_path' in refs and 'mlbb_heroes:balmond_soul_lock_hit' in refs,'S1 native path and hit handlers remain wired')
    source=(ROOT/'src/balmond.rs').read_text()
    check('sim.entity_knockback(caster, target, 1_000, 3)' in source,'S1 impact knockback reduced to 1,000 × 3 ticks')
    check('sim.entity_knockback(caster, target, 2_500, 6)' not in source,'old 2,500 × 6 S1 knockback removed')
    check('apply_slow(sim, target, S1_SLOW, 40, 180)' in source,'existing S1 40% slow and 180-tick duration retained')
    check('sim.deal_damage(caster, target, damage, 0, AttackTypeV1::Skill)' in source,'existing S1 hit damage call retained')
    # The existing Ult damage branch is preserved; only the independent visual timing changes.
    ult=data['ult']; unodes=list(walk(ult.get('effect',{})))
    check(ult.get('casting_type')=='Direction' and ult.get('range')==110000,'Ult remains directional with 110,000 cast range')
    lines=[n for n in unodes if n.get('type')=='LineRangeProjectile' and n.get('name')=='mlbb_balmond_ult_line']
    check(len(lines)==1,'exactly one visual-only Ult line exists')
    line=lines[0]
    check(line.get('length')==110000 and line.get('width')==60000,'Ult line geometry remains 110,000 × 60,000')
    check(line.get('apply')==1 and line.get('delay')==30,'Ult line uses immediate trigger and 30-tick lifetime timing')
    check(line.get('applied_effects')==[],'Ult line remains visual-only')
    fan=[n for n in unodes if n.get('type')=='RangeEffect' and 'DirDot' in n.get('shape',{})]
    check(len(fan)==1 and fan[0]['shape']['DirDot']=={'radius':110000,'range':707},'original DirDot damage fan geometry unchanged')
    check(len(fan[0].get('effects',[]))==1 and fan[0]['effects'][0].get('effect_ref')=='mlbb_heroes:balmond_lethal_counter','existing native execute remains the only fan child')
    check(not any(n.get('type')=='ViewEffect' and n.get('name')=='mlbb_balmond_ult_slash' for n in unodes),'per-target Ult slash ViewEffect remains absent')
    binding=[x for x in data.get('view_projectiles',[]) if x.get('name')=='mlbb_balmond_ult_line']
    check(len(binding)==1 and binding[0].get('type')=='Animated' and binding[0].get('anim')=='asset/mlbb_heroes/effects/balmond_ult_fx' and binding[0].get('tag')=='slash','Ult line visual binding resolves to the slash animation')
    check(Image.open(ROOT/'effects/balmond_ult_fx#sheet.png').size==(201,122),'Ult slash sheet remains original 201×122 art')
    # Keep the prior small S2 VFX and S2 timing intact.
    s2=data['skill2']; s2nodes=list(walk(s2.get('effect',{})))
    anims=[n for n in s2nodes if n.get('type')=='CasterAnimation' and n.get('name')=='skill2']
    check(len(anims)==1 and anims[0].get('tick')==12 and s2.get('duration')==180,'V7 S2 entry and native 180-tick behavior retained')
    im=Image.open(ROOT/'effects/balmond_s2_fx#sheet.png').convert('RGBA')
    check(im.size==(1086,362),'S2 sheet remains the v7 1086×362 sheet')
    fanim=json.loads((ROOT/'effects/balmond_s2_fx#anim.fanim').read_text())['anims']['spin']['frames']
    check(len(fanim)==8 and all(f.get('duration')==0.09375 for f in fanim),'S2 8-frame timing remains unchanged')
    print('\n'.join('PASS: '+m for m in checks))
    print(f'\n{len(checks)} static checks passed, 0 failed')
if __name__=='__main__':main()
