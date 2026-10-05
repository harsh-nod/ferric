"""Publish actual ordered-segment runtime preparation without executing evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
OUT = F / 'qualification/projection-ordered-segment-runtime-v1'
CPU = dict(path=str(E / 'projection-ordered-segment-cpu-v228-v2/complete.json'), bytes=624332,
    sha256='6ac67053d9b7d5d15772b5e4071a09933f013b6efa27394c31eb2b8279af884f')
PACKAGE_SHA = '054b11e3eae9389dad6dc42f216b2ccfc81115436340cab572fda7f93d8d4d69'
PURE_SHA = '000fb1158483b2a0255d85743da2c9c35e59fd2e6ea5d5c7a3dbc54401dfa4d5'
BINARIES = {
    'shared-parent': ('parent', 'ferric-qwen3-finite-projection-residual-decode-shared-host-engineering',
        14028632, 'a5bcd94625bf58844ccce15588aaaef1dfbe79a03132968c96598e4110d8aa7e'),
    'ordered-parent': ('parent', 'ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering',
        14029072, '0cb1806bc90654efb64871041123a8ece5b0465d2f1adcc5acb8d65b53517340'),
    'worker': ('worker', 'ferric-tp-peer-finite-engineering-worker-v1',
        5381704, '7f9f1bbe8c1bb88c2c6330311e4094f9c10cecfdbc50d5b9b82680ee4b3bb5be'),
}
AUDIT_NAMES = {'complete.json', 'inputs.json', 'software.json', 'topology-before.json', 'topology-after.json'} | {
    f'{leaf}/{name}' for leaf in ('readelf', 'ldd')
    for name in ('command.json', 'started.json', 'owner.json', 'audit.json', 'stdout', 'stderr')}
ASSEMBLY_NAMES = {'assembly.json', 'request.json', 'decode-review.json', 'plan.json'}
ROOT_NAMES = {route + suffix for route in ('shared', 'ordered')
    for suffix in ('-config.json', '-notes.json', '-parent-review.json')} | {'worker-review.json'}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def pin(path, body):
    return dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical retained path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= 4 << 20, 'bounded ordinary body')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as stream:
        opened = os.fstat(stream.fileno())
        body = stream.read((4 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_uid, s.st_gid, s.st_nlink,
                       s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(opened) == stamp(after) == stamp(path.lstat())
            and len(body) == before.st_size, 'stable retained body')
    return body


def parse(body):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(body, object_pairs_hook=pairs,
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def filepins(value):
    if isinstance(value, dict):
        if set(value) == {'path', 'bytes', 'sha256'}:
            yield value
        else:
            for child in value.values():
                yield from filepins(child)
    elif isinstance(value, list):
        for child in value:
            yield from filepins(child)


class Copies:
    def __init__(self):
        self.bodies, self.ledger, self.checked = {}, {}, {}

    def add(self, local, destination, original=None, expected=None):
        body = read(local)
        actual = pin(original or local, body)
        if expected is not None:
            require(actual == expected, 'exact retained FilePin: ' + str(local))
        require(destination not in self.bodies, 'unique publication destination')
        self.bodies[destination] = body
        self.checked[local] = pin(local, body)
        self.ledger[destination] = dict(original=actual, retained=pin(local, body))
        require(sum(map(len, self.bodies.values())) <= 32 << 20, 'bounded publication total')
        return actual, parse(body) if local.suffix == '.json' else None

    def tree(self, label, names, destination, terminal, digest):
        root = W / label
        require({str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()} == names,
                'exact retained file roster: ' + label)
        pins, values = {}, {}
        for name in sorted(names):
            record, value = self.add(root / name, destination + '/' + name, E / label / name)
            pins[record['path']] = record
            if value is not None:
                values[name] = value
        completion = pins[str(E / label / terminal)]
        require(completion['sha256'] == digest, 'caller-pinned actual terminal')
        linked = set()
        for value in values.values():
            for record in filepins(value):
                if record['path'].startswith(str(E / label) + '/'):
                    require(pins.get(record['path']) == record, 'internal retained body join')
                    linked.add(record['path'])
        return completion, pins, values, linked


def audit(copies, role, label, digest):
    require(re.fullmatch('projection-ordered-segment-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', label),
            'closed actual audit namespace')
    completion, pins, values, linked = copies.tree(label, AUDIT_NAMES, 'audits/' + role, 'complete.json', digest)
    value, inputs = values['complete.json'], values['inputs.json']
    require(value['schema'] == 'ferric-p227-prefix-runtime-audit-v1'
        and value['authority'] == 'none' and value['reviewed'] is False
        and all(value[k] is False for k in ('gpu_execution', 'numerical_acceptance',
            'production_authority', 'runtime_premises_discharged')), 'audit nonauthority flags')
    target, name, size, sha = BINARIES[role]
    expected_binary = dict(path=str(Path(CPU['path']).parent / 'target' / target / 'debug' / name), bytes=size, sha256=sha)
    require(len(linked) == 15 and value['binary'] == inputs['binary'] == expected_binary,
            'selected new ELF metadata and 15 internal audit pins')
    require(inputs['source_pins'][CPU['path']] == CPU, 'actual CPU1848 generation')
    expected_sources = dict(inputs['source_pins'])
    for row in value['libraries']:
        expected_sources[row['resolved']['path']] = row['resolved']
    require(value['source_pins'] == expected_sources and value['tools'] == inputs['tools']
        and len(value['libraries']) == (3 if role == 'worker' else 4), 'recorded resolved-library closure')
    outcomes = []
    for index, leaf in enumerate(('readelf', 'ldd')):
        command, owner = values[leaf + '/command.json'], values[leaf + '/owner.json']
        result, started = values[leaf + '/audit.json'], values[leaf + '/started.json']
        argv = ['/usr/bin/readelf', '-d', value['binary']['path']] if leaf == 'readelf' else ['/usr/bin/ldd', value['binary']['path']]
        require(result['argv'] == command['audit_argv'] == argv
            and command['argv'] == ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30', '--fsize=1048576', '--core=0', '--', *argv]
            and result['exit_code'] == 0 and result['deadline_seconds'] == command['deadline_seconds'] == 30
            and command['gpu_execution_requested'] is False, 'exact bounded audit command')
        outcome = owner['outcome']
        require(owner['gpu_execution'] is False and outcome['exit_code'] == 0 and outcome['reason'] is None
            and outcome['cleanup_signalled'] is False and outcome['owned_groups_absent'] is True
            and outcome['owned_processes_reaped'] is True, 'natural owned audit success')
        require(owner['command'] == value['commands'][index] and value['owners'][index] == pins[str(E / label / leaf / 'owner.json')]
            and owner['started'] == pins[str(E / label / leaf / 'started.json')]
            and started['parent']['ppid'] == started['supervisor_pid']
            and any(row['identity'] == started['parent'] and row['event'] == 'owned' for row in outcome['lineage']),
            'audit ownership record joins')
        outcomes.append(outcome)
    return dict(completion=completion, binary=value['binary'], libraries=value['libraries'],
        host=value['host'], boot_id=value['boot_id'], topology_identity=value['topology_identity'],
        readelf=value['readelf'], ldd=value['ldd'], natural_outcomes=outcomes)


def assembly(copies, route, label, digest, audits, roots):
    require(re.fullmatch('prefix-projection-ordered-segment-' + route + r'-inputs-v228-v[1-9][0-9]{0,8}', label),
            'closed actual assembly namespace')
    actual, _, values, _ = copies.tree(label, ASSEMBLY_NAMES, 'inputs/' + route, 'assembly.json', digest)
    value, plan, request, review = (values[name] for name in ('assembly.json', 'plan.json', 'request.json', 'decode-review.json'))
    config, notes = roots[route + '-config.json'][1], roots[route + '-notes.json'][1]
    require(value['schema'] == 'ferric-p228-projection-ordered-segment-root-assembly-v1'
        and plan['schema'] == 'ferric-p228-projection-ordered-segment-inputs-v1'
        and value['route'] == plan['route'] == config['route'] == route
        and config['input_label'] == label and config['output_label'] == plan['output_label']
        and value['source_postchecks_passed'] is True and value['copied_root_decisions'] is True
        and all(value[k] is False for k in ('automatic_approval', 'new_gpu_execution', 'new_compiler_execution',
            'numerical_acceptance', 'performance_claim', 'production_authority', 'all_transitive_compiler_inputs_rehashed',
            'compiler_binary_bodies_replayed', 'fresh_runtime_audits_performed')), 'completed preparation only')
    require(value['configuration'] == roots[route + '-config.json'][0]
        and value['root_notes'] == roots[route + '-notes.json'][0] and notes['configuration'] == config
        and value['selected_cpu'] == config['cpu'] == plan['parent_cpu'] == plan['worker_cpu'] == CPU
        and value['supervisor_manifest']['sha256'] == PACKAGE_SHA
        and value['supervisor_tests'] == config['supervisor_tests'] == plan['supervisor_tests']
        and config['supervisor_tests']['sha256'] == PURE_SHA, 'actual generation/package/root inputs')
    for role, audit_role, review_name in (('parent', route + '-parent', route + '-parent-review.json'), ('worker', 'worker', 'worker-review.json')):
        require(plan[role] == value['selected_runtime'][role] == audits[audit_role]['binary']
            and plan[role + '_runtime_review'] == config[role + '_runtime_review'] == roots[review_name][0], 'fresh selected runtime join')
        runtime = roots[review_name][1]
        require(all(runtime[key] == audits[audit_role][key] for key in ('binary', 'host', 'boot_id', 'readelf', 'ldd', 'libraries')),
            'root runtime review preserves actual audit identities')
    changed = ['decode.worker', 'decode.session', 'decode.evidence_directory'] + (['schema'] if route == 'ordered' else [])
    require(value['plan'] == value['outputs']['plan.json'] and plan['request'] == value['outputs']['request.json']
        and plan['decode_review'] == value['outputs']['decode-review.json'] and value['changed_request_fields'] == changed
        and review['schema'] == 'ferric-p228-projection-ordered-segment-engineering-review-v1'
        and review['review_topics'] == notes['review_topics'] and review['notes'] == notes['notes']
        and review['route'] == route and review['gpu_attempts'] == notes['gpu_attempts'] == 1,
        'verbatim root decision and four-file assembly joins')
    expected_schema = 'FerricFiniteProjectionResidualMlpOrderedRequestV1' if route == 'ordered' else 'FerricFiniteProjectionResidualDecodeRequestV1'
    require(request['schema'] == expected_schema and request['decode']['mode'] == 'autoregressive'
        and request['decode']['session'] == list(bytes.fromhex(config['session']))
        and request['decode']['device_ids'] == [16366993098680759275, 10838076764495710945], 'lossless own-output AR4 inputs')
    return dict(completion=actual, plan=value['plan'], request=plan['request'], selected_runtime=value['selected_runtime'],
        session=config['session'], output_label=plan['output_label'], recorded_source_postchecks=True,
        root_decisions_copied_without_new_approval=True)


def selections(rows, expected):
    require(len(rows) == len(expected) and {row[0] for row in rows} == expected, 'exact caller-pinned role roster')
    for _, _, digest in rows:
        require(re.fullmatch('[0-9a-f]{64}', digest), 'explicit actual digest')
    return {role: (label, digest) for role, label, digest in rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--audit', nargs=3, action='append', required=True, metavar=('ROLE', 'LABEL', 'SHA'))
    parser.add_argument('--assembly', nargs=3, action='append', required=True, metavar=('ROUTE', 'LABEL', 'SHA'))
    parser.add_argument('--root-inputs', type=Path, required=True)
    parser.add_argument('--primary-observation', nargs=2, action='append', default=[], metavar=('PATH', 'SHA'))
    args = parser.parse_args()
    audit_pins = selections(args.audit, set(BINARIES))
    assembly_pins = selections(args.assembly, {'shared', 'ordered'})
    require(args.root_inputs.is_absolute() and re.fullmatch(r'p228-projection-ordered-segment-root-inputs-v[1-9][0-9]{0,8}', args.root_inputs.name),
            'explicit retained root-input namespace')
    copies = Copies()
    audits = {role: audit(copies, role, *audit_pins[role]) for role in sorted(BINARIES)}
    require(len({(v['host'], v['boot_id']) for v in audits.values()}) == 1
        and all(v['topology_identity'] == audits['worker']['topology_identity'] for v in audits.values()), 'same recorded runtime platform')
    roots = {name: copies.add(args.root_inputs / name, 'root-inputs/' + name, E / args.root_inputs.name / name)
             for name in sorted(ROOT_NAMES)}
    assemblies = {route: assembly(copies, route, *assembly_pins[route], audits, roots) for route in ('shared', 'ordered')}
    require(assemblies['shared']['session'] != assemblies['ordered']['session'], 'distinct actual arm sessions')
    observations = []
    for index, (name, digest) in enumerate(args.primary_observation):
        require(re.fullmatch('[0-9a-f]{64}', digest), 'explicit primary observation digest')
        source = Path(name)
        original, _ = copies.add(source, f'primary/observation-{index + 1}.json')
        require(original['sha256'] == digest and source.suffix == '.json', 'actual primary observation')
        observations.append(dict(pin=original, interpreted_as_remote_owner_receipt=False,
            independently_replayed=False, claims='Verbatim primary-tool observation; not an approval or GPU result.'))
    copies.add(Path(__file__).resolve(), 'tools/publish.py')
    require(len(copies.bodies) == 67 + len(observations), '51 audit + eight assembly + seven root + publisher roster')
    result = dict(schema='ferric-p228-projection-ordered-segment-runtime-publication-v1',
        audits=audits, assemblies=assemblies, primary_observations=observations, copied=copies.ledger,
        copied_count=len(copies.bodies), internal_audit_pin_count=45,
        actual_cpu=CPU, supervisor_package_sha256=PACKAGE_SHA, supervisor_pure_sha256=PURE_SHA,
        metadata_and_copied_bodies_rehashed=True, audit_commands_reexecuted=False,
        executable_and_library_bodies_locally_rehashed=False, full_assembly_input_graph_replayed=False,
        root_decisions_reassessed=False, new_approval_authored=False, gpu_execution=False, pair_execution=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    for local, expected in copies.checked.items():
        require(pin(local, read(local)) == expected, 'source unchanged before publication')
    require(not OUT.is_symlink() and (not OUT.exists() or {p.name for p in OUT.iterdir()} <= {'README.md'}), 'fresh output except root README')
    OUT.mkdir(parents=True, exist_ok=True)
    for name, body in sorted(copies.bodies.items()):
        target = OUT / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        os.chmod(target, 0o644)
        require(read(target) == body, 'published byte identity')
    for local, expected in copies.checked.items():
        require(pin(local, read(local)) == expected, 'source unchanged after publication')
    with (OUT / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    os.chmod(OUT / 'result.json', 0o644)
    print(json.dumps(dict(result=pin(OUT / 'result.json', read(OUT / 'result.json')), copied=len(copies.bodies)), sort_keys=True))


if __name__ == '__main__':
    main()
