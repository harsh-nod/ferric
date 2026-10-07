"""Startup-only dtype/config attestation for the two frozen container images.

No forward method or tensor is changed. Missing/failed evidence blocks admission.
"""
import functools
import hashlib
import importlib.abc
import importlib.machinery
import json
import os
from pathlib import Path
import sys
import time

SOURCES = {
    'vllm': ('vllm.v1.worker.gpu.model_runner', 'GPUModelRunner',
             'fb08762e3a025fdc55ab8d2605a96a57d1fad7efae0fbb605cc4fe2973028aca',
             'e15cd9d2827fa3690ec683d73a6095a4be8189118e9a8e124fd21d4c58930ff2'),
    'sglang': ('sglang.srt.model_executor.model_runner', 'ModelRunner',
               'd790cf6e10f314a6f16b847cea16a5fbf38e9111b51902e86c1b0a9c4135dfd0',
               '2bac06a38974a19b7bed79050d98d47ee543f71ab89fcdfcecf656011915a1a9'),
}
AITER_JIT_PACKAGE = '/sgl-workspace/aiter/aiter/jit'
AITER_PRIVATE_JIT = '/matched-cache/.aiter/jit'
AITER_JIT_INIT_SHA = '99591116b9ed472601113e8892f6de59298f7343c2e2334eee4b0eb39d843e09'


def file_sha(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def snapshot(engine, runner):
    model = runner.get_model() if engine == 'vllm' else runner.model
    processor = model.logits_processor
    processor_module = sys.modules[type(processor).__module__]
    if file_sha(processor_module.__file__) != SOURCES[engine][3]:
        raise RuntimeError('frozen logits processor source drifted')
    if engine == 'vllm':
        config = runner.vllm_config
        resolved = {'model_dtype': str(config.model_config.dtype),
                    'head_dtype': str(processor.head_dtype),
                    'tensor_parallel_size': config.parallel_config.tensor_parallel_size,
                    'max_model_len': config.model_config.max_model_len,
                    'max_running_requests': config.scheduler_config.max_num_seqs,
                    'prefix_cache': config.cache_config.enable_prefix_caching,
                    'kv_cache_dtype': config.cache_config.cache_dtype,
                    'speculation': config.speculative_config is not None,
                    'quantization': config.model_config.quantization,
                    'shutdown_timeout_seconds': config.shutdown_timeout}
    else:
        config = runner.server_args
        resolved = {'model_dtype': str(runner.model_config.dtype),
                    'head_dtype': 'torch.float32' if processor.use_fp32_lm_head else 'native',
                    'tensor_parallel_size': config.tp_size,
                    'max_model_len': config.context_length,
                    'max_running_requests': config.max_running_requests,
                    'prefix_cache': not config.disable_radix_cache,
                    'kv_cache_dtype': config.kv_cache_dtype,
                    'speculation': config.speculative_algorithm is not None,
                    'quantization': config.quantization}
    resolved['head_weight_dtype'] = str(model.lm_head.weight.dtype)
    resolved['embedding_weight_dtype'] = str(model.model.embed_tokens.weight.dtype)
    configuration = repr(config)
    if len(configuration.encode()) > 256 * 1024:
        raise RuntimeError('resolved configuration evidence budget exhausted')
    return {'schema': 'FerricMatched128ResolvedHeadV1', 'engine': engine,
            'pid_inside_container': os.getpid(), 'captured_unix_ns': time.time_ns(),
            'scope': 'post-load configuration and resident weight dtypes; no forward instrumentation',
            'model_class': type(model).__module__ + '.' + type(model).__qualname__,
            'runner_class': type(runner).__module__ + '.' + type(runner).__qualname__,
            'runner_source_sha256': SOURCES[engine][2],
            'logits_processor_source_sha256': SOURCES[engine][3],
            'observer_sha256': file_sha(__file__), 'resolved': resolved,
            'effective_configuration_repr': configuration}


class ObserveLoader(importlib.abc.Loader):
    def __init__(self, inner, engine):
        self.inner, self.engine = inner, engine

    def create_module(self, spec):
        return self.inner.create_module(spec)

    def exec_module(self, module):
        self.inner.exec_module(module)
        if file_sha(module.__file__) != SOURCES[self.engine][2]:
            raise RuntimeError('frozen model runner source drifted')
        cls = getattr(module, SOURCES[self.engine][1])
        original = cls.load_model
        engine = self.engine

        @functools.wraps(original)
        def observed(runner, *args, **kwargs):
            result = original(runner, *args, **kwargs)
            value = snapshot(engine, runner)
            path = Path(os.environ['FERRIC_MATCHED128_OBSERVER_DIR']) / (engine + '-' + str(os.getpid()) + '.json')
            with path.open('x', encoding='utf-8') as output:
                json.dump(value, output, sort_keys=True, allow_nan=False)
                output.write('\n')
            return result
        cls.load_model = observed


class ObserveFinder(importlib.abc.MetaPathFinder):
    def __init__(self, engine):
        self.engine = engine

    def find_spec(self, fullname, path=None, target=None):
        if self.engine == 'sglang' and fullname == 'aiter.jit':
            spec = importlib.machinery.PathFinder.find_spec(fullname, path, target)
            if (spec is None or spec.loader is None
                    or spec.origin != AITER_JIT_PACKAGE + '/__init__.py'
                    or list(spec.submodule_search_locations or ()) != [AITER_JIT_PACKAGE]
                    or file_sha(spec.origin) != AITER_JIT_INIT_SHA):
                raise ImportError('frozen AITER JIT package identity drifted')
            private = Path(AITER_PRIVATE_JIT)
            if (private.resolve(strict=True) != private or not private.is_dir()
                    or private.stat().st_uid != os.getuid() or private.stat().st_mode & 0o777 != 0o700
                    or 'AITER_JIT_DIR' in os.environ):
                raise ImportError('owned AITER JIT fallback directory invalid')
            # Preserve packaged modules first; include only this run's newly built extensions.
            spec.submodule_search_locations.append(str(private))
            return spec
        if fullname != SOURCES[self.engine][0]:
            return None
        spec = importlib.machinery.PathFinder.find_spec(fullname, path, target)
        if spec is None or spec.loader is None:
            raise ImportError('frozen model runner not found')
        spec.loader = ObserveLoader(spec.loader, self.engine)
        return spec


engine = os.environ.get('FERRIC_MATCHED128_OBSERVER_ENGINE')
if engine is not None:
    if engine not in SOURCES or not os.environ.get('FERRIC_MATCHED128_OBSERVER_DIR'):
        raise RuntimeError('invalid explicit matched-cell observer configuration')
    sys.meta_path.insert(0, ObserveFinder(engine))
