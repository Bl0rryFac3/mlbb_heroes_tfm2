"""Apply 1.1.152 metadata: nonmoving basic cycle and pixel-anchored head marks."""
from pathlib import Path
from PIL import Image
import json
ROOT=Path(__file__).resolve().parents[1]
src=Image.open(ROOT/'effects/hanzo_anim_hanzo_dark_marks#sheet.png').convert('RGBA');out=Image.new('RGBA',(160,96))
for i in range(5):out.paste(src.crop((i*32,0,i*32+32,32)),(i*32,16))
out.save(ROOT/'effects/hanzo_marks_head#sheet.png')
anims={f'mark_{i+1}':{'frames':[{'duration':.2,'data':{'x':i*32,'y':0,'w':32,'h':96}}]}for i in range(5)}
(ROOT/'effects/hanzo_marks_head#anim.fanim').write_text(json.dumps({'anchor_x':.5,'anchor_y':80/96,'anims':anims},indent=1)+'\n')
p=ROOT/'champion/mlbb_hanzo.data_champion';d=json.loads(p.read_text());d['attack'].update(duration=18,cancelable=False,can_use_with_move=False)
for v in d['view_effects']:
 if v['name'].startswith('mlbb_hanzo_overlay_mark_'):v.update(anim='asset/mlbb_heroes/effects/hanzo_marks_head',tag=v['name'].removeprefix('mlbb_hanzo_overlay_'),z=5)
for p in [p,ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion']:p.write_text(json.dumps(d,indent=1)+'\n')
print('PASS:18-tick nonmoving/noncancelable attack; original mark pixels padded above target at z5')
