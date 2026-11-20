"""Compile two SpriteCook masters as a static projection rig; preserve native pixels."""
from pathlib import Path
from PIL import Image,ImageOps
import json,shutil,hashlib
ROOT=Path(__file__).resolve().parents[1]
if tuple(map(int,json.loads((ROOT/'mod.mod_info').read_text())['version'].split('.'))) >= (1,1,150):raise SystemExit('Legacy1.1.149 rig compiler; use compile_hanzo_animations.py and update_hanzo_full_text.py for1.1.150+')
SRC=ROOT/'source/hanzo/masters';SRC.mkdir(parents=True,exist_ok=True)
MASTER=ROOT.parent/'hanzo_master'
def write(p,d):p.write_text(json.dumps(d,ensure_ascii=False,indent=1)+'\n')
frames={};audit={}
for name,file,offset in [('human','Hanzo_Human_Master_v2.png',12),('spirit','Hanzo_Hanekage_Master_v1.png',8)]:
 source=MASTER/file
 if source.exists():shutil.copyfile(source,SRC/file)
 im=Image.open(SRC/file).convert('RGBA');b=im.getchannel('A').getbbox();assert b[3]-b[1]<=48
 cell=Image.new('RGBA',(96,64));cell.paste(im,(offset,8));assert cell.crop((offset,8,offset+im.width,8+im.height)).tobytes()==im.tobytes()
 frames[name]=cell
 for left in [False,True]:
  art=ImageOps.mirror(cell)if left else cell
  stem=ROOT/'effects'/('hanzo_probe_'+name+('_left'if left else ''));art.save(str(stem)+'#sheet.png')
  write(Path(str(stem)+'#anim.fanim'),{'anchor_x':64/96 if left else 32/96,'anchor_y':56/64,'anims':{'main':{'frames':[{'duration':.12,'data':{'x':0,'y':0,'w':96,'h':64}}]}}})
 audit[name]={'file':file,'sha256':hashlib.sha256((SRC/file).read_bytes()).hexdigest(),'source_size':im.size,'visible_height':b[3]-b[1],'cell':[96,64],'offset':[offset,8],'anchor':[32,56],'native_pixels_preserved':True}
frames['human'].save(ROOT/'icons/hanzo_body.png')
sheet=Image.new('RGBA',(192,64));sheet.paste(frames['human'],(0,0));sheet.save(ROOT/'sprite/hanzo_probe#sheet.png')
def frame(x,duration):return {'duration':duration,'data':{'x':x,'y':0,'w':96,'h':64}}
anims={tag:{'frames':[frame(96,.12)]}for tag in ['run','attack','skill','skill2','ult','hit','dead','main']}
anims['idle']={'frames':[frame(0,.000001),frame(96,86400.)]}
write(ROOT/'sprite/hanzo_probe#anim.fanim',{'anchor_x':32/96,'anchor_y':56/64,'anims':anims})
icons=ROOT.parent/'hanzo_phase0/official_icons'
for slot,file in [('skill','6910.png'),('skill2','6920.png'),('ult','6930.png')]:
 dest=ROOT/f'icons/hanzo_{slot}.png'
 if icons.exists():shutil.copyfile(icons/file,dest)
 assert dest.exists()
d=json.loads((ROOT/'champion/mlbb_granger.data_champion').read_text())
d.update(id='mlbb_hanzo',category='Assassin',tags=['AD','Melee'],sprite='asset/mlbb_heroes/sprite/hanzo_probe',skill_icons=[f'asset/mlbb_heroes/icons/hanzo_{s}'for s in ['skill','skill2','ult']])
d['stat'].update(attack=90,hp=700,defence=22,magic_resistance=20,move_speed=1080,hp_regen=6)
d['growth'].update(attack=5,hp=90,defence=3,magic_resistance=2,move_speed=4,hp_regen=0)
for slot,op,cd,range_,casting,target in [('attack','basic',54,18000,'Targeting','Enemy'),('skill','feast',30,75000,'Targeting','EnemyWithoutTower'),('skill2','dash',30,40000,'Direction','EnemyWithoutTower'),('ult','ult',60,100000,'None','AllyOnlySelf')]:
 d[slot]={'action_name':slot,'description':f'#asset/base/text/champion?description.mlbb_hanzo.{slot}','duration':12 if slot=='attack'else 1,'cooltime':cd,'start_timing':0,'cancelable':True,'range':range_,'casting_type':casting,'casting_target':target,'attack_type':'BaseAttack'if slot=='attack'else'Skill','can_use_with_move':True,'effect':{'type':'Native','effect_ref':f'mlbb_heroes:hanzo_{op}'}}
