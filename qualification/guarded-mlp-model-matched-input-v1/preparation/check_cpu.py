#!/usr/bin/env python3
"""Run the reviewed pure test gate on MI350 without creating a GPU container."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
SOURCE = E / 'guarded-mlp-model-matched-input-mlp-v228-v1/source'
ROOT = E / 'guarded-mlp-model-matched-input-tests-v228-v1'
assert os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
assert len(sys.argv) == 2
manifest_raw = (SOURCE / 'source-manifest.json').read_bytes()
assert hashlib.sha256(manifest_raw).hexdigest() == sys.argv[1]
manifest = json.loads(manifest_raw)
for name, expected in manifest['files'].items():
    raw = (SOURCE / name).read_bytes()
    assert dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest()) == expected
sys.path.insert(0, str(SOURCE))
import launch
from common import compact, encoded, load, read, save

ROOT.mkdir(mode=0o700)
for name in ('evidence', 'output', 'scratch'):
    (ROOT / name).mkdir(mode=0o700)
launch.ROOT = ROOT
owned = load(SOURCE / 'owned.py', manifest['files']['owned.py'], 'mlp_test_owned')
owned.subreaper()
started = time.monotonic()
error = None
try:
    launch.command(owned, 'tests', ['/usr/bin/taskset', '-c', '8,9', '/usr/bin/nice', '-n', '10',
                   '/usr/bin/python3', '-B', '-m', 'unittest', '-v', 'test_mlp'],
                   started + 300, 120, SOURCE)
    launch.test_census((ROOT / 'evidence/tests/stderr').read_bytes())
    for name, expected in manifest['files'].items():
        assert compact((SOURCE / name).read_bytes()) == expected
except BaseException as exc:
    error = type(exc).__name__ + ': ' + str(exc)
save(ROOT / 'complete.json', encoded(dict(passed=error is None, error=error,
    source_manifest=compact(manifest_raw), test_names=sorted(launch.TEST_NAMES),
    elapsed_seconds=time.monotonic() - started, gpu_executed=False)))
print(json.dumps(dict(passed=error is None, error=error, root=str(ROOT))))
raise SystemExit(0 if error is None else 1)
