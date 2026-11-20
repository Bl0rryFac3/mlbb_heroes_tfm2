import json,unittest
from pathlib import Path
from validate_override_info import validate
class OverrideSchemaTests(unittest.TestCase):
 def test_full_shipped_roster_has_required_tags(self):
  data=json.loads((Path(__file__).resolve().parents[1]/'mod.override_info').read_text())
  self.assertGreater(validate(data),400)
 def test_rejects_the_1147_granger_loading_regression(self):
  bad={'asset/base/sound/sfx/mlbb_granger_voice_select':{'remapping':'asset/mlbb_heroes/sound/sfx/mlbb_granger_voice_select'}}
  with self.assertRaisesRegex(ValueError,'missing required field type'):validate(bad)
 def test_merge_and_override_routes_are_supported(self):
  self.assertEqual(validate({'asset/base/text/champion':{'type':'merge','remapping':'asset/mlbb_heroes/text/champion'},'asset/base/sound/sfx/example':{'type':'override','remapping':'asset/mlbb_heroes/sound/sfx/example'}}),2)
 def test_rejects_a_tagged_route_without_a_destination(self):
  with self.assertRaisesRegex(ValueError,'remapping'):validate({'asset/base/sound/sfx/example':{'type':'override'}})
if __name__=='__main__':unittest.main()
