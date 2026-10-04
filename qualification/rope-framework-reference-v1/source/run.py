"""CPU-only installed-framework RoPE probe; no model, candidate, or GPU launch."""
import argparse
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import resource
import signal
import socket
import stat
import struct
import sys
from types import SimpleNamespace

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ENV = E.parent / 'qwen3-long-reference-env-v1/venv'
SITE = ENV / 'lib/python3.10/site-packages'
POSITIONS = (0, 1, 2, 3, 2047, 2048, 2303)
CAPTURE_SHA = 'cf7512025bb469e06f87c32f788da807b4e6607297b98bb0b33c4135d1e2c78e'
OUTER_SHA = '46fd9acbca798f05bc737a651e6fba54e65c78688e2d425a8287eef402065edc'
ORACLE = E / 'p228-independent-layer-reference-v1/helpers/residual_oracle.py'
ORACLE_SHA = '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'
SOURCES = {
    'qwen': ('transformers/models/qwen3/modeling_qwen3.py',
             '704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2'),
    'rope': ('transformers/modeling_rope_utils.py',
             'c28b3e88edca8fdb5497e5c36091bf753db49bd94ace33a84e9f9c61cbf66032'),
}
TRACKED = {}


def require(value, message):
    if not value:
        raise ValueError(message)


def read(path, digest, size=None, maximum=2 << 20):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical file path')
    st = path.lstat()
    require(stat.S_ISREG(st.st_mode) and 0 < st.st_size <= maximum, 'bounded regular file')
    body = path.read_bytes()
    require(len(body) == st.st_size and (size is None or len(body) == size), 'file extent')
    require(hashlib.sha256(body).hexdigest() == digest, 'file digest: ' + str(path))
    TRACKED[str(path)] = {'path': str(path), 'bytes': len(body), 'sha256': digest}
    return body


