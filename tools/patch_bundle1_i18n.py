#!/usr/bin/env python3
"""
Bundle 1 i18n patch — move passive content out of the `attack` slot and into
the passive slot, and fill the two gaps (Aldous text, Aldous/Fanny names).

Writes text/champion.i18n in place. Run from build_v2081/.
"""
import json, re, shutil, sys, os

PATH = 'text/champion.i18n'
LOCS = ['en', 'zh-hans', 'zh-hant', 'ko', 'ja', 'fr', 'ru', 'vi']
# NB: the templates below write the '#' themselves - these must NOT carry one.
GOLD = 'ffb900ff'    # numbers / thresholds
CYAN = '5ad0e6ff'    # ability names

# ---------------------------------------------------------------- helpers
def first_sentence(raw):
    """Raw prefix ending at the first sentence break at tag depth 0."""
    depth = 0
    n = len(raw)
    for i, ch in enumerate(raw):
        if ch == '<':
            depth += 1; continue
        if ch == '>':
            depth = max(0, depth - 1); continue
        if depth:
            continue
        if ch in '。！':
            return raw[:i + 1]
        if ch in '.!':
            nxt = raw[i + 1] if i + 1 < n else ''
            if nxt == '' or nxt in ' \t\n':
                return raw[:i + 1]
    return raw

def balanced(s):
    return s.count('<') == s.count('>')

vis = lambda s: re.sub(r'<[^>]*>', '', s)

