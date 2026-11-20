#!/usr/bin/env python3
"""Angela's implemented kit in each existing locale; leave other heroes intact."""
import json
from pathlib import Path
p=Path(__file__).resolve().parents[1]/'text/champion.i18n'
x=json.loads(p.read_text())
text={
'en':[
"Love Waves: sends a travelling wave up to 92,000, dealing 80 + 30% AP magic damage to enemies and healing allies for 46 + 40% AP. Lover's Mark lasts 3s, up to 5 stacks: each adds 12% damage to subsequent waves and S2's final hit, and slows by 6%. Up to 5 charges; one charge recharges each 1s before cooldown reduction.",
"Puppet-on-a-String: connects an enemy within 95,000, dealing 95 + 25% AP magic damage. Slows progressively up to 45% over 3s. If still connected, deals 140 + 35% AP (increased by Lover's Marks) and roots for 0.7s. Breaks beyond 150,000 or on death/purge; no final hit on a broken link.",
"Heartguard: grants an allied hero 220 + 180% AP shield for 6s, at any distance. Angela disappears into that ally for 12s; her native AI casts S1/S2 from the ally's position while attached. A doll marks the host. Recast or the host's death ends attachment. She reappears at the host after the short banish expires. Cooldown 40s.",
"Smart Heart: casting a skill grants 7.5% Move Speed for 4s, up to 4 stacks (30%). While attached, this bonus belongs to the host. Uses and cooldowns carry across attachment; native cooldown counters can outlast the engine's displayed counters."],
'zh-hans':[
"爱意波动：向前方92,000发出移动波动，对敌人造成80 + 30%法术攻击的魔法伤害，为友方回复46 + 40%法术攻击生命。恋人印记持续3秒，最多5层；每层使后续波动和二技能终结伤害提高12%，并减速6%。最多5次充能；未计冷却缩减时每1秒恢复一次。",
"傀儡之线：连接95,000范围内的敌人，造成95 + 25%法术攻击的魔法伤害。3秒内逐渐减速至45%。连线保持到结束时，造成140 + 35%法术攻击的魔法伤害（受恋人印记增幅），并束缚0.7秒。距离超过150,000、死亡或驱散会中断连线，不触发终结伤害。",
"真心守护：为任意距离的一名友方英雄提供220 + 180%法术攻击的护盾，持续6秒。安琪拉消失并附身该友方12秒；附身时由原生AI从友方位置施放一、二技能。头顶人偶标记附身目标。再次施放或友方死亡时结束附身；短暂放逐结束后在友方位置现身。冷却40秒。",
"灵心：施放技能获得7.5%移动速度，持续4秒，最多4层（30%）。附身时加速转移给友方。附身前后共用充能和冷却；原生冷却计时可能比界面显示更久。"],
'zh-hant':[
"愛意波動：向前方92,000發出移動波動，對敵人造成80 + 30%法術攻擊的魔法傷害，為友方回復46 + 40%法術攻擊生命。戀人印記持續3秒，最多5層；每層使後續波動和二技能終結傷害提高12%，並減速6%。最多5次充能；未計冷卻縮減時每1秒恢復一次。",
"傀儡之線：連接95,000範圍內的敵人，造成95 + 25%法術攻擊的魔法傷害。3秒內逐漸減速至45%。連線保持到結束時，造成140 + 35%法術攻擊的魔法傷害（受戀人印記增幅），並束縛0.7秒。距離超過150,000、死亡或驅散會中斷連線，不觸發終結傷害。",
"真心守護：為任意距離的一名友方英雄提供220 + 180%法術攻擊的護盾，持續6秒。安琪拉消失並附身該友方12秒；附身時由原生AI從友方位置施放一、二技能。頭頂人偶標記附身目標。再次施放或友方死亡時結束附身；短暫放逐結束後在友方位置現身。冷卻40秒。",
"靈心：施放技能獲得7.5%移動速度，持續4秒，最多4層（30%）。附身時加速轉移給友方。附身前後共用充能和冷卻；原生冷卻計時可能比介面顯示更久。"],
'ko':[
"러브 웨이브: 전방 92,000까지 이동하는 파동이 적에게 80 + 마법력의 30%만큼 마법 피해를 주고 아군을 46 + 마법력의 40%만큼 회복합니다. 연인 표식은 3초간 최대 5중첩되며 중첩마다 이후 파동과 두 번째 스킬의 마지막 피해가 12% 증가하고 6% 둔화됩니다. 최대 5회 충전, 재사용 대기시간 감소 적용 전 1초마다 1회 충전됩니다.",
"꼭두각시 인형: 95,000 안의 적을 연결해 95 + 마법력의 25%만큼 마법 피해를 줍니다. 3초 동안 최대 45%까지 서서히 둔화합니다. 연결이 유지되면 140 + 마법력의 35%만큼 마법 피해를 주고(연인 표식으로 증가) 0.7초간 속박합니다. 거리 150,000 초과, 사망 또는 해제 시 연결이 끊기며 마지막 피해가 발생하지 않습니다.",
"하트 가드: 거리에 상관없이 아군 영웅에게 6초간 220 + 마법력의 180% 보호막을 줍니다. 안젤라는 사라져 12초간 아군에게 부착되며 원생 AI가 그 아군 위치에서 첫 번째와 두 번째 스킬을 사용합니다. 머리 위 인형으로 표시됩니다. 재사용하거나 아군이 사망하면 종료되고 짧은 추방이 끝난 뒤 아군 위치에 나타납니다. 재사용 대기시간 40초.",
"스마트 하트: 스킬 사용 시 4초간 이동 속도 7.5%를 얻고 최대 4중첩(30%)됩니다. 부착 중에는 이 효과가 아군에게 이전됩니다. 부착 전후 충전과 대기시간을 공유합니다. 원생 대기시간은 화면 표시보다 길 수 있습니다."],
'ja':[
"ラブウェーブ：前方92,000まで進む波が敵に80 + 魔力の30%の魔法ダメージを与え、味方を46 + 魔力の40%回復します。恋人の刻印は3秒間、最大5スタック。1スタックごとに後続の波と第2スキルの最終ダメージが12%増加し、6%減速します。最大5チャージ。クールダウン短縮前は1秒ごとに1回復します。",
"パペットストリング：95,000以内の敵と繋がり、95 + 魔力の25%の魔法ダメージを与えます。3秒かけて最大45%まで減速。繋がったままなら140 + 魔力の35%の魔法ダメージ（刻印で増加）と0.7秒の拘束を与えます。距離150,000超過、死亡、解除で切れた場合、最終ダメージは発生しません。",
"ハートガード：距離に関係なく味方英雄に220 + 魔力の180%のシールドを6秒付与。アンジェラは姿を消して12秒間その味方に憑依し、ネイティブAIが味方の位置から第1・第2スキルを使います。頭上の人形が目印です。再使用または味方の死亡で終了し、短い追放の終了後に味方の位置に現れます。クールダウン40秒。",
"スマートハート：スキル使用で移動速度が4秒間7.5%上昇し、最大4スタック（30%）。憑依中は味方に効果が移ります。チャージとクールダウンは憑依前後で共有されます。ネイティブの待ち時間が画面表示より長い場合があります。"],
'fr':[
"Vagues d'Amour : une vague avance sur 92 000, inflige 80 + 30% AP de dégâts magiques aux ennemis et soigne les alliés de 46 + 40% AP. La Marque dure 3s, jusqu'à 5 cumuls : chacun augmente de 12% les dégâts des vagues suivantes et du dernier coup de S2, et ralentit de 6%. Jusqu'à 5 charges ; une charge revient chaque seconde avant réduction des délais.",
"Marionnette à Fils : relie un ennemi à 95 000 maximum et inflige 95 + 25% AP de dégâts magiques. Le ralentissement atteint progressivement 45% en 3s. Si le lien tient, inflige 140 + 35% AP, amplifiés par les Marques, et immobilise 0,7s. Au-delà de 150 000, à la mort ou à la dissipation, le lien se rompt sans coup final.",
"Garde du Cœur : donne à un héros allié, sans limite de distance, un bouclier de 220 + 180% AP pendant 6s. Angela disparaît dans cet allié pendant 12s ; son IA native lance S1/S2 depuis sa position. Une poupée le signale. Une réactivation ou la mort de l'allié termine l'attachement. Angela réapparaît à sa position après le bref bannissement. Délai : 40s.",
"Cœur Malin : chaque sort donne 7,5% de vitesse de déplacement pendant 4s, jusqu'à 4 cumuls (30%). Pendant l'attachement, l'allié reçoit ce bonus. Charges et délais sont partagés entre les deux états ; le délai natif peut dépasser celui affiché par le moteur."],
'ru':[
"Волны любви: волна проходит до 92 000, наносит врагам 80 + 30% силы магии магического урона и лечит союзников на 46 + 40% силы магии. Клеймо действует 3 с, до 5 стаков: каждый повышает урон следующих волн и завершающего удара второго навыка на 12% и замедляет на 6%. До 5 зарядов; один восстанавливается каждую секунду до сокращения перезарядки.",
"Кукла на нитях: связывает врага в пределах 95 000 и наносит 95 + 25% силы магии магического урона. Замедление за 3 с нарастает до 45%. Если связь сохранилась, наносит 140 + 35% силы магии (усиливается клеймами) и обездвиживает на 0,7 с. Дистанция свыше 150 000, смерть или очищение обрывают нить без завершающего удара.",
"Хранитель сердца: даёт союзному герою на любой дистанции щит 220 + 180% силы магии на 6 с. Анжела исчезает внутри союзника на 12 с; её нативный ИИ применяет первый и второй навыки с его позиции. Над союзником появляется кукла. Повторное применение или смерть союзника завершают прикрепление. Анжела появляется у него после короткого изгнания. Перезарядка 40 с.",
"Умное сердце: применение навыка даёт 7,5% скорости передвижения на 4 с, до 4 стаков (30%). При прикреплении бонус получает союзник. Заряды и перезарядки общие для обоих состояний; нативный таймер может превышать показания интерфейса."],
'vi':[
"Sóng Tình Yêu: sóng di chuyển tới 92.000, gây 80 + 30% Sức Mạnh Phép sát thương phép lên địch và hồi 46 + 40% Sức Mạnh Phép cho đồng minh. Dấu Ấn kéo dài 3 giây, tối đa 5 tầng: mỗi tầng tăng 12% sát thương sóng tiếp theo và đòn kết thúc kỹ năng 2, đồng thời làm chậm 6%. Tối đa 5 tích trữ; hồi một lần mỗi giây trước giảm hồi chiêu.",
"Con Rối Dây: nối địch trong tầm 95.000, gây 95 + 25% Sức Mạnh Phép sát thương phép. Làm chậm tăng dần tới 45% trong 3 giây. Nếu dây còn nguyên, gây 140 + 35% Sức Mạnh Phép (tăng theo Dấu Ấn) và trói 0,7 giây. Quá 150.000, chết hoặc bị thanh tẩy sẽ đứt dây, không có đòn kết thúc.",
"Hộ Vệ Trái Tim: tạo khiên 220 + 180% Sức Mạnh Phép trong 6 giây cho một tướng đồng minh ở bất kỳ đâu. Angela biến mất vào đồng minh trong 12 giây; AI gốc dùng kỹ năng 1/2 từ vị trí của họ. Búp bê trên đầu đánh dấu chủ thể. Dùng lại hoặc đồng minh chết sẽ kết thúc nhập thể. Angela xuất hiện ở vị trí họ sau thời gian trục xuất ngắn. Hồi chiêu 40 giây.",
"Trái Tim Thông Tuệ: dùng kỹ năng nhận 7,5% Tốc Độ Di Chuyển trong 4 giây, tối đa 4 tầng (30%). Khi nhập thể, đồng minh nhận hiệu ứng này. Hai trạng thái dùng chung tích trữ và hồi chiêu; bộ đếm gốc có thể dài hơn hiển thị của trò chơi."]}
for locale,lines in text.items():
 for key,line in zip(('skill','skill2','ult','passive'),lines):x[locale]['description']['mlbb_angela'][key]=line
p.write_text(json.dumps(x,ensure_ascii=False,indent=1)+'\n')
print('Updated Angela descriptions in',len(text),'locales')
