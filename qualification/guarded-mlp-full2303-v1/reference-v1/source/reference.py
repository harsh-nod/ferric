"""Closed Readiness40 reference records and diagnostics, never acceptance."""
import struct

from common import compact, parse, require
import diagnostics as D

FORWARDS = 40
SELECTED = (0, 5, 16, 39)
LAYERS = 36
HIDDEN = 4096
VOCABULARY = 151936
PAYLOAD_BYTES = 606976
ROLES = {'prompt_manifest', 'prompt_text', 'prompt_tokens', 'workload',
         'qualified_owner', 'qualified_model_record'}


def uint(value, maximum):
    require(type(value) is int and 0 <= value <= maximum, 'closed unsigned integer')
    return value


def pin(value, extent):
    require(type(value) is dict and set(value) == {'bytes', 'sha256'}
            and type(value['bytes']) is int and value['bytes'] == extent
            and type(value['sha256']) is str and len(value['sha256']) == 64
            and all(c in '0123456789abcdef' for c in value['sha256']), 'exact compact pin')


def prompt_inputs(contract, bodies):
    require(set(bodies) == ROLES and set(contract['files']) == ROLES, 'closed original input roles')
    require(contract['model_id'] == 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
            and contract['bundle_id'] == '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
            and contract['candidate_receipt'] is None and contract['candidate_captures'] is None,
            'reference-only fixed target identity, no native input')
    for role, raw in bodies.items():
        require(compact(raw) == {k: contract['files'][role][k] for k in ('bytes', 'sha256')},
                'authenticated original input bytes')
    p = parse(bodies['prompt_manifest'])
    require(p['schema'] == 'FerricQwen3LongPromptV1'
            and p['revision'] == 'b968826d9c46dd6066d109eabc6255188de91218'
            and p['input_tokens'] == 2048 and p['output_tokens'] == 256
            and p['add_special_tokens'] is False and p['chat_template'] is None
            and p['round_trip_verified'] is True and p['generated_reference'] is False,
            'authentic full prompt, not a forty-token substitute')
    tokens = p['input_token_ids']
    require(type(tokens) is list and len(tokens) == 2048
            and all(type(t) is int and 0 <= t < VOCABULARY for t in tokens), '2048 exact token IDs')
    require(bodies['prompt_tokens'] == struct.pack('<2048I', *tokens), 'full binary prompt identity')
    roles = {'prompt.txt': 'prompt_text', 'prompt.u32le': 'prompt_tokens', 'workload.json': 'workload'}
    require(len(p['files']) == 3 and {r['path'] for r in p['files']} == set(roles), 'original prompt file roster')
    for row in p['files']:
        require(compact(bodies[roles[row['path']]]) == {k: row[k] for k in ('bytes', 'sha256')}, 'prompt inner pin')
    text = bodies['prompt_text'].decode('utf-8')
    require(parse(bodies['workload']) == {'schema': 'FerricQwen3TpWorkloadV2', 'requests': [
        {'name': 'qwen3-8b-2048-256', 'prompt': text, 'new_tokens': 256, 'arrival_tick': 0}]},
        'original workload identity, not generated readiness outputs')
    owner, model = parse(bodies['qualified_owner']), parse(bodies['qualified_model_record'])
    require(owner['schema'] == 'ferric-guarded-mlp-matched-input-owned-v1'
            and model['schema'] == 'ferric-guarded-mlp-matched-input-framework-v1'
            and owner['passed'] is True and owner['errors'] == [] and owner['postcheck_errors'] == []
            and owner['container_removed'] is True and owner['framework_attempts'] == 1
            and owner['retries'] == 0 and model['passed'] is True and model['error'] is None
            and model['postcheck_errors'] == []
            and {k: owner['framework_report'][k] for k in ('bytes', 'sha256')}
                == compact(bodies['qualified_model_record']), 'qualified loader/model lineage')
    expected = {n: {k: r[k] for k in ('bytes', 'sha256')} for n, r in model['model_sources'].items()}
    require(expected == contract['model_files'] and len(expected) == 9, 'exact original nine-file checkpoint')
    require(all({k: p['tokenizer_sources'][n][k] for k in ('bytes', 'sha256')} == expected[n]
                for n in ('tokenizer.json', 'tokenizer_config.json')), 'prompt/model tokenizer join')
    return tokens, p, expected


class Capture:
    def __init__(self, raw):
        self.raw, self.hidden, self.norm, self.handles = raw, {}, [], []

    def layer(self, index):
        uint(index, LAYERS - 1)
        def observe(_module, _arguments, output):
            require(index not in self.hidden, 'duplicate selected layer output')
            value = output[0] if isinstance(output, (tuple, list)) else output
            self.hidden[index] = self.raw(value, (1, 1, HIDDEN))
        return observe

    def norm_input(self, _module, arguments):
        require(len(arguments) >= 1 and not self.norm, 'exactly one final norm input')
        self.norm.append(self.raw(arguments[0], (1, 1, HIDDEN)))

    def attach(self, model):
        require(len(model.model.layers) == LAYERS, '36 real model layers')
        for index, module in enumerate(model.model.layers):
            self.handles.append(module.register_forward_hook(self.layer(index)))
        self.handles.append(model.model.norm.register_forward_pre_hook(self.norm_input))

    def payload(self, final, logits):
        require(set(self.hidden) == set(range(LAYERS)) and self.norm == [self.hidden[35]],
                'all real layer outputs and final norm input join')
        raw = b''.join(self.hidden[i] for i in range(LAYERS)) + final + logits
        D.split_payload(raw)
        return raw

    def close(self):
        first = None
        for handle in reversed(self.handles):
            try:
                handle.remove()
            except BaseException as error:
                first = first or error
        self.handles.clear()
        if first is not None:
            raise first


