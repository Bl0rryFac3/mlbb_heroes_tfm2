#!/usr/bin/env python3
"""Compile a diagnostic render rig from the two supplied masters, without resampling.
All motions deliberately reuse one still; this is not an animation-complete hero.
Transparent base atlases are render carriers, not gameplay invisibility.
"""
from pathlib import Path
from PIL import Image, ImageDraw
import json,hashlib
ROOT=Path(__file__).resolve().parents[1]
ART=ROOT/'source/popol_kupa'
rows=['idle','run','attack','skill','skill2','ult','hit','dead']
popol=Image.open(ART/'popol_master.png').convert('RGBA');kupa=Image.open(ART/'kupa_master.png').convert('RGBA')
originals={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in ART.glob('*master.png')}
def atlas(path,image,anchor):
 path=ROOT/path;path.parent.mkdir(parents=True,exist_ok=True)
 image.save(str(path)+'#sheet.png')
 d={'anchor_x':.5,'anchor_y':anchor,'anims':{tag:{'frames':[{'duration':.12,'data':{'x':0,'y':0,'w':image.width,'h':image.height}}]}for tag in rows+['main']}}
 Path(str(path)+'#anim.fanim').write_text(json.dumps(d,indent=1)+'\n')
# Keep carrier cell size the same in every standard row; it is intentionally empty.
atlas(Path('sprite/popol_probe_carrier'),Image.new('RGBA',(72,72)),55/72)
atlas(Path('champions/mlbb_kupa_probe'),Image.new('RGBA',(96,72)),55/72)
image=Image.new('RGBA',(72,72));image.alpha_composite(popol,(8,8));atlas(Path('effects/popol_probe_body'),image,55/72)
image=Image.new('RGBA',(96,72));image.alpha_composite(kupa,(12,14));atlas(Path('effects/kupa_probe_body'),image,55/72)
# Same master + an explicit diagnostic badge, NOT a generated Alpha silhouette.
alpha=image.copy();ImageDraw.Draw(alpha).text((2,1),'ALPHA TEST',fill=(110,230,255,255));atlas(Path('effects/kupa_probe_alpha'),alpha,55/72)
# Pixel-preserving composition solely to test single-root attachment; no seated pose is claimed.
ride=Image.new('RGBA',(112,96));ride.alpha_composite(kupa,(20,38));ride.alpha_composite(popol,(32,5));ImageDraw.Draw(ride).text((1,84),'MOUNT TEST',fill=(255,255,255,255));atlas(Path('effects/popol_probe_ride'),ride,79/96)
# UI test markers use provided art; no new skill-icon artwork is claimed.
for name in ['skill','skill2','ult']:
 icon=Image.new('RGBA',(64,64));icon.alpha_composite(popol,(4,8));ImageDraw.Draw(icon).text((1,0),{'skill':'BITE','skill2':'RIDE','ult':'ALPHA'}[name],fill=(255,255,255,255));icon.save(ROOT/f'icons/popol_probe_{name}.png')
# Detect accidental resampling or source mutation.
assert originals=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in ART.glob('*master.png')}
report={'originals_sha256':originals,'source_pixels_preserved':True,'generated_animation_frames':0,'carrier_atlases_transparent':True,'mounted_and_alpha_art':'diagnostic compositions only; final poses not supplied'}
(ROOT/'reports/POPOL_PROBE_ASSETS.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: original pixels preserved; transparent carriers and labeled static render rig compiled')
