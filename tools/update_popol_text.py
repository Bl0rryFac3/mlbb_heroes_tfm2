#!/usr/bin/env python3
"""Update only Popol's neutral, colored tooltips; preserve other champions/locales."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
Y=lambda s:f'<#ffb900ff>{s}<>'
AD=lambda s:f'<i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>{s}<>'
C=lambda s:f'<#5ad0e6ff>{s}<>'
CC=lambda s:f'<#ef5350ff>{s}<>'
D={
'name':'Popol & Kupa',
'attack':f'Popol throws a spear that deals {AD("100% AD")} physical damage on impact. Kupa bites nearby enemies for {AD("45% AD")} physical damage, increased to {AD("75% AD")} in Alpha Wolf Form. Kupa’s ordinary bites use Popol’s basic-attack item effects.',
'skill':f'{C("Bite ’em, Kupa!")}: Popol throws a spear for {AD("50 + 50% AD")} physical damage and commands Kupa to bite the target for {AD("60 + 60% AD")} physical damage, {CC("slowing")} it by {Y("50%")} for {Y("1s")}. Alpha Wolf Form: the bite instead deals {AD("90 + 75% AD")} physical damage to nearby enemies and knocks them {CC("airborne")} for {Y("0.6s")}.',
'skill2':f'{C("Kupa, Help!")}: Popol gains a {Y("90 + 50% AD")} shield for {Y("3s")} and calls Kupa back. When Kupa arrives, Popol mounts him for up to {Y("1.5s")}, gaining {Y("50%")} Movement Speed while Kupa recovers {Y("25%")} Max HP over the ride. Attacks, other skills and hard crowd control end the ride. Alpha Wolf Form: the shield increases to {Y("140 + 70% AD")} and Kupa fully recovers HP at mount start.',
'ult':f'<#e64a6aff>We Are Angry!<>: Kupa fully recovers HP and enters {C("Alpha Wolf Form")} for {Y("9s")}, gaining {Y("35%")} Max HP and enhancing Bite ’em, Kupa! and Kupa, Help! Cooperative stack bonuses are doubled. Requires a living Kupa.',
'passive':f'{C("We Are Friends")}: When Popol and Kupa damage the same enemy within {Y("2s")}, both gain {Y("5%")} Attack Speed and {Y("2%")} Movement Speed for {Y("4s")}, stacking up to {Y("3")} times. Each paired exchange grants one stack.\nKupa has {Y("560 + 80 per level after Lv.1 + 50% of Popol’s bonus Max HP")} HP and inherits Popol’s current Armor, Magic Resist and Attack Speed. After {Y("5s")} out of combat, Kupa recovers {Y("8%")} Max HP per second. If Kupa falls, Popol begins a stationary {Y("3s")} prayer after {Y("3s")} without combat; movement, damage or another action interrupts it. Successful revival has a {Y("30s")} cooldown.\n{C("Popol’s Surprise")}: During combat, Popol automatically places a trap every {Y("10s")}, up to {Y("2")}. Traps arm after {Y("0.5s")} and last {Y("15s")}. An enemy triggers {AD("25 + 20% AD")} physical damage and {Y("0.6s")} of {CC("immobilization")}, leaving frost for {Y("3s")} that {CC("slows")} enemies by {Y("20%")} and reduces Armor by {Y("12%")}. Frost from the same Popol does not stack. Placement pauses during casting, recall, riding and prayer.'
}
N={'passive':'We Are Friends','skill1':'Bite ’em, Kupa!','skill2':'Kupa, Help!','ult':'We Are Angry!'}
p=ROOT/'text/champion.i18n';d=json.loads(p.read_text())
# Match the existing Popol locale fallback: English in all eight entries.
for locale in d.values():
 locale['description']['mlbb_popol_kupa']=D.copy()
 locale['skill_name']['mlbb_popol_kupa']=N.copy()
p.write_text(json.dumps(d,ensure_ascii=False,indent=1)+'\n')
print('Updated neutral, colored Popol text in all 8 existing locale entries; English fallback retained.')
