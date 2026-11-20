#!/usr/bin/env python3
from pathlib import Path
from PIL import Image
import json,hashlib,zipfile
r=Path(__file__).resolve().parents[1]
d=json.loads((r/'champion/mlbb_popol_kupa.data_champion').read_text())
for mirror in ['sdk/champion','sdk/candidate/champion']:
 assert (r/mirror/'mlbb_popol_kupa.data_champion').read_bytes()==(r/'champion/mlbb_popol_kupa.data_champion').read_bytes()
for stem in ['champions/mlbb_kupa_probe']:
 im=Image.open(str(r/stem)+'#sheet.png');assert im.getchannel('A').getextrema()==(0,0)
carrier=Image.open(str(r/'sprite/popol_probe_carrier')+'#sheet.png').convert('RGBA')
meta=json.loads(Path(str(r/'sprite/popol_probe_carrier')+'#anim.fanim').read_text())
assert carrier.crop((0,0,72,72)).getchannel('A').getbbox() is not None
for tag,anim in meta['anims'].items():
 for index,frame in enumerate(anim['frames']):
  b=frame['data'];pixels=carrier.crop((b['x'],b['y'],b['x']+b['w'],b['y']+b['h']))
  if tag=='idle' and index==0:assert frame['duration']<=.000001
  else:assert pixels.getchannel('A').getextrema()==(0,0)
for binding in d['view_buffs']:
 stem=r/binding['anim'].removeprefix('asset/mlbb_heroes/');im=Image.open(str(stem)+'#sheet.png');a=json.loads(Path(str(stem)+'#anim.fanim').read_text());assert binding['tag'] in a['anims'];assert im.getchannel('A').getextrema()[0]==0 and im.getchannel('A').getextrema()[1]>0
 for frame in a['anims'][binding['tag']]['frames']:
  b=frame['data'];assert 0<=b['x'] and 0<=b['y'] and b['x']+b['w']<=im.width and b['y']+b['h']<=im.height
for entry in json.loads((r/'source/popol_kupa/spritecook-assets.json').read_text()):
 p=r/'source/popol_kupa'/(entry['label']+'.png');assert hashlib.sha256(p.read_bytes()).hexdigest()[:12]==entry['sha12']
for locale,body in json.loads((r/'text/champion.i18n').read_text()).items():assert 'mlbb_popol_kupa' in body['description'] and 'mlbb_popol_kupa' in body['skill_name']
print('PASS: body/carrier alpha, frame bounds, raw source hashes, data mirrors and 8 locale entries')
