#!/usr/bin/env python3
"""Add Eudora's eight locales and normalize the approved English copy overlay."""
from pathlib import Path
import json,re
ROOT=Path(__file__).resolve().parents[1]
def paint(s):
 token=re.compile(r'(?:\d+(?:\.\d+)?\s*\+\s*)?\d+(?:\.\d+)?%\s*(?:AP|AD)\b|\d+(?:,\d{3})*(?:\.\d+)?(?:[–-]\d+(?:\.\d+)?)?%?')
 def sub(m):
  t=m.group();c='a974ffff' if t.endswith('AP') else 'ff9028ff' if t.endswith('AD') else 'ffb900ff'
  icon='<i#asset/base/ui/banpick/champion_stat_icon:'+('ap_0' if t.endswith('AP') else 'ad_0')+'>' if t.endswith(('AP','AD')) else ''
  return icon+f'<#{c}>{t}<>'
 return token.sub(sub,s)
def decorate(names,values):
 out={'name':values[0],'attack':paint(values[1])}
 for key,title,body in zip(['skill','skill2','ult','passive'],[names[1],names[2],names[3],names[0]],values[2:]):
  out[key]=f'<#{"e64a6aff" if key=="ult" else "5ad0e6ff"}>{title}<>: '+paint(body)
 return out

