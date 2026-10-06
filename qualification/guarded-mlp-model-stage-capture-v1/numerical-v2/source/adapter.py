"""Current capture admission and stage mapping; numerical operators are unchanged."""
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


def terminal_contract(c, terminal):
    c.require(terminal['mode'] == 'ar4'
              and terminal['schema'] == 'ferric-guarded-mlp-model-stage-capture-gpu-v1',
              'explicit current guarded stage capture')
    c.require(terminal['passed'] is True and terminal['errors'] == []
              and terminal['capture_requested'] is True and terminal['capture_verified'] is True
              and all(terminal[name] is False for name in FALSE)
              and terminal['native_attempts'] == 1 and terminal['retries'] == 0
              and terminal['gpu_execution'] is True and terminal['gpu_execution_confirmed'] is True
              and terminal['native_spawn_observed'] is True,
              'actual successful finite capture, not numerical acceptance')


def guarded_candidate(reader, c, validator, announcement, terminal_pin, controller_pin, capture_admission):
    require = c.require
    terminal_pin = c.pin(terminal_pin)
    terminal = c.doc(reader.read, terminal_pin)
    case = Path(terminal_pin['path']).parent
    terminal_contract(c, terminal)
    require(c.pin(terminal['controller']) == c.pin(controller_pin), 'actual capture controller binding')
    require(terminal_pin['path'].endswith('/ar4/complete.json'), 'actual current complete path')
    expected_raw = RAW | {'observation.json', 'capture-observation.json'}
    require(set(terminal['raw']) == expected_raw and len(expected_raw) == 78,
            'closed successful capture raw roster')
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
            and decode['mode'] == 'autoregressive'
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
    require(same_json(c, checked, c.doc(reader.read, terminal['raw']['observation.json'])),
            'successful AR4 separately saved observation')
    capture_checked = capture_admission(raw, native_read, validator)
    require(same_json(c, capture_checked, terminal['capture_observation'])
            and same_json(c, capture_checked, c.doc(reader.read, terminal['raw']['capture-observation.json'])),
            'exact replayed and saved capture observation')
    parent = next(row for row in terminal['phases'] if row['label'] == 'parent')
    command = c.doc(reader.read, parent['command'])
    require(command['argv'] == [c.pin(plan['parent'])['path'], '--request', c.pin(plan['request'])['path'],
                               '--capture-guarded-layer-zero', '--allow-unauthenticated-machine-code']
            and command['env'] == ENV and command['cwd'] == str(case.parent.parent)
            and command['deadline_seconds'] == 4000 and command['affinity'] == [8, 9]
            and command['nice'] == 10 and command['address_space_bytes'] == 32 << 30
            and command['file_cap_bytes'] == 64 << 20 and command['stream_cap_bytes'] == 8 << 20
            and command['gpu_execution_requested'] is True, 'unchanged owned guarded native envelope')
    lineage = announcement.validate_lineage(c.get(reader.read, parent['stderr']), parent,
                                            c.doc(reader.read, parent['started']), checked['child_pid'])
    observed = c.document(raw)
    records = c.records(observed)
    payloads = [c.get(reader.read, terminal['raw'][f'native/observation-{i}.bin'], 606976) for i in range(4)]
    envelope = validator.parse(native_read(capture_checked['source']))
    return records, payloads, envelope, dict(terminal=terminal_pin, controller=controller_pin,
        observation=checked, capture_observation=capture_checked, owned_lineage=lineage,
        request=plan['request'], parent=plan['parent'], worker=plan['worker'],
        controls=[terminal['raw'][f'native/control-{i}.bin'] for i in range(4)],
        payloads=[terminal['raw'][f'native/observation-{i}.bin'] for i in range(4)],
        model_source_authenticated_by_qualified_parent=terminal['model_source_authenticated_by_qualified_parent'],
        outer_model_shards_rehashed=terminal['outer_model_shards_rehashed'])


