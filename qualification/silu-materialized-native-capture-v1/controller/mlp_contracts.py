"""MLP-specific metadata/emission joins; arithmetic and runtime authority stay open."""
import json
import re

SYMBOL = 'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2'


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def sha(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'SHA256')


def fields(raw, header):
    lines = raw.decode('ascii').splitlines()
    require(lines and lines.pop(0) == header, 'exact receipt header')
    result = {}
    for line in lines:
        key, value = line.split(' ', 1)
        require(key not in result, 'duplicate receipt field')
        result[key] = value
    return result


def exact_test(raw, selector):
    text = raw.decode('utf-8')
    require(re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
            == [('1', '0', '0')] and re.search(r'^test ' + re.escape(selector) + r' \.\.\. ok$', text, re.M),
            'one exact retained callback must actually execute')


def metadata(raw, image):
    found = fields(raw, 'fe2o3-finite-join-request-metadata-v1')
    expected = dict(authority='none', object_sha256=image['sha256'], object_bytes=str(image['bytes']),
        entry_symbol_hex=SYMBOL.encode().hex(), target='gfx950:xnack-', code_object_version='6',
        explicit_argument_bytes='88', kernarg_segment_bytes='344', kernarg_alignment='8',
        workgroup='64,1,1', max_grid_workgroups='64,1,1')
    require(set(found) == set(expected) | {'descriptor_sha256', 'canonical_code_object_digest', 'descriptor_symbol_hex'}
            and all(found[key] == value for key, value in expected.items()), 'unchanged checked MLP ABI')
    for key in ('descriptor_sha256', 'canonical_code_object_digest'):
        sha(found[key])
    require(re.fullmatch('(?:[0-9a-f]{2})+', found['descriptor_symbol_hex']), 'descriptor symbol')
    return found


def emission(raw, outputs, worker, worker_build, llvm_build, config):
    lines = raw.decode('utf-8').splitlines()
    require(lines and lines.pop(0) == 'wave_mlp_tile_engineering_emission_v2', 'MLP emission schema')
    found, provider_files, imports = {}, {}, []
    for line in lines:
        key, value = line.split(' ', 1)
        if key == 'builtin_device_library_file':
            name, digest = value.split()
            require(name not in provider_files, 'duplicate provider file')
            sha(digest)
            provider_files[name] = digest
        elif key == 'builtin_device_library_import':
            require(value not in imports, 'duplicate provider import')
            imports.append(value)
        else:
            require(key not in found, 'duplicate emission field')
            found[key] = value
    expected = dict(status='PASS', authority='none', generic_proofs='unsupported',
        runtime_requirements='required_undischarged', unresolved_requirement_count='8',
        external_provider_count='0', builtin_device_library_provider='gfx950-ocml-rocm-7.2.1-v1',
        builtin_device_library_import_count='1', builtin_device_library_file_count='9',
        worker_timeout_seconds_per_pass='120', worker_stdout_cap=str(40 << 20),
        worker_stderr_cap=str(64 << 10), hsaco_cap=str(32 << 20))
    identities = {'outer_v3_content', 'outer_v3_domain_identity', 'capsule_content', 'pair_binding_content',
                  'nested_v2_content', 'worker_executable', 'bootstrap_request', 'bootstrap_response',
                  'replay_request', 'replay_response', 'finalized_hsaco_content'}
    require(set(found) == set(expected) | identities | {'worker_build', 'llvm_build',
        'observer_compile_replay_finalize_wall_ms', 'builtin_device_library_manifest'}, 'closed emission fields')
    require(all(found[key] == value for key, value in expected.items()), 'unchanged checked engineering limits')
    require(len(config) == 9 and provider_files == config and imports == ['__ocml_exp_f32'], 'pinned nine-file Exp provider')
    sha(found['builtin_device_library_manifest'])
    for name in identities:
        require(re.fullmatch('[0-9a-f]{64} [1-9][0-9]*', found[name]), 'typed artifact identity')
    require(re.fullmatch('[0-9]+', found['observer_compile_replay_finalize_wall_ms']), 'host-only observed duration')
    require(json.loads(found['worker_build']) == worker_build and json.loads(found['llvm_build']) == llvm_build,
            'reviewed worker/LLVM generation')
    for key, pin in (('outer_v3_content', outputs['source.handoff-v3']),
                     ('nested_v2_content', outputs['compiler.handoff-v2']),
                     ('finalized_hsaco_content', outputs['artifact.hsaco']), ('worker_executable', worker)):
        require(found[key] == pin['sha256'] + ' ' + str(pin['bytes']), 'exact artifact content join')
    return found