N={
'en':['Superconductor','Forked Lightning','Ball Lightning',"Thunder’s Wrath"],
'zh-hans':['超导','叉状闪电','雷电法球','雷霆之怒'],
'zh-hant':['超導','叉狀閃電','雷電法球','雷霆之怒'],
'ko':['초전도','갈래 번개','번개 구체','천둥의 분노'],
'ja':['超伝導','フォークライトニング','ボールライトニング','雷霆の怒り'],
'fr':['Supraconducteur','Éclair fourchu','Orbe de foudre','Courroux du tonnerre'],
'ru':['Сверхпроводник','Разветвлённая молния','Шаровая молния','Гнев грома'],
'vi':['Siêu Dẫn','Sét Phân Nhánh','Cầu Sét','Thịnh Nộ Lôi Đình'],
}
# Values describe the approved mod, including the explicit balance adaptations.
V={
'en':['Eudora','Basic attacks launch a lightning bolt that deals 100% AD physical damage.',
'Strikes enemies in a fan for 100 + 55% AP magic damage, doubled against lane minions. Enemies already marked are linked for up to 1s, taking 4 pulses of 4 + 2% AP and a final hit of 100 + 55% AP if the link holds. Eudora gains 20% Movement Speed while linked. A final hit reduces this skill’s cooldown by 1.5s, once per cast. The link breaks beyond 95,000 range. Cooldown: 7s.',
'Launches an orb that reduces the target’s Magic Resistance by 8 for 1.8s, deals 115 + 45% AP magic damage and stuns for 0.8s. Against an enemy already marked, the same effects also hit nearby enemies. Each enemy is hit once. Cooldown: 11s.',
'Strikes a ground area for 220 + 100% AP magic damage at the center or 110 + 50% AP in the outer ring. Enemies already marked leave warnings at their hit locations. After 0.6s, those areas explode for 135 + 65% AP magic damage. Overlapping explosions from this cast hit each enemy once. Cooldown: 32s.',
'Skill hits mark enemy heroes and jungle monsters for 5s. Skills gain additional effects against enemies already marked. Basic attacks do not apply the mark; marks do not stack.'],
'zh-hans':['尤朵拉','普攻发射闪电，造成100% AD物理伤害。',
'以扇形闪电造成100 + 55% AP魔法伤害，对兵线小兵伤害翻倍。已被标记的敌人会被连接最多1秒，承受4次4 + 2% AP伤害；连接保持至结束时，再造成100 + 55% AP伤害。连接期间，尤朵拉获得20%移速。终结命中使本技能冷却减少1.5秒，每次施放仅触发一次。距离超过95,000时连接断开。冷却：7秒。',
'发射雷球，使目标魔抗降低8点，持续1.8秒，并造成115 + 45% AP魔法伤害及0.8秒眩晕。若目标已有标记，附近敌人也受到相同效果。每个敌人仅命中一次。冷却：11秒。',
'轰击地面区域，中心造成220 + 100% AP魔法伤害，外环造成110 + 50% AP伤害，两者不叠加。已有标记的敌人在命中位置留下预警，0.6秒后爆炸，造成135 + 65% AP魔法伤害。同次施放的重叠爆炸对每个敌人仅命中一次。冷却：32秒。',
'技能命中敌方英雄和野怪时施加5秒标记。技能对已有标记的敌人产生额外效果。普攻不施加标记，标记不叠层。'],
'zh-hant':['尤朵拉','普攻發射閃電，造成100% AD物理傷害。',
'以扇形閃電造成100 + 55% AP魔法傷害，對兵線小兵傷害加倍。已有標記的敵人會被連接最多1秒，承受4次4 + 2% AP傷害；連接維持至結束時，再造成100 + 55% AP傷害。連接期間，尤朵拉獲得20%移速。終結命中使本技能冷卻減少1.5秒，每次施放僅觸發一次。距離超過95,000時連接中斷。冷卻：7秒。',
'發射雷球，使目標魔抗降低8點，持續1.8秒，並造成115 + 45% AP魔法傷害及0.8秒暈眩。若目標已有標記，附近敵人也受到相同效果。每個敵人僅命中一次。冷卻：11秒。',
'轟擊地面區域，中心造成220 + 100% AP魔法傷害，外環造成110 + 50% AP傷害，兩者不疊加。已有標記的敵人在命中位置留下預警，0.6秒後爆炸，造成135 + 65% AP魔法傷害。同次施放的重疊爆炸對每個敵人僅命中一次。冷卻：32秒。',
'技能命中敵方英雄及野怪時施加5秒標記。技能對已有標記的敵人產生額外效果。普攻不施加標記，標記不疊層。'],
'ko':['유도라','기본 공격이 번개를 발사해 100% AD의 물리 피해를 준다.',
'부채꼴 범위에 100 + 55% AP의 마법 피해를 준다. 라인 미니언에게는 피해가 두 배다. 이미 표식이 있는 적을 최대 1초 동안 연결해 4 + 2% AP의 피해를 4회 주고, 연결이 유지되면 마지막으로 100 + 55% AP의 피해를 준다. 연결 중 이동 속도가 20% 증가한다. 마지막 타격 적중 시 재사용 대기시간이 1.5초 감소하며, 시전당 한 번만 적용된다. 거리가 95,000을 넘으면 연결이 끊어진다. 재사용 대기시간: 7초.',
'번개 구체가 대상의 마법 저항력을 1.8초 동안 8 낮추고, 115 + 45% AP의 마법 피해와 0.8초 기절을 준다. 대상에게 이미 표식이 있으면 주변 적에게도 같은 효과가 적용된다. 각 적은 한 번만 맞는다. 재사용 대기시간: 11초.',
'지면을 강타해 중심에 220 + 100% AP, 바깥 고리에 110 + 50% AP의 마법 피해를 준다. 이미 표식이 있던 적의 피격 위치에 경고가 남고, 0.6초 후 폭발해 135 + 65% AP의 마법 피해를 준다. 같은 시전의 폭발이 겹쳐도 각 적은 한 번만 맞는다. 재사용 대기시간: 32초.',
'스킬 적중 시 적 영웅과 정글 몬스터에게 5초 동안 표식을 남긴다. 이미 표식이 있는 적에게 스킬의 추가 효과가 발동한다. 기본 공격은 표식을 남기지 않으며 표식은 중첩되지 않는다.'],
'ja':['エウドラ','通常攻撃で雷の弾を放ち、100% ADの物理ダメージを与える。',
'扇状の範囲に100 + 55% APの魔法ダメージを与える。レーンミニオンにはダメージが2倍になる。すでにマークのある敵と最大1秒間つながり、4 + 2% APのダメージを4回与える。接続が続くと最後に100 + 55% APのダメージを与える。接続中は移動速度が20%増加する。最後の一撃が当たると、このスキルのCDが1.5秒短縮される。1回の発動につき1度のみ。距離が95,000を超えると接続が切れる。CD：7秒。',
'雷球で対象の魔法防御を1.8秒間8低下させ、115 + 45% APの魔法ダメージと0.8秒のスタンを与える。対象にすでにマークがあれば、周囲の敵にも同じ効果が発生する。各敵への命中は1回のみ。CD：11秒。',
'地面の範囲を雷で攻撃し、中心に220 + 100% AP、外周に110 + 50% APの魔法ダメージを与える。すでにマークのある敵は命中地点に予告を残す。0.6秒後、その地点が爆発して135 + 65% APの魔法ダメージを与える。同じ発動の爆発が重なっても、各敵への命中は1回のみ。CD：32秒。',
'スキルが敵ヒーローやジャングルモンスターに当たると、5秒間のマークを付与する。すでにマークのある敵にはスキルの追加効果が発生する。通常攻撃では付与せず、マークは重複しない。'],
'fr':['Eudora','Les attaques de base lancent un éclair qui inflige 100% AD de dégâts physiques.',
'Frappe en éventail pour 100 + 55% AP de dégâts magiques, doublés contre les sbires de voie. Les ennemis déjà marqués sont reliés pendant 1 s maximum : 4 impulsions de 4 + 2% AP, puis 100 + 55% AP si le lien tient. Eudora gagne 20% de vitesse de déplacement tant qu’un lien reste actif. Le coup final réduit le délai de récupération de 1.5 s, une seule fois par lancement. Le lien casse au-delà de 95,000 de portée. Récupération : 7 s.',
'Lance un orbe qui réduit la résistance magique de 8 pendant 1.8 s, inflige 115 + 45% AP de dégâts magiques et étourdit pendant 0.8 s. Si la cible était déjà marquée, les mêmes effets touchent les ennemis proches. Chaque ennemi est touché une fois. Récupération : 11 s.',
'Frappe une zone au sol : 220 + 100% AP de dégâts magiques au centre ou 110 + 50% AP sur le bord. Les ennemis déjà marqués laissent un avertissement à leur position d’impact. Après 0.6 s, ces zones explosent pour 135 + 65% AP de dégâts magiques. Les explosions d’un même lancement ne touchent chaque ennemi qu’une fois. Récupération : 32 s.',
'Les compétences marquent les héros ennemis et les monstres de jungle pendant 5 s. Elles produisent des effets supplémentaires contre les ennemis déjà marqués. Les attaques de base ne marquent pas. Les marques ne se cumulent pas.'],
'ru':['Эйдора','Базовые атаки выпускают молнию и наносят 100% AD физического урона.',
'Бьёт молнией по вееру, нанося 100 + 55% AP магического урона; миньонам на линиях наносится двойной урон. Уже отмеченные враги соединяются с Эйдорой на срок до 1 с: 4 импульса по 4 + 2% AP, затем 100 + 55% AP, если связь сохранилась. Пока связь действует, скорость передвижения повышена на 20%. Финальное попадание сокращает перезарядку на 1.5 с, один раз за применение. Связь рвётся на расстоянии свыше 95,000. Перезарядка: 7 с.',
'Выпускает сферу, которая снижает магическое сопротивление цели на 8 на 1.8 с, наносит 115 + 45% AP магического урона и оглушает на 0.8 с. Если цель уже была отмечена, те же эффекты действуют на ближайших врагов. Каждый враг получает одно попадание. Перезарядка: 11 с.',
'Бьёт по области на земле: 220 + 100% AP магического урона в центре или 110 + 50% AP во внешнем кольце. Уже отмеченные враги оставляют предупреждение в месте попадания. Через 0.6 с эти области взрываются, нанося 135 + 65% AP магического урона. Перекрывающиеся взрывы одного применения поражают каждого врага один раз. Перезарядка: 32 с.',
'Попадания навыками отмечают вражеских героев и монстров леса на 5 с. Против уже отмеченных врагов навыки получают дополнительные эффекты. Базовые атаки не оставляют метки. Метки не суммируются.'],
'vi':['Eudora','Đòn đánh thường phóng tia sét, gây sát thương vật lý bằng 100% AD.',
'Giáng sét theo hình quạt, gây 100 + 55% AP sát thương phép; sát thương lên lính đường tăng gấp đôi. Kẻ địch đã có dấu ấn bị nối tối đa 1 giây, chịu 4 lần sát thương 4 + 2% AP và thêm 100 + 55% AP nếu liên kết được duy trì đến cuối. Eudora tăng 20% tốc chạy khi còn liên kết. Đòn kết thúc giảm hồi chiêu 1.5 giây, chỉ một lần mỗi lượt dùng. Liên kết đứt khi khoảng cách vượt 95,000. Hồi chiêu: 7 giây.',
'Phóng cầu sét, giảm 8 kháng phép trong 1.8 giây, gây 115 + 45% AP sát thương phép và làm choáng 0.8 giây. Nếu mục tiêu đã có dấu ấn, kẻ địch ở gần cũng chịu các hiệu ứng tương tự. Mỗi kẻ địch chỉ trúng một lần. Hồi chiêu: 11 giây.',
'Giáng sét xuống một vùng đất, gây 220 + 100% AP sát thương phép ở tâm hoặc 110 + 50% AP ở vòng ngoài. Kẻ địch đã có dấu ấn để lại cảnh báo tại vị trí trúng đòn. Sau 0.6 giây, các vùng đó phát nổ, gây 135 + 65% AP sát thương phép. Các vụ nổ chồng nhau trong cùng lượt dùng chỉ đánh mỗi kẻ địch một lần. Hồi chiêu: 32 giây.',
'Kỹ năng đánh trúng sẽ đánh dấu tướng địch và quái rừng trong 5 giây. Kỹ năng có thêm hiệu ứng lên kẻ địch đã được đánh dấu. Đòn đánh thường không tạo dấu ấn; dấu ấn không cộng dồn.'],
}

