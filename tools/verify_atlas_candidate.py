"""Check Atlas champion wiring and the compiled runtime sprite rectangles."""
from pathlib import Path
import json
import re
from PIL import Image

root = Path(__file__).resolve().parents[1]
def read(path):
    return json.loads(path.read_text())

atlas = root / 'champion/mlbb_atlas.data_champion'
data = read(atlas)
candidate = root / 'sdk/candidate'
assert atlas.read_bytes() == (candidate / 'champion/mlbb_atlas.data_champion').read_bytes()
assert data['id'] == 'mlbb_atlas' and data['category'] == 'Tank'
assert len(list((root / 'champion').glob('*.data_champion'))) >= 26
assert [data[slot]['cooltime'] for slot in ['attack', 'skill', 'skill2', 'ult']] == [56, 420, 600, 2040]

code = (candidate / 'atlas.rs').read_text()
visual = (candidate / 'atlas_visual.rs').read_text()
refs = set(re.findall(r'mlbb_heroes:atlas(?:_[a-z_]+)?', atlas.read_text()))
assert all(f'"{name}"' in code for name in refs)
assert not re.search(r'CcKindV1::Animation|entity_set_pos|entity_banish|entity_set_hp', code + visual)
assert 'atlas::register(&mut module)' in (candidate / 'lib.rs').read_text()
bindings = {entry['name']:entry for key in ['view_buffs','view_effects'] for entry in data[key]}
assert len(bindings) == len(data['view_buffs']) + len(data['view_effects'])
required_fx = ['annihilate_pulse','freeze_impact','frost_exposure','ejection_flash',
               'reunion_slow','reunion_stun','sentry_wake','link_head','slam_impact']
assert all('mlbb_atlas_fx_'+name in bindings for name in required_fx)
assert 'mlbb_atlas_frigid_breath' in bindings
for facing in ['', '_left']:
    assert 'mlbb_atlas_s1_ice'+facing in bindings
    for clip in ['suit_follow','suit_s1']:
        assert all(f'mlbb_atlas_{clip}{facing}_{i}' in bindings for i in range(9))
    for clip in ['idle','run','attack','skill','skill2','ult','hit','rejoin','ult_slam',
                 'pilot_idle','pilot_move','pilot_attack','pilot_s1','pilot_hit']:
        assert f'mlbb_atlas_pose_{clip}{facing}' in bindings
for angle in range(12):
    assert all(f'mlbb_atlas_chain_{angle}_{frame}' in bindings for frame in range(9))
for name in data['skill_icons']:
    assert (root / (name.removeprefix('asset/mlbb_heroes/')+'.png')).is_file()

seen = set()
for binding in bindings.values():
    asset = root / binding['anim'].removeprefix('asset/mlbb_heroes/')
    tag = binding['tag']
    if (asset, tag) in seen:
        continue
    seen.add((asset,tag))
    anim = read(Path(str(asset)+'#anim.fanim'))['anims'][tag]
    with Image.open(Path(str(asset)+'#sheet.png')) as sheet:
        alpha = sheet.getchannel('A')
        for frame in anim['frames']:
            d = frame['data']
            assert 0 <= d['x'] < d['x']+d['w'] <= sheet.width
            assert 0 <= d['y'] < d['y']+d['h'] <= sheet.height
            assert frame['duration'] > 0
            if asset.name in {'atlas_rig','atlas_rig_left'} or asset.name.startswith(('atlas_pilot_','atlas_suit_')):
                bounds = alpha.crop((d['x'],d['y'],d['x']+d['w'],d['y']+d['h'])).getbbox()
                assert not bounds or bounds[3]-bounds[1] <= 48, (asset,tag)
carrier = read(root / 'sprite/atlas_carrier#anim.fanim')
with Image.open(root / 'sprite/atlas_carrier#sheet.png') as sheet:
    alpha = sheet.getchannel('A')
    assert alpha.crop((0,0,96,88)).getbbox()
    assert not alpha.crop((96,0,192,88)).getbbox()
assert len(carrier['anims']['idle']['frames']) == 2
locales = read(root/'text/champion.i18n')
assert len(locales) == 8
for locale in locales.values():
    assert len(locale['description']['mlbb_atlas']) == 6
    assert len(locale['skill_name']['mlbb_atlas']) == 4
print(f'Atlas reference integrity: {len(bindings)} view bindings, 8 locales, 3 icons, all body poses within 48 px.')
