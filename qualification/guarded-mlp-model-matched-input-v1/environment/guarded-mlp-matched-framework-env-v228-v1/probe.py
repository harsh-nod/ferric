#!/usr/bin/env python3
"""CPU-only import/source observation of the private container overlay."""
import hashlib
import importlib.metadata
import inspect
import json
from pathlib import Path
import sys

sys.path.insert(0, '/packages')
import torch
import transformers
from transformers.models.qwen3 import modeling_qwen3
from transformers.integrations import sdpa_attention

packages = ('torch', 'transformers', 'tokenizers', 'huggingface-hub',
            'numpy', 'safetensors', 'psutil', 'accelerate', 'triton')
sources = {}
for name, module in [('qwen3', modeling_qwen3), ('sdpa', sdpa_attention),
                     ('activation', torch.nn.modules.activation),
                     ('functional', torch.nn.functional)]:
    path = Path(inspect.getfile(module))
    raw = path.read_bytes()
    sources[name] = dict(path=str(path), bytes=len(raw),
                         sha256=hashlib.sha256(raw).hexdigest())
result = dict(python=sys.version, executable=sys.executable, prefix=sys.prefix,
              packages={name: importlib.metadata.version(name) for name in packages},
              torch_hip=torch.version.hip, torch_git=torch.version.git_version,
              installed_sources=sources, gpu_executed=False,
              image='sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba')
with Path('/output/probe.json').open('x') as stream:
    json.dump(result, stream, sort_keys=True, indent=2)
    stream.write('\n')
print(json.dumps(result, sort_keys=True))
