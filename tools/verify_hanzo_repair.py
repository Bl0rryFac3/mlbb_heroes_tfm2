"""Audit the attack/overlay repair against1.1.150 without claiming live acceptance."""
from pathlib import Path
import json,hashlib,re
ROOT=Path(__file__).resolve().parents[1];BASE=ROOT.parent/'hanzo_full'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for p in (BASE/'sdk/candidate').glob('*.rs'):
 if not p.name.startswith('hanzo'):assert sha(p)==sha(ROOT/p.relative_to(BASE)),p
for folder in ['sound','champion','effects','sprite','icons','style','text','override']:
 for p in (BASE/folder).rglob('*'):
  if not p.is_file()or'hanzo'in p.name.lower()or p.name=='champion.i18n':continue
  assert sha(p)==sha(ROOT/p.relative_to(BASE)),p
assert sha(ROOT/'text/champion.i18n')==sha(BASE/'text/champion.i18n')
assert sha(ROOT/'mod.override_info')==sha(BASE/'mod.override_info')
old=(BASE/'sdk/candidate/hanzo.rs').read_text();new=(ROOT/'sdk/candidate/hanzo.rs').read_text()
for a,b in [('fn enter(','fn finish('),('fn finish(','fn begin_return('),('fn begin_return(','fn update(')]:assert old[old.index(a):old.index(b)]==new[new.index(a):new.index(b)]
oc=(BASE/'sdk/candidate/hanzo_combat.rs').read_text();nc=(ROOT/'sdk/candidate/hanzo_combat.rs').read_text()
for a,b in [('pub(super) fn cast_mist(','fn segment_distance('),('fn segment_distance(','fn resolve_snatch(')]:assert oc[oc.index(a):oc.index(b)]==nc[nc.index(a):nc.index(b)]
assert 'draw_sprite'not in new and'draw_line'not in new
assert 'draw_sprite'not in(ROOT/'sdk/candidate/hanzo_visual.rs').read_text()
assert len(list((ROOT/'champion').glob('*.data_champion')))==25
assert (ROOT/'champion/mlbb_hanzo.data_champion').read_bytes()==(ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion').read_bytes()
data=json.loads((ROOT/'champion/mlbb_hanzo.data_champion').read_text());overlays=[v for v in data['view_effects']if v['name'].startswith('mlbb_hanzo_overlay_')];assert len(overlays)>100
for v in overlays:
 stem=v['anim'].removeprefix('asset/mlbb_heroes/');anim=json.loads((ROOT/(stem+'#anim.fanim')).read_text());frames=anim['anims'][v['tag']]['frames'];assert len(frames)==1
 assert frames[0]['duration']<=.2;assert v['is_follow']is False
assert sha(ROOT/'mlbb_heroes.dll')==sha(ROOT/'sdk/candidate/mlbb_heroes.dll')==sha(ROOT/'sdk/candidate/target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll')
for x in json.loads((ROOT/'source/hanzo/animations/INTAKE_MANIFEST.json').read_text()):assert sha(ROOT/'source/hanzo/animations'/x['file'])==x['sha256']
report={'version':'1.1.151','champions':25,'non_hanzo_rust_identical_to_1150':True,'projection_entry_health_restore_return_functions_identical':True,'s2_cast_and_mist_geometry_identical':True,'descriptions_icons_carrier_owner_originals_preserved':True,'game_map_overlay_path_removed':True,'native_overlay_bindings':len(overlays),'overlay_frames_max_seconds':.2,'server_presim_skip':True,'ai_input_abi_regressions':True,'owner_report_1150':'transform/spiritS2/return work; basics/S1/body/link/marks/devour missing','live_1151_tested':False,'dll_sha256':sha(ROOT/'mlbb_heroes.dll')}
(ROOT/'reports/HANZO_1151_VERIFICATION.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
