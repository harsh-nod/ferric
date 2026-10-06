"""Pure guarded-observation admission for the unchanged numerical comparers."""
from pathlib import Path
import types

MODEL_ID = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
BUNDLE_ID = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
TOKENS = [9112, 2190, 3772, 220]
PROMPT = {
    'manifest': (21318, '30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600'),
    'text': (11224, 'a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a'),
    'tokens': (8192, '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02'),
}
FALSE = ('numerical_acceptance', 'full_model_acceptance', 'independent_full_model_reference',
         'performance_claim', 'production_authority', 'full_long_workload')
LABELS = ['parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd']
LABELS += ['before-' + str(i) for i in range(3)] + ['parent'] + ['after-' + str(i) for i in range(3)]
NATIVE = {'complete.json', 'child-stderr.bin'} | {
    f'{stem}-{i}.{suffix}' for i in range(4)
    for stem, suffix in [('request', 'json'), ('control', 'bin'), ('observation', 'bin')]}
RAW = {label + '/' + name for label in LABELS
       for name in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
RAW |= {'initial-topology.json'} | {label + '-topology.json' for label in LABELS
                                   if label.startswith(('before-', 'after-'))}
RAW |= {'native/' + name for name in NATIVE}
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
           OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
           HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')


def same_json(c, left, right):
    encode = lambda value: c.json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False)
    return encode(left) == encode(right)


def terminal_contract(c, terminal, tag):
    c.require(tag in ('tf4', 'ar4') and terminal['mode'] == tag
              and terminal['schema'] == 'ferric-guarded-mlp-model-gpu-v1', 'guarded case identity')
    c.require(all(terminal[name] is False for name in FALSE)
              and terminal['native_attempts'] == 1 and terminal['retries'] == 0
              and terminal['gpu_execution'] is True and terminal['gpu_execution_confirmed'] is True
              and terminal['native_spawn_observed'] is True, 'actual finite guarded execution, no acceptance')
    if tag == 'tf4':
        c.require(terminal['passed'] is False
                  and terminal['errors'] == ['RuntimeError: one actual worker announcement'],
                  'preserved original TF4 controller failure')
    else:
        c.require(terminal['passed'] is True and terminal['errors'] == [], 'actual successful AR4 controller')


