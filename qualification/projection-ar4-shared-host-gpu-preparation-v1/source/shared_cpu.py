"""Admission of the actual shared-full CPU generation, with route-specific real ELFs."""
from pathlib import Path
import re

import layer_validation as V
import observer_cpu as B

CONTROLLER_SHA = '1f9fc61f35cc2d8367592ecf29ff7ea48e6d6250556a72940abe3acb028a41c5'
PROPOSAL_SHA = 'faaece5ab1afea85b7b6f03b02d772847889565a0ae70922180d6f62a1770461'
SOURCE_SHA = '1d173129c684afe5bcdc009ec939f8e4f0371bad8a4edecdf62b4548cb04f920'
DEFAULT = 'ferric-qwen3-finite-projection-residual-decode-host-engineering'
PARENT = 'ferric-qwen3-finite-projection-residual-decode-shared-host-engineering'
PLAIN = 'ferric-qwen3-finite-projection-residual-decode-engineering'
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
REPORT = 'projection_residual_decode_host_observation_v1::tests::'
SNAPSHOTS = {'sources-base.json', 'sources-unformatted.json', 'sources-before.json',
             'sources-after.json', 'old-targets-before.json', 'old-targets-after.json', 'configurations.json'}


def contract(value, root):
    V.require(value['schema'] == 'ferric-p228-projection-ar4-shared-host-cpu-result-v1'
        and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['empty_initial_target'] is True,
        'actual successful observer CPU qualification')
    V.require(len(value['phases']) == 63 and set(value['metadata']) == {'parent', 'worker'}
        and set(value['binaries']) == {PARENT, DEFAULT, PLAIN, WORKER}, 'scoped63 phases and four actual artifacts')
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
    V.require(len(expected) == 322 and set(value['raw']) == expected
        and all(pin['path'] == str(root / name) for name, pin in value['raw'].items()), 'actual322 raw identities')
    tests = value['tests']
    for row in tests.values():
        V.require(type(row['passed']) is int and type(row['ignored']) is int
            and row['passed'] >= 0 and row['ignored'] >= 0
            and len(row['names']) == len(set(row['names'])) == row['passed'] + row['ignored']
            and row['summaries'] and all(len(s) == 3 and all(type(n) is int and n >= 0 for n in s)
                and s[1] == 0 for s in row['summaries'])
            and sum(s[0] for s in row['summaries']) == row['passed']
            and sum(s[2] for s in row['summaries']) == row['ignored'], 'named CPU outcomes')
    V.require(type(value['tests_passed']) is int and value['tests_passed'] == 883
        and sum(r['passed'] for r in tests.values()) == 883
        and type(value['tests_ignored']) is int and value['tests_ignored'] == 4
        and sum(r['ignored'] for r in tests.values()) == 4
        and tests['worker-tests']['summaries'] == [[519, 0, 4], [13, 0, 0]], 'scoped883 and split worker summaries')
    selected = {}
    for name in (PARENT, DEFAULT, PLAIN, WORKER):
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
            selected['worker' if role == 'worker' else ('shared' if name == PARENT else 'default')] = row
    return selected


def test_delta(value, prior, proposal):
    additions = proposal['added_tests']
    V.require(value['declared_test_additions'] == additions
        and set(additions) == {'worker', 'parent_library', 'parent_binary'}
        and [len(additions[k]) for k in ('worker', 'parent_library', 'parent_binary')] == [14, 13, 1],
        'exact shared-full test additions')
    V.require(set(value['tests']) == set(prior['tests']) | {PARENT + '-tests'},
              'all scoped baseline test selections retained')
    for name, previous in prior['tests'].items():
        if name == 'worker-tests':
            added = additions['worker']
        elif name == 'parent-client':
            added = [n for n in additions['parent_library'] if n.startswith('tp_finite_client::')]
        elif name == 'parent-projection-host-report':
            added = [n for n in additions['parent_library'] if n.startswith(REPORT)]
        else:
            added = []
        old = set(previous['names'])
        V.require(not old.intersection(added) and set(value['tests'][name]['names']) == old | set(added)
            and value['tests'][name]['ignored'] == previous['ignored'], 'same old names and exact additions')
    V.require(set(value['tests'][PARENT + '-tests']['names']) == set(additions['parent_binary']),
              'new shared-parent binary selection')


