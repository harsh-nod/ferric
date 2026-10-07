"""Independent position-zero framework intermediates; no candidate input or acceptance."""
import hashlib
import inspect
import json
import os
from pathlib import Path
import re
import sys
import time
import types

BASE_SHA = '613579c7c84ed9b6f94abf6864c9934b72331dd001bf6badb973ab4576f4db97'
MODEL_SHA = '704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
SOURCES = {'modeling_qwen3', 'activation', 'sdpa', 'torch_functional'}
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
JOINS = (
    ('embedding', 'input-norm-input'), ('input-norm', 'q-input'),
    ('input-norm', 'k-input'), ('input-norm', 'v-input'),
    ('q-projection', 'q-norm-input'), ('k-projection', 'k-norm-input'),
    ('post-norm', 'gate-input'), ('post-norm', 'up-input'), ('gate', 'silu-input'),
    ('down-projection', 'mlp-output'), ('rotary-k', 'cache-key'),
    ('v-projection', 'cache-value'),
)


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda value: require(False, 'nonfinite JSON'))


def load_base(pin):
    require(set(pin) == {'path', 'bytes', 'sha256'} and pin['sha256'] == BASE_SHA
            and type(pin['bytes']) is int and 0 < pin['bytes'] <= 65536, 'fixed reference helper pin')
    path = Path(pin['path'])
    require(path.is_absolute() and path.resolve(strict=True) == path and path.is_file()
            and path.name == 'framework_reference.py', 'canonical retained reference helper')
    raw = path.read_bytes()
    require(len(raw) == pin['bytes'] and digest(raw) == BASE_SHA, 'retained reference helper bytes')
    module = types.ModuleType('layer0_retained_framework_reference')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    require(module.file_pin(path) == pin, 'reference helper unchanged during load')
    return module


def plan_shape(value):
    require(type(value) is dict and set(value) == {'schema', 'harness_sha256', 'reference_helper', 'reference_plan',
            'output_root', 'implementation_sources', 'execution_review'}, 'closed layer0 plan')
    require(value['schema'] == 'ferric-p228-layer0-framework-capture-plan-v1', 'new layer0 plan schema')
    require(isinstance(value['harness_sha256'], str)
            and re.fullmatch('[0-9a-f]{64}', value['harness_sha256']), 'exact new capture source digest')
    path = Path(value['output_root'])
    require(path.parent == E and re.fullmatch(r'layer0-framework-capture-v228-v[1-9][0-9]*', path.name),
            'fresh independent namespace only')
    require(type(value['implementation_sources']) is dict
            and set(value['implementation_sources']) == SOURCES
            and value['implementation_sources']['modeling_qwen3']['sha256'] == MODEL_SHA,
            'exact installed-source roles and original model implementation')


def reviewed(value, review, limits):
    require(type(review) is dict and set(review) == {'schema', 'reviewed', 'plan_projection_sha256',
            'resources', 'gpu_execution_authorized', 'numerical_acceptance', 'performance_claim',
            'production_authority'}, 'closed separately root-authored execution review')
    projection = {key: item for key, item in value.items() if key != 'execution_review'}
    require(review['schema'] == 'ferric-p228-layer0-framework-execution-review-v1'
            and review['reviewed'] is True and review['gpu_execution_authorized'] is True
            and review['plan_projection_sha256'] == digest(encoded(projection))
            and review['resources'] == limits
            and all(review[key] is False for key in ('numerical_acceptance', 'performance_claim',
                                                    'production_authority')), 'explicit exact execution review')


