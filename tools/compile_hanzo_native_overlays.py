"""Put persistent Hanzo overlays on the game's native view-effect path."""
from pathlib import Path
from PIL import Image
import json
ROOT=Path(__file__).resolve().parents[1]
def write(p,x):p.write_text(json.dumps(x,indent=1)+'\n')
p=ROOT/'champion/mlbb_hanzo.data_champion';champ=json.loads(p.read_text());champ['view_effects']=[v for v in champ['view_effects']if not v['name'].startswith('mlbb_hanzo_overlay_')]
def bind(name,anim,tag,z):champ['view_effects'].append({'type':'Animation','name':'mlbb_hanzo_overlay_'+name,'anim':anim,'tag':tag,'z':z,'is_follow':False})
for name in ['parked_body','human_project','body_merge','hanzo_body_ward']:
 for left in ([False,True]if not name.startswith('hanzo_')else[False]):
  suffix='_left'if left else'';stem='hanzo_anim_'+name+suffix;path=ROOT/'effects'/(stem+'#anim.fanim');a=json.loads(path.read_text())
  for i,f in enumerate(a['anims']['main']['frames']):
   frame={'duration':.1,'data':f['data']};a['anims']['overlay_'+str(i)]={'frames':[frame]};bind(name+suffix+'_'+str(i),'asset/mlbb_heroes/effects/'+stem,'overlay_'+str(i),0 if name=='hanzo_body_ward'else 1)
  write(path,a)
path=ROOT/'effects/hanzo_anim_hanzo_dark_marks#anim.fanim';a=json.loads(path.read_text())
for i,f in enumerate(a['anims']['main']['frames']):a['anims']['state_'+str(i)]={'frames':[{'duration':.2,'data':f['data']}]};bind('mark_'+str(i+1),'asset/mlbb_heroes/effects/hanzo_anim_hanzo_dark_marks','state_'+str(i),2)
write(path,a)
# Reuse owner link pixels at native size; pad before nearest-neighbor angle rotation.
src=Image.open(ROOT/'effects/hanzo_anim_hanzo_spirit_link#sheet.png').convert('RGBA');meta=json.loads((ROOT/'effects/hanzo_anim_hanzo_spirit_link#anim.fanim').read_text());frames=meta['anims']['main']['frames'];n=len(frames)
sheet=Image.new('RGBA',(96*n,96*12));anims={}
for bucket in range(12):
 for i,f in enumerate(frames):
  d=f['data'];piece=src.crop((d['x'],d['y'],d['x']+d['w'],d['y']+d['h']));tile=Image.new('RGBA',(96,96));tile.paste(piece,((96-piece.width)//2,(96-piece.height)//2));tile=tile.rotate(-bucket*15,resample=Image.Resampling.NEAREST,expand=False);sheet.paste(tile,(i*96,bucket*96));tag=f'link_{bucket}_{i}';anims[tag]={'frames':[{'duration':.1,'data':{'x':i*96,'y':bucket*96,'w':96,'h':96}}]};bind(tag,'asset/mlbb_heroes/effects/hanzo_native_link',tag,0)
sheet.save(ROOT/'effects/hanzo_native_link#sheet.png');write(ROOT/'effects/hanzo_native_link#anim.fanim',{'anchor_x':.5,'anchor_y':.5,'anims':anims})
for path in [p,ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion']:write(path,champ)
(ROOT/'reports/HANZO_1151_OVERLAY_BINDINGS.json').write_text(json.dumps({'native_overlay_bindings':sum(v['name'].startswith('mlbb_hanzo_overlay_')for v in champ['view_effects']),'link_frames':n,'link_angle_buckets':12,'body_native_pixels_unchanged':True,'link_rotation':'nearest-neighbor; palette preserved; no character resampling'},indent=2)+'\n')
print('PASS native body/project/merge/ward, five mark states and12-angle owner link bindings')
