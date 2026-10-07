#!/usr/bin/env python3
"""Frozen matched-cell preparation and admission; no launch on import."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import sys

MODEL = 'Qwen/Qwen3-8B'
CANONICAL_TARGET = '/tmp/ferric-qwen8.TRNKht/models/target'
TARGET = '/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target'
HOST = 'smci350-rck-g03-b19-03'
HOST_UID = 9661
RENDER_GID = 993
CLIENT_SHA = '979136caea4f134f33f19c62b82a8ac9537205eaa11d11a43b7a3af466af0a2d'
SERVING_SHA = '347754cb8d88da4639beb0163f54c0ca091288e9c11d6e47e29e47db1dddaf7d'
GPU_IDS = [16366993098680759275, 10838076764495710945, 15340779317222226279,
           17768109136179504279, 230301366604081239, 9271120227827419749,
           10521817515609150476, 7699048935395882881]
IMAGES = {
    'vllm': {'id': 'sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba',
             'digest': 'vllm/vllm-openai-rocm@sha256:e0a3b2bd3fe7ec563916c3a5d949898d133458c18d6b2f460c906885cfb32032',
             'version_label': '0.28.0', 'port': 18981},
    'sglang': {'id': 'sha256:cb8089ca16bd9182698b1bb5a915e6982bf9eeff63d2f6d027a9c31d8d6279d3',
              'digest': 'lmsysorg/sglang-rocm@sha256:4ed6de613ad8f9ac7af18c0947fbe0db2faaae0509a2309778fb0e4c7bc5de5b',
              'version_label': '0.5.15.post1-rocm720-mi35x-20260715', 'port': 18982},
}
POLICY = 'exact-greedy-token-ids-and-decoded-utf8-v1'
PLAN_SCHEMA = 'FerricNativeMatched128LaunchPlanV1'
DRIVER_FILES = ('run_matched128.py', 'matched128_contract.py', 'sitecustomize.py',
    'selected_native.py', 'http_lifecycle.py', 'gpu_attribution.py', 'container_custody.py',
    'gpu_probe.py', 'probe_cli.py', 'owned_command.py', 'http_cpu.py', 'prepare_native_http.py', 'bind_http_cpu.py',
    'frozen/native_lifecycle.py', 'frozen/gpu_activity.py', 'gpu_activity.py')
SETTINGS = {'prompt_tokens': 128, 'completion_tokens': 128, 'concurrency': 1,
            'warmups': 10, 'samples': 30, 'temperature': 0, 'seed': 0,
            'ignore_eos': True, 'speculation': False, 'prefix_cache': False,
            'context': 8192, 'dtype': 'bfloat16', 'head_dtype': 'float32', 'arrival': 'closed-loop',
            'request_timeout_seconds': 120, 'ttft_slo_ms': 10000, 'tpot_slo_ms': 1000}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(path, limit=256 * 1024**2):
    path = Path(path)
    require(not path.is_symlink(), f'symlink input rejected: {path}')
    with path.open('rb') as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit, 'bounded regular file required')
        result = hashlib.file_digest(source, 'sha256').hexdigest()
        after = os.fstat(source.fileno())
        require(all(getattr(before, key) == getattr(after, key) for key in
                    ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')), 'file changed while hashing')
    return result


def sha(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'exact SHA256 required')
    return value


def read(path, expected=None, limit=1024**2):
    path = Path(path)
    if expected is not None:
        require(digest(path, limit) == sha(expected), f'file hash drifted: {path}')
    with path.open('rb') as source:
        raw = source.read(limit + 1)
    require(0 < len(raw) <= limit, 'bounded JSON input required')
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON field')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def write(path, value):
    with Path(path).open('x', encoding='utf-8') as output:
        json.dump(value, output, sort_keys=True, indent=2, allow_nan=False)
        output.write('\n')


def module(name, binding, expected):
    require(binding['sha256'] == expected and digest(binding['path']) == expected, 'frozen helper source drifted')
    spec = importlib.util.spec_from_file_location(name, binding['path'])
    result = importlib.util.module_from_spec(spec)
    sys.modules[name] = result
    spec.loader.exec_module(result)
    return result


def binding_value(binding, limit=1024**2):
    require(type(binding) is dict and set(binding) == {'path', 'sha256'}, 'exact file binding required')
    return read(binding['path'], binding['sha256'], limit)


def validate_plan(plan):
    require(type(plan) is dict and set(plan) == {'schema', 'settings', 'workload', 'tokenizer_receipt',
        'target_manifest', 'reference', 'client', 'serving', 'ferric', 'driver_sources', 'render_device',
        'cpu_qualification'},
        'launch plan field set drifted')
    require(plan['schema'] == PLAN_SCHEMA and plan['settings'] == SETTINGS,
            'frozen 128/128 settings drifted')
    require(type(plan['render_device']) is str and re.fullmatch('/dev/dri/renderD[0-9]+', plan['render_device']),
            'explicit render device required')
    for name, expected in (('client', CLIENT_SHA), ('serving', SERVING_SHA)):
        require(plan[name]['sha256'] == expected and digest(plan[name]['path']) == expected, 'shared helper hash drifted')
    require(set(plan['driver_sources']) == set(DRIVER_FILES),
            'exact external driver source set required')
    for name in plan['driver_sources']:
        require(plan['driver_sources'].get(name) == digest(Path(__file__).parent / name), 'driver source drifted')
    if plan['cpu_qualification'] is not None:
        cpu_path = Path(__file__).with_name('http_cpu.py')
        cpu = module('native_http_cpu_contract', {'path': str(cpu_path),
            'sha256': plan['driver_sources']['http_cpu.py']}, plan['driver_sources']['http_cpu.py'])
        def raw(binding, limit):
            require(digest(binding['path'], limit) == binding['sha256'], 'raw CPU receipt changed')
            return Path(binding['path']).read_bytes()
        cpu.validate(binding_value(plan['cpu_qualification']), plan['driver_sources'],
            bound=binding_value, raw=raw, digest=digest, root=Path(__file__).parent)
    workload = binding_value(plan['workload'])
    receipt = binding_value(plan['tokenizer_receipt'])
    target = binding_value(plan['target_manifest'])
    require(workload.get('schema') == 'FerricCompetitiveWorkloadV1' and workload.get('model') == MODEL
            and len(workload.get('requests', [])) == 1, 'one canonical raw-prompt workload required')
    item = workload['requests'][0]
    require(set(item) == {'id', 'prompt', 'max_tokens'} and item['id'] == 'matched-128-v1'
            and item['max_tokens'] == 128 and type(item['prompt']) is str and item['prompt'].isascii()
            and len(item['prompt'].encode()) <= 4096, 'fixed raw prompt/output length required')
    require(receipt.get('schema') == 'FerricMatched128TokenizerReceiptV1'
            and receipt.get('workload_sha256') == plan['workload']['sha256']
            and receipt.get('target_manifest_sha256') == plan['target_manifest']['sha256']
            and receipt.get('add_special_tokens_true_ids_equal_false') is True
            and receipt.get('chat_template_applied') is False, 'tokenization receipt scope drifted')
    ids = receipt.get('prompt_token_ids')
    require(type(ids) is list and len(ids) == 128 and all(type(i) is int and 0 <= i < 151936 for i in ids),
            '128 canonical prompt IDs required')
    require(hashlib.sha256(item['prompt'].encode()).hexdigest() == receipt['prompt_utf8_sha256'], 'prompt bytes drifted')
    require(target.get('schema') == 'FerricCanonicalTargetFileHashesV1' and target.get('root') == CANONICAL_TARGET
            and len(target.get('files', [])) == 9, 'complete canonical target file identity required')
    names = {'config.json', 'tokenizer.json', 'tokenizer_config.json', 'model.safetensors.index.json'}
    names.update(f'model-{i:05d}-of-00005.safetensors' for i in range(1, 6))
    require({entry['name'] for entry in target['files']} == names, 'target filename roster drifted')
    for entry in target['files']:
        sha(entry['sha256'])
        require(type(entry['bytes']) is int and 0 < entry['bytes'] < 8 * 1024**3,
                'target file size invalid')
    reference = binding_value(plan['reference']) if plan['reference'] is not None else None
    if reference is not None:
        require(reference.get('schema') == 'FerricMatched128ReferenceV1' and reference.get('policy') == POLICY
                and reference.get('workload_sha256') == plan['workload']['sha256']
                and reference.get('target_manifest_sha256') == plan['target_manifest']['sha256']
                and reference.get('prompt_token_ids') == ids
                and reference.get('independent_producer') not in (None, '', 'ferric'), 'independent reference binding required')
        output = reference.get('generated_token_ids')
        require(type(output) is list and len(output) == 128
                and all(type(i) is int and 0 <= i < 151936 for i in output), '128 independent reference output IDs required')
        sha(reference.get('producer_evidence_sha256'))
        text = bytes.fromhex(reference['generated_utf8_hex']).decode('utf-8')
        require(0 < len(text.encode()) <= 32768, 'bounded reference UTF8 required')
    if plan['ferric'] is not None:
        validate_ferric(plan['ferric'], plan['driver_sources'])
        require(plan['ferric']['prompt'] == item['prompt'], 'HTTP and native prerequisite prompts must match')
    return workload, receipt, target, reference


def selection_adapter(sources):
    path = Path(__file__).with_name('selected_native.py')
    return module('native_http_selection', {'path': str(path), 'sha256': sources['selected_native.py']},
                  sources['selected_native.py'])


def validate_ferric(ferric, sources):
    require(type(ferric) is dict and ferric.get('schema') in ('FerricV17SelectedHttpArmV1',
            'FerricV19SelectedHttpArmV1', 'FerricWidth55cSelectedHttpArmV1', 'FerricGateUpDa6bSelectedHttpArmV1',
            'FerricDownDa6bSelectedHttpArmV1'),
            'closed selected native HTTP arm required')
    adapter = selection_adapter(sources)
    selected = adapter.admit(ferric.get('native_evidence'))
    max_batches, _ = adapter.selected_geometry(selected)
    require(json.dumps(selected, sort_keys=True, allow_nan=False)
            == json.dumps(ferric, sort_keys=True, allow_nan=False), 'native selection differs from complete raw replay')
    for key in ('controller', 'worker'):
        require(digest(ferric[key]['path']) == ferric[key]['sha256'], 'actual selected executable changed')
    argv = ferric['argv']
    for flag, value in (('--source', str(Path(TARGET).parent)), ('--device-unique-id', str(GPU_IDS[0])),
                        ('--worker', ferric['worker']['path']), ('--worker-sha256', ferric['worker']['sha256']),
                        ('--context', '8192'), ('--pages', '512'), ('--max-batches', str(max_batches))):
        require(argv.count(flag) == 1 and argv[argv.index(flag) + 1] == value, 'exact matched native CLI scope')
    return selected


def validate_ferric_closed(events, setup, ferric, sources):
    selection = selection_adapter(sources)
    selection.validate_setup(setup, ferric)
    stage = Path(ferric['native_evidence']['plan']['path']).parent
    native = binding_value(ferric['native_evidence']['plan'], 32 * 1024**2)
    cell = module('native_http_closed_cell', {'path': str(stage / 'measurement/native_token_cell.py'),
        'sha256': native['sources']['measurement/native_token_cell.py']},
        native['sources']['measurement/native_token_cell.py'])
    selection.validate_closed(events, setup, ferric, cell)


def baseline_argv(engine, name, cache, render_device, observer, evidence):
    image = IMAGES[engine]
    argv = ['docker', 'create', '--pull=never', '--name', name, '--label', 'ferric.matched128=' + name,
            '--user', f'{HOST_UID}:{HOST_UID}', '--group-add', str(RENDER_GID),
            '--network', 'host', '--ipc', 'private', '--shm-size', '8g', '--memory', '64g',
            '--memory-swap', '64g', '--pids-limit', '512', '--device', '/dev/kfd', '--device', render_device,
            '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
            '--log-driver', 'local', '--log-opt', 'max-size=4m', '--log-opt', 'max-file=1',
            '--log-opt', 'compress=false',
            '--mount', f'type=bind,src={TARGET},dst=/model,readonly',
            '--mount', f'type=bind,src={cache},dst=/matched-cache', '--tmpfs', '/tmp:rw,nosuid,size=8g',
            '--mount', f'type=bind,src={observer},dst=/matched-observer,readonly',
            '--mount', f'type=bind,src={evidence},dst=/matched-evidence']
    for key, value in {'HIP_VISIBLE_DEVICES': '0', 'ROCR_VISIBLE_DEVICES': '0', 'HF_HUB_OFFLINE': '1',
                       'TRANSFORMERS_OFFLINE': '1', 'TOKENIZERS_PARALLELISM': 'false',
                       'OMP_NUM_THREADS': '8', 'USER': 'ferricmatched', 'LOGNAME': 'ferricmatched',
                       'HOME': '/matched-cache', 'HF_HOME': '/matched-cache/huggingface',
                       'TRITON_CACHE_DIR': '/matched-cache/triton', 'TORCHINDUCTOR_CACHE_DIR': '/matched-cache/inductor',
                       'PYTHONPATH': '/matched-observer',
                       'FERRIC_MATCHED128_OBSERVER_ENGINE': engine,
                       'FERRIC_MATCHED128_OBSERVER_DIR': '/matched-evidence'}.items():
        argv += ['--env', f'{key}={value}']
    if engine == 'vllm':
        return argv + ['--entrypoint', 'vllm', image['digest'], 'serve', '/model', '--host', '127.0.0.1',
            '--port', str(image['port']), '--served-model-name', MODEL, '--dtype', 'bfloat16',
            '--tensor-parallel-size', '1', '--max-model-len', '8192', '--max-num-seqs', '1',
            '--gpu-memory-utilization', '0.25', '--kv-cache-dtype', 'auto', '--no-enable-prefix-caching',
            '--generation-config', 'vllm', '--hf-overrides', '{"head_dtype":"float32"}',
            '--shutdown-timeout', '20']
    argv += ['--env', 'FLYDSL_RUNTIME_CACHE_DIR=/matched-cache/flydsl']
    return argv + ['--entrypoint', '/opt/venv/bin/python3', image['digest'], '-m', 'sglang.launch_server',
        '--model-path', '/model', '--host', '127.0.0.1', '--port', str(image['port']),
        '--served-model-name', MODEL, '--dtype', 'bfloat16', '--tp-size', '1', '--context-length', '8192',
        '--max-running-requests', '1', '--mem-fraction-static', '0.25', '--kv-cache-dtype', 'auto',
        '--disable-radix-cache', '--base-gpu-id', '0', '--enable-fp32-lm-head']


def client_argv(plan, engine, out):
    port = 18980 if engine == 'ferric' else IMAGES[engine]['port']
    return [sys.executable, '-B', plan['client']['path'], '--endpoint', f'http://127.0.0.1:{port}/v1/completions',
            '--engine', engine, '--workload', plan['workload']['path'], '--identity', str(out / 'identity.json'),
            '--output', str(out / 'timed-raw.json'), '--concurrency', '1', '--warmups', '10', '--samples', '30',
            '--timeout', '120', '--arrival', 'closed-loop', '--ttft-slo-ms', '10000', '--tpot-slo-ms', '1000']


def diagnostic_payload(engine, prompt):
    if engine == 'vllm':
        return '/v1/completions', {'model': MODEL, 'prompt': prompt, 'max_tokens': 128, 'temperature': 0,
            'seed': 0, 'ignore_eos': True, 'n': 1, 'stream': False, 'return_token_ids': True}
    require(engine == 'sglang', 'baseline diagnostic engine required')
    return '/generate', {'text': prompt, 'stream': False, 'return_prompt_token_ids': True,
        'sampling_params': {'temperature': 0, 'max_new_tokens': 128, 'ignore_eos': True,
                            'skip_special_tokens': True, 'sampling_seed': 0}}


def numerical_diagnostic(engine, raw, receipt, reference):
    if engine == 'vllm':
        require(len(raw.get('choices', [])) == 1, 'workload invalid: one diagnostic choice required')
        choice = raw['choices'][0]
        prompt, ids, text = choice.get('prompt_token_ids'), choice.get('token_ids'), choice.get('text')
        counts = raw.get('usage', {})
        require(choice.get('finish_reason') == 'length', 'workload invalid: diagnostic length finish required')
    elif engine == 'sglang':
        prompt, ids, text = raw.get('prompt_token_ids'), raw.get('output_ids'), raw.get('text')
        counts = raw.get('meta_info', {})
        require(counts.get('finish_reason', {}).get('type') == 'length', 'workload invalid: diagnostic length finish required')
    else:
        prompt, ids = raw.get('prompt_tokens'), raw.get('generated_tokens')
        text = bytes(raw.get('generated_utf8_bytes', [])).decode('utf-8')
        counts = {'prompt_tokens': len(prompt or []), 'completion_tokens': len(ids or [])}
        require(raw.get('state') == 'Completed', 'workload invalid: Ferric diagnostic incomplete')
    require(prompt == receipt['prompt_token_ids'], 'workload invalid: diagnostic prompt IDs differ')
    require(type(ids) is list and len(ids) == 128 and all(type(i) is int and 0 <= i < 151936 for i in ids)
            and type(text) is str and counts.get('prompt_tokens') == 128
            and counts.get('completion_tokens') == 128, 'workload invalid: exact diagnostic output/usage required')
    if reference is None:
        return {'admitted': False, 'classification': 'independent_reference_missing'}
    return {'admitted': ids == reference['generated_token_ids']
            and text.encode().hex() == reference['generated_utf8_hex'],
            'classification': 'exact_match' if ids == reference['generated_token_ids']
                and text.encode().hex() == reference['generated_utf8_hex'] else 'token_or_utf8_numerical_mismatch',
            'first_token_mismatch': next((i for i, (a, b) in enumerate(zip(ids, reference['generated_token_ids'])) if a != b), None)}


def validate_timing(client, report, plan, engine, reference, identity_sha256):
    require(report.get('schema') == 'FerricCompetitiveStreamingRunV1' and report.get('completed') is True
            and report.get('qualification') is False and report.get('engine') == engine
            and report.get('client_sha256') == CLIENT_SHA
            and report.get('workload_sha256') == plan['workload']['sha256']
            and report.get('identity_sha256') == identity_sha256
            and report.get('ttft_slo_ms') == 10000 and report.get('tpot_slo_ms') == 1000
            and report.get('concurrency') == 1 and report.get('arrival_policy') == 'bounded-closed-loop-windows',
            'timing workload/client scope invalid')
    numerical = reference is not None
    previous = 0
    for phase, count in (('warmups', 10), ('samples', 30)):
        windows = report.get(phase)
        require(type(windows) is list and len(windows) == count, 'missing timed/warmup windows')
        for index, window in enumerate(windows):
            require(window.get('index') == index and len(window.get('requests', [])) == 1, 'window order/cardinality invalid')
            record = window['requests'][0]
            require(previous <= window['started_ns'] <= record['started_ns'] <= record['completed_ns']
                    <= window['completed_ns'], 'window/request clock containment invalid')
            previous = window['completed_ns']
            require(record.get('success') is True and record.get('id') == 'matched-128-v1', 'request failed or workload drifted')
            replay = client.summarize_stream([(json.dumps(chunk['event']).encode(), chunk['received_ns'])
                for chunk in record['chunks']] + [(b'[DONE]', record['completed_ns'])], record['started_ns'], 128)
            require(all(record.get(key) == value for key, value in replay.items()), 'raw timing SSE replay differs')
            require(record['usage'] == {'prompt_tokens': 128, 'completion_tokens': 128, 'total_tokens': 256},
                    'workload invalid: timed 128/128 usage differs')
            expected = client.aggregate([record], window['started_ns'], window['completed_ns'], 10000, 1000)
            require(window.get('metrics') == expected, 'raw timed metrics differ')
            numerical = numerical and record['text'].encode().hex() == reference['generated_utf8_hex']
    return {'timing_admitted': numerical, 'classification': 'exact_utf8_and_usage' if numerical
            else 'independent_reference_missing' if reference is None else 'timed_utf8_numerical_mismatch',
            'qualification': False, 'framework_win_claim': False,
            'window_semantics': 'closed-loop finite cohort including drain, not steady-state',
            'token_itl_available': False, 'token_ids_scope': 'untimed before/after diagnostics only'}


def validate_resolved(value, engine, observer_sha):
    require(value.get('schema') == 'FerricMatched128ResolvedHeadV1' and value.get('engine') == engine
            and value.get('observer_sha256') == observer_sha, 'startup dtype evidence identity invalid')
    resolved = value.get('resolved', {})
    expected = {'model_dtype': 'torch.bfloat16', 'head_dtype': 'torch.float32',
                'head_weight_dtype': 'torch.bfloat16', 'embedding_weight_dtype': 'torch.bfloat16',
                'tensor_parallel_size': 1, 'max_model_len': 8192, 'max_running_requests': 1,
                'prefix_cache': False, 'speculation': False, 'quantization': None, 'kv_cache_dtype': 'auto'}
    require(all(resolved.get(key) == item for key, item in expected.items()),
            'actual resolved dtype/config differs from predeclared FP32-head cell')
    if engine == 'vllm':
        require(resolved.get('shutdown_timeout_seconds') == 20
                and value.get('runner_class') == 'vllm.v1.worker.gpu.model_runner.GPUModelRunner',
                'actual default runner/shutdown grace differs from pinned v3 cell')
    return value
