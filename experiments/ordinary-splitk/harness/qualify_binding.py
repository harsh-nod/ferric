"""Fixed 32 MiB G36 actual split-K binding; no native launch or source mutation."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
I = D / 'inputs/splitk-model-binding-a001'
Q = D / 'inputs/splitk-model-harness-a003/qualify.py'
Q_SHA = 'c3e5b5350dc1abaa7a7e76d292878356ad40735facabbfae72a631686a3f68c6'
CONFIG = I / 'actual-binding-inputs-a001.json'
CONFIG_SHA = '20888465c7e7443bfc9713f7eb94b489edc4ad5fb85c4020043e71e35bf3efea'
OUT = D / 'splitk-model-binding-a001'
LOGS = D / 'splitk-model-binding-logs-a001'


def main():
    assert Q.resolve(strict=True) == Q and hashlib.sha256(Q.read_bytes()).hexdigest() == Q_SHA
    spec = importlib.util.spec_from_file_location('fixed_splitk_qualifier', Q)
    q = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(q)
    q.environment()
    allocation_before = q.allocation(32 * 1024**2)
    files = q.payload()
    source_before = q.check_sources(files)
    raw, _ = q.read(CONFIG, CONFIG_SHA)
    config = json.loads(raw)
    definition = importlib.util.spec_from_file_location('fixed_actual_binder', q.ROOT / 'bind_build.py')
    binder = importlib.util.module_from_spec(definition)
    definition.loader.exec_module(binder)
    inputs = {}

    def visit(value):
        if isinstance(value, dict):
            if set(value) >= {'path', 'sha256'}:
                prior = inputs.setdefault(value['path'], value['sha256'])
                q.require(prior == value['sha256'], 'one path has conflicting bindings')
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(config)
    image = config['splitk']['image']
    inputs[str(Path(image['path']) / 'observation.hsaco')] = image['hsaco']
    inputs[str(Path(image['path']) / 'observation.json')] = image['manifest']

    def check_inputs():
        return {path: binder.c.read(path, digest, 512 * 1024**2, empty=True)[1]
                for path, digest in inputs.items()}

    inputs_before = check_inputs()
    LOGS.mkdir(mode=0o700)
    command = ['/usr/bin/python3', '-I', '-B', str(q.ROOT / 'bind_build.py'),
               '--config', str(CONFIG), '--config-sha256', CONFIG_SHA, '--output', str(OUT)]
    result, error = None, None
    try:
        with (LOGS / 'stdout').open('xb') as stdout, (LOGS / 'stderr').open('xb') as stderr:
            result = subprocess.run(command, stdout=stdout, stderr=stderr, timeout=1000, check=False)
        q.require(result.returncode == 0, 'actual binder refused')
        response = json.loads((LOGS / 'stdout').read_bytes())
        q.require(response['accepted'] is True and response['native_executed'] is False
                  and not (LOGS / 'stderr').read_bytes(), 'binding completion scope')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        source_after = q.check_sources(files)
        inputs_after = check_inputs()
        q.read(CONFIG, CONFIG_SHA)
        q.require(source_after == source_before and inputs_after == inputs_before,
                  'source or bound input changed')
        allocation_after = q.allocation()
        q.require(allocation_after['stage_allocated_bytes'] - allocation_before['stage_allocated_bytes']
                  <= 32 * 1024**2, 'actual binding growth exceeds fixed envelope')
        q.environment()
        receipt = {'schema': 'FerricSplitKActualBindingQualificationV1',
            'command': command, 'accepted': error is None, 'error': error,
            'returncode': None if result is None else result.returncode,
            'source_before': source_before, 'source_after': source_after,
            'inputs_before': inputs_before, 'inputs_after': inputs_after,
            'allocation_before': allocation_before, 'allocation_after': allocation_after,
            'native_executed': False}
        with (LOGS / 'receipt.json').open('x') as stream:
            json.dump(receipt, stream, sort_keys=True, indent=2)
            stream.write('\n')
        print(json.dumps(receipt, sort_keys=True), flush=True)
    return 0 if error is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
