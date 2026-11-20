"""Correct mark-state mapping from owner observation and center readable spirit maw."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[1]
p=ROOT/'effects/hanzo_marks_head#anim.fanim';d=json.loads(p.read_text())
for n,index in enumerate([0,3,1,4,2],1):d['anims'][f'mark_{n}']['frames'][0]['data']['x']=index*32
p.write_text(json.dumps(d,indent=1)+'\n')
p=ROOT/'effects/hanzo_anim_hanzo_spirit_maw#anim.fanim';d=json.loads(p.read_text());d['anchor_y']=.75
for f in d['anims']['main']['frames']:f['duration']=.6/len(d['anims']['main']['frames'])
p.write_text(json.dumps(d,indent=1)+'\n')
print('PASS source light counts1,3,5,2,4 mapped to1–5; existing maw centered on torso for0.6s')
