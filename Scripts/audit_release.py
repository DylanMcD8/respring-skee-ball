#!/usr/bin/env python3
"""Check source/package hygiene and documentation assets."""
from pathlib import Path
import argparse,zipfile,plistlib,hashlib,io,json,struct
p=argparse.ArgumentParser();p.add_argument('--ipa',type=Path);a=p.parse_args()
r=Path(__file__).resolve().parents[1];d=json.loads((r/'distribution.json').read_text())
for f in r.rglob('*'):
 if any(x in f.parts for x in ('.git','.build','DerivedData','release-output')) or not f.is_file():continue
 assert f.suffix not in ('.mobileprovision','.p12','.p8','.key'),str(f)
 assert f.stat().st_size<100*1024*1024,str(f)
assert (r/'docs/images'/d['header']).is_file()
def check_zip(z):
 assert z.testzip() is None
 for n in z.namelist():
  assert n.startswith('Payload/'),n
  assert not any(x in ('_CodeSignature','SC_Info','embedded.mobileprovision','iTunesMetadata.plist','.DS_Store') or x.startswith('._') for x in Path(n).parts),n
with zipfile.ZipFile(r/d['original_archive']) as z:check_zip(z)
result={'source_hygiene':'pass','header_present':True,'execution':d['execution']}
if a.ipa:
 with zipfile.ZipFile(a.ipa) as z:
  check_zip(z);n=next(n for n in z.namelist() if n.count('/')==2 and n.endswith('/Info.plist'));info=plistlib.loads(z.read(n))
  assert info['CFBundleSupportedPlatforms']==['iPhoneOS']
  assert info['CFBundleDisplayName']==d['name']
  assert info['CFBundleVersion']==d['build']
  binary=z.read(n.rsplit('/',1)[0]+'/'+info['CFBundleExecutable'])
  assert struct.unpack_from('<II',binary)==(0xfeedfacf,0x100000c),'Expected ARM64 Mach-O'
  offset=32;platform=None
  for _ in range(struct.unpack_from('<I',binary,16)[0]):
   command,size=struct.unpack_from('<II',binary,offset)
   if command==0x32:platform=struct.unpack_from('<I',binary,offset+8)[0]
   offset+=size
  assert platform==2,'Expected iPhoneOS Mach-O, not a simulator executable'

  for n in z.namelist():
   if n.endswith('.ipa'):
    with zipfile.ZipFile(io.BytesIO(z.read(n))) as nested:check_zip(nested)
  result.update(version=info['CFBundleShortVersionString'],build=info['CFBundleVersion'],ipa_sha256=hashlib.sha256(a.ipa.read_bytes()).hexdigest(),unsigned_package_hygiene='pass')
print(json.dumps(result,indent=2))
