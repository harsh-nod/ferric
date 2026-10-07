"""Closed native HTTP adapters; admission replays every native cell, never launches."""
import hashlib
import importlib.util
import json
from pathlib import Path

CONTRACT_SHA = 'f48ebe829c9a4753971e17f242ba856d6e2e021a32aebb5f8d57ef946bc8ee4d'
REPLAY_SHA = '48d50c2137cb967dca694a01f2f851e3ef52a20d20a67770752e1a875d6cff54'
LIFECYCLE_SHA = '0b4ad2aa60bfe50d962f304ae5794ad66ec507ee32b1a453ac1aa5e9a3128ac3'
SCANNER_SHA = 'c08462f1c740759af8a9119f699955ed9789abddaac264b48bd35ef005e10730'
ADAPTERS = {
    'V17': (CONTRACT_SHA, REPLAY_SHA, 'native-prefill613-ttft-only-v1'),
    'V19': ('58b65d85930fc490a0aabe3e4241aac08720d302c3bc7230f198459d2415acf8',
            '85a48cd74dbfec50c7eef68486ed9eb5aa30090c690b256c8b94ac17d693dedc',
            'native-cached-abi-preparation-only-v1'),
    'Width55c': ('9bbc328a6aa3fc1c8a351a7bcde0223f5dda2a5c923f320d8a0f74e1047a84c0',
                 'b08dcaf72398c6778c21bbdfae7726e7476acc3281d927a96ae5e2bb8ea6a07b',
                 'native-prefill-width16-vs32-ttft-v1'),
    'GateUpDa6b': ('97f83d65302fd0d8fd276c57dc1454e4daf743259f733b0907c16b44eb542815',
                   '37e83245d8b2094d4643a46c379afbed7358c5df4bc6e7b64f865818d715956f',
                   'native-gate-up-control-vs-splitk4-tpot-r1'),
    'DownDa6b': ('207af7b843cefba73da7beff658a213ee280bd21d2ea55a7ec6e6d84c5508739',
                 '023503f46deaec44032f3507d7dd3533854bc78476b1d5f2a42b885c90903b57',
                 'native-down-control-vs-splitk8-tpot-r1'),
}
REQUESTS, BATCHES, DISPATCHES = 42, 135, 87711
SESSION_FIELDS = {'worker_pids', 'session_id', 'setup_seconds'}
WIDTH_CELL_SHA = '866218ffa64d6e0078fee7f4088b2150527fbee233461116fd64f42cbc5dd2e4'
WIDTH_CORE = '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
WIDTH_SOURCES = {
    'runtime_source': 'a9772e55f9a140d0749a621c07ea2c9cb19c36b2416656278dd8d2a2095321d3',
    'controller_source': '6089abea6ebc3363557c8e5fb8b4e2e488fa49b4f30f99029dfcc16b7c5bddd7',
    'worker': 'af246e5b872b639b90906336493629eb2bbfa90ff57ffc4ffd859f8b963d9b30',
}
WIDTH_LIVE = '222f90f913d068e53d5d0494e444cb9bcfbc71a140c6a4f7a9b362bc7a898514'
WIDTH_GEOMETRY = {'A': (16, 135, 87711, 613, 216), 'B': (32, 131, 85403, 649, 396)}
GATE_UP_CELL_SHA = 'f2536da4ad7f6906c1ca027cfdfe46b88bdf245a6b47d1b070e1089e5e61f35b'
GATE_UP_CORE = 'da6b561c5a3f12acc5b0e6da74c808273e728710'
GATE_UP_SOURCES = {
    'runtime_source': 'f44d31f140938f918f159f47fb6c81975063c34526221029a4bc1ff10c72a462',
    'controller_source': '5a49de79a9fe0d4bedcfa77212c6832fd97ebd626e97a37ca5753545bdc24f7d',
    'worker': '74d764e8797ba0413e38b762765d07faf74aa6a0af5daf60c0a1a56adc64f20a',
}
GATE_UP_LIVE = '60bf47551d1ebdfc91f49b529dab8211e3f860a07bdd05921e642b72d505e9a4'
GATE_UP_IMAGE = {
    'artifact_hsaco_id': 'd28610d291eeec0589afbf269e26d21b7111c96f08106e1f661d6a66f024bf03',
    'artifact_manifest_id': 'a7417322dafe8e0af927a6457f0dea2c324e94ec2692329f7d76a215ed3273bb',
    'artifact_handoff_id': 'ea8672a0acfcfef4606c6597f6b7f3af0fbbfcd9634d8feffe5d29d5f11b39b9',
}
GATE_UP_ROSTER_SHA = '54f6c32544669ad4407643e12eb6c22efc0a24967f20ea2d695e24f46488e379'
DOWN_CELL_SHA = 'e59c15758711f40b0f0dac40af246f9471c8dc8938803c430775712fc548e9a3'
DOWN_CORE = 'da6b561c5a3f12acc5b0e6da74c808273e728710'
DOWN_SOURCES = {
    'runtime_source': 'f44d31f140938f918f159f47fb6c81975063c34526221029a4bc1ff10c72a462',
    'controller_source': '9e53fdc9f6c9e0690afac86ccaeed83ce000ebaa772f0deb40a6c57df0fe0b88',
    'worker': '74d764e8797ba0413e38b762765d07faf74aa6a0af5daf60c0a1a56adc64f20a',
}
DOWN_LIVE = 'f1fca91663c1fa682ada28fd79ef3e119d7de9845dfb1d8fb0c860ca0ffada7b'
DOWN_IMAGE = {
    'artifact_hsaco_id': '1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae',
    'artifact_manifest_id': '2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54',
    'artifact_handoff_id': 'cd23060449750a973f05742c0d5a043cb637491743e2ac56b56e847cc2c4dbdf',
}
DOWN_ROSTER_SHA = '10f64ed7e1d41e68b4ec1c4549bba0c559fc9564ea7d181a2c64d1babe035177'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()