# ------------------------------------------------- new passive: Aldous
# Mechanics read from sdk/candidate/lib.rs (AldousContractPassive):
#   stacks: +1 / 5s ambient (300 ticks), +10 per kill, +N when S1 connects,
#           capped at ALDOUS_MAX_STACKS = 1000
#   shield: every 3rd basic attack with a 5.0s (300-tick) ICD ->
#           35 + 5*level + 0.45*stacks, for 2.0s (120 ticks)
#   stacks also feed S1 damage: 40 + level_bonus + attack + stacks*5/2
ALDOUS_PASSIVE = {
 'en': ("Contract: Transform: Aldous absorbs soul energy — <#{g}>+1<> stack every "
        "<#{g}>5s<>, <#{g}>+10<> on a kill, and more whenever "
        "<#{c}>Contract: Soul Steal<> connects (cap <#{g}>1,000<>). Every "
        "<#{g}>3rd<> basic attack (<#{g}>5s<> cooldown) turns those stacks into a "
        "<#{g}>2s<> shield worth <#{g}>35<> + <#{g}>5<> per level + <#{g}>0.45<> per "
        "stack. Stacks also empower <#{c}>Contract: Soul Steal<>."),
 'zh-hans': ("契约·变换：奥杜斯吸取灵魂能量——每<#{g}>5秒<>+1层，击杀+10层，"
             "<#{c}>灵魂窃取<>命中时额外获得（上限<#{g}>1000<>层）。"
             "每第<#{g}>3<>次普攻（<#{g}>5秒<>冷却）将层数转化为持续<#{g}>2秒<>的护盾，"
             "数值为<#{g}>35<>+<#{g}>5<>/级+<#{g}>0.45<>/层。层数同时强化<#{c}>灵魂窃取<>。"),
 'zh-hant': ("契約·變換：奧杜斯吸取靈魂能量——每<#{g}>5秒<>+1層，擊殺+10層，"
             "<#{c}>靈魂竊取<>命中時額外獲得（上限<#{g}>1000<>層）。"
             "每第<#{g}>3<>次普攻（<#{g}>5秒<>冷卻）將層數轉化為持續<#{g}>2秒<>的護盾，"
             "數值為<#{g}>35<>+<#{g}>5<>/級+<#{g}>0.45<>/層。層數同時強化<#{c}>靈魂竊取<>。"),
 'ko': ("계약: 변환 — 알두스는 영혼 에너지를 흡수합니다. <#{g}>5초<>마다 "
        "<#{g}>+1<>중첩, 처치 시 <#{g}>+10<>중첩, <#{c}>영혼 강탈<>이 적중할 때 추가 중첩"
        "(최대 <#{g}>1,000<>). <#{g}>3<>번째 기본 공격마다(<#{g}>5초<> 재사용 대기) "
        "이 중첩을 <#{g}>2초<>간 유지되는 보호막으로 전환하며, 수치는 "
        "<#{g}>35<> + 레벨당 <#{g}>5<> + 중첩당 <#{g}>0.45<>입니다. "
        "중첩은 <#{c}>영혼 강탈<>도 강화합니다."),
 'ja': ("契約：変容 — アルダスは魂のエネルギーを吸収する：<#{g}>5秒<>ごとに"
        "<#{g}>+1<>、キルで<#{g}>+10<>、<#{c}>ソウルスティール<>命中時にも獲得"
        "（上限<#{g}>1,000<>）。<#{g}>3<>回目の通常攻撃ごとに（<#{g}>5秒<>クールダウン）"
        "<#{g}>2秒<>間のシールドを得る。数値は<#{g}>35<>+レベル×<#{g}>5<>+スタック×"
        "<#{g}>0.45<>。スタックは<#{c}>ソウルスティール<>も強化する。"),
 'fr': ("Contrat : Transformation — Aldous absorbe l'énergie animale : "
        "<#{g}>+1<> cumul toutes les <#{g}>5 s<>, <#{g}>+10<> par élimination, et "
        "davantage chaque fois que <#{c}>Vol d'âme<> touche (max <#{g}>1 000<>). "
        "Toutes les <#{g}>3e<> attaques de base (<#{g}>5 s<> de recharge), il "
        "convertit ces cumuls en un bouclier de <#{g}>2 s<> valant <#{g}>35<> + "
        "<#{g}>5<> par niveau + <#{g}>0,45<> par cumul. Les cumuls renforcent "
        "aussi <#{c}>Vol d'âme<>."),
 'ru': ("Контракт: Трансформация — Алдус поглощает энергию душ: "
        "<#{g}>+1<> заряд каждые <#{g}>5 с<>, <#{g}>+10<> за убийство и больше при "
        "попадании <#{c}>Кражи души<> (максимум <#{g}>1 000<>). Каждая "
        "<#{g}>3-я<> базовая атака (перезарядка <#{g}>5 с<>) превращает заряды в "
        "щит на <#{g}>2 с<>: <#{g}>35<> + <#{g}>5<> за уровень + <#{g}>0,45<> за "
        "заряд. Заряды также усиливают <#{c}>Кражу души<>."),
 'vi': ("Khế Ước: Biến Đổi — Aldous hấp thụ năng lượng linh hồn: <#{g}>+1<> điểm "
        "mỗi <#{g}>5 giây<>, <#{g}>+10<> khi hạ gục, và thêm khi "
        "<#{c}>Đánh Cắp Linh Hồn<> trúng đích (tối đa <#{g}>1.000<>). Cứ mỗi "
        "<#{g}>3<> đòn đánh thường (hồi <#{g}>5 giây<>), các điểm này hóa thành "
        "khiên trong <#{g}>2 giây<>: <#{g}>35<> + <#{g}>5<> mỗi cấp + <#{g}>0,45<> "
        "mỗi điểm. Điểm số cũng cường hóa <#{c}>Đánh Cắp Linh Hồn<>."),
}
ALDOUS_PASSIVE = {k: v.format(g=GOLD, c=CYAN) for k, v in ALDOUS_PASSIVE.items()}

# ------------------------------------------------- missing skill names
NAMES = {
  'mlbb_aldous': {  # Contract: Transform
    'en': 'Contract: Transform', 'zh-hans': '契约·变换', 'zh-hant': '契約·變換',
    'ko': '계약: 변환', 'ja': '契約：変容', 'fr': 'Contrat : Transformation',
    'ru': 'Контракт: Трансформация', 'vi': 'Khế Ước: Biến Đổi',
  },
  'mlbb_fanny': {   # Air Superiority
    'en': 'Air Superiority', 'zh-hans': '制空权', 'zh-hant': '制空權',
    'ko': '제공권', 'ja': '制空権', 'fr': 'Supériorité aérienne',
    'ru': 'Господство в воздухе', 'vi': 'Ưu Thế Trên Không',
  },
}

# ------------------------------- attack slots carrying passive content
STRIP = ['mlbb_aldous', 'mlbb_chou', 'mlbb_estes', 'mlbb_gusion',
         'mlbb_hanabi', 'mlbb_lancelot', 'mlbb_obsidia']

