#!/usr/bin/env python3
"""Build the Uranus asset/data integration from reviewed source copies.

The original submitted art/audio and approved review extractions remain untouched.
Runtime VFX copies are packed with transparent padding; only the scale and alpha
changes explicitly requested after the user's TFM2 screenshot are applied. No
recoloring or checkerboard keying is performed.
"""
from __future__ import annotations
import json
import shutil
import wave
from array import array
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT.parent
ASSET_REVIEW = WORK / "uranus_asset_review"
REF = WORK / "uranus_reference_assets"


def write_json(path: Path, obj: object) -> None:
    path.write_text(json.dumps(obj, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def copy_vfx(
    group: str,
    name: str,
    tag: str,
    duration: float,
    cols: int,
    *,
    state_tags: list[str] | None = None,
    scale: float = 1.0,
    opacity: float = 1.0,
    pad_bottom: int = 0,
) -> None:
    src_dir = ASSET_REVIEW / "frames" / group
    files = sorted(src_dir.glob("frame_*.png"))
    if not files:
        raise FileNotFoundError(src_dir)
    frames = []
    for path in files:
        im = Image.open(path).convert("RGBA")
        if scale != 1.0:
            size = (max(1, round(im.width * scale)), max(1, round(im.height * scale)))
            im = im.resize(size, Image.Resampling.LANCZOS)
        if pad_bottom > 0:
            # Add only transparent canvas below the intact art. This shifts a
            # bottom-anchored mark upward when TFM2 centers the effect on its target.
            padded = Image.new("RGBA", (im.width, im.height + pad_bottom), (0, 0, 0, 0))
            padded.alpha_composite(im, (0, 0))
            im = padded
        if opacity != 1.0:
            alpha = im.getchannel("A").point(lambda value: round(value * opacity))
            im.putalpha(alpha)
        frames.append(im)
    cell_w = max(im.width for im in frames)
    cell_h = max(im.height for im in frames)
    rows = (len(frames) + cols - 1) // cols
    sheet = Image.new("RGBA", (cell_w * cols, cell_h * rows), (0, 0, 0, 0))
    records = []
    for i, im in enumerate(frames):
        col, row = i % cols, i // cols
        # Preserve every extracted pixel; only center-pad to a common animation cell.
        ox = col * cell_w + (cell_w - im.width) // 2
        oy = row * cell_h + (cell_h - im.height) // 2
        sheet.paste(im, (ox, oy))
        records.append({"duration": duration, "data": {"x": col * cell_w, "y": row * cell_h, "w": cell_w, "h": cell_h}})
    sheet.save(ROOT / "effects" / f"{name}#sheet.png", optimize=True)
    if state_tags:
        anims = {state_tags[i]: {"frames": [records[i]]} for i in range(len(records))}
    else:
        anims = {tag: {"frames": records}}
    write_json(ROOT / "effects" / f"{name}#anim.fanim", {"anims": anims})


def copy_character_and_icons() -> None:
    src = ASSET_REVIEW / "tfm2_character_candidate"
    shutil.copy2(src / "uranus#sheet.png", ROOT / "sprite" / "uranus#sheet.png")
    shutil.copy2(src / "uranus#anim.fanim", ROOT / "sprite" / "uranus#anim.fanim")
    icons = {
        "uranus_passive": "uranus_passive_radiance.png",
        "uranus_skill1": "uranus_skill1_ionic_edge.png",
        "uranus_skill2": "uranus_skill2_transcendent_ward.png",
        "uranus_ult": "uranus_ult_consecration.png",
    }
    for dest, source in icons.items():
        shutil.copy2(REF / "icons" / source, ROOT / "icons" / f"{dest}.png")


def copy_audio() -> None:
    src = REF / "audio_wav_22050_mono"
    routes = {
        "skill1": "Uranus.skill.wav",
        "ult": "Uranus.ult.wav",
        "death01": "Uranus.death01.wav",
        "death02": "Uranus.death02.wav",
        "death03": "Uranus.death03.wav",
    }
    volumes = {"skill1": 0.84, "ult": 0.88, "death01": 0.82, "death02": 0.82, "death03": 0.82}
    for key, filename in routes.items():
        event = f"mlbb_uranus_voice_{key}"
        clip = f"mlbb_uranus_sfx_vo_{key}"
        target = ROOT / "sound" / "sfx" / f"{clip}.wav"
        source = src / filename
        if key != "death03":
            shutil.copy2(source, target)
        else:
            # The converted source reaches positive full scale; attenuate the
            # review copy by 1 dB, retaining PCM16/22.05kHz/mono exactly.
            with wave.open(str(source), "rb") as inp:
                params = inp.getparams()
                raw = inp.readframes(inp.getnframes())
            samples = array("h")
            samples.frombytes(raw)
            if samples.itemsize != 2:
                raise ValueError("expected PCM16 source")
            for i, sample in enumerate(samples):
                samples[i] = max(-32768, min(32767, round(sample * 0.891250938)))
            with wave.open(str(target), "wb") as out:
                out.setparams(params)
                out.writeframes(samples.tobytes())
        write_json(ROOT / "sound" / "sfx" / f"{event}.sound_info", {
            "plays": [{"delay": 0.0, "clip": clip, "volume": volumes[key]}]
        })


def data_champion() -> dict:
    def native(ref: str) -> dict:
        return {"type": "Native", "effect_ref": f"mlbb_heroes:{ref}"}
    def apply_projectile(name: str, tick: int, radius: int) -> dict:
        return {
            "type": "ApplyInProjectile", "name": name, "follow_caster": True,
            "tick": tick, "shape": {"Circle": {"radius": radius}},
            "applied_target": "Enemy",
            "applied_effects": [{"effect": native("uranus_skill1_hit"), "casting_type": "Targeting"}],
        }
    def s1_cast(include_voice: bool) -> dict:
        effects = []
        if include_voice:
            effects.extend([
                native("audio_voice_uranus_skill1"),
                {"type": "AddCasterBuff", "only_to_enemy": False,
                 "buff_state": {"name": "mlbb_heroes_uranus_s1_voice_gate", "duration": {"Time": {"tick": 480}}}},
            ])
        effects.extend([
            {"type": "CasterViewEffect", "name": "uranus_skill1_orbit"},
            apply_projectile("uranus_ionic_edge_wave_near", 2, 42000),
            apply_projectile("uranus_ionic_edge_wave_far", 13, 52000),
        ])
        return {"type": "Combine", "effects": effects}
    arrival = {
        "type": "RangeEffect", "shape": {"Circle": {"radius": 40000}},
        "target": "Enemy", "apply_type": "AroundCaster",
        "effects": [native("uranus_skill2_arrival_hit")],
    }
    path_pulse = lambda tick: {
        "type": "ApplyInProjectile", "name": f"uranus_ward_path_{tick}",
        "follow_caster": True, "tick": tick,
        "shape": {"Circle": {"radius": 32000}}, "applied_target": "Enemy",
        "applied_effects": [{"effect": native("uranus_skill2_path_hit"), "casting_type": "Targeting"}],
    }
    return {
        "id": "mlbb_uranus", "category": "Melee", "tags": ["AP", "Melee", "Tank", "Shield", "CC"],
        "sprite": "asset/mlbb_heroes/sprite/uranus", "anim_prefix": "",
        "skill_icons": ["asset/mlbb_heroes/icons/uranus_skill1", "asset/mlbb_heroes/icons/uranus_skill2", "asset/mlbb_heroes/icons/uranus_ult"],
        "stat": {"attack": 54, "magic_power": 25, "hp": 800, "defence": 38, "magic_resistance": 30, "move_speed": 1040, "hp_regen": 9, "stack": 0, "crit_chance": 0},
        "growth": {"attack": 4, "magic_power": 5, "hp": 105, "defence": 4, "magic_resistance": 3, "move_speed": 0, "hp_regen": 0, "stack": 0, "crit_chance": 0},
        "attack": {
            "action_name": "attack", "description": "#asset/base/text/champion?description.mlbb_uranus.attack",
            "duration": 20, "cooltime": 54, "start_timing": 5, "cancelable": True,
            "range": 14000, "casting_type": "Targeting", "casting_target": "Enemy",
            "attack_type": "BaseAttack", "effect": native("uranus_basic_attack"),
        },
        "skill": {
            "action_name": "skill", "description": "#asset/base/text/champion?description.mlbb_uranus.skill",
            "duration": 20, "cooltime": 240, "start_timing": 2, "cancelable": False,
            "range": 52000, "casting_type": "Targeting", "casting_target": "EnemyWithoutTower",
            "attack_type": "Skill", "can_use_with_move": True,
            "effect": {"type": "Combine", "effects": [
                native("uranus_skill1_ai_hint"),
                {"type": "SwitchByBuff", "buff_name": "mlbb_heroes_uranus_s1_voice_gate",
                 "effect_none": s1_cast(True), "effect_buff": s1_cast(False)},
            ]},
        },
        "skill2": {
            "action_name": "skill2", "description": "#asset/base/text/champion?description.mlbb_uranus.skill2",
            "duration": 24, "cooltime": 600, "start_timing": 0, "cancelable": False,
            "range": 90000, "casting_type": "Position", "casting_target": "Enemy",
            "attack_type": "Skill", "can_use_with_move": True,
            "effect": {"type": "Combine", "effects": [
                native("uranus_skill2_start"),
                {"type": "MoveTo", "speed": 6000, "range": 90000, "end_effects": [arrival]},
                path_pulse(2), path_pulse(6), path_pulse(10), path_pulse(14),
            ]},
        },
        "ult": {
            "action_name": "ult", "description": "#asset/base/text/champion?description.mlbb_uranus.ult",
            "duration": 24, "cooltime": 2520, "start_timing": 0, "cancelable": False,
            "range": 95000, "casting_type": "Targeting", "casting_target": "EnemyWithoutTower",
            "attack_type": "Skill", "can_use_with_move": True,
            "effect": {"type": "Combine", "effects": [native("audio_voice_uranus_ult"), native("uranus_ultimate")]}, 
        },
        "passive": {"passive_ref": "mlbb_heroes:uranus_radiance", "params": {}},
        "view_effects": [
            {"type": "Animation", "name": "uranus_skill1_orbit", "anim": "asset/mlbb_heroes/effects/uranus_skill1_orbit", "tag": "orbit", "z": 2, "is_follow": True},
            {"type": "Animation", "name": "uranus_skill2_dash_trail", "anim": "asset/mlbb_heroes/effects/uranus_skill2_dash_trail", "tag": "trail", "z": 2, "is_follow": False},
            {"type": "Animation", "name": "uranus_skill2_shield_burst", "anim": "asset/mlbb_heroes/effects/uranus_skill2_burst", "tag": "burst", "z": 2, "is_follow": False},
            {"type": "Animation", "name": "uranus_ultimate_flare", "anim": "asset/mlbb_heroes/effects/uranus_ultimate_effect", "tag": "flare", "z": 2, "is_follow": True},
        ],
        "view_buffs": [
            *[{"type": "Animated", "name": name, "anim": "asset/mlbb_heroes/effects/uranus_skill1_marks", "tag": f"mark_{i+1}", "z": 3} for i, name in enumerate([
                "mlbb_heroes_uranus_ionic_edge_mark_1_2", "mlbb_heroes_uranus_ionic_edge_mark_3_4", "mlbb_heroes_uranus_ionic_edge_mark_5_6", "mlbb_heroes_uranus_ionic_edge_mark_7_8"])],
            {"type": "Animated", "name": "mlbb_heroes_uranus_s2_shield", "anim": "asset/mlbb_heroes/effects/uranus_skill2_shield", "tag": "loop", "z": 2},
            {"type": "Animated", "name": "mlbb_heroes_uranus_ult", "anim": "asset/mlbb_heroes/effects/uranus_radiance_aura", "tag": "loop", "z": 2},
        ],
    }


def localizations() -> dict:
    # All values include the pack's existing inline color/icon markup.
    rows = {
        "en": {
            "name": "Uranus",
            "attack": "Basic attacks swipe with Uranus’s arm for <i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>100% AD<> <#ff9028ff>physical damage<>. Basic attacks do <#ffb900ff>not<> apply Ionic Edge marks.",
            "passive": "<#5ad0e6ff>Radiance<>: Taking damage grants <#ffb900ff>1 stack<> (up to <#ffb900ff>20<>, lasting <#ffb900ff>10s<> each). Every <#ffb900ff>0.8s<>, Uranus heals <#5bd18aff>48–224 HP at 20 stacks<>, scaling with level.",
            "skill": "<#5ad0e6ff>Ionic Edge<>: Sends two blades outward and back; each hit deals <#a974ffff>79–131 (+18% Total Magic Power)<> magic damage on hit and slowing by <#ffb900ff>30%<> for <#ffb900ff>2s<>. Each blade can hit a target once. Each cast applies at most <#ffb900ff>1 Ionic Mark per target<> for <#ffb900ff>6s<>; each mark increases later Skill 1 damage by <#ffb900ff>20%<>, up to <#ffb900ff>8 marks<>; Towers and Nexus cannot be marked.",
            "skill2": "<#5ad0e6ff>Transcendent Ward<>: Dashes to the chosen position. Enemies along the path take <#a974ffff>48–120 (+12% Total Magic Power)<> magic damage; enemies near the endpoint take <#a974ffff>96–240 (+48% Total Magic Power)<> and are slowed by <#ffb900ff>25%<>. Gains a <#5bd18aff>300–750 (+200% Total Magic Power)<> shield for <#ffb900ff>4s<>. When it breaks or expires, it bursts for <#a974ffff>156–390 (+78% Total Magic Power)<> magic damage nearby.",
            "ult": "<#e64a6aff>Consecration<>: Clears <#ef5350ff>all crowd control<> (TFM2 version; the source removes slows), restores <#5bd18aff>200–400 HP<>, and grants <#ffb900ff>60% move speed<> that decays over <#ffb900ff>8s<>. Adds <#ffb900ff>5–15 Radiance stacks<>. For <#ffb900ff>8s<>, increases HP Regen and Uranus’s own Transcendent Ward shield by <#ffb900ff>20%<>.",
        },
        "zh-hans": {
            "name": "乌瑞纳斯",
            "attack": "普通攻击以手臂挥击造成 <i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>100% AD<> <#ff9028ff>物理伤害<>。普通攻击<#ffb900ff>不会<>施加离子之刃印记。",
            "passive": "<#5ad0e6ff>光辉<>：受到伤害时获得<#ffb900ff>1层<>，最多<#ffb900ff>20层<>，每层持续<#ffb900ff>10秒<>。每<#ffb900ff>0.8秒<>回复生命；20层时回复<#5bd18aff>48–224生命值<>，随等级成长。",
            "skill": "<#5ad0e6ff>离子之刃<>：释放两枚能量刃，向外飞出后返回；每次命中造成<#a974ffff>79–131（+18%总魔法强度）<>魔法伤害，并使敌人<#ffb900ff>减速30%<>，持续<#ffb900ff>2秒<>。每枚刀刃对同一目标最多命中一次。每次施放对每个目标最多施加<#ffb900ff>1层离子印记<>，持续<#ffb900ff>6秒<>；每层使后续技能1伤害提高<#ffb900ff>20%<>，最多<#ffb900ff>8层<>；防御塔和水晶不会获得印记。",
            "skill2": "<#5ad0e6ff>超然护盾<>：冲向指定位置。路径上的敌人受到<#a974ffff>48–120（+12%总魔法强度）<>魔法伤害；终点附近敌人受到<#a974ffff>96–240（+48%总魔法强度）<>并减速<#ffb900ff>25%<>。获得<#5bd18aff>300–750（+200%总魔法强度）<>护盾，持续<#ffb900ff>4秒<>；护盾破裂或结束时，对附近造成<#a974ffff>156–390（+78%总魔法强度）<>魔法伤害。",
            "ult": "<#e64a6aff>奉献<>：移除<#ef5350ff>所有控制效果<>（TFM2实现；原版仅移除减速），回复<#5bd18aff>200–400生命值<>并获得<#ffb900ff>60%移速<>，持续<#ffb900ff>8秒<>且逐渐衰减。增加<#ffb900ff>5–15层光辉<>。持续<#ffb900ff>8秒<>，生命回复与乌瑞纳斯自身的超然护盾提高<#ffb900ff>20%<>。",
        },
        "zh-hant": {
            "name": "烏瑞納斯",
            "attack": "普通攻擊以手臂揮擊造成 <i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>100% AD<> <#ff9028ff>物理傷害<>。普通攻擊<#ffb900ff>不會<>施加離子之刃印記。",
            "passive": "<#5ad0e6ff>光輝<>：受到傷害時獲得<#ffb900ff>1層<>，最多<#ffb900ff>20層<>，每層持續<#ffb900ff>10秒<>。每<#ffb900ff>0.8秒<>恢復生命；20層時恢復<#5bd18aff>48–224生命值<>，隨等級成長。",
            "skill": "<#5ad0e6ff>離子之刃<>：釋放兩枚能量刃，向外飛出後返回；每次命中造成<#a974ffff>79–131（+18%總魔法強度）<>魔法傷害，並使敵人<#ffb900ff>減速30%<>，持續<#ffb900ff>2秒<>。每枚刀刃對同一目標最多命中一次。每次施放對每個目標最多施加<#ffb900ff>1層離子印記<>，持續<#ffb900ff>6秒<>；每層使後續技能1傷害提高<#ffb900ff>20%<>，最多<#ffb900ff>8層<>；防禦塔與水晶不會獲得印記。",
            "skill2": "<#5ad0e6ff>超然護盾<>：衝向指定位置。路徑上的敵人受到<#a974ffff>48–120（+12%總魔法強度）<>魔法傷害；終點附近敵人受到<#a974ffff>96–240（+48%總魔法強度）<>並減速<#ffb900ff>25%<>。獲得<#5bd18aff>300–750（+200%總魔法強度）<>護盾，持續<#ffb900ff>4秒<>；護盾破裂或結束時，對附近造成<#a974ffff>156–390（+78%總魔法強度）<>魔法傷害。",
            "ult": "<#e64a6aff>奉獻<>：移除<#ef5350ff>所有控制效果<>（TFM2實作；原版僅移除緩速），恢復<#5bd18aff>200–400生命值<>並獲得<#ffb900ff>60%移速<>，持續<#ffb900ff>8秒<>且逐漸衰減。增加<#ffb900ff>5–15層光輝<>。持續<#ffb900ff>8秒<>，生命回復與烏瑞納斯自身的超然護盾提高<#ffb900ff>20%<>。",
        },
        "ko": {
            "name": "우라노스",
            "attack": "기본 공격은 팔을 휘둘러 <i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>공격력의 100%<>에 해당하는 <#ff9028ff>물리 피해<>를 줍니다. 기본 공격은 이온 엣지 표식을<#ffb900ff> 적용하지 않습니다<>.",
            "passive": "<#5ad0e6ff>라디언스<>: 피해를 받으면<#ffb900ff> 1중첩<>을 얻으며 최대<#ffb900ff> 20중첩<>까지 쌓입니다. 각 중첩은<#ffb900ff> 10초<> 지속됩니다. 매<#ffb900ff> 0.8초<>마다 회복하며 20중첩에서<#5bd18aff> 48–224 HP<>를 회복합니다.",
            "skill": "<#5ad0e6ff>이온 엣지<>: 에너지 칼날 두 개를 바깥으로 발사해 되돌립니다. 명중할 때마다<#a974ffff> 79–131(+총 마법력의 18%)<> 마법 피해를 주고<#ffb900ff> 30%<> 둔화시킵니다(<#ffb900ff>2초<>). 각 칼날은 같은 대상을 한 번만 타격합니다. 한 번 시전할 때 대상마다<#ffb900ff> 이온 표식 1개<>까지만 부여하며, 표식은<#ffb900ff> 6초<> 동안 지속됩니다. 표식 하나당 이후 스킬 1 피해가<#ffb900ff> 20%<> 증가하며 최대<#ffb900ff> 8중첩<>입니다. 포탑과 넥서스에는 표식을 남기지 않습니다.",
            "skill2": "<#5ad0e6ff>초월의 보호<>: 지정 위치로 돌진합니다. 경로상의 적은<#a974ffff> 48–120(+총 마법력의 12%)<> 마법 피해를 받고, 도착 지점 근처 적은<#a974ffff> 96–240(+총 마법력의 48%)<> 피해와<#ffb900ff> 25%<> 둔화를 받습니다.<#5bd18aff> 300–750(+총 마법력의 200%)<> 보호막을<#ffb900ff> 4초<> 동안 얻으며, 보호막이 깨지거나 끝나면 주변에<#a974ffff> 156–390(+총 마법력의 78%)<> 마법 피해를 줍니다.",
            "ult": "<#e64a6aff>컨세크레이션<>: <#ef5350ff>모든 군중 제어<>를 해제합니다(TFM2 구현; 원본은 둔화만 해제).<#5bd18aff> 200–400 HP<>를 회복하고<#ffb900ff> 60%<> 이동 속도를 얻어<#ffb900ff> 8초<> 동안 감소합니다. 라디언스<#ffb900ff> 5–15중첩<>을 얻습니다. 8초 동안 HP 회복과 우라노스 자신의 초월 보호막이<#ffb900ff> 20%<> 증가합니다.",
        },
        "ja": {
            "name": "ウラノス",
            "attack": "通常攻撃は腕を振り、<i#asset/base/ui/banpick/champion_stat_icon:ad_0><#ff9028ff>攻撃力の100%<>に相当する<#ff9028ff>物理ダメージ<>を与えます。通常攻撃ではイオンエッジの印を<#ffb900ff>付与しません<>。",
            "passive": "<#5ad0e6ff>ラディアンス<>：ダメージを受けると<#ffb900ff>1スタック<>獲得（最大<#ffb900ff>20<>、各<#ffb900ff>10秒<>持続）。<#ffb900ff>0.8秒<>ごとに回復し、20スタック時は<#5bd18aff>48～224 HP<>を回復します。",
            "skill": "<#5ad0e6ff>イオンエッジ<>：2枚のエネルギーブレードを外側へ放って戻し、命中ごとに<#a974ffff>79～131（総魔法力の18%加算）<>の魔法ダメージを与え、<#ffb900ff>30%<>のスロウを<#ffb900ff>2秒<>付与します。各刃は同じ対象に1回まで命中。1回の発動につき対象ごとに<#ffb900ff>イオンの印を1つ<>付与し、<#ffb900ff>6秒<>持続します。印1つにつき以降のスキル1ダメージが<#ffb900ff>20%<>増加し、最大<#ffb900ff>8スタック<>。タワーとネクサスには印を付与しません。",
            "skill2": "<#5ad0e6ff>超越の守護<>：指定地点へ突進します。経路上の敵に<#a974ffff>48～120（総魔法力の12%加算）<>、到着地点付近の敵に<#a974ffff>96～240（総魔法力の48%加算）<>の魔法ダメージを与え、<#ffb900ff>25%<>のスロウを付与。<#5bd18aff>300～750（総魔法力の200%加算）<>のシールドを<#ffb900ff>4秒<>獲得し、破壊または終了時に周囲へ<#a974ffff>156～390（総魔法力の78%加算）<>の魔法ダメージを与えます。",
            "ult": "<#e64a6aff>コンセクレーション<>：<#ef5350ff>すべての行動妨害<>を解除します（TFM2版。原作ではスロウのみ解除）。<#5bd18aff>200～400 HP<>を回復し、<#ffb900ff>60%<>の移動速度を得て<#ffb900ff>8秒<>かけて減衰します。ラディアンスを<#ffb900ff>5～15スタック<>獲得。8秒間、HP回復とウラノス自身の超越の守護シールドが<#ffb900ff>20%<>増加します。",
        },
    }
    skill_names = {
        "en": ("Radiance", "Ionic Edge", "Transcendent Ward", "Consecration"),
        "zh-hans": ("光辉", "离子之刃", "超然护盾", "奉献"),
        "zh-hant": ("光輝", "離子之刃", "超然護盾", "奉獻"),
        "ko": ("라디언스", "이온 엣지", "초월의 보호", "컨세크레이션"),
        "ja": ("ラディアンス", "イオンエッジ", "超越の守護", "コンセクレーション"),
    }
    return {locale: {
        "name": data["name"],
        "attack": data["attack"],
        "passive": data["passive"],
        "skill": data["skill"],
        "skill2": data["skill2"],
        "ult": data["ult"],
        "skill_names": skill_names[locale],
    } for locale, data in rows.items()}


def main() -> None:
    copy_character_and_icons()
    # Runtime-only VFX sizes. Skill 1 orbit matches Estes Domain's 128px frame;
    # the Ward burst is brought to the same frame size. The aura is body-sized,
    # and marks are slightly reduced again while staying top-of-head anchored.
    # Original source/review frames remain unchanged; no keying or recoloring.
    copy_vfx("skill1_orbit", "uranus_skill1_orbit", "orbit", 0.04, 8, scale=0.40)
    copy_vfx("dash_trail", "uranus_skill2_dash_trail", "trail", 0.04, 4, scale=0.15, opacity=0.25)
    copy_vfx("skill2_burst", "uranus_skill2_burst", "burst", 0.05, 4, scale=0.28)
    copy_vfx("skill2_shield", "uranus_skill2_shield", "loop", 0.10, 4, scale=0.13)
    copy_vfx("ultimate_effect", "uranus_ultimate_effect", "flare", 0.06, 4, scale=0.18)
    copy_vfx("radiance_aura", "uranus_radiance_aura", "loop", 0.06, 4, scale=0.18)
    copy_vfx("skill1_marks", "uranus_skill1_marks", "", 0.10, 4,
             state_tags=["mark_1", "mark_2", "mark_3", "mark_4"], scale=0.12, pad_bottom=77)
    write_json(ROOT / "champion" / "mlbb_uranus.data_champion", data_champion())

    loc = json.loads((ROOT / "text" / "champion.i18n").read_text(encoding="utf-8"))
    localized = localizations()
    for locale, entry in localized.items():
        descriptions = loc[locale]["description"]
        descriptions["mlbb_uranus"] = {key: entry[key] for key in ("name", "attack", "passive", "skill", "skill2", "ult")}
        passive, skill1, skill2, ult = entry["skill_names"]
        loc[locale]["skill_name"]["mlbb_uranus"] = {"passive": passive, "skill1": skill1, "skill2": skill2, "ult": ult}
    write_json(ROOT / "text" / "champion.i18n", loc)

    style_path = ROOT / "style" / "champion_view.champion_view"
    style = json.loads(style_path.read_text(encoding="utf-8"))
    style["entries"]["mlbb_uranus"] = {
        "face": {"x": -2, "y": -38},
        "center": {"x": 0, "y": -14},
        "banpick_center": {"x": 0, "y": -10},
    }
    write_json(style_path, style)

    # Match the five Original voice clips to the shared voice cue router.
    copy_audio()
    override_path = ROOT / "mod.override_info"
    overrides = json.loads(override_path.read_text(encoding="utf-8"))
    for suffix in ("skill1", "ult", "death01", "death02", "death03"):
        for alias in (f"mlbb_uranus_voice_{suffix}", f"mlbb_uranus_sfx_vo_{suffix}"):
            overrides[f"asset/base/sound/sfx/{alias}"] = {
                "remapping": f"asset/mlbb_heroes/sound/sfx/{alias}", "type": "override"
            }
    write_json(override_path, overrides)

    cargo_path = ROOT / "Cargo.toml"
    cargo = cargo_path.read_text(encoding="utf-8")
    cargo = cargo.replace('version = "1.1.25"', 'version = "1.1.26"', 1)
    cargo = cargo.replace('description = "Teamfight Manager 2 — MLBB Heroes Native Mod (Aldous, Fanny, Lancelot, Estes, Gusion, and Karrie)"',
                          'description = "Teamfight Manager 2 — MLBB Heroes Native Mod (Aldous, Fanny, Lancelot, Estes, Gusion, Karrie, and Uranus)"', 1)
    cargo_path.write_text(cargo, encoding="utf-8")
    info_path = ROOT / "mod.mod_info"
    info = json.loads(info_path.read_text(encoding="utf-8"))
    info["version"] = "1.1.26"
    info["last_updated"] = "2026-10-25"
    info["description"] = ("MLBB Heroes Pack for Teamfight Manager 2: native Aldous, Fanny, Lancelot, Estes, Gusion, Karrie, and Uranus. Uranus includes Radiance, eight-step Ionic Edge marks, Transcendent Ward, and Consecration; Original voice cues use shared anti-overlap routing. Runtime gameplay and VFX calibration pending in-match verification.")
    write_json(info_path, info)

if __name__ == "__main__":
    main()
