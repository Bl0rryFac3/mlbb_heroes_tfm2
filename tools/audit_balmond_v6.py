#!/usr/bin/env python3
"""Static audit of the Balmond v6 movement, target-VFX, and diagnostics patch."""
import json, sys
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
checks=[]; errors=[]
def check(ok,msg): (checks if ok else errors).append(("PASS: " if ok else "FAIL: ")+msg)
def read_json(p): return json.loads(p.read_text())
kit=read_json(ROOT/'champion/mlbb_balmond.data_champion')
s2=kit['skill2']; s2fx=s2['effect']['effects']
anim=next((x for x in s2fx if x.get('type')=='CasterAnimation'),{})
check(anim.get('name')=='skill2' and anim.get('tick')==12,'S2 forced body animation is only 12 ticks')
check(s2.get('duration')==180 and s2.get('can_use_with_move') is True,'S2 action retains its three-second duration and moving-use permission')
check(any(x.get('effect_ref')=='mlbb_heroes:balmond_cyclone_start' for x in s2fx),'S2 still starts the native cyclone state')
ult=kit['ult']; ef=ult['effect']['effects']
fan=next((x for x in ef if x.get('type')=='RangeEffect'),{})
check(fan.get('shape',{}).get('DirDot')=={'radius':110000,'range':707},'existing Balmond damage fan geometry is unchanged')
check(fan.get('target')=='EnemyWithoutTower','fan still selects the prior enemy set')
fanfx=fan.get('effects',[])
check(any(x.get('type')=='ViewEffect' and x.get('name')=='mlbb_balmond_ult_slash' for x in fanfx),'slash is now a target-position hit visual inside the fan')
check(any(x.get('effect_ref')=='mlbb_heroes:balmond_lethal_counter' for x in fanfx),'native execute remains in the fan target effects')
check(not any(x.get('type')=='CasterViewEffect' and x.get('name')=='mlbb_balmond_ult_slash' for x in ef),'old caster-anchored Ult slash trigger is removed')
binding=next((x for x in kit['view_effects'] if x.get('name')=='mlbb_balmond_ult_slash'),{})
check(binding.get('is_follow') is True,'target impact visual follows its hit target briefly')
img=Image.open(ROOT/'effects/balmond_ult_fx#sheet.png').convert('RGBA')
check(img.size==(161,98),'Ult art is 80% of the v5 sheet dimensions (161x98)')
anim_data=read_json(ROOT/'effects/balmond_ult_fx#anim.fanim')['anims']['slash']['frames'][0]
check(anim_data['data']['w']==161 and anim_data['data']['h']==98,'Ult FAnim crop matches resized sheet')
check(anim_data['duration']==0.5,'Ult effect timing remains unchanged')
rust=(ROOT/'src/balmond.rs').read_text()
for marker in ['[BALMOND_S2] native_start','[BALMOND_S2] passive_active','[BALMOND_S2] pulse=']:
    check(marker in rust,f'S2 runtime diagnostic present: {marker}')
check('sim.deal_damage(owner, id, damage, 0, AttackTypeV1::Skill)' in rust,'S2 damage still calls the stable damage pipeline')
check('S2_HITS: usize = 14' in rust and 'S2_RADIUS: f64 = 36_000.0' in rust,'S2 hit count and radius remain unchanged')
lib=(ROOT/'src/lib.rs').read_text()
check('balmond::register(&mut module)' in lib,'Balmond native module remains registered')
for line in checks+errors: print(line)
print(f'\n{len(checks)} checks passed; {len(errors)} failures')
sys.exit(bool(errors))
