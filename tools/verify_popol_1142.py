#!/usr/bin/env python3
from pathlib import Path
from PIL import Image
import json,re,hashlib,wave
r=Path(__file__).resolve().parents[1]
d=json.loads((r/'text/champion.i18n').read_text())
for locale,body in d.items():
 p=body['description']['mlbb_popol_kupa'];assert p['name']=='Popol & Kupa'
 for field,text in p.items():
  assert not re.search(r'\b(you|your|yours|test|unfinished|supplied|probe|diagnostic)\b',text,re.I),(locale,field)
  if field!='name':
   assert '<#' in text,(locale,field)
   assert len(re.findall(r'<#[0-9a-f]{8}>',text))==text.count('<>'),(locale,field)
 for name in body['skill_name']['mlbb_popol_kupa'].values():assert 'TEST' not in name
for suffix in ['skill','skill2','ult']:
 im=Image.open(r/f'icons/popol_probe_{suffix}.png');assert im.size==(96,96);assert im.getbbox()
for info in (r/'sound/sfx').glob('mlbb_popol*.sound_info'):
 for play in json.loads(info.read_text())['plays']:assert (info.parent/(play['clip']+'.wav')).is_file()
with wave.open(str(r/'sound/sfx/mlbb_popol_voice_intro.wav')) as w:
 assert w.getnchannels()==1 and w.getframerate()==22050 and w.getsampwidth()==2
 assert w.getnframes()/w.getframerate()*60<=218
print('PASS: 8 neutral colored locale entries, test labels removed, 3 source icons, valid intro sound references/duration')
