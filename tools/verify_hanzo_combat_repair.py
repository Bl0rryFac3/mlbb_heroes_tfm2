"""Verify 1.1.152 preservation and metadata against the delivered 1.1.151 source archive."""
from pathlib import Path
from PIL import Image
import zipfile,json,hashlib
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT.parent/'hanzo_151_artifacts/MLBB_Heroes_TFM2_v1.1.151_full_source_with_SDK.zip'
sha=lambda b:hashlib.sha256(b).hexdigest()
with zipfile.ZipFile(BASE)as z:
 count=0
 for n in z.namelist():
  p=ROOT/n
  if n.startswith(('sound/','champion/','champions/','effects/','sprite/','icons/','view/','override/','text/','style/')) and 'hanzo'not in n.lower():
   assert p.read_bytes()==z.read(n),n;count+=1
  if n.startswith('sdk/candidate/')and n.endswith('.rs')and 'hanzo'not in n.lower():assert p.read_bytes()==z.read(n),n
 assert (ROOT/'mod.override_info').read_bytes()==z.read('mod.override_info')
 old=z.read('sdk/candidate/hanzo.rs').decode();new=(ROOT/'sdk/candidate/hanzo.rs').read_text()
 for a,b in [('fn enter(','fn finish('),('fn finish(','fn begin_return('),('fn begin_return(','fn update(')]:assert old[old.index(a):old.index(b)]==new[new.index(a):new.index(b)]
 old=z.read('sdk/candidate/hanzo_combat.rs').decode();new=(ROOT/'sdk/candidate/hanzo_combat.rs').read_text()
 assert old[old.index('pub(super) fn cast_mist('):]==new[new.index('pub(super) fn cast_mist('):]
 for n in z.namelist():
  if n.startswith(('source/hanzo/','sprite/hanzo','icons/hanzo','effects/hanzo')):
   assert (ROOT/n).read_bytes()==z.read(n),n
p=ROOT/'champion/mlbb_hanzo.data_champion';data=json.loads(p.read_text());assert p.read_bytes()==(ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion').read_bytes()
a=data['attack'];assert a['duration']==18 and not a['cancelable']and not a['can_use_with_move'];assert a['range']==18000 and a['cooltime']==54
for v in data['view_effects']:
 anim=json.loads((ROOT/(v['anim'].removeprefix('asset/mlbb_heroes/')+'#anim.fanim')).read_text());assert v['tag']in anim['anims']
 if v['name'].startswith('mlbb_hanzo_overlay_mark_'):assert v['z']==5 and anim['anchor_y']==80/96
src=Image.open(ROOT/'effects/hanzo_anim_hanzo_dark_marks#sheet.png').convert('RGBA');head=Image.open(ROOT/'effects/hanzo_marks_head#sheet.png').convert('RGBA')
for i in range(5):assert head.crop((i*32,16,i*32+32,48)).tobytes()==src.crop((i*32,0,i*32+32,32)).tobytes()
audit=json.loads((ROOT/'reports/HANZO_1150_ANIMATION_AUDIT.json').read_text())
for name in audit:
 if name.startswith('hanzo_'):continue
 stem=ROOT/'effects'/('hanzo_anim_'+name);im=Image.open(str(stem)+'#sheet.png');meta=json.loads(Path(str(stem)+'#anim.fanim').read_text())
 for f in meta['anims']['main']['frames']:
  d=f['data'];b=im.crop((d['x'],d['y'],d['x']+d['w'],d['y']+d['h'])).getchannel('A').getbbox();assert b[3]-b[1]<=48
for x in json.loads((ROOT/'source/hanzo/animations/INTAKE_MANIFEST.json').read_text()):assert sha((ROOT/'source/hanzo/animations'/x['file']).read_bytes())==x['sha256']
assert len(list((ROOT/'champion').glob('*.data_champion')))==25
lib=(ROOT/'mlbb_heroes.dll').read_bytes();assert lib==(ROOT/'sdk/candidate/mlbb_heroes.dll').read_bytes()==(ROOT/'sdk/candidate/target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll').read_bytes()
report={'version':'1.1.152','champions':25,'preserved_non_hanzo_runtime_files':count,'all_non_hanzo_rust_identical':True,'projection_entry_restore_return_identical':True,'s2_cast_geometry_damage_energy_identical':True,'all_original_hanzo_sheets_and_existing_compiled_art_identical':True,'mark_pixels_preserved_with_head_padding':True,'native_attack_duration':18,'native_attack_move_cancel':False,'character_height_max':48,'dll_sha256':sha(lib),'owner_accepted_1152':False,'accepted_rollback':'1.1.149','previous_1151':'blocked by owner combat retest'}
(ROOT/'reports/HANZO_1152_VERIFICATION.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