def geometry(campaign, arm):
    require(campaign in ADAPTERS and arm in ('A', 'B'), 'closed campaign and explicit arm required')
    if campaign == 'DownDa6b':
        return 131, 85403 if arm == 'A' else 89975
    if campaign == 'GateUpDa6b':
        return 131, 85403 if arm == 'A' else 94547
    return WIDTH_GEOMETRY[arm][1:3] if campaign == 'Width55c' else (BATCHES, DISPATCHES)


def selected_geometry(selected):
    schemas = {'Ferric' + name + 'SelectedHttpArmV1': name for name in ADAPTERS}
    require(selected.get('schema') in schemas, 'closed selected native schema required')
    batches, dispatches = geometry(schemas[selected['schema']], selected.get('arm'))
    require(selected.get('requests') == REQUESTS
            and selected.get('model_batches') == REQUESTS * batches
            and selected.get('model_dispatches') == REQUESTS * dispatches,
            'selected request, batch and dispatch budgets must match the exact campaign')
    return REQUESTS * batches, REQUESTS * dispatches


def width_scope(arm, spec, result, build, report):
    require(build.get('schema') == 'FerricPrefillWidthBuildBindingV1'
            and build.get('runtime_main') == WIDTH_CORE
            and all(build.get(key, {}).get('sha256') == value for key, value in WIDTH_SOURCES.items())
            and all(build['controllers'].get('live-' + key, {}).get('sha256') == WIDTH_LIVE for key in ('A', 'B')),
            'exact current55c qualified width source and binaries required')
    require(report.get('status') == 'promotable'
            and report.get('promotion_scope') == 'prefill-width-within-slots512-native-backend-only'
            and report.get('ordered64_comparison_performed') is False,
            'complete passing standalone width campaign required before HTTP selection')
    rows, _, _, commands, slots = WIDTH_GEOMETRY[arm]
    argv = spec['argv']
    require(argv[1:3] == ['--native-prefill-rows', str(rows)]
            and argv.count('--native-prefill-rows') == 1,
            'explicit canonical selected-width prefix required')
    setup = result['setup']
    require(setup.get('prefill_chunk') == rows
            and setup.get('live_profile') == f'prefill{rows}-native{commands}-slots512-worker-decode652-v1'
            and setup.get('prefill_program', {}).get('rows') == rows
            and setup['prefill_program'].get('dispatches') == commands
            and setup['prefill_program'].get('dynamic_slots') == slots
            and setup.get('token_program', {}).get('backend') == 'native-whole-program-slots512-v1',
            'selected width and native512 profile must remain explicit')


