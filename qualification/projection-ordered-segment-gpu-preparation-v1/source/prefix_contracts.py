"""Pure closed contracts for the ordinary V41/OCML prefix source pipeline."""
import hashlib
import json
from pathlib import Path
import re

GROUPS = ('provider', 'core', 'compiler', 'consumers', 'native')
COUNTS = dict(provider=469, core=3166, compiler=1214, consumers=692, native=1636)
CLOSURE = '037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675'
SYMBOL = 'ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6'
CAPTURES = ('prefix-tiles-semantic.bin', 'prefix-tiles-neutral-kir.bin',
            'prefix-tiles-target-kir.bin', 'prefix-tiles.handoff-v3')
FALSE_FLAGS = ('gpu_execution', 'production_authority', 'launch_authority',
               'numerical_acceptance', 'performance_claim', 'runtime_requirements_discharged',
               'isa_review_accepted')
LEAVES = (
    ('compiler-tools', 900), ('finalizer-tools', 900), ('provider-build', 900),
    ('fake-build', 900), ('genuine-authentication', 900), ('fake-authentication', 900),
    ('fixture-lock', 900), ('fixture-metadata', 900), ('checked-lowering', 1800),
    ('actual-replay', 1800), ('actual-inert-join', 900), ('emit', 900),
    ('extractor-build', 900), ('extract-retained', 900), ('descriptor-metadata', 900),
    ('elf-notes', 900), ('disassembly', 900),
)
DEADLINE = sum(seconds for _, seconds in LEAVES) + 600
INSPECTION_TOOLS = {
    'readelf': ('/opt/rocm-7.3.0/lib/llvm/bin/llvm-readobj',
                '8cb6079bc349196a3f3af7589912ca48f716194970ccd0e6c6a36b09e0b21259'),
    'objdump': ('/opt/rocm-7.3.0/lib/llvm/bin/llvm-objdump',
                'be832bf5ddf82b3feb8a6e24f453e2602bd01a406f9f5bde47b225c4989d1a83'),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def parse(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def sha(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'SHA256')


def pin(value, root):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}, 'FilePin fields')
    require(type(value['path']) is str and type(value['bytes']) is int
            and 0 < value['bytes'] <= 1 << 30, 'FilePin extent')
    path = Path(value['path'])
    require(path.is_absolute() and path.is_relative_to(root) and '..' not in path.parts
            and str(path) == value['path'], 'task-owned FilePin path')
    sha(value['sha256'])


def plan(value, root, evidence):
    fields = {'schema', 'installed', 'root_source', 'pipeline_source', 'cohorts',
              'provider_closure', 'fixture_recipe', 'readelf', 'objdump', 'measurement_history'}
    require(type(value) is dict and set(value) == fields
            and value['schema'] == 'ferric-p227-prefix-tiles-source-inputs-v5', 'closed input schema')
    for key in fields - {'schema', 'cohorts', *INSPECTION_TOOLS}:
        pin(value[key], root)
    for key, (path, expected) in INSPECTION_TOOLS.items():
        pin(value[key], Path('/opt/rocm-7.3.0/lib/llvm/bin'))
        require(value[key]['path'] == path and value[key]['sha256'] == expected,
                'exact retained read-only inspection tool')
    require(type(value['cohorts']) is dict and set(value['cohorts']) == set(GROUPS),
            'five fresh current-source CPU cohorts')
    for group in GROUPS:
        item = value['cohorts'][group]
        require(type(item) is dict and set(item) == {'complete', 'owned'}, 'cohort input fields')
        for key in item:
            pin(item[key], evidence)
        require(Path(item['complete']['path']).name == 'complete.json'
                and Path(item['owned']['path']).name == 'result.json'
                and Path(item['complete']['path']).parent.parent == evidence
                and Path(item['owned']['path']).parent.parent == evidence, 'flat actual cohort outputs')
    require(Path(value['installed']['path']).name == 'installed.json'
            and Path(value['root_source']['path']).name == 'sources-after.json'
            and Path(value['pipeline_source']['path']).name == 'sources.json'
            and len({Path(value[key]['path']).parent for key in
                     ('installed', 'root_source', 'pipeline_source')}) == 1,
            'same actual installation source record')
    require(Path(value['measurement_history']['path']) ==
            Path(value['installed']['path']).parent / 'measurement-inputs-before.json',
            'exact overlay installation historical-input record')


