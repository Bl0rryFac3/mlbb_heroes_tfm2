from pathlib import Path
from PIL import Image
import json,re,hashlib
r=Path(__file__).resolve().parents[1]
champ=json.loads((r/'champion/mlbb_granger.data_champion').read_text())
assert (r/'champion/mlbb_granger.data_champion').read_bytes()==(r/'sdk/candidate/champion/mlbb_granger.data_champion').read_bytes()
assert champ['category']=='Range'and champ['tags']==['AD','Range']
for slot,op in [('attack','basic'),('skill','burst'),('skill2','rondo'),('ult','cannon')]:
 assert champ[slot]['effect']=={'type':'Native','effect_ref':f'mlbb_heroes:granger_{op}'}
 assert f'("{op}",' in (r/'sdk/candidate/granger.rs').read_text()
m=r/'source/granger/received/granger_master.png'
assert hashlib.sha256(m.read_bytes()).hexdigest()=='673947eeba9faee054c4d840d7ae1f65ba3d5f0b884b0746765e4f7065b691cb'
im=Image.open(r/'source/granger/animations/granger_idle.png').convert('RGBA').crop((0,0,72,72))
sheet=Image.open(r/'sprite/granger_probe#sheet.png').convert('RGBA')
assert sheet.crop((0,0,72,72)).tobytes()==im.tobytes()
a=json.loads((r/'sprite/granger_probe#anim.fanim').read_text())
assert a['anims']['idle']['frames'][0]['duration']==.000001
assert sheet.crop((72,0,144,72)).getchannel('A').getbbox() is None
view=json.loads((r/'style/champion_view.champion_view').read_text())['entries']['mlbb_granger']
x=int(30+view['face']['x']);y=int(60+view['face']['y']);assert sheet.crop((x-7,y-7,x+7,y+7)).getchannel('A').getbbox()
for slot in ['skill','skill2','ult']:
 icon=Image.open(r/f'icons/granger_probe_{slot}.png');assert icon.size==(100,100)and icon.getbbox()
for motion in ['idle','run','fire','rondo','cannon','roll','hit','dead']:
 for suffix in ['', '_left']:
  stem=r/f'effects/granger_anim_{motion}{suffix}'
  im=Image.open(str(stem)+'#sheet.png').convert('RGBA');anim=json.loads(Path(str(stem)+'#anim.fanim').read_text())
  frames=anim['anims']['main']['frames'];assert len(frames)==(13 if motion=='dead'else 9)
  for frame in frames:
   f=frame['data'];bbox=im.crop((f['x'],f['y'],f['x']+f['w'],f['y']+f['h'])).getbbox();assert bbox and bbox[3]-bbox[1]<=48
for v in champ['view_buffs']+champ['view_effects']+champ['view_projectiles']:
 stem=r/v['anim'].removeprefix('asset/mlbb_heroes/')
 a=json.loads(Path(str(stem)+'#anim.fanim').read_text());assert v['tag']in a['anims']
 assert Path(str(stem)+'#sheet.png').is_file()
text=json.loads((r/'text/champion.i18n').read_text());assert len(text)==8
for locale,body in text.items():
 desc=body['description']['mlbb_granger'];assert desc['name']=='Granger'
 assert '28 + 40% AD' in desc['skill'] and '130 + 105% AD' in desc['ult']
 for field,s in desc.items():
  assert not re.search(r'\b(you|your|yours|test|probe|diagnostic|supplied|unfinished)\b',s,re.I),(locale,field)
  if field!='name':assert '<#' in s and len(re.findall(r'<#[0-9a-f]{8}>',s))==s.count('<>'),(locale,field)
code=(r/'sdk/candidate/granger.rs').read_text()
assert not any(token in code for token in ['ui_find','ui_get','ui_set','ui_tree','draw_set_camera'])
for hook in ['pre_update(ctx, dt)','post_update(ctx, dt)','on_end()']:assert 'VoicePathExtension.'+hook in code
assert (r/'mlbb_heroes.dll').read_bytes()==(r/'sdk/candidate/mlbb_heroes.dll').read_bytes()
assert (r/'mlbb_heroes.dll').read_bytes()[:2]==b'MZ'
assert json.loads((r/'mod.mod_info').read_text())['version']=='1.1.158'
assert 'version = "1.1.158"'in(r/'sdk/candidate/Cargo.toml').read_text()
print('PASS: mirror data, effect routes, source pixels, native preview face crop, all 8 body clips at <=48px, 3 official icons, 8 neutral colored locale entries, voice-hook forwarding, no UI scanners, PE DLL/version parity')
