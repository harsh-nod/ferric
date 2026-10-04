"""Local retained-byte publication only; never imports a compiler or inspector."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
PL = L / 'proposals'
F = Path('/home/harsh/ferric-p227-integration')
R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
H = Path('/home/harsh/ferric-session-evidence/20260920/paired-mfma/device-evidence-v1')
Q = F / 'qualification/projection-residual-lowering-v1'
CPU_Q = F / 'qualification/projection-residual-cpu-v1'
LOW = 'projection-residual-lowering-v228-v1'
CPU = 'projection-residual-cpu-v228-v1'
DIRECTORY = 'qwen3-tp-projection-residual-kernels-v1'
CRATE = 'ferric_qwen3_tp_projection_residual_kernels_device_v1'
KERNEL = 'ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1'
LOW_SHA = '4f46acb4eaaadefc2c1426146424bda91a7e455ab3b57b23c935401e9c4d79bb'
CPU_SHA = '2c29833be2202402f40b0255f54e717d5a8be1168a4d55d84c041d7d1820b9df'
CPU_PUBLIC_SHA = '637bb536b6920ed0cd3d7529ae44b9261ac2f6b0b4d86da10ef98ddcf7d87fbe'
OLD_INSPECT_SHA = 'cfa62d8fd326918a2589711e80d59716da14e4953f39bee1134a9f6f14353cfb'
CONTROLLERS = {
    'lowering': ('p228-projection-residual-lowering-v1', '7d8ce691badbc6bd8601979fc22a43d9e85d946c3bd61e87a17ae5064ee3ec0f'),
    'inspection-v1': ('p228-projection-residual-inspection-v1', '24da43d44e544b9dcd3150a36ba0a8f834babe1b2065b5b635af394ac987af65'),
    'inspection-v2': ('p228-projection-residual-inspection-v2', '60a14b701d6476cf609f2c2f4bc6401eb63a8e3fec17c547b167d19ddd56b54c'),
}
TOOLS = {
    'descriptor-metadata': (str(E / 'ordinary-induction-compiler-cpu-v228-v1/target/debug/examples/finite_join_request_metadata_v1'), 23544336, '3c024e79a263a286dc6ab46c5cd3c69e1173c46ba61d1eb415ecde903f74cdb8'),
    'elf-notes': ('/opt/rocm-7.3.0/lib/llvm/bin/llvm-readobj', 12210352, '8cb6079bc349196a3f3af7589912ca48f716194970ccd0e6c6a36b09e0b21259'),
    'disassembly': ('/opt/rocm-7.3.0/lib/llvm/bin/llvm-objdump', 29241992, 'be832bf5ddf82b3feb8a6e24f453e2602bd01a406f9f5bde47b225c4989d1a83'),
}
FALSE = ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority', 'load_authority', 'launch_authority')
READS, COPIES, OMITTED = {}, {}, {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    result = {}
    for key, value in items:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def local(path):
    p = Path(path)
    if p.is_relative_to(E):
        relative = p.relative_to(E)
        return (PL if relative.parts[0].startswith('p228-') else L) / relative
    require(p.is_relative_to(L) or p.is_relative_to(F) or p.is_relative_to(H), 'closed retained local path')
    return p


def pin_shape(item):
    require(set(item) == {'path', 'bytes', 'sha256'} and Path(item['path']).is_absolute()
        and type(item['bytes']) is int and 0 <= item['bytes'] <= 16 << 20
        and re.fullmatch(r'[0-9a-f]{64}', item['sha256']), 'bounded FilePin')


def read(item):
    pin_shape(item)
    path = local(item['path'])
    require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink(), 'canonical retained file')
    require(path.stat().st_size == item['bytes'], 'retained extent: ' + str(path))
    body = path.read_bytes()
    require(len(body) == item['bytes'] and hashlib.sha256(body).hexdigest() == item['sha256'], 'retained hash: ' + str(path))
    key = str(path)
    require(key not in READS or READS[key]['sha256'] == item['sha256'], 'conflicting retained pin')
    READS[key] = dict(path=key, bytes=len(body), sha256=item['sha256'])
    return body


def actual(path, digest=None):
    path = Path(path)
    require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink()
        and path.stat().st_size <= 16 << 20, 'bounded local input')
    body = path.read_bytes()
    value = dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
    require(digest is None or value['sha256'] == digest, 'selected actual digest')
    read(value)
    return value


def doc(item):
    return json.loads(read(item), object_pairs_hook=pairs)


def copy(item, destination):
    body = read(item)
    path = Path(destination)
    require(not path.is_absolute() and '..' not in path.parts and str(path) not in COPIES
        and len(body) <= 1 << 20 and not body.startswith(b'\x7fELF'), 'small unique textual publication')
    body.decode('utf-8')
    COPIES[str(path)] = (item, body)


def raw(value, directory, public):
    for name, item in value['raw'].items():
        require(Path(item['path']) == E / directory / name and Path(name).name == name, 'exact raw output path')
        read(item)
        if name in ('inputs.json', 'inputs-after.json', 'targets-before.json'):
            OMITTED[public + '/' + name] = item
        else:
            copy(item, public + '/' + name)


def original_map(item):
    value = doc(item)
    require(isinstance(value, dict) and 0 < len(value) <= 30000, 'bounded original input roster')
    for name, entry in value.items():
        require(set(entry) == {'path', 'bytes', 'sha256'} and entry['path'] == name
            and type(entry['bytes']) is int and entry['bytes'] >= 0
            and re.fullmatch(r'[0-9a-f]{64}', entry['sha256']), 'original input identity')
    return value


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and len(sys.argv) == 2
        and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'usage: publish.py ACTUAL_INSPECTION_V2_COMPLETE_SHA')
    require(not Q.is_symlink() and (not Q.exists() or (Q.is_dir()
        and {p.name for p in Q.iterdir()} <= {'README.md'})), 'fresh public destination, optional root README only')
    readme = actual(Q / 'README.md') if (Q / 'README.md').exists() else None
    self_pin = actual(Path(__file__).resolve(strict=True))
    copy(self_pin, 'controllers/publish.py')
    controllers = {}
    for name, (directory, digest) in CONTROLLERS.items():
        item = actual(PL / directory / 'run.py', digest)
        controllers[name] = dict(item, path=str(E / directory / 'run.py'))
        copy(controllers[name], 'controllers/' + name + '.py')
    old_source = read(controllers['inspection-v1']).decode()
    new_source = read(controllers['inspection-v2']).decode()
    wrong, right = 'amdgpu-amd-amdhsa-unknown-gfx950:xnack-', 'amdgcn-amd-amdhsa--gfx950:xnack-'
    require(old_source.count(wrong) == 1 and old_source.replace(wrong, right) == new_source,
        'inspection V2 changes only actual LLVM target spelling')

    lower_pin = dict(actual(L / LOW / 'complete.json', LOW_SHA), path=str(E / LOW / 'complete.json'))
    lower = doc(lower_pin)
    require(lower['schema'] == 'ferric-p228-projection-residual-lowering-result-v1'
        and lower['passed'] is True and lower['errors'] == lower['postcheck_errors'] == []
        and lower['source_unchanged'] is True and lower['natural_exit_code'] == lower['exit_code'] == 0
        and lower['natural_group_absent_before_cleanup'] is True and lower['owned_group_empty'] is True
        and lower['automatic_retries'] == 0 and lower['controller'] == controllers['lowering']
        and lower['retained_handoff_or_llvm'] is False and all(lower[k] is False for k in FALSE), 'actual closed generic lowering')
    require(set(lower['raw']) == {'command.json', 'compile.stdout', 'compile.stderr', 'configurations.json',
        'inputs.json', 'owned-process.json', 'owned-process-audit.json', 'sources-before.json', 'targets-before.json'}, 'nine lowering raw records')
    copy(lower_pin, 'lowering/complete.json')
    raw(lower, LOW, 'lowering')
    lower_inputs = original_map(lower['raw']['inputs.json'])
    require(lower_inputs[lower['controller']['path']] == lower['controller'], 'executed lowering source')
    owned, audit = (doc(lower['raw'][name]) for name in ('owned-process.json', 'owned-process-audit.json'))
    require(type(owned['pid']) is int and owned['pid'] > 1 and owned['pid'] == owned['pgid'] == audit['pgid']
        and audit['empty'] is True and audit['group_exists_before'] is False and audit['group_exists_after'] is False
        and audit['processes'] == dict(matches=[], unknown=[]), 'natural empty compiler tree before and after cleanup')
    require(all(v is None for v in doc(lower['raw']['configurations.json']).values()), 'no injected Cargo configuration')
    for stream in ('stdout', 'stderr'):
        require(lower['stream_bytes'][stream] == lower['raw']['compile.' + stream]['bytes'], 'compiler stream extent')

    cpu = doc(lower['cpu_complete'])
    require(lower['cpu_complete']['sha256'] == CPU_SHA and cpu['passed'] is True
        and cpu['schema'] == 'ferric-p228-projection-residual-cpu-result-v1' and cpu['test_count'] == 31
        and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
        and lower['source_root'] == cpu['formatted_source_root'], 'actual prerequisite CPU generation')
    sources = doc(lower['raw']['sources-before.json'])
    require(sources == doc(cpu['raw']['sources-after.json']) == doc(cpu['raw']['sources-before.json'])
        and len(sources) == 17, 'same complete formatted source generation')
    for name, item in sources.items():
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'source relative member')
        read(dict(path=str(Path(lower['source_root']) / name), **item))
    public_cpu_pin = actual(CPU_Q / 'result.json', CPU_PUBLIC_SHA)
    public_cpu = doc(public_cpu_pin)
    require(public_cpu['passed'] is True and public_cpu['original_cpu_complete'] == lower['cpu_complete'], 'published CPU prerequisite')
    selected_sources = {name.removeprefix(DIRECTORY + '/'): value for name, value in sources.items() if name.startswith(DIRECTORY + '/')}
    require(set(selected_sources) == {'Cargo.toml', 'Cargo.lock', 'build.rs', 'src/lib.rs', 'src/collective.rs', 'tests/contract.rs'}, 'exact six candidate files')
    for name, item in selected_sources.items():
        require(public_cpu['file_ledger']['source/' + name] == item, 'original CPU source publication')
        for path in (CPU_Q / 'source' / name, F / 'device' / DIRECTORY / name):
            read(dict(path=str(path), **item))

    old_command_pin = actual(H / 'device-control-paired-on-v1/command.json', '84b76a8781ddc545b1271faaba5803df79a5fe365221f1f579894b2dc4e14ab9')
    old_command = doc(old_command_pin)
    command = doc(lower['raw']['command.json'])
    argv = old_command['argv'][:]
    for flag, value in (('--crate', CRATE), ('--output-root', str(E / LOW / 'fe2o3-engineering-v1')),
        ('--manifest-path', str(Path(lower['source_root']) / DIRECTORY / 'Cargo.toml')), ('--features', 'gfx950')):
        require(argv.count(flag) == 1, 'closed generic argument substitution')
        argv[argv.index(flag) + 1] = value
    expected = dict(argv=argv, cwd=str(Path(lower['source_root']) / DIRECTORY),
        environment=dict(old_command['environment'], HOME=str(E / LOW), TMPDIR=str(E / LOW / 'scratch')),
        affinity=[8, 9], nice=10, deadline_seconds=600, cleanup_seconds=5,
        address_space_bytes=12 << 30, scratch_cap_bytes=6 << 30, stream_cap_bytes=64 << 20,
        gpu_execution=False, explicit_inner_cargo_job_limit=False)
    require(command == expected, 'exact old checked pipeline with only four candidate substitutions')
    for index, flag in enumerate(argv):
        if flag.endswith('-sha256'):
            tool = argv[argv.index(flag.removesuffix('-sha256')) + 1]
            require(lower_inputs[tool]['sha256'] == argv[index + 1], 'actual compiler tool identity')
    copy(old_command_pin, 'lowering/historical-command.json')

    artifact = lower['artifact']
    image = artifact['image']
    require(image['bytes'] == 10864 and image['sha256'] == '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25', 'actual emitted image')
    image_body = read(image)
    observation_body = read(artifact['observation'])
    observation = doc(artifact['observation'])
    require(observation == artifact['value'] and observation['authority'] == 'none'
        and observation['schema'] == 'EngineeringHsacoObservationV1' and observation['crate_name'] == CRATE
        and observation['target'] == 'gfx950:xnack-' and observation['code_object_version'] == 6
        and observation['execution']['exact_output_replay'] is True and observation['providers'] == []
        and observation['grants'] == dict(publication=False, load=False, launch=False)
        and observation['hsaco']['kernel_names'] == [KERNEL]
        and observation['hsaco']['identity'] == dict(byte_len=image['bytes'], sha256=image['sha256']), 'authority-free generic observation')
    folder = Path(image['path']).parent
    content = hashlib.sha256(b'FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0'
        + len(observation_body).to_bytes(8, 'little') + observation_body
        + len(image_body).to_bytes(8, 'little') + image_body).hexdigest()
    require(folder == E / LOW / 'fe2o3-engineering-v1' / content
        and Path(image['path']).name == 'observation.hsaco'
        and Path(artifact['observation']['path']) == folder / 'observation.json'
        and read(lower['raw']['compile.stdout']).decode().splitlines() == [str(folder)], 'content addressed sole image output')
    copy(artifact['observation'], 'lowering/observation.json')
    OMITTED['image'] = image

    inspections = {}
    for version, filename, digest in ((1, 'failed.json', OLD_INSPECT_SHA), (2, 'complete.json', sys.argv[1])):
        label = 'projection-residual-inspection-v228-v' + str(version)
        record_pin = dict(actual(L / label / filename, digest), path=str(E / label / filename))
        record = doc(record_pin)
        public = 'history/inspection-v1' if version == 1 else 'inspection'
        copy(record_pin, public + '/' + filename)
        require(record['schema'] == 'ferric-p228-projection-residual-inspection-result-v1'
            and record['controller'] == controllers['inspection-v' + str(version)]
            and record['lowering_complete'] == lower_pin and record['cpu_complete'] == lower['cpu_complete']
            and record['image'] == image and record['observation'] == artifact['observation']
            and record['postcheck_errors'] == [] and record['source_unchanged'] is True
            and record['automatic_retries'] == 0 and record['arithmetic_isa_review_performed'] is False
            and all(record[k] is False for k in FALSE), 'independent bounded inspection joins')
        require(record['passed'] is (version == 2)
            and record['error'] == (None if version == 2 else 'RuntimeError: sole gfx950 metadata kernel')
            and (record['inspection'] is not None) == (version == 2), 'preserve failed V1 and actual successful V2')
        require(set(record['phases']) == set(TOOLS), 'three actual inspector tools')
        expected_raw = {'inputs.json', 'inputs-after.json', 'sources-before.json', 'sources-after.json'}
        expected_raw |= {name + suffix for name in TOOLS for suffix in ('-command.json', '-result.json', '-started.json', '-stdout', '-stderr')}
        require(set(record['raw']) == expected_raw, 'nineteen inspection raw records')
        raw(record, label, public)
        inputs = original_map(record['raw']['inputs.json'])
        require(inputs == original_map(record['raw']['inputs-after.json'])
            and all(inputs.get(k) == v for k, v in lower_inputs.items())
            and inputs[record['controller']['path']] == record['controller'], 'input maps stable and prior compiler closure retained')
        require(doc(record['raw']['sources-before.json']) == sources == doc(record['raw']['sources-after.json']), 'inspection source generation stable')
        env = dict(PATH='/usr/bin:/bin', HOME=str(E / label), LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
            TMPDIR=str(E / label / 'scratch'), HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
            LD_LIBRARY_PATH=str(E / 'ordinary-induction-compiler-cpu-v228-v1/target/debug/deps') + ':'
                + str(R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/lib'))
        for name, (tool, size, sha) in TOOLS.items():
            require(inputs[tool] == dict(path=tool, bytes=size, sha256=sha), 'authenticated actual inspector tool identity')
            tail = [image['path'], image['sha256'], str(image['bytes'])] if name == 'descriptor-metadata' else (
                ['--notes', '--symbols', image['path']] if name == 'elf-notes' else ['--disassemble', '--mcpu=gfx950', image['path']])
            c = doc(record['raw'][name + '-command.json'])
            require(c == dict(affinity=[8, 9], argv=[tool] + tail, cache_cap_bytes=6 << 30, deadline_seconds=120,
                env=env, expected_exit=0, gpu_execution=False, nice=10, tools={k: v[2] for k, v in TOOLS.items()}), 'exact inspector command/environment')
            result = doc(record['raw'][name + '-result.json'])
            started = doc(record['raw'][name + '-started.json'])
            require(type(started['pid']) is int and started['pid'] > 1 and started['pid'] == started['pgid']
                and result == record['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['elapsed_seconds'] <= 125
                and all(result[s + '_sha256'] == record['raw'][name + '-' + s]['sha256'] for s in ('stdout', 'stderr')), 'natural inspector leaves with exact streams')
        inspections[version] = (record_pin, record)

    checked = inspections[2][1]['inspection']
    descriptor = checked['descriptor']
    require(descriptor['object_sha256'] == image['sha256'] and descriptor['object_bytes'] == str(image['bytes'])
        and descriptor['target'] == 'gfx950:xnack-' and descriptor['entry_symbol_hex'] == KERNEL.encode().hex()
        and descriptor['explicit_argument_bytes'] == '168' and descriptor['kernarg_segment_bytes'] == '424'
        and descriptor['canonical_code_object_digest'] == observation['hsaco']['canonical_descriptor_sha256']
        and checked['arithmetic_isa_review_performed'] is False and checked['instruction_count_claim'] is False, 'actual metadata result and explicit nonclaims')
    lines = read(inspections[2][1]['raw']['descriptor-metadata-stdout']).decode().splitlines()
    require(lines[0] == 'fe2o3-finite-join-request-metadata-v1'
        and pairs(line.split(' ', 1) for line in lines[1:]) == descriptor, 'descriptor summary exactly replays retained stdout')
    require(re.search(r"^amdhsa\.target:\s*'" + re.escape(right) + r"'\s*$",
        read(inspections[2][1]['raw']['elf-notes-stdout']).decode(), re.MULTILINE),
        'corrected target spelling is in actual notes')
    for name in TOOLS:
        require(inspections[1][1]['raw'][name + '-stdout']['sha256']
            == inspections[2][1]['raw'][name + '-stdout']['sha256'], 'both actual inspector outputs agree')

    for item in list(READS.values()):
        read(item)
    Q.mkdir(exist_ok=True)
    files = {}
    for name, (item, body) in sorted(COPIES.items()):
        require(read(item) == body, 'copy source unchanged')
        path = Q / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body)
        files[name] = dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest(), source_pin=item)
    for item in list(READS.values()):
        read(item)
    result = dict(schema='ferric-p228-projection-residual-lowering-publication-v1', passed=True,
        lowering_complete=lower_pin, inspection_complete=inspections[2][0], failed_inspection=inspections[1][0],
        cpu_publication=public_cpu_pin, source_pins=selected_sources, actual_generic_compile_seconds=lower['elapsed_seconds'],
        artifact=image, observation=artifact['observation'], metadata=checked,
        files=files, externally_retained=OMITTED, local_inputs=list(READS.values()), root_readme=readme,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False, production_authority=False,
        load_authority=False, launch_authority=False, arithmetic_isa_review_performed=False,
        all_transitive_tool_and_library_bodies_rehashed=False, yaml_metadata_reparsed_locally=False,
        compiler_handoff_or_llvm_retained=False,
        limitations=['Local replay rehashes all receipt/raw/source/image bodies used here, not every remote tool, vendor, sysroot or YAML-parser body listed in authenticated input maps.',
            'V1 inspection ran three successful tools but failed its target-spelling validator; V2 corrects only that literal and is a separate actual invocation.',
            'The generic compiler observation grants no load, launch or publication authority. This evidence record is not runtime approval.',
            'HSACO, large input/target maps and existing CPU evidence are linked by identity rather than duplicated in this subtree.',
            'Compiler elapsed time includes engineering compilation and is not kernel or inference performance.'])
    with (Q / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(dict(output=str(Q), files=len(files), result_sha256=hashlib.sha256((Q / 'result.json').read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
