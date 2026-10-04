"""Pure genuine-chain diagnostics. The caller authenticates completed executions."""
import hashlib
import json
from pathlib import Path
import re
import struct
import types

DIAGNOSTICS_SHA = '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf'
MODEL = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
BUNDLE = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
TOKENS = [9112, 2190, 3772, 220]
V7 = '4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5'
DOWN2 = '65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449'
EMBEDDING_SHA = '68b2cf48a968e475175c36edb4b0fb94836fcb2afcdedcb4403b208cf98f6b1c'
SHAPES = {
    'embedding': (1, 1, 4096), 'input-norm-input': (1, 1, 4096),
    'input-norm': (1, 1, 4096), 'q-input': (1, 1, 4096),
    'k-input': (1, 1, 4096), 'v-input': (1, 1, 4096),
    'q-projection': (1, 1, 4096), 'k-projection': (1, 1, 1024),
    'v-projection': (1, 1, 1024), 'q-norm-input': (1, 1, 32, 128),
    'q-norm': (1, 1, 32, 128), 'k-norm-input': (1, 1, 8, 128),
    'k-norm': (1, 1, 8, 128), 'rotary-cos': (1, 1, 128),
    'rotary-sin': (1, 1, 128), 'rotary-q': (1, 32, 1, 128),
    'rotary-k': (1, 8, 1, 128), 'attention-output': (1, 1, 4096),
    'o-projection': (1, 1, 4096), 'first-residual': (1, 1, 4096),
    'post-norm': (1, 1, 4096), 'gate-input': (1, 1, 4096),
    'up-input': (1, 1, 4096), 'gate': (1, 1, 12288),
    'up': (1, 1, 12288), 'silu-input': (1, 1, 12288),
    'silu': (1, 1, 12288), 'product': (1, 1, 12288),
    'down-projection': (1, 1, 4096), 'mlp-output': (1, 1, 4096),
    'layer0-hidden': (1, 1, 4096), 'cache-key': (1, 8, 1, 128),
    'cache-value': (1, 8, 1, 128),
}
JOINS = (('embedding', 'input-norm-input'), ('input-norm', 'q-input'),
         ('input-norm', 'k-input'), ('input-norm', 'v-input'),
         ('q-projection', 'q-norm-input'), ('k-projection', 'k-norm-input'),
         ('post-norm', 'gate-input'), ('post-norm', 'up-input'), ('gate', 'silu-input'),
         ('down-projection', 'mlp-output'), ('rotary-k', 'cache-key'),
         ('v-projection', 'cache-value'))
