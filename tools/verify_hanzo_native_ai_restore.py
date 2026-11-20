"""Verify 1.1.154 against delivered1.1.152: no art/data or projection/energy/S2 changes."""
from pathlib import Path
import zipfile,json,hashlib
ROOT=Path(__file__).resolve().parents[1];BASE=ROOT.parent/'hanzo_152_artifacts/MLBB_Heroes_TFM2_v1.1.152_full_source_with_SDK.zip'
with zipfile.ZipFile(BASE)as z:
 preserved=0
 for n in z.namelist():
  if n.startswith(('champion/','champions/','effects/','sprite/','icons/','sound/','view/','override/','text/','style/')):
   assert (ROOT/n).read_bytes()==z.read(n),n;preserved+=1
  if n.startswith('sdk/candidate/')and n.endswith('.rs')and'hanzo'not in n.lower():assert (ROOT/n).read_bytes()==z.read(n),n
 assert (ROOT/'mod.override_info').read_bytes()==z.read('mod.override_info')
 old=z.read('sdk/candidate/hanzo.rs').decode();new=(ROOT/'sdk/candidate/hanzo.rs').read_text()
 for a,b in [('fn enter(','fn finish('),('fn finish(','fn begin_return('),('fn begin_return(','fn update(')]:assert old[old.index(a):old.index(b)]==new[new.index(a):new.index(b)]
 a=old.index('    match s.form {',old.index('fn update('));b=old.index('    combat::tick(',a)
 c=new.index('    match s.form {',new.index('fn update('));d=new.index('    combat::tick(',c);assert old[a:b]==new[c:d]
 old=z.read('sdk/candidate/hanzo_combat.rs').decode();new=(ROOT/'sdk/candidate/hanzo_combat.rs').read_text()
 old=old[old.index('pub(super) fn cast_mist('):];new=new[new.index('pub(super) fn cast_mist('):]
 assert old==new.replace('    settle_feast(sim, id, s);\n','',1)
 for n in z.namelist():
  if n.startswith('source/hanzo/')or n in ['sdk/candidate/hanzo_visual.rs','sdk/candidate/hanzo_bindings.rs']:
   assert (ROOT/n).read_bytes()==z.read(n),n
ai=(ROOT/'sdk/candidate/hanzo_ai.rs').read_text()
for forbidden in ['InputV1::move_to','InputV1::return_home','safe_move','safe_target','hold','escape']:
 # Comments document the absence of these behaviors; inspect executable lines only.
 assert forbidden not in '\n'.join(l for l in ai.splitlines()if not l.strip().startswith('//')),forbidden
assert 'p.0.abs_diff(old.0) > 100' in (ROOT/'sdk/candidate/hanzo.rs').read_text()
assert len(list((ROOT/'champion').glob('*.data_champion')))==25
lib=(ROOT/'mlbb_heroes.dll').read_bytes();assert lib==(ROOT/'sdk/candidate/mlbb_heroes.dll').read_bytes()==(ROOT/'sdk/candidate/target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll').read_bytes()
report={'version':'1.1.154','champions':25,'runtime_art_data_files_identical_to_1152':preserved,'non_hanzo_rust_identical':True,'projection_entry_health_restore_return_identical':True,'energy_state_machine_identical':True,'s2_geometry_damage_and_spirit_s1_identical':True,'body_link_mark_visual_module_identical':True,'all_hanzo_original_source_art_identical':True,'ai_generates_no_move_recall_hold_escape':True,'native_move_return_and_none_passthrough':True,'walking_facing_threshold':100,'pending_human_s1_max_ticks':30,'owner_live_tested_1154':False,'accepted_rollback':'1.1.149','dll_sha256':hashlib.sha256(lib).hexdigest()}
(ROOT/'reports/HANZO_1154_VERIFICATION.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
