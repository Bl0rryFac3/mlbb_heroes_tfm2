"""Compact neutral colored gameplay text; full formulas live in balance notes."""
from pathlib import Path
import json,re
ROOT=Path(__file__).resolve().parents[1]
Y=lambda s:f'<#ffb900ff>{s}<>'
C=lambda s:f'<#5ad0e6ff>{s}<>'
AD=lambda s:f'<i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>{s}<>'
H=lambda s:f'<#66cc66ff>{s}<>'
R=lambda s:f'<#ef5350ff>{s}<>'
D={'name':'Hanzo',
'attack':f'Strikes for {AD("100% Attack")} physical damage and applies a mark.',
'skill':f'{C("Demon Feast")}: At {Y("5 marks")}, bites for {AD("170 + 6 per level + 100% bonus Attack")} physical damage, heals and refunds {Y("60%")} cooldown. Ordinary camps are devoured, healing {H("20% maximum Health")}.\n{C("Soul Snatch")}: Lunges for {AD("110 + 5 per level + 80% bonus Attack")} physical damage, increased {Y("20% per mark")}. At {Y("5 marks")}, adds damage based on maximum Health and heals.\nCooldown: {Y("12s / 10s")}.',
'skill2':f'{C("Dark Mist")}: Dashes and gains {Y("15% Movement Speed")} for {Y("2s")}. Leaves {Y("5")} mist pulses.\n{C("Black Mist")}: Dashes through walls for {AD("110 + 4 per level + 55% bonus Attack")} physical damage and {R("0.4s airborne")}. Leaves {Y("4")} pulses.\nMist deals physical damage every {Y("0.5s")}, applies marks and {R("slows 35%")}; it cannot critically strike. Cooldown: {Y("10s per form")}.',
'ult':f'Projects Hanekage for up to {Y("15s")}, leaving an invulnerable body. Gains {Y("35% Movement Speed")}, {Y("25% Attack Speed")} and wall traversal, with {Y("30% less maximum Health")}.\nRecast returns and heals {H("90 + 50% bonus Attack")}. Empty Energy draws the body to Hanekage. Defeat forces a return to the body, {R("slows Hanzo 50% for 6s")} and doubles the {Y("30s")} cooldown.',
'passive':f'Basic attacks and mist hits apply up to {Y("5 marks")} for {Y("6s")}. New hits refresh them. Hanekage starts with {Y("100 Energy")} and spends {Y("10 per second")}; hits and nearby deaths restore Energy.'}
p=ROOT/'text/champion.i18n';d=json.loads(p.read_text())
for loc in d.values():
 loc['description']['mlbb_hanzo']=D.copy()
 loc['skill_name']['mlbb_hanzo'].update(skill1='Demon Feast',skill2='Dark Mist',ult='Pinnacle Ninja')
p.write_text(json.dumps(d,ensure_ascii=False,indent=1)+'\n')
for k,v in D.items():
 plain=re.sub(r'<[^>]*>','',v);assert not re.search(r'\b(you|your|yours)\b',plain,re.I)
 if k not in ['name','attack']:assert len(plain)<=425,(k,len(plain))
 print(k,len(plain))
