#!/usr/bin/env python3
"""Check actual returned-frame preservation, bindings, descriptions and audio."""
from pathlib import Path
import json,re,sys,copy,hashlib
from PIL import Image
import numpy as np
ROOT=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parents[1]
COUNT=0
def check(ok,msg):
 global COUNT
 if not ok:raise AssertionError(msg)
 COUNT+=1
def bindings(data):
 for block,allowed in [('view_effects',{'Animation','LoopAnimation'}),('view_projectiles',{'Animated','Sprite','ThreePhase'}),('view_buffs',{'Animated','ThreePhase'})]:
  for b in data.get(block,[]):
   check(b['type'] in allowed,f'{block} renderer schema: {b["name"]}')
   base=ROOT/b['anim'].removeprefix('asset/mlbb_heroes/');m=json.loads(Path(str(base)+'#anim.fanim').read_text());im=Image.open(str(base)+'#sheet.png')
   check(b['tag'] in m['anims'],'missing animation tag '+b['tag'])
   for f in m['anims'][b['tag']]['frames']:
    x=f['data'];check(x['x']>=0 and x['y']>=0 and x['w']>0 and x['h']>0 and x['x']+x['w']<=im.width and x['y']+x['h']<=im.height,'frame outside atlas')
def main():
 d=json.loads((ROOT/'champion/mlbb_eudora.data_champion').read_text());bindings(d)
 # Run the same binding validator against a deliberately wrong tag.
 bad=copy.deepcopy(d);bad['view_effects'][0]['tag']='not_a_real_animation'
 try:bindings(bad)
 except AssertionError:pass
 else:raise AssertionError('binding guard accepted an invalid tag')
 check(len(list((ROOT/'champion').glob('*.data_champion')))==21,'roster count')
 check(json.loads((ROOT/'mod.mod_info').read_text())['version']=='1.1.122','internal version')
 check(d['passive']['passive_ref']=='mlbb_heroes:eudora_superconductor','passive selected')
 check(d['skill2']['casting_type']=='Targeting' and d['ult']['casting_type']=='Position','spell targeting')
 # Count and compare visible RGBA pixels, not arbitrary RGB hidden under alpha 0.
 meta=json.loads((ROOT/'sprite/eudora#anim.fanim').read_text());sheet=Image.open(ROOT/'sprite/eudora#sheet.png').convert('RGBA')
 for tag,file in [('idle','idle'),('run','run'),('attack','basicattack'),('skill','1stskill'),('skill2','2ndskill'),('ult','ult'),('hit','hit'),('dead','death')]:
  src=Image.open(ROOT/f'source/eudora/returned/eudora-{file}.png').convert('RGBA');cols=src.width//72
  expected=sum(src.crop((x,y,x+72,y+72)).getbbox() is not None for y in range(0,src.height,72) for x in range(0,src.width,72))
  fs=meta['anims'][tag]['frames'];check(len(fs)==expected,f'{tag}: frame count')
  for i,f in enumerate(fs):
   s=np.asarray(src.crop((i%cols*72,i//cols*72,i%cols*72+72,i//cols*72+72)))
   r=f['data'];a=np.asarray(sheet.crop((r['x'],r['y'],r['x']+72,r['y']+72)))
   check(np.array_equal(s[:,:,3],a[:,:,3]) and np.array_equal(s[s[:,:,3]>0],a[s[:,:,3]>0]),f'{tag}/{i}: changed source pixels')
 check(meta['anchor_y']==55/72,'native roster anchor')
 # Existing non-English wording remains exactly as the canonical baseline.
 texts=json.loads((ROOT/'text/champion.i18n').read_text());old=json.loads((ROOT/'source/eudora/english_original.i18n').read_text())
 check(len(texts)==8,'eight locales')
 for locale,body in texts.items():
  for hero,desc in body['description'].items():
   for field in ['attack','skill','skill2','ult','passive']:
    s=desc[field];check(bool(s.strip()),f'{locale}/{hero}/{field} empty')
    tokens=re.findall(r'<#[0-9a-fA-F]{8}>|<>',s);balance=0
    for t in tokens:
     balance+=-1 if t=='<>' else 1;check(balance>=0,'unbalanced color tag')
    check(balance==0,'unclosed color tag')
    if locale=='en':check(re.search(r'\b(you|your|yours|we|our|I)\b',re.sub('<[^>]*>','',s),re.I) is None,'English perspective')
   if hero!='mlbb_eudora' and locale!='en':check(desc==old[locale]['description'][hero],'existing locale drift')
  check('mlbb_eudora' in body['description'] and 'mlbb_eudora' in body['skill_name'],'Eudora translation missing')
 for f in (ROOT/'icons').glob('eudora_*.png'):
  check(f.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'),'icon is not real PNG');Image.open(f).verify()
 overrides=json.loads((ROOT/'mod.override_info').read_text());voices=list((ROOT/'sound/sfx').glob('mlbb_eudora_voice_*.sound_info'))
 check(len(voices)==14,'14 default voice events')
 for f in voices:
  clip=json.loads(f.read_text())['plays'][0]['clip'];wav=ROOT/'sound/sfx'/(clip+'.wav')
  check(wav.read_bytes().startswith(b'RIFF'),'audio is not WAV')
  for name in [f.stem,clip]:check(overrides['asset/base/sound/sfx/'+name]['remapping']=='asset/mlbb_heroes/sound/sfx/'+name,'missing sound override')
 print(f'PASS {COUNT} checks: returned frames, renderer schemas, negative tag guard, English perspective, eight locales and audio routes.')
if __name__=='__main__':main()
