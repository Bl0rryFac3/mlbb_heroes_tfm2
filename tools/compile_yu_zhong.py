"""Compile the supplied Yu Zhong sheets into stable TFM2 bindings."""
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageOps
from build_popol_preview import svg

ROOT = Path(__file__).resolve().parents[1]
RECEIVED = ROOT / "source/yu_zhong/received"
effects, audit = [], {}
BODY_SAMPLE_TICKS = 4

BODY = {
    "human_idle": (72,72,9,72), "human_jog": (72,72,9,39),
    "basic": (72,72,9,24), "empowered_basic": (72,72,9,24),
    "dragon_tail": (72,72,9,38), "soul_grip": (72,72,9,22),
    "furious_dive": (72,72,9,22), "dragonoid_idle": (72,72,9,72),
    "dragonoid_jog": (72,72,9,39), "dragonoid_attack": (72,72,9,24),
    "dragonoid_soul_grip": (72,72,9,22), "dragonoid_dragon_tail": (72,72,9,38),
    "dragonoid_furious_dive": (72,72,9,22), "black_dragon_entry": (128,88,9,36),
    "black_dragon_flight": (144,88,9,45), "black_dragon_contact": (160,104,5,14),
    "dragon_exit": (128,88,9,24), "dragonoid_to_human": (64,48,5,18),
}
FX = {
    "tail_warning": (96,64,5,24), "tail_crescents": (96,64,4,16),
    "tail_inner_hit": (32,32,5,12), "tail_outer_hit": (32,32,5,14),
    "grip_wave": (172,84,9,20), "grip_ready_glow": (32,32,5,32),
    "grip_empowered_flash": (32,32,5,12), "dive_warning": (96,64,5,24),
    "dive_spikes": (96,64,4,16), "dragon_channel": (64,64,5,36),
    "dragon_contact_fx": (32,32,5,12), "dragon_exit_fx": (96,64,5,24),
    "eruption_flash": (32,32,5,12), "returning_mote": (24,24,9,16),
}
def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2)+'\n')

