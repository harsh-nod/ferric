"""Pure same-side-gated stage diagnostics; callers authenticate both capture streams."""
import hashlib
import struct

import diagnostics as D
import observer as O

POSITIONS = 6
NATIVE_EXTENTS = {
    'input': 8192, 'cache_metadata': 580, 'rotary': 512,
    'input_normalized': 8192, 'raw_qkv': 6144, 'query': 4096,
    'used_key': None, 'used_value': None, 'attention': 4096,
    'output_partial': 16384, 'first_residual': 8192, 'post_normalized': 8192,
    'gate': 12288, 'up': 12288, 'activation': 12288, 'down_partial': 16384,
    'final_hidden': 8192,
}
F32 = {'rotary', 'output_partial', 'down_partial'}
ORDER = {
    'input': (0, 'embedding'), 'input_normalized': (1, 'input_norm'),
    'q_projection': (2, 'qkv_projection'), 'k_projection': (2, 'qkv_projection'),
    'v_projection': (2, 'qkv_projection'), 'query': (3, 'qk_norm_rope_and_cache'),
    'current_key': (3, 'qk_norm_rope_and_cache'), 'current_value': (3, 'qk_norm_rope_and_cache'),
    'used_key': (3, 'qk_norm_rope_and_cache'), 'used_value': (3, 'qk_norm_rope_and_cache'),
    'attention': (4, 'attention'), 'first_residual': (5, 'o_projection_and_residual'),
    'post_normalized': (6, 'post_norm'), 'gate': (7, 'gate_up_projection'),
    'up': (7, 'gate_up_projection'), 'activation': (8, 'silu_times_up'),
    'final_hidden': (9, 'down_projection_and_residual'),
}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def half(raw, rank, elements):
    require(type(elements) is int and elements > 0 and elements % 2 == 0
            and type(raw) is bytes and len(raw) == elements * 2
            and type(rank) is int and rank in (0, 1),
            'exact contiguous rank half')
    return raw[rank * elements:(rank + 1) * elements]


def reference_rank(stages, position, rank):
    return {
        'input': stages['embedding'], 'input_normalized': stages['input-norm'],
        'q_projection': half(stages['q-projection'], rank, 4096),
        'k_projection': half(stages['k-projection'], rank, 1024),
        'v_projection': half(stages['v-projection'], rank, 1024),
        'query': half(stages['rotary-q'], rank, 4096),
        'current_key': half(stages['rotary-k'], rank, 1024),
        'current_value': half(stages['v-projection'], rank, 1024),
        'used_key': O.cache_rows(stages['cache-key'], position, rank),
        'used_value': O.cache_rows(stages['cache-value'], position, rank),
        'attention': half(stages['attention-output'], rank, 4096),
        'first_residual': stages['first-residual'], 'post_normalized': stages['post-norm'],
        'gate': half(stages['gate'], rank, 12288), 'up': half(stages['up'], rank, 12288),
        'activation': half(stages['product'], rank, 12288), 'final_hidden': stages['layer0-hidden'],
    }


def native_rank(stages):
    qkv = stages['raw_qkv']
    return {
        'input': stages['input'], 'input_normalized': stages['input_normalized'],
        'q_projection': qkv[:4096], 'k_projection': qkv[4096:5120], 'v_projection': qkv[5120:6144],
        'query': stages['query'], 'current_key': stages['used_key'][-1024:],
        'current_value': stages['used_value'][-1024:], 'used_key': stages['used_key'],
        'used_value': stages['used_value'], 'attention': stages['attention'],
        'first_residual': stages['first_residual'], 'post_normalized': stages['post_normalized'],
        'gate': stages['gate'], 'up': stages['up'], 'activation': stages['activation'],
        'final_hidden': stages['final_hidden'],
    }


