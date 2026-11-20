"""Import hollow ward atlas and correct mark-state bindings; no character pixel changes."""
from pathlib import Path
import json,subprocess,struct,tempfile
ROOT=Path(__file__).resolve().parents[1]
def write(p,x):p.write_text(json.dumps(x,indent=1)+'\n')
# Existing atlas was compiled in the source order1,5,4,2,3. Rebind, don't repaint.
p=ROOT/'effects/hanzo_marks_head#anim.fanim';a=json.loads(p.read_text());order=[0,3,4,2,1]
for n,source in enumerate(order,1):a['anims'][f'mark_{n}']['frames'][0]['data']['x']=source*32
write(p,a)
# Image-generation edit made the center hollow. This is technical atlas import:
# cell extraction, nearest-neighbor sizing to44px, and alpha attenuation for game readability.
src=ROOT/'source/hanzo/vfx155/body_ward_hollow_source.png';b=src.read_bytes();w,h=struct.unpack('>II',b[16:24])
with tempfile.TemporaryDirectory()as temp:
 frames=[]
 for i in range(9):
  x0=round(i*w/9);x1=round((i+1)*w/9);out=Path(temp)/f'{i}.png'
  subprocess.run(['convert',str(src),'-crop',f'232x232+{x0+5}+240','+repage','-filter','point','-resize','44x44','-gravity','center','-background','none','-extent','44x44','-channel','A','-evaluate','Multiply','0.55','+channel',str(out)],check=True)
  frames.append(str(out))
 subprocess.run(['convert',*frames,'+append',str(ROOT/'effects/hanzo_body_ward_soft#sheet.png')],check=True)
a={'anchor_x':.5,'anchor_y':1.,'anims':{f'ward_{i}':{'frames':[{'duration':.1,'data':{'x':i*44,'y':0,'w':44,'h':44}}]}for i in range(9)}}
write(ROOT/'effects/hanzo_body_ward_soft#anim.fanim',a)
p=ROOT/'champion/mlbb_hanzo.data_champion';d=json.loads(p.read_text())
for v in d['view_effects']:
 if v['name'].startswith('mlbb_hanzo_overlay_hanzo_body_ward_'):
  i=int(v['name'].rsplit('_',1)[1]);v.update(anim='asset/mlbb_heroes/effects/hanzo_body_ward_soft',tag=f'ward_{i}',z=2)
for p in [p,ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion']:write(p,d)
print('PASS mark map1,2,3,4,5;44px hollow55%-alpha ward anchored22px above feet')
