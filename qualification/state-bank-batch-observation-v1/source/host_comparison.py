"""Real diagnostic argv/wrapper intake; frozen baseline/reference arithmetic."""
import host_validation as H


def candidate(C, read, value, h):
    C.keys(value, 'native_files host_sidecar request parent owner command started stdout stderr')
    v = h['decode_validation']; raw, files, observed = C.native_files(read, value['native_files'], v)
    structural = v.validate(raw, files)
    external_request = C.doc(read, value['request'], 65536)
    decode, policy = H.request(external_request)
    C.require(H.same(decode, observed['request']), 'actual external V2 request decode projection')
    C.get(read, value['parent'], 128 << 20)
    owner = C.doc(read, value['owner']); C.clean_owner(owner)
    C.require(owner['gpu_execution_requested'] is True, 'actual diagnostic model leaf')
    for name in ('command', 'started', 'stdout', 'stderr'):
        C.require(C.pin(owner[name]) == C.pin(value[name]), 'owner raw record join')
    command = C.doc(read, value['command']); start = C.doc(read, value['started'])
    C.keys(command, 'argv env cwd deadline_seconds affinity nice address_space_bytes file_cap_bytes '
        'stream_cap_bytes gpu_execution_requested')
    C.require(command['argv'] == [C.pin(value['parent'])['path'], '--request', C.pin(value['request'])['path'],
        '--allow-unauthenticated-machine-code', '--observe-host-policy']
        and command['cwd'] == C.ROOT and command['deadline_seconds'] == 4000
        and command['address_space_bytes'] == 32 << 30 and command['file_cap_bytes'] == 64 << 20
        and command['stream_cap_bytes'] == 8 << 20 and command['affinity'] == [8, 9]
        and command['nice'] == 10 and command['gpu_execution_requested'] is True,
        'unchanged owned envelope with actual opt-in diagnostic argv')
    C.require(start['command_sha256'] == C.pin(value['command'])['sha256'], 'started command identity')
    sidecar = C.get(read, value['host_sidecar'], H.MAX_BYTES)
    C.require(sum(C.pin(v)['bytes'] for v in value['native_files'].values()) + len(sidecar) <= 8 << 20,
              'native plus host sidecar aggregate8MiB')
    host = H.validate(C, C.get(read, value['stdout'], H.MAX_BYTES), sidecar, value['host_sidecar'], raw,
                      observed, value['parent'], external_request)
    C.parent_stderr(C.get(read, value['stderr']), observed['child_pid'], observed['request']['mode'])
    return observed, files, structural, C.lineage(owner, start, observed['child_pid']), host


def compare(C, plan, read, h):
    C.keys(plan, 'schema mode policy candidate baseline reference')
    C.require(plan['schema'] == 'ferric-p228-prefix-decode-host-policy-comparison-inputs-v2'
        and plan['mode'] in C.MODES and type(plan['policy']) is str and plan['policy'] in H.POLICIES,
        'closed host policy diagnostic comparison')
    observed, files, structural, ownership, host = candidate(C, read, plan['candidate'], h)
    C.require(observed['request']['mode'] == plan['mode'] and host['policy'] == plan['policy'],
              'candidate selected mode and policy')
    previous, baseline_files = C.baseline(read, plan['baseline'], plan['mode'], h)
    for name in ('expected_model_id', 'expected_bundle_id'):
        C.require(H.same(observed['request'][name], previous['request'][name]), 'same actual model identity')
    pairs = [(observed['request']['prompt'][name], previous['request']['prompt'][name])
             for name in ('manifest', 'text', 'tokens')]
    pairs += [(observed['request']['images'][name], previous['request']['images'][name])
              for name in ('prefix', 'mlp', 'residual', 'tail')]
    pairs += [(observed['request']['tiles_image'], previous['request']['tiles_image'])]
    for left, right in pairs:
        a, b = C.pin(left), C.pin(right)
        C.require((a['bytes'], a['sha256']) == (b['bytes'], b['sha256']), 'same prompt and original/MLP548 images')
    for position in range(4):
        current = C.document(files[f'request-{position}.json'])['command']
        old = C.document(baseline_files[f'request-{position}.json'])['command']
        C.require(H.same(current['cache_metadata'], old['cache_metadata'])
            and H.same(current['rotary_bits'], old['rotary_bits']), 'same full KV mapping and rotary input')
    rows = [files[f'observation-{i}.bin'] for i in range(4)]
    comparison = C.compare_rows(C.records(previous), [baseline_files[f'observation-{i}.bin'] for i in range(4)],
        C.records(observed), rows, h['diagnostics'])
    ref_records, ref_rows = C.reference(read, plan['reference'], plan['mode'], h)
    independent = C.compare_rows(ref_records, ref_rows, C.records(observed), rows, h['diagnostics'])
    exact = all(row['same_input_history'] and all(t['byte_equal'] for t in row['tensors']) for row in comparison)
    return dict(schema='ferric-p228-prefix-decode-host-policy-comparison-v2', mode=plan['mode'], policy=plan['policy'],
        status='EXACT_NATIVE_BASELINE_PARITY' if exact else 'NATIVE_BASELINE_MISMATCH',
        full152_tensor_rows_bitwise_equal=exact, structural=structural, owned_record_checks=ownership,
        native_baseline=comparison, independent_framework=independent, host_observation=host,
        inputs=plan, gpu_launched=False, recorded_close_and_owner_reap_checked=True,
        current_source_binary_image_authority_verified=False, current_platform_idle_audits_verified=False,
        independent_tensor_acceptance=False, independent_tensor_threshold=None,
        full_model_acceptance=False, sustained_2048_256=False, performance_claim=False, production_authority=False)
