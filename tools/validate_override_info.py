"""Reject malformed overrides before TFM2 disables the mod at startup."""
from pathlib import Path
import argparse,json

def validate(data):
 if not isinstance(data,dict):raise ValueError('override_info must be an object')
 for source,node in data.items():
  if not isinstance(node,dict):raise ValueError(f'{source}: route must be an object')
  if 'type'not in node:raise ValueError(f'{source}: missing required field type')
  # These are the two modes used by this project's shipped route contract.
  if node['type']not in ('override','merge'):raise ValueError(f'{source}: unsupported route type {node["type"]!r}')
  if not isinstance(node.get('remapping'),str)or not node['remapping'].startswith('asset/'):
   raise ValueError(f'{source}: missing/invalid remapping')
  if not source.startswith('asset/'):raise ValueError(f'{source}: invalid source asset')
 return len(data)

def main():
 p=argparse.ArgumentParser();p.add_argument('path',type=Path,nargs='?',default=Path(__file__).resolve().parents[1]/'mod.override_info');a=p.parse_args()
 try:count=validate(json.loads(a.path.read_text()))
 except (ValueError,TypeError)as e:raise SystemExit(f'FAIL override_info: {e}')
 print(f'PASS override_info: {count} routes, explicit type/remapping on every route')
if __name__=='__main__':main()
