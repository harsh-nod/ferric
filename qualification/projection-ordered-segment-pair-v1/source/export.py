"""Bounded original-path courier for a terminal ordered-segment pair, including failure."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import stat
import tarfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
PACKAGE_SHA = '054b11e3eae9389dad6dc42f216b2ccfc81115436340cab572fda7f93d8d4d69'
LEAVES = {'parent', *[f'{side}-{i}' for side in ('before', 'after') for i in range(3)]}
LEAF_NAMES = {'command.json', 'started.json', 'result.json', 'stdout', 'stderr'}
NATIVE = {'complete.json', 'child-stderr.bin', *[f'{kind}-{i}.{suffix}'
    for kind, suffix in (('observation', 'bin'), ('control', 'bin'), ('request', 'json')) for i in range(4)]}
INPUT_NAMES = {'plan.json', 'request.json', 'decode-review.json', 'assembly.json'}


def common(path):
    path = Path(path).absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical retained data helper')
    before = path.stat(); raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_mode, s.st_nlink)
    if stamp(before) != stamp(path.stat()) or hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact stable retained data helper')
    module = types.ModuleType('ordered_pair_courier_data'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return module


def filepins(value):
    if type(value) is dict:
        if set(value) == {'path', 'bytes', 'sha256'}:
            yield value
        else:
            for child in value.values():
                yield from filepins(child)
    elif type(value) is list:
        for child in value:
            yield from filepins(child)


def tree(X, root, allowed, directories, maximum):
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical evidence directory')
    found, total = set(), 0
    def failed(error):
        raise error
    for parent, dirs, files in os.walk(root, followlinks=False, onerror=failed):
        for name in dirs + files:
            path = Path(parent) / name; rel = str(path.relative_to(root)); row = path.lstat()
            X.require(row.st_uid == os.getuid(), 'owned evidence')
            if name in dirs:
                X.require(stat.S_ISDIR(row.st_mode) and rel in directories, 'closed ordinary evidence directories')
            else:
                X.require(rel in allowed and stat.S_ISREG(row.st_mode) and row.st_nlink == 1
                    and row.st_size <= 8 << 20, 'closed bounded evidence body')
                found.add(rel); total += row.st_size
                X.require(len(found) <= 128 and total <= maximum, 'bounded evidence tree')
    return found


def roster(X, terminal, digest):
    X.digest(digest)
    terminal = terminal.absolute()
    X.require(terminal.parent.parent == E and terminal.name in ('complete.json', 'failure.json')
        and re.fullmatch(r'prefix-projection-ordered-segment-pair-v228-v[1-9][0-9]{0,8}', terminal.parent.name),
        'explicit terminal pair receipt path')
    records, parsed, roots, linked, unparsed = {}, {}, {}, set(), {}

    def add(path, expected=None):
        path = Path(path); name = str(X.relative(str(path.relative_to(E))))
        X.require('/target/' not in '/' + name + '/' and path.stat().st_uid == os.getuid(), 'owned evidence only')
        raw = X.read(path, expected, cap=8 << 20)
        record = dict(path=str(path), **X.extent(raw))
        X.require(name not in records or records[name] == record, 'unchanged original-path body')
        records[name] = record
        if path.suffix == '.json':
            try:
                parsed[str(path)] = X.parse(raw)
            except (ValueError, UnicodeError) as error:
                unparsed[str(path)] = type(error).__name__
        if expected is not None:
            linked.add(str(path))
        X.require(len(records) <= 132 and sum(p['bytes'] for p in records.values()) <= 144 << 20,
            'bounded pair plus fixed inputs only')
        return raw

    raw = add(terminal)
    X.require(X.extent(raw)['sha256'] == digest, 'caller-pinned actual terminal receipt')
    value = X.parse(raw)
    X.require(value['schema'] == 'ferric-p228-projection-ordered-segment-pair-v1'
        and type(value['passed']) is bool and value['passed'] == (terminal.name == 'complete.json')
        and value['fixed_order'] == ['shared', 'ordered'] and value['retries'] == 0
        and value['each_arm_max_attempts'] == 1 and value['each_arm_max_seconds'] == 4300
        and value['pair_max_seconds'] == 8720 and type(value['failures']) is list
        and value['paired_comparison_performed'] is value['passed']
        and all(value[k] is False for k in ('independent_numerical_acceptance', 'gpu_time',
            'performance_claim', 'production_authority')), 'terminal pair scope, not success inference')
    X.require((value['passed'] and not value['failures'] and set(value['arms']) == {'shared', 'ordered'}
               and type(value['comparison']) is dict)
        or (not value['passed'] and bool(value['failures']) and set(value['arms']) <= {'shared', 'ordered'}),
        'honest complete versus failure record')
    roots[terminal.parent] = ({terminal.name}, set(), 8 << 20)
    X.require(tree(X, terminal.parent, *roots[terminal.parent]) == {terminal.name}, 'sole actual pair terminal')
    pair_plan = X.pin(value['plan'])
    X.require(Path(pair_plan['path']).parent == E and re.fullmatch(
        r'ordered-segment-pair-plan-v228-v[1-9][0-9]{0,8}[.]json', Path(pair_plan['path']).name), 'closed pair-plan path')
    plan = X.parse(add(Path(pair_plan['path']), pair_plan))
    X.require(set(plan) == {'schema', 'output_label', 'shared', 'ordered'}
        and plan['schema'] == 'ferric-p228-projection-ordered-segment-pair-inputs-v1'
        and plan['output_label'] == terminal.parent.name, 'actual closed pair plan')
    observations = {}
    for route in ('shared', 'ordered'):
        plan_pin = X.pin(plan[route]); inputs = Path(plan_pin['path']).parent
        X.require(inputs.parent == E and Path(plan_pin['path']).name == 'plan.json'
            and re.fullmatch(r'prefix-projection-ordered-segment-' + route + r'-inputs-v228-v[1-9][0-9]{0,8}', inputs.name),
            'closed route-specific four-file input namespace')
        arm_plan = X.parse(add(inputs / 'plan.json', plan_pin))
        X.require(arm_plan['schema'] == 'ferric-p228-projection-ordered-segment-inputs-v1'
            and arm_plan['route'] == route and re.fullmatch(
                r'prefix-projection-ordered-segment-' + route + r'-gpu-v228-v[1-9][0-9]{0,8}', arm_plan['output_label']),
            'explicit planned arm route')
        roots[inputs] = (INPUT_NAMES, set(), 8 << 20)
        X.require(tree(X, inputs, *roots[inputs]) == INPUT_NAMES, 'all eight retained assembly inputs')
        for name in sorted(INPUT_NAMES - {'plan.json'}):
            expected = arm_plan['request'] if name == 'request.json' else arm_plan['decode_review'] if name == 'decode-review.json' else None
            if expected is not None:
                X.require(X.pin(expected)['path'] == str(inputs / name), 'arm input FilePin namespace')
            add(inputs / name, expected)
        assembly = parsed[str(inputs / 'assembly.json')]
        X.require(assembly['plan'] == plan_pin and assembly['route'] == route
            and assembly['supervisor_manifest']['sha256'] == PACKAGE_SHA
            and assembly['source_postchecks_passed'] is True and assembly['new_gpu_execution'] is False,
            'existing actual assembly identity, not new approval')
        arm = E / arm_plan['output_label']
        sidecar = ('native-projection-shared-host-observation.json' if route == 'shared' else
            'native-projection-residual-mlp-ordered-observation.json')
        allowed = {'complete.json', 'failure.json', 'observation.json', 'host-observation.json', sidecar} | {
            f'{leaf}/{name}' for leaf in LEAVES for name in LEAF_NAMES} | {
            f'{leaf}-topology.json' for leaf in LEAVES - {'parent'}} | {'native/' + name for name in NATIVE}
        if not os.path.lexists(arm):
            X.require(route not in value['arms'] and not value['passed'], 'missing only unrecorded unsuccessful arm')
            observations[route] = dict(output=str(arm), directory_present=False, terminal=None, recorded_by_pair=False)
            continue
        roots[arm] = (allowed, LEAVES | {'native'}, 64 << 20)
        names = tree(X, arm, *roots[arm])
        X.require(not {'complete.json', 'failure.json'} <= names, 'no conflicting arm terminals')
        for name in sorted(names):
            add(arm / name)
        receipt_name = next((name for name in ('complete.json', 'failure.json') if name in names), None)
        receipt = records[str((arm / receipt_name).relative_to(E))] if receipt_name else None
        if route in value['arms']:
            X.require(receipt == X.pin(value['arms'][route]), 'actual pair-to-arm terminal pin')
            linked.add(receipt['path'])
        if receipt is not None and receipt['path'] in parsed:
            arm_value = parsed[receipt['path']]
            X.require(arm_value['schema'] == ('ferric-p228-projection-ar4-shared-host-gpu-v1' if route == 'shared' else
                'ferric-p228-projection-ordered-segment-gpu-v1')
                and arm_value['route'] == route and arm_value['plan'] == plan_pin
                and type(arm_value['passed']) is bool and arm_value['passed'] == (receipt_name == 'complete.json')
                and arm_value['supervisor_manifest']['sha256'] == PACKAGE_SHA, 'actual arm terminal family')
            if arm_value['passed']:
                expected_names = allowed - {'failure.json'}
                X.require(names == expected_names and len(names) == 59, 'closed successful arm evidence')
                for i in range(4):
                    X.require(records[str((arm / f'native/observation-{i}.bin').relative_to(E))]['bytes'] == 606976,
                        'four actual full payload extents')
        if value['passed']:
            X.require(receipt_name == 'complete.json' and route in value['arms'] and receipt['path'] in parsed,
                'two successful arms for successful pair')
        observations[route] = dict(output=str(arm), directory_present=True, files=len(names), terminal=receipt,
            terminal_json_parsed=receipt is not None and receipt['path'] in parsed,
            recorded_by_pair=route in value['arms'], payloads_present=[i for i in range(4) if f'native/observation-{i}.bin' in names])

    # Only pins inside the closed selected roots are replayed, never arbitrary input graphs.
    for document in parsed.values():
        for record in filepins(document):
            path = Path(record['path'])
            if path == Path(pair_plan['path']) or any(path.is_relative_to(root) for root in roots):
                normalized = dict(record)
                if type(record['sha256']) is list:
                    X.require(len(record['sha256']) == 32 and all(type(b) is int and 0 <= b <= 255 for b in record['sha256']),
                        'canonical Rust SHA256 byte array')
                    normalized['sha256'] = bytes(record['sha256']).hex()
                X.require(records.get(str(path.relative_to(E))) == X.pin(normalized), 'recorded evidence body drift or missing body')
                linked.add(str(path))
    X.require(not value['passed'] or not unparsed, 'successful pair has no truncated JSON body')
    unindexed = sorted(p['path'] for p in records.values() if p['path'] not in linked and p['path'] != str(terminal))
    return value, records, roots, observations, unindexed, unparsed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('actual_terminal', type=Path)
    parser.add_argument('actual_terminal_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper); out = args.archive.absolute()
    X.require(os.getuid() == os.geteuid() == 9661, 'original MI350 evidence owner')
    X.require(out.parent == E and re.fullmatch(r'projection-ordered-segment-pair-evidence-v228-v[1-9][0-9]{0,8}[.]tar[.]gz', out.name)
        and not os.path.lexists(out), 'fresh bounded pair evidence archive')
    for kind, cap in ((resource.RLIMIT_AS, 768 << 20), (resource.RLIMIT_CPU, 180),
                      (resource.RLIMIT_FSIZE, 160 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    value, records, roots, observations, unindexed, unparsed = roster(X, args.actual_terminal, args.actual_terminal_sha256)
    original_rosters = {root: {str(Path(p['path']).relative_to(root)) for p in records.values()
        if Path(p['path']).is_relative_to(root)} for root in roots}
    for root, limits in roots.items():
        X.require(tree(X, root, *limits) == original_rosters[root], 'collected exact roster before archive')
    absent = [Path(row['output']) for row in observations.values() if not row['directory_present']]
    X.require(all(not os.path.lexists(path) for path in absent), 'unstarted arms remain absent')
    manifest = dict(schema='ferric-p228-projection-ordered-segment-pair-export-v1', original_root=str(E),
        terminal=records[str(args.actual_terminal.absolute().relative_to(E))], recorded_pair_passed=value['passed'],
        recorded_pair_failures=value['failures'], arms=observations, files=records,
        collector_observed_without_incoming_terminal_pin=unindexed,
        unparsed_failed_json_bodies=unparsed,
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(X.read(Path(__file__).resolve()))),
        data_helper=dict(path=str(args.common_helper.absolute()), **X.extent(X.read(args.common_helper.absolute()))),
        partial_failure_bodies_preserved=True, native_payloads_retained_when_present=True,
        model_or_executable_or_source_tree_exported=False, terminal_pin_is_caller_supplied=True,
        process_quiescence_independently_observed=False, lifecycle_or_numerical_validators_replayed=False,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, record in sorted(records.items()):
            raw = X.read(Path(record['path']), record, cap=8 << 20)
            member = tarfile.TarInfo(name); member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json'); member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for record in records.values():
        X.read(Path(record['path']), record, cap=8 << 20)
    for root, limits in roots.items():
        X.require(tree(X, root, *limits) == original_rosters[root], 'evidence roster unchanged after export')
    X.require(all(not os.path.lexists(path) for path in absent), 'unstarted arms remain absent after export')
    for name in ('exporter', 'data_helper'):
        X.read(Path(manifest[name]['path']), manifest[name])
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out, cap=160 << 20))),
        files=len(records), recorded_pair_passed=value['passed'],
        uncompressed_bytes=sum(p['bytes'] for p in records.values())), sort_keys=True))


if __name__ == '__main__':
    main()

