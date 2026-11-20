#!/usr/bin/env python3
"""Shrink only the visible S2 spin art by 10%; preserve frame canvas and timing."""
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SHEET = ROOT / 'effects' / 'balmond_s2_fx#sheet.png'
ANIM = ROOT / 'effects' / 'balmond_s2_fx#anim.fanim'
SCALE = 0.90


def frame_metrics(image: Image.Image, anim: dict) -> list[dict]:
    results=[]
    for index, frame in enumerate(anim['anims']['spin']['frames']):
        data=frame['data']
        bounds=(round(data['x']), round(data['y']), round(data['x']+data['w']), round(data['y']+data['h']))
        crop=image.crop(bounds)
        alpha_box=crop.getchannel('A').getbbox()
        if alpha_box is None:
            raise SystemExit(f'frame {index} is fully transparent')
        x0,y0,x1,y1=alpha_box
        results.append({'frame':index,'canvas':list(crop.size),'alpha_bbox':list(alpha_box),'alpha_size':[x1-x0,y1-y0]})
    return results


def main() -> None:
    anim=json.loads(ANIM.read_text())
    original=Image.open(SHEET).convert('RGBA')
    if len(anim.get('anims',{}).get('spin',{}).get('frames',[])) != 8:
        raise SystemExit('expected the existing eight-frame S2 spin animation')
    before=frame_metrics(original,anim)
    sheet=original.copy()
    for item, frame in zip(before,anim['anims']['spin']['frames']):
        data=frame['data']
        bounds=(round(data['x']),round(data['y']),round(data['x']+data['w']),round(data['y']+data['h']))
        cell=original.crop(bounds)
        x0,y0,x1,y1=item['alpha_bbox']
        visible=cell.crop((x0,y0,x1,y1))
        new_w=max(1,round(visible.width*SCALE)); new_h=max(1,round(visible.height*SCALE))
        smaller=visible.resize((new_w,new_h),Image.Resampling.NEAREST)
        cx=(x0+x1)/2; cy=(y0+y1)/2
        paste_x=round(cx-new_w/2); paste_y=round(cy-new_h/2)
        # Clear the original alpha footprint, then center the reduced frame content in the same cell.
        cell.paste((0,0,0,0),(x0,y0,x1,y1))
        cell.alpha_composite(smaller,(paste_x,paste_y))
        sheet.paste(cell,bounds[:2])
    after=frame_metrics(sheet,anim)
    for old,new in zip(before,after):
        if not (new['alpha_size'][0] <= round(old['alpha_size'][0]*SCALE)+2 and new['alpha_size'][1] <= round(old['alpha_size'][1]*SCALE)+2):
            raise SystemExit(f'frame {old["frame"]} did not shrink by the requested 10%')
    sheet.save(SHEET,optimize=True)
    report={'scale':SCALE,'method':'10% content-only downscale per frame; unchanged sheet/cell dimensions and FAnim timing','sheet_size':list(sheet.size),'frames_before':before,'frames_after':after}
    report_path=ROOT/'reports'/'BALMOND_V7_S2_VFX_SCALE_AUDIT.json'
    report_path.parent.mkdir(parents=True,exist_ok=True)
    report_path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))

if __name__=='__main__':main()
