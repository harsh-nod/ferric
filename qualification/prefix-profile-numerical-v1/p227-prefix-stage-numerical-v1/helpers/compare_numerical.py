"""Conditional P214 attention/P215 O checks; never a GPU or model verifier."""
import hashlib
import json
import math
import os
from pathlib import Path
import stat
import struct
import sys
import types

sys.dont_write_bytecode = True
SCHEMA = 'ferric-p227-prefix-conditional-numerical-v1'
CLASS = 'bf16-wave64-xor-separate-f32-online-attention-o2048-v1'
HELPERS = {
    'validation.py': 'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1',
    'attention_reference.py': '99d61a03e5ad31642f322dc614c9098b26a8374a3f83c55c8ccecce3ec92466c',
    'output_reference.py': '34d86aa6e02f872b219cbb105c335633ca0dc8ed666e97a29376606cd97137cc',
}
ATTENTION_POLICY = '438b10d7cf2bdc7cd693024edcfa7f3f84929e4a852787d72964c43263cd0fc2'
WEIGHTS = (
    '695a205027c01cc6bd7238b76d8ca2e503f508f4be7d6ad7f6dfe427a0c2fb8e',
    '79bca2f8b04191dd004a3248160b1e97848d5b1613b72bfed63ef8a944ec4b4c',
)
PREREQUISITES = ['binary32-rne-gradual-underflow', 'separate-dot-multiply-add',
    'wave64-xor-1-2-4-8-16-32', 'ascending-causal-online-attention',
    'reviewed-ocml-exp', 'attention-f32-then-bf16-rne', 'o-k2048-f32-partial']
FALSE_FIELDS = ('full_prefix_acceptance', 'full_model_acceptance', 'gpu_execution_verified',
                'runtime_premises_discharged', 'performance_claim', 'production_authority')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def json_bytes(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode('ascii')


def raw_file(path, maximum):
    path = Path(path)
    require(path.is_absolute() and str(path) == str(path.resolve(strict=True)), 'canonical regular file')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= maximum, 'bounded regular file')
        chunks, count = [], 0
        while True:
            block = os.read(fd, min(1 << 20, maximum + 1 - count))
            if not block:
                break
            count += len(block)
            require(count <= maximum, 'file grew beyond bound')
            chunks.append(block)
        after = os.fstat(fd)
        final_path = path.lstat()
        identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        require(stat.S_ISREG(final_path.st_mode) and str(path) == str(path.resolve(strict=True))
            and identity(before) == identity(after) == identity(final_path), 'input changed while reading')
        raw = b''.join(chunks)
        require(len(raw) == before.st_size, 'exact file extent')
        return raw
    finally:
        os.close(fd)


def helpers():
    require(not sys.flags.optimize, 'frozen reference assertions must remain enabled')
    modules = []
    for name, sha in HELPERS.items():
        path = Path(__file__).resolve().with_name(name)
        raw = raw_file(path, 1 << 20)
        require(sha is not None and digest(raw) == sha, 'exact frozen numerical helper')
        module = types.ModuleType('p227_sidecar_' + name[:-3])
        module.__file__ = str(path)
        exec(compile(raw, str(path), 'exec'), module.__dict__)
        modules.append(module)
    require(modules[2].np.__version__ == '2.2.6', 'retained NumPy arithmetic environment')
    policy = raw_file(Path(__file__).resolve().with_name('attention-policy.json'), 64 << 10)
    require(digest(policy) == ATTENTION_POLICY and modules[0].parse(policy) == modules[1].POLICY,
            'exact preregistered attention policy and unchanged helper constants')
    return tuple(modules)


class Reader:
    def __init__(self, validation):
        self.validation, self.records = validation, {}

    def __call__(self, record, maximum=64 << 10):
        self.validation.pin(record, maximum)
        raw = raw_file(record['path'], maximum)
        require(len(raw) == record['bytes'] and digest(raw) == record['sha256'], 'input FilePin mismatch')
        old = self.records.setdefault(record['path'], dict(record))
        require(old == record, 'conflicting original file identity')
        return raw

    def recheck(self):
        for record in list(self.records.values()):
            self(record, max(record['bytes'], 64 << 10))