def gate_up_scope(arm, spec, result, build, report):
    require(build.get('schema') == 'FerricNativeGateUpBuildBindingR1'
            and build.get('runtime_main') == GATE_UP_CORE
            and all(build.get(key, {}).get('sha256') == value for key, value in GATE_UP_SOURCES.items())
            and all(build['controllers'].get('live-' + key, {}).get('sha256') == GATE_UP_LIVE for key in ('A', 'B')),
            'exact da6b gate/up source and binaries required')
    require(report.get('status') == 'experimental-gates-passed'
            and report.get('promotion_scope') == 'explicit-gate-up-selector-native32-backend-only'
            and report.get('ordered64_comparison_performed') is False,
            'complete passing standalone gate/up campaign required before HTTP selection')
    selection, commands = ('control', 652) if arm == 'A' else ('splitk4', 724)
    argv = spec['argv']
    gate = spec.get('gate_up_expected', {})
    roster = gate.get('compiler_roster', {})
    require(gate.get('artifact') == GATE_UP_IMAGE and roster.get('sha256') == GATE_UP_ROSTER_SHA
            and gate.get('loaded_image_count') == 10 and gate.get('scratch_bytes') == 196608
            and gate.get('prefill_unchanged') is True,
            'same qualified tenth image, roster, scratch and unchanged prefill required')
    flags = {'--native-prefill-rows': '32', '--native-gate-up': selection,
             '--gate-up-artifact': gate.get('artifact_path'), '--gate-up-roster': roster.get('path'),
             '--gate-up-roster-sha256': GATE_UP_ROSTER_SHA,
             '--gate-up-hsaco-sha256': GATE_UP_IMAGE['artifact_hsaco_id'],
             '--gate-up-manifest-sha256': GATE_UP_IMAGE['artifact_manifest_id'],
             '--gate-up-handoff-sha256': GATE_UP_IMAGE['artifact_handoff_id']}
    require(argv[1:3] == ['--native-prefill-rows', '32']
            and all(argv.count(flag) == 1 and argv.index(flag) + 1 < len(argv)
                    and argv[argv.index(flag) + 1] == value for flag, value in flags.items()),
            'exact selected gate/up flags required')
    setup = result['setup']
    selected = {**gate, 'enabled': arm == 'B', 'decode_dispatches': commands}
    require(setup.get('prefill_chunk') == 32
            and setup.get('live_profile') == f'prefill32-native649-decode{commands}-gate-up-{selection}-r1'
            and setup.get('prefill_program', {}).get('rows') == 32
            and setup['prefill_program'].get('dispatches') == 649
            and setup['prefill_program'].get('dynamic_slots') == 396
            and setup.get('token_program', {}).get('backend') == 'native-whole-program-slots512-v1'
            and setup.get('native_gate_up') == selected
            and setup.get('performance_profile', {}).get('native_gate_up') == selected,
            'exact native32 selected gate/up setup and profile required')


def down_scope(arm, spec, result, build, report):
    require(build.get('schema') == 'FerricNativeDownBuildBindingR1'
            and build.get('runtime_main') == DOWN_CORE
            and all(build.get(key, {}).get('sha256') == value for key, value in DOWN_SOURCES.items())
            and all(build['controllers'].get('live-' + key, {}).get('sha256') == DOWN_LIVE for key in ('A', 'B')),
            'exact da6b standalone down source and binaries required')
    require(report.get('status') == 'experimental-gates-passed'
            and report.get('promotion_scope') == 'explicit-down-selector-native32-backend-only'
            and report.get('ordered64_comparison_performed') is False,
            'complete passing standalone down campaign required before HTTP selection')
    selection, commands = ('control', 652) if arm == 'A' else ('splitk8', 688)
    argv = spec['argv']
    down = spec.get('down_expected', {})
    roster = down.get('compiler_roster', {})
    require(down.get('artifact') == DOWN_IMAGE and roster.get('sha256') == DOWN_ROSTER_SHA
            and down.get('loaded_image_count') == 10 and down.get('scratch_bytes') == 131072
            and down.get('prefill_unchanged') is True,
            'same qualified tenth image, roster, scratch and unchanged prefill required')
    require('gate_up_expected' not in spec
            and not any(word.startswith(('--native-gate-up', '--gate-up-')) for word in argv),
            'standalone down cannot compose a gate/up selector')
    flags = {'--native-prefill-rows': '32', '--native-down': selection,
             '--down-artifact': down.get('artifact_path'), '--down-roster': roster.get('path'),
             '--down-roster-sha256': DOWN_ROSTER_SHA,
             '--down-hsaco-sha256': DOWN_IMAGE['artifact_hsaco_id'],
             '--down-manifest-sha256': DOWN_IMAGE['artifact_manifest_id'],
             '--down-handoff-sha256': DOWN_IMAGE['artifact_handoff_id']}
    require(argv[1:3] == ['--native-prefill-rows', '32']
            and all(argv.count(flag) == 1 and argv.index(flag) + 1 < len(argv)
                    and argv[argv.index(flag) + 1] == value for flag, value in flags.items()),
            'exact selected down flags required')
    setup = result['setup']
    selected = {**down, 'enabled': arm == 'B', 'decode_dispatches': commands}
    require('native_gate_up' not in setup and 'native_gate_up' not in setup.get('performance_profile', {})
            and setup.get('prefill_chunk') == 32
            and setup.get('live_profile') == f'prefill32-native649-decode{commands}-down-{selection}-r1'
            and setup.get('prefill_program', {}).get('rows') == 32
            and setup['prefill_program'].get('dispatches') == 649
            and setup['prefill_program'].get('dynamic_slots') == 396
            and setup.get('token_program', {}).get('backend') == 'native-whole-program-slots512-v1'
            and setup.get('native_down') == selected
            and setup.get('performance_profile', {}).get('native_down') == selected,
            'exact standalone native32 selected down setup and profile required')


