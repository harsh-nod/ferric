"""Four actual framework MLP calls with explicit captured intermediate inputs."""
import hashlib
import importlib.metadata
import inspect
import os
from pathlib import Path
import sys
import time

SOURCE = Path('/source')
sys.path.insert(0, str(SOURCE))
from common import Reader, compact, encoded, load, parse, read, require, save

OUTPUT = Path('/output')
INPUTS = Path('/inputs')
PACKAGES = Path('/packages')
MODEL_SHA = '704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2'
IMAGE = 'sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba'
PACKAGE_NAMES = {'accelerate', 'numpy', 'psutil', 'safetensors', 'tokenizers', 'torch',
                 'transformers', 'triton', 'huggingface-hub'}


def source_contract(reader):
    value = parse(reader.read(SOURCE / 'source-manifest.json'))
    require(value['schema'] == 'ferric-guarded-mlp-matched-input-source-v1', 'source manifest schema')
    for name, row in value['files'].items():
        require(Path(name).name == name, 'flat closed source name')
        reader.read(SOURCE / name, row)
    require({p.name for p in SOURCE.iterdir()} == set(value['files']) | {'source-manifest.json'},
            'closed source directory')
    return value


def environment_contract(reader):
    environment = parse(reader.read(INPUTS / 'environment.json'))
    require(environment['schema'] == 'ferric-guarded-mlp-matched-input-environment-v1'
            and environment['image'] == IMAGE and environment['model_root'] == '/model'
            and environment['framework_gpu_execution'] is False, 'observed environment contract')
    require(sys.flags.isolated == 1 and sys.dont_write_bytecode and sys.byteorder == 'little'
            and list(sys.version_info[:3]) == environment['python_version']
            and sys.executable == environment['python_executable'], 'actual isolated Python identity')
    require(set(environment['packages']) == PACKAGE_NAMES, 'exact installed package roster')
    require(PACKAGES.is_dir() and not PACKAGES.is_symlink(), 'private package overlay')
    sys.path.insert(0, str(PACKAGES))
    require({name: importlib.metadata.version(name) for name in PACKAGE_NAMES} == environment['packages'],
            'observed package versions, not historical-version substitution')
    require(environment['packages']['transformers'] == '4.51.0', 'qualified Transformers API')
    require(os.environ.get('HIP_VISIBLE_DEVICES') == '0' and 'ROCR_VISIBLE_DEVICES' not in os.environ
            and 'CUDA_VISIBLE_DEVICES' not in os.environ, 'single unambiguous selected GPU')
    require(all(os.environ.get(key) == value for key, value in {
        'HF_HUB_OFFLINE': '1', 'TRANSFORMERS_OFFLINE': '1', 'HF_DATASETS_OFFLINE': '1',
        'TOKENIZERS_PARALLELISM': 'false', 'CUBLAS_WORKSPACE_CONFIG': ':4096:8'}.items()),
        'offline and deterministic process environment')
    return environment


def load_inputs(reader):
    contract = parse(reader.read(SOURCE / 'inputs.json'))
    require(contract['schema'] == 'ferric-guarded-mlp-matched-input-files-v1'
            and set(contract['locations']) == set(contract['files']), 'closed data role map')
    bodies = {}
    for name, original in contract['files'].items():
        relative = contract['locations'][name]
        require(Path(relative).name == relative, 'flat immutable input alias')
        bodies[name] = reader.read(INPUTS / relative,
                                  {key: original[key] for key in ('bytes', 'sha256')})
    require({p.name for p in INPUTS.iterdir()} == set(contract['locations'].values()) | {'environment.json'},
            'closed original-data-plus-environment input tree')
    return contract, bodies


