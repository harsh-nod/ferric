"""Admission of a root-pinned completed observer CPU run, never a predicted ELF."""
from pathlib import Path
import re

import layer_validation as V

CONTROLLER_SHA = '12898f6b9aa1a1af122cf61d0393089fa38cdb898097114dbb67a36ea4fc780a'
PROPOSAL_SHA = 'ab5d055a09c320780a0d0da7d235bb0fe023469c9132b37a877fbc7841738074'
SOURCE_SHA = '22c23c3effbf79c1b38e8c816f43474cdc201aedb971d81bffd75c99dc5ba0a9'
PARENT = 'ferric-qwen3-finite-projection-residual-decode-host-engineering'
PLAIN = 'ferric-qwen3-finite-projection-residual-decode-engineering'
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
REPORT = 'projection_residual_decode_host_observation_v1::tests::'
SNAPSHOTS = {'sources-base.json', 'sources-unformatted.json', 'sources-before.json',
             'sources-after.json', 'old-targets-before.json', 'old-targets-after.json', 'configurations.json'}


def contract(value, root):
    V.require(value['schema'] == 'ferric-p228-projection-ar4-host-observation-cpu-result-v1'
        and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['empty_initial_target'] is True,
        'actual successful observer CPU qualification')
    V.require(len(value['phases']) == 61 and set(value['metadata']) == {'parent', 'worker'}
        and set(value['binaries']) == {PARENT, PLAIN, WORKER}, 'scoped61 phases and three actual artifacts')
    for row in value['phases'].values():
        V.require(type(row['exit_code']) is int and row['exit_code'] == 0
            and row['reason'] is None and row['group_absent'] is True, 'natural CPU leaf')
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority',
                'full_cpu1037_cohort_requalified', 'compiler_requalified'):
        V.require(value[key] is False, 'scoped CPU nonclaims')
    V.require(type(value['historical_runtime_tests_not_repeated']) is int
        and value['historical_runtime_tests_not_repeated'] == 208, 'runtime208 remains historical')
    expected = SNAPSHOTS | {name + suffix for name in value['phases'] for suffix in
        ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}
    V.require(len(expected) == 312 and set(value['raw']) == expected
        and all(pin['path'] == str(root / name) for name, pin in value['raw'].items()), 'actual312 raw identities')
    tests = value['tests']
    for row in tests.values():
        V.require(type(row['passed']) is int and type(row['ignored']) is int
            and row['passed'] >= 0 and row['ignored'] >= 0
            and len(row['names']) == len(set(row['names'])) == row['passed'] + row['ignored']
            and row['summaries'] and all(len(s) == 3 and all(type(n) is int and n >= 0 for n in s)
                and s[1] == 0 for s in row['summaries'])
            and sum(s[0] for s in row['summaries']) == row['passed']
            and sum(s[2] for s in row['summaries']) == row['ignored'], 'named CPU outcomes')
    V.require(type(value['tests_passed']) is int and value['tests_passed'] == 855
        and sum(r['passed'] for r in tests.values()) == 855
        and type(value['tests_ignored']) is int and value['tests_ignored'] == 4
        and sum(r['ignored'] for r in tests.values()) == 4
        and tests['worker-tests']['summaries'] == [[505, 0, 4], [13, 0, 0]], 'scoped855 and split worker summaries')
    selected = {}
    for name in (PARENT, PLAIN, WORKER):
        role = 'worker' if name == WORKER else 'parent'
        row = value['binaries'][name]; V.keys(row, 'artifact binary')
        artifact, pin = row['artifact'], row['binary']; V.keys(pin, 'path bytes sha256')
        source = root / 'sources/ferric/adapters' / (
            'tp-peer-finite-engineering-worker-v1' if role == 'worker' else 'm1-engineering-execution-v1')
        V.require(0 < V.uint(pin['bytes'], 128 << 20) and type(pin['sha256']) is str
            and re.fullmatch('[0-9a-f]{64}', pin['sha256'])
            and pin['path'] == str(root / 'target' / role / 'debug' / name)
            and artifact['reason'] == 'compiler-artifact' and artifact['target']['name'] == name
            and artifact['target']['kind'] == artifact['target']['crate_types'] == ['bin']
            and artifact['executable'] == pin['path'] and artifact['filenames'] == [pin['path']]
            and artifact['manifest_path'] == str(source / 'Cargo.toml')
            and artifact['target']['src_path'] == str(source / 'src' / (
                'main.rs' if role == 'worker' else 'bin/' + name + '.rs'))
            and artifact['profile'] == dict(test=False, opt_level='2', debug_assertions=True,
                overflow_checks=True, debuginfo=0), 'actual bounded observer Cargo product')
        V.require(artifact['features'] == [] if role == 'worker' else 'tp-batch-engineering' in artifact['features'],
                  'separate executable feature gate')
        if name != PLAIN:
            selected[role] = row
    return selected


