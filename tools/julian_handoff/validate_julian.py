#!/usr/bin/env python3
"""Strict source/integration checks; --release additionally requires actual art and PE DLL."""
from pathlib import Path
from PIL import Image
import argparse,json,re,struct
H=Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--release',action='store_true');a=p.parse_args();r=a.root
 data=json.loads((r/'champion/mlbb_julian.data_champion').read_text());text=json.loads((r/'text/champion.i18n').read_text());info=json.loads((r/'mod.mod_info').read_text());assert re.fullmatch(r'1\.1\.\d+',info['version']),info['version']
 cargo=(r/'sdk/candidate/Cargo.toml').read_text();assert f'version = "{info["version"]}"' in cargo,'mod_info/Cargo version mismatch'
 assert data['passive']['passive_ref']=='mlbb_heroes:julian_legacy';assert data['tags']==['AP','Magic','Melee','CC']
 for lang in ['en','zh-hans','zh-hant','ko','ja','fr','ru','vi']:
  assert set(text[lang]['description']['mlbb_julian'])=={'name','attack','skill','skill2','ult','passive'}
  assert set(text[lang]['skill_name']['mlbb_julian'])=={'passive','skill1','skill2','ult'}
 names=[v['name'] for v in data['view_effects']];assert len(names)==len(set(names))==333
 assert 'crate::julian::tick_all(sim);' in (r/'sdk/candidate/zhask.rs').read_text();assert 'julian::register(&mut module);' in (r/'sdk/candidate/lib.rs').read_text()
 assert json.loads((r/'sdk/champion/mlbb_julian.data_champion').read_text())==data
 for v in data['view_effects']:
  assert v['type'] in ['Animation','LoopAnimation'];assert v['anim'].startswith('asset/mlbb_heroes/effects/julian_')
  if a.release:
   prefix=r/v['anim'].removeprefix('asset/mlbb_heroes/');image=Image.open(str(prefix)+'#sheet.png');f=json.loads(Path(str(prefix)+'#anim.fanim').read_text());assert v['tag'] in f['anims']
   for entry in f['anims'][v['tag']]['frames']:
    d=entry['data'];assert d['x']>=0 and d['y']>=0 and d['x']+d['w']<=image.width and d['y']+d['h']<=image.height
 if a.release:
  im=Image.open(r/'sprite/julian#sheet.png');assert im.size==(1656,576) and im.mode=='RGBA';assert im.getextrema()[3][0]==0
  f=json.loads((r/'sprite/julian#anim.fanim').read_text());assert len(f['anims'])==8;assert all(len(a['frames'])>=2 for a in f['anims'].values())
  for n in ['skill','skill2','ult']:
   with Image.open(r/'icons'/('julian_'+n+'.png')) as i:assert i.format=='PNG' and i.size==(64,64)
  dll=(r/'mlbb_heroes.dll').read_bytes();assert dll[:2]==b'MZ';pe=struct.unpack_from('<I',dll,0x3c)[0];assert dll[pe:pe+4]==b'PE\0\0';assert struct.unpack_from('<H',dll,pe+22)[0]&0x2000,'not a Windows DLL'
 print('PASS:', 'release art/PE/static checks (live gameplay not tested)' if a.release else 'source integration checks only; returned assets and release DLL NOT validated')
if __name__=='__main__':main()