def implementations(reader, env, model, torch, original):
    module = sys.modules[model.__class__.__module__]
    layer = model.model.layers[0]
    objects = {'modeling_qwen3': model.__class__, 'activation': type(layer.mlp.act_fn),
               'sdpa': module.ALL_ATTENTION_FUNCTIONS['sdpa'], 'torch_functional': torch.nn.functional}
    require(set(env['implementation_sources']) == set(objects), 'four actual implementation sources')
    for name, obj in objects.items():
        path = Path(inspect.getsourcefile(obj)).resolve(strict=True)
        row = env['implementation_sources'][name]
        require(str(path) == row['path'], 'actual implementation source path')
        raw = reader.read(path, row)
        save(OUTPUT / ('implementation-' + name + '.py'), raw)
    require(env['implementation_sources']['modeling_qwen3']['sha256'] == MODEL_SHA
            and type(layer.mlp).__name__ == 'Qwen3MLP'
            and inspect.getsourcefile(type(layer.mlp)) == inspect.getsourcefile(model.__class__)
            and isinstance(layer.mlp.act_fn, torch.nn.Module)
            and model.config.hidden_act == 'silu', 'actual qualified MLP implementation')
    original.verify_implementations(type('Base', (), {'file_pin': lambda _, p: read(p)[1]})(),
                                   {'implementation_sources': env['implementation_sources']}, model, torch)
    return layer


