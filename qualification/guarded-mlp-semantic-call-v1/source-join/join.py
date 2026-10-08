"""Join qualified call-inspection records to predeclared compiler input files."""
import hashlib
import json
import os
from pathlib import Path

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-call-source-join-v228-v1'
CPU = E / 'guarded-mlp-semantic-call-inspect-cpu-v228-v1'
PAIR = E / 'guarded-mlp-early-stop-compile-v228-v12'
PROVIDER = E / 'guarded-mlp-inline-fndef-origin-cpu-v228-v1/fe2o3/crates/fe2o3-device/src/finite_join/wave_mlp_tiles_v2.rs'
CPU_SHA = '862acab9993408c8303ba047219b2fca5c2c8a4d9c940807b22569b8f3257a7b'
PROVIDER_SHA = 'df217a21e317623af943282a7ae4b43d840c69db983cfbbcd9418a16953f7c8b'
before = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path):
    require(path.resolve(strict=True) == path and path.is_file()
            and not path.is_symlink() and path.stat().st_size <= 8 << 20, 'bounded canonical input')
    raw = path.read_bytes()
    require(len(raw) <= 8 << 20, 'read extent')
    row = dict(path=str(path), **pin(raw))
    require(str(path) not in before or before[str(path)] == row, 'input drift')
    before[str(path)] = row
    return raw


def coordinate(raw, offset):
    prefix = raw[:offset].decode('utf-8')
    return [prefix.count('\n') + 1, len(prefix.rsplit('\n', 1)[-1]) + 1]


def origins(value, label=''):
    if isinstance(value, dict):
        for key, item in value.items():
            name = label + '.' + key if label else key
            if key in ('call_site', 'expansion') and item is not None:
                yield name, item
            else:
                yield from origins(item, name)
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from origins(item, label + '[' + str(index) + ']')


