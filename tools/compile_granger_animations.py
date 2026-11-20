"""Compile owner sheets without resampling. Native UI carrier + bounded pose buffs."""
from pathlib import Path
from PIL import Image,ImageOps
import json,hashlib
ROOT=Path(__file__).resolve().parents[1]
SRC=ROOT/'source/granger/animations'; SRC.mkdir(parents=True,exist_ok=True)
REPORT={}
def compile(name,file,cell,count,seconds,body=False):
 im=Image.open(SRC/file).convert('RGBA');w,h=cell;cols=im.width//w
 frames=[im.crop((i%cols*w,i//cols*h,i%cols*w+w,i//cols*h+h)) for i in range(count)]
 if body: assert all(f.getbbox() and f.getbbox()[3]-f.getbbox()[1]<=48 for f in frames)
 for left in ([False,True] if body or 'muzzle' in name else [False]):
  # Owner's action sheets face right; mirror each cell for left-facing playback.
  mirror=body and left
  if 'muzzle' in name:mirror=left
  sheet=Image.new('RGBA',im.size)
  for i,f in enumerate(frames):sheet.paste(ImageOps.mirror(f) if mirror else f,(i%cols*w,i//cols*h))
  stem=ROOT/'effects'/('granger_anim_'+name+('_left' if left else ''))
  sheet.save(str(stem)+'#sheet.png')
  anchor=(42/72 if left else 30/72) if body else .5
  data={'anchor_x':anchor,'anchor_y':60/72 if body else ((cell[1]/2+24)/cell[1] if 'projectile' in name else .5),'anims':{'main':{'frames':[{'duration':seconds/count,'data':{'x':i%cols*w,'y':i//cols*h,'w':w,'h':h}} for i in range(count)]}}}
  Path(str(stem)+'#anim.fanim').write_text(json.dumps(data,indent=1)+'\n')
 REPORT[name]={'source':file,'sha256':hashlib.sha256((SRC/file).read_bytes()).hexdigest(),'cell':cell,'frames':count,'seconds':seconds,'max_visible_height':max((f.getbbox()[3]-f.getbbox()[1] if f.getbbox() else 0) for f in frames),'resampled':False}
body=[('idle','idle',9,1.2),('run','run',9,.6),('fire','handgun_fire_v1',9,.3),('rondo','rondo',9,.2),('cannon','cannon_fire',9,.2),('roll','roll',9,.2),('hit','hit',9,.2),('dead','dead',13,.9)]
for name,file,count,sec in body:compile(name,'granger_'+file+'.png',(72,72),count,sec,True)
for name,cell,sec in [('gun_projectile',(32,16),.3),('sixth_projectile',(48,24),.3),('cannon_projectile',(64,32),.3),('gun_muzzle',(32,32),.15),('gun_hit',(32,32),.18),('sixth_hit',(48,48),.24),('cannon_muzzle',(64,48),.2),('cannon_impact',(96,96),.4)]:compile(name,'granger_'+name+'.png',cell,9,sec)
# Preserve a visible first idle frame for native menus/ban icons, blank carrier in match.
first=Image.open(SRC/'granger_idle.png').convert('RGBA').crop((0,0,72,72))
sheet=Image.new('RGBA',(144,72));sheet.paste(first,(0,0));sheet.save(ROOT/'sprite/granger_probe#sheet.png')
f=lambda x,t:{'duration':t,'data':{'x':x,'y':0,'w':72,'h':72}}
a={n:{'frames':[f(72,.12)]}for n in ['run','attack','skill','skill2','ult','hit','dead','main']};a['idle']={'frames':[f(0,.000001),f(72,86400.)]}
(ROOT/'sprite/granger_probe#anim.fanim').write_text(json.dumps({'anchor_x':30/72,'anchor_y':60/72,'anims':a},indent=1)+'\n')
p=ROOT/'champion/mlbb_granger.data_champion';d=json.loads(p.read_text())
d['view_buffs']=[{'type':'Animated','name':'mlbb_granger_pose_'+n+('_left'if left else ''),'anim':'asset/mlbb_heroes/effects/granger_anim_'+n+('_left'if left else ''),'tag':'main','z':1} for n,_,_,_ in body if n!='dead' for left in [False,True]]
d['view_projectiles']=[{'type':'Animated','name':'mlbb_granger_'+n,'anim':'asset/mlbb_heroes/effects/granger_anim_'+fx+'_projectile','tag':'main','z':1,'repeat':True}for n,fx in [('bullet','gun'),('sixth','sixth'),('cannon','cannon')]]
d['view_effects']=[{'type':'Animation','name':'mlbb_granger_fx_'+n+('_left'if left else ''),'anim':'asset/mlbb_heroes/effects/granger_anim_'+n+('_left'if left else ''),'tag':'main','z':2 if n!='dead'else 1,'is_follow':False}for n in ['gun_muzzle','cannon_muzzle','gun_hit','sixth_hit','cannon_impact','dead']for left in ([False,True] if n in ['gun_muzzle','cannon_muzzle','dead']else [False])]
for base in [ROOT/'champion',ROOT/'sdk/candidate/champion']:(base/p.name).write_text(json.dumps(d,indent=1)+'\n')
p=ROOT/'style/champion_view.champion_view';d=json.loads(p.read_text());d['entries']['mlbb_granger']={'face':{'x':0,'y':-37},'center':{'x':0,'y':-20},'banpick_center':{'x':0,'y':-18}};p.write_text(json.dumps(d,indent=1)+'\n')
(ROOT/'reports/GRANGER_ANIMATION_INTAKE_1146.json').write_text(json.dumps(REPORT,indent=2)+'\n')
print('PASS: 8 body motions, 13 death frames, 8 FX sheets, native UI carrier; no resampling; all body frames <=48px')