def inputs(native, framework):
    require(type(native) is list and type(framework) is list
            and len(native) == len(framework) == POSITIONS, 'six decoded captures per side')
    previous_framework = None
    previous_native = None
    pages = None
    tokens = []
    for position, (n, f) in enumerate(zip(native, framework)):
        require(type(n) is dict and set(n) == {'position', 'input_token', 'ranks'}
                and type(f) is dict and set(f) == {'position', 'input_token', 'stages'},
                'closed decoded capture containers')
        require(type(n['position']) is type(f['position']) is int
                and n['position'] == f['position'] == position
                and type(n['input_token']) is type(f['input_token']) is int
                and 0 <= n['input_token'] == f['input_token'] < 151936,
                'same exact six-position prompt history')
        require(type(n['ranks']) is list and len(n['ranks']) == 2, 'two native ranks')
        O.validate_values(f['stages'], position, previous_framework)
        previous_framework = f['stages']
        for rank, stages in enumerate(n['ranks']):
            require(type(stages) is dict and set(stages) == set(NATIVE_EXTENTS), 'complete native role roster')
            for name, extent in NATIVE_EXTENTS.items():
                extent = extent if extent is not None else (position + 1) * 1024
                raw = stages[name]
                require(type(raw) is bytes and len(raw) == extent, 'exact native role extent: ' + name)
                if name in F32:
                    require(all(word & 0x7f800000 != 0x7f800000 for word, in struct.iter_unpack('<I', raw)),
                            'finite noncomparable native FP32')
                elif name != 'cache_metadata':
                    D.words(raw, extent // 2)
            metadata = struct.unpack('<145I', stages['cache_metadata'])
            require(metadata[0] == position and sorted(metadata[1:]) == list(range(144)),
                    'actual native position/page metadata')
            if pages is None:
                pages = metadata[1:]
            require(metadata[1:] == pages, 'stable physical pages across both ranks and six positions')
            if previous_native is not None:
                require(all(stages[role].startswith(previous_native[rank][role])
                            for role in ('used_key', 'used_value')), 'completed native KV prefix changed')
        previous_native = n['ranks']
        tokens.append(n['input_token'])
    return tokens


def dependencies(position, rank, name):
    local = lambda role: (position, rank, role)
    if name == 'input':
        return [], ['embedding_table_row'], 'embedding_lookup'
    if name == 'input_normalized':
        return [local('input')], [], 'input_rmsnorm'
    if name in ('q_projection', 'k_projection', 'v_projection'):
        return [local('input_normalized')], [], name
    if name == 'query':
        return [local('q_projection')], ['native_q_norm_output', 'comparable_rotary_values'], 'q_norm_rope_pipeline'
    if name in ('current_key', 'used_key'):
        prior = [(position - 1, rank, 'used_key')] if position else []
        return [local('k_projection')] + prior, ['native_k_norm_output', 'comparable_rotary_values'], 'k_norm_rope_and_cache'
    if name == 'current_value':
        return [local('v_projection')], [], 'current_value_cache_append'
    if name == 'used_value':
        prior = [(position - 1, rank, 'used_value')] if position else []
        return [local('v_projection')] + prior, [], 'value_cache_append_and_prior_prefix'
    if name == 'attention':
        return [local(role) for role in ('query', 'used_key', 'used_value')], [], 'attention_from_captured_qkv'
    if name == 'first_residual':
        return [(position, r, role) for r in range(2) for role in ('attention', 'input')], [], 'o_projection_tp_reduce_and_residual'
    if name == 'post_normalized':
        return [local('first_residual')], [], 'post_rmsnorm'
    if name in ('gate', 'up'):
        return [local('post_normalized')], [], name + '_projection'
    if name == 'activation':
        return [local('gate'), local('up')], [], 'silu_times_up_composite'
    if name == 'final_hidden':
        return [(position, r, role) for r in range(2) for role in ('activation', 'first_residual')], [], 'down_projection_tp_reduce_and_residual'
    raise ValueError('closed comparison role')


def first_group(rows, eligible):
    changed = [row for row in rows if row['different_words'] and eligible(row)]
    if not changed:
        return None
    key = min((row['position'], row['stage_order']) for row in changed)
    group = [row for row in changed if (row['position'], row['stage_order']) == key]
    return dict(position=key[0], stage_order=key[1], stage=group[0]['stage'],
                row_ids=[row['id'] for row in group])


def compare(native, framework):
    """Inputs are already authenticated and same-side-parity-gated; this grants no launch authority."""
    tokens = inputs(native, framework)
    rows, lookup, unavailable = [], {}, []
    for position, (n, f) in enumerate(zip(native, framework)):
        for rank in range(2):
            left = reference_rank(f['stages'], position, rank)
            right = native_rank(n['ranks'][rank])
            require(set(left) == set(right) == set(ORDER), 'exact seventeen comparable mapped roles')
            for name, reference in left.items():
                actual = right[name]
                metrics = D.compare_tensor(reference, actual)
                stage_order, stage = ORDER[name]
                row = dict(id='%d:%d:%s' % (position, rank, name), position=position, rank=rank,
                    input_token=tokens[position], role=name, stage_order=stage_order, stage=stage,
                    reference=pin(reference), native=pin(actual), metrics=metrics,
                    different_words=metrics['elements'] - metrics['exact_words'])
                rows.append(row); lookup[(position, rank, name)] = row
            for name, why in (
                    ('output_partial', 'rank-local FP32 partial is not a framework BF16 full O projection'),
                    ('down_partial', 'rank-local FP32 partial is not a framework BF16 full Down projection'),
                    ('rotary', 'native FP32 source table and framework BF16 evaluated cos/sin have different layouts and stages')):
                unavailable.append(dict(position=position, rank=rank, role=name,
                    native=pin(n['ranks'][rank][name]), comparable=False, metrics=None, reason=why))
    for row in rows:
        deps, missing, operation = dependencies(row['position'], row['rank'], row['role'])
        checked = [lookup[key] for key in deps]
        differing = [item['id'] for item in checked if item['different_words']]
        complete = bool(deps) and not missing
        row.update(operation_scope=operation, captured_prerequisites=[item['id'] for item in checked],
            differing_prerequisites=differing, unresolved_prerequisites=missing,
            same_captured_inputs=complete and not differing,
            input_relation=('propagated_input_difference_possible' if differing else
                'same_captured_inputs' if complete else 'inputs_not_fully_comparable'))
    rows.sort(key=lambda row: (row['position'], row['stage_order'], row['rank'], row['role']))
    require(len(rows) == 204 and len(unavailable) == 36, 'bounded six-position comparison census')
    return dict(schema='ferric-readiness40-causal-layer0-stage-diagnostics-v1',
        positions=list(range(6)), input_tokens=tokens, comparable_rows=rows, noncomparable_rows=unavailable,
        earliest_observed_difference=first_group(rows, lambda _: True),
        earliest_same_captured_input_difference=first_group(rows, lambda row: row['same_captured_inputs']),
        exact_rows=sum(row['different_words'] == 0 for row in rows),
        scope='own-history six-prefix captures; same-input labels are conditional on caller-authenticated common model and policy',
        composite_boundaries={'first_residual': 'O projection, TP reduction and residual addition, not O alone',
            'activation': 'SiLU times up, not standalone SiLU',
            'final_hidden': 'Down projection, TP reduction and residual addition, not Down alone'},
        framework_only_uncompared=['q-norm', 'k-norm', 'rotary-cos', 'rotary-sin',
                                  'o-projection', 'silu', 'down-projection'],
        receipt_authentication=False, same_side_parity_verified_here=False,
        framework_input_substitution=False, partials_summed_or_rounded=False,
        numerical_acceptance=False, acceptance_threshold=None, semantic_bug_claimed=False,
        full_model_correctness=False, performance_measured=False, production_authority=False)
