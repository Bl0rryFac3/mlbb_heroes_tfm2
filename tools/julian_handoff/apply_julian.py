#!/usr/bin/env python3
"""Apply additive source/data changes to an extracted, verified 1.1.122 baseline.
Does NOT claim that sprites/VFX exist. Run compile_assets.py separately before release.
"""
import argparse,hashlib,json,pathlib,shutil
HERE=pathlib.Path(__file__).resolve().parents[1]
def main():
 a=argparse.ArgumentParser();a.add_argument('root',type=pathlib.Path);args=a.parse_args();r=args.root
 info=json.loads((r/'mod.mod_info').read_text());assert info['version']=='1.1.122','Use the verified 21-hero 1.1.122 baseline; do not overwrite another candidate.'
 expected=json.loads((HERE/'BASELINE_FILES.json').read_text())
 for rel,digest in expected.items():assert hashlib.sha256((r/rel).read_bytes()).hexdigest()==digest,f'Baseline differs: {rel}'
 for f in ['julian.rs','julian_tests.rs']:shutil.copyfile(HERE/'code'/f,r/'sdk/candidate'/f)
 p=r/'sdk/candidate/lib.rs';s=p.read_text();assert s.count('mod eudora;')==1 and s.count('eudora::register(&mut module);')==1;s=s.replace('mod eudora;','mod eudora;\nmod julian;').replace('eudora::register(&mut module);','eudora::register(&mut module);\n    julian::register(&mut module);');p.write_text(s)
 p=r/'sdk/candidate/zhask.rs';s=p.read_text();assert s.count('crate::eudora::tick_all(sim);')==1;p.write_text(s.replace('crate::eudora::tick_all(sim);','crate::eudora::tick_all(sim);\n        crate::julian::tick_all(sim);'))
 for p in [r/'sdk/candidate/Cargo.toml',r/'sdk/candidate/Cargo.lock']:
  s=p.read_text();assert 'version = "1.1.122"' in s;p.write_text(s.replace('version = "1.1.122"','version = "1.1.123"'))
 info['version']='1.1.123';info['last_updated']='2026-11-15';info['description']='22 HEROES · Julian source/art-integration candidate. Not a validated runtime until returned assets and release gates pass.'
 (r/'mod.mod_info').write_text(json.dumps(info,ensure_ascii=False,indent=1)+'\n')
 data={"id":"mlbb_julian","category":"Assassin","sprite":"asset/mlbb_heroes/sprite/julian","anim_prefix":"","skill_icons":[f"asset/mlbb_heroes/icons/julian_{n}" for n in ['skill','skill2','ult']],"stat":{"attack":82,"magic_power":50,"hp":760,"defence":26,"magic_resistance":22,"move_speed":1080,"hp_regen":6,"stack":0,"crit_chance":0},"growth":{"attack":4,"magic_power":7,"hp":100,"defence":3,"magic_resistance":3,"move_speed":4,"hp_regen":0,"stack":0,"crit_chance":0},"tags":["AP","Magic","Melee","CC"],"passive":{"passive_ref":"mlbb_heroes:julian_legacy","params":{}},"stack_skill_index":0,"view_projectiles":[],"view_buffs":[],"view_effects":[]}
 for action,effect,range_,duration in [('attack','basic',28000,24),('skill','scythe',70000,18),('skill2','sword',70000,18),('ult','chain',70000,20)]:
  data[action]={"action_name":action,"description":f"#asset/base/text/champion?description.mlbb_julian.{action}","duration":duration,"cooltime":60 if action=='attack' else 360,"start_timing":6,"cancelable":False,"range":range_,"casting_type":"Targeting" if action=='attack' else "Position","casting_target":"Enemy" if action=='attack' else "EnemyWithoutTower","attack_type":"BaseAttack" if action=='attack' else "Skill","can_use_with_move":False,"effect":{"type":"Native","effect_ref":f"mlbb_heroes:julian_{effect}"}}
 for asset in ['scythe','scythe_enhanced']:
  for direction in range(16):
   for frame in range(9):data['view_effects'].append({"name":f"julian_{asset}_{direction}_{frame}","type":"Animation","anim":f"asset/mlbb_heroes/effects/julian_{asset}","tag":f"dir_{direction}_{frame}","z":1,"is_follow":False})
 for asset in ['sword','sword_enhanced']:
  for direction in range(16):data['view_effects'].append({"name":f"julian_{asset}_{direction}","type":"Animation","anim":f"asset/mlbb_heroes/effects/julian_{asset}","tag":f"dir_{direction}","z":1,"is_follow":False})
 for frame in range(9):data['view_effects'].append({"name":f"julian_scythe_spin_{frame}","type":"Animation","anim":"asset/mlbb_heroes/effects/julian_scythe_spin","tag":f"phase_{frame}","z":0,"is_follow":False})
 for asset in ['chain_warning','chain_bind','chain_enhanced','chain_launch']:data['view_effects'].append({"name":f"julian_{asset}","type":"LoopAnimation" if asset=='scythe_spin' else "Animation","anim":f"asset/mlbb_heroes/effects/julian_{asset}","tag":"play","z":1 if asset=='chain_launch' else 0,"is_follow":False})
 (r/'champion/mlbb_julian.data_champion').write_text(json.dumps(data,indent=1)+'\n');(r/'sdk/champion/mlbb_julian.data_champion').write_text(json.dumps(data,indent=1)+'\n')
 p=r/'style/champion_view.champion_view';style=json.loads(p.read_text());style['entries']['mlbb_julian']={"face":{"x":0,"y":-34},"center":{"x":0,"y":-14},"banpick_center":{"x":0,"y":-10}};p.write_text(json.dumps(style,indent=1)+'\n')
 p=r/'text/champion.i18n';text=json.loads(p.read_text());english=json.loads((HERE/'ENGLISH_TEXT.json').read_text())
 localized=json.loads((HERE/'LOCALIZED_TEXT.json').read_text());localized['en']=english
 for locale in ['en','zh-hans','zh-hant','ko','ja','fr','ru','vi']:
  assert locale in text;text[locale]['description']['mlbb_julian']=localized[locale]['description'];text[locale]['skill_name']['mlbb_julian']=localized[locale]['skill_name']
 p.write_text(json.dumps(text,ensure_ascii=False,indent=1)+'\n')
 (r/'reports/JULIAN_LOCALIZATION_REVIEW.json').write_text(json.dumps({'locales':['en','zh-hans','zh-hant','ko','ja','fr','ru','vi'],'status':'draft translations; numeric/source structure can be validated, native-language quality not independently reviewed'},indent=2)+'\n')
 print('Applied source and bindings. Missing returned assets and validation are release blockers; no runtime ZIP was produced.')
if __name__=='__main__':main()
