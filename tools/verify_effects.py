#!/usr/bin/env python3
"""Verify every effect bound by mlbb_belerick.data_champion resolves to a real,
sane asset, and assert the five playtest fixes from v2.0.7-rc.1 are still in
place.

Checks two independent things for each binding:
  1. the .fanim and its .png sheet both exist on disk
  2. the fanim's frame rects stay inside the sheet (a canvas resize that forgot
     to update the rects is invisible until the art renders garbage in game)

Then re-measures the four recut effects against the hero sprite and prints the
before/after the owner can compare with what he saw in the match.
"""
import json
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image

def _find_root() -> Path:
    # The script ships in two places (belerick_work/ and build_v2081/tools/),
    # so don't hardcode a depth — walk up until the build root shows itself.
    for cand in Path(__file__).resolve().parents:
        # Require BOTH markers. belerick_work/ keeps its own stale
        # champion/ + effects/ working copies (that is where art is cut
        # before it is installed), and must never be mistaken for the
        # build root — only the build root has sdk/candidate/belerick.rs.
        if (cand / "champion" / "mlbb_belerick.data_champion").is_file() and (
            cand / "sdk" / "candidate" / "belerick.rs"
        ).is_file():
            return cand
    # Not a parent — try the sibling. This is the belerick_work/ case:
    # the work dir sits *beside* the build root, not inside it.
    here = Path(__file__).resolve()
    for sib in (here.parent / "build_v2081", here.parent.parent / "build_v2081"):
        if (sib / "champion" / "mlbb_belerick.data_champion").is_file():
            return sib
    raise SystemExit("cannot locate the build root near " + str(here))


ROOT = _find_root()
DATA = ROOT / "champion" / "mlbb_belerick.data_champion"
FX = ROOT / "effects"
SPRITE = ROOT / "sprite"

HERO_W = HERO_H = 72  # every roster sprite frame is 72x72

failures: list[str] = []
warnings: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        failures.append(msg)


def sheet_size(name: str):
    for ext in ("#sheet.png", "#sheet.PNG"):
        p = FX / f"{name}{ext}"
        if p.is_file():
            with Image.open(p) as im:
                return im.size
    return None


def anim_of(name: str):
    p = FX / f"{name}#anim.fanim"
    if not p.is_file():
        return None
    return json.loads(p.read_text())


def frames(d):
    return list((d.get("anims") or d).values())[0]["frames"]


def content_bbox(sheet_path, n, frame_w, h):
    """Largest per-frame content bbox + peak alpha coverage over the canvas."""
    im = Image.open(sheet_path).convert("RGBA")
    W, H = im.size
    fw = W // n
    peak = (0, 0, 0, 0.0)
    for i in range(n):
        a = np.array(im.crop((i * fw, 0, (i + 1) * fw, H)))[..., 3]
        m = a > 8
        if m.sum() > peak[0]:
            ys, xs = np.nonzero(m)
            peak = (int(m.sum()),
                    int(xs.max() - xs.min() + 1),
                    int(ys.max() - ys.min() + 1),
                    100.0 * m.sum() / (fw * H))
    return peak[1], peak[2], peak[3]


