"""Actual paired runtime/Ferric CPU admission; future completion pins are caller inputs."""
from pathlib import Path
import re

import layer_validation as V
import shared_cpu as B

CONTROLLER_SHA = 'c5fd9ef33af9f6bdae2adac608f69a7d88a969e1385210f8c65c371a86077d6f'
PROPOSALS = {
    'ferric': ('p228-projection-residual-mlp-ordered-ferric-v2', 'ca3245e948456d804f1f7624f8eb808324086068ac1b674013b4cb31b16e70b4'),
    'fe2o3': ('p228-projection-residual-mlp-ordered-runtime-v1', '0157f78f2104dfbbbe7531e7bf08d21ef001e6313c0b06676fb0d90ffbc62e19'),
}
PARENT = 'ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering'
PRIOR = (466118, 'deb2aeacded08c336d1fb5a1638cd2accc2b65ec65ffa3e90177bd5e61146d74')
SOURCE_SHA = 'ed98abb445b2dc3209fcd575bfb25e817b86b7bce0dbdb5f299984067149f3b4'
ADDED_COUNTS = dict(runtime=25, worker=30, parent_library=12, parent_binary=1)
BINARIES = {PARENT, B.PARENT, B.DEFAULT, B.PLAIN, B.WORKER}


def contract(value, root):
    V.require(value['schema'] == 'ferric-p228-projection-ordered-segment-cpu-result-v1'
        and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['empty_initial_target'] is True
        and value['complete_runtime_library_suite_selected'] is True, 'actual completed paired CPU qualification')
    V.require(len(value['phases']) == 75 and set(value['metadata']) == {'parent', 'worker'}
        and set(value['binaries']) == BINARIES, '75 actual leaves and five actual artifacts')
    for row in value['phases'].values():
        V.require(type(row['exit_code']) is int and row['exit_code'] == 0
            and row['reason'] is None and row['group_absent'] is True, 'natural CPU leaf')
    for key in ('compiler_requalified', 'gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority'):
        V.require(value[key] is False, 'CPU-only nonclaims')
    expected = B.SNAPSHOTS | {name + suffix for name in value['phases'] for suffix in
        ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}
    V.require(len(expected) == 382 and set(value['raw']) == expected
        and all(pin['path'] == str(root / name) for name, pin in value['raw'].items()), 'closed382 raw identities')
    additions = value['declared_test_additions']
    V.require(set(additions) == set(ADDED_COUNTS), 'four exact compiled addition groups')
    for key, count in ADDED_COUNTS.items():
        names = additions[key]
        V.require(type(names) is list and len(names) == count and names == sorted(set(names))
            and all(type(n) is str and re.fullmatch('[A-Za-z0-9_:]+', n) for n in names), 'sorted full test names')
    for row in value['tests'].values():
        V.require(type(row['passed']) is int and type(row['ignored']) is int
            and row['passed'] >= 0 and row['ignored'] >= 0
            and len(row['names']) == len(set(row['names'])) == row['passed'] + row['ignored']
            and row['summaries'] and all(len(s) == 3 and all(type(n) is int and n >= 0 for n in s)
                and s[1] == 0 for s in row['summaries'])
            and sum(s[0] for s in row['summaries']) == row['passed']
            and sum(s[2] for s in row['summaries']) == row['ignored'], 'named CPU outcomes')
    V.require(type(value['tests_passed']) is int and value['tests_passed'] == 1848
        and sum(r['passed'] for r in value['tests'].values()) == 1848
        and type(value['tests_ignored']) is int and value['tests_ignored'] == 7
        and sum(r['ignored'] for r in value['tests'].values()) == 7
        and value['tests']['runtime-tests']['summaries'] == [[922, 0, 3]]
        and value['tests']['worker-tests']['summaries'] == [[549, 0, 4], [13, 0, 0]],
        'actual full runtime and Ferric outcomes, never predicted completion')
    selected = {}
    for name in sorted(BINARIES):
        role = 'worker' if name == B.WORKER else 'parent'
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
                overflow_checks=True, debuginfo=0), 'actual qualified Cargo product')
        V.require(artifact['features'] == [] if role == 'worker' else 'tp-batch-engineering' in artifact['features'],
                  'separate executable feature gate')
        if name != B.PLAIN:
            selected[{PARENT: 'ordered', B.PARENT: 'shared', B.DEFAULT: 'default', B.WORKER: 'worker'}[name]] = row
    return selected


def test_delta(value, prior, plan, runtime_names):
    additions = plan['added_tests']
    V.require(value['declared_test_additions'] == additions, 'root plan matches actual declared additions')
    expected = {}
    consumed = set()
    for name, previous in prior['tests'].items():
        if name == 'worker-tests':
            added = additions['worker']
        elif name.startswith('parent-'):
            # Exact existing selectors are recorded as commands, supplied by sources().
            selector = plan['parent_selectors'][name]
            added = [n for n in additions['parent_library'] if selector in n]
            consumed.update(added)
        else:
            added = []
        old = set(previous['names'])
        V.require(not old.intersection(added), 'additive names do not replace old tests')
        expected[name] = (old | set(added), previous['ignored'])
    for i, name in enumerate(additions['parent_library']):
        if name not in consumed:
            expected['parent-added-' + str(i)] = ({name}, 0)
    V.require(len(runtime_names) == 900 and not runtime_names.intersection(additions['runtime']), 'actual full old runtime inventory')
    expected['runtime-tests'] = (runtime_names | set(additions['runtime']), 3)
    expected[PARENT + '-tests'] = (set(additions['parent_binary']), 0)
    V.require(set(value['tests']) == set(expected), 'closed exact selected test suite')
    for name, (names, ignored) in expected.items():
        V.require(set(value['tests'][name]['names']) == names
            and value['tests'][name]['ignored'] == ignored, 'old and added compiled tests ran: ' + name)


