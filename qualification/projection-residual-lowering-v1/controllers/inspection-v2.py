"""Three bounded, CPU-only inspections of one checked projection-residual image."""
import ctypes
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
N = R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu'
BOUND = R / 'evidence/wave-output-lowering-v216/bounded.py'
BOUND_SHA = 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1'
LOWERING_SHA = '7d8ce691badbc6bd8601979fc22a43d9e85d946c3bd61e87a17ae5064ee3ec0f'
GENERATION = E / 'ordinary-induction-compiler-cpu-v228-v1/target/debug'
KERNEL = 'ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1'
CRATE = 'ferric_qwen3_tp_projection_residual_kernels_device_v1'
TOOLS = {
    'descriptor-metadata': dict(path=str(GENERATION / 'examples/finite_join_request_metadata_v1'),
        bytes=23544336, sha256='3c024e79a263a286dc6ab46c5cd3c69e1173c46ba61d1eb415ecde903f74cdb8'),
    'elf-notes': dict(path='/opt/rocm-7.3.0/lib/llvm/bin/llvm-readobj', bytes=12210352,
        sha256='8cb6079bc349196a3f3af7589912ca48f716194970ccd0e6c6a36b09e0b21259'),
    'disassembly': dict(path='/opt/rocm-7.3.0/lib/llvm/bin/llvm-objdump', bytes=29241992,
        sha256='be832bf5ddf82b3feb8a6e24f453e2602bd01a406f9f5bde47b225c4989d1a83'),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, 'duplicate mapping key')
        value[key] = item
    return value


