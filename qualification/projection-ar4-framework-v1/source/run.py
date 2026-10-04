"""Genuine AR4 and separate native-input framework replay; no numerical acceptance."""
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
FORWARDS = 4
SEED = 9112
VALIDATORS = {
    'stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'decode_validation.py': '0eb96d4ac5e10e7f6ac10b018692c56f969286949681be336cb6040d56f01422',
}

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
            'output_root', 'implementation_sources', 'native', 'execution_review'}, 'closed AR4 plan')
    require(value['schema'] == 'ferric-p228-projection-ar4-framework-plan-v1', 'new AR4 plan schema')
    require(isinstance(value['harness_sha256'], str)
            and re.fullmatch('[0-9a-f]{64}', value['harness_sha256']), 'exact new capture source digest')
    path = Path(value['output_root'])
    require(path.parent == E and re.fullmatch(r'projection-ar4-framework-reference-v228-v[1-9][0-9]*', path.name),
            'fresh independent namespace only')
    native_shape(value['native'])
    require(type(value['implementation_sources']) is dict
            and set(value['implementation_sources']) == SOURCES
            and value['implementation_sources']['modeling_qwen3']['sha256'] == MODEL_SHA,
            'exact installed-source roles and original model implementation')


def reviewed(value, review, limits):
    require(type(review) is dict and set(review) == {'schema', 'reviewed', 'plan_projection_sha256',
            'resources', 'gpu_execution_authorized', 'numerical_acceptance', 'performance_claim',
            'production_authority'}, 'closed separately root-authored execution review')
    projection = {key: item for key, item in value.items() if key != 'execution_review'}
    require(review['schema'] == 'ferric-p228-projection-ar4-framework-execution-review-v1'
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
    require(digest(raw) == expected, 'exact AR4 plan digest')
    value = parse(raw)
    plan_shape(value)
    base = load_base(value['reference_helper'])
    pin = base.file_pin(path)
    require(pin['sha256'] == expected, 'unchanged AR4 plan')
    environment, old, diag, _, inputs = base.checked_plan(value['reference_plan']['path'],
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
    require(own['sha256'] == value['harness_sha256'], 'reviewed AR4 harness identity')
    pins = [pin, own, value['reference_helper'], value['execution_review'], *inputs,
            *value['implementation_sources'].values()]
    native = native_context(value['native'], base, diag)
    pins.extend(native['retained_pins'])
    return value, environment, base, old, diag, pins, native



def native_shape(value):
    require(type(value) is dict and set(value) == {'complete', 'transport', 'validators'},
            'closed retained native inputs')
    require(type(value['transport']) is dict and len(value['transport']) <= 16,
            'bounded explicit native transport map')
    require(type(value['validators']) is dict and set(value['validators']) == set(VALIDATORS),
            'three exact data-only validator dependencies')
    for name, pin in value['validators'].items():
        require(pin['sha256'] == VALIDATORS[name] and Path(pin['path']).name == name,
                'frozen AR4 data validator, not intake or execution')


def native_context(value, base, diag):
    native_shape(value)
    consumed, pins, aliases = {}, [], value['transport']
    def read(original, cap=4 << 20):
        require(type(original) is dict and set(original) == {'path', 'bytes', 'sha256'}
                and Path(original['path']).is_absolute(), 'original native FilePin')
        retained = aliases.get(original['path'], original)
        require((retained['bytes'], retained['sha256']) == (original['bytes'], original['sha256']),
                'transport preserves original bytes and digest')
        if retained['bytes'] == 0:
            require(type(retained['bytes']) is int and base.file_pin(Path(retained['path'])) == retained
                    and retained['sha256'] == digest(b''), 'authenticated empty native stream')
            raw = b''
        else:
            raw = base.read_pin(retained, cap)
        pair = dict(original=original, retained=retained)
        require(original['path'] not in consumed or consumed[original['path']] == pair,
                'unique native original identity')
        consumed[original['path']] = pair
        pins.append(retained)
        return raw

    outer = parse(read(value['complete']))
    require(outer['schema'] == 'ferric-p228-projection-ar4-decode-gpu-v1'
            and outer['passed'] is True and outer['failures'] == []
            and outer['native_attempts'] == 1 and outer['retries'] == 0
            and outer['captured_tensor_rows'] == 152 and outer['captured_payloads'] == 4
            and outer['own_output_trajectory_checked'] is True
            and outer['numerical_acceptance'] is False, 'actual successful AR4 outer record')
    request = parse(read(outer['request']))
    # Restore every import alias, including a preexisting None, even on refusal.
    sentinel = object()
    saved = {name: sys.modules.get(name, sentinel) for name in ('stage_core', 'smoke_validation')}
    try:
        modules = {}
        for name in ('stage_core.py', 'smoke_validation.py', 'decode_validation.py'):
            pin = value['validators'][name]
            raw = base.read_pin(pin, 1 << 20)
            module = types.ModuleType('ar4_reference_' + name[:-3])
            module.__file__ = pin['path']
            exec(compile(raw, pin['path'], 'exec'), module.__dict__)
            modules[name] = module
            pins.append(pin)
            if name != 'decode_validation.py':
                sys.modules[name[:-3]] = module
        validator = modules['decode_validation.py']
        roster = outer['retained_native']
        require(type(roster) is dict and set(roster) == validator.FILES | {'complete.json'},
                'all fourteen actual native files')
        files = {name: read(roster[name]) for name in validator.FILES}
        checked = validator.validate(read(roster['complete.json'], 65536), files, request)
    finally:
        for name, before in saved.items():
            if before is sentinel:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = before
    require(checked == outer['checked'], 'replayed native structure equals recorded structure')
    require(set(aliases) <= set(consumed), 'no unused native transport aliases')
    cases = []
    for position in range(FORWARDS):
        record = dict(generation=position + 1, position=position,
                      input_token=checked['input_tokens'][position],
                      output_token=checked['output_tokens'][position])
        raw = files[f'observation-{position}.bin']
        diag.validate_case(record, raw, position)
        cases.append((record, raw))
    trajectory([record for record, _ in cases])
    return dict(complete=value['complete'], cases=cases, checked=checked,
                consumed=list(consumed.values()), retained_pins=pins,
                external_native_lifecycle_replayed=False,
                native_source_image_bodies_rehashed=False)


def trajectory(records, forced=None):
    require(type(records) is list and len(records) == FORWARDS, 'four complete records')
    if forced is not None:
        require(type(forced) is list and len(forced) == FORWARDS
                and all(type(x) is int and 0 <= x < 151936 for x in forced)
                and forced[0] == SEED, 'authenticated native conditional inputs')
    expected = SEED
    for position, record in enumerate(records):
        require(type(record) is dict and set(record) == {'generation', 'position', 'input_token', 'output_token'}
                and all(type(x) is int for x in record.values())
                and record['generation'] == position + 1 and record['position'] == position
                and 0 <= record['output_token'] < 151936, 'exact integer record identity')
        require(record['input_token'] == (expected if forced is None else forced[position]),
                'own prior output, or separately labeled conditional input')
        expected = record['output_token']
    return [record['input_token'] for record in records]


def comparable_prefix(reference, candidate):
    trajectory(reference)
    trajectory(candidate)
    same, result = True, []
    for left, right in zip(reference, candidate):
        same = same and left['input_token'] == right['input_token']
        result.append(same)
    return result


def compare_cases(diag, reference, candidate, conditional=False):
    require(len(reference) == len(candidate) == FORWARDS, 'four comparison cases')
    native_records = [record for record, _ in candidate]
    native_inputs = trajectory(native_records)
    reference_records = [record for record, _ in reference]
    trajectory(reference_records, native_inputs if conditional else None)
    comparable = [True] * FORWARDS if conditional else comparable_prefix(reference_records, native_records)
    rows = []
    for position, ((left, ref), (right, actual)) in enumerate(zip(reference, candidate)):
        a = diag.validate_case(left, ref, position)
        b = diag.validate_case(right, actual, position)
        rows.append(dict(position=position, comparable=comparable[position],
            reference_input_token=left['input_token'], candidate_input_token=right['input_token'],
            reference_output_token=left['output_token'], candidate_output_token=right['output_token'],
            output_token_equal=left['output_token'] == right['output_token'],
            tensors={name: diag.compare_tensor(raw, b[name]) for name, raw in a.items()}
                    if comparable[position] else None))
    return dict(scope=('native-input conditional framework replay with independent own KV'
                       if conditional else 'genuine own-output AR histories; no recovered comparability'),
                comparisons=rows, tensor_rows=sum(38 for row in rows if row['comparable']),
                conditional=conditional, numerical_acceptance=False, acceptance_threshold=None)


def repeat_equal(passes, forced=None):
    require(type(passes) is list and len(passes) == 2, 'two fresh-cache passes')
    for ordinal, run in enumerate(passes, 1):
        require(set(run) == {'ordinal', 'fresh_cache', 'kind', 'cases'}
                and type(run['ordinal']) is int and run['ordinal'] == ordinal
                and run['fresh_cache'] is True
                and run['kind'] == ('genuine_ar' if forced is None else 'native_input_conditional'),
                'explicit independent pass identity and kind')
        trajectory([case['record'] for case in run['cases']], forced)
        for case in run['cases']:
            require(set(case) == {'record', 'payload', 'tensors', 'cache_sha256'}
                    and case['payload']['bytes'] == 606976
                    and set(case['tensors']) == {*(f'layer{n}-hidden' for n in range(36)), 'final-norm', 'logits'}
                    and len(case['cache_sha256']) == 36, 'complete payload and own cache roster')
            for name, row in case['tensors'].items():
                require(set(row) == {'bytes', 'sha256'} and row['bytes'] == (303872 if name == 'logits' else 8192)
                        and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'typed captured tensor pin')
            for row in case['cache_sha256']:
                require(set(row) == {'key', 'value'} and all(re.fullmatch('[0-9a-f]{64}', x) for x in row.values()),
                        'all framework KV cache digests')
    return all(a['record'] == b['record'] and a['payload']['sha256'] == b['payload']['sha256']
               and a['tensors'] == b['tensors'] and a['cache_sha256'] == b['cache_sha256']
               for a, b in zip(passes[0]['cases'], passes[1]['cases']))


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


def run_pass(model, torch, base, diag, capture, ordinal, bounded, forced=None):
    from transformers.cache_utils import DynamicCache
    cache = DynamicCache()
    require(cache.get_seq_length() == 0, 'fresh pass cache')
    cases = []
    kind = 'genuine_ar' if forced is None else 'native_input_conditional'
    for position in range(FORWARDS):
        token = (SEED if position == 0 else cases[-1]['record']['output_token']) if forced is None else forced[position]
        hidden, norm_inputs, hooks = {}, [], []
        def layer_hook(layer):
            def observe(_module, _arguments, output):
                require(layer not in hidden, 'duplicate layer capture')
                value = output[0] if isinstance(output, (tuple, list)) else output
                hidden[layer] = base.bf16_raw(value, torch, (1, 1, 4096))
                bounded()
            return observe
        def norm_hook(_module, arguments):
            require(len(arguments) >= 1 and not norm_inputs, 'final norm input count')
            norm_inputs.append(base.bf16_raw(arguments[0], torch, (1, 1, 4096)))
        try:
            for layer, module in enumerate(model.model.layers):
                hooks.append(module.register_forward_hook(layer_hook(layer)))
            hooks.append(model.model.norm.register_forward_pre_hook(norm_hook))
            with torch.inference_mode():
                ids = torch.tensor([[token]], dtype=torch.long, device='cuda')
                mask = torch.ones((1, position + 1), dtype=torch.long, device='cuda')
                result = model.model(input_ids=ids, attention_mask=mask, past_key_values=cache,
                                     use_cache=True, return_dict=True)
                final = base.bf16_raw(result.last_hidden_state, torch, (1, 1, 4096))
                logits = base.bf16_raw(model.lm_head(result.last_hidden_state), torch, (1, 1, diag.VOCABULARY))
                cache = result.past_key_values
                require(cache is not None and cache.get_seq_length() == position + 1 and
                        len(cache.key_cache) == len(cache.value_cache) == 36, 'causal own-cache length/layers')
                cache_hashes = []
                for key, value in zip(cache.key_cache, cache.value_cache):
                    cache_hashes.append({'key': digest(base.bf16_raw(key, torch, (1, 8, position + 1, 128))),
                                         'value': digest(base.bf16_raw(value, torch, (1, 8, position + 1, 128)))})
                require(set(hidden) == set(range(36)) and norm_inputs == [hidden[35]], 'all raw layer outputs/final norm join')
                payload = b''.join(hidden[layer] for layer in range(36)) + final + logits
                rows = diag.split_payload(payload)
                choice = diag.argmax(logits)
                record = {'generation': position + 1, 'position': position, 'input_token': token, 'output_token': choice}
                diag.validate_case(record, payload, position)
                pin = capture.write(f'{kind.replace("_", "-")}-pass{ordinal}-pos{position}.bf16', payload)
                cases.append({'record': record, 'payload': pin,
                              'tensors': {name: {'bytes': len(raw), 'sha256': digest(raw)} for name, raw in rows.items()},
                              'cache_sha256': cache_hashes})
                del ids, mask, result, final, logits, payload, rows
        finally:
            for handle in hooks:
                handle.remove()
        torch.cuda.synchronize()
        bounded()
    del cache
    torch.cuda.synchronize()
    return {'ordinal': ordinal, 'fresh_cache': True, 'kind': kind, 'cases': cases}


def execute(plan, environment, base, old, diag, pins, native):
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
    passes = [run_pass(model, torch, base, diag, capture, ordinal, bounded) for ordinal in (1, 2)]
    equal = repeat_equal(passes)
    require(equal, 'genuine AR repeat mismatch')
    native_inputs = trajectory([record for record, _ in native['cases']])
    genuine_inputs = trajectory([case['record'] for case in passes[0]['cases']])
    conditional = []
    if genuine_inputs != native_inputs:
        conditional = [run_pass(model, torch, base, diag, capture, ordinal, bounded, native_inputs)
                       for ordinal in (1, 2)]
        require(repeat_equal(conditional, native_inputs), 'conditional repeat mismatch')
    def loaded(run):
        return [(case['record'], base.read_pin(case['payload'], 1 << 20)) for case in run['cases']]
    comparisons = dict(genuine_ar=compare_cases(diag, loaded(passes[0]), native['cases']))
    if conditional:
        comparisons['native_input_conditional'] = compare_cases(diag, loaded(conditional[0]), native['cases'], True)
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
    report = dict(schema='ferric-p228-projection-ar4-framework-reference-v1',
        status='PASS', model='Qwen/Qwen3-8B', revision=old.REVISION,
        model_id=base.MODEL_ID, bundle_id=base.BUNDLE_ID, seed=SEED,
        genuine_passes=passes, conditional_passes=conditional, comparisons=comparisons,
        native_complete=native['complete'], native_checked=native['checked'],
        native_consumed=native['consumed'], external_native_lifecycle_replayed=False,
        native_source_image_bodies_rehashed=False,
        repeat_passes_byte_equal=True, captured_tensors_per_forward=38,
        framework_forward_count=8 + (8 if conditional else 0),
        conditional_replay_performed=bool(conditional),
        conditional_inputs_only_tokens=True, candidate_intermediate_inputs=False,
        genuine_framework_chain=True, model_sources=sources, input_pins=pins, runtime=runtime,
        retained_implementation_sources=retained_sources,
        candidate_gpu_execution=False, gpu_execution=True,
        numerical_acceptance=False, acceptance_threshold=None, full_model_correctness=False,
        production_authority=False, performance_measured=False, sustained_2048_256=False,
        supervisor_lifecycle_required=True, gpu_context_process_exit_not_yet_observed=True)
    result = capture.write('reference.json', base.encoded(report))
    return result


def validate_report(base, diag, report, directory, plan_pin):
    require(report['schema'] == 'ferric-p228-projection-ar4-framework-reference-v1'
            and report['status'] == 'PASS' and report['seed'] == SEED
            and report['repeat_passes_byte_equal'] is True
            and report['genuine_framework_chain'] is True and report['gpu_execution'] is True
            and report['conditional_inputs_only_tokens'] is True
            and plan_pin in report['input_pins']
            and all(report[key] is False for key in ('candidate_gpu_execution', 'candidate_intermediate_inputs',
                'numerical_acceptance', 'full_model_correctness', 'production_authority', 'performance_measured',
                'sustained_2048_256', 'external_native_lifecycle_replayed', 'native_source_image_bodies_rehashed')),
            'completed independent reference with explicit limits')
    native_inputs = report['native_checked']['input_tokens']
    require(len(native_inputs) == FORWARDS and native_inputs[0] == SEED, 'recorded native seed')
    genuine = report['genuine_passes']
    require(repeat_equal(genuine), 'genuine repeat byte identities')
    expected_conditional = trajectory([case['record'] for case in genuine[0]['cases']]) != native_inputs
    conditional = report['conditional_passes']
    require(report['conditional_replay_performed'] is expected_conditional
            and (repeat_equal(conditional, native_inputs) if expected_conditional else conditional == [])
            and report['framework_forward_count'] == (16 if expected_conditional else 8)
            and report['captured_tensors_per_forward'] == 38, 'conditional replay exactly when inputs differ')
    names = set()
    for run in genuine + conditional:
        for position, case in enumerate(run['cases']):
            pin = case['payload']
            name = f'{run["kind"].replace("_", "-")}-pass{run["ordinal"]}-pos{position}.bf16'
            require(pin['path'] == str(directory / name) and name not in names, 'unique owned payload path')
            names.add(name)
            raw = base.read_pin(pin, 1 << 20)
            rows = diag.validate_case(case['record'], raw, position)
            require(case['tensors'] == {key: dict(bytes=len(value), sha256=digest(value))
                                      for key, value in rows.items()}, 'all actual tensor hashes')
    for pin in report['input_pins']:
        require(base.file_pin(Path(pin['path'])) == pin, 'retained reference input unchanged')


def main():
    require(len(sys.argv) == 4 and sys.argv[1] in ('--inspect', '--run-reviewed-ar4-reference'),
            'run.py (--inspect|--run-reviewed-ar4-reference) PLAN SHA256')
    values = checked_plan(sys.argv[2], sys.argv[3])
    if sys.argv[1] == '--inspect':
        print(json.dumps(dict(schema='ferric-p228-projection-ar4-framework-inspection-v1',
                              input_pins=values[-2], gpu_opened=False,
                              installed_callables_loaded=False, numerical_acceptance=False,
                              native_structure_replayed=True), sort_keys=True))
    else:
        print(json.dumps(execute(*values), sort_keys=True))


if __name__ == '__main__':
    main()