def crop(name, w,h,count):
    path=RECEIVED / f"{name}.png"
    with Image.open(path) as f:
        im=f.convert('RGBA')
    assert im.width % w == 0 and im.height % h == 0, (name, im.size)
    cols=im.width//w
    cells=[im.crop((i%cols*w,i//cols*h,(i%cols+1)*w,(i//cols+1)*h)) for i in range(count)]
    assert any(c.getbbox() for c in cells), name
    audit.setdefault(name,{"original_sha256":hashlib.sha256(path.read_bytes()).hexdigest(),
                 "source_cell":[w,h],"frames":len(cells)})
    return cells

def fit_ground(im):
    x0,y0,x1,y1=im.getbbox()
    sc=min(1.,48/(y1-y0),70/(x1-x0))
    if sc==1. and im.size==(72,72) and y1<=68:
        return im
    content=im.crop((x0,y0,x1,y1))
    content=content.resize((max(1,round(content.width*sc)),max(1,round(content.height*sc))),Image.Resampling.NEAREST)
    out=Image.new('RGBA',(72,72))
    out.alpha_composite(content,((72-content.width)//2,62-content.height))
    return out

def write_anim(folder, name, frames, anchor, durations, tags=None):
    base=ROOT/folder/f"yu_zhong_{name}"
    w,h=frames[0].size
    assert all(f.size==(w,h) for f in frames)
    cols=min(6,len(frames))
    sheet=Image.new("RGBA",(cols*w,((len(frames)+cols-1)//cols)*h))
    specs=[]
    for i,(im,t) in enumerate(zip(frames,durations,strict=True)):
        x,y=i%cols*w,i//cols*h
        sheet.alpha_composite(im,(x,y))
        specs.append(dict(duration=t/60,data=dict(x=x,y=y,w=w,h=h)))
    sheet.save(str(base)+"#sheet.png")
    write_json(Path(str(base)+"#anim.fanim"),dict(anchor_x=anchor[0],anchor_y=anchor[1],
        anims={tag:dict(frames=[specs[i] for i in ids]) for tag,ids in (tags or {"main":range(len(frames))}).items()}))
    return f"asset/mlbb_heroes/{folder}/yu_zhong_{name}"

def effect(name, anim, tag="main",follow=False,z=2):
    effects.append(dict(type="Animation",name=f"mlbb_yu_fx_{name}",anim=anim,tag=tag,z=z,is_follow=follow))

human=None
for name,(w,h,n,t) in BODY.items():
    frames=crop("human_walk" if name == "human_jog" else name,w,h,n)
    if not name.startswith("black_dragon") and name!="dragon_exit":
        frames=[fit_ground(im) for im in frames]
        if name=="human_idle":
            human=frames
        anchor=(.5,62/72)
    else:
        anchor=(.70,.61) if name in ("black_dragon_flight","black_dragon_contact") else (.55,.69)
    for suffix, seq, pivot in [("",frames,anchor),("_left",[ImageOps.mirror(f) for f in frames],(1-anchor[0],anchor[1]))]:
        # A detached effect can expire independently when the live actor is
        # removed. The final empty frame also prevents a retained final pose.
        blank = Image.new("RGBA", seq[0].size)
        tags = {"main": range(n)}
        tags.update({f"body_{i}": [n+i, 2*n] for i in range(n)})
        asset = write_anim("effects", name+suffix, seq+seq+[blank], pivot,
                           [t/n]*n + [BODY_SAMPLE_TICKS]*n + [1], tags)
        for i in range(n):
            effect(f"body_{name}{suffix}_{i}", asset, tag=f"body_{i}", z=1)
    audit["human_walk" if name == "human_jog" else name]["runtime_cell"]=list(frames[0].size)
assert human is not None
# One visible portrait frame followed by a transparent match carrier. Draft animates
# via the same shared grid preview used for the shipped layered champions.
blank = Image.new("RGBA", (72, 72))
# The long frame keeps the draft carrier transparent in a live match. Give
# the game's death state its own short frame so the native corpse can finish.
write_anim("sprite", "carrier", [human[0], blank, blank], (.5, 62/72), [.00006, 5_184_000, 7.2],
    {"idle":[0,1], **{n:[1] for n in ("main","run","attack","hit","skill","skill2","ult")}, "dead":[2]})
for i,im in enumerate(human):
    (ROOT/"icons"/f"yu_zhong_draft_idle_{i}.svg").write_text(svg(im))
for name,(w,h,n,t) in FX.items():
    frames=crop(name,w,h,n)
    if name=="grip_wave":
        variants=[]
        for mode, width in [("",55),("_large",66)]:
            scaled=[f.resize((width,round(h*width/w)),Image.Resampling.NEAREST) for f in frames]
            for angle in range(16):
                degrees=angle*22.5
                variants.append((f"{mode}_{angle}",[f.rotate(-degrees,Image.Resampling.NEAREST,expand=True) for f in scaled]))
    elif name in ("tail_warning","tail_crescents","dive_warning","dive_spikes"):
        width=70 if name.startswith("tail") else 46
        variants=[(suffix,[f.resize((size,round(h*size/w)),Image.Resampling.NEAREST) for f in frames])
                  for suffix,size in [("",width),("_large",round(width*1.2))]]
    else:
        variants=[("",frames)]
    for suffix, seq in variants:
        follow=name in ("grip_ready_glow","tail_warning")
        impact=name in ("tail_warning","dive_warning","tail_crescents","dive_spikes")
        asset=write_anim("effects",name+suffix,seq,(.5,.58 if impact else .5),[t/n]*n)
        effect(name+suffix,asset,follow=follow,z=0 if impact else 2)
marks=crop("residue",72,72,5)[:3]
asset=write_anim("effects","residue",marks,(.5,1.35),[20]*3,{f"mark_{i}":[i-1] for i in range(1,4)})
for i in range(1,4):
    effect(f"mark_{i}",asset,tag=f"mark_{i}",follow=True,z=4)

def icon(name, cell):
    image=cell.crop(cell.getbbox())
    image.thumbnail((92,92),Image.Resampling.NEAREST)
    if image.width<46 and image.height<46:
        image=image.resize((image.width*2,image.height*2),Image.Resampling.NEAREST)
    out=Image.new("RGBA",(100,100),(29,24,48,255))
    out.alpha_composite(image,((100-image.width)//2,(100-image.height)//2))
    out.save(ROOT/"icons"/f"mlbb_yu_zhong_{name}.png")
icon("base_attack", crop("basic", 72, 72, 9)[4])
for name, remote in {"passive":"passive", "skill":"skill_1", "skill2":"skill_2", "ult":"ultimate"}.items():
    with Image.open(ROOT / "source/yu_zhong/skill_icons" / f"{remote}.png") as original:
        original.convert("RGBA").resize((100, 100), Image.Resampling.LANCZOS).save(
            ROOT / "icons" / f"mlbb_yu_zhong_{name}.png")

base=json.loads((ROOT/"champion/mlbb_atlas.data_champion").read_text())
base.update(id="mlbb_yu_zhong",category="Melee",tags=["AD","Melee","CC"],
            sprite="asset/mlbb_heroes/sprite/yu_zhong_carrier",
            skill_icons=[f"asset/mlbb_heroes/icons/mlbb_yu_zhong_{slot}" for slot in ("skill","skill2","ult")],
            passive={"passive_ref":"mlbb_heroes:yu_zhong"},view_buffs=[],view_effects=effects)
base["stat"].update(hp=830,attack=122,defence=33,magic_resistance=25,move_speed=1080)
base["growth"].update(hp=105,attack=5,defence=4,magic_resistance=2)
for slot, cooldown, reach, action in [
    ("attack",60,24000,"basic"),("skill",420,35000,"tail"),("skill2",600,66000,"grip"),("ult",2760,95000,"dragon")]:
    data=base[slot]
    data.update(cooltime=cooldown,range=reach,description=f"#asset/base/text/champion?description.mlbb_yu_zhong.{slot}")
    native=dict(type="Native",effect_ref=f"mlbb_heroes:yu_{action}")
    data["effect"]=native
    if slot=="attack":
        data["effect"]=dict(type="Combine",effects=[native,{"type":"Attack","damage":0,"attack_ratio":100,"attack_effect_type":"Target"}])
    elif slot in ("skill2","ult"):
        data.update(casting_type="Targeting",casting_target="EnemyWithoutTower" if slot=="skill2" else "EnemyChampion")

write_json(ROOT/"champion/mlbb_yu_zhong.data_champion",base)
write_json(ROOT/"sdk/candidate/champion/mlbb_yu_zhong.data_champion",base)
write_json(ROOT/"source/yu_zhong/compiled_assets.json",audit)

# Keep the simulation sampler in sync with the compiler's art timings.
body_rust = "// Generated by tools/compile_yu_zhong.py.\n"
body_rust += f"const BODY_SAMPLE_TICKS: usize = {BODY_SAMPLE_TICKS};\n"
body_rust += "fn body_clip(name: &str) -> (usize, usize) {\n    match name {\n"
for name, (_, _, frames, ticks) in BODY.items():
    body_rust += f'        "{name}" => ({frames}, {ticks}),\n'
body_rust += '        _ => unreachable!("unknown Yu Zhong body clip: {name}"),\n    }\n}\n'
(ROOT/"sdk/candidate/yu_zhong_body.rs").write_text(body_rust)
