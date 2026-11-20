"""Compose menu SVG frames from the shipped Popol and Kupa idle sprite sheets."""
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def frames(character):
    base = ROOT / 'effects' / f'popol_anim_{character}_idle'
    info = json.loads(Path(f'{base}#anim.fanim').read_text())
    with Image.open(f'{base}#sheet.png') as source:
        for frame in info['anims']['main']['frames']:
            x, y, w, h = (frame['data'][key] for key in ('x', 'y', 'w', 'h'))
            yield source.crop((x, y, x + w, y + h)).convert('RGBA')


def svg(image):
    width, height = image.size
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
             f'width="{width}" height="{height}" shape-rendering="crispEdges">']
    for y in range(height):
        x = 0
        while x < width:
            rgba = image.getpixel((x, y))
            end = x + 1
            while end < width and image.getpixel((end, y)) == rgba:
                end += 1
            r, g, b, a = rgba
            if a:
                opacity = f' fill-opacity="{a / 255:.4f}"' if a < 255 else ''
                parts.append(f'<rect x="{x}" y="{y}" width="{end-x}" height="1" '
                             f'fill="#{r:02x}{g:02x}{b:02x}"{opacity}/>')
            x = end
    return ''.join(parts) + '</svg>\n'


if __name__ == '__main__':
    popol = list(frames('popol'))
    kupa = list(frames('kupa'))
    count = max(len(popol), len(kupa))
    assert count == 9, count
    for i in range(count):
        canvas = Image.new('RGBA', (120, 72))
        canvas.alpha_composite(kupa[i % len(kupa)], (24, 0))
        canvas.alpha_composite(popol[i % len(popol)], (0, 0))
        path = ROOT / 'icons' / f'popol_menu_idle_{i}.svg'
        path.write_text(svg(canvas))

    for name, sheet, tag, expected in [
        ('granger', 'granger_anim_idle', 'main', 9),
        ('hanzo', 'hanzo_anim_human_idle', 'main', 5),
        ('atlas', 'atlas_rig', 'idle', 9),
    ]:
        base = ROOT / 'effects' / sheet
        info = json.loads(Path(f'{base}#anim.fanim').read_text())
        clip = info['anims'][tag]['frames']
        assert len(clip) == expected
        assert abs(sum(f['duration'] for f in clip) - 1.2) < .001
        with Image.open(f'{base}#sheet.png') as source:
            for i, f in enumerate(clip):
                data = f['data']
                x, y, w, h = (data[key] for key in ('x', 'y', 'w', 'h'))
                frame = source.crop((x, y, x+w, y+h)).convert('RGBA')
                assert frame.getchannel('A').getbbox(), (name, i)
                path = ROOT / 'icons' / f'{name}_draft_idle_{i}.svg'
                path.write_text(svg(frame))
