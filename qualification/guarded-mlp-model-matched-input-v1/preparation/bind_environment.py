#!/usr/bin/env python3
"""Bind the observed container implementation and the selected gfx950 topology."""
import hashlib
import json
import os
from pathlib import Path

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ENV = E / 'guarded-mlp-matched-framework-env-v228-v2'
ROOT = E / 'guarded-mlp-model-matched-input-mlp-v228-v1'
assert os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
assert int(Path('/sys/class/drm/renderD128/device/unique_id').read_text().strip(), 16) == 16366993098680759275
probe_raw = (ENV / 'probe.json').read_bytes()
probe = json.loads(probe_raw)
assert probe['gpu_executed'] is False and probe['python'].startswith('3.12.13 ')
assert probe['image'] == 'sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba'
nodes = []
for node in Path('/sys/class/kfd/kfd/topology/nodes').iterdir():
    properties = dict(line.split(maxsplit=1) for line in (node / 'properties').read_text().splitlines())
    if properties.get('unique_id') == '16366993098680759275':
        assert properties['gfx_target_version'] == '90500'
        nodes.append(str(node))
assert len(nodes) == 1
mapping = {'qwen3': 'modeling_qwen3', 'activation': 'activation',
           'sdpa': 'sdpa', 'functional': 'torch_functional'}
environment = dict(schema='ferric-guarded-mlp-matched-input-environment-v1',
    image=probe['image'], model_root='/model', framework_gpu_execution=False,
    python_version=[3, 12, 13], python_executable=probe['executable'],
    packages=probe['packages'], hip=probe['torch_hip'], expected_arch='gfx950',
    implementation_sources={mapping[key]: value for key, value in probe['installed_sources'].items()},
    observation=dict(cpu_probe_sha256=hashlib.sha256(probe_raw).hexdigest(),
                     cpu_probe_path=str(ENV / 'probe.json'), kfd_node=nodes[0],
                     gpu_unique_id='16366993098680759275',
                     boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                     torch_device_feature_string_observed=False))
raw = (json.dumps(environment, sort_keys=True, indent=2) + '\n').encode()
with (ROOT / 'inputs/environment.json').open('xb') as output:
    output.write(raw)
package_files = json.loads((ENV / 'package-files.json').read_bytes())
assert all(name.startswith('packages/') for name in package_files)
overlay_manifest = dict(files={name.removeprefix('packages/'): value
                               for name, value in package_files.items()},
                        provision_manifest=str(ENV / 'package-files.json'))
with (ENV / 'overlay-manifest.json').open('x') as output:
    json.dump(overlay_manifest, output, indent=2, sort_keys=True)
print(json.dumps(dict(path=str(ROOT / 'inputs/environment.json'), bytes=len(raw),
                      sha256=hashlib.sha256(raw).hexdigest(), gpu_executed=False)))
