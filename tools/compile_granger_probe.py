#!/usr/bin/env python3
"""Compile the owner's exact PNG as a static mechanics rig; no generated art."""
from pathlib import Path
from PIL import Image, ImageDraw
import json, hashlib, shutil
ROOT=Path(__file__).resolve().parents[1]
ART=ROOT/'source/granger/received'
ART.mkdir(parents=True,exist_ok=True)
master=ART/'granger_master.png'
image=Image.open(master).convert('RGBA')
assert image.size==(72,48)
cell=Image.new('RGBA',(72,72));cell.paste(image,(0,7))
def atlas(name,im,anchor=.5):
    base=ROOT/name;base.parent.mkdir(parents=True,exist_ok=True)
    im.save(str(base)+'#sheet.png')
    d={'anchor_x':.5,'anchor_y':anchor,'anims':{tag:{'frames':[{'duration':.12,'data':{'x':0,'y':0,'w':im.width,'h':im.height}}]}for tag in ['idle','run','attack','skill','skill2','ult','hit','dead','main']}}
    Path(str(base)+'#anim.fanim').write_text(json.dumps(d,indent=1)+'\n')
atlas('sprite/granger_probe',cell,55/72)
assert cell.crop((0,7,72,55)).tobytes()==image.tobytes()
for name,colour,length in [('bullet',(255,218,138,255),10),('sixth',(255,80,101,255),14),('cannon',(237,113,150,255),20)]:
    im=Image.new('RGBA',(24,16));d=ImageDraw.Draw(im);d.rectangle((2,6,length,9),fill=colour);d.rectangle((2,7,length-2,8),fill=(255,245,218,255));atlas('effects/granger_probe_'+name,im)
# Temporary, explicit slot glyphs; these are not claimed as official MLBB icons.
for slot,label,colour in [('skill','S1',(210,164,61,255)),('skill2','S2',(62,146,178,255)),('ult','ULT',(188,65,94,255))]:
    im=Image.new('RGBA',(64,64),(18,22,30,255));d=ImageDraw.Draw(im);d.rectangle((1,1,62,62),outline=colour,width=2)
    crop=image.crop((17,0,59,48));im.alpha_composite(crop,(11,2));d.text((20,50),label,fill=colour);im.save(ROOT/f'icons/granger_probe_{slot}.png')
champ=json.loads((ROOT/'champion/mlbb_karrie.data_champion').read_text())
champ.update(id='mlbb_granger',sprite='asset/mlbb_heroes/sprite/granger_probe',tags=['AD','Range'])
champ['skill_icons']=[f'asset/mlbb_heroes/icons/granger_probe_{s}'for s in ['skill','skill2','ult']]
champ['stat'].update(attack=84,hp=660,defence=18,magic_resistance=18,move_speed=1080,hp_regen=6)
champ['growth'].update(attack=5,hp=90,defence=3,magic_resistance=2,move_speed=4,hp_regen=0)
for slot,op,duration,cd,range_,cast,target in [
 ('attack','basic',12,66,72000,'Targeting','Enemy'),
 ('skill','burst',1,18,90000,'Targeting','EnemyWithoutTower'),
 ('skill2','rondo',1,720,35000,'Direction','EnemyWithoutTower'),
 ('ult','cannon',1,36,140000,'Targeting','EnemyChampion')]:
    champ[slot]={'action_name':slot,'description':f'#asset/base/text/champion?description.mlbb_granger.{slot}','duration':duration,'cooltime':cd,'start_timing':0,'cancelable':True,'range':range_,'casting_type':cast,'casting_target':target,'attack_type':'BaseAttack'if slot=='attack'else'Skill','can_use_with_move':True,'effect':{'type':'Native','effect_ref':f'mlbb_heroes:granger_{op}'}}
champ['passive']={'passive_ref':'mlbb_heroes:granger'}
for k in list(champ):
    if k.startswith('view_'):del champ[k]