def diagnostic_predecessor(value):
    require(value['schema'] == 'ferric-provider-closure-diagnostic-measurement-v226-v1'
            and value['source_closure_sha256']
                == 'c002d1aba603d26f5d5827ea00dca466383d1e198a1a5919f1193fef83fd8213'
            and all(value[key] is False for key in ('source_edited', 'tests_run', 'compiler_tested',
                'provider_target_checked', 'provider_authenticated', 'qualification_complete',
                'gpu_execution', 'production_authority')),
            'historical51 diagnostic flags stay false; new CPU evidence is independent')


def measurement(value, provider_package, predecessor, coverage, provider):
    require(type(CLOSURE) is str and value['schema'] == 'ferric-provider-closure-measurement-v227-v1'
            and value['source_closure_sha256'] == CLOSURE
            and value['provider_target_checked'] is True
            and all(value[key] is False for key in ('source_edited', 'compiler_tested',
                'provider_authenticated', 'qualification_complete', 'gpu_execution', 'production_authority')),
            'actual53 measurement and independent provider target, without authentication authority')
    require(value['reviewed_provider_package'] == provider_package
            and value['installed'] == predecessor['installed']
            and value['source_snapshots'] == {key: predecessor[key] for key in ('root', 'pipeline')}
            and value['source_coverage'] == coverage, 'measurement original provider installation')
    require(value['domain'] == 'FE2O3/WORKGROUP-SYNC-PROVIDER-SOURCE-CLOSURE/V1\0'
            and value['package_root'] == str(provider)
            and value['added'] == [
                'src/finite_join/wave_qkv_attention_output_tiles_v6.rs',
                'src/finite_join/wave_qkv_attention_output_tiles_v6_tests.rs']
            and value['changed'] == ['src/finite_join.rs'], 'exact measured provider extension')
    for key in ('cpu_receipt', 'owned_result', 'cohort_controller', 'previous_closure'):
        pin(value[key], provider.parents[2])
    require(type(value['files']) is list and len(value['files']) == 53,
            'exact measured53 source roster')
    for record in value['files']:
        pin(record, provider)
    require(len({record['path'] for record in value['files']}) == 53,
            'distinct provider source paths')
    return sorted(value['files'], key=lambda record: str(Path(record['path']).relative_to(provider)))


def cargo_rows(raw):
    rows = [parse(line) for line in raw.splitlines() if line.startswith(b'{')]
    require([r.get('success') for r in rows if r.get('reason') == 'build-finished'] == [True],
            'one successful Cargo completion')
    return rows


def select_artifact(raw, name, kind, test, manifest, target, extension=None):
    candidates = []
    for row in cargo_rows(raw):
        if row.get('reason') != 'compiler-artifact' or row.get('target', {}).get('name') != name:
            continue
        if row['target'].get('kind') != kind or row.get('manifest_path') != str(manifest):
            continue
        require(type(row.get('profile', {}).get('test')) is bool, 'Cargo test boolean')
        if row['profile']['test'] != test:
            continue
        if extension is None:
            paths = [row.get('executable')]
        else:
            require(type(row.get('filenames')) is list, 'Cargo filename list')
            paths = [p for p in row['filenames'] if type(p) is str and p.endswith(extension)]
        require(len(paths) == 1 and type(paths[0]) is str, 'one actual artifact path')
        path = Path(paths[0])
        require(path.is_absolute() and path.is_relative_to(target) and '..' not in path.parts,
                'actual Cargo artifact under selected target')
        candidates.append((path, row))
    require(len(candidates) == 1, 'one selected Cargo artifact: ' + name)
    return candidates[0]


def exact_test(raw, selector):
    text = raw.decode('utf-8')
    require(re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
            == [('1', '0', '0')], 'one actual ignored callback executed')
    require(re.search(r'^test ' + re.escape(selector) + r' \.\.\. ok$', text, re.M),
            'exact selected callback passed')


