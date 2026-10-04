"""CPU-only source-derived checks over closed, already retained prefix captures."""
import hashlib
import json
import os
from pathlib import Path
import sys
import types

sys.dont_write_bytecode = True
SCHEMA = 'ferric-p227-prefix-stage-numerical-v1'
CLASS = 'wave4096-serial128-dual-bf16-split-half-rope-v1'
HELPERS = {
    'extract.py': '76872f3558d6f90c21b9eee3ec0ba4ebdd79168b620338faa6bdbdd8b78bdf4a',
    'reference.py': '2e41d6e5715cc561bf818fdee794e4ced74a77f93acd4961d8b15f427f196c6e',
    'validation.py': 'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1',
    'compare_numerical.py': 'b0423467ddda3f55d9e3fb33d5dab186312695d03445db3809f4e903c3c328d8',
}
INPUTS = {
    'qkv-manifest.json': '9516d017300facd7c481149f865c6a4817045d083702d05558297a82619b0eda',
    'genuine-manifest.json': 'fbf766e1ded38055e03bfd512184eb4b54f362eedc08c2ea9388b00171a041ea',
    'post-report.json': 'd6b2fdba3e4b9a1d24860d083928cfdd65faaf60d00edd53f5432c9c5b3e7f6b',
    'rotary-manifest.json': '6b99079c47aade347a4f13f197b8897b6f3d0b390ec723396489ea85c99383c9',
}
SOURCES = {
    'wave_numerics_v1.rs': '22ef471c341936b3bc6f7b5812405c60152292cef07dcbd03173bf2dd7ee49ea',
    'head_rope_numerics_v3.rs': '4b88b9ce6608c791a0df8d0d2c5166de7aad77d63e7b2c52383cce068a1996e9',
    'finite_qkv_attention_output_tiles_v6.rs': '3a18e3cb7bab842249bbbef1fd3d5470ead1e7ab5760ae2c5a3b8fae08499bcd',
    'prefix_tiles_numerics_v6.rs': '3f729b83bc50d4e252a73911ac69bb4e4e1829e1cb05ca3f0bcffa36f09475ad',
}
POLICY_SHA = 'c3f085ad8230f4dcac7a36dd2a72872f00bc454f0996467af90ea4fde69a0f8d'
MATH_SHA = '407cbf19a1c68ce4ee3d9672c6ecacb4bc42fb683f65b0841aeb47a73d477016'
PREREQUISITES = ['binary32-rne-gradual-underflow', 'bf16-rne-finite-boundaries',
    'separate-product-add-no-reassociation', 'wave64-xor-1-2-4-8-16-32',
    'sqrt-within-one-binary32-ulp', 'correctly-rounded-binary32-divide',
    'dual-bf16-normalization', 'serial128-head-squares', 'split-half-rope-no-contraction',
    'authenticated-supplied-rotary-not-trig-recomputed']
FALSE = ('full_prefix_acceptance', 'full_model_acceptance', 'gpu_execution_verified',
         'runtime_premises_discharged', 'performance_claim', 'production_authority')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def module(raw, path, name):
    result = types.ModuleType(name)
    result.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), result.__dict__)
    return result