def sources(I, pins, value, root):
    V.require(value['controller']['path'] == str(I.E / 'p228-projection-ordered-segment-cpu-v2/run.py')
        and value['controller']['sha256'] == CONTROLLER_SHA
        and value['plan']['path'] == str(I.E / 'p228-projection-ordered-segment-cpu-v2/inputs.json'),
        'actual reviewed paired controller/plan')
    I.read(pins, value['controller']); plan = I.doc(pins, value['plan'])
    V.keys(plan, 'schema proposals added_tests parent_binary runtime_full_suite_cpu_reviewed')
    V.require(plan['schema'] == 'ferric-p228-projection-ordered-segment-cpu-inputs-v1'
        and plan['parent_binary'] == PARENT and plan['runtime_full_suite_cpu_reviewed'] is True
        and set(plan['proposals']) == set(PROPOSALS), 'closed reviewed paired CPU plan')
    proposals = {}
    for project, (directory, digest) in PROPOSALS.items():
        pin = plan['proposals'][project]
        V.require(pin['path'] == str(I.E / directory / 'source-manifest.json')
            and pin['sha256'] == digest, 'exact frozen source proposal')
        proposals[project] = I.doc(pins, pin)
    merged = {**proposals['fe2o3']['added_tests'], **proposals['ferric']['added_tests']}
    V.require(merged == plan['added_tests'] == value['declared_test_additions'], 'both exact source test manifests')
    base = I.doc(pins, value['raw']['sources-base.json'])
    V.require(value['raw']['sources-base.json']['sha256'] == SOURCE_SHA and len(base) == 7003,
              'actual CPU883 complete paired baseline')
    expected = dict(base); changed = set()
    for project, proposal in proposals.items():
        for row in proposal['files']:
            key = project + '/' + row['path']
            V.require((key not in base) if row['before'] is None else base.get(key) == row['before'], 'exact source preimage')
            expected[key] = row['after']
            if key.endswith('.rs'): changed.add(key)
    before = I.doc(pins, value['raw']['sources-before.json'])
    V.require(len(proposals['fe2o3']['files']) == 24 and len(proposals['ferric']['files']) == 36
        and len(expected) == 7018 and len(changed) == 59
        and I.doc(pins, value['raw']['sources-unformatted.json']) == expected
        and set(before) == set(expected) and all(before[k] == v for k, v in expected.items() if k not in changed)
        and before == I.doc(pins, value['raw']['sources-after.json']), 'restricted formatting and no source drift')
    for phase, names in (('worker-build', [B.WORKER]), ('parent-builds', sorted(BINARIES - {B.WORKER}))):
        raw = I.read(pins, value['raw'][phase + '-stdout'], 32 << 20)
        rows = [I.D.parse(line) for line in raw.splitlines() if line.strip()]
        V.require(I.hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256']
            and [r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True], 'successful actual build stream')
        artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        V.require(len(artifacts) == len(names) and {r['target']['name'] for r in artifacts} == set(names)
            and all(artifacts.count(value['binaries'][n]['artifact']) == 1 for n in names), 'exact five production products')
    return plan


def qualified(I, pins, pin, capture):
    root = Path(pin['path']).parent
    V.require(root.parent == I.E and Path(pin['path']).name == 'complete.json'
        and re.fullmatch(r'projection-ordered-segment-cpu-v228-v(?:[2-9]|[1-9][0-9]+)', root.name), 'root-pinned actual CPU receipt')
    value = I.doc(pins, pin); selected = contract(value, root)
    previous = value['prior_completion']
    V.require(previous == dict(path=str(I.E / 'projection-ar4-shared-host-cpu-v228-v1/complete.json'),
        bytes=PRIOR[0], sha256=PRIOR[1]), 'exact CPU883 predecessor')
    prior, _ = B.qualified(I, pins, previous, capture)
    plan = sources(I, pins, value, root)
    V.require(value['raw']['sources-base.json']['sha256'] == prior['raw']['sources-after.json']['sha256'],
              'same complete prior paired source')
    plan['parent_selectors'] = {}
    for name in prior['tests']:
        if name.startswith('parent-'):
            command = I.doc(pins, prior['raw'][name + '-command.json'])
            argv = command['argv']; plan['parent_selectors'][name] = argv[argv.index('--lib') + 1]
    records = [p for p in value['inputs'] if p['path'] == str(I.E / 'gfx950-clock-cpu-v228-v1/runtime-list-stdout')]
    V.require(len(records) == 1, 'actual historical complete runtime test inventory')
    raw = I.read(pins, records[0], 1 << 20).decode('utf-8')
    runtime_names = {line[:-6] for line in raw.splitlines() if line.endswith(': test')}
    V.require(all(re.fullmatch('[A-Za-z0-9_:]+', name) for name in runtime_names), 'exact runtime names')
    test_delta(value, prior, plan, runtime_names)
    return value, selected


def evidence(I, pins, plan, capture):
    V.require(plan['route'] in ('default', 'shared', 'ordered') and plan['parent_cpu'] == plan['worker_cpu'],
              'closed route and one qualified generation')
    value, selected = qualified(I, pins, plan['parent_cpu'], capture)
    return value, dict(parent=selected[plan['route']], worker=selected['worker'])
