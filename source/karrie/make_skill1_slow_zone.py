#!/usr/bin/env python3
"""Create the stationary lingering Skill 1 field from the existing impact art."""
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
EFFECTS = ROOT / "effects"
SOURCE = EFFECTS / "karrie_skill1_impact_game#sheet.png"
OUT = EFFECTS / "karrie_skill1_slow_zone"

if not SOURCE.is_file():
    raise SystemExit(f"Missing source impact sheet: {SOURCE}")
outputs = [OUT.with_name(OUT.name + "#sheet.png"), OUT.with_name(OUT.name + "#anim.fanim")]
if any(p.exists() for p in outputs):
    raise SystemExit(f"Refusing to overwrite existing slow-zone outputs: {outputs}")

sheet = Image.open(SOURCE).convert("RGBA")
frame_w, frame_h = 64, 79
if sheet.width < frame_w * 2 or sheet.height != frame_h:
    raise SystemExit(f"Unexpected Skill 1 impact atlas dimensions: {sheet.size}")
# The second impact frame is the clearest stable violet ring with the spiky interior.
zone = sheet.crop((frame_w, 0, frame_w * 2, frame_h))
if zone.getchannel("A").getbbox() is None:
    raise SystemExit("Selected slow-zone ring frame is transparent")
zone.save(outputs[0])
meta = {
    "anims": {
        "zone": {
            "frames": [
                {"duration": 0.1, "data": {"x": 0, "y": 0, "w": frame_w, "h": frame_h}}
            ]
        }
    },
    "anchor_x": 0.5,
    "anchor_y": 0.5,
}
outputs[1].write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
print(f"Created stationary 1-frame Skill 1 zone art: {zone.size}")