def document(V, read, record):
    return V.parse(read(record, 64 << 10))


def review(V, value, requested, baseline):
    V.keys(value, 'schema authority arithmetic_class baseline_image_sha256 tiles_image_sha256 '
        'source_lineage_review isa_review attention_policy_sha256 output_reference_sha256 '
        'prerequisites reviewed production_authority notes')
    require(value['schema'] == 'ferric-p227-prefix-numerical-prerequisites-v1'
        and value['authority'] == 'none' and value['arithmetic_class'] == CLASS,
        'closed non-reassociated numerical class')
    require(value['baseline_image_sha256'] == baseline['producer']['sha256']
        and value['tiles_image_sha256'] == requested['tiles']['object']['sha256']
        and value['source_lineage_review'] == requested['reviews'][0]
        and value['isa_review'] == requested['reviews'][2], 'actual image and arithmetic reviews')
    require(value['attention_policy_sha256'] == ATTENTION_POLICY
        and value['output_reference_sha256'] == HELPERS['output_reference.py']
        and value['prerequisites'] == PREREQUISITES, 'unchanged fixed numerical prerequisites')
    require(value['reviewed'] is True and value['production_authority'] is False,
            'explicit engineering review only')
    require(type(value['notes']) is str and 0 < len(value['notes'].strip())
        and len(value['notes'].encode()) <= 16384, 'bounded substantive numerical review')


def split_capture(V, raw):
    require(type(raw) is bytes and len(raw) == V.CAPTURE_BYTES, 'exact capture extent')
    stages, offset = [], 0
    for size in V.EXTENTS[7:]:
        stages.append(raw[offset:offset + size]); offset += size
    return stages


