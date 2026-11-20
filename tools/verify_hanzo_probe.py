"""Evidence for the projection candidate; never labels mock tests as engine proof."""
from pathlib import Path
import json,hashlib,re
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
base=ROOT.parent/'granger_release_v250'
unchanged=[]
# Exact preservation of every old hero's data, graphics, icons, sounds and source.
for folder in ['champion','champions','effects','sprite','icons','sound','view','override','patches']:
 for p in (base/folder).rglob('*'):
  if p.is_file():assert sha(p)==sha(ROOT/p.relative_to(base)),p;unchanged.append(str(p.relative_to(base)))
for p in (base/'sdk/candidate').glob('*.rs'):
 if p.name not in ['lib.rs','zhask.rs']:assert sha(p)==sha(ROOT/p.relative_to(base)),p
old=(base/'sdk/candidate/lib.rs').read_text();new=(ROOT/'sdk/candidate/lib.rs').read_text()
assert new==old.replace('mod granger;','mod granger;\nmod hanzo;').replace('module.set_extension(granger::Extension);','hanzo::register(&mut module);\n    module.set_extension(hanzo::Extension);')
assert (ROOT/'sdk/candidate/zhask.rs').read_text()==(base/'sdk/candidate/zhask.rs').read_text().replace('crate::granger::tick_all(sim);','crate::granger::tick_all(sim);\n        crate::hanzo::tick_all(sim);')
assert sha(ROOT/'mod.override_info')==sha(base/'mod.override_info')
oldtext=json.loads((base/'text/champion.i18n').read_text());newtext=json.loads((ROOT/'text/champion.i18n').read_text())
for lang,locale in oldtext.items():
 for group,entries in locale.items():
  for k,v in entries.items():assert newtext[lang][group][k]==v,(lang,group,k)
 d=newtext[lang]['description']['mlbb_hanzo'];assert '<#'in d['ult'];assert not re.search(r'\b(you|your|yours)\b',' '.join(d.values()),re.I)
assert len(list((ROOT/'champion').glob('*.data_champion')))==25
assert (ROOT/'champion/mlbb_hanzo.data_champion').read_bytes()==(ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion').read_bytes()
assert sha(ROOT/'mlbb_heroes.dll')==sha(ROOT/'sdk/candidate/mlbb_heroes.dll')==sha(ROOT/'sdk/candidate/target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll')
asset=json.loads((ROOT/'reports/HANZO_ASSET_AUDIT_1149.json').read_text());assert all(a['native_pixels_preserved']and a['visible_height']<=48 for a in asset.values())
for f in ['human','spirit']:
 right=Image.open(ROOT/f'effects/hanzo_probe_{f}#sheet.png').convert('RGBA');left=Image.open(ROOT/f'effects/hanzo_probe_{f}_left#sheet.png').convert('RGBA')
 assert left.tobytes()==right.transpose(Image.Transpose.FLIP_LEFT_RIGHT).tobytes()
for slot,file in [('skill','6910.png'),('skill2','6920.png'),('ult','6930.png')]:
 ref=ROOT.parent/'hanzo_phase0/official_icons'/file
 if ref.exists():assert sha(ROOT/f'icons/hanzo_{slot}.png')==sha(ref)
report={'version':'1.1.149','champions':25,'preserved_runtime_files':len(unchanged),'existing_hero_data_art_audio_unchanged':True,'old_hero_rust_unchanged_except_additive_lib_and_match_hook':True,'typed_overrides_unchanged':True,'old_locale_entries_unchanged':True,'hanzo_locale_entries':len(newtext),'native_master_pixels_preserved':True,'dll_sha256':sha(ROOT/'mlbb_heroes.dll'),'live_tfm2_tested':False,'lethal_interception_engine_verified':False,'parked_body_renderer_verified':False}
(ROOT/'reports/HANZO_1149_VERIFICATION.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