def main() -> int:
    data = json.loads(DATA.read_text())

    bindings = []
    for key in ("view_effects", "view_buffs", "view_projectiles"):
        for e in data.get(key) or []:
            bindings.append((key, e))

    print(f"=== {len(bindings)} effect bindings in mlbb_belerick.data_champion ===\n")
    print(f"  {'binding':16s} {'name':30s} {'canvas':10s} {'anchor_y':9s} frames")

    for key, e in bindings:
        name = e["name"]
        # view_buffs / view_projectiles point at animations by anim path
        anim_path = e.get("anim") or e.get("name")
        base = anim_path.split(":")[-1].split("/")[-1]
        d = anim_of(base)
        if d is None:
            check(False, f"[{key}] {name}: missing {base}#anim.fanim")
            print(f"  {key:16s} {name:30s} MISSING FANIM")
            continue
        fr = frames(d)
        n = len(fr)
        w = fr[0]["data"]["w"]
        h = fr[0]["data"]["h"]
        size = sheet_size(base)
        if size is None:
            check(False, f"[{key}] {name}: missing {base}#sheet.png")
            print(f"  {key:16s} {name:30s} MISSING SHEET")
            continue
        # frame rects must stay inside the sheet
        need_w = max(f["data"]["x"] + f["data"]["w"] for f in fr)
        need_h = max(f["data"]["y"] + f["data"]["h"] for f in fr)
        check(need_w <= size[0] and need_h <= size[1],
              f"[{key}] {name}: frame rects need {need_w}x{need_h} but sheet is {size}")
        check(size[0] % n == 0,
              f"[{key}] {name}: sheet {size[0]}px is not a whole multiple of {n} frames")
        print(f"  {key:16s} {name:30s} {w}x{h:<7} {d.get('anchor_y')!s:<9} {n}")

    # ---- the five playtest fixes ----
    print("\n=== the five playtest fixes ===")

    # 1. thorn_eruption REMOVED at the owner's request (rc.3) - it looked ugly.
    #    The S1 damage + taunt survive on the *_zone RangeProjectile, so assert
    #    the picture is gone AND the gameplay is still there.
    gone = not (FX / "belerick_thorn_eruption#sheet.png").exists() \
        and not (FX / "belerick_thorn_eruption#anim.fanim").exists()
    raw = DATA.read_text()
    zone = "belerick_thorn_eruption_zone" in raw and '"Taunt"' in raw
    ok = gone and zone
    check(gone, "belerick_thorn_eruption art still present - it was meant to be deleted")
    check(zone, "belerick_thorn_eruption_zone vanished - S1 would lose its damage and taunt")
    print(f"  1 eruption removed  art gone: {gone}, taunt zone kept: {zone}"
          f"                 {'OK' if ok else 'FAIL'}")

    # 2. dryad_wrath: z must be > 0 and content taller than the 72px body
    ve = {e["name"]: e for e in data.get("view_effects") or []}
    z = ve.get("belerick_dryad_wrath", {}).get("z")
    cw, chh, cov = content_bbox(FX / "belerick_dryad_wrath#sheet.png", 9, 208, 208)
    ok = z is not None and z > 0 and chh > HERO_H
    check(ok, f"dryad_wrath z={z} content height {chh} <= body {HERO_H}")
    print(f"  2 dryad_wrath      z={z} (was -1), content {cw}x{chh}"
          f" — taller than his {HERO_H}px body  {'OK' if ok else 'FAIL'}")

    # 3. taunt_marker: tall canvas + anchor_y 1.0 + small glyph
    d = anim_of("belerick_taunt_marker")
    fr = frames(d)
    w, h = fr[0]["data"]["w"], fr[0]["data"]["h"]
    cw, chh, cov = content_bbox(FX / "belerick_taunt_marker#sheet.png", 9, w, h)
    ok = d.get("anchor_y") == 1.0 and h > 96 and max(cw, chh) <= 30
    check(ok, f"taunt_marker anchor_y={d.get('anchor_y')} canvas {w}x{h} glyph {cw}x{chh}")
    print(f"  3 taunt_marker     canvas {w}x{h}, anchor_y={d.get('anchor_y')},"
          f" glyph {cw}x{chh} @ {cov:.1f}%        {'OK' if ok else 'FAIL'}")

    # 4. nature_ready: big enough to reach his hand, smaller than the original.
    #    rc.2 cut it to 50% of the hero and the owner reported the S2 glow "does
    #    not align with his hand anymore" - at 81% it covered the hand, at 50%
    #    it was a blob in his chest. 65% is the compromise.
    d4 = anim_of("belerick_nature_ready")
    fr4 = frames(d4)
    cw, chh, cov = content_bbox(FX / "belerick_nature_ready#sheet.png", 9,
                                fr4[0]["data"]["w"], fr4[0]["data"]["h"])
    pct = 100 * max(cw, chh) / HERO_W
    ok = 60 <= pct <= 72
    check(ok, f"nature_ready is {pct:.0f}% of hero; wants 60-72% "
              f"(81% was too big, 50% did not reach his hand)")
    print(f"  4 nature_ready     content {cw}x{chh} = {pct:.0f}% of hero"
          f"  (81% too big, 50% rc.2)      {'OK' if ok else 'FAIL'}")

    # 5. thorn_sweep: flight must cover most of the animation
    rs = (ROOT / "sdk" / "candidate" / "belerick.rs").read_text()
    speed = int(re.search(r"const THORN_SPEED: u64 = ([\d_]+)", rs).group(1).replace("_", ""))
    rng = int(re.search(r"const THORN_RANGE: u64 = ([\d_]+)", rs).group(1).replace("_", ""))
    d = anim_of("belerick_thorn_sweep")
    fr = frames(d)
    anim_ticks = round(sum(f["duration"] for f in fr) * 60)
    flight = rng // speed
    ok = flight * 4 >= anim_ticks * 3
    check(ok, f"thorn_sweep flight {flight} ticks < 3/4 of anim {anim_ticks}")
    print(f"  5 thorn_sweep      flight {flight} ticks vs {anim_ticks}-tick animation"
          f"          {'OK' if ok else 'FAIL'}")

    # ---- extras: ult node type, cooldowns, S2 animation ----
    print()
    print("=== also changed in rc.3 ===")
    src = "".join(DATA.read_text().split())  # strip all whitespace: the
    # data is written indented or compact depending on the builder, so match
    # the raw text rather than assuming one layout.
    ult_i = src.find('\"ult\":')
    ok = '\"type\":\"CasterViewEffect\",\"name\":\"belerick_dryad_wrath\"' in src[ult_i:]
    check(ok, "ult does not use CasterViewEffect - the ult renders invisible")
    print(f"  6 ult node         CasterViewEffect (not ViewEffect)          "
          f"{'OK' if ok else 'FAIL'}")
    for n, (slot, want) in enumerate([("skill", 270), ("skill2", 360), ("ult", 1350)], start=7):
        i = src.find(f'\"{slot}\":')
        j = src.find('\"cooltime\":', i)
        num = ""
        for ch in src[j + len('\"cooltime\":'):]:
            if ch.isdigit():
                num += ch
            else:
                break
        got = int(num)
        okc = got == want
        check(okc, f"{slot} cooltime {got}, expected {want}")
        print(f"  {n} {slot:8s} CD      {got} ticks = {got/60:.1f}s  (was {want*2/60:.1f}s)   "
              f"{'OK' if okc else 'FAIL'}")

    print()
    for w in warnings:
        print(f"  WARN: {w}")
    if failures:
        print(f"\n  {len(failures)} FAILURE(S):")
        for f in failures:
            print(f"   - {f}")
        return 1
    print(f"  ALL CHECKS PASSED — {len(bindings)} bindings resolve, 5/5 fixes in place")
    return 0


if __name__ == "__main__":
    sys.exit(main())