def logical_cache(V, raw, position):
    require(type(position) is int and 0 <= position < 2304, 'causal position')
    require(type(raw) is bytes and len(raw) == 2304 * 512 * 2, 'full physical cache extent')
    rows = []
    for token in range(position + 1):
        slot = ((token // 16 * 5 + 7) % 144) * 16 + token % 16
        row = raw[slot * 1024:(slot + 1) * 1024]
        V.finite(row, 2)
        rows.append(row)
    return b''.join(rows)


def words(raw):
    require(type(raw) is bytes and len(raw) % 2 == 0, 'BF16 word bytes')
    return [word for (word,) in struct.iter_unpack('<H', raw)]


def check_partial(raw, exact, bound):
    require(type(raw) is bytes and len(raw) == 4096 * 4
        and len(exact) == len(bound) == 4096, 'conditional O extents')
    maximum_error, maximum_ratio = 0.0, 0.0
    for index, ((actual,), reference, allowance) in enumerate(zip(struct.iter_unpack('<f', raw), exact, bound)):
        reference, allowance = float(reference), float(allowance)
        require(all(math.isfinite(x) for x in (actual, reference, allowance)) and allowance >= 0,
                'finite O partial/reference/bound')
        error = abs(float(actual) - reference)
        upper = 0.0 if error == 0 else math.nextafter(error, math.inf)
        require(upper <= allowance, 'conditional O error bound at element ' + str(index))
        maximum_error = max(maximum_error, upper)
        maximum_ratio = max(maximum_ratio, 0.0 if allowance == 0 else upper / allowance)
    return dict(elements=4096, max_abs_error_upper=maximum_error, max_bound_ratio=maximum_ratio)


def compare(plan, read, frozen):
    require(not sys.flags.optimize, 'frozen reference assertions must remain enabled')
    V, A, O = frozen
    V.keys(plan, 'schema case request inspection observation numerical_review output_weights')
    require(plan['schema'] == 'ferric-p227-prefix-numerical-inputs-v1', 'input schema')
    case = plan['case']; _, position = V.case_scope(case)
    requested = document(V, read, plan['request'])
    baseline = document(V, read, requested['baseline_request'])
    V.request(requested, baseline, case)
    for kind, record in zip(V.KINDS, requested['reviews']):
        V.review(document(V, read, record), kind, requested, baseline, case)
    review(V, document(V, read, plan['numerical_review']), requested, baseline)
    inspected = document(V, read, plan['inspection'])
    observed = document(V, read, plan['observation'])
    captures = {}
    require(type(observed.get('captures')) is list and len(observed['captures']) == 2, 'two capture profiles')
    for pair in observed['captures']:
        require(type(pair) is list and len(pair) == 2, 'two capture ranks')
        for record in pair:
            require(record['path'] not in captures, 'distinct closed captures')
            captures[record['path']] = read(record, V.CAPTURE_BYTES)
    parity = V.observation(observed, inspected, requested, plan['request'], baseline, case, captures)
    require(type(plan['output_weights']) is list and len(plan['output_weights']) == 2, 'ordered O shards')
    weights = []
    for rank, record in enumerate(plan['output_weights']):
        V.pin(record, 16777216)
        require(record['bytes'] == 16777216 and record['sha256'] == WEIGHTS[rank]
            and observed['input_sha256'][rank][6] == record['sha256'], 'authentic rank-local actual O input')
        raw = read(record, 16777216); V.finite(raw, 2)
        weights.append(O.np.frombuffer(raw, dtype='<u2').reshape(4096, 2048))
    rows = []
    for profile, name in enumerate(('baseline_v5', 'tiles_v6')):
        for rank in range(2):
            record = observed['captures'][profile][rank]
            stages = split_capture(V, captures[record['path']])
            query, keys, values, attention, partial = stages[2:]
            V.finite(query, 2); V.finite(attention, 2); V.finite(partial, 4)
            logical_key = logical_cache(V, keys, position)
            logical_value = logical_cache(V, values, position)
            reference, maxima = A.dense_reference(words(query), words(logical_key), words(logical_value), position)
            attention_checked = A.compare(words(attention), reference, maxima, position)
            actual_attention = O.np.frombuffer(attention, dtype='<u2')
            exact, bound, _, _ = O.reference(weights[rank], actual_attention)
            output_checked = check_partial(partial, exact, bound)
            rows.append(dict(profile=name, rank=rank, capture=record,
                conditioning=dict(query_sha256=digest(query), causal_key_sha256=digest(logical_key),
                    causal_value_sha256=digest(logical_value), attention_sha256=digest(attention),
                    output_weights=plan['output_weights'][rank]),
                attention=attention_checked, output_partial=output_checked))
    return dict(schema=SCHEMA, authority='none', case=case, arithmetic_class=CLASS,
        scheduling_bitwise_parity=parity['bitwise_match'], conditional_operator_checks_passed=True,
        raw_observation_claims_closed=True, attention_policy_sha256=ATTENTION_POLICY,
        output_reference_sha256=HELPERS['output_reference.py'], numerical_review=plan['numerical_review'],
        rows=rows, **{key: False for key in FALSE_FIELDS})


def main(args):
    require(len(args) == 3, 'PLAN_PATH PLAN_SHA NEW_RESULT_PATH')
    frozen = helpers(); V = frozen[0]; read = Reader(V)
    path, sha, output = args
    V.digest(sha)
    raw = raw_file(path, 64 << 10)
    plan_pin = dict(path=str(Path(path)), bytes=len(raw), sha256=sha)
    plan = document(V, read, plan_pin)
    destination = Path(output)
    require(destination.is_absolute() and str(destination) == str(destination.resolve())
        and not os.path.lexists(destination), 'fresh canonical result path')
    result = compare(plan, read, frozen)
    read.recheck()
    result.update(inputs=plan_pin, input_pins=dict(read.records))
    encoded = json_bytes(result)
    require(len(encoded) <= 64 << 10, 'bounded sidecar result')
    fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as stream:
        stream.write(encoded); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(result=dict(path=str(destination), bytes=len(encoded), sha256=digest(encoded)),
                          gpu_execution=False, full_model_acceptance=False), sort_keys=True))


if __name__ == '__main__':
    main(sys.argv[1:])
