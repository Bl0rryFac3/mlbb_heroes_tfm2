#!/usr/bin/env python3
"""Merge Chou's localization/style metadata and route its selected audio clips."""
from __future__ import annotations
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

STRINGS = {
    "en": {
        "name": "Chou",
        "passive": "Only Fast",
        "skill1": "Jeet Kune Do",
        "skill2": "Shunpo",
        "ult": "The Way of Dragon",
        "attack": "Basic attacks deal 100% AD physical damage. After moving about 42,000 TFM2 world units, Chou's next basic attack is empowered to 180% AD, slows by 80% for 1 second, and reduces physical defense by 10 for 2 seconds (up to 2 stacks). The movement threshold is a TFM2 approximation.",
        "passive_desc": "<#5ad0e6ff>Only Fast<>: After moving about <#ffb900ff>42,000<> TFM2 world units, Chou's next Basic Attack deals <#ff9028ff>180% AD<> physical damage, slows by <#ffb900ff>80%<> for <#ffb900ff>1s<>, and reduces Physical Defense by <#ffb900ff>10<> for <#ffb900ff>2s<> (up to <#ffb900ff>2<> stacks). The distance is tuned to TFM2's map scale.",
        "skill": "<#ff9028ff>Jeet Kune Do<>: Cast up to <#ffb900ff>3<> times: a rising kick, a dash uppercut, then a leaping downward slam that knocks enemies airborne. Each hit deals <#ff9028ff>100–165 + 45% Total Physical Attack<>. The first two casts refresh this skill so the combo can continue; the third starts its cooldown.",
        "skill2_desc": "<#5ad0e6ff>Shunpo<>: Dash toward a chosen direction, gaining <#ffb900ff>Control Immunity<> during the dash. On arrival, gain a shield equal to <#5bd18aff>200–300 + 100% Extra Physical Attack<> and <#ffb900ff>15<> Physical Penetration for <#ffb900ff>2.5s<>.",
        "ult_desc": "<#e64a6aff>The Way of Dragon<>: Roundhouse-kick an enemy champion for <#ff9028ff>275–420 + 110% Extra Physical Attack<> and knock them away. Recast within <#ffb900ff>4s<> to chase and perform an aerial kick flurry for <#ff9028ff>330–500 + 132% Extra Physical Attack<>. Damage is one engine event until live proc testing validates splitting it across kicks.",
    },
    "zh-hans": {
        "name": "周",
        "passive": "迅捷",
        "skill1": "截拳道",
        "skill2": "瞬步",
        "ult": "龙之道",
        "attack": "普通攻击造成100%物理攻击的物理伤害。移动约42,000个TFM2世界单位后，下一次普攻强化至180%物理攻击，减速80%持续1秒，并降低10点物理防御持续2秒，最多叠加2层。移动距离为适配TFM2地图的近似值。",
        "passive_desc": "<#5ad0e6ff>迅捷<>：移动约<#ffb900ff>42,000<>个TFM2世界单位后，下一次普攻造成<#ff9028ff>180%物理攻击<>的物理伤害，使目标减速<#ffb900ff>80%<>持续<#ffb900ff>1秒<>，并降低<#ffb900ff>10<>点物理防御，持续<#ffb900ff>2秒<>，最多<#ffb900ff>2<>层。距离按TFM2地图比例调整。",
        "skill": "<#ff9028ff>截拳道<>：最多施放<#ffb900ff>3<>次：上踢、突进上勾踢，最后跃起下砸并击飞敌人。每次命中造成<#ff9028ff>100–165 + 45%总物理攻击<>伤害。前两次施放会刷新技能以继续连招；第三次后进入冷却。",
        "skill2_desc": "<#5ad0e6ff>瞬步<>：朝指定方向突进，突进期间获得<#ffb900ff>控制免疫<>。抵达后获得护盾，数值为<#5bd18aff>200–300 + 100%额外物理攻击<>，并获得<#ffb900ff>15<>点物理穿透，持续<#ffb900ff>2.5秒<>。",
        "ult_desc": "<#e64a6aff>龙之道<>：回旋踢击敌方英雄，造成<#ff9028ff>275–420 + 110%额外物理攻击<>伤害并将其击退。<#ffb900ff>4秒<>内再次施放可追击目标并在空中连续踢击，造成<#ff9028ff>330–500 + 132%额外物理攻击<>伤害。通过实战验证多段触发前，伤害按一次引擎事件结算。",
    },
    "zh-hant": {
        "name": "周",
        "passive": "迅捷",
        "skill1": "截拳道",
        "skill2": "瞬步",
        "ult": "龍之道",
        "attack": "普通攻擊造成100%物理攻擊的物理傷害。移動約42,000個TFM2世界單位後，下一次普攻強化至180%物理攻擊，緩速80%持續1秒，並降低10點物理防禦持續2秒，最多疊加2層。移動距離為配合TFM2地圖的近似值。",
        "passive_desc": "<#5ad0e6ff>迅捷<>：移動約<#ffb900ff>42,000<>個TFM2世界單位後，下一次普攻造成<#ff9028ff>180%物理攻擊<>的物理傷害，使目標緩速<#ffb900ff>80%<>持續<#ffb900ff>1秒<>，並降低<#ffb900ff>10<>點物理防禦，持續<#ffb900ff>2秒<>，最多<#ffb900ff>2<>層。距離依TFM2地圖比例調整。",
        "skill": "<#ff9028ff>截拳道<>：最多施放<#ffb900ff>3<>次：上踢、突進上勾踢，最後躍起下砸並擊飛敵人。每次命中造成<#ff9028ff>100–165 + 45%總物理攻擊<>傷害。前兩次施放會重置技能以繼續連招；第三次後進入冷卻。",
        "skill2_desc": "<#5ad0e6ff>瞬步<>：朝指定方向突進，突進期間獲得<#ffb900ff>控制免疫<>。抵達後獲得護盾，數值為<#5bd18aff>200–300 + 100%額外物理攻擊<>，並獲得<#ffb900ff>15<>點物理穿透，持續<#ffb900ff>2.5秒<>。",
        "ult_desc": "<#e64a6aff>龍之道<>：迴旋踢擊敵方英雄，造成<#ff9028ff>275–420 + 110%額外物理攻擊<>傷害並將其擊退。<#ffb900ff>4秒<>內再次施放可追擊目標並在空中連續踢擊，造成<#ff9028ff>330–500 + 132%額外物理攻擊<>傷害。通過實戰驗證多段觸發前，傷害以一次引擎事件結算。",
    },
    "ko": {
        "name": "초우",
        "passive": "일섬",
        "skill1": "절권도",
        "skill2": "순보",
        "ult": "용의 길",
        "attack": "기본 공격은 물리 공격력의 100%에 해당하는 물리 피해를 줍니다. TFM2 월드 거리 약 42,000만큼 이동하면 다음 기본 공격이 180% 물리 공격력 피해를 주고 1초 동안 80% 둔화시키며, 2초 동안 물리 방어력을 10 감소시킵니다(최대 2중첩). 이동 거리는 TFM2 맵 규모에 맞춘 근삿값입니다.",
        "passive_desc": "<#5ad0e6ff>일섬<>: TFM2 월드 거리 약 <#ffb900ff>42,000<>만큼 이동하면 다음 기본 공격이 <#ff9028ff>물리 공격력의 180%<> 물리 피해를 주고 <#ffb900ff>1초<> 동안 <#ffb900ff>80%<> 둔화시키며, <#ffb900ff>2초<> 동안 물리 방어력을 <#ffb900ff>10<> 감소시킵니다(최대 <#ffb900ff>2<>중첩). 이동 거리는 TFM2 맵 규모에 맞춰 조정됩니다.",
        "skill": "<#ff9028ff>절권도<>: 최대 <#ffb900ff>3<>회 시전합니다. 올려차기, 돌진 어퍼컷, 도약 후 내려찍기 순서이며 마지막 타격은 적을 공중에 띄웁니다. 각 타격은 <#ff9028ff>100–165 + 총 물리 공격력의 45%<> 피해를 줍니다. 처음 두 시전은 기술을 초기화하며 세 번째 시전 후 재사용 대기시간이 시작됩니다.",
        "skill2_desc": "<#5ad0e6ff>순보<>: 지정 방향으로 돌진하며 이동 중 <#ffb900ff>제어 면역<>을 얻습니다. 도착 시 <#5bd18aff>200–300 + 추가 물리 공격력의 100%<>만큼 보호막을 얻고, <#ffb900ff>2.5초<> 동안 물리 관통력 <#ffb900ff>15<>를 얻습니다.",
        "ult_desc": "<#e64a6aff>용의 길<>: 적 영웅을 돌려차기해 <#ff9028ff>275–420 + 추가 물리 공격력의 110%<> 피해를 주고 밀쳐냅니다. <#ffb900ff>4초<> 안에 재사용하면 대상을 추격해 공중에서 연속 발차기를 퍼부으며 <#ff9028ff>330–500 + 추가 물리 공격력의 132%<> 피해를 줍니다. 다단 타격 발동을 실전 검증하기 전까지 피해는 엔진 이벤트 1회로 처리합니다.",
    },
    "ja": {
        "name": "チョウ",
        "passive": "迅速",
        "skill1": "截拳道",
        "skill2": "瞬歩",
        "ult": "龍の道",
        "attack": "通常攻撃は物理攻撃力の100%の物理ダメージを与える。TFM2ワールド距離で約42,000移動すると、次の通常攻撃が物理攻撃力の180%となり、1秒間80%減速させ、2秒間物理防御を10低下させる（最大2スタック）。移動距離はTFM2のマップ規模に合わせた近似値。",
        "passive_desc": "<#5ad0e6ff>迅速<>：TFM2ワールド距離で約<#ffb900ff>42,000<>移動すると、次の通常攻撃が<#ff9028ff>物理攻撃力の180%<>の物理ダメージを与え、<#ffb900ff>1秒<>間<#ffb900ff>80%<>減速させ、<#ffb900ff>2秒<>間物理防御を<#ffb900ff>10<>低下させる（最大<#ffb900ff>2<>スタック）。距離はTFM2のマップ規模に合わせて調整。",
        "skill": "<#ff9028ff>截拳道<>：最大<#ffb900ff>3<>回発動。上昇蹴り、突進アッパー、跳び上がって叩きつける蹴りの順に繰り出し、最後の一撃で敵を浮かせる。各攻撃は<#ff9028ff>100–165 + 総物理攻撃力の45%<>ダメージ。最初の2回は再発動でき、3回目でクールダウンが始まる。",
        "skill2_desc": "<#5ad0e6ff>瞬歩<>：指定方向へダッシュし、移動中は<#ffb900ff>行動妨害無効<>を得る。到着時に<#5bd18aff>200–300 + 追加物理攻撃力の100%<>のシールドと、<#ffb900ff>2.5秒<>間物理貫通<#ffb900ff>15<>を得る。",
        "ult_desc": "<#e64a6aff>龍の道<>：敵ヒーローに回し蹴りを放ち、<#ff9028ff>275–420 + 追加物理攻撃力の110%<>ダメージを与えて吹き飛ばす。<#ffb900ff>4秒<>以内に再発動すると対象を追い、空中で連続蹴りを浴びせて<#ff9028ff>330–500 + 追加物理攻撃力の132%<>ダメージを与える。多段ヒットの発動確認までは、ダメージ処理をエンジンイベント1回にまとめる。",
    },
}

