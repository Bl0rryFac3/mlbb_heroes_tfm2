"""Compile a shared first-idle-frame UI preview without a runtime UI scan.

Static champion UI reads the first idle frame. Timed gameplay animation advances
past its microsecond presentation frame into a transparent carrier. Combat body
layers keep owning the match appearance. No invisibility/banish is involved.
"""
from pathlib import Path
from PIL import Image
import json
ROOT=Path(__file__).resolve().parents[1]
source=Image.open(ROOT/'source/popol_kupa/received/popol_idle.png').convert('RGBA').crop((0,0,72,72))
sheet=Image.new('RGBA',(144,72));sheet.paste(source,(0,0))
base=ROOT/'sprite/popol_probe_carrier'
sheet.save(str(base)+'#sheet.png')
def frame(x,duration):return {'duration':duration,'data':{'x':x,'y':0,'w':72,'h':72}}
anims={name:{'frames':[frame(72,.12)]}for name in ['run','attack','skill','skill2','ult','hit','dead','main']}
anims['idle']={'frames':[frame(0,.000001),frame(72,86400.)]}
Path(str(base)+'#anim.fanim').write_text(json.dumps({'anchor_x':.5,'anchor_y':55/72,'anims':anims},indent=1)+'\n')
assert sheet.crop((0,0,72,72)).tobytes()==source.tobytes()
assert sheet.crop((72,0,144,72)).getchannel('A').getextrema()==(0,0)
# Face center from champion_view.face is (36,55)+(0,-35).
assert source.crop((27,11,45,29)).getchannel('A').getbbox() is not None
print('PASS: first idle frame/face visible; timed carrier frames transparent; original pixels preserved')