def load_oracle(path=ORACLE):
    read(path, ORACLE_SHA)
    spec = importlib.util.spec_from_file_location('_retained_rope_integer_oracle', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def word_float(word):
    require(type(word) is int and 0 <= word < 1 << 32 and
            word & 0x7f800000 != 0x7f800000, 'finite FP32 word')
    return struct.unpack('<f', struct.pack('<I', word))[0]


def float_word(value):
    require(math.isfinite(value), 'finite real')
    word = struct.unpack('<I', struct.pack('<f', value))[0]
    word_float(word)
    return word


def multiply(left, right):
    # Every binary32 product has <=48 significant bits and exponent >=-298:
    # binary64 multiplication is exact here, including before FP32 underflow.
    return float_word(word_float(left) * word_float(right))


def words(body, width):
    require(width in (16, 32) and len(body) % (width // 8) == 0, 'word extent')
    result = list(struct.unpack('<' + str(len(body) // (width // 8)) +
                                ('H' if width == 16 else 'I'), body))
    for word in result:
        word_float(word << 16 if width == 16 else word)
    return result


def pack_words(values, width):
    return struct.pack('<' + str(len(values)) + ('H' if width == 16 else 'I'), *values)


def rotate(values, cosine, sine, oracle, materialize_products):
    require(len(values) > 0 and len(values) % 128 == 0, 'whole 128-element heads')
    require(len(cosine) == len(sine) == 128, '128-element tables')
    require(cosine[:64] == cosine[64:] and sine[:64] == sine[64:], 'duplicated half tables')
    out = []
    for index, word in enumerate(values):
        require(type(word) is int and 0 <= word < 65536, 'BF16 word')
        offset = index % 128
        paired = values[index + 64 if offset < 64 else index - 64] << 16
        if offset < 64:
            paired ^= 0x80000000
        first = multiply(word << 16, cosine[offset])
        second = multiply(paired, sine[offset])
        if materialize_products:
            first = oracle.narrow_bf16_rne(first) << 16
            second = oracle.narrow_bf16_rne(second) << 16
        out.append(oracle.narrow_bf16_rne(oracle.add_f32_rne(first, second)))
    return out


def f64_table_model(position):
    require(type(position) is int and position in POSITIONS, 'probe position')
    angles = [float(position) * math.pow(1000000.0, -pair / 64.0) for pair in range(64)]
    cosine = [float_word(math.cos(x)) for x in angles]
    sine = [float_word(math.sin(x)) for x in angles]
    return cosine + cosine, sine + sine


def compare(actual, expected):
    require(len(actual) == len(expected), 'comparison extent')
    differences = [{'index': i, 'actual_bits': a, 'expected_bits': b}
                   for i, (a, b) in enumerate(zip(actual, expected)) if a != b]
    return {'words': len(actual), 'exact_words': len(actual) - len(differences),
            'different_words': len(differences), 'first_differences': differences[:16]}


def synthetic():
    # Finite tiny/subnormal, signed zero, adjacent-to-one and cancellation inputs.
    pattern = (0, 0x8000, 1, 0x8001, 0x0080, 0x8080, 0x3f80, 0xbf80,
               0x3f81, 0xbf81, 0x3f7f, 0xbf7f, 0x3f00, 0xbf00)
    return [pattern[i % len(pattern)] for i in range(256)], [pattern[(i + 5) % len(pattern)] for i in range(128)]


def captured_inputs():
    outer_path = E / 'layer0-framework-launch-v228-v1/complete.json'
    capture_path = E / 'layer0-framework-capture-v228-v1/capture.json'
    outer = json.loads(read(outer_path, OUTER_SHA, 17449))
    capture = json.loads(read(capture_path, CAPTURE_SHA, 43427))
    require(outer['schema'] == 'ferric-p228-layer0-framework-launch-complete-v1' and
            outer['passed'] is True and outer['failures'] == [] and
            outer['native_attempts'] == 1 and outer['retries'] == 0, 'passed original framework owner')
    require(outer['reference'] == TRACKED[str(capture_path)], 'owner/capture pin')
    require(capture['schema'] == 'ferric-p228-layer0-framework-capture-v1' and
            capture['genuine_framework_chain'] is True and capture['position'] == 0 and
            capture['input_token'] == 9112 and capture['repeat_passes_byte_equal'] is True and
            len(capture['passes']) == 2, 'genuine position-zero capture')
    layouts = {'q-norm': [1, 1, 32, 128], 'k-norm': [1, 1, 8, 128],
               'rotary-q': [1, 32, 1, 128], 'rotary-k': [1, 8, 1, 128],
               'rotary-cos': [1, 1, 128], 'rotary-sin': [1, 1, 128]}
    retained = []
    for ordinal, item in enumerate(capture['passes'], 1):
        require((item['ordinal'], item['position'], item['input_token']) == (ordinal, 0, 9112), 'pass identity')
        rows = {}
        for name, shape in layouts.items():
            row = item['stages'][name]
            require(row['dtype'] == 'bfloat16' and row['shape'] == shape, 'stage shape/dtype')
            pin = row['pin']
            require(pin['path'] == str(capture_path.parent / ('pass%d-%s.bf16' % (ordinal, name))), 'stage path')
            rows[name] = words(read(pin['path'], pin['sha256'], math.prod(shape) * 2), 16)
        retained.append(rows)
    require(retained[0] == retained[1], 'selected repeated capture bodies')
    retained[0]['historical_inv_frequency_sha256'] = capture['runtime']['rotary_sha256']
    return retained[0]


def bounds():
    require(socket.gethostname() == 'asrock-1w300-g2-2b' and os.getuid() == 9661, 'ASROCK user/host')
    require(Path(sys.executable) == ENV / 'bin/python' and Path(sys.prefix) == ENV, 'original interpreter')
    require(sys.dont_write_bytecode and sys.flags.optimize == 0 and sys.flags.isolated, 'isolated -I -B Python')
    require(not os.environ.get('PYTHONPATH') and not os.environ.get('PYTHONHOME') and
            not os.environ.get('LD_PRELOAD'), 'no import/preload overrides')
    for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        require(os.environ.get(name) == '', 'GPU hidden: ' + name)
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'CPU8/9 nice10')
    require(sys.byteorder == 'little' and sys.float_info.mant_dig == 53 and
            sys.float_info.radix == 2 and sys.float_info.rounds == 1, 'IEEE binary64 RNE host')
    for kind, cap in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_NOFILE, 128),
                      (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (bound, bound))
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('180-second probe deadline')))
    signal.alarm(180)
    for name in ('OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
        os.environ[name] = '2'
    os.environ['HF_HUB_OFFLINE'] = '1'
    os.environ['TRANSFORMERS_OFFLINE'] = '1'
    os.environ['TOKENIZERS_PARALLELISM'] = 'false'


def execute(args):
    bounds()
    require(re.fullmatch(r'[0-9a-f]{64}', args.self_sha256), 'actual controller SHA')
    read(Path(__file__).resolve(), args.self_sha256)
    interpreter = read(ENV / 'bin/python', 'a2f33a6e006989270f4340528eb61f8f97366e00a5d1b602ac8672ea44fc56ae',
                       5941864, 8 << 20)
    del interpreter
    for relative, digest in SOURCES.values():
        read(SITE / relative, digest)
    oracle = load_oracle()
    corpus = captured_inputs()
    require(re.fullmatch(r'rope-framework-reference-v228-v[1-9][0-9]*', args.label), 'fresh label')
    output = E / args.label
    require(not output.exists() and not output.is_symlink(), 'fresh output')
    output.mkdir(mode=0o700)
    outputs = []

    def save(name, body):
        require(re.fullmatch(r'[a-zA-Z0-9_.-]+', name) and len(body) <= 16 << 20, 'bounded output')
        target = output / name
        with target.open('xb') as stream:
            stream.write(body)
            stream.flush()
            os.fsync(stream.fileno())
        record = {'path': str(target), 'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
        outputs.append(record)
        return record

    import torch
    import transformers
    from transformers.models.qwen3 import modeling_qwen3 as qwen
    from transformers import modeling_rope_utils as rope_utils
    require(torch.__version__ == '2.12.1+rocm7.2' and transformers.__version__ == '4.51.0', 'installed versions')
    for key, module in (('qwen', qwen), ('rope', rope_utils)):
        require(Path(module.__file__).resolve() == SITE / SOURCES[key][0], 'installed module path')
    require(qwen.ROPE_INIT_FUNCTIONS is rope_utils.ROPE_INIT_FUNCTIONS, 'actual default RoPE initializer')
    require(not torch.cuda.is_initialized(), 'no initialized GPU context')
    torch.set_num_threads(2)
    torch.set_num_interop_threads(1)
    torch.use_deterministic_algorithms(True)
    flush_supported = torch.set_flush_denormal(False)
    config = SimpleNamespace(rope_theta=1000000.0, head_dim=128, hidden_size=4096,
                             num_attention_heads=32, max_position_embeddings=40960,
                             rope_scaling=None, partial_rotary_factor=1.0)
    rotary = qwen.Qwen3RotaryEmbedding(config, device='cpu')
    require(rotary.inv_freq.device.type == 'cpu' and rotary.inv_freq.dtype == torch.float32 and
            tuple(rotary.inv_freq.shape) == (64,) and rotary.attention_scaling == 1.0, 'FP32 default frequency')

    def tensor(values, shape):
        return torch.tensor(values, dtype=torch.int32, device='cpu').to(torch.int16).view(torch.bfloat16).reshape(shape)

    def tensor_words(value, width):
        require(value.device.type == 'cpu' and value.dtype == (torch.bfloat16 if width == 16 else torch.float32), 'CPU dtype')
        raw = value.contiguous().view(torch.int16 if width == 16 else torch.int32).reshape(-1).tolist()
        result = [x & ((1 << width) - 1) for x in raw]
        words(pack_words(result, width), width)
        return result

    inv = tensor_words(rotary.inv_freq, 32)
    save('inv-frequency.f32le', pack_words(inv, 32))
    qsynthetic, ksynthetic = synthetic()
    corpora = {'genuine-pos0-conditional': (corpus['q-norm'], corpus['k-norm']),
               'synthetic-boundaries': (qsynthetic, ksynthetic)}
    rows = []
    tables = []
    for name, (qbits, kbits) in corpora.items():
        save(name + '-q.bf16le', pack_words(qbits, 16))
        save(name + '-k.bf16le', pack_words(kbits, 16))
        q = tensor(qbits, (1, len(qbits) // 128, 1, 128))
        k = tensor(kbits, (1, len(kbits) // 128, 1, 128))
        with torch.no_grad():
            for position in POSITIONS:
                cos, sin = rotary(q, torch.tensor([[position]], dtype=torch.int64, device='cpu'))
                cos16, sin16 = tensor_words(cos, 16), tensor_words(sin, 16)
                require(len(cos16) == len(sin16) == 128, 'framework coefficient shape')
                cos32, sin32 = [x << 16 for x in cos16], [x << 16 for x in sin16]
                model_cos, model_sin = f64_table_model(position)
                narrowed_cos = [oracle.narrow_bf16_rne(x) for x in model_cos]
                narrowed_sin = [oracle.narrow_bf16_rne(x) for x in model_sin]
                if name == 'genuine-pos0-conditional':
                    raw_cos, raw_sin = rotary(q.float(), torch.tensor([[position]], dtype=torch.int64, device='cpu'))
                    raw_cos_bits, raw_sin_bits = tensor_words(raw_cos, 32), tensor_words(raw_sin, 32)
                    require([oracle.narrow_bf16_rne(x) for x in raw_cos_bits] == cos16 and
                            [oracle.narrow_bf16_rne(x) for x in raw_sin_bits] == sin16, 'framework FP32-to-BF16 table join')
                    tables.append({'position': position, 'framework_cos_bf16_bits': cos16,
                                   'framework_sin_bf16_bits': sin16,
                                   'framework_companion_cos_f32_bits': raw_cos_bits,
                                   'framework_companion_sin_f32_bits': raw_sin_bits,
                                   'f64_model_cos_f32_bits': model_cos,
                                   'f64_model_sin_f32_bits': model_sin, 'f64_model_cos_bf16_bits': narrowed_cos,
                                   'f64_model_sin_bf16_bits': narrowed_sin,
                                   'cos_comparison': compare(narrowed_cos, cos16),
                                   'sin_comparison': compare(narrowed_sin, sin16)})
                framework = qwen.apply_rotary_pos_emb(q, k, cos, sin)
                for kind, bits, actual_tensor in zip(('q', 'k'), (qbits, kbits), framework):
                    actual = tensor_words(actual_tensor, 16)
                    modes = {
                        'framework': actual,
                        'independent-materialized': rotate(bits, cos32, sin32, oracle, True),
                        'framework-table-fp32-products': rotate(bits, cos32, sin32, oracle, False),
                        'f64-model-fp32-products': rotate(bits, model_cos, model_sin, oracle, False),
                        'f64-model-materialized': rotate(bits, [x << 16 for x in narrowed_cos],
                                                         [x << 16 for x in narrowed_sin], oracle, True),
                    }
                    pins = {mode: save('%s-pos%d-%s-%s.bf16le' % (name, position, kind, mode),
                                       pack_words(value, 16)) for mode, value in modes.items()}
                    rows.append({'corpus': name, 'position': position, 'kind': kind,
                                 'output_shape': [1, len(bits) // 128, 1, 128], 'outputs': pins,
                                 'vs_framework': {mode: compare(value, actual) for mode, value in modes.items() if mode != 'framework'}})
                    if name == 'genuine-pos0-conditional' and position == 0:
                        require(actual == corpus['rotary-' + kind], 'original position-zero rotation bytes')
                if name == 'genuine-pos0-conditional' and position == 0:
                    require(cos16 == corpus['rotary-cos'] and sin16 == corpus['rotary-sin'], 'original zero-position table')
    require(not torch.cuda.is_initialized(), 'CPU-only postcheck')
    for pin in list(TRACKED.values()):
        read(pin['path'], pin['sha256'], pin['bytes'], max(pin['bytes'], 2 << 20))
    for pin in outputs:
        require(hashlib.sha256(Path(pin['path']).read_bytes()).hexdigest() == pin['sha256'], 'output postcheck')
    result = {'schema': 'ferric-p228-rope-framework-reference-v1', 'completed': True,
              'positions': list(POSITIONS), 'head_dim': 128, 'theta': 1000000, 'rows': rows, 'tables': tables,
              'inputs': list(TRACKED.values()), 'outputs': outputs, 'source_unchanged': True,
              'historical_inv_frequency_sha256': corpus['historical_inv_frequency_sha256'],
              'inv_frequency_matches_historical_capture': hashlib.sha256(pack_words(inv, 32)).hexdigest() == corpus['historical_inv_frequency_sha256'],
              'runtime': {'host': socket.gethostname(), 'uid': os.getuid(), 'python': sys.version,
                          'interpreter': sys.executable, 'torch': torch.__version__,
                          'transformers': transformers.__version__, 'torch_build': torch.__config__.show(),
                          'cpu_threads': torch.get_num_threads(), 'interop_threads': torch.get_num_interop_threads(),
                          'affinity': sorted(os.sched_getaffinity(0)), 'nice': os.getpriority(os.PRIO_PROCESS, 0),
                          'flush_denormal_disable_supported': flush_supported,
                          'limits': {str(k): list(resource.getrlimit(k)) for k in
                                     (resource.RLIMIT_AS, resource.RLIMIT_CPU, resource.RLIMIT_FSIZE, resource.RLIMIT_NOFILE)}},
              'all_independent_materialized_rows_exact': all(row['vs_framework']['independent-materialized']['different_words'] == 0 for row in rows),
              'conditional_on_position_zero_qk': True, 'synthetic_corpus_included': True,
              'f64_table_is_python_libm_model': True, 'actual_rust_tables_replayed': False,
              'cpu_trig_is_gpu_trig_evidence': False, 'new_genuine_later_position_capture': False,
              'transitive_framework_libraries_rehashed': False, 'historical_owner_audits_replayed': False,
              'model_loaded': False, 'gpu_execution': False, 'numerical_acceptance': False,
              'full_model_correctness': False, 'performance_claim': False, 'production_authority': False}
    save('complete.json', (json.dumps(result, sort_keys=True, indent=2) + '\n').encode())
    signal.alarm(0)
    print(json.dumps({'complete': outputs[-1], 'rows': len(rows), 'numerical_acceptance': False}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('label')
    parser.add_argument('self_sha256')
    execute(parser.parse_args())
