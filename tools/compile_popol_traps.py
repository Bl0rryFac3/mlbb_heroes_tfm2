"""Compile received trap grids at native world scale. Originals remain untouched."""
from pathlib import Path
from PIL import Image
import json,hashlib
ROOT=Path(__file__).resolve().parents[1];SRC=ROOT/'source/popol_kupa/traps_received'
bindings=[];report={}
for name,cell,seconds in [('idle',32,1.2),('trigger',32,.6),('frost_loop',48,1.2)]:
    p=SRC/f'trap_{name}.png';im=Image.open(p).convert('RGBA');assert im.size==(288,288)
    sheet=Image.new('RGBA',(cell*3,cell*3));frames=[]
    for i in range(9):
        x,y=i%3*96,i//3*96
        frame=im.crop((x,y,x+96,y+96)).resize((cell,cell),Image.Resampling.NEAREST)
        dx,dy=i%3*cell,i//3*cell;sheet.paste(frame,(dx,dy))
        frames.append({'duration':seconds/9,'data':{'x':dx,'y':dy,'w':cell,'h':cell}})
    base=ROOT/'effects'/f'popol_trap_{name}';sheet.save(str(base)+'#sheet.png')
    anims={'main':{'frames':frames}}
    if name!='trigger':
        for i,f in enumerate(frames):
            anims[f'lease_{i}']={'frames':[{'duration':8/60,'data':f['data']}]}
            bindings.append({'type':'Animation','name':f'mlbb_pk_trap_{name}_{i}','anim':f'asset/mlbb_heroes/effects/popol_trap_{name}','tag':f'lease_{i}','z':0,'is_follow':False})
    else:bindings.append({'type':'Animation','name':'mlbb_pk_trap_trigger','anim':'asset/mlbb_heroes/effects/popol_trap_trigger','tag':'main','z':1,'is_follow':False})
    Path(str(base)+'#anim.fanim').write_text(json.dumps({'anchor_x':.5,'anchor_y':.5,'anims':anims},indent=1)+'\n')
    report[p.name]={'source_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'frames':9,'source_cell':[96,96],'runtime_cell':[cell,cell],'scale':'nearest-neighbor compiler derivative','center_anchor':[.5,.5]}
for folder in ['champion','sdk/champion','sdk/candidate/champion']:
    p=ROOT/folder/'mlbb_popol_kupa.data_champion';d=json.loads(p.read_text());d['view_effects']=[v for v in d['view_effects']if not v['name'].startswith('mlbb_pk_trap_')]+bindings;p.write_text(json.dumps(d,indent=1)+'\n')
(ROOT/'reports/POPOL_TRAP_ASSETS.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: 27 trap frames compiled; exact original grids retained; bounded eight-tick visual leases')