def checked_plan(path, expected):
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path and path.is_file()
            and path.stat().st_size <= 1 << 20 and re.fullmatch('[0-9a-f]{64}', expected), 'bounded canonical plan')
    raw = path.read_bytes()
    require(digest(raw) == expected, 'exact layer0 plan digest')
    value = parse(raw)
    plan_shape(value)
    base = load_base(value['reference_helper'])
    pin = base.file_pin(path)
    require(pin['sha256'] == expected, 'unchanged layer0 plan')
    environment, old, _, _, inputs = base.checked_plan(value['reference_plan']['path'],
                                                       value['reference_plan']['sha256'])
    require(inputs[0] == value['reference_plan'] and environment['output_root'] == value['output_root'],
            'environment request and actual output are the same fresh path')
    review = base.document(base.read_pin(value['execution_review'], 1 << 20))
    reviewed(value, review, base.LIMITS)
    for source in value['implementation_sources'].values():
        require(Path(source['path']).is_relative_to(Path(environment['python_prefix'])),
                'implementation from the retained isolated environment')
        base.read_pin(source, 4 << 20)
    own = base.file_pin(Path(__file__).resolve(strict=True))
    require(own['sha256'] == value['harness_sha256'], 'reviewed layer0 harness identity')
    pins = [pin, own, value['reference_helper'], value['execution_review'], *inputs,
            *value['implementation_sources'].values()]
    return value, environment, base, old, pins


def stage_bytes(name):
    require(name in SHAPES, 'known stage')
    count = 2
    for dimension in SHAPES[name]:
        count *= dimension
    return count


def validate_values(values):
    require(type(values) is dict and set(values) == set(SHAPES), 'complete genuine stage roster')
    for name, raw in values.items():
        require(type(raw) is bytes and len(raw) == stage_bytes(name), 'exact BF16 stage extent')
    require(all(values[left] == values[right] for left, right in JOINS), 'actual producer/consumer byte joins')


class Stages:
    def __init__(self, base, torch, bounded):
        self.base, self.torch, self.bounded = base, torch, bounded
        self.values = {}

    def add(self, name, tensor):
        require(name in SHAPES and name not in self.values, 'one real invocation per stage')
        self.values[name] = self.base.bf16_raw(tensor, self.torch, SHAPES[name])
        self.bounded()

    def pre(self, name):
        def hook(_module, arguments):
            require(type(arguments) is tuple and len(arguments) >= 1, 'actual positional module input')
            self.add(name, arguments[0])
        return hook

    def post(self, name):
        def hook(_module, _arguments, output):
            self.add(name, output)
        return hook


class RotaryObserver:
    """Observe the real callable without recomputing or replacing its result."""
    def __init__(self, module, stages):
        self.module, self.stages = module, stages
        self.original = module.apply_rotary_pos_emb
        self.active = False
        self.calls = 0
        self.selected = 0
        self.wrapper = self.observe

    def enter(self, _module, _arguments):
        require(not self.active, 'non-nested selected attention')
        self.active = True

    def leave(self, _module, _arguments, _output):
        require(self.active, 'selected attention entered')
        self.active = False

    def observe(self, *args, **kwargs):
        result = self.original(*args, **kwargs)
        self.calls += 1
        if self.active:
            require(self.selected == 0 and type(result) is tuple and len(result) == 2,
                    'one genuine rotary pair for layer0')
            self.stages.add('rotary-q', result[0])
            self.stages.add('rotary-k', result[1])
            self.selected += 1
        return result

    def install(self):
        require(self.module.apply_rotary_pos_emb is self.original, 'unchanged original rotary callable')
        self.module.apply_rotary_pos_emb = self.wrapper

    def restore(self):
        unchanged = self.module.apply_rotary_pos_emb is self.wrapper
        self.module.apply_rotary_pos_emb = self.original
        require(unchanged, 'rotary observer replaced during execution')


def verify_implementations(base, plan, model, torch):
    module = sys.modules[model.__class__.__module__]
    layer = model.model.layers[0]
    require(model.config.hidden_act == 'silu' and isinstance(layer.mlp.act_fn, torch.nn.Module),
            'actual hookable SiLU module')
    sdpa = module.ALL_ATTENTION_FUNCTIONS['sdpa']
    objects = {'modeling_qwen3': model.__class__, 'activation': type(layer.mlp.act_fn),
               'sdpa': sdpa, 'torch_functional': torch.nn.functional}
    for name, value in objects.items():
        path = Path(inspect.getsourcefile(value)).resolve(strict=True)
        require(base.file_pin(path) == plan['implementation_sources'][name], 'actual installed callable source: ' + name)
    require(layer.self_attn.forward.__func__.__globals__ is module.__dict__
            and module.apply_rotary_pos_emb.__module__ == module.__name__, 'actual attention global routing')
    return module, layer, sdpa


