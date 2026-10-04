"""Pure per-profile numerical checks; no launch, receipt, or acceptance."""
import hashlib
from pathlib import Path
import sys
import types

sys.dont_write_bytecode = True
SCHEMA = 'ferric-p228-prefix-profile-conditional-numerical-v1'
PLAN_SCHEMA = 'ferric-p228-prefix-profile-numerical-inputs-v1'
PROFILES = ('baseline_v5', 'tiles_v6')
REFERENCE_PINS = {
    'stage': 'd03e52b797ab2f969f51829038bb4a3669a219aea3178f8bfc503d907491fdcb',
    'sidecar': 'b0423467ddda3f55d9e3fb33d5dab186312695d03445db3809f4e903c3c328d8',
}
FALSE_FIELDS = (
    'independent_numerical_acceptance', 'full_prefix_acceptance',
    'full_model_acceptance', 'gpu_execution_verified',
    'capture_provenance_verified', 'runtime_premises_discharged',
    'arithmetic_prerequisites_verified', 'historical_kv_numerics_checked',
    'untouched_kv_bytes_checked', 'performance_claim', 'production_authority',
)


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def _module(path, expected, name):
    require(path.is_absolute() and path.resolve(strict=True) == path
            and path.is_file() and not path.is_symlink(), 'canonical reference module')
    with path.open('rb') as stream:
        raw = stream.read((1 << 20) + 1)
    require(0 < len(raw) <= 1 << 20 and sha(raw) == expected, 'pinned reference module')
    result = types.ModuleType(name)
    result.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), result.__dict__)
    return result


def helpers(stage_root=None, sidecar_root=None):
    """Load unchanged references and their own closed helper/policy manifests."""
    require(not sys.flags.optimize, 'reference assertions must remain enabled')
    parent = Path(__file__).resolve().parent.parent
    stage_root = parent / 'p227-prefix-stage-numerical-v1' if stage_root is None else Path(stage_root)
    sidecar_root = parent / 'p227-prefix-numerical-sidecar-v1' if sidecar_root is None else Path(sidecar_root)
    S = _module(stage_root / 'compare_prefix.py', REFERENCE_PINS['stage'], 'profile_stage')
    N = _module(sidecar_root / 'compare_numerical.py', REFERENCE_PINS['sidecar'], 'profile_sidecar')
    V, M, _, H, fixtures = S.helpers()
    sidecar_validation, A, O = N.helpers()
    require(V.EXTENTS == sidecar_validation.EXTENTS
            and V.STAGES == sidecar_validation.STAGES
            and V.CAPTURE_BYTES == sidecar_validation.CAPTURE_BYTES,
            'same complete capture layout')
    require(M.np.__version__ == O.np.__version__ == '2.2.6', 'retained NumPy2.2.6')
    return S, V, M, N, H, fixtures, A, O


