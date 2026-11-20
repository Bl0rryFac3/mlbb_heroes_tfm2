#!/usr/bin/env python3
"""Compile retrieved MLBB UI media. Keep all downloaded originals unchanged."""
from pathlib import Path
from PIL import Image
import json,subprocess,hashlib,math,shutil
root=Path(__file__).resolve().parents[1];src=root/'source/popol_kupa/wiki'
for ident,name in [(9410,'skill'),(9420,'skill2'),(9440,'ult')]:
 im=Image.open(src/f'{ident}.webp').convert('RGBA');assert im.size==(96,96);im.save(root/f'icons/popol_probe_{name}.png')
line='mlbb_popol_voice_intro';wav=root/f'sound/sfx/{line}.wav'
subprocess.run(['ffmpeg','-v','error','-y','-i',str(src/'popol_official_select.mp3'),'-ac','1','-ar','22050','-c:a','pcm_s16le',str(wav)],check=True)
shutil.copyfile(wav,root/f'sound/sfx/mlbb_heroes_clip_{line}.wav')
(root/f'sound/sfx/{line}.sound_info').write_text(json.dumps({'plays':[{'delay':0.0,'clip':'mlbb_heroes_clip_'+line,'volume':0.68}]},indent=2)+'\n')
overrides_path=root/'mod.override_info'
overrides=json.loads(overrides_path.read_text())
for name in [line,'mlbb_heroes_clip_'+line]:
 overrides['asset/base/sound/sfx/'+name]={'remapping':'asset/mlbb_heroes/sound/sfx/'+name,'type':'override'}
overrides_path.write_text(json.dumps(overrides,ensure_ascii=False,indent=1)+'\n')
duration=float(subprocess.check_output(['ffprobe','-v','error','-show_entries','format=duration','-of','csv=p=0',str(wav)]))
report={'icons_source':'https://metammo.com/mobile-legends-guide/heroes/popol-and-kupa','audio_source':json.loads((src/'official_audio_source.json').read_text()),'duration':duration,'gate_ticks':math.ceil(duration*60),'originals':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in src.iterdir() if p.suffix in ('.webp','.mp3')},'remaining_audio':'Default movement/skill/death OGG downloads blocked by Fandom HTTP 402. No synthetic or substituted voices.'}
(root/'reports/POPOL_1142_MEDIA.json').write_text(json.dumps(report,indent=2)+'\n');print('MLBB skill icons and publisher intro compiled; intro gate',report['gate_ticks'],'ticks')