def project_arm(arm, spec, result, build, report, *, campaign='V17'):
    """Pure projection; caller must supply the independently replayed objects."""
    batches, dispatches = geometry(campaign, arm)
    require(arm in ('A', 'B') and spec['arm'] == arm and spec['mode'] == 'latency',
            'one explicit uninstrumented native arm required')
    require(result.get('accepted') is True and result.get('raw_replay_passed') is True
            and result.get('instrumented') is False and result.get('latency_admitted') is True,
            'completed raw-replayed native latency arm required')
    require(spec['controller']['sha256'] == build['controllers']['live-' + arm]['sha256']
            and spec['worker']['sha256'] == build['worker']['sha256'],
            'same CPU-qualified source/executables required')
    argv = list(spec['argv'])
    require(argv[0] == spec['controller']['path'] and argv.count('--max-batches') == 1,
            'closed native argv must contain one controller and batch budget')
    position = argv.index('--max-batches') + 1
    require(position < len(argv) and argv[position] == str(6 * batches), 'six-request native source budget')
    require('--token-program-backend' not in argv and '--token-program-fence-mode' not in argv,
            'instrumented or composed native arm cannot enter HTTP timing')
    argv[position] = str(REQUESTS * batches)
    setup = {key: value for key, value in result['setup'].items() if key not in SESSION_FIELDS}
    require(setup.get('max_batches') == 6 * batches and setup.get('head_precision') == 'fp32-v8'
            and setup.get('dtype') == 'BF16' and setup.get('tensor_parallel') == 1,
            'frozen native precision and geometry required')
    setup['max_batches'] = REQUESTS * batches
    require(setup.get('token_program', {}).get('counter_diagnostic') is False
            and setup.get('performance_profile', {}).get('runtime_profiling') is False,
            'no profiling in timed HTTP controller')
    require(campaign in ADAPTERS and report.get('experiment') == ADAPTERS[campaign][2]
            and report.get('default_promotion') is False
            and report.get('vendor_comparison_performed') is False,
            'exact standalone prerequisite scope required')
    if campaign == 'Width55c':
        width_scope(arm, spec, result, build, report)
    elif campaign == 'GateUpDa6b':
        gate_up_scope(arm, spec, result, build, report)
    elif campaign == 'DownDa6b':
        down_scope(arm, spec, result, build, report)
    return {'schema': 'Ferric' + campaign + 'SelectedHttpArmV1', 'arm': arm, 'argv': argv,
        'controller': spec['controller'], 'worker': spec['worker'],
        'controller_source': build['controller_source'], 'runtime_source': build['runtime_source'],
        'expected_setup': setup, 'reference': spec['reference'], 'prompt': spec['prompt'],
        'native_spec': spec, 'native_promotion_status': report['status'],
        'default_promotion': False, 'vendor_comparison_performed': False,
        'requests': REQUESTS, 'model_batches': REQUESTS * batches,
        'model_dispatches': REQUESTS * dispatches,
        'scope': 'selected native prerequisite; no HTTP launch or comparison result'}


