"""Publish exact tested sources and the actual conditional historical MLP result."""
import hashlib
import json
import os
from pathlib import Path
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/'
REF = '/home/harmenon/ferric-asrock-42/evidence/resident-layer-tp2-v218/reference/'
Q = Path('/home/harsh/ferric-p227-integration/qualification/historical-mlp-capture-replay-v1')


def checked(path, pin):
    raw = path.read_bytes()
    if len(raw) != pin['bytes'] or hashlib.sha256(raw).hexdigest() != pin['sha256']:
        raise ValueError('retained bytes differ: ' + str(path))
    return raw


def main():
    if sys.flags.optimize or 'PYTHONOPTIMIZE' in os.environ:
        raise RuntimeError('ordinary Python required for publication checks')
    records = {}
    for label, size, sha, kind in (
        ('historical-mlp-pure-v228-v1', 1536, '3c849835909ac7e67e012bbc87235787a94dad5a13daaaea9e3e983766e9e51c', 'pure'),
        ('historical-mlp-replay-v228-v1', 1417, 'c452d70ba5fe9bef423246ac7c03dbdd9b9db3680102e1e9b9db9a84d5ec4e72', 'actual')):
        record = dict(path=E + label + '/complete.json', bytes=size, sha256=sha)
        raw = checked(L / label / 'complete.json', record)
        value = json.loads(raw)
        assert value['passed'] is True and value['source_postchecks_passed'] is True
        assert value['gpu_execution'] is value['full_model_acceptance'] is value['performance_claim'] is False
        records[kind + '/complete.json'] = raw
        for key in ('sources_before', 'sources_after'):
            pin = value[key]
            records[kind + '/' + Path(pin['path']).name] = checked(L / label / Path(pin['path']).name, pin)
        assert records[kind + '/sources-before.json'] == records[kind + '/sources-after.json']
        if kind == 'pure':
            tests = value['details']
            assert (tests['tests'], tests['adapter_tests'], tests['original_arithmetic_tests']) == (41, 22, 19)
            assert tests['errors'] == tests['failures'] == tests['skipped'] == 0
            records['pure/tests.log'] = checked(L / label / 'tests.log', tests['transcript'])
            sources = json.loads(records['pure/sources-before.json'])
        else:
            records['actual/mlp.json'] = checked(L / label / 'mlp.json', value['details']['historical_result'])
        records['run_cpu.py'] = checked(L / 'run_historical_mlp_p228_v1.py', value['controller'])
    result = json.loads(records['actual/mlp.json'])
    assert result['passed'] is True and result['conditional_stage_checks_passed'] is True
    assert result['new_v7_image_checked'] is result['full_layer_acceptance'] is result['full_model_acceptance'] is False
    assert result['original_status'] == 'FAILED_UNCHANGED'
    assert [p['profile'] for p in result['profiles']] == ['baseline', 'candidate']
    for profile in result['profiles']:
        assert [r['rank'] for r in profile['ranks']] == [0, 1]
        for rank in profile['ranks']:
            assert set(rank['stages']) == {'norm', 'gate', 'up', 'swiglu', 'down'}
            assert all(s['mismatches'] == 0 for s in rank['stages'].values())
            assert rank['stages']['down']['max_bound_ratio'] <= 1
        capture = result['original_captures'][profile['profile']]
        raw = checked(L / capture['path'][len(E):], capture)
        for span in result['capture_slices'][profile['profile']]:
            part = raw[span['offset']:span['offset'] + span['bytes']]
            assert len(part) == span['bytes'] and hashlib.sha256(part).hexdigest() == span['sha256']
    for name in ('run.py', 'test_run.py', 'README.md'):
        records['source/' + name] = checked(L / 'proposals/p228-historical-mlp-capture-replay-v1' / name,
            sources[E + 'p228-historical-mlp-capture-replay-v1/' + name])
    for name in ('reference.py', 'extract.py', 'policy.json', 'test_reference.py'):
        records['reference/' + name] = checked(L / 'proposals/p218-reference' / name, sources[REF + name])
    oracle = next(p for path, p in sources.items() if '/tp2-residual-reference/' in path)
    records['reference/residual_oracle.py'] = checked(
        L / 'proposals/p228-independent-layer-reference-v1/helpers/residual_oracle.py', oracle)
    if Q.exists():
        assert Q.is_dir() and not Q.is_symlink(), 'canonical existing publication'
    else:
        Q.mkdir()
    for name, raw in records.items():
        destination = Q / name
        destination.parent.mkdir(exist_ok=True)
        if destination.exists():
            assert not destination.is_symlink() and destination.read_bytes() == raw, 'unchanged publication'
        else:
            with destination.open('xb') as stream:
                stream.write(raw)
    print(json.dumps(dict(published_files=len(records), checks=20, captures_rehashed=2,
                         capture_slices_rehashed=sum(map(len, result['capture_slices'].values())))))


if __name__ == '__main__':
    main()
