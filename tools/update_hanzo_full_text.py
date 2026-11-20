"""Neutral colored full-kit descriptions with honest English fallback."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[1]
Y=lambda s:f'<#ffb900ff>{s}<>'
C=lambda s:f'<#5ad0e6ff>{s}<>'
AD=lambda s:f'<i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>{s}<>'
H=lambda s:f'<#66cc66ff>{s}<>'
R=lambda s:f'<#ef5350ff>{s}<>'
D={'name':'Hanzo',
 'attack':f'Strikes an enemy for {AD("100% Attack")} physical damage and applies Dark Ninjutsu.',
 'skill':f'{C("Demon Feast")}: Consumes {Y("5")} Dark Ninjutsu stacks on a nearby enemy champion or boss, dealing {AD("140 + 6 per level + 80% bonus Attack")} physical damage and restoring {H("50 + 3 per level + 30% Attack")} Health. A successful hit refunds {Y("60%")} of its {Y("12s")} cooldown. Ordinary jungle monsters are devoured instead, restoring {H("20% maximum Health")} over {Y("2s")}.\n{C("Soul Snatch")}: In Hanekage form, lunges toward an enemy and strikes after {Y("0.25s")} for {AD("80 + 4 per level + 60% bonus Attack")} physical damage, increased by {Y("20%")} per stack. At five stacks, adds {AD("(6 + 0.01 × bonus Attack)%")} of the target’s maximum Health as physical damage, up to {Y("12%")} and capped at {Y("220")} against bosses, and restores the same Health. Consumes the target’s stacks. Cooldown: {Y("10s")}.',
 'skill2':f'{C("Dark Mist")}: Dashes {Y("30")} range and gains {Y("15%")} Movement Speed for {Y("2s")}. Leaves a mist trail that hits {Y("5")} times at {Y("0.5s")} intervals for {AD("18 + 1 per level + 15% bonus Attack")} physical damage per hit.\n{C("Black Mist")}: In Hanekage form, dashes {Y("40")} range through walls. Enemies along the path take {AD("90 + 4 per level + 45% bonus Attack")} physical damage and become {R("airborne")} for {Y("0.4s")}. Hitting a champion restores {Y("20")} Energy once. The trail hits {Y("4")} times at {Y("0.5s")} intervals for {AD("24 + 1 per level + 15% bonus Attack")} physical damage per hit. Both mist trails {R("slow")} enemies by {Y("35%")}; mist hits count as basic-attack damage and cannot critically strike. Cooldown: {Y("10s")} per form.',
 'ult':f'{C("Pinnacle Ninja")}: Leaves a protected body behind and projects Hanekage with {Y("30%")} reduced maximum Health, {Y("35%")} increased Movement Speed, {Y("25%")} increased Attack Speed and wall traversal. Starts with {Y("100")} Energy, draining {Y("10")} each second. Recasting returns to the body; depleted Energy draws the body toward Hanekage over {Y("1s")}. Projection lasts at most {Y("15s")}. Returning restores stored body Health and heals {H("90 + 50% bonus Attack")}. Cooldown after returning: {Y("30s")}. Defeated Hanekage forces a return, applies a {Y("50%")} Movement Speed reduction for {Y("6s")} and doubles this cooldown.',
 'passive':f'{C("Ame no Habakiri")}: Damaging basic attacks, mist pulses and Black Mist path hits apply up to {Y("5")} Dark Ninjutsu stacks per target for {Y("6s")}. New stacks refresh the duration. Hanekage restores {Y("1.5")} Energy on basic contact with a champion and {Y("4.5")} against a boss. Nearby champion deaths restore {Y("20")} Energy; other nearby unit deaths restore {Y("2")}. Energy cannot exceed {Y("100")}. Bonus Attack is Attack above {Y("90 + 5 per level")}; level-based bonuses count levels gained after level one.'}
p=ROOT/'text/champion.i18n';d=json.loads(p.read_text())
for locale in d.values():locale['description']['mlbb_hanzo']=D.copy()
p.write_text(json.dumps(d,ensure_ascii=False,indent=1)+'\n')
print('PASS eight locales, neutral colored full-kit English fallback')