def test_delta(value, prior, proposal):
    additions = proposal['added_tests']
    V.require(value['declared_test_additions'] == additions
        and set(additions) == {'worker', 'parent_library', 'parent_binary'}
        and [len(additions[k]) for k in ('worker', 'parent_library', 'parent_binary')] == [13, 12, 1],
        'exact additive observer test roster')
    omitted = {name for name in prior['tests'] if name != 'worker-tests'
               and not name.startswith(('parent-', 'ferric-'))}
    V.require(len(omitted) == 28 and sum(prior['tests'][k]['passed'] for k in omitted) == 208,
              'only old runtime selections omitted')
    new = {'parent-projection-host-report', PARENT + '-tests'}
    V.require(set(value['tests']) == (set(prior['tests']) - omitted) | new, 'all old parent/worker selections retained')
    for name in set(prior['tests']) - omitted:
        added = additions['worker'] if name == 'worker-tests' else (
            [n for n in additions['parent_library'] if n.startswith('tp_finite_client::')] if name == 'parent-client' else [])
        old = set(prior['tests'][name]['names'])
        V.require(not old.intersection(added) and set(value['tests'][name]['names']) == old | set(added)
            and value['tests'][name]['ignored'] == prior['tests'][name]['ignored'], 'unchanged old names and exact additions')
    V.require(set(value['tests']['parent-projection-host-report']['names']) ==
        {n for n in additions['parent_library'] if n.startswith(REPORT)}
        and set(value['tests'][PARENT + '-tests']['names']) == set(additions['parent_binary']), 'new report/bin selections')


def evidence(I, pins, plan, capture):
    pin = plan['parent_cpu']; root = Path(pin['path']).parent
    V.require(pin == plan['worker_cpu'] and root.parent == I.E and Path(pin['path']).name == 'complete.json'
        and re.fullmatch(r'projection-ar4-host-observation-cpu-v228-v[1-9][0-9]*', root.name), 'root-pinned actual observer receipt')
    value = I.doc(pins, pin); selected = contract(value, root)
    prior, _ = I.prior_cpu_evidence(pins, dict(parent_cpu=value['prior_completion'],
        worker_cpu=value['prior_completion']), capture)
    V.require(value['controller']['path'] == str(I.E / 'p228-projection-ar4-host-observation-cpu-v1/run.py')
        and value['controller']['sha256'] == CONTROLLER_SHA
        and value['proposal']['path'] == str(I.E / 'p228-projection-ar4-host-observation-v1/source-manifest.json')
        and value['proposal']['sha256'] == PROPOSAL_SHA, 'reviewed CPU controller and observer proposal')
    I.read(pins, value['controller']); proposal = I.doc(pins, value['proposal'])
    V.require(proposal['base_cpu'] == value['prior_completion'] and len(proposal['files']) == 17
        and proposal['schema'] == 'ferric-p228-projection-ar4-host-observation-source-proposal-v1', 'observer proposal lineage')
    test_delta(value, prior, proposal)
    base = I.doc(pins, value['raw']['sources-base.json'])
    V.require(value['raw']['sources-base.json']['sha256'] == SOURCE_SHA
        and base == I.doc(pins, prior['raw']['sources-after.json']), 'actual CPU1037 source preimage')
    expected = dict(base); changed = set()
    for row in proposal['files']:
        key = 'ferric/' + row['path']
        V.require((key not in base) if row['before'] is None else base.get(key) == row['before'], 'exact source preimage')
        expected[key] = row['after']
        if key.endswith('.rs'): changed.add(key)
    unformatted = I.doc(pins, value['raw']['sources-unformatted.json'])
    before = I.doc(pins, value['raw']['sources-before.json'])
    V.require(unformatted == expected and len(expected) == len(base) + 6 and len(changed) == 16
        and set(before) == set(expected) and all(before[k] == v for k, v in expected.items() if k not in changed)
        and before == I.doc(pins, value['raw']['sources-after.json']), 'bounded formatting and no compiled-source drift')
    for phase, names in (('worker-build', [WORKER]), ('parent-builds', [PLAIN, PARENT])):
        raw = I.read(pins, value['raw'][phase + '-stdout'], 32 << 20)
        rows = [I.D.parse(line) for line in raw.splitlines() if line.strip()]
        V.require(I.hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256']
            and [r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True], 'successful actual build stream')
        artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        V.require(len(artifacts) == len(names) and {r['target']['name'] for r in artifacts} == set(names)
            and all(artifacts.count(value['binaries'][n]['artifact']) == 1 for n in names), 'exact production build products')
    return value, selected
