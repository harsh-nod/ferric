#!/usr/bin/env python3
"""Exercise actual captured-data admission before requesting a GPU context."""
import hashlib
import json
import os
from pathlib import Path
import sys
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-model-matched-input-mlp-v228-v1'
SOURCE = ROOT / 'source'
OUT = E / 'guarded-mlp-model-matched-input-data-v228-v1'
assert os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
raw = (SOURCE / 'source-manifest.json').read_bytes()
assert hashlib.sha256(raw).hexdigest() == '4c3df6c7a6068681c9415caa4ded85ce5999c168f62d7da8d75d0ec366fc8bef'
manifest = json.loads(raw)
for name, expected in manifest['files'].items():
    body = (SOURCE / name).read_bytes()
    assert dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest()) == expected
sys.path.insert(0, str(SOURCE))
import run
import mlp
from common import Reader, encoded, save

OUT.mkdir(mode=0o700)
run.SOURCE, run.INPUTS = SOURCE, ROOT / 'inputs'
reader = Reader()
started = time.monotonic()
result = dict(passed=False, error=None, gpu_executed=False, model_executed=False,
              numerical_acceptance=False, performance_claim=False)
try:
    run.source_contract(reader)
    contract, bodies = run.load_inputs(reader)
    native, admission = mlp.native_inputs(bodies, contract)
    framework = mlp.framework_inputs(bodies, contract)
    result.update(native_admission=admission, original_bodies=len(bodies),
                  selected_native_parts=len(native), historical_control_stages=len(framework),
                  input_difference=mlp.D.compare_tensor(framework['input'], native[(0, 'post_normalized')]))
    assert not {'torch', 'transformers', 'numpy'} & set(sys.modules)
    result['passed'] = True
except BaseException as exc:
    result['error'] = type(exc).__name__ + ': ' + str(exc)
result['postcheck_errors'] = reader.postcheck()
result['passed'] = result['passed'] and not result['postcheck_errors']
result['input_pins'] = list(reader.pins.values())
result['elapsed_seconds'] = time.monotonic() - started
pin = save(OUT / 'complete.json', encoded(result))
print(json.dumps(dict(passed=result['passed'], error=result['error'], pin=pin)))
raise SystemExit(0 if result['passed'] else 1)