def run_pass(model, torch, base, plan, capture, ordinal, bounded, module, layer):
    from transformers.cache_utils import DynamicCache
    cache = DynamicCache()
    require(cache.get_seq_length() == 0, 'independent empty KV')
    stages = Stages(base, torch, bounded)
    rotary = RotaryObserver(module, stages)
    hooks = []
    bindings = [
        (model.model.embed_tokens, None, 'embedding'),
        (layer.input_layernorm, 'input-norm-input', 'input-norm'),
        (layer.self_attn.q_proj, 'q-input', 'q-projection'),
        (layer.self_attn.k_proj, 'k-input', 'k-projection'),
        (layer.self_attn.v_proj, 'v-input', 'v-projection'),
        (layer.self_attn.q_norm, 'q-norm-input', 'q-norm'),
        (layer.self_attn.k_norm, 'k-norm-input', 'k-norm'),
        (layer.self_attn.o_proj, 'attention-output', 'o-projection'),
        (layer.post_attention_layernorm, 'first-residual', 'post-norm'),
        (layer.mlp.gate_proj, 'gate-input', 'gate'), (layer.mlp.up_proj, 'up-input', 'up'),
        (layer.mlp.act_fn, 'silu-input', 'silu'),
        (layer.mlp.down_proj, 'product', 'down-projection'), (layer.mlp, None, 'mlp-output'),
    ]
    def rotary_output(_module, _arguments, output):
        require(type(output) is tuple and len(output) == 2, 'actual shared rotary outputs')
        stages.add('rotary-cos', output[0])
        stages.add('rotary-sin', output[1])
    def layer_output(_module, _arguments, output):
        require(type(output) is tuple and len(output) >= 1, 'actual decoder output')
        stages.add('layer0-hidden', output[0])
    try:
        for owner, before, after in bindings:
            if before is not None:
                hooks.append(owner.register_forward_pre_hook(stages.pre(before)))
            if after is not None:
                hooks.append(owner.register_forward_hook(stages.post(after)))
        hooks.append(model.model.rotary_emb.register_forward_hook(rotary_output))
        hooks.append(layer.self_attn.register_forward_pre_hook(rotary.enter))
        hooks.append(layer.self_attn.register_forward_hook(rotary.leave))
        hooks.append(layer.register_forward_hook(layer_output))
        rotary.install()
        try:
            with torch.inference_mode():
                ids = torch.tensor([[9112]], dtype=torch.long, device='cuda')
                mask = torch.ones((1, 1), dtype=torch.long, device='cuda')
                result = model.model(input_ids=ids, attention_mask=mask, past_key_values=cache,
                                     use_cache=True, return_dict=True)
                cache = result.past_key_values
                require(cache is not None and cache.get_seq_length() == 1
                        and len(cache.key_cache) == len(cache.value_cache) == 36, 'original full-model own KV chain')
                stages.add('cache-key', cache.key_cache[0])
                stages.add('cache-value', cache.value_cache[0])
                require(not rotary.active and rotary.calls == 36 and rotary.selected == 1,
                        'one observed layer0 within the unchanged 36-layer framework route')
        finally:
            rotary.restore()
    finally:
        for handle in reversed(hooks):
            handle.remove()
    torch.cuda.synchronize()
    bounded()
    validate_values(stages.values)
    rows = {}
    for name, raw in stages.values.items():
        rows[name] = {'dtype': 'bfloat16', 'shape': list(SHAPES[name]),
                      'pin': capture.write(f'pass{ordinal}-{name}.bf16', raw)}
    del cache, result, ids, mask
    return {'ordinal': ordinal, 'position': 0, 'input_token': 9112, 'fresh_cache': True, 'stages': rows}