def admit(selection):
    schemas = {'Ferric' + name + 'HttpSelectionV1': name for name in ADAPTERS}
    require(type(selection) is dict and set(selection) == {'schema', 'arm', 'plan', 'report'}
            and selection['schema'] in schemas
            and selection['arm'] in ('A', 'B'), 'closed explicit native campaign and arm selection required')
    campaign = schemas[selection['schema']]
    contract_sha, replay_sha, _ = ADAPTERS[campaign]
    binding = selection['plan']
    require(type(binding) is dict and set(binding) == {'path', 'sha256'}, 'bound native plan required')
    path = Path(binding['path'])
    stage = path.parent
    require(path.is_absolute() and path.name == 'plan.json' and path.resolve(strict=True) == path,
            'original canonical retained native stage required')
    source = stage / 'launch_contract.py'
    require(hashlib.sha256(source.read_bytes()).hexdigest() == contract_sha,
            'only the exact qualified prerequisite adapter is admitted')
    definition = importlib.util.spec_from_file_location('http_qualified_native_contract', source)
    contract = importlib.util.module_from_spec(definition)
    definition.loader.exec_module(contract)
    raw = contract.read(path, binding['sha256'], 32 * 1024**2)[0]
    plan = contract.decode(raw)
    loaded, _, evidence, cell, _, _ = contract.validate_plan(plan, stage)
    _, _, legacy, _, _, runner, _, _ = loaded
    require(plan['sources']['measurement/native_campaign_replay.py'] == replay_sha
            and plan['sources']['measurement/native_lifecycle.py'] == LIFECYCLE_SHA
            and plan['sources']['measurement/gpu_activity.py'] == SCANNER_SHA,
            'qualified raw replay, lifecycle and descriptor attribution required')
    if campaign == 'Width55c':
        require(plan.get('schema') == 'FerricPrefillWidthCampaignPlanV1'
                and len(plan.get('cells', [])) == 14
                and plan['sources']['measurement/native_token_cell.py'] == WIDTH_CELL_SHA,
                'complete current55c width campaign and exact width parser required')
    elif campaign == 'GateUpDa6b':
        require(plan.get('schema') == 'FerricNativeGateUpCampaignPlanR1'
                and len(plan.get('cells', [])) == 14
                and plan['sources']['measurement/native_token_cell.py'] == GATE_UP_CELL_SHA,
                'complete da6b gate/up campaign and exact parser required')
    elif campaign == 'DownDa6b':
        require(plan.get('schema') == 'FerricNativeDownCampaignPlanR1'
                and len(plan.get('cells', [])) == 14
                and plan['sources']['measurement/native_token_cell.py'] == DOWN_CELL_SHA,
                'complete da6b down campaign and exact parser required')
    replay = contract.module(stage / 'measurement/native_campaign_replay.py', replay_sha)
    rows = [replay.load_retained_cell(entry, Path(entry['output']), plan_binding=binding,
            runner=runner, legacy=legacy, evidence=evidence) for entry in plan['cells']]
    replay.validate_counter_pair(rows[:2])
    actual = cell.evaluate_campaign(plan['comparison'], rows[2:], rows[:2])
    actual['cpu_cost_by_cell'] = {row['cell_id']: row['completion']['cpu_cost'] for row in rows}
    report = contract.bound(selection['report'])
    require(canonical(report) == canonical(actual), 'completed native report differs from raw replay')
    selected = next(row for row in rows[2:] if row['spec']['arm'] == selection['arm'])
    build = contract.bound(plan['build'])
    result = project_arm(selection['arm'], selected['spec'], selected['result'], build, actual, campaign=campaign)
    result['native_evidence'] = selection
    result['native_cells'] = [row['custody'] for row in rows]
    result['native_build'] = plan['build']
    result['native_cpu'] = plan['cpu']
    result['images'] = plan['images']
    if campaign == 'GateUpDa6b':
        result['gate_up'] = plan['gate_up']
    elif campaign == 'DownDa6b':
        result['down'] = plan['down']
    require(contract.read(path, binding['sha256'])[0] == raw, 'native plan changed during HTTP selection')
    contract.verify_files(stage, plan['files'])
    return result


def validate_setup(actual, selected):
    expected = selected['expected_setup']
    require(canonical({key: value for key, value in actual.items() if key not in SESSION_FIELDS})
            == canonical(expected), 'all non-session native setup fields must remain exact')
    pids = actual.get('worker_pids')
    require(type(pids) is list and len(pids) == 1 and type(pids[0]) is int and pids[0] > 1,
            'one actual HTTP worker required')


def validate_closed(events, setup, selected, cell):
    _, dispatches = selected_geometry(selected)
    closed = [row for row in events if row.get('schema') == 'FerricQwen3TpBatchClosedV2']
    require(len(closed) == 1, 'one complete native HTTP close required')
    value = closed[0]
    cell.composition(value, selected['native_spec'])
    cell.exact(value, selected['native_spec']['closed_expected'], 'frozen close metadata changed')
    cell.exact(value, {'authority': 'none', 'execution_completed': True, 'all_workers_exited': True,
        'worker_pids': setup['worker_pids'], 'rank_dispatch_counts': [dispatches]},
        'exact 42-request native HTTP close required')