def validate_case(case, tokens):
    require(type(case) is dict and set(case) == {'position', 'generation', 'input_token',
            'predicted_token', 'cache_length', 'logits', 'payload', 'tensors', 'cache_hashes'}, 'closed reference record')
    position = uint(case['position'], FORWARDS - 1)
    require(type(case['generation']) is int and case['generation'] == position + 1
            and type(case['input_token']) is int and case['input_token'] == tokens[position]
            and type(case['cache_length']) is int and case['cache_length'] == position + 1,
            'same prompt position and own causal cache')
    uint(case['predicted_token'], VOCABULARY - 1)
    pin(case['logits'], 2 * VOCABULARY)
    if position not in SELECTED:
        require(case['payload'] is None and case['tensors'] is None and case['cache_hashes'] is None,
                'no unselected retained full capture')
        return
    pin(case['payload'], PAYLOAD_BYTES)
    names = {'layer%d-hidden' % i for i in range(LAYERS)} | {'final-norm', 'logits'}
    require(type(case['tensors']) is dict and set(case['tensors']) == names, '38 exact tensor roles')
    for name, row in case['tensors'].items():
        pin(row, 2 * (VOCABULARY if name == 'logits' else HIDDEN))
    require(case['tensors']['logits'] == case['logits'], 'selected logit identity')
    require(type(case['cache_hashes']) is list and len(case['cache_hashes']) == LAYERS,
            '36 selected own-cache hashes')
    for pair in case['cache_hashes']:
        require(type(pair) is dict and set(pair) == {'key', 'value'}, 'two KV roles')
        for row in pair.values():
            pin(row, 2 * 8 * (position + 1) * 128)


def validate_pass(value, tokens, payloads):
    require(type(tokens) is list and len(tokens) == 2048
            and all(type(t) is int and 0 <= t < VOCABULARY for t in tokens), 'full prompt remains bound')
    require(type(value) is dict and set(value) == {'ordinal', 'fresh_cache', 'cases'}
            and type(value['ordinal']) is int and value['ordinal'] in (1, 2)
            and value['fresh_cache'] is True and type(value['cases']) is list
            and len(value['cases']) == FORWARDS, 'fresh forty-position pass')
    require(type(payloads) is dict and all(type(k) is int for k in payloads)
            and set(payloads) == set(SELECTED), 'four exact selected payload positions')
    for position, case in enumerate(value['cases']):
        validate_case(case, tokens)
        require(case['position'] == position, 'complete ordered prompt history')
        if position in SELECTED:
            raw = payloads[position]
            rows = D.split_payload(raw)
            require(compact(raw) == case['payload']
                    and {n: compact(b) for n, b in rows.items()} == case['tensors']
                    and D.argmax(rows['logits']) == case['predicted_token'], 'actual captured bytes/finite/argmax joins')


def repeat_gate(passes, tokens, payloads):
    require(type(passes) is list and len(passes) == 2 and len(payloads) == 2, 'two reference passes')
    for ordinal, (value, raw) in enumerate(zip(passes, payloads), 1):
        validate_pass(value, tokens, raw)
        require(value['ordinal'] == ordinal, 'two distinct ordered passes')
    return passes[0]['cases'] == passes[1]['cases'] and payloads[0] == payloads[1]


def compare_selected(passes, reference_payloads, candidate_cases, tokens, candidate_history):
    """Pure comparison only. A separate pinned checker must admit native receipts."""
    require(repeat_gate(passes, tokens, reference_payloads), 'reference repeat gate before comparison')
    require(type(candidate_history) is list and len(candidate_history) == FORWARDS
            and all(type(t) is int for t in candidate_history)
            and candidate_history == tokens[:FORWARDS], 'all40 candidate prompt inputs, not four selected inputs')
    require(type(candidate_cases) is dict and all(type(k) is int for k in candidate_cases)
            and set(candidate_cases) == set(SELECTED), 'four candidate captures')
    output = []
    for position in SELECTED:
        case = candidate_cases[position]
        require(type(case) is dict and set(case) == {'position', 'input_token', 'output_token', 'payload'}
                and type(case['position']) is int and case['position'] == position
                and type(case['input_token']) is int and case['input_token'] == tokens[position],
                'candidate same authentic prompt history')
        uint(case['output_token'], VOCABULARY - 1)
        right, left = D.split_payload(case['payload']), D.split_payload(reference_payloads[0][position])
        require(case['output_token'] == D.argmax(right['logits']), 'candidate own argmax')
        output.append(dict(position=position, input_token=tokens[position],
            reference_output_token=passes[0]['cases'][position]['predicted_token'],
            candidate_output_token=case['output_token'],
            output_token_equal=passes[0]['cases'][position]['predicted_token'] == case['output_token'],
            tensors={n: D.compare_tensor(raw, right[n]) for n, raw in left.items()}))
    return dict(schema='ferric-readiness40-position5-selected-tensor-diagnostic-v1', comparisons=output,
        tensor_rows=152, input_history='authentic first40 prompt IDs; independent own KV; no generated feedback',
        receipt_authentication=False, numerical_acceptance=False, acceptance_threshold=None,
        full_model_acceptance=False, full_long_workload=False, performance_claim=False)
