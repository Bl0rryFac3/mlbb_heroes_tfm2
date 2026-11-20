#!/usr/bin/env python3
import json, pathlib, sys
v10=pathlib.Path(sys.argv[1])
v11=pathlib.Path(sys.argv[2])
checks=[]
def check(name, ok): checks.append((name, bool(ok)))
a=json.loads((v10/'champion/mlbb_balmond.data_champion').read_text())
b=json.loads((v11/'champion/mlbb_balmond.data_champion').read_text())
s1=b['skill']
check('S1 is position-aimed', s1['casting_type']=='Position')
check('S1 target selector preserved', s1['casting_target']==a['skill']['casting_target']=='EnemyChampion')
check('S1 action range and timing preserved', all(s1[k]==a['skill'][k] for k in ('range','duration','cooltime','start_timing','cancelable','can_use_with_move')))
fx=s1['effect']['effects']
move=[x for x in fx if x.get('type')=='MoveTo']
check('S1 uses one MoveTo at previous speed/range', len(move)==1 and move[0].get('speed')==7000 and move[0].get('range')==90000)
check('S1 no longer uses RushTime', not any(x.get('type')=='RushTime' for x in fx))
check('MoveTo completion calls S1 hit resolver', bool(move and move[0].get('end_effects')==[{'type':'Native','effect_ref':'mlbb_heroes:balmond_soul_lock_hit'}]))
# All non-S1 action data, and the rest of Balmond champion data, must remain unchanged.
a2=json.loads(json.dumps(a)); b2=json.loads(json.dumps(b))
a2.pop('skill'); b2.pop('skill')
check('All non-S1 Balmond data unchanged', a2==b2)
for key in ('attack','skill2','ult'):
    check(f'{key} unchanged', a[key]==b[key])
rs=(v11/'src/balmond.rs').read_text()
check('Native route accepts position and caps sampled path', 'InputTargetKindV1::Pos.code()' in rs and 'S1_MAX_DASH_DISTANCE' in rs)
check('Nearest-at-arrival resolver retains slow and bump', 'S1_ARRIVAL_HIT_RADIUS' in rs and 'apply_slow(sim, target, S1_SLOW, 40, 180)' in rs and 'sim.entity_knockback(caster, target, 1_000, 3)' in rs)
for name, ok in checks: print(('PASS' if ok else 'FAIL')+' '+name)
print(f'{sum(ok for _,ok in checks)}/{len(checks)} checks passed')
sys.exit(0 if all(ok for _,ok in checks) else 1)
