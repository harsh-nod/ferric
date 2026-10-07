#!/usr/bin/env python3
"""Independent four-forward rearm diagnostic; original weights and own KV only."""
import hashlib
import importlib.metadata
import inspect
import json
import os
from pathlib import Path
import socket
import stat
import struct
import sys
import time
import types

LEGACY_SHA = '242abaa1662372ec999d6d056272646ede14ed8285223bebb293d1a64c17c9cf'
LEGACY_PLAN_SHA = '5d0a2940e746255c8ab8e23551e567a17f89342f4cecdf7cac6733e84bb3c8da'
MODEL_SOURCE_SHA = '704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2'
PROMPT_SHA = '30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600'
PROMPT_IDS_SHA = '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02'
MODEL_ID = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
BUNDLE_ID = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
FORWARDS = 4
INPUT_TOKENS = [9112, 2190, 3772, 220]
LIMITS = {'timeout_seconds': 900, 'host_rss_cap_bytes': 64 << 30,
          'minimum_free_host_bytes': 48 << 30, 'minimum_free_gpu_bytes': 48 << 30,
          'output_cap_bytes': 32 << 20, 'cache_cap_bytes': 1 << 30}
PACKAGES = {'accelerate': '1.14.0', 'numpy': '1.26.4', 'psutil': '7.0.0',
            'safetensors': '0.5.3', 'tokenizers': '0.21.4', 'torch': '2.12.1+rocm7.2',
            'transformers': '4.51.0', 'triton-rocm': '3.7.1'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def document(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def invalid(_):
        raise ValueError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def file_pin(path):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path and not path.is_symlink(), 'canonical file')
    before = path.stat()
    require(stat.S_ISREG(before.st_mode), 'regular file')
    value = hashlib.sha256()
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        require(os.fstat(stream.fileno()) == before, 'opened file identity')
        while True:
            chunk = stream.read(4 << 20)
            if not chunk:
                break
            value.update(chunk)
        after = os.fstat(stream.fileno())
    require(all(getattr(before, key) == getattr(after, key) == getattr(path.stat(), key)
                for key in ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')), 'file changed')
    return {'path': str(path), 'bytes': before.st_size, 'sha256': value.hexdigest()}


def read_pin(pin, maximum):
    require(isinstance(pin, dict) and set(pin) == {'path', 'bytes', 'sha256'} and
            type(pin['bytes']) is int and 0 < pin['bytes'] <= maximum and
            isinstance(pin['sha256'], str) and len(pin['sha256']) == 64 and
            all(c in '0123456789abcdef' for c in pin['sha256']), 'bounded file pin')
    path = Path(pin['path'])
    require(path.is_absolute() and path.resolve(strict=True) == path and not path.is_symlink(), 'canonical pin')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size == pin['bytes'], 'pin regular extent')
        raw = stream.read(maximum + 1)
        after = os.fstat(stream.fileno())
    require(len(raw) == pin['bytes'] and digest(raw) == pin['sha256'], 'pin content')
    require(all(getattr(before, key) == getattr(after, key) == getattr(path.stat(), key)
                for key in ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')), 'pin changed')
    return raw


def load_module(path, expected_sha, maximum):
    pin = file_pin(path)
    require(pin['sha256'] == expected_sha, 'module source identity')
    raw = read_pin(pin, maximum)
    module = types.ModuleType('_p224_rearm_' + path.stem)
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, pin


def select_prompt_tokens(manifest, raw, mode, explicit):
    require(isinstance(raw, bytes) and len(raw) == 8192, 'original prompt IDs extent')
    tokens = list(struct.unpack('<2048I', raw))
    require(manifest['schema'] == 'FerricQwen3LongPromptV1' and
            manifest['input_token_ids'] == tokens and manifest['input_tokens'] == 2048 and
            manifest['add_special_tokens'] is False and manifest['chat_template'] is None and
            all(type(x) is int and 0 <= x < 151643 for x in tokens), 'original raw prompt contract')
    require(mode == 'teacher_forced' and explicit == INPUT_TOKENS == tokens[:FORWARDS] and
            all(type(x) is int for x in explicit), 'explicit first-token selection')
    return explicit


def checked_plan(path, sha):
    pin = file_pin(Path(path))
    require(pin['sha256'] == sha, 'plan digest')
    raw = read_pin(pin, 1 << 20)
    plan = document(raw)
    fields = {'schema', 'model_id', 'bundle_id', 'harness_sha256', 'diagnostics', 'policy', 'legacy_plan', 'mode', 'input_tokens',
              'token_provenance', 'source_authentication', 'host', 'boot_id', 'gpu_unique_id',
              'gpu_unique_id_file', 'hip_visible_devices', 'python_executable', 'python_prefix',
              'model_root', 'output_root', 'supervisor_pid', 'execution_review', 'resources'}
    require(isinstance(plan, dict) and set(plan) == fields and
            plan['schema'] == 'ferric-p224-rearm-four-framework-reference-plan-v1', 'plan schema')
    require(plan['model_id'] == MODEL_ID and plan['bundle_id'] == BUNDLE_ID, 'target and bundle identity')
    root = Path(__file__).resolve(strict=True).parent
    own = file_pin(root / 'framework_reference.py')
    require(plan['harness_sha256'] == own['sha256'], 'current harness identity')
    require(plan['diagnostics']['path'] == str(root / 'diagnostics.py') and
            plan['policy']['path'] == str(root / 'policy.json'), 'local policy/helper paths')
    diag, diag_pin = load_module(root / 'diagnostics.py', plan['diagnostics']['sha256'], 32768)
    require(diag_pin == plan['diagnostics'], 'diagnostic module pin')
    policy = document(read_pin(plan['policy'], 32768))
    require(policy['schema'] == 'ferric-p224-rearm-four-framework-reference-policy-v1' and
            policy['numerical_acceptance'] is False and policy['acceptance_threshold'] is None,
            'diagnostic policy')
    old, old_pin = load_module(root / 'helpers/long_reference.py', LEGACY_SHA, 65536)
    legacy = document(read_pin(plan['legacy_plan'], 65536))
    require(plan['legacy_plan']['sha256'] == LEGACY_PLAN_SHA and legacy['harness_sha256'] == LEGACY_SHA,
            'retained framework plan')
    require(plan['model_root'] == legacy['model_root'] and
            Path(plan['model_root']).resolve(strict=True) == Path(plan['model_root']), 'original model root')
    require(plan['python_executable'] == legacy['python_executable'] == str(Path(sys.executable).absolute()) and
            plan['python_prefix'] == legacy['python_prefix'] == sys.prefix and sys.prefix != sys.base_prefix,
            'retained isolated environment')
    require(sys.flags.isolated == 1 and sys.dont_write_bytecode and sys.version_info[:2] == (3, 10) and
            sys.byteorder == 'little', 'Python3.10 -I -B little endian')
    require(legacy['packages'] == PACKAGES and
            all(importlib.metadata.version(key) == value for key, value in PACKAGES.items()), 'retained packages')
    require(plan['host'] == socket.gethostname() and
            plan['boot_id'] == Path('/proc/sys/kernel/random/boot_id').read_text().strip(), 'fresh host/boot')
    require(type(plan['supervisor_pid']) is int and plan['supervisor_pid'] > 1, 'supervisor PID')
    require(plan['resources'] == LIMITS and all(type(x) is int for x in plan['resources'].values()), 'fixed bounds')
    require(isinstance(plan['gpu_unique_id'], str) and len(plan['gpu_unique_id']) == 16 and
            all(c in '0123456789abcdef' for c in plan['gpu_unique_id']) and
            isinstance(plan['hip_visible_devices'], str) and plan['hip_visible_devices'].isdigit(), 'device identity')
    require(os.environ.get('HIP_VISIBLE_DEVICES') == plan['hip_visible_devices'] and
            'ROCR_VISIBLE_DEVICES' not in os.environ, 'unambiguous selected GPU')
    require(Path(plan['gpu_unique_id_file']).read_text().strip() == plan['gpu_unique_id'], 'selected physical GPU')
    provenance = plan['token_provenance']
    require(isinstance(provenance, dict) and set(provenance) == {'manifest', 'prompt_ids'} and
            provenance['manifest']['sha256'] == PROMPT_SHA and provenance['prompt_ids']['sha256'] == PROMPT_IDS_SHA,
            'authentic prompt pins')
    prompt = document(read_pin(provenance['manifest'], 1 << 20))
    ids = read_pin(provenance['prompt_ids'], 8192)
    diag.token_inputs(plan['mode'], plan['input_tokens'])
    select_prompt_tokens(prompt, ids, plan['mode'], plan['input_tokens'])
    require(prompt['revision'] == old.REVISION, 'prompt/model revision')
    read_pin(plan['source_authentication'], 16 << 20)
    read_pin(plan['execution_review'], 16 << 20)
    pins = [pin, own, diag_pin, old_pin, plan['policy'], plan['legacy_plan'],
            provenance['manifest'], provenance['prompt_ids'], plan['source_authentication'], plan['execution_review']]
    return plan, old, diag, policy, pins


class Capture:
    def __init__(self, path):
        self.path, self.total, self.files = Path(path), 0, {}
        require(self.path.is_absolute() and self.path.parent.resolve(strict=True) == self.path.parent and
                not self.path.exists() and not self.path.is_symlink(), 'new canonical output')
        self.path.mkdir(mode=0o700)
        (self.path / 'private-cache').mkdir(mode=0o700)

    def write(self, name, raw):
        require(isinstance(raw, bytes) and name and len(name) < 128 and name.isascii() and
                all(c.isalnum() or c in '.-' for c in name) and name not in ('.', '..') and
                name not in self.files and
                len(self.files) < 256 and self.total + len(raw) <= LIMITS['output_cap_bytes'], 'bounded unique output')
        with (self.path / name).open('xb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        self.total += len(raw)
        result = {'path': str(self.path / name), 'bytes': len(raw), 'sha256': digest(raw)}
        self.files[name] = result
        return result


def check_live_bounds(capture, started, psutil):
    require(time.monotonic() - started <= LIMITS['timeout_seconds'], 'reference wall deadline')
    require(psutil.Process().memory_info().rss <= LIMITS['host_rss_cap_bytes'], 'reference RSS cap')
    total, count = 0, 0
    def onerror(error):
        raise error
    for directory, directories, files in os.walk(capture.path / 'private-cache', followlinks=False, onerror=onerror):
        for name in [*directories, *files]:
            path = Path(directory) / name
            status = path.lstat()
            require(stat.S_ISDIR(status.st_mode) or stat.S_ISREG(status.st_mode), 'private cache links/special file')
            total += status.st_size if stat.S_ISREG(status.st_mode) else 0
            count += 1
            require(count <= 16384 and total <= LIMITS['cache_cap_bytes'], 'private cache bound')


def bf16_raw(tensor, torch, shape):
    require(tensor.dtype == torch.bfloat16 and tuple(tensor.shape) == shape, 'captured BF16 tensor shape')
    require(bool(torch.isfinite(tensor).all().item()), 'nonfinite framework tensor')
    return tensor.detach().reshape(-1).contiguous().cpu().view(torch.uint16).numpy().tobytes()


def run_pass(model, torch, diag, plan, capture, ordinal, bounded):
    from transformers.cache_utils import DynamicCache
    cache = DynamicCache()
    require(cache.get_seq_length() == 0, 'fresh pass cache')
    cases = []
    diag.token_inputs(plan['mode'], plan['input_tokens'])
    for position in range(FORWARDS):
        token = plan['input_tokens'][position]
        hidden, norm_inputs, hooks = {}, [], []
        def layer_hook(layer):
            def observe(_module, _arguments, output):
                require(layer not in hidden, 'duplicate layer capture')
                value = output[0] if isinstance(output, (tuple, list)) else output
                hidden[layer] = bf16_raw(value, torch, (1, 1, 4096))
                bounded()
            return observe
        def norm_hook(_module, arguments):
            require(len(arguments) >= 1 and not norm_inputs, 'final norm input count')
            norm_inputs.append(bf16_raw(arguments[0], torch, (1, 1, 4096)))
        try:
            for layer, module in enumerate(model.model.layers):
                hooks.append(module.register_forward_hook(layer_hook(layer)))
            hooks.append(model.model.norm.register_forward_pre_hook(norm_hook))
            with torch.inference_mode():
                ids = torch.tensor([[token]], dtype=torch.long, device='cuda')
                mask = torch.ones((1, position + 1), dtype=torch.long, device='cuda')
                result = model.model(input_ids=ids, attention_mask=mask, past_key_values=cache,
                                     use_cache=True, return_dict=True)
                final = bf16_raw(result.last_hidden_state, torch, (1, 1, 4096))
                logits = bf16_raw(model.lm_head(result.last_hidden_state), torch, (1, 1, diag.VOCABULARY))
                cache = result.past_key_values
                require(cache is not None and cache.get_seq_length() == position + 1 and
                        len(cache.key_cache) == len(cache.value_cache) == 36, 'causal own-cache length/layers')
                cache_hashes = []
                for key, value in zip(cache.key_cache, cache.value_cache):
                    cache_hashes.append({'key': digest(bf16_raw(key, torch, (1, 8, position + 1, 128))),
                                         'value': digest(bf16_raw(value, torch, (1, 8, position + 1, 128)))})
                require(set(hidden) == set(range(36)) and norm_inputs == [hidden[35]], 'all raw layer outputs/final norm join')
                payload = b''.join(hidden[layer] for layer in range(36)) + final + logits
                rows = diag.split_payload(payload)
                choice = diag.argmax(logits)
                record = {'generation': position + 1, 'position': position, 'input_token': token, 'output_token': choice}
                diag.validate_case(record, payload, position)
                pin = capture.write(f'pass{ordinal}-pos{position}.bf16', payload)
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
    return {'ordinal': ordinal, 'fresh_cache': True, 'cases': cases}


def repeat_equal(passes):
    if not isinstance(passes, list) or len(passes) != 2:
        return False
    for ordinal, value in enumerate(passes, 1):
        if (not isinstance(value, dict) or value.get('ordinal') != ordinal or
                value.get('fresh_cache') is not True or not isinstance(value.get('cases'), list) or
                len(value['cases']) != FORWARDS):
            return False
        for position, case in enumerate(value['cases']):
            if (not isinstance(case, dict) or set(case) != {'record', 'payload', 'tensors', 'cache_sha256'} or
                    not isinstance(case['record'], dict) or not isinstance(case['payload'], dict) or
                    not isinstance(case['payload'].get('sha256'), str) or
                    not isinstance(case['tensors'], dict) or not isinstance(case['cache_sha256'], list) or
                    len(case['cache_sha256']) != 36 or case['record'].get('position') != position or
                    case['record'].get('generation') != position + 1 or
                    case['record'].get('input_token') != INPUT_TOKENS[position]):
                return False
    return all(left['record'] == right['record'] and left['payload']['sha256'] == right['payload']['sha256'] and
               left['tensors'] == right['tensors'] and left['cache_sha256'] == right['cache_sha256']
               for left, right in zip(passes[0]['cases'], passes[1]['cases']))


def execute(plan, old, diag, policy, pins):
    require(os.getppid() == plan['supervisor_pid'], 'root-owned direct supervisor required')
    for key in ('HF_HUB_OFFLINE', 'TRANSFORMERS_OFFLINE', 'HF_DATASETS_OFFLINE'):
        os.environ[key] = '1'
    os.environ['TOKENIZERS_PARALLELISM'] = 'false'
    os.environ['CUBLAS_WORKSPACE_CONFIG'] = ':4096:8'
    capture = Capture(plan['output_root'])
    for name in ('XDG_CACHE_HOME', 'HF_HOME', 'TORCH_HOME', 'TRITON_CACHE_DIR', 'MIOPEN_USER_DB_PATH'):
        os.environ[name] = str(capture.path / 'private-cache' / name.lower())
    capture.write('input-plan.json', encoded(plan))
    started = time.monotonic()
    root = Path(plan['model_root'])
    for name in old.MODEL_FILES:
        path = root / name
        require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical original model file')
    sources = old.checked_sources(root, old.MODEL_FILES)
    import psutil
    require(psutil.virtual_memory().available >= LIMITS['minimum_free_host_bytes'], 'free host memory prerequisite')
    import torch
    import transformers
    require(torch.cuda.is_available() and torch.cuda.device_count() == 1 and
            str(torch.version.hip).startswith('7.2'), 'ROCm/visibility')
    properties = torch.cuda.get_device_properties(0)
    require(properties.gcnArchName == 'gfx950:sramecc+:xnack-' and
            torch.cuda.mem_get_info()[0] >= LIMITS['minimum_free_gpu_bytes'], 'gfx950/free VRAM')
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
    require(model.__class__.__name__ == 'Qwen3ForCausalLM' and
            model.__class__.__module__ == 'transformers.models.qwen3.modeling_qwen3', 'framework model class')
    require(all(not loading[name] for name in ('missing_keys', 'unexpected_keys', 'mismatched_keys', 'error_msgs')),
            'exact checkpoint load')
    model_source = file_pin(Path(inspect.getsourcefile(model.__class__)).resolve(strict=True))
    require(model_source['sha256'] == MODEL_SOURCE_SHA, 'retained model implementation')
    require(all(getattr(model.config, key) == value for key, value in {
        'hidden_size':4096, 'intermediate_size':12288, 'num_hidden_layers':36, 'num_attention_heads':32,
        'num_key_value_heads':8, 'head_dim':128, 'vocab_size':151936, 'rope_theta':1000000.0,
        'rms_norm_eps':1e-6, 'tie_word_embeddings':False, 'model_type':'qwen3'}.items()), 'model geometry')
    inverse = old.move_bf16_model_preserving_fp32_rope(model, torch, 'cuda')
    require(len(model.model.layers) == 36 and model.config._attn_implementation == 'sdpa' and
            all(parameter.dtype == torch.bfloat16 for parameter in model.parameters()), 'actual framework profile')
    bounded = lambda: check_live_bounds(capture, started, psutil)
    bounded()
    passes = [run_pass(model, torch, diag, plan, capture, ordinal, bounded) for ordinal in (1, 2)]
    torch.cuda.synchronize()
    equal = repeat_equal(passes)
    runtime = {'python':sys.version, 'python_executable':sys.executable, 'python_prefix':sys.prefix,
               'packages':PACKAGES, 'model_class_source':model_source, 'hip':torch.version.hip,
               'gcn_arch':properties.gcnArchName, 'sdpa_backend':'math-only',
               'deterministic_algorithms':True, 'bf16_reduced_precision_matmul_reduction':False,
               'sdpa_low_precision_reduction':False, 'rotary_fp32_preserved':True,
               'rotary_sha256':digest(inverse.numpy().tobytes()), 'torch_build':torch.__config__.show(),
               'loaded_provider_paths':sorted({line.split()[-1] for line in Path('/proc/self/maps').read_text().splitlines()
                                              if '/' in line and any(key in line for key in
                                                ('libtorch','libamdhip','libhsa-runtime','librocblas','libhipblas','libMIOpen'))})}
    del model
    torch.cuda.empty_cache()
    torch.cuda.synchronize()
    bounded()
    require(old.checked_sources(root, old.MODEL_FILES) == sources, 'original model changed')
    for pin in [*pins, model_source]:
        require(file_pin(Path(pin['path'])) == pin, 'input/runtime pin changed')
    require(plan['boot_id'] == Path('/proc/sys/kernel/random/boot_id').read_text().strip() and
            plan['gpu_unique_id'] == Path(plan['gpu_unique_id_file']).read_text().strip(), 'device/boot changed')
    report = {'schema':'ferric-p224-rearm-four-framework-reference-v1', 'status':'PASS' if equal else 'FAIL',
              'model':'Qwen/Qwen3-8B', 'revision':old.REVISION, 'model_id':MODEL_ID, 'bundle_id':BUNDLE_ID,
              'mode':plan['mode'], 'selected_input_tokens':plan['input_tokens'], 'passes':passes,
              'repeat_passes_byte_equal':equal, 'policy':policy, 'input_pins':pins, 'model_sources':sources,
              'runtime':runtime, 'gpu_execution':True, 'candidate_gpu_execution':False,
              'candidate_intermediate_inputs':False, 'numerical_acceptance':False, 'acceptance_threshold':None,
              'full_model_correctness':False, 'production_authority':False, 'performance_measured':False,
              'supervisor_lifecycle_required':True, 'gpu_context_process_exit_not_yet_observed':True}
    result = capture.write('reference.json', encoded(report))
    require(equal, 'independent repeat passes disagree; FAIL record preserved')
    return result


def main():
    require(len(sys.argv) == 4 and sys.argv[1] in ('--inspect', '--run-reviewed-framework-reference'),
            'framework_reference.py (--inspect|--run-reviewed-framework-reference) PLAN SHA256')
    plan, old, diag, policy, pins = checked_plan(sys.argv[2], sys.argv[3])
    if sys.argv[1] == '--inspect':
        print(json.dumps({'schema':'ferric-p224-rearm-four-framework-reference-inspection-v1', 'mode':plan['mode'],
                          'input_tokens':plan['input_tokens'], 'input_pins':pins, 'gpu_opened':False,
                          'model_source_rehashed':False, 'numerical_acceptance':False}, sort_keys=True))
    else:
        print(json.dumps(execute(plan, old, diag, policy, pins), sort_keys=True))


if __name__ == '__main__':
    main()
