#!/usr/bin/env python3
"""Build a Balmond runtime-only ZIP and audit referenced assets; never delete source."""
from __future__ import annotations
import argparse, hashlib, json, re, zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
RUNTIME_DIRS=('champion','champions','effects','icons','override','sound','sprite','style','text','view')
ROOT_FILES=('mod.mod_info','mod.override_info','thumbnail.png','RELEASE_README.md')
def files_for(library:Path):
    out=[]
    for n in ROOT_FILES:
        p=ROOT/n
        if not p.is_file(): raise SystemExit(f'missing required file: {n}')
        out.append(p)
    if not library.is_file(): raise SystemExit(f'missing native library: {library}')
    out.append(library)
    for name in RUNTIME_DIRS:
        base=ROOT/name
        if not base.is_dir(): raise SystemExit(f'missing runtime directory: {name}')
        for p in sorted(base.rglob('*')):
            if not p.is_file(): continue
            rel=p.relative_to(base)
            if name=='effects' and rel.as_posix()=='IMPORT_REPORT.md': continue
            if name=='sprite' and (len(rel.parts)!=1 or not (rel.name.endswith('#sheet.png') or rel.name.endswith('#anim.fanim'))): continue
            if name=='sound' and (not rel.parts or rel.parts[0]!='sfx'): continue
            out.append(p)
    return sorted(set(out),key=lambda p:str(p.relative_to(ROOT)) if p!=library else 'zz-native-dll')
def validate(files,library):
    included={p.relative_to(ROOT).as_posix() for p in files if p!=library}
    info=json.loads((ROOT/'mod.mod_info').read_text()); ov=json.loads((ROOT/'mod.override_info').read_text())
    if info.get('version')!=__import__('tomllib').loads((ROOT/'sdk'/'candidate'/'Cargo.toml').read_text())['package']['version']:
        raise SystemExit('mod.mod_info / Cargo.toml version mismatch')
    # Parse all runtime data, view metadata, animation files, and audio manifests.
    for glob in ('champion/*.data_champion','override/*.data_champion','style/*.champion_view','text/*.i18n','view/*.view_effects','sprite/*.fanim','effects/*.fanim','sound/sfx/*.sound_info','mod.mod_info','mod.override_info'):
        for p in ROOT.glob(glob): json.loads(p.read_text())
    # Resolve all asset references.
    refs=set()
    for p in [*ROOT.glob('champion/*.data_champion'),*ROOT.glob('override/*.data_champion')]:
        refs.update(re.findall(r'asset/mlbb_heroes/[^\"?]+',p.read_text()))
    for ref in refs:
        stem=ref.removeprefix('asset/mlbb_heroes/')
        if stem.startswith(('effects/','sprite/','champions/')):
            for suff in ('#sheet.png','#anim.fanim'):
                path=stem+suff
                if path not in included: raise SystemExit(f'unresolved asset: {ref} -> {path}')
        elif stem.startswith('icons/'):
            if stem+'.png' not in included: raise SystemExit(f'unresolved icon: {ref}')
        elif stem.startswith(('text/','style/')): pass
    # Every event and clip has distinct IDs, WAV, and independent override mappings.
    sounds=ROOT/'sound/sfx'
    event_count=clip_count=0
    for p in sounds.glob('*.sound_info'):
        event=p.stem; event_count+=1
        expected=f'asset/mlbb_heroes/sound/sfx/{event}'
        if ov.get(f'asset/base/sound/sfx/{event}',{}).get('remapping')!=expected: raise SystemExit(f'missing sound override: {event}')
        data=json.loads(p.read_text()); plays=data.get('plays',[])
        if not plays: raise SystemExit(f'empty sound_info: {p}')
        for play in plays:
            clip=play.get('clip',''); clip_count+=1
            if not clip or clip==event: raise SystemExit(f'invalid event/clip pair in {p}')
            if f'sound/sfx/{clip}.wav' not in included: raise SystemExit(f'missing WAV for {clip}')
            if ov.get(f'asset/base/sound/sfx/{clip}',{}).get('remapping')!=f'asset/mlbb_heroes/sound/sfx/{clip}': raise SystemExit(f'missing clip override: {clip}')
    balmond=json.loads((ROOT/'champion/mlbb_balmond.data_champion').read_text())
    assert balmond['id']=='mlbb_balmond'
    assert balmond['passive']['passive_ref']=='mlbb_heroes:balmond_bloodthirst'
    # Every Balmond voice line registered from native code resolves to an event file.
    rust='\n'.join(p.read_text(errors='ignore') for p in ROOT.glob('sdk/candidate/*.rs'))
    for line in re.findall(r'"(mlbb_balmond_voice_[a-z0-9]+)"',rust):
        if not (sounds/f'{line}.sound_info').is_file(): raise SystemExit(f'Rust voice line lacks runtime event: {line}')
    # Resolve all custom native effect refs in champion data against registered Rust IDs.
    for p in ROOT.glob('champion/*.data_champion'):
        for ref in re.findall(r'"effect_ref":\s*"mlbb_heroes:([^\"]+)"',p.read_text()):
            if ref.startswith(('audio_voice_','audio_sfx_')): continue
            if ref.startswith('balmond_') and f'mlbb_heroes:{ref}' not in rust: raise SystemExit(f'unregistered Balmond native effect {ref} in {p.name}')
    if not library.read_bytes().startswith(b'MZ'): raise SystemExit('expected Windows PE DLL')
    forbidden=[x for x in included if x.startswith(('src/','source/','tools/')) or x in ('Cargo.toml','Cargo.lock') or 'mod-api-stable' in x]
    if forbidden: raise SystemExit(f'source/SDK leaked into runtime package: {forbidden[:10]}')
    return {'runtime_paths':len(included)+1,'sound_events':event_count,'clip_refs':clip_count,'asset_refs':len(refs)}
def main():
    ap=argparse.ArgumentParser();ap.add_argument('output',type=Path);ap.add_argument('--library',type=Path,default=ROOT/'target/x86_64-pc-windows-gnu/release/mlbb_heroes.dll');ap.add_argument('--force',action='store_true');args=ap.parse_args()
    out=args.output.resolve();
    if out.exists() and not args.force: raise SystemExit(f'refusing overwrite: {out}')
    chosen=files_for(args.library.resolve()); metrics=validate(chosen,args.library.resolve());out.parent.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
        for p in chosen:
            name='mlbb_heroes/mlbb_heroes.dll' if p==args.library.resolve() else ('mlbb_heroes/README.md' if p.name=='RELEASE_README.md' else 'mlbb_heroes/'+p.relative_to(ROOT).as_posix())
            z.write(p,name)
    audit={'archive':str(out),'mod_version':json.loads((ROOT/'mod.mod_info').read_text())['version'],'target':'TFM2 0.7.0 (unverified in game)','platform':'Windows x86-64','metrics':metrics,'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'bytes':out.stat().st_size,'source_or_sdk_in_runtime':False}
    out.with_suffix('.audit.json').write_text(json.dumps(audit,indent=2)+'\n')
    print(json.dumps(audit,indent=2))
if __name__=='__main__':main()