champ['view_projectiles']=[{'type':'Animated','name':'mlbb_granger_'+name,'anim':f'asset/mlbb_heroes/effects/granger_probe_{name}','tag':'main','z':1,'repeat':True}for name in ['bullet','sixth','cannon']]
for base in [ROOT/'champion',ROOT/'sdk/candidate/champion']:
    (base/'mlbb_granger.data_champion').write_text(json.dumps(champ,indent=1)+'\n')
p=ROOT/'style/champion_view.champion_view';d=json.loads(p.read_text());d['entries']['mlbb_granger']={'face':{'x':-2,'y':-39},'center':{'x':0,'y':-15},'banpick_center':{'x':0,'y':-12}};p.write_text(json.dumps(d,indent=1)+'\n')
Y=lambda s:f'<#ffb900ff>{s}<>'
C=lambda s:f'<#5ad0e6ff>{s}<>'
AD=lambda s:f'<i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>{s}<>'
D={'name':'Granger',
'attack':f'Fires a handgun round that deals {AD("100% AD")} physical damage on impact.',
'skill':f'{C("Rhapsody")}: Locks onto an enemy and fires {Y("6")} homing rounds over {Y("1.5s")}, each dealing {AD("28 + 40% AD")} physical damage and consuming {Y("70")} Gun Energy. Requires more than {Y("420")} Gun Energy. Movement Speed increases by {Y("10%")} while firing. Each hit reduces Rondo’s cooldown by {Y("0.5s")}. The final round gains {Y("100%")} Critical Chance for its impact. Rondo and Death Sonata can be used during the barrage.',
'skill2':f'{C("Rondo")}: Dashes a short distance and restores {Y("15%")} of maximum Gun Energy. Walls and movement-disabling effects stop the dash. Equipment cooldown reduction increases Attack instead of shortening Rondo’s effective cooldown.',
'ult':f'{C("Death Sonata")}: Consumes {Y("250")} Cannon Energy to fire a homing cannon round at an enemy champion. Impact triggers an explosion, dealing {AD("130 + 105% AD")} physical damage to nearby enemies and <#ef5350ff>slowing<> them by {Y("30%")} for {Y("0.6s")}. Granger then rolls a short distance away from the firing direction.',
'passive':f'{C("Capriccio")}: Uses separate Gun and Cannon Energy pools, each starting at {Y("800")}. Gun Energy regenerates at {Y("8%")} of its maximum per second; Cannon Energy regenerates at {Y("2%")}. Credited lane-minion kills increase maximum Gun Energy by {Y("6")} and restore {Y("5%")}; jungle kills grant the same benefit to Cannon Energy. Champion kills and assists increase both maxima by {Y("12")} and restore {Y("10%")}. Capacity growth persists through death. Equipment cooldown reduction grants {Y("0.5")} Attack per percentage point, up to {Y("20")} Attack.'}
N={'passive':'Capriccio','skill1':'Rhapsody','skill2':'Rondo','ult':'Death Sonata'}
p=ROOT/'text/champion.i18n';d=json.loads(p.read_text())
for locale in d.values():locale['description']['mlbb_granger']=D.copy();locale['skill_name']['mlbb_granger']=N.copy()
p.write_text(json.dumps(d,ensure_ascii=False,indent=1)+'\n')
report={'input_sha256':hashlib.sha256(master.read_bytes()).hexdigest(),'input_dimensions':list(image.size),'input_mode':image.mode,'alpha_bbox':image.getbbox(),'cell_dimensions':[72,72],'native_pixels_preserved':True,'padding_offset':[0,7],'generated_animation_frames':0,'skill_icons':'temporary slot glyphs over supplied sprite, not official icons','projectiles':'temporary technical markers','portrait_face_center':[34,16],'locales':'English fallback in all eight entries'}
(ROOT/'reports/GRANGER_ASSET_INTAKE.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS original 72x48 pixels preserved in padded cell; selection/face/ban entries and three slot icons compiled')