def helpers():
    require(not sys.flags.optimize, 'assertions must remain enabled')
    root = Path(__file__).resolve().parent
    raw = {}
    for name, expected in HELPERS.items():
        data = (root / 'helpers' / name).read_bytes()
        require(sha(data) == expected, 'exact unchanged helper ' + name)
        raw[name] = data
    N = module(raw['compare_numerical.py'], root / 'helpers/compare_numerical.py', 'prefix_prior_sidecar')
    V = module(raw['validation.py'], root / 'helpers/validation.py', 'prefix_validation')
    E = module(raw['extract.py'], root / 'helpers/extract.py', 'prefix_extract')
    previous = sys.modules.get('extract')
    try:
        sys.modules['extract'] = E
        M = module(raw['reference.py'], root / 'helpers/reference.py', 'prefix_mlp_reference')
    finally:
        if previous is None:
            sys.modules.pop('extract', None)
        else:
            sys.modules['extract'] = previous
    require(M.np.__version__ == '2.2.6', 'retained NumPy2.2.6 environment')
    fixtures = {}
    for name, expected in {**INPUTS, **SOURCES}.items():
        data = N.raw_file(root / 'inputs' / name, 1 << 20)
        require(sha(data) == expected, 'exact source/metadata ' + name)
        if name in INPUTS:
            fixtures[name] = V.parse(data)
    policy = N.raw_file(root / 'policy.json', 64 << 10)
    math_raw = N.raw_file(root / 'prefix_math.py', 64 << 10)
    require(POLICY_SHA is not None and sha(policy) == POLICY_SHA
            and MATH_SHA is not None and sha(math_raw) == MATH_SHA, 'frozen policy/math identity')
    H = module(math_raw, root / 'prefix_math.py', 'prefix_stage_math')
    return V, M, N, H, fixtures


def expected_inputs(V, N, read, baseline, case, fixtures):
    history, position = V.case_scope(case)
    q = fixtures['qkv-manifest.json']['files']
    g = fixtures['genuine-manifest.json']['files']
    p = fixtures['post-report.json']['files']
    r = fixtures['rotary-manifest.json']['files']
    result = []
    for rank, record in enumerate(baseline['prefix_requests']):
        request = N.document(V, read, record)
        require(request['schema'] == 'fe2o3-qwen-wave-qkv-attention-output-source-request-v5'
                and type(request['rank']) is int and request['rank'] == rank
                and request['history_kind'] == history and request['position'] == position,
                'actual original rank request scope')
        first = ('input-pos%d.bf16' % position, g, request['genuine_fixture_directory']) if history == 'genuine' else (
            'input.bf16', q, request['fixture_directory'])
        rotary = ('rotary-pos%d.f32' % position, r, request['rotary_fixture_directory']) if history == 'genuine' else (
            'rotary-%d.f32' % position, p, request['post_fixture_directory'])
        metadata = ('metadata-pos%d.u32' % position, r, request['rotary_fixture_directory']) if history == 'genuine' else (
            'metadata-%d.u32' % position, p, request['post_fixture_directory'])
        rows = [first, ('norm-weight.bf16', q, request['fixture_directory']),
                ('packed-qkv-rank%d.bf16' % rank, q, request['fixture_directory']),
                ('head-weights.bf16', p, request['post_fixture_directory']), rotary, metadata]
        pins = []
        for name, source, directory in rows:
            item = source[name]
            pin = dict(path=directory + '/' + name, bytes=item['bytes'], sha256=item['sha256'])
            V.pin(pin); pins.append(pin)
        result.append(pins)
    require(len(result) == 2, 'two rank input rosters')
    return result


def numerical_review(V, value, requested, baseline):
    V.keys(value, 'schema authority arithmetic_class baseline_image_sha256 tiles_image_sha256 '
           'source_lineage_review isa_review source_sha256 policy_sha256 prerequisites reviewed production_authority notes')
    require(value['schema'] == 'ferric-p227-prefix-stage-prerequisites-v1'
            and value['authority'] == 'none' and value['arithmetic_class'] == CLASS
            and value['baseline_image_sha256'] == baseline['producer']['sha256']
            and value['tiles_image_sha256'] == requested['tiles']['object']['sha256']
            and value['source_lineage_review'] == requested['reviews'][0]
            and value['isa_review'] == requested['reviews'][2]
            and value['source_sha256'] == SOURCES and value['policy_sha256'] == POLICY_SHA
            and value['prerequisites'] == PREREQUISITES and value['reviewed'] is True
            and value['production_authority'] is False, 'explicit actual artifact arithmetic prerequisites')
    require(type(value['notes']) is str and 0 < len(value['notes'].strip())
            and len(value['notes'].encode()) <= 16384, 'bounded substantive review notes')