def execute():
    started = time.monotonic()
    reader = Reader()
    report = dict(schema='ferric-guarded-mlp-matched-input-framework-v1', passed=False,
                  error=None, postcheck_errors=[], module_calls_completed=0,
                  full_model_forward_calls=0, numerical_acceptance=False, acceptance_threshold=None,
                  full_model_acceptance=False, performance_claim=False, production_authority=False,
                  original_environment_reused=False, gpu_context_process_exit_not_yet_observed=True,
                  gpu_context_requested=False, gpu_device_admitted=False, framework_execution=False)
    require(OUTPUT.is_dir() and not list(OUTPUT.iterdir()), 'fresh container output')
    model = layer = old = source_pins = None
    torch = None
    gpu_requested = False
    try:
        manifest = source_contract(reader)
        environment = environment_contract(reader)
        import mlp
        contract, bodies = load_inputs(reader)
        native, native_admission = mlp.native_inputs(bodies, contract)
        old_stages = mlp.framework_inputs(bodies, contract)
        base = load(SOURCE / 'framework_reference.py', manifest['files']['framework_reference.py'], 'matched_base')
        old = load(SOURCE / 'long_reference.py', manifest['files']['long_reference.py'], 'matched_long')
        historical = load(SOURCE / 'historical_capture.py', manifest['files']['historical_capture.py'], 'matched_old_capture')
        import psutil
        import torch
        import transformers
        require(psutil.virtual_memory().available >= 48 << 30, '48 GiB initial free host memory')
        gpu_requested = True
        report.update(gpu_context_requested=True, native_execution=False)
        require(torch.cuda.is_available() and torch.cuda.device_count() == 1, 'exactly one exposed GPU')
        actual_arch = torch.cuda.get_device_properties(0).gcnArchName
        require(torch.version.hip == environment['hip']
                and actual_arch.split(':')[0] == environment['expected_arch'] == 'gfx950'
                and torch.cuda.mem_get_info()[0] >= 48 << 30,
                'observed ROCm/gfx950 and free GPU memory')
        report['actual_gcn_arch'] = actual_arch
        report['gpu_device_admitted'] = True
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
        require({p.name for p in Path('/model').iterdir()} == set(old.MODEL_FILES), 'original nine-file checkpoint roster')
        source_pins = old.checked_sources(Path('/model'), old.MODEL_FILES)
        model, loading = transformers.AutoModelForCausalLM.from_pretrained(
            '/model', torch_dtype=torch.bfloat16, attn_implementation='sdpa', local_files_only=True,
            trust_remote_code=False, use_safetensors=True, output_loading_info=True)
        require(type(model).__name__ == 'Qwen3ForCausalLM'
                and model.__class__.__module__ == 'transformers.models.qwen3.modeling_qwen3'
                and all(loading[key] == [] for key in ('missing_keys', 'unexpected_keys',
                                                     'mismatched_keys', 'error_msgs')), 'exact model loading')
        expected = dict(hidden_size=4096, intermediate_size=12288, num_hidden_layers=36,
            num_attention_heads=32, num_key_value_heads=8, head_dim=128, vocab_size=151936,
            rope_theta=1000000, rms_norm_eps=1e-6, tie_word_embeddings=False, model_type='qwen3')
        require(all(getattr(model.config, key) == value for key, value in expected.items()), 'model geometry')
        inverse = old.move_bf16_model_preserving_fp32_rope(model, torch, 'cuda')
        require(not model.training and all(not m.training for m in model.modules())
                and all(p.dtype == torch.bfloat16 and p.device.type == 'cuda' for p in model.parameters()),
                'actual BF16 GPU eval model')
        layer = implementations(reader, environment, model, torch, historical)
        report.update(environment=environment, historical_environment=parse(bodies['framework_capture'])['runtime'],
                      native_admission=native_admission, model_sources=source_pins,
                      model_loader='qualified full AutoModelForCausalLM loader; only layer0.mlp invoked',
                      rotary_sha256=hashlib.sha256(inverse.numpy().tobytes()).hexdigest())
        def bounded():
            require(time.monotonic() - started < 900 and psutil.Process().memory_info().rss <= 64 << 30,
                    'framework wall/RSS bound')
        calls, call_pins = {}, {}
        for name in mlp.CALLS:
            bounded()
            supplied = old_stages['input'] if name.startswith('control') else native[(0, 'post_normalized')]
            tensor = torch.frombuffer(bytearray(supplied), dtype=torch.uint16).clone().view(
                torch.bfloat16).reshape(1, 1, 4096).to('cuda')
            require(tensor.is_contiguous(), 'actual contiguous input')
            raw_tensor = lambda t, shape: base.bf16_raw(t, torch, shape)
            require(raw_tensor(tensor, (1, 1, 4096)) == supplied, 'bit-preserving BF16 device input')
            call_pins[name] = {}
            def retain(stage, body):
                bounded()
                call_pins[name][stage] = save(OUTPUT / (name + '-' + stage + '.bf16'), body)
            hooks = mlp.Hooks(raw_tensor, retain)
            try:
                hooks.add('input', tensor)
                hooks.attach(layer.mlp)
                with torch.inference_mode():
                    returned = layer.mlp(tensor)
                torch.cuda.synchronize()
                require(raw_tensor(returned, (1, 1, 4096)) == hooks.values['mlp-output'], 'returned module output')
                require(raw_tensor(tensor, (1, 1, 4096)) == supplied, 'input unchanged after module')
                mlp.validate_stages(hooks.values, supplied)
                calls[name] = hooks.values
                report['module_calls_completed'] += 1
                report['framework_execution'] = True
            finally:
                hooks.close()
            del tensor, returned
        comparison = mlp.compare_calls(calls, native, old_stages)
        report.update(calls=call_pins, diagnostic=comparison, torch_build=torch.__config__.show(),
                      gpu_execution=True, framework_execution=True, native_execution=False)
        save(OUTPUT / 'diagnostic.json', encoded(comparison))
        require(old.checked_sources(Path('/model'), old.MODEL_FILES) == source_pins, 'model source posthash')
        bounded()
        require(comparison['control_and_repeat_gate_passed'],
                'historical control or repeated native-input calls differ; no matched-input interpretation')
        report['passed'] = True
    except BaseException as error:
        report['error'] = type(error).__name__ + ': ' + str(error)
    finally:
        layer = model = None
        if torch is not None and gpu_requested:
            try:
                torch.cuda.empty_cache()
                torch.cuda.synchronize()
            except BaseException as error:
                report['postcheck_errors'].append('framework cleanup: ' + str(error))
        if source_pins is not None:
            try:
                require(old.checked_sources(Path('/model'), old.MODEL_FILES) == source_pins,
                        'model source final posthash')
            except BaseException as error:
                report['postcheck_errors'].append('model posthash: ' + str(error))
        report['postcheck_errors'].extend(reader.postcheck())
        report['input_pins'] = list(reader.pins.values())
        report['elapsed_seconds'] = time.monotonic() - started
        report['passed'] = report['passed'] and not report['postcheck_errors'] and report['elapsed_seconds'] < 900
        report['output_pins'] = [read(p)[1] for p in sorted(OUTPUT.iterdir())]
        row = save(OUTPUT / ('complete.json' if report['passed'] else 'failed.json'), encoded(report))
        print(encoded(row).decode(), end='', flush=True)
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    require(len(sys.argv) == 1, 'no implicit alternate model/input flags')
    raise SystemExit(execute())
