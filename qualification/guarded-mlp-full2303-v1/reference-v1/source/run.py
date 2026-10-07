"""Two fresh-cache, authentic full2048/256 independent reference passes."""
import hashlib
import importlib.metadata
import inspect
import os
from pathlib import Path
import sys
import struct
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
    require(value['schema'] == 'ferric-full2303-reference-source-v1', 'source manifest schema')
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
    require(contract['schema'] == 'ferric-full2303-reference-files-v1'
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


def run_pass(model, torch, base, reference, tokens, ordinal, bounded, progress, decode):
    from transformers.cache_utils import DynamicCache
    cache = DynamicCache()
    require(cache.get_seq_length() == 0, 'fresh pass cache')
    cases, payloads, captures, generated = [], {}, [], []
    logit_stream = hashlib.sha256()
    for step in range(reference.OUTPUTS):
        bounded()
        current = reference.input_ids(tokens, step, generated)
        length = len(current)
        position = reference.INPUTS - 1 + step
        before = 0 if step == 0 else position
        require(cache.get_seq_length() == before, 'exact own cache before model call')
        positions = (0, 2047) if step == 0 else ((position,) if position in reference.SELECTED else ())
        def raw_slice(tensor, selected_position):
            require(tensor.dtype == torch.bfloat16 and tensor.device.type == 'cuda'
                    and tuple(tensor.shape) == (1, length, 4096), 'actual full-prefill/decode hidden geometry')
            local = selected_position if step == 0 else 0
            return base.bf16_raw(tensor[:, local:local + 1, :], torch, (1, 1, 4096))
        capture = reference.Capture(raw_slice, positions) if positions else None
        try:
            if capture is not None:
                capture.attach(model)
            with torch.inference_mode():
                ids = torch.tensor([current], dtype=torch.long, device='cuda')
                mask = torch.ones((1, position + 1), dtype=torch.long, device='cuda')
                result = model.model(input_ids=ids, attention_mask=mask, past_key_values=cache,
                                     use_cache=True, return_dict=True)
                hidden = result.last_hidden_state
                require(hidden.dtype == torch.bfloat16 and hidden.device.type == 'cuda'
                        and tuple(hidden.shape) == (1, length, 4096)
                        and bool(torch.isfinite(hidden).all().item()), 'finite actual full-model hidden')
                logits = base.bf16_raw(model.lm_head(hidden[:, -1:, :]), torch, (1, 1, 151936))
                greedy = reference.summarize(logits)
                cache = result.past_key_values
                require(isinstance(cache, DynamicCache) and cache.get_seq_length() == position + 1
                        and len(cache.key_cache) == len(cache.value_cache) == 36, 'actual own causal cache')
                cache_hashes = []
                for key, value in zip(cache.key_cache, cache.value_cache):
                    shape = (1, 8, position + 1, 128)
                    require(tuple(key.shape) == tuple(value.shape) == shape
                            and key.dtype == value.dtype == torch.bfloat16
                            and key.device.type == value.device.type == 'cuda', 'all36 KV shapes/dtypes/devices')
                    if positions:
                        cache_hashes.append(dict(key=compact(base.bf16_raw(key, torch, shape)),
                                                 value=compact(base.bf16_raw(value, torch, shape))))
                for selected_position in positions:
                    local = selected_position if step == 0 else 0
                    final = raw_slice(hidden, selected_position)
                    selected_logits = logits if selected_position == position else base.bf16_raw(
                        model.lm_head(hidden[:, local:local + 1, :]), torch, (1, 1, 151936))
                    raw = capture.payload(selected_position, final, selected_logits)
                    save(OUTPUT / ('pass%d-pos%d.bf16' % (ordinal, selected_position)), raw)
                    payloads[selected_position] = raw
                    captures.append(dict(position=selected_position, forward_call=step,
                        generated_index=None if selected_position == 0 else step,
                        input_token=tokens[selected_position] if step == 0 else current[0],
                        cache_length_at_call=position + 1, greedy=reference.summarize(selected_logits),
                        payload=compact(raw),
                        tensors={name: compact(value) for name, value in reference.D.split_payload(raw).items()},
                        cache_hashes=cache_hashes))
                cases.append(dict(step=step, position=position, input_token=current[-1], input_length=length,
                                  cache_before=before, cache_after=position + 1, greedy=greedy))
                logit_stream.update(logits)
                generated.append(greedy['token_id'])
                torch.cuda.synchronize()
                progress(length)
                del ids, mask, result, hidden, logits
                bounded()
        finally:
            if capture is not None:
                capture.close()
    require(cache.get_seq_length() == reference.POSITIONS, 'final cache excludes unconsumed last output')
    del cache
    torch.cuda.synchronize()
    outputs = reference.output_bodies(ordinal, generated, decode)
    for name, raw in outputs.items():
        save(OUTPUT / ('pass%d.%s' % (ordinal, name)), raw)
    value = dict(ordinal=ordinal, fresh_cache=True, framework_calls=len(cases),
        positions_processed=reference.POSITIONS, generated_tokens=generated,
        prompt_tokens=compact(struct.pack('<2048I', *tokens)), cases=cases, captures=captures,
        logit_stream=dict(bytes=reference.OUTPUTS * reference.VOCABULARY * 2, sha256=logit_stream.hexdigest()),
        logit_pin_chain_sha256=hashlib.sha256(b''.join(
            bytes.fromhex(c['greedy']['logits']['sha256']) for c in cases)).hexdigest(),
        output_pins={name: compact(raw) for name, raw in outputs.items()})
    reference.validate_pass(value, tokens, payloads, outputs, decode)
    return value, payloads, outputs

def execute():
    started = time.monotonic()
    reader = Reader()
    report = dict(schema='ferric-full2303-framework-reference-v1', passed=False,
        error=None, postcheck_errors=[], full_model_forward_calls=0, generated_tokens=256, generated_token_choices=0, positions_processed=0,
        prompt_tokens_authenticated=0, selected_positions=[0, 2047, 2048, 2302], repeat_gate_passed=False,
        numerical_acceptance=False, acceptance_threshold=None, full_model_acceptance=False,
        full_long_workload=True, performance_claim=False, production_authority=False,
        native_execution=False, native_intermediates_used=False, candidate_receipt=None,
        gpu_context_process_exit_not_yet_observed=True, gpu_context_requested=False,
        gpu_device_admitted=False, framework_execution=False)
    require(OUTPUT.is_dir() and not list(OUTPUT.iterdir()), 'fresh container output')
    model = old = source_pins = torch = None
    gpu_requested = False
    try:
        manifest = source_contract(reader)
        environment = environment_contract(reader)
        import full_reference as reference
        contract, bodies = load_inputs(reader)
        tokens, prompt, expected_sources = reference.prompt_inputs(contract, bodies)
        base = load(SOURCE / 'framework_reference.py', manifest['files']['framework_reference.py'], 'readiness_base')
        old = load(SOURCE / 'long_reference.py', manifest['files']['long_reference.py'], 'readiness_long')
        import psutil
        import torch
        import transformers
        require(psutil.virtual_memory().available >= 48 << 30, '48 GiB initial free host memory')
        def bounded():
            require(time.monotonic() - started < 900 and psutil.Process().memory_info().rss <= 64 << 30,
                    'framework wall/RSS bound')
        gpu_requested = True
        report['gpu_context_requested'] = True
        require(torch.cuda.is_available() and torch.cuda.device_count() == 1, 'exactly one exposed GPU')
        actual_arch = torch.cuda.get_device_properties(0).gcnArchName
        require(torch.version.hip == environment['hip']
                and actual_arch.split(':')[0] == environment['expected_arch'] == 'gfx950'
                and torch.cuda.mem_get_info()[0] >= 48 << 30, 'actual ROCm/gfx950 and free GPU memory')
        report.update(actual_gcn_arch=actual_arch, gpu_device_admitted=True)
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
        require({n: {k: p[k] for k in ('bytes', 'sha256')} for n, p in source_pins.items()} == expected_sources,
                'same original checkpoint before any model forward')
        tokenizer, decode, special = old.tokenizer_and_raw_decoder(Path('/model'))
        checked_tokens, checked_prompt = old.checked_prompt(dict(prompt_bundle=str(INPUTS),
            prompt_manifest_sha256=contract['files']['prompt_manifest']['sha256'],
            prompt_seed_sha256=prompt['seed_sha256']), tokenizer, decode, special)
        require(checked_tokens == tokens and checked_prompt == prompt, 'actual tokenizer/decode full2048 roundtrip')
        report['prompt_tokens_authenticated'] = len(tokens)
        bounded()
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
                and all(p.dtype == torch.bfloat16 and p.device.type == 'cuda' for p in model.parameters())
                and len(model.model.layers) == 36 and model.config._attn_implementation == 'sdpa',
                'actual BF16 GPU eval model and all36 layers')
        # The same four implementation files are authenticated and retained, without an old-host identity claim.
        objects = {'modeling_qwen3': model.__class__,
            'activation': type(model.model.layers[0].mlp.act_fn),
            'sdpa': sys.modules[model.__class__.__module__].ALL_ATTENTION_FUNCTIONS['sdpa'],
            'torch_functional': torch.nn.functional}
        require(set(environment['implementation_sources']) == set(objects), 'four actual implementation sources')
        for name, obj in objects.items():
            path = Path(inspect.getsourcefile(obj)).resolve(strict=True)
            row = environment['implementation_sources'][name]
            require(str(path) == row['path'], 'actual implementation source path')
            save(OUTPUT / ('implementation-' + name + '.py'), reader.read(path, row))
        require(environment['implementation_sources']['modeling_qwen3']['sha256'] == MODEL_SHA
                and all(type(layer).__name__ == 'Qwen3DecoderLayer'
                        and type(layer).__module__ == model.__class__.__module__ for layer in model.model.layers),
                'real Qwen3 decoder classes from pinned implementation')
        report.update(environment=environment, model_sources=source_pins, model_id=contract['model_id'],
            bundle_id=contract['bundle_id'], prompt=compact(bodies['prompt_manifest']),
            full_prompt_tokens=tokens, input_tokens=tokens, output_tokens_per_pass=256,
            reference_prefill='one full2048-token independent matrix-model call per pass',
            generated_feedback='own preceding BF16 lowest-ID argmax, never reference/native tokens',
            ignore_eos=True, decoded_special_token_policy='skip',
            selected_position0='slice of matrix-prefill call with actual cache length2048; diagnostic only',
            logit_storage='all256 finite rows hashed in order; raw rows retained only in four full captures',
            model_loader='qualified full BF16 AutoModelForCausalLM loader',
            policy=dict(sdpa='math-only', deterministic_algorithms=True,
                bf16_reduced_precision_matmul_reduction=False, sdpa_low_precision_reduction=False,
                rotary_fp32_preserved=True, autocast=False),
            rotary_sha256=hashlib.sha256(inverse.numpy().tobytes()).hexdigest())
        passes, payloads, outputs = [], [], []
        def progress(length):
            report['full_model_forward_calls'] += 1
            report['positions_processed'] += length
            report['generated_token_choices'] += 1
            report['framework_execution'] = True
        for ordinal in (1, 2):
            value, raw, out = run_pass(model, torch, base, reference, tokens, ordinal, bounded, progress, decode)
            passes.append(value)
            payloads.append(raw)
            outputs.append(out)
            save(OUTPUT / ('pass%d.json' % ordinal), encoded(value))
        report.update(passes=passes, torch_build=torch.__config__.show(),
                      repeat_gate_passed=reference.repeat_gate(passes, tokens, payloads, outputs, decode),
                      bounded_output_bytes=reference.evidence_bound(passes, payloads, outputs))
        require(report['full_model_forward_calls'] == 512 and report['positions_processed'] == 4606
                and report['generated_token_choices'] == 512 and report['repeat_gate_passed'],
                'two authentic full workloads, 512 calls/choices and exact two-pass repeat gate')
        require(old.checked_sources(Path('/model'), old.MODEL_FILES) == source_pins, 'model source posthash')
        bounded()
        report['passed'] = True
    except BaseException as error:
        report['error'] = type(error).__name__ + ': ' + str(error)
    finally:
        model = None
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