def compare(plan, read, frozen):
    require(not sys.flags.optimize, 'assertions must remain enabled')
    V, M, N, H, fixtures = frozen
    V.keys(plan, 'schema case request inspection observation numerical_review')
    require(plan['schema'] == 'ferric-p227-prefix-stage-inputs-v1', 'closed prefix-stage plan')
    case = plan['case']; _, position = V.case_scope(case)
    requested = N.document(V, read, plan['request'])
    baseline = N.document(V, read, requested['baseline_request'])
    V.request(requested, baseline, case)
    for kind, pin in zip(V.KINDS, requested['reviews']):
        V.review(N.document(V, read, pin), kind, requested, baseline, case)
    numerical_review(V, N.document(V, read, plan['numerical_review']), requested, baseline)
    inspected = N.document(V, read, plan['inspection'])
    observed = N.document(V, read, plan['observation'])
    require(type(observed.get('captures')) is list and len(observed['captures']) == 2, 'two profiles')
    captures = {}
    for pair in observed['captures']:
        require(type(pair) is list and len(pair) == 2, 'two ranks')
        for pin in pair:
            require(pin['path'] not in captures, 'distinct capture files')
            captures[pin['path']] = read(pin, V.CAPTURE_BYTES)
    parity = V.observation(observed, inspected, requested, plan['request'], baseline, case, captures)
    inputs = expected_inputs(V, N, read, baseline, case, fixtures)
    raw_inputs = []
    for rank, pins in enumerate(inputs):
        require([p['bytes'] for p in pins] == V.EXTENTS[:6]
                and [p['sha256'] for p in pins] == observed['input_sha256'][rank][:6],
                'authentic exact six actual input buffers')
        values = [read(pin, pin['bytes']) for pin in pins]
        q = fixtures['qkv-manifest.json']['files']
        require(sha(values[3][:256]) == q['query-norm.bf16']['sha256']
                and sha(values[3][256:]) == q['key-norm.bf16']['sha256'], 'authentic Q/K learned head weights')
        raw_inputs.append(values)
    rows = []
    for profile, name in enumerate(('baseline_v5', 'tiles_v6')):
        for rank in range(2):
            pin = observed['captures'][profile][rank]
            stages = N.split_capture(V, captures[pin['path']])
            checked = H.check_rank(M, raw_inputs[rank], stages, position)
            rows.append(dict(profile=name, rank=rank, capture=pin, inputs=inputs[rank],
                             preceding_stage_sha256=[sha(x) for x in stages[:2]], **checked))
    return dict(schema=SCHEMA, authority='none', case=case, policy_sha256=POLICY_SHA,
                numerical_review=plan['numerical_review'], scheduling_bitwise_parity=parity['bitwise_match'],
                conditional_operator_checks_passed=True, historical_kv_numerics_checked=False,
                attention_and_o_checked=False, rows=rows, **{key: False for key in FALSE})


def main(args):
    require(len(args) == 3, 'PLAN_PATH PLAN_SHA NEW_RESULT_PATH')
    frozen = helpers(); V, _, N, _, _ = frozen
    read = N.Reader(V)
    path, digest, output = args; V.digest(digest)
    raw = N.raw_file(path, 64 << 10)
    pin = dict(path=str(Path(path)), bytes=len(raw), sha256=digest)
    plan = N.document(V, read, pin)
    destination = Path(output)
    require(destination.is_absolute() and str(destination) == str(destination.resolve())
            and not os.path.lexists(destination), 'fresh canonical result path')
    result = compare(plan, read, frozen)
    read.recheck()
    result.update(inputs=pin, input_pins=dict(read.records))
    encoded = N.json_bytes(result)
    require(len(encoded) <= 64 << 10, 'bounded prefix-stage result')
    fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as stream:
        stream.write(encoded); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(result=dict(path=str(destination), bytes=len(encoded), sha256=sha(encoded)),
                          gpu_execution=False, full_model_acceptance=False), sort_keys=True))


if __name__ == '__main__':
    main(sys.argv[1:])
