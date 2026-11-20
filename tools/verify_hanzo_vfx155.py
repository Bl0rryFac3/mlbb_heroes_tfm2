"""Narrow preservation/pixel audit for the owner-accepted1.1.154 polish."""
from pathlib import Path
from PIL import Image
import zipfile,json,hashlib
ROOT=Path(__file__).resolve().parents[1];BASE=ROOT.parent/'hanzo_154_artifacts/MLBB_Heroes_TFM2_v1.1.154_full_source_with_SDK.zip'
changed={'champion/mlbb_hanzo.data_champion','effects/hanzo_marks_head#anim.fanim'}
with zipfile.ZipFile(BASE)as z:
 preserved=0
 for n in z.namelist():
  if n.startswith(('champion/','champions/','effects/','sprite/','icons/','sound/','view/','override/','text/','style/'))and n not in changed:
   assert (ROOT/n).read_bytes()==z.read(n),n;preserved+=1
  if n.startswith('source/hanzo/animations/')or(n.startswith('sdk/candidate/')and n.endswith('.rs')and'hanzo'not in n.lower()):assert (ROOT/n).read_bytes()==z.read(n),n
 for n in ['hanzo_ai.rs','hanzo_bindings.rs']:assert (ROOT/'sdk/candidate'/n).read_bytes()==z.read('sdk/candidate/'+n)
 old=z.read('sdk/candidate/hanzo.rs').decode();assert (ROOT/'sdk/candidate/hanzo.rs').read_text()==old.replace('candidate1.1.154','candidate1.1.155')
 old=z.read('sdk/candidate/hanzo_visual.rs').decode();assert (ROOT/'sdk/candidate/hanzo_visual.rs').read_text()==old.replace('(s.body.0, s.body.1.saturating_sub(24000)),','s.body,',1)
 old=z.read('sdk/candidate/hanzo_combat.rs').decode();new=(ROOT/'sdk/candidate/hanzo_combat.rs').read_text()
 expected=old.replace('''            if contact(sim, id, s, target, z.damage, AttackTypeV1::BaseAttack, true) {
                mark(s, k, now);''','''            // One mark per valid positive mist pulse, matching accepted basic-contact bookkeeping.
            // The host can process damage after this callback's immutable read view.
            if z.damage > 0 { mark(s, k, now); }
            if contact(sim, id, s, target, z.damage, AttackTypeV1::BaseAttack, true) {''');assert new==expected
 old=json.loads(z.read('champion/mlbb_hanzo.data_champion'));new=json.loads((ROOT/'champion/mlbb_hanzo.data_champion').read_text())
 for k in old:
  if k!='view_effects':assert old[k]==new[k],k
 for a,b in zip(old['view_effects'],new['view_effects']):
  if not a['name'].startswith('mlbb_hanzo_overlay_hanzo_body_ward_'):assert a==b,a['name']
 assert (ROOT/'champion/mlbb_hanzo.data_champion').read_bytes()==(ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion').read_bytes()
 marks=json.loads((ROOT/'effects/hanzo_marks_head#anim.fanim').read_text())
 assert [marks['anims'][f'mark_{i}']['frames'][0]['data']['x']//32 for i in range(1,6)]==[0,3,4,2,1]
ward=Image.open(ROOT/'effects/hanzo_body_ward_soft#sheet.png').convert('RGBA');assert ward.size==(396,44)
assert ward.getchannel('A').getextrema()[1]<=141
for i in range(9):
 cell=ward.crop((44*i,0,44*i+44,44));assert cell.getchannel('A').getbbox()
 assert cell.crop((18,16,26,24)).getchannel('A').getextrema()[1]<=3
meta=json.loads((ROOT/'effects/hanzo_body_ward_soft#anim.fanim').read_text());assert meta['anchor_y']==1. and meta['anchor_x']==.5
lib=(ROOT/'mlbb_heroes.dll').read_bytes();assert lib==(ROOT/'sdk/candidate/mlbb_heroes.dll').read_bytes()==(ROOT/'sdk/candidate/target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll').read_bytes()
r={'version':'1.1.155','accepted_rollback':'1.1.154','preserved_runtime_art_data_files':preserved,'ai_recall_facing_s1_projection_energy_identical':True,'combat_only_delta':'one mark per valid positive trail pulse outside inline HP gate','character_pixels_and_original_owner_sheets_identical':True,'mark_source_state_map':[0,3,4,2,1],'mark_display_lights':[1,2,3,4,5],'ward_cell_px':44,'ward_alpha_max':ward.getchannel('A').getextrema()[1],'ward_center_transparent':True,'ward_center_above_body_foot_px':22,'live_1155_tested':False,'dll_sha256':hashlib.sha256(lib).hexdigest()}
(ROOT/'reports/HANZO_1155_VERIFICATION.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