def main():
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and Path(__file__).resolve() == ROOT / 'join.py', 'owned host/script')
    require(not (ROOT / 'result.json').exists(), 'fresh result')
    read(ROOT / 'join.py')
    cpu_raw = read(CPU / 'evidence/complete.json')
    require(pin(cpu_raw)['sha256'] == CPU_SHA, 'qualified reader terminal')
    cpu = json.loads(cpu_raw)
    require(cpu['passed'] and cpu['failure'] is None and cpu['postcheck_errors'] == []
            and cpu['tests']['reader']['passed'] == 14 and cpu['tests']['reader']['failed'] == 0
            and cpu['tests']['reader']['ignored'] == 0 and cpu['source_unchanged']
            and len(cpu['phases']) == 11, 'actual complete reader scope')
    provider_raw = read(PROVIDER)
    require(pin(provider_raw) == dict(bytes=37296, sha256=PROVIDER_SHA), 'exact provider body')
    results = {}
    for arm, block, locals_ in (('fixed', 307, [84, 1206, 1468, 927]),
                                ('early', 159, [1558, 1312, 598, 653])):
        label = 'inspect-call-' + arm
        phase = next(row for row in cpu['phases'] if row['label'] == label)
        require(phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped']
                and phase['process_group_absent'] and not phase['timed_out']
                and not phase['forced_cleanup'] and phase['exception'] is None, 'clean reader leaf')
        path = CPU / ('evidence/' + label + '.stdout')
        raw = read(path)
        require(before[str(path)] == phase['stdout'], 'original reader stdout')
        decoded = json.loads(raw)
        require(decoded == cpu['inspections']['call-' + arm]
                and decoded['canonical_decode_verified']
                and not decoded['execution_authority'] and not decoded['production_admission']
                and not decoded['kir_binding_types_observed'], 'exact diagnostic-only inspection')
        mapping_path = PAIR / (arm + '-run/fe2o3-engineering-diagnostics-v1/semantic-source-map-v1.json')
        mapping_raw = read(mapping_path)
        require(before[str(mapping_path)] == cpu['input_sources']['call-captures/' + arm + '/semantic-source-map-v1.json'],
                'actual capture source-map join')
        mapping = json.loads(mapping_raw)
        files = {row['identity']: row for row in mapping['files']}
        source = PAIR / arm / 'src/lib.rs'
        source_raw = read(source)
        allowed = {'src/lib.rs': (source, source_raw), str(PROVIDER): (PROVIDER, provider_raw)}
        for name in ('sources-before.json', 'sources-after.json'):
            snapshot = json.loads(read(PAIR / (arm + '-run/evidence/' + name)))
            require(snapshot[str(source)] == before[str(source)]
                    and snapshot[str(PROVIDER)] == before[str(PROVIDER)], 'actual compiler source snapshots')
        require(decoded['caller']['identity'] == '99b4ed58d73a44829a63b63699ef6041ebf197774a13655de68898d4954450f7'
                and decoded['callee']['identity'] == '577e317b6857250fd42b10547b73d2af0644e68f06ad883e27b5fbec8e15e40a'
                and decoded['caller']['function_index'] == decoded['call']['function_index'] == 2
                and decoded['callee']['function_index'] == decoded['call']['callee_function_index'] == 1
                and decoded['call']['block_index'] == block, 'exact captured call identities')
        arguments = decoded['call']['arguments']
        require(len(arguments) == 4
                and [row['operand']['local']['index'] for row in arguments] == locals_
                and [row['operand']['kind'] for row in arguments] == ['Copy', 'Copy', 'Move', 'Move']
                and all(row['semantic_type_matches'] and row['operand']['projections'] == []
                        for row in arguments), 'exact four unprojected semantic operands')
        expected_scalars = [dict(kind='Integer', signed=False, bits=32),
                            dict(kind='Integer', signed=False, bits=64),
                            dict(kind='Integer', signed=False, bits=64), dict(kind='Bool')]
        require([row['operand']['type']['scalar'] for row in arguments] == expected_scalars,
                'actual source parameter widths and signedness')
        joined = {}
        for name, origin in origins(decoded):
            require(origin['display_path'] in allowed and not origin['source_file_contents_verified'],
                    'predeclared display path only')
            path, body = allowed[origin['display_path']]
            file = files[origin['file_identity']]
            require(file['display_path'] == origin['display_path']
                    and file['byte_len'] == origin['file_bytes'] == len(body), 'full source identity and extent')
            start, end = origin['byte_range']
            require(0 <= start < end <= len(body) and end - start <= 8192
                    and coordinate(body, start) == origin['start']
                    and coordinate(body, end) == origin['end'], 'exact byte and coordinate join')
            joined[name] = dict(origin, span_utf8=body[start:end].decode('utf-8'),
                                source_file_contents_verified=True, verified_source=before[str(path)])
        require(joined['callee.call_site']['span_utf8'].startswith('fn complete_coverage(')
                and joined['call.call_site']['span_utf8']
                == 'complete_coverage(self.token, lane, self.written, self.valid)', 'actual source callee and call')
        results[arm] = dict(inspection=before[str(CPU / ('evidence/' + label + '.stdout'))],
                           function=2, block=block, origins=joined, arguments=arguments)
    after = {path: dict(path=path, **pin(Path(path).read_bytes())) for path in before}
    require(after == before, 'all inputs unchanged')
    result = dict(schema='ferric-canonical-call-source-join-v1', passed=True,
                  inputs=before, inputs_after=after, arms=results,
                  source_contents_join_verified=True, kir_binding_types_observed=False,
                  gpu_execution=False, production_authority=False, compiler_hsaco_reproduced=False)
    raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
    with (ROOT / 'result.json').open('xb') as stream:
        stream.write(raw)
    require((ROOT / 'result.json').read_bytes() == raw, 'result readback')
    print(json.dumps(dict(result=pin(raw), input_count=len(before), passed=True), sort_keys=True))


if __name__ == '__main__':
    main()