def main():
 p=ROOT/'text/champion.i18n';d=json.loads(p.read_text());review=[]
 overlay=ROOT/'source/eudora/english_copy_overlay.i18n'
 if overlay.exists():
  e=json.loads(overlay.read_text())['en']
  for hero,desc in e['description'].items():
   for key in ['attack','skill','skill2','ult','passive']:
    original=json.loads((ROOT/'source/eudora/english_original.i18n').read_text())
    before=original['en']['description'][hero][key];s=re.sub(r'<[^>]*>','',desc[key])
    s=s.replace('Magic Power magic damage','AP magic damage').replace('Attack as physical damage','AD physical damage')
    s=re.sub(r'(?<=%) Attack\b',' AD',s);s=re.sub(r'(?<=%) Magic Power\b',' AP',s)
    title=d['en']['skill_name'].get(hero,{}).get({'skill':'skill1'}.get(key,key))
    if key!='attack' and title:
     if s.startswith(title+':'):s=s[len(title)+1:].lstrip()
     after=f'<#{"e64a6aff" if key=="ult" else "5ad0e6ff"}>{title}<>: '+paint(s)
    else:after=paint(s)
    assert not re.search(r'\b(you|your|yours|we|our|I)\b',s,re.I),(hero,key,s)
    d['en']['description'][hero][key]=after
    review.append({'hero':hero,'slot':key,'before':before,'after':after})
 for locale,names in N.items():
  d[locale]['description']['mlbb_eudora']=decorate(names,V[locale])
  d[locale]['skill_name']['mlbb_eudora']=dict(zip(['passive','skill1','skill2','ult'],names))
 p.write_text(json.dumps(d,ensure_ascii=False,indent=1)+'\n')
 (ROOT/'reports/ENGLISH_COPY_REVIEW.json').write_text(json.dumps(review,ensure_ascii=False,indent=1)+'\n')
 print('Added Eudora in',len(N),'locales; reviewed',len(review),'existing English descriptions.')
if __name__=='__main__':main()
