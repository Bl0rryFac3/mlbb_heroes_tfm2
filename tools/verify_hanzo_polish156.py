"""Verify mark lights from source pixels, compact text,0.7.3 roles, buffs and preservation."""
from pathlib import Path
from PIL import Image
import json,zipfile,hashlib,re
ROOT=Path(__file__).resolve().parents[1];BASE=ROOT.parent/'hanzo_155_artifacts/MLBB_Heroes_TFM2_v1.1.155_full_source_with_SDK.zip'
tanks={'mlbb_franco','mlbb_johnson','mlbb_belerick','mlbb_uranus'}
allowed={*(f'champion/{n}.data_champion'for n in tanks),'champion/mlbb_hanzo.data_champion','text/champion.i18n','effects/hanzo_marks_head#anim.fanim','effects/hanzo_anim_hanzo_spirit_maw#anim.fanim'}
with zipfile.ZipFile(BASE)as z:
 kept=0
 for n in z.namelist():
  if n.startswith(('champion/','champions/','effects/','sprite/','icons/','sound/','view/','override/','text/','style/'))and n not in allowed:
   assert (ROOT/n).read_bytes()==z.read(n),n;kept+=1
  if n.startswith('source/hanzo/animations/')or(n.startswith('sdk/candidate/')and n.endswith('.rs')and'hanzo'not in n.lower()):assert (ROOT/n).read_bytes()==z.read(n),n
 old=z.read('sdk/candidate/hanzo.rs').decode();new=(ROOT/'sdk/candidate/hanzo.rs').read_text()
 for a,b in [('fn enter(','fn finish('),('fn finish(','fn begin_return('),('fn begin_return(','fn update(')]:assert old[old.index(a):old.index(b)]==new[new.index(a):new.index(b)]
 a=old.index('    match s.form {',old.index('fn update('));b=old.index('    combat::tick(',a);c=new.index('    match s.form {',new.index('fn update('));d=new.index('    combat::tick(',c);assert old[a:b]==new[c:d]
 old=z.read('sdk/candidate/hanzo_ai.rs').decode();new=(ROOT/'sdk/candidate/hanzo_ai.rs').read_text();assert new==old.replace('80 + e.stat().attack * 60 / 100','110 + e.stat().attack * 80 / 100')
 for name in tanks:
  old=json.loads(z.read(f'champion/{name}.data_champion'));new=json.loads((ROOT/f'champion/{name}.data_champion').read_text());old['category']='Tank'
  if'Tank'not in old['tags']:old['tags'].append('Tank')
  assert old==new,name
 previous_text=json.loads(z.read('text/champion.i18n'))
text=json.loads((ROOT/'text/champion.i18n').read_text());lengths={}
for lang,loc in text.items():
 for group,entries in previous_text[lang].items():
  if isinstance(entries,dict):
   for k,v in entries.items():
    if k!='mlbb_hanzo':assert loc[group][k]==v,(lang,group,k)
 desc=loc['description']['mlbb_hanzo']
 for k,v in desc.items():
  plain=re.sub(r'<[^>]*>','',v)
  assert not re.search(r'\b(you|your|yours)\b',plain,re.I)
  if k not in ['name','attack']:assert len(plain)<=425 and'<#'in v
  if lang=='en':lengths[k]=len(plain)
# Count bright connected clusters, excluding tiny isolated sparkles. This verifies source order.
im=Image.open(ROOT/'source/hanzo/animations/hanzo_dark_marks.png').convert('RGBA');counts=[]
for i in range(5):
 cell=im.crop((i%3*32,i//3*32,(i%3+1)*32,(i//3+1)*32));pts={(x,y)for y in range(32)for x in range(32)if(lambda c:c[3]>80 and c[0]>160 and c[2]>70 and c[0]>c[1]*1.5)(cell.getpixel((x,y)))};n=0
 while pts:
  q=[pts.pop()];size=0
  while q:
   x,y=q.pop();size+=1
   for dy in [-1,0,1]:
    for dx in [-1,0,1]:
     t=x+dx,y+dy
     if t in pts:pts.remove(t);q.append(t)
  if size>=15:n+=1
 counts.append(n)
assert counts==[1,3,5,2,4]
meta=json.loads((ROOT/'effects/hanzo_marks_head#anim.fanim').read_text());order=[meta['anims'][f'mark_{n}']['frames'][0]['data']['x']//32 for n in range(1,6)]
assert [counts[i]for i in order]==[1,2,3,4,5]
maw=json.loads((ROOT/'effects/hanzo_anim_hanzo_spirit_maw#anim.fanim').read_text());assert maw['anchor_y']==.75 and abs(sum(f['duration']for f in maw['anims']['main']['frames'])-.6)<1e-6
roles={}
for p in (ROOT/'champion').glob('*.data_champion'):
 d=json.loads(p.read_text());roles[d['id']]=d['category']
 if d['id'] in tanks|{'mlbb_hanzo'}:assert p.read_bytes()==(ROOT/'sdk/candidate/champion'/p.name).read_bytes()
assert {n for n,r in roles.items()if r=='Tank'}==tanks
h=json.loads((ROOT/'champion/mlbb_hanzo.data_champion').read_text());assert(h['stat']['attack'],h['stat']['hp'],h['growth']['attack'],h['growth']['hp'])==(100,770,6,95)
assert json.loads((ROOT/'mod.mod_info').read_text())['dependencies'][0]['version']=='>=0.7.3'
lib=(ROOT/'mlbb_heroes.dll').read_bytes();assert lib==(ROOT/'sdk/candidate/mlbb_heroes.dll').read_bytes()==(ROOT/'sdk/candidate/target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll').read_bytes()
r={'version':'1.1.156','requires_tfm2':'0.7.3+','accepted_rollback':'1.1.154','preserved_runtime_art_data_files':kept,'movement_recall_and_projection_energy_preserved':True,'non_hanzo_rust_preserved':True,'source_light_counts':counts,'mark_state_map':order,'display_light_counts':[counts[i]for i in order],'tooltip_plain_character_lengths':lengths,'tank_roles':sorted(tanks),'all_role_categories':roles,'maw_seconds':.6,'maw_anchor_y':.75,'base_stats':h['stat'],'growth':h['growth'],'live_1156_tested':False,'dll_sha256':hashlib.sha256(lib).hexdigest()}
(ROOT/'reports/HANZO_1156_VERIFICATION.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
