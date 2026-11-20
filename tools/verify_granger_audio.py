"""Check actual shipped audio, provenance, routing and measured shared gates."""
from pathlib import Path
import json,wave,hashlib,math,re,array
R=Path(__file__).resolve().parents[1]
a=json.loads((R/'reports/GRANGER_1147_AUDIO_INTAKE.json').read_text());assert len(a)==19
route=json.loads((R/'mod.override_info').read_text());
from validate_override_info import validate
validate(route)
code=(R/'sdk/candidate/audio.rs').read_text()
source=json.loads((R/'source/granger/audio/download_manifest.json').read_text());source={x['file']:x for x in source if 'error'not in x}
catalog=json.loads((R/'source/granger/audio/default_catalog.json').read_text());catalog={Path(x['mp3_path']).name:x for x in catalog}
for row in a:
 name=row['event'];clip='mlbb_heroes_clip_'+name
 raw=R/'source/granger/audio/original'/row['source_mp3'];assert hashlib.sha256(raw.read_bytes()).hexdigest()==source[raw.name]['sha256']
 assert catalog[raw.name]['skin_name']=='Default'
 p=R/'sound/sfx'/(name+'.wav');b=p.read_bytes();assert hashlib.sha256(b).hexdigest()==row['wav_sha256']
 assert b==(R/'sound/sfx'/(clip+'.wav')).read_bytes()
 with wave.open(str(p))as w:
  assert(w.getnchannels(),w.getsampwidth(),w.getframerate())==(1,2,44100)
  seconds=w.getnframes()/w.getframerate();samples=array.array('h',w.readframes(w.getnframes()));peak=max(abs(n)for n in samples)
  assert 100<peak<32767,(name,peak)
 assert math.ceil(seconds*60)+12==row['lock_ticks']
 assert re.search(r'"'+name+r'"\s*=>\s*'+str(row['lock_ticks'])+r'\b',code)
 event=json.loads((R/'sound/sfx'/(name+'.sound_info')).read_text());assert event['plays'][0]['clip']==clip
 for n in [name,clip]:
  assert route['asset/base/sound/sfx/'+n]=={'remapping':'asset/mlbb_heroes/sound/sfx/'+n,'type':'override'}
 assert f'"granger_{row["cue"]}", "{name}"'in code
assert 'line.starts_with("mlbb_granger_voice")'in code
logic=(R/'sdk/candidate/granger_voice.rs').read_text();assert 'queue_effect'not in logic and 'play_sfx'not in logic
for token in ['ui_find','ui_tree','ui_get','ui_set']:assert token not in logic
print('PASS: 19 original-source/default-set bindings, PCM/peak/hash/duration checks, distinct event/clip routes, guarded path registration, no queue or UI scanner')
