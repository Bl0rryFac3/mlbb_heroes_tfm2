#!/usr/bin/env python3
"""Prepare Karrie sprite/effect assets from the supplied atlas and VFX sheets.

Original user uploads are preserved under source/karrie/import_2026-10-24.
This script only creates new Karrie-named outputs and refuses to overwrite them.
"""
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
IN = ROOT / "source/karrie/import_2026-10-24"

# Relative timing per manifest hold-unit (seconds). These are preliminary visual
# playback values; action timing still needs TFM2 build/runtime verification.
BASE = {"idle": .12, "run": .06, "attack": .02, "skill": .02,
        "skill2": .02, "ult": .02, "hit": .04, "dead": .035}

def save_new(path: Path, image: Image.Image):
    if path.exists():
        raise FileExistsError(f"Refusing to overwrite existing file: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path)

def save_json_new(path: Path, data):
    if path.exists():
        raise FileExistsError(f"Refusing to overwrite existing file: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")

def clean_alpha(im: Image.Image, threshold=6) -> Image.Image:
    im = im.convert("RGBA")
    a = im.getchannel("A")
    lut = [0 if i < threshold else i for i in range(256)]
    im.putalpha(a.point(lut))
    return im

def frame_from_cell(im: Image.Image, box, crop_box=None) -> Image.Image:
    frame = im.crop(crop_box if crop_box else box)
    frame = clean_alpha(frame, 6)
    # Remove only faint transparent noise from the crop bounds; retain soft VFX glow.
    alpha = frame.getchannel("A")
    bbox = alpha.point([0 if i < 10 else 255 for i in range(256)]).getbbox()
    if bbox:
        frame = frame.crop(bbox)
    return frame

def pack_strip(frames, path: Path, anim_tag: str, duration: float, anchor=(.5,.5)):
    if not frames:
        raise ValueError(f"No frames for {path}")
    w=max(f.width for f in frames); h=max(f.height for f in frames)
    sheet=Image.new("RGBA", (w*len(frames),h), (0,0,0,0))
    for i,fr in enumerate(frames):
        x=(w-fr.width)//2; y=(h-fr.height)//2
        sheet.alpha_composite(fr, (i*w+x,y))
    save_new(path.with_name(path.name+"#sheet.png"),sheet)
    fanim={"anims":{anim_tag:{"frames":[
        {"duration":duration,"data":{"x":i*w,"y":0,"w":w,"h":h}}
        for i in range(len(frames))]}}}
    fanim["anchor_x"],fanim["anchor_y"]=anchor
    save_json_new(path.with_name(path.name+"#anim.fanim"),fanim)
    return (w,h)

def get_grid_cells(im: Image.Image, cols:int, rows:int):
    w,h=im.size
    for r in range(rows):
        y0=round(r*h/rows); y1=round((r+1)*h/rows)
        for c in range(cols):
            x0=round(c*w/cols); x1=round((c+1)*w/cols)
            yield r,c,(x0,y0,x1,y1)

# Character atlas: 22 columns x 8 rows, 72 px per cell.
manifest=json.loads((IN/"karrie_atlas.json").read_text(encoding="utf-8"))
atlas=Image.open(IN/"karrie_atlas.png").convert("RGBA")
expected=(manifest["cell_width"]*manifest["columns"],manifest["cell_height"]*len(manifest["rows"]))
if atlas.size != expected:
    raise ValueError(f"Atlas dimensions {atlas.size} do not match manifest grid {expected}")
save_new(ROOT/"sprite/karrie#sheet.png",atlas)
rows={name:i for i,name in enumerate(manifest["rows"])}
anims={}
for name,spec in manifest["animations"].items():
    row=rows[name]
    base=BASE[name]
    anims[name]={"frames":[
        {"duration":round(base*frame["hold"],6),"data":{
            "x":frame["col"]*manifest["cell_width"],
            "y":row*manifest["cell_height"],
            "w":manifest["cell_width"],"h":manifest["cell_height"]}}
        for frame in spec["frames"]]}
save_json_new(ROOT/"sprite/karrie#anim.fanim",{"anims":anims})

# Basic attack lightwheel projectile: six horizontal frames on the supplied sheet.
basic=Image.open(IN/"karrie-basicprojectile.png").convert("RGBA")
frames=[frame_from_cell(basic,box) for _,_,box in get_grid_cells(basic,6,1)]
pack_strip(frames,ROOT/"effects/karrie_basic_lightwheel","basic_projectile",.07)

# Skill 1: remove the baked white panel frames by cropping just inside each
# detected panel; top row is flight, lower row is the transient impact flare.
s1=Image.open(IN/"karrie-skill1projectile.png").convert("RGBA")
x0s=[15,297,579,862,1145,1427]
x1s=[285,567,850,1132,1415,1699]
row_boxes=[(109,438),(471,800)]
for row,(yt,yb) in enumerate(row_boxes):
    frames=[]
    for x0,x1 in zip(x0s,x1s):
        box=(x0+4,yt+4,x1-4,yb-4)
        frames.append(frame_from_cell(s1,box))
    tag="projectile" if row==0 else "impact"
    prefix="karrie_skill1_projectile" if row==0 else "karrie_skill1_impact"
    pack_strip(frames,ROOT/f"effects/{prefix}","effect",.06)

# Two 6x2 transparent sheets: upper/lower rows become independent animations.
ult=Image.open(IN/"Ultimate activation pulse andr Ultimate paired basic-attack projectiles.png").convert("RGBA")
for row,prefix,dur in [(0,"karrie_ult_activation_pulse",.06),(1,"karrie_ult_paired_lightwheels",.07)]:
    frames=[frame_from_cell(ult,box) for r,c,box in get_grid_cells(ult,6,2) if r==row]
    pack_strip(frames,ROOT/f"effects/{prefix}","effect",dur)

passive=Image.open(IN/"karrie-Lightwheel Mark and passive proc.png").convert("RGBA")
# Top row are six discrete stack/readiness states; each is stored as a one-frame tag.
mark_frames=[frame_from_cell(passive,box) for r,c,box in get_grid_cells(passive,6,2) if r==0]
w=max(f.width for f in mark_frames); h=max(f.height for f in mark_frames)
mark_sheet=Image.new("RGBA",(w*6,h),(0,0,0,0))
for i,fr in enumerate(mark_frames): mark_sheet.alpha_composite(fr,(i*w+(w-fr.width)//2,(h-fr.height)//2))
save_new(ROOT/"effects/karrie_passive_marks#sheet.png",mark_sheet)
mark_anims={}
for i in range(6):
    mark_anims[f"mark_{i+1}"]={"frames":[{"duration":.1,"data":{"x":i*w,"y":0,"w":w,"h":h}}]}
save_json_new(ROOT/"effects/karrie_passive_marks#anim.fanim",{"anims":mark_anims,"anchor_x":.5,"anchor_y":.5})
proc=[frame_from_cell(passive,box) for r,c,box in get_grid_cells(passive,6,2) if r==1]
pack_strip(proc,ROOT/"effects/karrie_passive_proc","effect",.06)

print("Prepared Karrie sprite atlas and VFX sheets.")