HISTORICAL_LAYOUT = (
    ('norm', 8192), ('qkv', 6144), ('query', 4096), ('key-cache', 2359296),
    ('value-cache', 2359296), ('attention', 4096), ('output-partial', 16384),
    ('first-residual', 8192), ('mlp-norm', 8192), ('gate', 12288), ('up', 12288),
    ('activation', 12288), ('down-partial', 16384), ('final-hidden', 8192))


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def document(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON member')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _value: require(False, 'nonfinite JSON'))


def wire(value):
    require(type(value) is list and len(value) == 32
            and all(type(v) is int and 0 <= v <= 255 for v in value), '32 wire digest bytes')
    return bytes(value).hex()


def checked_read(read, pin, maximum):
    require(type(pin) is dict and set(pin) == {'path', 'bytes', 'sha256'}
            and type(pin['path']) is str and Path(pin['path']).is_absolute()
            and '..' not in Path(pin['path']).parts and type(pin['bytes']) is int
            and 0 < pin['bytes'] <= maximum and type(pin['sha256']) is str
            and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'closed bounded FilePin')
    raw = read(pin)
    require(type(raw) is bytes and len(raw) == pin['bytes'] and sha(raw) == pin['sha256'], 'actual pinned bytes')
    return raw


def load_diagnostics(read, pin):
    require(pin['sha256'] == DIAGNOSTICS_SHA, 'unchanged tested BF16 diagnostics')
    raw = checked_read(read, pin, 65536)
    value = types.ModuleType('genuine_layer0_retained_diagnostics')
    value.__file__ = pin['path']
    exec(compile(raw, pin['path'], 'exec'), value.__dict__)
    return value


def read_framework(report, read, diagnostics):
    require(report['schema'] == 'ferric-p228-layer0-framework-capture-v1'
            and report['status'] == 'PASS' and report['position'] == 0 and report['input_token'] == 9112
            and report['model_id'] == MODEL and report['bundle_id'] == BUNDLE
            and report['genuine_framework_chain'] is True and report['conditional_replay_performed'] is False
            and report['candidate_intermediate_inputs'] is False and report['candidate_gpu_execution'] is False
            and report['repeat_passes_byte_equal'] is True and report['captured_stages_per_pass'] == 33
            and report['numerical_acceptance'] is False and report['acceptance_threshold'] is None,
            'genuine position-zero framework capture, not a conditional oracle')
    require(type(report['passes']) is list and len(report['passes']) == 2, 'two independent framework passes')
    passes = []
    for ordinal, record in enumerate(report['passes'], 1):
        require(record['ordinal'] == ordinal and record['position'] == 0 and record['input_token'] == 9112
                and record['fresh_cache'] is True and set(record['stages']) == set(SHAPES), 'genuine pass identity/roster')
        values = {}
        for name, shape in SHAPES.items():
            stage = record['stages'][name]
            require(set(stage) == {'dtype', 'shape', 'pin'} and stage['dtype'] == 'bfloat16'
                    and stage['shape'] == list(shape), 'observed BF16 shape without casting')
            count = 1
            for dimension in shape:
                count *= dimension
            values[name] = checked_read(read, stage['pin'], count * 2)
            diagnostics.words(values[name], count)
        require(all(values[a] == values[b] for a, b in JOINS), 'actual framework producer/consumer joins')
        require(sha(values['embedding']) == EMBEDDING_SHA, 'authentic original token9112 embedding row')
        require(values['rotary-cos'] == b'\x80\x3f' * 128 and values['rotary-sin'] == b'\0\0' * 128,
                'actual position-zero cosine1/sine+0, not a general rotary equivalence')
        passes.append(values)
    require(passes[0] == passes[1], 'actual repeat stage bytes, not merely claimed equality')
    return passes[0]


def original_framework_hidden(report, read, diagnostics):
    require(report['schema'] == 'ferric-p224-rearm-four-framework-reference-v1' and report['status'] == 'PASS'
            and report['model_id'] == MODEL and report['bundle_id'] == BUNDLE
            and report['mode'] == 'teacher_forced' and report['selected_input_tokens'] == TOKENS
            and report['repeat_passes_byte_equal'] is True and report['candidate_intermediate_inputs'] is False,
            'original genuine TF4 framework report')
    require(len(report['passes']) == 2, 'two original independent passes')
    values = []
    for ordinal, record in enumerate(report['passes'], 1):
        require(record['ordinal'] == ordinal and record['fresh_cache'] is True and len(record['cases']) == 4,
                'original four-forward pass identity')
        case = record['cases'][0]
        require(case['record']['input_token'] == 9112, 'original position-zero token')
        raw = checked_read(read, case['payload'], diagnostics.PAYLOAD_BYTES)
        hidden = diagnostics.validate_case(case['record'], raw, 0)['layer0-hidden']
        require(case['tensors']['layer0-hidden'] == {'bytes': 8192, 'sha256': sha(hidden)}, 'original layer0 slice hash')
        values.append(hidden)
    require(values[0] == values[1], 'original framework layer0 actual repeat equality')
    return values[0]


def current_hidden(complete, request, read, diagnostics):
    require(complete['schema'] == 'ferric-p228-down2-clock-gpu-v1' and complete['passed'] is True
            and complete['failures'] == [] and complete['native_attempts'] == 1 and complete['retries'] == 0
            and complete['checked']['all_payloads_tokens_and_tensors_equal'] is True
            and complete['checked']['recorded_close_and_owner_reap_checked'] is True,
            'actual successful current native observation; caller verifies its receipt pin')
    require(document(checked_read(read, complete['request'], 1 << 20)) == request,
            'actual native request FilePin')
    require(request['schema'] == 'FerricFinitePrefixDecodeDeviceClockRequestV2', 'current clock request')
    value = request['decode']
    require(value['mode'] == 'teacher_forced'
            and wire(value['expected_model_id']) == MODEL and wire(value['expected_bundle_id']) == BUNDLE
            and value['prefix_image']['bytes'] == 53560 and wire(value['prefix_image']['sha256']) == V7
            and value['tiles_image']['bytes'] == 33112 and wire(value['tiles_image']['sha256']) == DOWN2,
            'current same-model/token workload with explicit V7 prefix and Down2 image')
    require(set(value['prompt']) == {'manifest', 'text', 'tokens'}, 'actual native prompt contract')
    def prompt_pin(role):
        pin = value['prompt'][role]
        require(type(pin) is dict and set(pin) == {'path', 'bytes', 'sha256'}, 'actual prompt wire FilePin')
        return dict(pin, sha256=wire(pin['sha256']))
    tokens_raw = checked_read(read, prompt_pin('tokens'), 8192)
    require(len(tokens_raw) == 8192, 'full original 2048-token prompt extent')
    tokens = list(struct.unpack('<2048I', tokens_raw))
    manifest = document(checked_read(read, prompt_pin('manifest'), 1 << 20))
    require(manifest['schema'] == 'FerricQwen3LongPromptV1' and manifest['input_token_ids'] == tokens
            and manifest['input_tokens'] == 2048 and manifest['add_special_tokens'] is False
            and manifest['chat_template'] is None and tokens[:4] == TOKENS
            and all(token < 151643 for token in tokens), 'authentic prompt IDs and first-four selection')
    observed = complete['checked']['structural']
    require(observed['mode'] == 'teacher_forced' and observed['input_tokens'] == TOKENS
            and observed['positions'] == [0, 1, 2, 3] and observed['logical_generations'] == [1, 2, 3, 4]
            and type(observed['output_tokens']) is list and len(observed['output_tokens']) == 4,
            'recorded native token/position/generation identity')
    raw = checked_read(read, complete['retained_native']['observation-0.bin'], diagnostics.PAYLOAD_BYTES)
    record = dict(generation=1, position=0, input_token=9112, output_token=observed['output_tokens'][0])
    hidden = diagnostics.validate_case(record, raw, 0)['layer0-hidden']
    # The native payload's actual first 8192 bytes are compared, not a recomputed
    # residual or a tensor value inferred from a previous comparison report.
    return hidden


def same_model_sources(left, right):
    def records(value):
        require(type(value) is dict and value, 'original model source roster')
        return {name: (row['bytes'], row['sha256']) for name, row in value.items()}
    require(records(left['model_sources']) == records(right['model_sources']), 'same complete original checkpoint bytes')


def diagnostic(diagnostics, name, reference, actual):
    value = diagnostics.compare_tensor(reference, actual)
    return dict(stage=name, reference_sha256=sha(reference), candidate_sha256=sha(actual),
                byte_equal=reference == actual, **value)


def compare_primary(framework, original, native, diagnostics):
    require(type(framework) is bytes and type(original) is bytes and type(native) is bytes
            and len(framework) == len(original) == len(native) == 8192, 'same layer0 output geometry')
    return dict(schema='ferric-p228-layer0-genuine-primary-diagnostic-v1', position=0, input_token=9112,
        original_framework_repeat=diagnostic(diagnostics, 'new-framework-vs-original-framework', original, framework),
        current_native=diagnostic(diagnostics, 'current-V7-Down2-layer0-hidden', framework, native),
        original_framework_matches_new=framework == original,
        genuine_independent_framework_outputs=True, candidate_intermediate_inputs=False,
        conditional_replay_performed=False, receipt_authentication=False,
        numerical_acceptance=False, acceptance_threshold=None, full_model_correctness=False,
        production_authority=False, performance_measured=False, gpu_execution=False)


def compare_retained(framework_pin, original_pin, native_pin, read, diagnostics):
    """Hash supplied records/stages; caller owns outer lifecycle qualification.

    In particular, framework_pin must come from the successful, reaped outer
    layer0 launcher, not merely from a still-live inner capture writer.
    """
    framework = document(checked_read(read, framework_pin, 4 << 20))
    original = document(checked_read(read, original_pin, 4 << 20))
    native = document(checked_read(read, native_pin, 4 << 20))
    same_model_sources(framework, original)
    values = read_framework(framework, read, diagnostics)
    old = original_framework_hidden(original, read, diagnostics)
    request = document(checked_read(read, native['request'], 1 << 20))
    current = current_hidden(native, request, read, diagnostics)
    result = compare_primary(values['layer0-hidden'], old, current, diagnostics)
    result['inputs'] = dict(framework_capture=framework_pin, original_framework=original_pin, current_native=native_pin)
    result['comparison_meaning'] = 'genuine independent chains; differences are diagnostic, not accepted or fitted'
    return result, values


def historical_views(framework, capture, physical_page, diagnostics):
    """Optional historical V227 hypothesis localization, not current V7 stages.

    Caller supplies the authenticated original position-zero physical page from
    the historical input record; no identity-page or old standalone-page guess.
    """
    require(type(capture) is bytes and len(capture) == 9670656
            and type(physical_page) is int and 0 <= physical_page < 144, 'historical full capture/page')
    require(set(framework) == set(SHAPES), 'complete framework values from read_framework')
    results, offset = [], 0
    for rank in range(2):
        parts = {}
        for name, count in HISTORICAL_LAYOUT:
            parts[name] = capture[offset:offset + count]
            offset += count
        q = framework['q-projection'][rank * 4096:(rank + 1) * 4096]
        k = framework['k-projection'][rank * 1024:(rank + 1) * 1024]
        v = framework['v-projection'][rank * 1024:(rank + 1) * 1024]
        start = physical_page * 16 * 1024
        pairs = {
            'norm': (framework['input-norm'], parts['norm']),
            'qkv': (q + k + v, parts['qkv']),
            'query': (framework['rotary-q'][rank * 4096:(rank + 1) * 4096], parts['query']),
            'key-current': (framework['cache-key'][rank * 1024:(rank + 1) * 1024], parts['key-cache'][start:start + 1024]),
            'value-current': (framework['cache-value'][rank * 1024:(rank + 1) * 1024], parts['value-cache'][start:start + 1024]),
            'attention': (framework['attention-output'][rank * 4096:(rank + 1) * 4096], parts['attention']),
            'first-residual': (framework['first-residual'], parts['first-residual']),
            'mlp-norm': (framework['post-norm'], parts['mlp-norm']),
            'gate': (framework['gate'][rank * 12288:(rank + 1) * 12288], parts['gate']),
            'up': (framework['up'][rank * 12288:(rank + 1) * 12288], parts['up']),
            'activation-product': (framework['product'][rank * 12288:(rank + 1) * 12288], parts['activation']),
            'final-hidden': (framework['layer0-hidden'], parts['final-hidden']),
        }
        results.append(dict(rank=rank, comparisons=[diagnostic(diagnostics, name, a, b) for name, (a, b) in pairs.items()]))
    require(offset == len(capture), 'exact historical 28-array partition')
    return dict(schema='ferric-p228-layer0-historical-genuine-diagnostic-v1', rows=results,
                historical_V227_only=True, current_V7_intermediate_evidence=False,
                cumulative_chain_differences_not_isolated_operator_errors=True,
                conditional_replay_performed=False, receipt_authentication=False,
                numerical_acceptance=False, acceptance_threshold=None, performance_measured=False)
