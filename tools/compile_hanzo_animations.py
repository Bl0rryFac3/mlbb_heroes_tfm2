"""Compile owner Hanzo sheets at native pixels; skip oversized character frames, never resize."""
from pathlib import Path
from PIL import Image,ImageOps
import json,hashlib
ROOT=Path(__file__).resolve().parents[1];SRC=ROOT/'source/hanzo/animations'
def write(p,x):p.write_text(json.dumps(x,ensure_ascii=False,indent=1)+'\n')
body=['human_idle','human_run','human_attack','human_feast','human_dash','human_project','human_hit','human_death','parked_body','body_merge','spirit_idle','spirit_move','spirit_attack','spirit_snatch','spirit_dash','spirit_return','spirit_break','spirit_hit']
durations=dict(zip(body,[1.2,.6,.3,.34,.2,.3,.2,.9,1.2,.3,1.2,.6,.3,.25,.25,.3,.3,.2]))
report={};poses=[];effects=[];world={}
def compile(name,cw,ch,seconds,isbody=False,count=None):
 im=Image.open(SRC/(name+'.png')).convert('RGBA');cols=im.width//cw;rows=im.height//ch;assert cols*cw==im.width and rows*ch==im.height
 fs=[im.crop((i%cols*cw,i//cols*ch,i%cols*cw+cw,i//cols*ch+ch))for i in range(count or cols*rows)]
 selected=[];empty=[];oversized=[]
 for i,f in enumerate(fs):
  b=f.getchannel('A').getbbox()
  if not b:empty.append(i);continue
  if isbody and b[3]-b[1]>48:oversized.append(i);continue
  selected.append((i,f))
 assert selected,name
 if isbody:
  dy=58-selected[0][1].getchannel('A').getbbox()[3];dx=24 if name=='human_idle'else 0
  frames=[]
  for i,f in selected:
   cell=Image.new('RGBA',(96,72));cell.paste(f,(dx,dy+4))
   # Uniform clip translation only. No per-frame recentering or pixel resampling.
   assert sum(cell.getchannel('A').histogram()[1:])==sum(f.getchannel('A').histogram()[1:])
   frames.append(cell)
  cw,ch=96,72;anchor=(48 if name.startswith('spirit_')else 44,62)
 else:frames=[f for _,f in selected];anchor=(cw/2,ch/2)
 for left in ([False,True]if isbody else[False]):
  n=len(frames);sheet=Image.new('RGBA',(cw*n,ch))
  for i,f in enumerate(frames):sheet.paste(ImageOps.mirror(f)if left else f,(i*cw,0))
  suffix='_left'if left else'';stem=ROOT/'effects'/('hanzo_anim_'+name+suffix);sheet.save(str(stem)+'#sheet.png')
  write(Path(str(stem)+'#anim.fanim'),{'anchor_x':(cw-anchor[0]if left else anchor[0])/cw,'anchor_y':anchor[1]/ch,'anims':{'main':{'frames':[{'duration':seconds/n,'data':{'x':i*cw,'y':0,'w':cw,'h':ch}}for i in range(n)]}}})
  if isbody:
   poses.append((name,left))
   effects.append({'type':'Animation','name':'mlbb_hanzo_fx_'+name+suffix,'anim':'asset/mlbb_heroes/effects/hanzo_anim_'+name+suffix,'tag':'main','z':1,'is_follow':False})
  else:effects.append({'type':'Animation','name':'mlbb_hanzo_fx_'+name.removeprefix('hanzo_'),'anim':'asset/mlbb_heroes/effects/hanzo_anim_'+name,'tag':'main','z':0 if 'ground' in name else 2,'is_follow':False})
  if not left and name in ['parked_body','body_merge','human_project','hanzo_body_ward','hanzo_spirit_link','hanzo_dark_marks']:
   
   for fi,frame_image in enumerate(frames):frame_image.save(ROOT/'icons'/f'hanzo_world_{name}_{fi}.png')
   world[name]={'count':n,'cell':[cw,ch],'pivot':[anchor[0]/cw,anchor[1]/ch]}
 report[name]={'source':name+'.png','sha256':hashlib.sha256((SRC/(name+'.png')).read_bytes()).hexdigest(),'source_cell':[64,48]if name=='human_idle'else [im.width//cols,im.height//rows],'source_frames':len(fs),'selected_frames':[i for i,_ in selected],'skipped_empty':empty,'skipped_over48':oversized,'native_pixels_preserved':True,'output_cell':[cw,ch],'seconds':seconds}
 return frames
for name in body:compile(name,64 if name=='human_idle'else 96,48 if name=='human_idle'else 64,durations[name],True)
fx={'hanzo_dark_marks':(32,32,1.),'hanzo_basic_slash':(64,48,.18),'hanzo_feast_bite':(64,80,.4),'hanzo_mist_ground_human':(128,32,2.5),'hanzo_mist_ground_spirit':(128,32,2.),'hanzo_mist_pulse':(128,32,.15),'hanzo_dash_streak':(96,48,.2),'hanzo_projection_flare':(80,80,.3),'hanzo_spirit_maw':(80,96,.4),'hanzo_airborne_contact':(48,64,.4),'hanzo_body_ward':(64,64,1.2),'hanzo_spirit_link':(64,16,.8),'hanzo_return_merge':(64,64,.3),'hanzo_spirit_failure':(64,64,.3),'hanzo_heal_return':(48,64,.4)}
for n,(w,h,t)in fx.items():compile(n,w,h,t,count=5 if n=='hanzo_dark_marks'else None)
p=ROOT/'champion/mlbb_hanzo.data_champion';d=json.loads(p.read_text());d['view_buffs']=[{'type':'Animated','name':'mlbb_hanzo_pose_'+n+('_left'if left else''),'anim':'asset/mlbb_heroes/effects/hanzo_anim_'+n+('_left'if left else''),'tag':'main','z':1}for n,left in poses];d['view_effects']=effects
for p in [p,ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion']:write(p,d)
first=Image.open(ROOT/'effects/hanzo_anim_human_idle#sheet.png').convert('RGBA').crop((0,0,96,72));sheet=Image.new('RGBA',(192,72));sheet.paste(first,(0,0));sheet.save(ROOT/'sprite/hanzo_probe#sheet.png')
frame=lambda x,t:{'duration':t,'data':{'x':x,'y':0,'w':96,'h':72}}
anims={n:{'frames':[frame(96,.12)]}for n in ['run','attack','skill','skill2','ult','hit','dead','main']};anims['idle']={'frames':[frame(0,.000001),frame(96,86400.)]}
write(ROOT/'sprite/hanzo_probe#anim.fanim',{'anchor_x':44/96,'anchor_y':62/72,'anims':anims})
lines=['// Generated by tools/compile_hanzo_animations.py.','pub(super) fn pose(name:&str,left:bool)->&\'static str { match(name,left){']
for n,left in poses:lines.append(f'("{n}",{str(left).lower()})=>"mlbb_hanzo_pose_{n}{"_left"if left else""}",')
lines+=['_=>"mlbb_hanzo_pose_human_idle",','}}','pub(super) fn world(name:&str)->(usize,(f32,f32)){match name{']
for n,v in world.items():lines.append(f'"{n}"=>({v["count"]},({v["pivot"][0]}f32,{v["pivot"][1]}f32)),')
lines+=['_=>(1,(0.5,0.5)),','}}']
(ROOT/'sdk/candidate/hanzo_bindings.rs').write_text('\n'.join(lines)+'\n')
write(ROOT/'reports/HANZO_1150_ANIMATION_AUDIT.json',report)
print('PASS33 owner sheets; native pixels, 48px character ceiling, original source frames preserved; excluded oversized:',sum(len(v['skipped_over48'])for v in report.values()))
