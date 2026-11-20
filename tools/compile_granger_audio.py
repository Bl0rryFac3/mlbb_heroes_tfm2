"""Compile the default Granger voice clips recovered through mlbb.tools' public player.
Source wiki supplies semantic event labels; preserve originals/provenance in source.
"""
from pathlib import Path
import subprocess,json,hashlib,math,re,shutil
R=Path(__file__).resolve().parents[1];S=R/'source/granger/audio';S.mkdir(parents=True,exist_ok=True)
M={'select':'show_vo_1791','move01':'vo_1791_012','move02':'vo_1791_018','move03':'vo_1791_009','move04':'vo_1791_017','move05':'vo_1791_021','move06':'vo_1791_014','move08':'vo_1791_005','move09':'vo_1791_010','move10':'vo_1791_011','skill01':'vo_1791_030','ult01':'vo_1791_024','ult02':'vo_1791_028','death01':'vo_1791_015','respawn01':'vo_1791_033','respawn02':'vo_1791_001','kill01':'vo_1791_008','kill02':'vo_1791_016','kill03':'vo_1791_006'}
overrides=json.loads((R/'mod.override_info').read_text());report=[]
for cue,raw in M.items():
 name='mlbb_granger_voice_'+cue;clip='mlbb_heroes_clip_'+name
 source=S/'original'/(raw+'.mp3');out=R/'sound/sfx'/(name+'.wav')
 subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(source),'-af','loudnorm=I=-20:TP=-2:LRA=7','-ac','1','-ar','44100','-c:a','pcm_s16le',str(out)],check=True)
 # Client path API resolves the WAV; distinct event/clip names preserve native routing too.
 shutil.copy2(out,R/'sound/sfx'/(clip+'.wav'))
 (R/'sound/sfx'/(name+'.sound_info')).write_text(json.dumps({'plays':[{'delay':0.,'clip':clip,'volume':.68}]},indent=2)+'\n')
 for key in [name,clip]:overrides['asset/base/sound/sfx/'+key]={'remapping':'asset/mlbb_heroes/sound/sfx/'+key,'type':'override'}
 duration=float(subprocess.check_output(['ffprobe','-v','error','-show_entries','format=duration','-of','default=nw=1:nk=1',str(out)]))
 report.append({'cue':cue,'event':name,'source_mp3':source.name,'seconds':duration,'lock_ticks':math.ceil(duration*60)+12,'wav_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'format':'PCM s16le mono 44100Hz','loudness':'-20 LUFS target, -2dBTP ceiling','source_page':'https://mlbb.tools/audio?hero=granger','semantic_labels':'https://mobile-legends.fandom.com/wiki/Granger/Audio#Default'})
(R/'mod.override_info').write_text(json.dumps(overrides,indent=2)+'\n')
p=R/'sdk/candidate/audio.rs';code=p.read_text()
# Named sections allow re-running without duplicate registrations.
code=re.sub(r'    // GRANGER_CUES_BEGIN.*?    // GRANGER_CUES_END\n','',code,flags=re.S)
code=re.sub(r'        // GRANGER_DURATIONS_BEGIN.*?        // GRANGER_DURATIONS_END\n','',code,flags=re.S)
rows='    // GRANGER_CUES_BEGIN\n'+''.join(f'    ("granger_{x["cue"]}", "{x["event"]}", false, false),\n'for x in report)+'    // GRANGER_CUES_END\n'
code=code.replace('pub(crate) const VOICE_CUES: &[(&str, &str, bool, bool)] = &[','pub(crate) const VOICE_CUES: &[(&str, &str, bool, bool)] = &[\n'+rows,1)
rows='        // GRANGER_DURATIONS_BEGIN\n'+''.join(f'        "{x["event"]}" => {x["lock_ticks"]},\n'for x in report)+'        // GRANGER_DURATIONS_END\n'
code=code.replace('pub(crate) fn voice_lock_ticks(line: &str) -> usize {\n    match line {','pub(crate) fn voice_lock_ticks(line: &str) -> usize {\n    match line {\n'+rows,1)
p.write_text(code);(R/'reports/GRANGER_1147_AUDIO_INTAKE.json').write_text(json.dumps(report,indent=2)+'\n');print('Compiled',len(report),'voice events; measured locks; distinct event/clip routes; guarded path playback')