d['passive']={'passive_ref':'mlbb_heroes:hanzo'}
for key in list(d):
 if key.startswith('view_'):del d[key]
d['view_buffs']=[{'type':'Animated','name':f'mlbb_hanzo_{form}'+('_left'if left else ''),'anim':f'asset/mlbb_heroes/effects/hanzo_probe_{form}'+('_left'if left else ''),'tag':'main','z':1}for form in ['human','spirit']for left in [False,True]]
for p in [ROOT/'champion/mlbb_hanzo.data_champion',ROOT/'sdk/candidate/champion/mlbb_hanzo.data_champion']:write(p,d)
p=ROOT/'style/champion_view.champion_view';d=json.loads(p.read_text());d['entries']['mlbb_hanzo']={'face':{'x':0,'y':-40},'center':{'x':0,'y':-20},'banpick_center':{'x':0,'y':-18}};write(p,d)
Y=lambda s:f'<#ffb900ff>{s}<>'
C=lambda s:f'<#5ad0e6ff>{s}<>'
AD=lambda s:f'<i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>{s}<>'
desc={'name':'Hanzo','attack':f'Strikes an enemy for {AD("100% Attack")} physical damage. Damaging basic attacks apply Dark Ninjutsu.',
 'skill':f'{C("Demon Feast")}: At {Y("5")} Dark Ninjutsu stacks, strikes a nearby enemy for {AD("140 + 6 per level + 80% bonus Attack")} physical damage and restores {Y("50 + 3 per level + 30% Attack")} Health. Cooldown: {Y("4.8s")}. In Hanekage form, {C("Soul Snatch")} lunges toward an enemy and deals {AD("80 + 4 per level + 60% bonus Attack")} physical damage, increased by {Y("20%")} per stack. At five stacks, restores the same Health. Cooldown: {Y("10s")}. Consumes the target’s stacks.',
 'skill2':f'{C("Dark Mist")}: Dashes {Y("30")} range while respecting walls. In Hanekage form, {C("Black Mist")} dashes {Y("40")} range through walls. Cooldown: {Y("10s")}.',
 'ult':f'{C("Pinnacle Ninja")}: Leaves a protected body behind and projects Hanekage with {Y("70%")} maximum Health, {Y("35%")} increased Movement Speed and {Y("25%")} increased Attack Speed. Energy lasts {Y("10s")}. Recasting returns to the body; depleted energy draws the body toward Hanekage over {Y("1s")}. Returning restores stored body Health and heals {Y("90 + 50% bonus Attack")}. Cooldown after returning: {Y("30s")}. Defeated Hanekage forces a return, applies a {Y("50%")} Movement Speed reduction for {Y("6s")} and doubles this cooldown.',
 'passive':f'{C("Ame no Habakiri")}: Damaging basic attacks apply up to {Y("5")} Dark Ninjutsu stacks to each target for {Y("6s")}. Bonus Attack is Attack above {Y("90 + 5 per level")}. Level-based bonuses count levels gained after level one.'}
p=ROOT/'text/champion.i18n';d=json.loads(p.read_text())
for locale in d.values():locale['description']['mlbb_hanzo']=desc.copy();locale['skill_name']['mlbb_hanzo']={'passive':'Ame no Habakiri','skill1':'Demon Feast / Soul Snatch','skill2':'Dark Mist / Black Mist','ult':'Pinnacle Ninja / Return'}
write(p,d)
p=ROOT/'mod.mod_info';d=json.loads(p.read_text());d.update(version='1.1.149',description='25 HEROES · Hanzo projection test candidate: human/Hanekage masters, protected parked body, separate health pools, wall traversal and return. Static art; native lethal safety and body rendering await owner playtest.');write(p,d)
for name in ['build_source_archive.py','rewrap_runtime_zip.py']:
 p=ROOT/'tools'/name;s=p.read_text().replace('EXPECTED_CHAMPIONS = 24','EXPECTED_CHAMPIONS = 25');p.write_text(s)
write(ROOT/'reports/HANZO_ASSET_AUDIT_1149.json',audit)
print('PASS native master pixels preserved, mirrored/padded96x64; official icons, eight neutral colored text entries, native portrait carrier; 25 champions')