# Karrie's attack is a single sentence, so the splitter cannot help: drop the
# trailing "and apply 1 Lightwheel Mark" clause per locale.
G = 'ffb900ff'   # templates below already carry the leading '#'
KARRIE = {
 'en':      (' with a single lightwheel and apply <#%s>1 Lightwheel Mark<>.' % G,
             ' with a single lightwheel.'),
 'zh-hans': ('，使用单个光轮并施加<#%s>1层光轮印记<>。' % G, '，使用单个光轮。'),
 'zh-hant': ('，使用單個光輪並施加<#%s>1層光輪印記<>。' % G, '，使用單個光輪。'),
 'ko':      ('를 주고 <#%s>라이트휠 표식 1개<>를 부여합니다.' % G, '를 줍니다.'),
 'ja':      ('を与え、<#%s>ライトホイールの印を1つ<>付与する。' % G, 'を与える。'),
 'fr':      (' avec une seule roue-lumière et appliquent <#%s>1 Marque de roue-lumière<>.' % G,
             ' avec une seule roue-lumière.'),
 'ru':      (' одним светоколесом и накладывают <#%s>1 Метку светоколеса<>.' % G,
             ' одним светоколесом.'),
 'vi':      (' bằng một bánh xe ánh sáng và áp dụng <#%s>1 Dấu Ấn Bánh Xe Ánh Sáng<>.' % G,
             ' bằng một bánh xe ánh sáng.'),
}

# Linter rules (tfm2_reference/lint_mod.py) - validate before writing.
TOOLTIP_MAX = {"zh-hans": 130, "zh-hant": 130, "en": 334, "ja": 147, "ko": 185,
               "fr": 411, "ru": 455, "vi": 381}

def shown_length(s):
    s = re.sub(r"\{\w+\}", "000", s)
    s = re.sub(r"<i#[^>]*>", "*", s)
    return len(re.sub(r"<[^>]*>", "", s))

def check(loc, key, s):
    """Raise if this string would make lint_mod.py complain."""
    opens = len(re.findall(r"<#[0-9a-fA-F]{6,8}>", s))
    closes = s.count("<>")
    assert opens == closes, f"{loc}.{key}: {opens} colour tags vs {closes} '<>'"
    assert balanced(s), f"{loc}.{key}: unbalanced markup"
    lim = TOOLTIP_MAX.get(loc)
    if lim:
        n = shown_length(s)
        assert n <= lim, f"{loc}.{key}: {n} chars shown, over the {lim} limit"
    return shown_length(s)

def main():
    d = json.load(open(PATH, encoding='utf-8'))
    report = []

    # 1. Aldous passive description + the two missing names
    for loc in LOCS:
        text = ALDOUS_PASSIVE[loc]
        n = check(loc, 'description.mlbb_aldous.passive', text)
        print(f"    {loc:<8} aldous passive: {n:>3} chars shown (limit {TOOLTIP_MAX.get(loc,'-')})")
        d[loc]['description']['mlbb_aldous']['passive'] = text
        for champ, table in NAMES.items():
            d[loc]['skill_name'][champ]['passive'] = table[loc]
    report.append("added description.mlbb_aldous.passive x8")
    report.append("added skill_name.mlbb_aldous.passive + skill_name.mlbb_fanny.passive x8")

    # 2. strip passive content out of the attack slots
    for champ in STRIP:
        for loc in LOCS:
            raw = d[loc]['description'][champ]['attack']
            new = first_sentence(raw)
            assert balanced(new), f"{champ}/{loc}: unbalanced markup"
            d[loc]['description'][champ]['attack'] = new
    report.append(f"stripped passive content from attack slots: {', '.join(STRIP)}")

    for loc in LOCS:
        raw = d[loc]['description']['mlbb_karrie']['attack']
        old, new = KARRIE[loc]
        assert old in raw, f"karrie/{loc}: clause not found"
        d[loc]['description']['mlbb_karrie']['attack'] = raw.replace(old, new)
        assert balanced(d[loc]['description']['mlbb_karrie']['attack'])
    report.append("stripped Lightwheel Mark clause from mlbb_karrie attack x8")

    json.dump(d, open(PATH, 'w', encoding='utf-8'), ensure_ascii=False, indent=2)
    open(PATH, 'a', encoding='utf-8').write('\n')

    for line in report:
        print("  " + line)

if __name__ == '__main__':
    if not os.path.exists('text/champion.i18n'):
        sys.exit("run me from build_v2081/")
    shutil.copy2(PATH, 'patch_backups_1108/champion.i18n.before_bundle1')
    print("backup -> patch_backups_1108/champion.i18n.before_bundle1")
    main()