def metadata(raw, image):
    lines = raw.decode('ascii').splitlines()
    require(lines and lines.pop(0) == 'fe2o3-finite-join-request-metadata-v1', 'metadata schema')
    fields = {}
    for line in lines:
        key, value = line.split(' ', 1)
        require(key not in fields, 'duplicate metadata field')
        fields[key] = value
    required = dict(authority='none', object_sha256=image['sha256'], object_bytes=str(image['bytes']),
        entry_symbol_hex=SYMBOL.encode().hex(), target='gfx950:xnack-', code_object_version='6',
        explicit_argument_bytes='120', kernarg_segment_bytes='376', kernarg_alignment='8',
        workgroup='64,1,1', max_grid_workgroups='64,1,1')
    require(set(fields) == set(required) | {'descriptor_sha256', 'canonical_code_object_digest',
                                          'descriptor_symbol_hex'}, 'exact metadata fields')
    require(all(fields[key] == value for key, value in required.items()), 'actual prefix-tile ABI')
    for key in ('descriptor_sha256', 'canonical_code_object_digest'):
        sha(fields[key])
    require(re.fullmatch('(?:[0-9a-f]{2})+', fields['descriptor_symbol_hex']), 'descriptor symbol bytes')
    return fields


def emission(raw, outputs, worker, worker_build, llvm_build, config):
    lines = raw.decode('utf-8').splitlines()
    require(lines and lines.pop(0) == 'wave_qkv_attention_output_tile_engineering_emission_v6',
            'prefix-tile emission schema')
    fields, provider_files, imports = {}, {}, []
    for line in lines:
        key, value = line.split(' ', 1)
        if key == 'builtin_device_library_file':
            basename, digest = value.split()
            require(basename not in provider_files, 'unique built-in library file')
            sha(digest)
            provider_files[basename] = digest
        elif key == 'builtin_device_library_import':
            require(value not in imports, 'unique built-in library import')
            imports.append(value)
        else:
            require(key not in fields, 'unique receipt fields')
            fields[key] = value
    exact = dict(status='PASS', authority='none', generic_proofs='unsupported',
        runtime_requirements='required_undischarged', unresolved_requirement_count='8',
        external_provider_count='0', builtin_device_library_provider='gfx950-ocml-rocm-7.2.1-v1',
        builtin_device_library_import_count='1', builtin_device_library_file_count='9',
        worker_timeout_seconds_per_pass='120',
        worker_stdout_cap=str(40 << 20), worker_stderr_cap=str(64 << 10), hsaco_cap=str(32 << 20))
    identities = {'outer_v3_content', 'outer_v3_domain_identity', 'capsule_content',
        'pair_binding_content', 'nested_v2_content', 'worker_executable', 'bootstrap_request',
        'bootstrap_response', 'replay_request', 'replay_response', 'finalized_hsaco_content'}
    require(set(fields) == set(exact) | identities | {'worker_build', 'llvm_build',
        'observer_compile_replay_finalize_wall_ms', 'builtin_device_library_manifest'},
        'closed exact OCML Exp emission roster')
    sha(fields['builtin_device_library_manifest'])
    require(type(config) is dict and len(config) == 9 and provider_files == config
            and imports == ['__ocml_exp_f32'], 'actual pinned nine-file Exp provider')
    for name in identities:
        require(re.fullmatch('[0-9a-f]{64} [1-9][0-9]*', fields[name]), 'typed receipt identity')
    require(re.fullmatch('[0-9]+', fields['observer_compile_replay_finalize_wall_ms']),
            'observed wall time, not GPU timing')
    require(all(fields.get(key) == value for key, value in exact.items()), 'unchanged engineering emission bounds')
    require(parse(fields['worker_build']) == worker_build and parse(fields['llvm_build']) == llvm_build,
            'actual reviewed LLVM worker builds')
    for key, record in [('outer_v3_content', outputs['source.handoff-v3']),
                        ('nested_v2_content', outputs['compiler.handoff-v2']),
                        ('finalized_hsaco_content', outputs['artifact.hsaco']),
                        ('worker_executable', worker)]:
        require(fields.get(key) == record['sha256'] + ' ' + str(record['bytes']), 'emission bytes: ' + key)
    return fields