def qualified(I, pins, pin, capture):
    root = Path(pin['path']).parent
    V.require(root.parent == I.E and Path(pin['path']).name == 'complete.json'
        and re.fullmatch(r'projection-ar4-shared-host-cpu-v228-v[1-9][0-9]*', root.name), 'root-pinned actual observer receipt')
    value = I.doc(pins, pin); selected = contract(value, root)
    V.require(value['prior_completion'] == dict(path=str(I.E / 'projection-ar4-host-observation-cpu-v228-v1/complete.json'),
        bytes=488734, sha256='7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c'),
        'actual CPU855 predecessor, not an interchangeable generation')
    prior, _ = B.evidence(I, pins, dict(parent_cpu=value['prior_completion'],
        worker_cpu=value['prior_completion']), capture)
    proposal = sources(I, pins, value, root)
    V.require(proposal['base_sources_after'] == prior['raw']['sources-after.json'], 'baseline source map join')
    test_delta(value, prior, proposal)
    return value, selected


def sources(I, pins, value, root):
    V.require(value['controller']['path'] == str(I.E / 'p228-projection-ar4-shared-host-cpu-v1/run.py')
        and value['controller']['sha256'] == CONTROLLER_SHA
        and value['proposal']['path'] == str(I.E / 'p228-projection-ar4-shared-host-v1/source-manifest.json')
        and value['proposal']['sha256'] == PROPOSAL_SHA, 'reviewed CPU controller and observer proposal')
    I.read(pins, value['controller']); proposal = I.doc(pins, value['proposal'])
    V.require(proposal['base_cpu'] == value['prior_completion'] and len(proposal['files']) == 14
        and proposal['schema'] == 'ferric-p228-projection-ar4-shared-host-source-proposal-v1', 'shared-full proposal lineage')
    V.require(proposal['shared_runtime_options'] == [False, False, True]
        and all(type(n) is bool for n in proposal['shared_runtime_options'])
        and proposal['original_group_fence_policy_unchanged'] is False
        and proposal['fresh_full_currentness_preserved'] is True
        and proposal['configuration_time_in_snapshots'] is False, 'reviewed explicit shared-full policy')
    base = I.doc(pins, value['raw']['sources-base.json'])
    V.require(value['raw']['sources-base.json']['sha256'] == SOURCE_SHA
        and proposal['base_sources_after'] == dict(path=str(I.E / 'projection-ar4-host-observation-cpu-v228-v1/sources-after.json'),
            bytes=1269472, sha256=SOURCE_SHA)
        and base == I.doc(pins, proposal['base_sources_after']), 'actual CPU855 source preimage')
    expected = dict(base); changed = set()
    for row in proposal['files']:
        key = 'ferric/' + row['path']
        V.require((key not in base) if row['before'] is None else base.get(key) == row['before'], 'exact source preimage')
        expected[key] = row['after']
        if key.endswith('.rs'): changed.add(key)
    unformatted = I.doc(pins, value['raw']['sources-unformatted.json'])
    before = I.doc(pins, value['raw']['sources-before.json'])
    V.require(unformatted == expected and len(expected) == len(base) + 4 and len(changed) == 13
        and set(before) == set(expected) and all(before[k] == v for k, v in expected.items() if k not in changed)
        and before == I.doc(pins, value['raw']['sources-after.json']), 'bounded formatting and no compiled-source drift')
    for phase, names in (('worker-build', [WORKER]), ('parent-builds', [PLAIN, DEFAULT, PARENT])):
        raw = I.read(pins, value['raw'][phase + '-stdout'], 32 << 20)
        rows = [I.D.parse(line) for line in raw.splitlines() if line.strip()]
        V.require(I.hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256']
            and [r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True], 'successful actual build stream')
        artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        V.require(len(artifacts) == len(names) and {r['target']['name'] for r in artifacts} == set(names)
            and all(artifacts.count(value['binaries'][n]['artifact']) == 1 for n in names), 'exact production build products')
    return proposal


def evidence(I, pins, plan, capture):
    V.require(plan['route'] in ('default', 'shared') and plan['parent_cpu'] == plan['worker_cpu'],
              'closed route and one qualified generation')
    value, selected = qualified(I, pins, plan['parent_cpu'], capture)
    return value, dict(parent=selected[plan['route']], worker=selected['worker'])
