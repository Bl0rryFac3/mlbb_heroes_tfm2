#!/usr/bin/env python3
import unittest,tempfile,json,subprocess,sys
from pathlib import Path
from PIL import Image,ImageDraw
H=Path(__file__).resolve().parents[1]
class Tools(unittest.TestCase):
 def test_missing_art_cannot_silently_compile_a_runtime(self):
  with tempfile.TemporaryDirectory() as tmp:
   r=subprocess.run([sys.executable,str(H/'tools/compile_assets.py'),tmp,tmp],capture_output=True,text=True)
   self.assertNotEqual(r.returncode,0);self.assertIn('Missing returned assets',r.stderr)
 def test_native_grids_compile_and_keep_all_nine_frames(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp)/'root';inbox=Path(tmp)/'in';root.mkdir();(root/'icons').mkdir();manifest=json.loads((H/'ASSET_MANIFEST.json').read_text())
   for e in manifest:
    w,h=e['cell'];im=Image.new('RGBA',(w*3,h*3));draw=ImageDraw.Draw(im)
    for j in range(9):
     ox=j%3*w;oy=j//3*h
     box=(24,11,47,55) if e['kind']=='character' else ((12,20,50,28) if e.get('origin') else (w//2-8,h//2-4,w//2+8,h//2+4))
     draw.rectangle((box[0]+ox,box[1]+oy,box[2]+ox,box[3]+oy),fill=(30,180,220,255))
    p=inbox/e['file'];p.parent.mkdir(parents=True,exist_ok=True);im.save(p)
   r=subprocess.run([sys.executable,str(H/'tools/compile_assets.py'),str(root),str(inbox)],capture_output=True,text=True);self.assertEqual(r.returncode,0,r.stderr)
   fanim=json.loads((root/'sprite/julian#anim.fanim').read_text());self.assertEqual(set(fanim['anims']),{'idle','run','attack','skill','skill2','ult','hit','dead'});self.assertTrue(all(len(v['frames'])==9 for v in fanim['anims'].values()));
   with Image.open(root/'sprite/julian#sheet.png') as im:self.assertEqual(im.size,(1656,576))
   bolt=json.loads((root/'effects/julian_scythe#anim.fanim').read_text());self.assertIn('dir_15_8',bolt['anims']);self.assertEqual(len(list((root/'effects').glob('*#sheet.png'))),9)
if __name__=='__main__':unittest.main()
