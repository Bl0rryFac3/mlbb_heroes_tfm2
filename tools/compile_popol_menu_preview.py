"""Lossless SVG run representation of the first supplied Popol idle frame.
Used only by the menu overlay. Never changes champion gameplay carrier pixels.
"""
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
image=Image.open(ROOT/'source/popol_kupa/received/popol_idle.png').convert('RGBA').crop((0,0,72,72))
rects=[]
for y in range(image.height):
    x=0
    while x<image.width:
        color=image.getpixel((x,y));end=x+1
        while end<image.width and image.getpixel((end,y))==color:end+=1
        assert color[3] in (0,255), 'partial-alpha art needs an explicit SVG opacity'
        if color[3]:
            rects.append(f'<rect x="{x}" y="{y}" width="{end-x}" height="1" fill="#{color[0]:02x}{color[1]:02x}{color[2]:02x}"/>')
        x=end
(ROOT/'icons/popol_menu_preview.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" width="72" height="72" viewBox="0 0 72 72" shape-rendering="crispEdges">'+''.join(rects)+'</svg>')
print('Compiled menu-only preview from unchanged supplied idle pixels.')
