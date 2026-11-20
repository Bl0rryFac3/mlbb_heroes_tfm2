#!/usr/bin/env python3
"""Offline V10 original-sprite visual audit; pass --v8-source to preserved V8 tree."""
from __future__ import annotations
import argparse, copy, hashlib, json
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]

def walk(obj):
    if isinstance(obj,dict):
        yield obj
        for v in obj.values(): yield from walk(v)
    elif isinstance(obj,list):
        for v in obj: yield from walk(v)

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--v8-source',type=Path,required=True); ap.add_argument('--report',type=Path); args=ap.parse_args()
    v8=args.v8_source
    v10=json.loads((ROOT/'champion/mlbb_balmond.data_champion').read_text())
    v8doc=json.loads((v8/'candidate/champion/mlbb_balmond.data_champion').read_text())
    results=[]
    def check(name,ok):
        if not ok: raise AssertionError(name)
        results.append(name)
    lines=[o for o in walk(v10) if o.get('type')=='LineRangeProjectile' and o.get('name')=='mlbb_balmond_ult_line']
    check('exactly one visual-only Ult line exists',len(lines)==1 and lines[0].get('applied_effects')==[])
    line=lines[0]
    check('original line length retained and intermediate width is 36,000',line.get('length')==110000 and line.get('width')==36000)
    check('immediate trigger/lifetime retained',line.get('apply')==1 and line.get('delay')==30)
    check('original slash animation binding restored',v10.get('view_projectiles')==v8doc.get('view_projectiles') and v10['view_projectiles'][0].get('anim')=='asset/mlbb_heroes/effects/balmond_ult_fx' and v10['view_projectiles'][0].get('tag')=='slash')
    norm=copy.deepcopy(v10); normline=[o for o in walk(norm) if o.get('type')=='LineRangeProjectile' and o.get('name')=='mlbb_balmond_ult_line'][0]
    oldline=[o for o in walk(v8doc) if o.get('type')=='LineRangeProjectile' and o.get('name')=='mlbb_balmond_ult_line'][0]
    normline['width']=oldline['width']
    check('all other champion data (fan, execute, S1/S2, range, targeting) matches V8',norm==v8doc)
    check('original 201×122 slash sheet is unchanged',Image.open(ROOT/'effects/balmond_ult_fx#sheet.png').size==(201,122) and hashlib.sha256((ROOT/'effects/balmond_ult_fx#sheet.png').read_bytes()).digest()==hashlib.sha256((v8/'candidate/effects/balmond_ult_fx#sheet.png').read_bytes()).digest())
    check('V8 native Balmond source and 1,000 × 3 S1 knockback are unchanged',(ROOT/'src/balmond.rs').read_bytes()==(v8/'candidate/src/balmond.rs').read_bytes() and 'entity_knockback(caster, target, 1_000, 3)' in (ROOT/'src/balmond.rs').read_text())
    check('no V9 custom wave assets/binding are included',not (ROOT/'effects/balmond_ult_wave#sheet.png').exists() and not any(o.get('anim')=='asset/mlbb_heroes/effects/balmond_ult_wave' for o in walk(v10)))
    output={'revision':'V10','checks_passed':len(results),'checks_failed':0,'checks':results,'v8_data_sha256':hashlib.sha256((v8/'candidate/champion/mlbb_balmond.data_champion').read_bytes()).hexdigest(),'v10_data_sha256':hashlib.sha256((ROOT/'champion/mlbb_balmond.data_champion').read_bytes()).hexdigest(),'in_game_verified':False}
    text=json.dumps(output,indent=2)+'\n'
    if args.report: args.report.parent.mkdir(parents=True,exist_ok=True); args.report.write_text(text)
    print(text,end='')
if __name__=='__main__': main()
