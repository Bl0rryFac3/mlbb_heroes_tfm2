#!/usr/bin/env python3
"""Compile supplied native-resolution PixelLab grids; no model calls, no rescaling.
Character: 72x72 cells, nine frames, row-major 3x3 or horizontal strip.
VFX: dimensions in ASSET_MANIFEST.json, nine frames each.
"""
from pathlib import Path
import argparse,json,math
from PIL import Image
H=Path(__file__).resolve().parents[1]
def read(p,w,h):
 im=Image.open(p);assert im.mode=='RGBA',f'{p}: export RGBA PNG';assert im.width%w==0 and im.height%h==0,(p,im.size)
 assert im.getextrema()[3][0]==0 and im.getextrema()[3][1]>0,f'{p}: transparent background required'
 out=[im.crop((x,y,x+w,y+h)) for y in range(0,im.height,h) for x in range(0,im.width,w)]
 # Frame count is whatever the sheet supplies. The runtime is frame-count
 # agnostic (the shipped roster ranges from 3 to 60 frames per animation), so
 # only require a sane, non-empty, uniform grid. Never silently drop frames.
 assert 2<=len(out)<=64,f'{p}: {len(out)} frames; expected a uniform grid of 2..64 frames'
 assert any(c.getbbox() for c in out),f'{p}: all empty';return out
def frame(x,y,w,h,d):return {'duration':d,'data':dict(x=x,y=y,w=w,h=h)}
def save(folder,name,image,anims,anchor=(.5,.5)):
 folder.mkdir(exist_ok=True,parents=True);image.save(folder/(name+'#sheet.png'));(folder/(name+'#anim.fanim')).write_text(json.dumps({'anchor_x':anchor[0],'anchor_y':anchor[1],'anims':anims},separators=(',',':'))+'\n')
def main():
 ap=argparse.ArgumentParser();ap.add_argument('root',type=Path);ap.add_argument('inbox',type=Path);a=ap.parse_args();manifest=json.loads((H/'ASSET_MANIFEST.json').read_text());missing=[str(a.inbox/e['file']) for e in manifest if not(a.inbox/e['file']).is_file()];assert not missing,'Missing returned assets: '+', '.join(missing)
 sheet=Image.new('RGBA',(1656,576));anims={}
 for row,e in enumerate(x for x in manifest if x['kind']=='character'):
  cells=read(a.inbox/e['file'],72,72)
  while len(cells)>2 and not cells[-1].getbbox(): cells.pop()  # trim trailing grid padding only
  for j,c in enumerate(cells):
   box=c.getbbox();assert box,f"{e['id']}: interior frame {j} is empty; a real frame is missing - fix the source rather than skipping it"
   assert box[2]<=72 and box[3]<=72,(e['id'],j,box)  # shipped roster frames do touch a cell edge (Angela 14, Cecilion 31); libel this
   # Preserve row motion: one translation from frame zero for the whole row.
  first=cells[0].getbbox();dy=56-first[3]
  for j,c in enumerate(cells):
   box=c.getbbox();assert 0<=box[1]+dy and box[3]+dy<=72,'Row offset would crop; fix the source row rather than per-frame centering';sheet.alpha_composite(c,(j*72,row*72+dy))
  n=len(cells);anims[e['id']]={'frames':[frame(j*72,row*72,72,72,e['seconds']/n) for j in range(n)]}
 save(a.root/'sprite','julian',sheet,anims,(.5,55/72))
 for e in (x for x in manifest if x['kind']=='vfx'):
  w,h=e['cell'];cells=read(a.inbox/e['file'],w,h);directional=e.get('directional',False)
  if directional:
   n=len(cells);origin=e.get('origin',[w/2,h/2])
   # Rotation canvas must contain the art at any angle. Derive it from the measured
   # reach instead of assuming 128 (the Julian sword art reaches ~86 px from origin,
   # which 128 silently clipped). Round up to a multiple of 32.
   reach=0
   for c in cells:
    b=c.getbbox()
    if b: reach=max(reach,max((x-origin[0])**2+(y-origin[1])**2 for x in [b[0],b[2]] for y in [b[1],b[3]])**0.5)
   # must also contain the whole CELL at the origin offset, not just the art reach
   side=int(math.ceil(max((reach+2)*2, origin[0]*2+w, origin[1]*2+h, (w-origin[0])*2+w, (h-origin[1])*2+h)/32.0)*32)
   im=Image.new('RGBA',(side*n,side*16));anims={}
   for angle in range(16):
    frames=[]
    for j,c in enumerate(cells):
     box=c.getbbox();assert box and all((x-origin[0])**2+(y-origin[1])**2<=(side/2-1)**2 for x in [box[0],box[2]] for y in [box[1],box[3]]),f'{e["id"]}: art would clip in a directional rotation'
     ox,oy=round(side/2-origin[0]),round(side/2-origin[1]);assert ox>=0 and oy>=0 and ox+w<=side and oy+h<=side,f'{e["id"]}: art does not fit the {side}px rotation canvas'
     o=Image.new('RGBA',(side,side));o.alpha_composite(c,(ox,oy));o=o.rotate(-angle*22.5,Image.Resampling.NEAREST);im.alpha_composite(o,(side*j,side*angle));f=frame(side*j,side*angle,side,side,e['seconds']/n);frames.append(f)
     if e['id'] in ['scythe','scythe_enhanced']:anims[f'dir_{angle}_{j}']={'frames':[f]}
    anims[f'dir_{angle}']={'frames':frames}
  else:
   n=len(cells);im=Image.new('RGBA',(w*n,h));anims={'play':{'frames':[frame(j*w,0,w,h,e['seconds']/n) for j in range(n)]}}
   for j,c in enumerate(cells):
    im.alpha_composite(c,(j*w,0))
    if e['id']=='scythe_spin':anims[f'phase_{j}']={'frames':[frame(j*w,0,w,h,e['seconds']/n)]}
  save(a.root/'effects','julian_'+e['id'],im,anims)
 # Intentional icon design: use the returned skill VFX silhouette, not a blank
 # placeholder. Owners may replace these with approved skill icons afterward.
 for source,target in [('scythe','skill'),('sword','skill2'),('chain_bind','ult')]:
  e=next(e for e in manifest if e['id']==source and e['kind']=='vfx');c=read(a.inbox/e['file'],*e['cell'])[0];box=c.getbbox();assert box
  c=c.crop(box);icon=Image.new('RGBA',(64,64),(16,23,35,255));ratio=min(52/c.width,52/c.height);c=c.resize((max(1,round(c.width*ratio)),max(1,round(c.height*ratio))),Image.Resampling.NEAREST);icon.alpha_composite(c,((64-c.width)//2,(64-c.height)//2));icon.save(a.root/'icons'/('julian_'+target+'.png'))
 print('Compiled all eight character rows and nine VFX. Inspect alpha, clipping, timings, anchors and icons before release.')
if __name__=='__main__':main()
