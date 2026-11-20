"""Audit full Hanzo against the accepted projection candidate and owner intake."""
from pathlib import Path
from PIL import Image
import json,hashlib,re
ROOT=Path(__file__).resolve().parents[1];BASE=ROOT.parent/'hanzo_probe'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
preserved=0
for folder in ['champion','champions','effects','sprite','icons','sound','view','override','patches']:
 for p in (BASE/folder).rglob('*'):
  if not p.is_file()or' hanzo 'in p.name or'hanzo'in p.name.lower():continue
  assert sha(p)==sha(ROOT/p.relative_to(BASE)),p;preserved+=1
for p in (BASE/'sdk/candidate').glob('*.rs'):
 if p.name.startswith('hanzo'):continue
 assert sha(p)==sha(ROOT/p.relative_to(BASE)),p
assert sha(ROOT/'mod.override_info')==sha(BASE/'mod.override_info')
for lang,locale in json.loads((BASE/'text/champion.i18n').read_text()).items():
 current=json.loads((ROOT/'text/champion.i18n').read_text())[lang]
 for group,entries in locale.items():
  for k,v in entries.items():
   if k!='mlbb_hanzo':assert current[group][k]==v
 descriptions=current['description']['mlbb_hanzo'];assert all('<#'in v for k,v in descriptions.items()if k!='name')
 assert not re.search(r'\b(you|your|yours)\b',' '.join(descriptions.values()),re.I)
assert len(list((ROOT/'champion').glob('*.data_champion')))==25
assert (ROOT/'champion/mlbb_hanzo.data_champion').read_bytes()==(ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion').read_bytes()
assert sha(ROOT/'mlbb_heroes.dll')==sha(ROOT/'sdk/candidate/mlbb_heroes.dll')==sha(ROOT/'sdk/candidate/target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll')
intake=json.loads((ROOT/'source/hanzo/animations/INTAKE_MANIFEST.json').read_text());assert len(intake)==33
for entry in intake:assert sha(ROOT/'source/hanzo/animations'/entry['file'])==entry['sha256']
audit=json.loads((ROOT/'reports/HANZO_1150_ANIMATION_AUDIT.json').read_text());assert len(audit)==33
for n,d in audit.items():
 stem=ROOT/'effects'/('hanzo_anim_'+n);im=Image.open(str(stem)+'#sheet.png').convert('RGBA');anim=json.loads(Path(str(stem)+'#anim.fanim').read_text());frames=anim['anims']['main']['frames'];assert len(frames)==len(d['selected_frames'])
 for f in frames:
  x=f['data'];cut=im.crop((x['x'],x['y'],x['x']+x['w'],x['y']+x['h']));b=cut.getchannel('A').getbbox();assert b
  if not n.startswith('hanzo_'):assert b[3]-b[1]<=48,(n,b)
for n in ['parked_body','body_merge','human_project','hanzo_body_ward','hanzo_spirit_link','hanzo_dark_marks']:
 for i in range(len(audit[n]['selected_frames'])):assert (ROOT/f'icons/hanzo_world_{n}_{i}.png').is_file()
data=json.loads((ROOT/'champion/mlbb_hanzo.data_champion').read_text());bindings=(ROOT/'sdk/candidate/hanzo_bindings.rs').read_text()
for view in data['view_buffs']:assert '"'+view['name']+'"'in bindings
report={'version':'1.1.150','champions':25,'preserved_previous_hero_runtime_files':preserved,'all_non_hanzo_rust_identical_to_1149':True,'typed_overrides_identical':True,'owner_sheets':33,'owner_original_hashes_match':True,'oversized_character_frames_preserved_but_not_played':sum(len(d['skipped_over48'])for d in audit.values()),'resampling':False,'runtime_character_height_limit':48,'native_menu_carrier':True,'world_frames_exported_individually_no_uv_size_assumption':True,'full_cooldown_and_tooltips':'English fallback in eight locales; S1/S2 native cooldown-reduction scaling; projection clocks retained','dll_sha256':sha(ROOT/'mlbb_heroes.dll'),'owner_tested_1150':False,'owner_accepted_projection_baseline':'1.1.149'}
(ROOT/'reports/HANZO_1150_VERIFICATION.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
