#!/usr/bin/env python3
"""Staged showcase using unchanged release PNG/FANIM assets; not a game capture."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import argparse, math, json, hashlib, subprocess
OUT=Path(__file__).resolve().parent
ROOT=OUT/'assets' if (OUT/'assets').exists() else Path('/workspace/work/granger_release_v250')
W,H,FPS=800,450,20;GROUND=318;SCALE=2.5
WHITE='#eeeaf0';MUTED='#b2a6b6';GOLD='#ffcb79';RED='#fa7188';BLUE='#8abed3';INK='#11121e'
FONTDIR=OUT/'fonts' if (OUT/'fonts').exists() else Path('/usr/share/fonts/truetype/dejavu')
FONTS={}
def font(size,bold=False):
 k=size,bold
 if k not in FONTS:FONTS[k]=ImageFont.truetype(str(FONTDIR/('DejaVuSans-Bold.ttf'if bold else 'DejaVuSans.ttf')),size)
 return FONTS[k]
def text(im,xy,s,size=12,c=WHITE,bold=False,anchor=None):ImageDraw.Draw(im).text(xy,str(s),font=font(size,bold),fill=c,anchor=anchor)
def smooth(v):v=max(0,min(1,v));return v*v*(3-2*v)
def panel(im,box,color='#503a4e',fill='#171622'):ImageDraw.Draw(im).rounded_rectangle(box,6,fill=fill,outline=color)
def bar(im,x,y,w,value,maximum,color,h=7):
 d=ImageDraw.Draw(im);d.rounded_rectangle((x,y,x+w,y+h),3,fill='#302737')
 if value>0:d.rounded_rectangle((x,y,x+max(1,w*min(1,value/max(1,maximum))),y+h),3,fill=color)
ASSETS={};FACTS={};USED={}
class Clip:
 def __init__(self,name):
  self.name=name;p=ROOT/'effects'/('granger_anim_'+name)
  a=json.loads(Path(str(p)+'#anim.fanim').read_text());s=Image.open(str(p)+'#sheet.png').convert('RGBA')
  self.anchor=(a['anchor_x'],a['anchor_y']);self.frames=[];self.times=[];self.cache={}
  for row in a['anims']['main']['frames']:
   r=row['data'];self.frames.append(s.crop((r['x'],r['y'],r['x']+r['w'],r['y']+r['h'])));self.times.append(row['duration'])
  self.duration=sum(self.times);USED[name]=set();FACTS[name]={'frames':len(self.frames),'anchor':self.anchor,'duration':self.duration,'sheet_sha256':hashlib.sha256(Path(str(p)+'#sheet.png').read_bytes()).hexdigest(),'fanim_sha256':hashlib.sha256(Path(str(p)+'#anim.fanim').read_bytes()).hexdigest()}
 def frame(self,t,seconds=None,loop=True,scale=SCALE):
  duration=seconds or self.duration;p=(max(0,t)%duration)/duration if loop else min(.999999,max(0,t)/duration)
  v=p*self.duration+1e-9;total=0;i=len(self.frames)-1
  for j,d in enumerate(self.times):
   total+=d
   if v<total:i=j;break
  USED[self.name].add(i);key=i,scale
  if key not in self.cache:
   f=self.frames[i];self.cache[key]=f.resize((round(f.width*scale),round(f.height*scale)),Image.Resampling.NEAREST)
  return self.cache[key]
 def draw(self,im,x,y,t=0,seconds=None,loop=True,scale=SCALE,center=False,angle=0):
  f=self.frame(t,seconds,loop,scale)
  if angle:f=f.rotate(angle,resample=Image.Resampling.NEAREST,expand=True)
  ax,ay=(.5,.5)if center else self.anchor
  im.alpha_composite(f,(round(x-f.width*ax),round(y-f.height*ay)))
def clip(n):
 if n not in ASSETS:ASSETS[n]=Clip(n)
 return ASSETS[n]
def body(im,n,x,t=0,seconds=None,loop=True):clip(n).draw(im,x,GROUND,t,seconds,loop)
def shadow(im,x):ImageDraw.Draw(im,'RGBA').ellipse((x-37,GROUND-5,x+37,GROUND+7),fill=(0,0,0,110))
def dummy(im,x,y=GROUND,hp=1,label='TRAINING DUMMY',flash=False,tint='#cb9370',alpha=255):
 layer=Image.new('RGBA',(W,H));d=ImageDraw.Draw(layer)
 d.ellipse((x-23,y-7,x+23,y+5),fill='#2b2632',outline='#655266')
 d.rectangle((x-5,y-70,x+5,y-5),fill='#a07954');d.line((x-30,y-48,x+30,y-48),fill='#c49c6a',width=6)
 d.ellipse((x-11,y-89,x+11,y-66),fill='#d8b382',outline='#815e50',width=2)
 d.rounded_rectangle((x-22,y-66,x+22,y-22),5,fill='#f6e3cd'if flash else tint,outline='#ead7b2',width=2)
 for rad,col in [(15,'#943e59'),(10,'#d9b992'),(6,'#90384e'),(2,'#f8dab1')]:d.ellipse((x-rad,y-44-rad,x+rad,y-44+rad),fill=col)
 text(layer,(x,y-121),label,9,MUTED,True,'mm');bar(layer,x-38,y-109,76,hp,1,RED,6)
 if alpha!=255:layer.putalpha(layer.getchannel('A').point(lambda a:a*alpha//255))
 im.alpha_composite(layer)
def popup(im,x,y,t,label,col=GOLD):
 if 0<=t<.8:text(im,(x,y-23*smooth(t/.8)),label,17,col,True,'mm')
def background():
 im=Image.new('RGBA',(W,H));d=ImageDraw.Draw(im)
 for y in range(H):d.line((0,y,W,y),fill=(15+round(y/90),15+round(y/160),26+round(y/50)))
 # Organ-pipe silhouettes and concentric floor rings suggest a concert hall.
 for x in range(60,780,55):
  height=70+int(24*math.sin(x/80));d.rounded_rectangle((x,180-height,x+11,291),5,fill='#211c2d',outline='#35273c')
 for radius in [85,125,164]:d.arc((400-radius,222-radius*.6,400+radius,222+radius*.6),180,360,fill='#3e293d',width=2)
 d.ellipse((29,285,771,351),fill='#24202e',outline='#624152',width=2)
 for x in range(90,760,90):d.line((400+(x-400)*.6,288,x,350),fill='#41303f')
 d.ellipse((55,294,745,341),outline='#49313f');d.line((38,GROUND+4,762,GROUND+4),fill='#806052')
 return im
BG=background()
SCENES=[('THE DEATH CHANTER',3.0,0),('RHAPSODY',4.8,1),('RONDO',3.5,2),('DEATH SONATA',4.8,3),('CAST THROUGH THE BARRAGE',5.3,1),('CAPRICCIO',5.0,0),('DEFEAT & RETURN',3.4,0),('THE FINAL NOTE',2.4,3)]
TOTAL=sum(d for _,d,_ in SCENES)
PASSIVE_ICON=Image.open(OUT/'passive_icon.png').convert('RGBA').resize((34,34),Image.Resampling.LANCZOS)
ICONS=[]
for slot in ['skill','skill2','ult']:ICONS.append(Image.open(ROOT/f'icons/granger_probe_{slot}.png').convert('RGBA').resize((34,34),Image.Resampling.LANCZOS))

def energy(t,events,g=800,c=800,gm=800,cm=800,restore=None,growth=None):
 gc=cc=0;at={}
 for start,kind,*_ in events:at.setdefault(round(start*60),[]).append(kind)
 growth=growth or {};restore=round(restore*60)if restore is not None else -1
 for tick in range(round(t*60)+1):
  if g<gm:gc+=gm*8;g=min(gm,g+gc//6000);gc%=6000
  else:gc=0
  if c<cm:cc+=cm*2;c=min(cm,c+cc//6000);cc%=6000
  else:cc=0
  if tick==restore:g=min(gm,g+gm*15//100)
  for kind in at.get(tick,[]):
   if kind=='cannon':c-=250
   elif kind!='basic':g-=70
  if tick in growth:
   what=growth[tick]
   if what in ['gun','both']:gm+=12 if what=='both'else 6;g=min(gm,g+gm*(10 if what=='both'else 5)//100)
   if what in ['cannon','both']:cm+=12 if what=='both'else 6;c=min(cm,c+cm*(10 if what=='both'else 5)//100)
 return max(0,g),max(0,c),gm,cm

def target_at(t):return (628+18*math.sin(t*1.4),GROUND)
def projectile(im,t,start,kind,origin,target):
 travel=.48 if kind!='cannon'else .55;age=t-start
 if 0<=age<travel:
  p=age/travel;sx,sy=origin;ex,ey=target
  x=sx+(ex-sx)*p;y=sy+(ey-sy)*p-13*math.sin(math.pi*p)
  angle=-math.degrees(math.atan2(ey-sy-13*math.pi*math.cos(math.pi*p),ex-sx))
  clip({'basic':'gun_projectile','round':'gun_projectile','sixth':'sixth_projectile','cannon':'cannon_projectile'}[kind]).draw(im,x,y,age,seconds=.45,scale=1.7,center=True,angle=angle)
 impact=age-travel
 if 0<=impact<.45:
  clip('cannon_impact'if kind=='cannon'else 'sixth_hit'if kind=='sixth'else 'gun_hit').draw(im,*target,impact,seconds=.45,loop=False,scale=1.5 if kind=='cannon'else 2.,center=True)
 if kind!='cannon'and 0<=age<.45:clip('gun_muzzle').draw(im,*origin,age,seconds=.45,loop=False,scale=1.8,center=True)
 if kind=='sixth':popup(im,target[0],185,impact,'FINAL ROUND',RED)

def shoot_events(im,t,events,position,targetfn=target_at):
 for start,kind in events:
  tx,ty=targetfn(t);projectile(im,t,start,kind,(position(start)+65,GROUND-64),(tx,ty-45))
def fire_pose(im,t,events,x):
 active=[(s,k)for s,k in events if s<=t<s+.4 and k!='cannon']
 if active and active[-1][1]!='basic':
  start,kind=active[-1];a=clip('fire');i=min(8,3+int((t-start)/.06));USED['fire'].add(i)
  f=a.frames[i].resize((180,180),Image.Resampling.NEAREST);im.alpha_composite(f,(round(x-180*a.anchor[0]),round(GROUND-180*a.anchor[1])))
 else:
  past=active[-1][0]if active else None
  upcoming=next((s for s,k in events if k!='cannon' and s-.2<=t<s),None)
  start=past if past is not None else upcoming
  age=t-(start-.2)if start is not None else None
  body(im,'fire'if age is not None else 'idle',x,age if age is not None else t,seconds=.6 if age is not None else None,loop=age is None)
def chrome(im,idx,t,elapsed,pools,caption,detail):
 name,dur,active=SCENES[idx];panel(im,(10,10,790,65),'#74505c');text(im,(24,15),'GRANGER',26,GOLD,True);text(im,(26,48),'DEATH CHANTER  /  MOBILE LEGENDS x TEAMFIGHT MANAGER 2',9,MUTED)
 text(im,(775,29),'MLBB HEROES  v2.5.0',14,WHITE,True,'rm');text(im,(775,49),'24 HEROES',10,RED,True,'rm')
 text(im,(24,85),name,19,WHITE,True);text(im,(25,114),detail,11,MUTED)
 g,c,gm,cm=pools
 for j,(n,v,m,col)in enumerate([('GUN ENERGY',g,gm,GOLD),('CANNON ENERGY',c,cm,RED)]):
  y=87+j*31;text(im,(574,y),n,9,col,True);text(im,(775,y),f'{int(v)} / {m}',9,WHITE,False,'ra');bar(im,574,y+15,200,v,m,col)
 panel(im,(10,345,790,369),'#493441',fill='#181520');text(im,(400,357),caption,11,WHITE,False,'mm')
 labels=['CAPRICCIO','RHAPSODY','RONDO','DEATH SONATA'];subs=['PASSIVE','SKILL 1','SKILL 2','ULTIMATE']
 for j,label in enumerate(labels):
  x=12+j*197;hot=j==active or idx==4 and j>0;panel(im,(x,377,x+185,422),GOLD if hot else '#453142',fill='#31232e'if hot else '#191622')
  if j>0:im.alpha_composite(ICONS[j-1],(x+6,383))
  else:
   im.alpha_composite(PASSIVE_ICON,(x+6,383))
  text(im,(x+47,383),label,11,WHITE,True);text(im,(x+47,401),subs[j],9,GOLD if hot else MUTED)
 text(im,(13,435),'STAGED ASSET PREVIEW  /  TRAINING TARGETS',9,MUTED)
 text(im,(787,435),'MLBB HEROES  •  LOOPING SHOWCASE',9,MUTED,False,'ra')
 # A narrow playhead connects chapters without changing camera or scale.
 ImageDraw.Draw(im).rectangle((12,444,12+776*min(1,elapsed/TOTAL),446),fill=GOLD)

def render(idx,t,elapsed):
 im=BG.copy();pools=(800,800,800,800);events=[];caption='';detail='';px=230
 if idx==0:
  px=130+100*smooth(t/1.2);shadow(im,px);body(im,'run'if t<1.2 else 'idle',px,t);dummy(im,635)
  text(im,(px,180),'GRANGER',13,GOLD,True,'mm');caption='A marksman with two weapons, two energy pools and a relentless barrage.';detail='RANGED  •  PHYSICAL DAMAGE'
 elif idx==1:
  events=[(.45,'basic')]+[(1.6+.3*i,'sixth'if i==5 else 'round')for i in range(6)]
  tx,ty=target_at(t);hits=sum(t>=s+.48 for s,k in events);dummy(im,tx,ty,1-hits*.095,flash=any(0<=t-s-.48<.1 for s,k in events));shadow(im,px);fire_pose(im,t,events,px);shoot_events(im,t,events,lambda _:px)
  n=sum(t>=s for s,k in events if k!='basic')
  for i in range(6):ImageDraw.Draw(im).rounded_rectangle((26+i*25,145,42+i*25,154),2,fill=(RED if i==5 else GOLD)if i<n else '#493442')
  text(im,(192,143),f'{n} / 6 ROUNDS',11,RED if n==6 else GOLD,True);pools=energy(t,events)
  caption='Six homing rounds. A heavier final shot closes the phrase.';detail='HANDGUN FIRE  →  RHAPSODY  •  70 GUN ENERGY / ROUND'
 elif idx==2:
  px=210+125*smooth((t-.8)/.6);shadow(im,px);dummy(im,655);age=t-.8
  body(im,'rondo'if 0<=age<.6 else 'idle',px,age if age>=0 else t,seconds=.6 if 0<=age<.6 else None,loop=not(0<=age<.6));pools=energy(t,[],g=300,restore=.8)
  popup(im,px,180,t-.8,'+15% GUN ENERGY',GOLD);caption='Rondo repositions Granger and restores 15% of maximum Gun Energy.';detail='SHORT DASH  •  ENERGY RECOVERY'
 elif idx==3:
  events=[(1.,'cannon'),(2.7,'cannon')]
  position=lambda at:265-40*smooth((at-1.3)/.5)-40*smooth((at-3.)/.5)
  px=position(t);hits=sum(t>=s+.55 for s,k in events)
  for x in [620,685]:dummy(im,x,hp=1-hits*.24,label='PRIMARY'if x==620 else 'NEARBY',flash=any(0<=t-s-.55<.1 for s,k in events))
  shadow(im,px);motion='idle';age=t;duration=None
  for start,_ in events:
   if start-.2<=t<start+.3:motion='cannon';age=t-(start-.2);duration=.5
   elif start+.3<=t<start+.8:motion='roll';age=t-(start+.3);duration=.5
  body(im,motion,px,age,seconds=duration,loop=motion=='idle');shoot_events(im,t,events,position,lambda _: (620,GROUND));pools=energy(t,events)
  if hits:text(im,(654,173),'AREA IMPACT  +  SLOW',11,RED,True,'mm')
  caption='A homing cannon round explodes on impact. Granger rolls away after firing.';detail='250 CANNON ENERGY  •  EXPLOSIVE IMPACT  •  RETREAT ROLL'
 elif idx==4:
  events=[(.65+.3*i,'sixth'if i==5 else 'round')for i in range(6)]+[(1.55,'cannon')]
  position=lambda at:195+100*smooth((at-1.)/.5)-45*smooth((at-1.85)/.5)
  px=position(t);tx,ty=target_at(t);hits=sum(t>=s+(.55 if k=='cannon'else .48)for s,k in events)
  dummy(im,tx,ty,1-hits*.095,flash=any(0<=t-s-(.55 if k=='cannon'else .48)<.1 for s,k in events));shadow(im,px)
  if 1.85<=t<2.35:body(im,'roll',px,t-1.85,seconds=.5,loop=False)
  elif 1.35<=t<1.85:body(im,'cannon',px,t-1.35,seconds=.5,loop=False)
  elif 1.<=t<1.5:body(im,'rondo',px,t-1.,seconds=.5,loop=False)
  else:fire_pose(im,t,events,px)
  shoot_events(im,t,events,position);pools=energy(t,events,g=700,c=500,restore=1.)
  n=sum(t>=s for s,k in events if k!='cannon');text(im,(27,146),f'RHAPSODY  {n} / 6',11,GOLD,True)
  caption='Rondo and Death Sonata can be used while Rhapsody continues firing.';detail='REPOSITION  +  CANNON  +  THE REMAINING GUN ROUNDS'
 elif idx==5:
  px=170;events=[(.5,'basic'),(1.9,'basic'),(3.3,'basic')];kills=[1.,2.4,3.8]
  for x,when,label,tint in [(500,1.,'MINION','#92abc2'),(615,2.4,'JUNGLE','#8cba9c'),(715,3.8,'CHAMPION','#ce929a')]:
   age=t-when;alpha=255 if age<0 else max(0,round(255*(1-age/.4)))
   if alpha:dummy(im,x,hp=1 if age<0 else 0,label=label,tint=tint,alpha=alpha)
  shadow(im,px);fire_pose(im,t,events,px)
  for (start,kind),x in zip(events,[500,615,715]):projectile(im,t,start,kind,(px+65,GROUND-64),(x,GROUND-45))
  pools=energy(t,[],g=500,c=500,growth={60:'gun',144:'cannon',228:'both'})
  for at,label,col in [(1.,'GUN CAPACITY +6',GOLD),(2.4,'CANNON CAPACITY +6',RED),(3.8,'BOTH CAPACITIES +12',GOLD)]:popup(im,400,184,t-at,label,col)
  caption='Minions grow Gun capacity. Jungle kills grow Cannon. Takedowns grow both.';detail='CAPACITY GROWTH  +  ENERGY RESTORATION  •  CONTROLLED TRAINING TARGETS'
 elif idx==6:
  px=240;dummy(im,640,label='SPARRING TARGET');shadow(im,px)
  if .35<=t<.7:
   q=(t-.35)/.35;x=620+(px-620)*q;y=GROUND-44-20*math.sin(math.pi*q);ImageDraw.Draw(im).ellipse((x-6,y-3,x+6,y+3),fill=BLUE)
  if .7<=t<1.2:body(im,'hit',px,t-.7,seconds=.5,loop=False)
  elif 1.25<=t<2.65:body(im,'dead',px,t-1.25,seconds=.9,loop=False)
  else:body(im,'idle',px,t)
  if t>=2.65:popup(im,px,178,t-2.65,'BACK FOR THE ENCORE',GOLD)
  caption='Hit reaction, defeat and return. Respawn timing is condensed for this preview.';detail='CHARACTER ANIMATIONS  •  TIMING CONDENSED'
 else:
  shadow(im,260);body(im,'idle',260,t);dummy(im,650)
  text(im,(470,202),'GRANGER',28,GOLD,True,'mm');text(im,(470,235),'THE DEATH CHANTER',14,WHITE,True,'mm')
  caption='MLBB Heroes v2.5.0  •  Granger joins the 24-hero roster.';detail='RHAPSODY  /  RONDO  /  DEATH SONATA'
 chrome(im,idx,t,elapsed,pools,caption,detail)
 # Subtle cuts: leave exact source sprite pixels untouched in the compositing layer.
 return im.convert('RGB')

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--preview',action='store_true');ap.add_argument('--encode',action='store_true');args=ap.parse_args()
 if args.preview:
  o=Image.new('RGB',(1600,900));elapsed=0
  for i,(_,dur,_)in enumerate(SCENES):
   t=[2.,2.9,1.1,1.6,2.,3.9,1.8,1.][i];f=render(i,t,elapsed+t);f.save(OUT/f'preview_{i}.png');o.paste(f.resize((400,225)),(i%4*400,i//4*225));elapsed+=dur
  o.crop((0,0,1600,450)).save(OUT/'Granger_Storyboard.png');return
 frames=OUT/'frames';frames.mkdir(exist_ok=True);total=0;elapsed=0
 for i,(title,dur,_)in enumerate(SCENES):
  for j in range(round(dur*FPS)):render(i,j/FPS,elapsed+j/FPS).save(frames/f'{total+j:05d}.png',compress_level=2)
  total+=round(dur*FPS);elapsed+=dur;print(title,total,flush=True)
 (OUT/'showcase_manifest.json').write_text(json.dumps({'size':[W,H],'fps':FPS,'frames':total,'duration':TOTAL,'scenes':[{'title':n,'seconds':d}for n,d,_ in SCENES],'staged_not_capture':True,'asset_facts':FACTS,'frames_used':{n:sorted(v)for n,v in USED.items()},'dummy_hp':'illustrative contact feedback, not a native damage or crit measurement','energy':'60-tick resource clocks; chapters reset presets; motion/flight clips retimed for readability'},indent=2)+'\n')
 if args.encode:
  src=str(frames/'%05d.png');palette=str(OUT/'palette.png')
  subprocess.run(['ffmpeg','-v','error','-y','-framerate',str(FPS),'-i',src,'-vf','palettegen=max_colors=256:stats_mode=diff','-frames:v','1',palette],check=True)
  subprocess.run(['ffmpeg','-v','error','-y','-framerate',str(FPS),'-i',src,'-i',palette,'-lavfi','paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle','-loop','0',str(OUT/'Granger_Showcase.gif')],check=True)
  subprocess.run(['ffmpeg','-v','error','-y','-framerate',str(FPS),'-i',src,'-c:v','libx264','-crf','19','-pix_fmt','yuv420p','-movflags','+faststart',str(OUT/'Granger_Showcase.mp4')],check=True)
if __name__=='__main__':main()
