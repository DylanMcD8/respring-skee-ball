#!/usr/bin/env python3
"""Run the mmap coverage tests against the production range-checking module."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--rustc', required=True)
args = parser.parse_args()
source = Path(__file__).resolve().parents[1] / 'compatibility-core/src/libc/sys/mman/ranges.rs'
with tempfile.TemporaryDirectory(prefix='skee-mmap-tests-') as directory:
    root = Path(directory)
    main = root / 'main.rs'
    main.write_text(f'#[path = {json.dumps(str(source))}] mod ranges;\n')
    binary = root / 'tests'
    subprocess.run([args.rustc, '--edition=2021', '--test', str(main), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
