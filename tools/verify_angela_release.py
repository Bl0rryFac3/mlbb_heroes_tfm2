#!/usr/bin/env python3
"""Fail-closed runtime/source checks for the Angela repair candidate.
Usage: verify_angela_release.py RUNTIME_MOD_ROOT [SOURCE_ROOT]
Behavior is tested separately by the Rust callback tests; these check wiring/assets.
"""
from pathlib import Path
import json,re,sys,hashlib,tomllib
from PIL import Image
root=Path(sys.argv[1]);source=Path(sys.argv[2]) if len(sys.argv)>2 else root
checks=[]
def check(ok,msg):
 if not ok:raise SystemExit('FAIL '+msg)
 checks.append(msg)
def walk(x):
 yield x
 if isinstance(x,dict):
  for v in x.values():yield from walk(v)
 elif isinstance(x,list):
  for v in x:yield from walk(v)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
info=json.loads((root/'mod.mod_info').read_text());check(info['version']=='1.1.122','internal version 1.1.122')
champions=list((root/'champion').glob('*.data_champion'));check(len(champions)==21,'21 champion files')
text=json.loads((root/'text/champion.i18n').read_text());check(len(text)==8,'8 locales')
heroes=[json.loads(p.read_text()) for p in champions]
# fanny's display id differs from its data-file stem, so use description key.
keys={h['attack']['description'].split('description.',1)[1].split('.',1)[0] for h in heroes}
for locale,l in text.items():
 for key in keys:
  desc=l['description'][key]
  check(all(isinstance(desc.get(f),str) and desc[f].strip() for f in ['name','attack','skill','skill2','ult','passive']),f'{locale}/{key} complete descriptions')
# Every animation declaration resolves to exact #sheet/#anim files, tag and bounds.
bindings=0
for data in heroes:
 refs=[v for v in walk(data) if isinstance(v,dict) and isinstance(v.get('anim'),str)]
 refs=[dict(v,tag=v[t]) for v in refs for t in (('pre_tag','loop_tag','remove_tag') if v.get('type')=='ThreePhase' else ('tag',))]
 refs.extend({'anim':data['sprite'],'tag':tag} for tag in ['idle','run','attack','skill','skill2','ult','hit','dead'])
 for v in refs:
  ref=v['anim'];prefix='asset/mlbb_heroes/'
  if not ref.startswith(prefix):continue
  base=root/ref[len(prefix):];meta=Path(str(base)+'#anim.fanim');sheet=Path(str(base)+'#sheet.png')
  check(meta.is_file() and sheet.is_file(),f'{ref} exact atlas pair')
  a=json.loads(meta.read_text())['anims'];tag=v.get('tag') or 'idle';check(tag in a,f'{ref}:{tag} exists')
  im=Image.open(sheet)
  for f in a[tag]['frames']:
   r=f['data'];check(r['w']>0 and r['h']>0 and r['x']>=0 and r['y']>=0 and r['x']+r['w']<=im.width and r['y']+r['h']<=im.height,f'{ref}:{tag} frame bounds')
  bindings+=1
 for icon in data.get('skill_icons',[]):
  if icon.startswith('asset/mlbb_heroes/'):
   p=root/(icon.removeprefix('asset/mlbb_heroes/')+'.png');check(p.is_file(),f'{icon} icon exists');Image.open(p).verify()
a=next(h for h in heroes if h['id']=='mlbb_angela')
check(a['passive']['passive_ref']=='mlbb_heroes:angela_smart_heart','Angela runtime selected')
check(not any(v.get('type')=='Shield' for v in walk(a['ult']) if isinstance(v,dict)),'shield has one native owner')
check(not any(v.get('type')=='ApAttack' for v in walk(a['skill2']) if isinstance(v,dict)),'S2 damage has one native owner')
check(a['attack']['effect']['type']=='TargetProjectile' and bool(a['attack']['effect']['applied_effects']),'basic damage occurs on impact')
names={v['name'] for v in a['view_effects']}
check(all(f'angela_wire_{i}' in names for i in range(12)),'12 live tether directions')
check(all(f'angela_wave_{i}_{n}' in names for i in range(16) for n in range(9)),'16 x 9 supplied wave bindings')
check('angela_attached_doll' in names,'host doll bound')
check((root/'mlbb_heroes.dll').read_bytes().startswith(b'MZ'),'Windows PE DLL')
# Validate source wiring and require exact DLL/data identity between both packages.
src=source/'sdk/candidate'
check((src/'angela_tests.rs').is_file(),'callback tests included')
check(tomllib.loads((src/'Cargo.toml').read_text())['package']['version']==info['version'],'crate and mod versions agree')
check('crate::angela::tick_all(sim);' in (src/'zhask.rs').read_text(),'match fallback composed with Zhask')
check('mod angela;' in (src/'lib.rs').read_text(),'Angela module included')
check(sha(root/'mlbb_heroes.dll')==sha(source/'mlbb_heroes.dll'),'runtime/source DLL identical')
for rel in ['champion/mlbb_angela.data_champion','text/champion.i18n','mod.mod_info']:
 check(sha(root/rel)==sha(source/rel),f'runtime/source {rel} identical')
print(f'PASS {len(checks)} checks; {len(heroes)} heroes, {len(text)} locales, {bindings} animation bindings')
print('Static package validation only. In-game behavior is unverified.')
