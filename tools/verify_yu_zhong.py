"""Verify the 1.1.170 Yu Zhong art, data and native bindings."""
import hashlib
import json
from pathlib import Path
from PIL import Image

root=Path(__file__).resolve().parents[1]
def read(p): return json.loads(p.read_text())
path=root/'champion/mlbb_yu_zhong.data_champion'
spec=read(path)
assert path.read_bytes()==(root/'sdk/candidate/champion'/path.name).read_bytes()
assert [spec[k]['cooltime'] for k in ('attack','skill','skill2','ult')]==[60,420,600,2760]
assert len(list((root/'champion').glob('*.data_champion')))==27
art=root/'source/yu_zhong'
assert len(list((art/'received').glob('*.png')))==35
for name,asset in read(art/'compiled_assets.json').items():
    assert hashlib.sha256((art/'received'/f'{name}.png').read_bytes()).hexdigest()==asset['original_sha256']
assert spec['view_buffs'] == [], "A retained actor pose can survive the death snapshot"
body_prefix = 'mlbb_yu_fx_body_'
body_sample_ticks = 4
body_count = 0
known=set()
for row in spec['view_buffs']+spec['view_effects']:
    assert row['name'] not in known
    known.add(row['name'])
    asset=root/row['anim'].removeprefix('asset/mlbb_heroes/')
    animation=read(Path(str(asset)+'#anim.fanim'))['anims'][row['tag']]['frames']
    with Image.open(Path(str(asset)+'#sheet.png')) as im:
        body = row['name'].startswith(body_prefix)
        if body:
            body_count += 1
            assert row['is_follow'] is False
            assert len(animation) == 2
            assert abs(animation[0]['duration'] * 60 - body_sample_ticks) < 1e-6
            assert abs(animation[1]['duration'] * 60 - 1) < 1e-6
        for index, f in enumerate(animation):
            d=f['data']
            assert 0<=d['x']<d['x']+d['w']<=im.width and 0<=d['y']<d['y']+d['h']<=im.height and f['duration']>0
            if body:
                bbox = im.crop((d['x'], d['y'], d['x']+d['w'], d['y']+d['h'])).getbbox()
                if index == 1:
                    assert bbox is None
                else:
                    assert bbox
                    if row['name'].startswith(body_prefix+'human'):
                        assert bbox[3]-bbox[1] <= 48
assert body_count > 0
assert {n for n in known if n.startswith('mlbb_yu_fx_mark_')} == {
    f'mlbb_yu_fx_mark_{n}' for n in range(1, 4)
}
carrier = root/'sprite/yu_zhong_carrier'
carrier_anim = read(Path(str(carrier)+'#anim.fanim'))
dead_frames = carrier_anim['anims']['dead']['frames']
assert len(dead_frames) == 1 and abs(dead_frames[0]['duration'] - 0.12) < 1e-9
assert carrier_anim['anims']['idle']['frames'][1]['duration'] == 86400
dead = dead_frames[0]['data']
with Image.open(Path(str(carrier)+'#sheet.png')) as image:
    assert not image.crop((dead['x'], dead['y'], dead['x']+dead['w'], dead['y']+dead['h'])).getbbox()
for suffix in ['passive','base_attack','skill','skill2','ult']:
    with Image.open(root/'icons'/f'mlbb_yu_zhong_{suffix}.png') as im:
        assert im.size==(100,100)
        im.verify()
for i in range(9):
    assert (root/'icons'/f'yu_zhong_draft_idle_{i}.svg').read_text().startswith('<svg ')
for locale in read(root/'text/champion.i18n').values():
    assert len(locale['description']['mlbb_yu_zhong'])==6
    assert len(locale['skill_name']['mlbb_yu_zhong'])==4
    assert 'At three marks, the target pulses five times.' in locale['description']['mlbb_yu_zhong']['passive']
print(f'{len(known)} unique Yu Zhong view bindings ({body_count} detached body frames), 5 icons, 9 draft preview frames, 35 original PNGs, 27 heroes.')