# TFM2 localization fields map action keys to the short skill label and prose.
for lang, fields in STRINGS.items():
    p = ROOT / "text" / "champion.i18n"
    doc = json.loads(p.read_text())
    desc = doc[lang]["description"]
    names = doc[lang]["skill_name"]
    desc["mlbb_chou"] = {
        "name": fields["name"],
        "attack": fields["attack"],
        "passive": fields["passive_desc"],
        "skill": fields["skill"],
        "skill2": fields["skill2_desc"],
        "ult": fields["ult_desc"],
    }
    names["mlbb_chou"] = {
        "passive": fields["passive"],
        "skill1": fields["skill1"],
        "skill2": fields["skill2"],
        "ult": fields["ult"],
    }
    p.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n")

style_path = ROOT / "style" / "champion_view.champion_view"
style = json.loads(style_path.read_text())
style["entries"]["mlbb_chou"] = {
    "face": {"x": 0, "y": -38},
    "center": {"x": 0, "y": -14},
    "banpick_center": {"x": 0, "y": -10},
}
style_path.write_text(json.dumps(style, ensure_ascii=False, indent=2) + "\n")

# Every custom sound event and its clip need a remapping entry.
override_path = ROOT / "mod.override_info"
override = json.loads(override_path.read_text())
for event in (
    "mlbb_chou_voice_skill1_a",
    "mlbb_chou_voice_skill1_b",
    "mlbb_chou_voice_skill2",
    "mlbb_chou_voice_ult",
):
    clip = "mlbb_heroes_clip_" + event
    for asset in (event, clip):
        override[f"asset/base/sound/sfx/{asset}"] = {
            "remapping": f"asset/mlbb_heroes/sound/sfx/{asset}",
            "type": "override",
        }
override_path.write_text(json.dumps(override, ensure_ascii=False, indent=2) + "\n")
print("Merged Chou localization, selection offsets, and eight audio remappings.")
