#!/usr/bin/env python3
"""Compile supplied Angela RGBA art. Deterministic slicing/rotation, no generation."""
from pathlib import Path
import json, shutil
from PIL import Image
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
FX=ROOT/'effects'
SOURCE=ROOT/'source/angela/new_love_wave_original.png'

def write(name,sheet,anims,**extra):
    sheet.save(FX/(name+'#sheet.png'))
    (FX/(name+'#anim.fanim')).write_text(json.dumps(dict(anchor_x=.5,anchor_y=.5,anims=anims,**extra),separators=(',',':'))+'\n')
def frame(x,y,w,h,d=.05):return {'duration':d,'data':dict(x=x,y=y,w=w,h=h)}
img=Image.open(SOURCE).convert('RGBA')
assert img.size==(300,300),img.size
# 3x3 cells, row-major. Preserve common cell center and relative frame sizes.
# The supplied wave travels from upper-right to lower-left; rotate its travel normal to each bucket.
sheet=Image.new('RGBA',(72*9,72*16))
anims={}
for bucket in range(16):
    for n in range(9):
        tile=img.crop((n%3*100,n//3*100,n%3*100+100,n//3*100+100)).resize((72,72),Image.Resampling.NEAREST)
        tile=tile.rotate(135-bucket*22.5,resample=Image.Resampling.NEAREST)
        sheet.alpha_composite(tile,(n*72,bucket*72))
        anims[f'wave_{bucket}_{n}']={'frames':[frame(n*72,bucket*72,72,72)]}
# Keep the original public tag valid for external references too.
anims['love_wave']={'frames':[frame(n*72,0,72,72) for n in range(9)]}
write('angela_love_wave',sheet,anims)
# Match the Estes 12-angle, 48px wire convention using Angela's own string.
old=Image.open(FX/'angela_puppet_string#sheet.png').convert('RGBA')
meta=json.loads((FX/'angela_puppet_string#anim.fanim').read_text())
r=meta['anims']['puppet_string']['frames'][0]['data']
base=old.crop((r['x'],r['y'],r['x']+r['w'],r['y']+r['h']))
# Isolate the supplied thread, determine its dominant axis, then normalize it.
a=np.array(base);ys,xs=np.where(a[:,:,3]>32)
assert len(xs)>3,'empty supplied puppet string'
coords=np.stack((xs,ys),axis=1);eigval,eigvec=np.linalg.eigh(np.cov(coords.T))
v=eigvec[:,np.argmax(eigval)];angle=np.degrees(np.arctan2(v[1],v[0]))
flat=base.rotate(angle,resample=Image.Resampling.NEAREST,expand=True)
flat=flat.crop(flat.getbbox()).resize((48,6),Image.Resampling.NEAREST)
tile=Image.new('RGBA',(48,48));tile.alpha_composite(flat,(0,21))
sheet=Image.new('RGBA',(48*12,48));anims={}
for bucket in range(12):
    sheet.alpha_composite(tile.rotate(-bucket*15,resample=Image.Resampling.NEAREST),(bucket*48,0))
    anims[f'wire_{bucket}']={'frames':[frame(bucket*48,0,48,48)]}
write('angela_puppet_wire',sheet,anims)
# Put the supplied doll above the host by padding the sheet; view bindings have
# no dependable offset field. Short refreshed view effects follow the host.
p=FX/'angela_attached_doll#anim.fanim';meta=json.loads(p.read_text())
old=Image.open(FX/'angela_attached_doll#sheet.png').convert('RGBA')
frames=meta['anims']['attached_doll']['frames']
sheet=Image.new('RGBA',(72*len(frames),112));out=[]
for n,f in enumerate(frames):
    r=f['data'];im=old.crop((r['x'],r['y'],r['x']+r['w'],r['y']+r['h']))
    # Original input retained separately so regeneration never compounds padding.
    sheet.alpha_composite(im,(n*72+(72-im.width)//2,12))
    out.append(frame(n*72,0,72,112,.02))
write('angela_attached_doll_overlay',sheet,{'attached_doll':{'frames':out}},anchor_y_override=1)
# Actual anchor_y rather than a fictional view offset.
p=FX/'angela_attached_doll_overlay#anim.fanim';m=json.loads(p.read_text());m.pop('anchor_y_override');m['anchor_y']=1.0;p.write_text(json.dumps(m,separators=(',',':'))+'\n')
print('Compiled owner S1 grid, Angela thread angle atlas and above-host doll.')
