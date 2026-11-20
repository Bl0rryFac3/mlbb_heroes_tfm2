#!/usr/bin/env python3
"""Audit V10 runtime package against preserved V8 runtime ZIP."""
from __future__ import annotations
import argparse, hashlib, json, struct, zipfile
from pathlib import Path
CHANGED={'mlbb_heroes/README.md','mlbb_heroes/champion/mlbb_balmond.data_champion','mlbb_heroes/mod.mod_info','mlbb_heroes/mlbb_heroes.dll'}
FORBIDDEN=('mod-api-stable/','candidate/src/','.rs','Cargo.toml','Cargo.lock','target/')
def readzip(p):
    with zipfile.ZipFile(p) as z:
        bad=z.testzip()
        if bad: raise ValueError(f'bad ZIP member {bad}')
        return {i.filename:z.read(i) for i in z.infolist() if not i.is_dir()}
def walk(o):
    if isinstance(o,dict):
        yield o
        for v in o.values(): yield from walk(v)
    elif isinstance(o,list):
        for v in o: yield from walk(v)
def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--v8',type=Path,required=True); ap.add_argument('--v10',type=Path,required=True); ap.add_argument('--report',type=Path); a=ap.parse_args()
    old,new=readzip(a.v8),readzip(a.v10)
    assert set(old)==set(new),'runtime path set changed'
    changed={p for p in old if old[p]!=new[p]}
    assert changed==CHANGED, f'unexpected runtime diffs: {sorted(changed ^ CHANGED)}'
    assert len(new)==490, f'expected 490 runtime files, got {len(new)}'
    assert not any(any(x in p for x in FORBIDDEN) for p in new),'source or SDK in runtime'
    meta=json.loads(new['mlbb_heroes/mod.mod_info']); champ=json.loads(new['mlbb_heroes/champion/mlbb_balmond.data_champion'])
    lines=[o for o in walk(champ) if o.get('type')=='LineRangeProjectile' and o.get('name')=='mlbb_balmond_ult_line']
    assert len(lines)==1 and lines[0].get('width')==36000 and lines[0].get('length')==110000 and lines[0].get('apply')==1 and lines[0].get('delay')==30 and lines[0].get('applied_effects')==[]
    assert champ['view_projectiles']==json.loads(old['mlbb_heroes/champion/mlbb_balmond.data_champion'])['view_projectiles']
    assert 'v10' in meta['description'].lower() and meta['version']=='1.1.63'
    dll=new['mlbb_heroes/mlbb_heroes.dll']; assert dll[:2]==b'MZ'
    pe=struct.unpack_from('<I',dll,0x3c)[0]; assert dll[pe:pe+4]==b'PE\0\0' and struct.unpack_from('<H',dll,pe+4)[0]==0x8664
    result={'baseline':a.v8.name,'archive':a.v10.name,'platform':'Windows x86-64','runtime_files':len(new),'runtime_paths_identical_to_v8':True,'changed_runtime_files':sorted(changed),'source_or_sdk_in_runtime':False,'dll_machine':'AMD64 PE32+','sha256':hashlib.sha256(a.v10.read_bytes()).hexdigest(),'bytes':a.v10.stat().st_size,'in_game_verified':False}
    text=json.dumps(result,indent=2)+'\n'
    if a.report: a.report.parent.mkdir(parents=True,exist_ok=True); a.report.write_text(text)
    print(text,end='')
if __name__=='__main__': main()
