"""Compare release repair to the delivered working 1.1.156 foundation."""
from pathlib import Path
from PIL import Image
import json, zipfile, hashlib

R = Path(__file__).resolve().parents[1]
BASE = R.parent/'hanzo_156_artifacts/MLBB_Heroes_TFM2_v1.1.156_full_source_with_SDK.zip'
with zipfile.ZipFile(BASE) as z:
    preserved = 0
    for n in z.namelist():
        if n.startswith(('champion/', 'champions/', 'effects/', 'sprite/', 'icons/', 'sound/', 'view/', 'override/', 'text/', 'style/')) and n != 'text/champion.i18n':
            assert (R/n).read_bytes() == z.read(n), n
            preserved += 1
        if n.startswith(('source/', 'sdk/mod-api-stable/')):
            assert (R/n).read_bytes() == z.read(n), n
        if n.startswith('sdk/candidate/') and n.endswith('.rs') and n not in {
            'sdk/candidate/granger.rs', 'sdk/candidate/granger_tests.rs',
            'sdk/candidate/hanzo_tests.rs'
        }:
            assert (R/n).read_bytes() == z.read(n), n
    old = z.read('sdk/candidate/granger.rs').decode()
    new = (R/'sdk/candidate/granger.rs').read_text()
    assert new == old.replace('damage: 20 + ad * 30 / 100', 'damage: 28 + ad * 40 / 100').replace('110 + ad * 90 / 100', '130 + ad * 105 / 100')
    previous = json.loads(z.read('text/champion.i18n'))
    for locale in previous.values():
        g = locale['description']['mlbb_granger']
        g['skill'] = g['skill'].replace('20 + 30% AD', '28 + 40% AD')
        g['ult'] = g['ult'].replace('110 + 90% AD', '130 + 105% AD')
    assert json.loads((R/'text/champion.i18n').read_text()) == previous
    assert (R/'mod.override_info').read_bytes() == z.read('mod.override_info')

assert not (R/'sdk/candidate/match_carrier.rs').exists()
assert not (R/'tools/compile_native_previews.py').exists()
for path in (R/'sdk/candidate').glob('*.rs'):
    assert 'match_carrier' not in path.read_text(), path
for name in ['popol_probe_carrier', 'granger_probe', 'hanzo_probe']:
    meta = json.loads((R/f'sprite/{name}#anim.fanim').read_text())
    assert 'mlbb_match_carrier' not in meta['anims']
    sheet = Image.open(R/f'sprite/{name}#sheet.png').convert('RGBA')
    for tag, anim in meta['anims'].items():
        for i, frame in enumerate(anim['frames']):
            b = frame['data']
            alpha = sheet.crop((b['x'], b['y'], b['x']+b['w'], b['y']+b['h'])).getchannel('A')
            assert bool(alpha.getbbox()) == (tag == 'idle' and i == 0), (name, tag, i)
assert len(list((R/'champion').glob('*.data_champion'))) == 25
cecilion = json.loads((R/'sprite/cecilion#anim.fanim').read_text())['anims']['idle']['frames']
result = {
    'public_version': 'v2.6.0', 'internal_version': '1.1.158',
    'foundation': '1.1.156', 'rejected_candidate': '1.1.157',
    'heroes': 25, 'preserved_runtime_asset_data_files': preserved,
    'visual_modules_and_carrier_metadata_restored_exactly': True,
    'hanzo_popol_gameplay_and_ai_identical_to_1156': True,
    'shared_self_animation_cc_helper_removed': True,
    'granger_gameplay_changes_exactly_two_damage_formulas': True,
    'all_original_source_art_and_frozen_sdk_preserved': True,
    'cecilion_idle_frames': len(cecilion),
    'cecilion_idle_seconds': sum(f['duration'] for f in cecilion),
    'animated_hover_previews_fixed': False,
    'native_owner_tested_1158': False,
    'dll_sha256': hashlib.sha256((R/'mlbb_heroes.dll').read_bytes()).hexdigest(),
}
(R/'reports/RELEASE_1158_VERIFICATION.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
