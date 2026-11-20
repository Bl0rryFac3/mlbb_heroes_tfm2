#!/usr/bin/env python3
"""Compile the owner's native-size PixelLab grids; preserve character pixels."""
from pathlib import Path
import json, math, shutil
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
SRC=ROOT/'source/eudora/returned'
FX=ROOT/'effects'; SPR=ROOT/'sprite'
def frame(x,y,w,h,d): return {'duration':d,'data':dict(x=x,y=y,w=w,h=h)}
def read(name,w,h,terminal=False):
 im=Image.open(SRC/name).convert('RGBA');assert im.width%w==0 and im.height%h==0,(name,im.size)
 cells=[im.crop((x,y,x+w,y+h)) for y in range(0,im.height,h) for x in range(0,im.width,w)]
 last=max(i for i,c in enumerate(cells) if c.getbbox())
 assert all(c.getbbox() for c in cells[:last+1]),(name,'interior blank')
 out=cells[:last+1]
 if terminal:out.append(Image.new('RGBA',(w,h)))
 return out
def write(folder,name,im,anims,ax=.5,ay=.5):
 im.save(folder/(name+'#sheet.png'))
 (folder/(name+'#anim.fanim')).write_text(json.dumps(dict(anchor_x=ax,anchor_y=ay,anims=anims),separators=(',',':'))+'\n')
def effect(name,src,w,h,total,terminal=False,ax=.5,ay=.5,pad=None):
 cells=read(src,w,h,terminal)
 if pad:
  prepared=[]
  for c in cells:
   o=Image.new('RGBA',pad);o.alpha_composite(c,((pad[0]-w)//2,(pad[1]-h)//2));prepared.append(o)
  cells=prepared;w,h=pad
 im=Image.new('RGBA',(w*len(cells),h))
 for i,c in enumerate(cells):im.alpha_composite(c,(i*w,0))
 write(FX,name,im,{'play':{'frames':[frame(i*w,0,w,h,total/len(cells)) for i in range(len(cells))]}},ax,ay)
 return cells
def angles(name,cells,side,origin=None,individual=False):
 n=len(cells);im=Image.new('RGBA',(side*n,side*16));anims={}
 for a in range(16):
  fs=[]
  for j,c in enumerate(cells):
   o=Image.new('RGBA',(side,side));ox,oy=origin or (c.width/2,c.height/2)
   o.alpha_composite(c,(round(side/2-ox),round(side/2-oy)))
   o=o.rotate(-a*22.5,Image.Resampling.NEAREST)
   im.alpha_composite(o,(j*side,a*side));f=frame(j*side,a*side,side,side,.04);fs.append(f)
   if individual:anims[f'dir_{a}_{j}']={'frames':[f]}
  anims[f'dir_{a}']={'frames':fs}
 write(FX,name,im,anims)

def main():
 FX.mkdir(exist_ok=True);SPR.mkdir(exist_ok=True)
 char=[('idle','eudora-idle.png',1.2),('run','eudora-run.png',.6),('attack','eudora-basicattack.png',.4),('skill','eudora-1stskill.png',.4),('skill2','eudora-2ndskill.png',.4),('ult','eudora-ult.png',.5),('hit','eudora-hit.png',.22),('dead','eudora-death.png',.8)]
 sheet=Image.new('RGBA',(1656,576));anims={};counts={}
 for row,(tag,file,total) in enumerate(char):
  cells=read(file,72,72);assert len(cells)<=23
  counts[tag]=len(cells)
  for i,c in enumerate(cells):sheet.alpha_composite(c,(i*72,row*72))
  anims[tag]={'frames':[frame(i*72,row*72,72,72,total/len(cells)) for i in range(len(cells))]}
 write(SPR,'eudora',sheet,anims,.5,55/72)
 angles('eudora_bolt',read('Basic bolt.png',72,32),72)
 angles('eudora_fan',read('Forked Lightning fan.png',72,56,True),128,(12,28))
 links=read('Maintained lightning link.png',48,32)
 angles('eudora_link',links,48,individual=True)
 assert len(links)==9,'Update LINK_VISUAL_FRAMES in eudora.rs when the source count changes'
 marks=read('Superconductor mark.png',48,48);assert len(marks)==9
 ms=Image.new('RGBA',(72*len(marks),112));ma={}
 for i,c in enumerate(marks):
  # VFX-only size adaptation: the full-size orb must not cover a 45px hero.
  small=c.resize((18,18),Image.Resampling.NEAREST);ms.alpha_composite(small,(72*i+27,42))
  ma[f'mark_{i}']={'frames':[frame(i*72,0,72,112,.12)]}
 write(FX,'eudora_mark',ms,ma,.5,1.)
 effect('eudora_impact','Small impact or completed link.png',48,48,.3,True)
 effect('eudora_ball','Ball Lightning projectile.png',48,48,.3)
 effect('eudora_ball_burst','Enhanced Ball Lightning area burst.png',48,48,.34,True)
 effect('eudora_warning','Ground warning.png',72,48,.6)
 effect('eudora_strike','Descending strike.png',48,72,.4,True,ay=60/72)
 effect('eudora_discharge','Ground discharge or outer ring.png',72,48,.4,True)
 for a,b in [('Forked_Lightning.png','eudora_skill'),('Ball_Lightning.png','eudora_skill2'),("Thunder's_Wrath.png",'eudora_ult'),('Superconductor.png','eudora_passive')]:
  Image.open(ROOT/'source/eudora/icons'/a).convert('RGBA').save(ROOT/'icons'/(b+'.png'))
 (ROOT/'reports/EUDORA_ASSET_COUNTS.json').write_text(json.dumps({'character':counts,'link_frames':len(links),'mark_frames':len(marks)},indent=2)+'\n')
 print('Compiled character rows:',counts)
 print('All eight character motions preserved at native size; VFX direction atlases and marker adaptation compiled.')
if __name__=='__main__':main()