def guarded_candidate(reader, c, validator, announcement, terminal_pin, tag, revalidation_pin=None):
    require = c.require
    terminal_pin = c.pin(terminal_pin)
    terminal = c.doc(reader.read, terminal_pin)
    case = Path(terminal_pin['path']).parent
    terminal_contract(c, terminal, tag)
    revalidated = None
    if tag == 'tf4':
        require(terminal['passed'] is False
                and terminal['errors'] == ['RuntimeError: one actual worker announcement']
                and case.name == 'tf4' and terminal_pin['path'].endswith('/tf4/failed.json'),
                'preserved original TF4 controller failure')
        revalidated = c.doc(reader.read, revalidation_pin)
        require(revalidated['schema'] == 'ferric-guarded-mlp-model-tf4-data-revalidation-v1'
                and revalidated['passed'] is True and revalidated['error'] is None
                and revalidated['original_controller_passed'] is False
                and revalidated['original_failure_preserved'] is True
                and revalidated['actual_original_observation_revalidated'] is True
                and revalidated['input_posthashes_complete'] is True
                and revalidated['native_rerun'] is False and revalidated['gpu_execution'] is False
                and all(revalidated[name] is False for name in FALSE)
                and c.pin(revalidated['original_terminal']) == terminal_pin
                and revalidated['original_raw_files'] == 76
                and revalidated['original_natural_phases'] == 11
                and revalidated['original_idle_snapshots'] == 6, 'separate actual TF4 revalidation')
        for path, pin in revalidated['inputs'].items():
            require(path == c.pin(pin)['path'], 'revalidation readset path')
            reader.read(pin, retain=False)
    else:
        require(revalidation_pin is None and terminal['passed'] is True and terminal['errors'] == []
                and terminal_pin['path'].endswith('/ar4/complete.json'), 'actual successful AR4 controller')
    expected_raw = RAW | ({'observation.json'} if tag == 'ar4' else set())
    require(set(terminal['raw']) == expected_raw and len(expected_raw) == (76 if tag == 'tf4' else 77),
            'closed mode-specific guarded raw roster')
    reader.tree(case, expected_raw | {Path(terminal_pin['path']).name})
    for name, pin in terminal['raw'].items():
        require(c.pin(pin)['path'] == str(case / name), 'guarded raw path custody')
        reader.read(pin, retain=False)
    for path, pin in terminal['readset'].items():
        require(path == c.pin(pin)['path'], 'guarded readset path')
        reader.read(pin, retain=False)
    require([row['label'] for row in terminal['phases']] == LABELS, 'eleven guarded owned phases')
    for phase in terminal['phases']:
        c.clean_owner(phase)
        label = phase['label']
        require(c.doc(reader.read, terminal['raw'][label + '/result.json'])
                == {k: v for k, v in phase.items() if k != 'label'}, 'original phase/result join')
        for key, filename in [('command', 'command.json'), ('started', 'started.json'),
                              ('stdout', 'stdout'), ('stderr', 'stderr')]:
            require(c.pin(phase[key]) == c.pin(terminal['raw'][label + '/' + filename]), 'phase raw pin join')
        started = c.doc(reader.read, phase['started'])
        require(started['command_sha256'] == c.pin(phase['command'])['sha256'], 'owned command start join')
    plan = c.doc(reader.read, terminal['plan'])
    request = c.doc(reader.read, plan['request'], 16384)
    decode = request['decode']
    require(request['schema'] == 'FerricFiniteGuardedMlpDecodeRequestV1'
            and decode['schema'] == 'FerricFinitePrefixDecodeRequestV1'
            and decode['mode'] == ('teacher_forced' if tag == 'tf4' else 'autoregressive')
            and decode['evidence_directory'] == str(case / 'native')
            and bytes(decode['expected_model_id']).hex() == MODEL_ID
            and bytes(decode['expected_bundle_id']).hex() == BUNDLE_ID
            and decode['device_ids'] == validator.IDS, 'same model, bundle, mode and native directory')
    require(set(decode['prompt']) == set(PROMPT), 'same three-part prompt')
    for name, expected in PROMPT.items():
        pin = c.pin(decode['prompt'][name])
        require((pin['bytes'], pin['sha256']) == expected, 'exact independent-reference prompt identity')
        reader.read(pin)
    require(c.pin(decode['worker']) == c.pin(plan['worker']), 'same actual worker')
    reader.read(plan['parent'], retain=False)
    reader.read(plan['worker'], retain=False)
    raw = c.get(reader.read, terminal['raw']['native/complete.json'], 65536)
    require(c.get(reader.read, terminal['raw']['parent/stdout']) == raw, 'sole parent summary bytes')

    def native_read(pin):
        pin = c.pin(pin)
        require(Path(pin['path']).parent == case / 'native' and Path(pin['path']).name in NATIVE,
                'validator reads only exact native roster')
        require(pin == c.pin(terminal['raw']['native/' + Path(pin['path']).name]), 'validator original raw join')
        return reader.read(pin)

    checked = validator.validate(raw, request, case / 'native', native_read)
    require(same_json(c, checked, terminal['observation']), 'exact original structural observation')
    if tag == 'ar4':
        require(same_json(c, checked, c.doc(reader.read, terminal['raw']['observation.json'])),
                'successful AR4 separately saved observation')
    parent = next(row for row in terminal['phases'] if row['label'] == 'parent')
    command = c.doc(reader.read, parent['command'])
    require(command['argv'] == [c.pin(plan['parent'])['path'], '--request', c.pin(plan['request'])['path'],
                               '--observe-guarded-mlp-decode', '--allow-unauthenticated-machine-code']
            and command['env'] == ENV and command['cwd'] == str(case.parent.parent)
            and command['deadline_seconds'] == 4000 and command['affinity'] == [8, 9]
            and command['nice'] == 10 and command['address_space_bytes'] == 32 << 30
            and command['file_cap_bytes'] == 64 << 20 and command['stream_cap_bytes'] == 8 << 20
            and command['gpu_execution_requested'] is True, 'unchanged owned guarded native envelope')
    lineage = announcement.validate_lineage(c.get(reader.read, parent['stderr']), parent,
                                            c.doc(reader.read, parent['started']), checked['child_pid'])
    if revalidated is not None:
        require(same_json(c, checked, revalidated['observation'])
                and lineage == revalidated['owned_lineage'], 'TF4 corrected replay joins this exact candidate')
        require(all(revalidated['inputs'].get(pin['path']) == pin for pin in terminal['raw'].values()),
                'TF4 revalidation covers every original raw body')
    observed = c.document(raw)
    records = c.records(observed)
    payloads = [c.get(reader.read, terminal['raw'][f'native/observation-{i}.bin'], 606976) for i in range(4)]
    return records, payloads, dict(terminal=terminal_pin, controller_passed=terminal['passed'],
        separate_tf4_revalidation=revalidation_pin, observation=checked, owned_lineage=lineage,
        request=plan['request'], parent=plan['parent'], worker=plan['worker'],
        controls=[terminal['raw'][f'native/control-{i}.bin'] for i in range(4)],
        payloads=[terminal['raw'][f'native/observation-{i}.bin'] for i in range(4)],
        model_source_authenticated_by_qualified_parent=terminal['model_source_authenticated_by_qualified_parent'],
        outer_model_shards_rehashed=terminal['outer_model_shards_rehashed'])


def compare_mode(reader, c, diagnostics, validator, announcement, terminal, reference, tag, revalidation=None):
    candidate_records, candidate_payloads, admission = guarded_candidate(
        reader, c, validator, announcement, terminal, tag, revalidation)
    # This immutable adapter supplies only the established reference identity constants.
    identity = types.SimpleNamespace(MODEL_ID=MODEL_ID, BUNDLE_ID=BUNDLE_ID, TOKENS=TOKENS)
    mode = 'teacher_forced' if tag == 'tf4' else 'autoregressive'
    reference_records, reference_payloads = c.reference(reader.read, reference, mode,
        dict(decode_validation=identity, diagnostics=diagnostics))
    rows = c.compare_rows(reference_records, reference_payloads, candidate_records, candidate_payloads, diagnostics)
    c.require(len(rows) == 4 and all(row['tensors'] is None if not row['same_input_history']
              else len(row['tensors']) == 38 for row in rows), 'same-history-only 38-tensor comparison')
    return dict(mode=mode, candidate=admission, reference_owner=reference,
        reference_records=reference_records, candidate_records=candidate_records, positions=rows,
        compared_positions=sum(row['same_input_history'] for row in rows),
        same_history_tensor_rows=sum(len(row['tensors'] or []) for row in rows),
        acceptance_threshold=None, numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False, full_long_workload=False)