def framework_values(reader, c, layer, diagnostics, owner_pin):
    owner = c.doc(reader.read, owner_pin)
    c.require(owner['schema'] == 'ferric-p228-layer0-framework-launch-complete-v1'
              and owner['passed'] is True and owner['failures'] == []
              and owner['native_attempts'] == 1 and owner['retries'] == 0
              and owner['gpu_execution'] is True and owner['candidate_gpu_execution'] is False
              and all(owner[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority')),
              'actual independent framework owner, not only a live inner report')
    execution = c.doc(reader.read, owner['execution_result'])
    c.require(type(execution['exit_code']) is int and execution['exit_code'] == 0
              and execution['failure'] is None and execution['group_absent'] is True,
              'historical framework natural zero exit and recorded absent group')
    for field in ('command', 'started', 'stdout', 'stderr'):
        reader.read(execution[field])
    started = c.doc(reader.read, execution['started'])
    c.require(started['command_sha256'] == c.pin(execution['command'])['sha256']
              and started['pid'] == started['pgid'] > 1, 'framework original owned command/start join')
    stdout = c.get(reader.read, execution['stdout'], 65536)
    c.require(c.pin(c.document(stdout)) == c.pin(owner['reference']), 'framework natural exit reports exact capture')
    report = c.doc(reader.read, owner['reference'])
    values = layer.read_framework(report, reader.read, diagnostics)
    c.require(len(values) == 33, 'all two-pass independent stage bodies consumed')
    return values, report, dict(owner=owner_pin, execution=owner['execution_result'], capture=owner['reference'])


ORDER = ('input', 'input_normalized', 'raw_qkv', 'query', 'current_key', 'current_value',
         'attention', 'first_residual', 'post_normalized', 'gate', 'up', 'activation', 'final_hidden')
PARTS = (
    ('before_prefix', (('input', 'bf16', 8192), ('cache_metadata', 'u32', 580), ('rotary', 'f32', 512))),
    ('after_prefix', (('input_normalized', 'bf16', 8192), ('raw_qkv', 'bf16', 6144),
        ('query', 'bf16', 4096), ('current_key', 'bf16', 1024), ('current_value', 'bf16', 1024),
        ('attention', 'bf16', 4096), ('output_partial', 'f32', 16384))),
    ('after_first_residual', (('first_residual', 'bf16', 8192),)),
    ('after_mlp', (('post_normalized', 'bf16', 8192), ('gate', 'bf16', 12288),
        ('up', 'bf16', 12288), ('activation', 'bf16', 12288), ('down_partial', 'f32', 16384))),
    ('after_final_residual', (('final_hidden', 'bf16', 8192),)),
)


def split_capture(c, envelope):
    capture = envelope['capture']
    payload = bytes(capture['payload'])
    c.require(len(payload) == capture['payload_bytes'] == 256136
              and c.sha(payload) == bytes(capture['payload_sha256']).hex()
              and len(capture['parts']) == 34, 'actual complete capture partition')
    parts = [{}, {}]
    excluded = []
    offset = ordinal = 0
    for boundary, specs in PARTS:
        for rank in (0, 1):
            for role, scalar, size in specs:
                row = capture['parts'][ordinal]
                width = 2 if scalar == 'bf16' else 4
                c.require((row['boundary'], row['rank'], row['role'], row['scalar'], row['elements'],
                           row['offset'], row['bytes']) == (boundary, rank, role, scalar, size // width, offset, size),
                          'closed comparison stage layout')
                raw = payload[offset:offset + size]
                c.require(c.sha(raw) == bytes(row['sha256']).hex(), 'comparison part digest')
                parts[rank][role] = raw
                if role in ('output_partial', 'down_partial'):
                    excluded.append(dict(rank=rank, role=role, bytes=size, sha256=c.sha(raw),
                        reason='FP32 rank partial is not the full BF16 framework projection'))
                offset += size
                ordinal += 1
    c.require(offset == len(payload) and ordinal == 34, 'complete comparison partition')
    return parts, excluded


def comparisons(c, layer, diagnostics, framework, parts):
    c.require(set(framework) == set(layer.SHAPES) and len(parts) == 2, 'two actual ranks and complete framework')
    rows = []
    for rank, values in enumerate(parts):
        q = framework['q-projection'][rank * 4096:(rank + 1) * 4096]
        k = framework['k-projection'][rank * 1024:(rank + 1) * 1024]
        v = framework['v-projection'][rank * 1024:(rank + 1) * 1024]
        references = (
            framework['embedding'], framework['input-norm'], q + k + v,
            framework['rotary-q'][rank * 4096:(rank + 1) * 4096],
            framework['cache-key'][rank * 1024:(rank + 1) * 1024],
            framework['cache-value'][rank * 1024:(rank + 1) * 1024],
            framework['attention-output'][rank * 4096:(rank + 1) * 4096],
            framework['first-residual'], framework['post-norm'],
            framework['gate'][rank * 12288:(rank + 1) * 12288],
            framework['up'][rank * 12288:(rank + 1) * 12288],
            framework['product'][rank * 12288:(rank + 1) * 12288], framework['layer0-hidden'])
        for ordinal, (role, reference) in enumerate(zip(ORDER, references)):
            row = dict(rank=rank, observable_order=ordinal,
                       **layer.diagnostic(diagnostics, role, reference, values[role]))
            if role == 'raw_qkv':
                row['components'] = [layer.diagnostic(diagnostics, name, expected, actual)
                    for name, expected, actual in (
                        ('q-projection-shard', q, values[role][:4096]),
                        ('k-projection-shard', k, values[role][4096:5120]),
                        ('v-projection-shard', v, values[role][5120:]))]
            rows.append(row)
    rows.sort(key=lambda row: (row['observable_order'], row['rank']))
    different = [row for row in rows if not row['byte_equal']]
    earliest = None if not different else dict(stage=different[0]['stage'],
        observable_order=different[0]['observable_order'],
        ranks=[row['rank'] for row in different if row['observable_order'] == different[0]['observable_order']])
    c.require(len(rows) == 26, '26 comparable captured BF16 rows plus six QKV detail rows')
    return rows, earliest


def compare(reader, c, layer, diagnostics, validator, announcement, terminal, controller,
            capture_admission, framework_owner):
    records, payloads, envelope, candidate = guarded_candidate(
        reader, c, validator, announcement, terminal, controller, capture_admission)
    framework, framework_report, reference = framework_values(reader, c, layer, diagnostics, framework_owner)
    c.require(records[0] == dict(generation=1, position=0, input_token=9112,
                               output_token=records[0]['output_token']), 'same genuine first input/history')
    prompt_pins = framework_report['input_pins']
    c.require(all(any(p['bytes'] == size and p['sha256'] == digest for p in prompt_pins)
                  for size, digest in (PROMPT['manifest'], PROMPT['tokens'])), 'same authentic framework prompt')
    parts, excluded = split_capture(c, envelope)
    c.require(parts[0]['input'] == parts[1]['input'] == framework['embedding'],
              'directly captured same-input embedding in both ranks')
    c.require(parts[0]['final_hidden'] == parts[1]['final_hidden'] == payloads[0][:8192],
              'actual first-forward hidden bytes, not previous native capture')
    rows, earliest = comparisons(c, layer, diagnostics, framework, parts)
    return dict(schema='ferric-guarded-mlp-model-stage-numerical-diagnostic-v1', position=0, layer=0,
        input_token=9112, candidate=candidate, framework=reference, comparisons=rows,
        comparable_rows=26, qkv_component_rows=6, observation_order=list(ORDER),
        earliest_observable_divergence=earliest, ordering_is_a_causal_proof=False,
        both_rank_input_embeddings_match=True, captured_dedicated_guarded_down=True,
        excluded_fp32_partials=excluded, fp32_partials_compared_to_full_bf16=False,
        genuine_independent_framework_outputs=True, candidate_intermediate_inputs=False,
        conditional_replay_performed=False, cumulative_chain_differences_not_isolated_operator_errors=True,
        source_model_shards_rehashed=False, acceptance_threshold=None, numerical_acceptance=False,
        full_model_acceptance=False, performance_claim=False, production_authority=False,
        gpu_execution=False, native_rerun=False)