def compare_profile(plan, read, frozen):
    """Check one profile's two ranks, conditional on its actual preceding stages.

    The caller authenticates the native child and supplies the corresponding
    input hashes/capture pins. This function verifies bytes and math, not that
    the files came from a GPU. There is deliberately no result-file writer.
    """
    require(not sys.flags.optimize, 'reference assertions must remain enabled')
    S, V, M, N, H, fixtures, A, O = frozen
    V.keys(plan, 'schema profile case baseline_request ranks')
    require(plan['schema'] == PLAN_SCHEMA and type(plan['profile']) is str
            and plan['profile'] in PROFILES, 'closed single-profile numerical plan')
    history, position = V.case_scope(plan['case'])
    require(type(plan['ranks']) is list and len(plan['ranks']) == 2, 'two ordered ranks')
    records = {}

    def checked_read(record, maximum=64 << 10):
        V.pin(record, maximum)
        previous = records.setdefault(record['path'], dict(record))
        require(previous == record, 'conflicting original input identity')
        raw = read(record, maximum)
        require(type(raw) is bytes and len(raw) == record['bytes']
                and sha(raw) == record['sha256'], 'exact input FilePin bytes')
        return raw

    baseline = N.document(V, checked_read, plan['baseline_request'])
    require(baseline.get('schema') == 'fe2o3-qwen-resident-prefix-tp2-request-v1'
            and baseline.get('history_kind') == history
            and type(baseline.get('position')) is int and baseline['position'] == position
            and type(baseline.get('prefix_requests')) is list
            and len(baseline['prefix_requests']) == 2, 'actual baseline input scope')
    for record in baseline['prefix_requests']:
        V.pin(record, 64 << 10)
    require(len({record['path'] for record in baseline['prefix_requests']}) == 2,
            'distinct rank source requests')
    for record in baseline['prefix_requests']:
        request = N.document(V, checked_read, record)
        require(type(request.get('position')) is int and request['position'] == position,
                'integer source request position')
    inputs = S.expected_inputs(V, N, checked_read, baseline, plan['case'], fixtures)
    capture_paths = set()
    for rank, row in enumerate(plan['ranks']):
        V.keys(row, 'rank capture input_sha256 output_weights')
        require(type(row['rank']) is int and row['rank'] == rank, 'explicit ordered rank')
        V.pin(row['capture'], V.CAPTURE_BYTES)
        require(row['capture']['bytes'] == V.CAPTURE_BYTES
                and row['capture']['path'] not in capture_paths, 'distinct full rank capture')
        capture_paths.add(row['capture']['path'])
        require(type(row['input_sha256']) is list and len(row['input_sha256']) == 7,
                'seven actual native input hashes')
        for digest in row['input_sha256']:
            V.digest(digest)
        require([record['bytes'] for record in inputs[rank]] == V.EXTENTS[:6]
                and [record['sha256'] for record in inputs[rank]] == row['input_sha256'][:6],
                'authentic first six rank inputs')
        V.pin(row['output_weights'], V.EXTENTS[6])
        require(row['output_weights']['bytes'] == V.EXTENTS[6]
                and row['output_weights']['sha256'] == N.WEIGHTS[rank]
                == row['input_sha256'][6], 'authentic rank-local O weight shard')

    rows = []
    for rank, row in enumerate(plan['ranks']):
        raw_inputs = [checked_read(record, record['bytes']) for record in inputs[rank]]
        qkv_files = fixtures['qkv-manifest.json']['files']
        require(sha(raw_inputs[3][:256]) == qkv_files['query-norm.bf16']['sha256']
                and sha(raw_inputs[3][256:]) == qkv_files['key-norm.bf16']['sha256'],
                'authentic Q/K learned head-weight halves')
        raw_weights = checked_read(row['output_weights'], V.EXTENTS[6])
        V.finite(raw_weights, 2)
        capture = checked_read(row['capture'], V.CAPTURE_BYTES)
        stages = N.split_capture(V, capture)
        require(len(stages) == 7 and [len(raw) for raw in stages] == V.EXTENTS[7:],
                'all seven complete output stages')
        # Entire KV caches are hashed, but future poison must not be treated as
        # a computed value. Native validate_run owns untouched-byte comparison.
        for stage in (0, 1, 2, 5, 6):
            V.finite(stages[stage], 4 if stage == 6 else 2)
        prefix = H.check_rank(M, raw_inputs, stages, position)
        query, keys, values, attention, partial = stages[2:]
        logical_key = N.logical_cache(V, keys, position)
        logical_value = N.logical_cache(V, values, position)
        reference, maxima = A.dense_reference(
            N.words(query), N.words(logical_key), N.words(logical_value), position)
        attention_checked = A.compare(N.words(attention), reference, maxima, position)
        weights = O.np.frombuffer(raw_weights, dtype='<u2').reshape(4096, 2048)
        actual_attention = O.np.frombuffer(attention, dtype='<u2')
        exact, bound, _, _ = O.reference(weights, actual_attention)
        output_checked = N.check_partial(partial, exact, bound)
        rows.append(dict(
            profile=plan['profile'], rank=rank, capture=dict(row['capture']),
            inputs=[dict(record) for record in inputs[rank]],
            stage_sha256=[sha(raw) for raw in stages],
            conditioning=dict(query_sha256=sha(query), causal_key_sha256=sha(logical_key),
                              causal_value_sha256=sha(logical_value),
                              attention_sha256=sha(attention), output_weights=dict(row['output_weights'])),
            prefix=prefix, attention=attention_checked, output_partial=output_checked,
        ))
    return dict(
        schema=SCHEMA, authority='none', profile=plan['profile'], case=plan['case'],
        conditional_operator_checks_passed=True, paired_comparison_performed=False,
        full_capture_bytes_hashed=True, current_kv_append_checked=True,
        prefix_policy_sha256=S.POLICY_SHA, attention_policy_sha256=N.ATTENTION_POLICY,
        output_reference_sha256=N.HELPERS['output_reference.py'],
        reference_entrypoint_sha256=dict(REFERENCE_PINS),
        required_prefix_prerequisites=list(S.PREREQUISITES),
        required_attention_output_prerequisites=list(N.PREREQUISITES),
        input_pins={path: dict(record) for path, record in records.items()}, rows=rows,
        **{key: False for key in FALSE_FIELDS},
    )
