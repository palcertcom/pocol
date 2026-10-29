#!/usr/bin/env python3
"""Explicit allowlists only: no network keys, private state or Android signing keys."""
import argparse, hashlib, json, shutil, subprocess
from pathlib import Path
p=argparse.ArgumentParser(); p.add_argument('--platform',choices=['Linux','Windows','Android'],required=True); p.add_argument('--out',type=Path,required=True); args=p.parse_args()
root=Path(__file__).resolve().parents[2]; args.out.mkdir()
if args.platform=='Android':
    files={'PoCol-testnet-arm64-debug.apk':root/'prototypes/pocol-android/app/build/outputs/apk/debug/app-debug.apk',
           'PoCol-testnet-arm64-unsigned.apk':root/'prototypes/pocol-android/app/build/outputs/apk/release/app-release-unsigned.apk'}
else:
    suffix='.exe' if args.platform=='Windows' else ''
    files={n+suffix:root/'prototypes/pocold/target/release'/(n+suffix) for n in ['pocol','pocold','pocol-key','pocol-sign','pocol-site']}
    for name in (['Start-PoCol.cmd'] if suffix else ['install-linux.sh']): files[name]=root/'deploy/standalone'/name
    info=subprocess.check_output([str(files['pocol'+suffix]),'build-info'])
    (args.out/'build-info.json').write_bytes(info)
for name,source in files.items(): shutil.copy2(source,args.out/name)
shutil.copy2(root/'deploy/standalone/README.md',args.out/'README.md')
(args.out/'source-commit.txt').write_text(subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()+'\n')
sums=''.join(f'{hashlib.sha256(f.read_bytes()).hexdigest()}  {f.name}\n' for f in sorted(args.out.iterdir()) if f.is_file())
(args.out/'SHA256SUMS').write_text(sums)