def pin(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    before = path.stat()
    require(stat.S_ISREG(before.st_mode), 'ordinary input file')
    def identity(item):
        return (item.st_dev, item.st_ino, item.st_mode, item.st_uid, item.st_nlink,
            item.st_size, item.st_mtime_ns, item.st_ctime_ns)
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        require(identity(os.fstat(stream.fileno())) == identity(before), 'opened input identity')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(identity(os.fstat(stream.fileno())) == identity(before), 'stable opened input')
    require(identity(path.stat()) == identity(before), 'file changed during hashing')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def checked(ledger, item):
    require(set(item) == {'path', 'bytes', 'sha256'} and pin(Path(item['path'])) == item,
        'exact retained FilePin')
    require(item['path'] not in ledger or ledger[item['path']] == item, 'conflicting input pin')
    ledger[item['path']] = item
    return item


def document(ledger, item):
    checked(ledger, item)
    require(item['bytes'] <= 16 << 20, 'bounded JSON input')
    data = Path(item['path']).read_bytes()
    require(len(data) == item['bytes'] and hashlib.sha256(data).hexdigest() == item['sha256'],
        'unchanged authenticated JSON')
    return json.loads(data, object_pairs_hook=pairs)


def source_map(root):
    value = {}
    for path in sorted(root.rglob('*')):
        require(not path.is_symlink(), 'source symlink refused')
        if path.is_file():
            item = pin(path)
            value[str(path.relative_to(root))] = {key: item[key] for key in ('bytes', 'sha256')}
            require(len(value) <= 128, 'bounded source roster')
    return value


def lowering(ledger, path, digest):
    require(path.name == 'complete.json' and path.parent.parent == E
        and re.fullmatch(r'projection-residual-lowering-v228-v[1-9][0-9]*', path.parent.name),
        'actual checked lowering namespace')
    actual = pin(path)
    require(actual['sha256'] == digest, 'caller-selected actual lowering completion')
    value = document(ledger, actual)
    require(value['schema'] == 'ferric-p228-projection-residual-lowering-result-v1'
        and value['passed'] is True and value['errors'] == [] and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['natural_exit_code'] == 0
        and value['exit_code'] == 0 and value['natural_group_absent_before_cleanup'] is True
        and value['owned_group_empty'] is True and value['automatic_retries'] == 0
        and value['controller']['sha256'] == LOWERING_SHA
        and all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance',
            'production_authority', 'performance_claim', 'load_authority', 'launch_authority')),
        'successful checked engineering emission, no execution authority')
    checked(ledger, value['controller'])
    for name, item in value['raw'].items():
        require(Path(item['path']) == path.parent / name and '/' not in name, 'local lowering raw leaf')
        checked(ledger, item)
    inputs = document(ledger, value['raw']['inputs.json'])
    for name, item in inputs.items():
        require(name == item['path'], 'original lowering input-map key')
        checked(ledger, item)
    owned = document(ledger, value['raw']['owned-process.json'])
    audit = document(ledger, value['raw']['owned-process-audit.json'])
    require(type(owned['pid']) is int and owned['pid'] > 1 and owned['pgid'] == owned['pid']
        and audit['empty'] is True, 'retained completed compiler ownership')
    root = Path(value['source_root'])
    expected_sources = document(ledger, value['raw']['sources-before.json'])
    require(source_map(root) == expected_sources, 'exact qualified formatted source')
    for name, item in expected_sources.items():
        checked(ledger, dict(path=str(root / name), **item))
    artifact = value['artifact']
    image = checked(ledger, artifact['image'])
    observation = document(ledger, artifact['observation'])
    folder = Path(image['path']).parent
    require(Path(image['path']).name == 'observation.hsaco'
        and Path(artifact['observation']['path']) == folder / 'observation.json'
        and folder.parent == path.parent / 'fe2o3-engineering-v1'
        and re.fullmatch(r'[0-9a-f]{64}', folder.name)
        and observation == artifact['value'] and observation['schema'] == 'EngineeringHsacoObservationV1'
        and observation['authority'] == 'none' and observation['namespace'] == 'fe2o3-engineering-v1'
        and observation['crate_name'] == CRATE and observation['target'] == 'gfx950:xnack-'
        and observation['code_object_version'] == 6 and observation['providers'] == []
        and observation['grants'] == dict(publication=False, load=False, launch=False)
        and observation['execution']['exact_output_replay'] is True
        and observation['hsaco']['kernel_names'] == [KERNEL]
        and observation['hsaco']['identity'] == dict(sha256=image['sha256'], byte_len=image['bytes'])
        and 0 < image['bytes'] <= 64 << 20, 'actual sole-kernel checked image and observation')
    metadata_bytes = Path(artifact['observation']['path']).read_bytes()
    image_bytes = Path(image['path']).read_bytes()
    content = hashlib.sha256(b'FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0'
        + len(metadata_bytes).to_bytes(8, 'little') + metadata_bytes
        + len(image_bytes).to_bytes(8, 'little') + image_bytes).hexdigest()
    require(content == folder.name, 'original content-addressed image generation')
    return actual, value, root, expected_sources, image, observation


def yaml_reader(ledger):
    spec = importlib.util.find_spec('yaml')
    require(spec is not None and spec.origin is not None, 'existing installed PyYAML required')
    root = Path(spec.origin).resolve(strict=True).parent
    files = sorted(p for p in root.rglob('*') if p.suffix in ('.py', '.so')
        and '__pycache__' not in p.parts)
    require(0 < len(files) <= 128, 'bounded installed YAML parser source closure')
    for path in files:
        checked(ledger, pin(path))
    import yaml
    require(Path(yaml.__file__).resolve(strict=True) == Path(spec.origin).resolve(strict=True),
        'selected installed YAML parser')
    return yaml, root, [str(p) for p in files]