def repeat_equal(passes):
    require(type(passes) is list and len(passes) == 2, 'two genuine independent passes')
    for ordinal, value in enumerate(passes, 1):
        require(set(value) == {'ordinal', 'position', 'input_token', 'fresh_cache', 'stages'}
                and type(value['ordinal']) is int and value['ordinal'] == ordinal
                and type(value['position']) is int and value['position'] == 0
                and type(value['input_token']) is int and value['input_token'] == 9112
                and value['fresh_cache'] is True and set(value['stages']) == set(SHAPES), 'closed pass identity')
        for name, row in value['stages'].items():
            require(set(row) == {'dtype', 'shape', 'pin'} and row['dtype'] == 'bfloat16'
                    and row['shape'] == list(SHAPES[name]) and row['pin']['bytes'] == stage_bytes(name),
                    'reported observed stage type/extent')
    return all(passes[0]['stages'][name]['pin']['sha256'] == passes[1]['stages'][name]['pin']['sha256']
               for name in SHAPES)


def execute(plan, environment, base, old, pins):
    require(os.getppid() == environment['supervisor_pid'], 'root-owned direct supervisor required')
    for key in ('HF_HUB_OFFLINE', 'TRANSFORMERS_OFFLINE', 'HF_DATASETS_OFFLINE'):
        os.environ[key] = '1'
    os.environ['TOKENIZERS_PARALLELISM'] = 'false'
    os.environ['CUBLAS_WORKSPACE_CONFIG'] = ':4096:8'
    capture = base.Capture(plan['output_root'])
    for name in ('XDG_CACHE_HOME', 'HF_HOME', 'TORCH_HOME', 'TRITON_CACHE_DIR', 'MIOPEN_USER_DB_PATH'):
        os.environ[name] = str(capture.path / 'private-cache' / name.lower())
    capture.write('input-plan.json', base.encoded(plan))
    retained_sources = {name: capture.write('implementation-' + name.replace('_', '-') + '.py',
        base.read_pin(pin, 4 << 20)) for name, pin in plan['implementation_sources'].items()}
    started = time.monotonic()
    root = Path(environment['model_root'])
    for name in old.MODEL_FILES:
        path = root / name
        require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical original model file')
    sources = old.checked_sources(root, old.MODEL_FILES)
    import psutil
    require(psutil.virtual_memory().available >= base.LIMITS['minimum_free_host_bytes'], 'free host memory')
    import torch
    import transformers
    require(torch.cuda.is_available() and torch.cuda.device_count() == 1
            and str(torch.version.hip).startswith('7.2'), 'retained ROCm/visibility')
    properties = torch.cuda.get_device_properties(0)
    require(properties.gcnArchName == 'gfx950:sramecc+:xnack-'
            and torch.cuda.mem_get_info()[0] >= base.LIMITS['minimum_free_gpu_bytes'], 'gfx950/free VRAM')
    torch.manual_seed(0)
    torch.backends.cuda.matmul.allow_tf32 = False
    torch.backends.cudnn.allow_tf32 = False
    torch.backends.cuda.matmul.allow_bf16_reduced_precision_reduction = False
    torch.set_float32_matmul_precision('highest')
    torch.use_deterministic_algorithms(True)
    torch.backends.cuda.enable_flash_sdp(False)
    torch.backends.cuda.enable_mem_efficient_sdp(False)
    torch.backends.cuda.enable_math_sdp(True)
    torch.backends.cuda.enable_cudnn_sdp(False)
    torch.backends.cuda.allow_fp16_bf16_reduction_math_sdp(False)
    model, loading = transformers.AutoModelForCausalLM.from_pretrained(
        str(root), torch_dtype=torch.bfloat16, attn_implementation='sdpa', local_files_only=True,
        trust_remote_code=False, use_safetensors=True, output_loading_info=True)
    require(model.__class__.__name__ == 'Qwen3ForCausalLM'
            and model.__class__.__module__ == 'transformers.models.qwen3.modeling_qwen3'
            and all(not loading[name] for name in ('missing_keys', 'unexpected_keys', 'mismatched_keys', 'error_msgs')),
            'exact checkpoint and framework class')
    require(all(getattr(model.config, key) == value for key, value in {
        'hidden_size': 4096, 'intermediate_size': 12288, 'num_hidden_layers': 36, 'num_attention_heads': 32,
        'num_key_value_heads': 8, 'head_dim': 128, 'vocab_size': 151936, 'rope_theta': 1000000.0,
        'rms_norm_eps': 1e-6, 'tie_word_embeddings': False, 'model_type': 'qwen3'}.items()), 'original geometry')
    inverse = old.move_bf16_model_preserving_fp32_rope(model, torch, 'cuda')
    require(len(model.model.layers) == 36 and model.config._attn_implementation == 'sdpa'
            and all(parameter.dtype == torch.bfloat16 for parameter in model.parameters()), 'BF16 original profile')
    module, layer, sdpa = verify_implementations(base, plan, model, torch)
    bounded = lambda: base.check_live_bounds(capture, started, psutil)
    bounded()
    passes = [run_pass(model, torch, base, plan, capture, ordinal, bounded, module, layer) for ordinal in (1, 2)]
    equal = repeat_equal(passes)
    require(module.ALL_ATTENTION_FUNCTIONS['sdpa'] is sdpa, 'selected SDPA callable unchanged')
    runtime = {'python': sys.version, 'python_executable': sys.executable, 'python_prefix': sys.prefix,
               'packages': base.PACKAGES, 'implementation_sources': plan['implementation_sources'],
               'hip': torch.version.hip, 'gcn_arch': properties.gcnArchName, 'sdpa_backend': 'math-only',
               'deterministic_algorithms': True, 'bf16_reduced_precision_matmul_reduction': False,
               'sdpa_low_precision_reduction': False, 'rotary_fp32_preserved': True,
               'rotary_sha256': base.digest(inverse.numpy().tobytes()), 'torch_build': torch.__config__.show()}
    del layer, model
    torch.cuda.empty_cache()
    torch.cuda.synchronize()
    bounded()
    require(old.checked_sources(root, old.MODEL_FILES) == sources, 'original model changed')
    for pin in pins:
        require(base.file_pin(Path(pin['path'])) == pin, 'input or installed implementation changed')
    require(environment['boot_id'] == Path('/proc/sys/kernel/random/boot_id').read_text().strip()
            and environment['gpu_unique_id'] == Path(environment['gpu_unique_id_file']).read_text().strip(),
            'same device and boot')
    for pin in list(capture.files.values()):
        require(base.file_pin(Path(pin['path'])) == pin, 'captured bytes changed')
    report = {'schema': 'ferric-p228-layer0-framework-capture-v1', 'status': 'PASS' if equal else 'FAIL',
              'model': 'Qwen/Qwen3-8B', 'revision': old.REVISION, 'model_id': base.MODEL_ID,
              'bundle_id': base.BUNDLE_ID, 'position': 0, 'input_token': 9112, 'passes': passes,
              'repeat_passes_byte_equal': equal, 'captured_stages_per_pass': len(SHAPES),
              'model_sources': sources, 'input_pins': pins, 'runtime': runtime,
              'retained_implementation_sources': retained_sources,
              'genuine_framework_chain': True, 'conditional_replay_performed': False,
              'candidate_intermediate_inputs': False, 'candidate_gpu_execution': False,
              'gpu_execution': True, 'numerical_acceptance': False, 'acceptance_threshold': None,
              'full_model_correctness': False, 'production_authority': False, 'performance_measured': False,
              'supervisor_lifecycle_required': True, 'gpu_context_process_exit_not_yet_observed': True}
    result = capture.write('capture.json', base.encoded(report))
    require(equal, 'independent repeat stages disagree; FAIL report retained')
    return result


def main():
    require(len(sys.argv) == 4 and sys.argv[1] in ('--inspect', '--run-reviewed-layer0-capture'),
            'run.py (--inspect|--run-reviewed-layer0-capture) PLAN SHA256')
    values = checked_plan(sys.argv[2], sys.argv[3])
    if sys.argv[1] == '--inspect':
        print(json.dumps({'schema': 'ferric-p228-layer0-framework-capture-inspection-v1',
                          'input_pins': values[-1], 'stage_shapes': SHAPES, 'gpu_opened': False,
                          'installed_callables_loaded': False, 'numerical_acceptance': False}, sort_keys=True))
    else:
        print(json.dumps(execute(*values), sort_keys=True))


if __name__ == '__main__':
    main()