def validate_outputs(out, image, observation, yaml):
    lines = (out / 'descriptor-metadata-stdout').read_text().splitlines()
    require(lines and lines[0] == 'fe2o3-finite-join-request-metadata-v1', 'metadata record header')
    metadata = pairs(line.split(' ', 1) for line in lines[1:])
    required = dict(authority='none', object_sha256=image['sha256'], object_bytes=str(image['bytes']),
        entry_symbol_hex=KERNEL.encode().hex(), descriptor_symbol_hex=(KERNEL + '.kd').encode().hex(),
        target='gfx950:xnack-', code_object_version='6', explicit_argument_bytes='168',
        kernarg_segment_bytes='424', kernarg_alignment='8', workgroup='64,1,1', max_grid_workgroups='64,1,1',
        canonical_code_object_digest=observation['hsaco']['canonical_descriptor_sha256'])
    require(set(metadata) == set(required) | {'descriptor_sha256'}
        and all(metadata[key] == item for key, item in required.items())
        and re.fullmatch(r'[0-9a-f]{64}', metadata['descriptor_sha256']), 'checked finalized ABI/launch metadata')
    notes = (out / 'elf-notes-stdout').read_text()
    require(notes.count('AMDGPU Metadata: ') == 1, 'one actual AMDGPU metadata document')
    text = notes.split('AMDGPU Metadata: ', 1)[1]
    require('\n...' in text, 'terminated AMDGPU metadata document')
    class Loader(yaml.SafeLoader):
        pass
    def mapping(loader, node):
        loader.flatten_mapping(node)
        return pairs((loader.construct_object(key), loader.construct_object(item))
            for key, item in node.value)
    Loader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, mapping)
    doc = yaml.load(text.split('\n...', 1)[0], Loader=Loader)
    kernels = doc['amdhsa.kernels']
    require(doc['amdhsa.target'] == 'amdgcn-amd-amdhsa--gfx950:xnack-'
        and isinstance(kernels, list) and len(kernels) == 1, 'sole gfx950 metadata kernel')
    kernel = kernels[0]
    expected = {'.name': KERNEL, '.symbol': KERNEL + '.kd', '.wavefront_size': 64,
        '.reqd_workgroup_size': [64, 1, 1], '.max_flat_workgroup_size': 64,
        '.kernarg_segment_size': 424, '.kernarg_segment_align': 8,
        '.group_segment_fixed_size': 0, '.private_segment_fixed_size': 0, '.uses_dynamic_stack': False}
    require(all(kernel.get(key) == item for key, item in expected.items()), 'actual wave/launch/storage ABI')
    explicit = [item for item in kernel['.args'] if not item['.value_kind'].startswith('hidden_')]
    hidden = [item for item in kernel['.args'] if item['.value_kind'].startswith('hidden_')]
    require(len(explicit) == 22, 'twenty-two explicit arguments')
    for index, item in enumerate(explicit):
        pointer = index < 20 and index % 2 == 0
        offset = index * 8 if index < 20 else 160 + (index - 20) * 4
        require(item['.offset'] == offset and item['.size'] == (8 if index < 20 else 4)
            and item['.value_kind'] == ('global_buffer' if pointer else 'by_value')
            and (not pointer or item.get('.address_space') == 'global'), 'exact explicit argument layout')
    layout = [('block_count_' + axis, 168 + index * 4, 4) for index, axis in enumerate('xyz')]
    layout += [('group_size_' + axis, 180 + index * 2, 2) for index, axis in enumerate('xyz')]
    layout += [('remainder_' + axis, 186 + index * 2, 2) for index, axis in enumerate('xyz')]
    layout += [('global_offset_' + axis, 208 + index * 8, 8) for index, axis in enumerate('xyz')]
    layout += [('grid_dims', 232, 2)]
    require([(item['.value_kind'], item['.offset'], item['.size']) for item in hidden]
        == [('hidden_' + name, offset, size) for name, offset, size in layout],
        'unchanged hidden-argument schema at explicit offset168 within reserved256 bytes')
    disassembly = (out / 'disassembly-stdout').read_text()
    require(re.search(r'^0+[0-9a-f]* <' + re.escape(KERNEL) + r'>:$', disassembly, re.MULTILINE),
        'actual sole entry is present in retained disassembly')
    return dict(descriptor=metadata, elf_kernel=kernel,
        arithmetic_isa_review_performed=False, instruction_count_claim=False)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
        and sys.dont_write_bytecode and len(sys.argv) == 4,
        'usage: python3 -B run.py LOWERING_COMPLETE LOWERING_SHA projection-residual-inspection-v228-vN')
    path, digest, label = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    require(re.fullmatch(r'[0-9a-f]{64}', digest)
        and re.fullmatch(r'projection-residual-inspection-v228-v[1-9][0-9]*', label), 'closed invocation')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'same task-owned two-core inspection host')
    require(not any(key in os.environ for key in ('PYTHONPATH', 'PYTHONHOME')),
        'no injected Python paths')
    require(shutil.disk_usage(R).free >= 40 << 30, '40 GiB setup floor')
    ledger = {}
    checked(ledger, pin(Path(__file__).resolve(strict=True)))
    bound = pin(BOUND)
    require(bound['sha256'] == BOUND_SHA, 'exact bounded command helper')
    checked(ledger, bound)
    b = types.ModuleType('projection_residual_inspection_bound')
    b.__file__ = str(BOUND)
    exec(compile(BOUND.read_bytes(), str(BOUND), 'exec'), b.__dict__)
    receipt, value, source, before, image, observation = lowering(ledger, path, digest)
    for item in TOOLS.values():
        checked(ledger, item)
    yaml, yaml_root, yaml_files = yaml_reader(ledger)
    out = E / label
    require(not os.path.lexists(out), 'fresh exclusive inspection output')
    os.umask(0o077)
    out.mkdir(mode=0o700)
    scratch = out / 'scratch'
    scratch.mkdir(mode=0o700)
    b.F, b.T = out, scratch
    b.PINS = {name: item['sha256'] for name, item in TOOLS.items()}
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    env = dict(PATH='/usr/bin:/bin', HOME=str(out), LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        TMPDIR=str(scratch), LD_LIBRARY_PATH=str(GENERATION / 'deps') + ':' + str(N / 'lib'),
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    commands = {
        'descriptor-metadata': [TOOLS['descriptor-metadata']['path'], image['path'], image['sha256'], str(image['bytes'])],
        'elf-notes': [TOOLS['elf-notes']['path'], '--notes', '--symbols', image['path']],
        'disassembly': [TOOLS['disassembly']['path'], '--disassemble', '--mcpu=gfx950', image['path']],
    }
    b.save(out / 'inputs.json', ledger)
    b.save(out / 'sources-before.json', before)
    phases, error, post_errors, inspected = {}, None, [], None
    try:
        for name, argv in commands.items():
            require(shutil.disk_usage(R).free >= 40 << 30, '40 GiB inspection launch floor')
            phases[name] = b.run(out, name, argv, env=env, deadline=120, gpu_execution=False)
        inspected = validate_outputs(out, image, observation, yaml)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    def source_postcheck():
        after = source_map(source)
        b.save(out / 'sources-after.json', after)
        require(after == before, 'qualified source drift')
    def input_postcheck():
        after = {name: pin(Path(name)) for name in ledger}
        b.save(out / 'inputs-after.json', after)
        require(after == ledger, 'original source/tool/image/input drift')
    for name, check in (
        ('sources', source_postcheck),
        ('inputs', input_postcheck),
        ('yaml-roster', lambda: require(sorted(str(p) for p in yaml_root.rglob('*')
            if p.suffix in ('.py', '.so') and '__pycache__' not in p.parts) == yaml_files,
            'YAML source roster drift')),
    ):
        try:
            check()
        except BaseException as failure:
            post_errors.append(name + ': ' + type(failure).__name__ + ': ' + str(failure))
    passed = error is None and not post_errors and len(phases) == 3 and inspected is not None
    result = dict(schema='ferric-p228-projection-residual-inspection-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, lowering_complete=receipt, cpu_complete=value['cpu_complete'],
        controller=ledger[str(Path(__file__).resolve())], image=image, observation=value['artifact']['observation'],
        phases=phases, inspection=inspected, source_unchanged=not post_errors,
        parser=dict(name='PyYAML.SafeLoader', version=yaml.__version__, root=str(yaml_root), sources=yaml_files),
        gpu_execution=False, numerical_acceptance=False, production_authority=False, performance_claim=False,
        load_authority=False, launch_authority=False, arithmetic_isa_review_performed=False,
        automatic_retries=0, raw={p.name: pin(p) for p in out.iterdir() if p.is_file()})
    b.save(out / ('complete.json' if passed else 'failed.json'), result)
    print(json.dumps(dict(passed=passed, output=str(out), error=error, postcheck_errors=post_errors)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
