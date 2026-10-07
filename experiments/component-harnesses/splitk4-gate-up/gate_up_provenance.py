"""Exact Wave/split-K4 provenance; observations grant no native authority."""
import hashlib

PINS = {
    "splitk4/capture.json": "9b4966c0bef0b78584567249ce2796df97df6606aa6fb19a982029ed55a2dc3c",
    "splitk4/clippy-default/inner.json": "24d801305e77f039a9c07aa5ec7f3f7d1660ead97aa15dd1909497ffcbda569b",
    "splitk4/clippy-default/outer.json": "898c9b1eb61369508fb4443b10fa9c4e8be09b11848d656d3e1f6cb51bddefd8",
    "splitk4/clippy-enabled/inner.json": "17c8ac6587eb01bc1850e066f8ea660887bf23c69a3ad1394925ec911b00ba35",
    "splitk4/clippy-enabled/outer.json": "6e0e95977d73c48b3e9dc0cbbe357b3705cfff91599c94a90b4956f869cbe2ae",
    "splitk4/emit/inner.json": "15259fcb41504af3f7084bb996b11093a42e859006fb737a07c68a495a56676e",
    "splitk4/emit/outer.json": "69a26c7cf879dfae6403571652a5f20f3e9e9c6fcfca52cb3b4d66c773860328",
    "splitk4/inspect/outer.json": "9c9d312f561dd7da72ab22b8ad3a6a47bc88050f3ad33fd26526a82ed0b67023",
    "splitk4/isa/disassembly.txt": "678c4b3028580426a096e866d7e9b392ea5c4e2bbcb4793eef07da433caeb9ac",
    "splitk4/isa/metadata.txt": "1f008684b3ce8d9515868a6f2179c855eb566ee3cabe1a17f26345d2bb9000e0",
    "splitk4/isa/receipt.json": "c827d0e0e711e07d3d695c7b8e85928377c6df9e36f5ca6f170d0891c91060ae",
    "splitk4/isa/rodata.txt": "3daf47469c702e1b240bbf954dfbafdbd72d8796a47b01e65fa6089136ca25b4",
    "splitk4/observation.json": "a7417322dafe8e0af927a6457f0dea2c324e94ec2692329f7d76a215ed3273bb",
    "splitk4/payload/compiler-handoff-v2": "ea8672a0acfcfef4606c6597f6b7f3af0fbbfcd9634d8feffe5d29d5f11b39b9",
    "splitk4/payload/pre-worker.ll": "3269ca9c0c9ea7ef77cf475cd2c25234eb9eea7ff8bc5e657657fff89a45afa5",
    "splitk4/payload/target-kir.txt": "255484a1ac03b14a630138e4b79e01abb8de5542c69684423c89e6901bf0e342",
    "splitk4/prepare/inner.json": "89f52f8a893997d1af76d0a428d4e3c718bbf849844826d0ebe78c6cc32a8251",
    "splitk4/prepare/outer.json": "c3c8b8baf3b44c2fcf7b1bbe86c533b292a3f9b9daa8995e11ec085ae48857a9",
    "splitk4/source/Cargo.lock": "09b83fed10fa66e6a541a9d8ff611a6778da144eff781a95be1723cedd5b9492",
    "splitk4/source/Cargo.toml": "9e7ed9fee9da923f607074d0bb3f2c3ff514477a20ffd4df8fc2bb396676a273",
    "splitk4/source/README.md": "764d903656c1a5c8111e7ea672c89f61aeea4bf483653feca5cf0d3532a4dcd3",
    "splitk4/source/build.rs": "fcfdd7e0d6ffd340231b532bd64e55b3861a302d1d8c6e7c0ccd8f4a840754ba",
    "splitk4/source/build/target_contract.rs": "d48a07c5e96b4908aedcd332fb470e00812fbeb51ca299fd655d6629a0e0b3ad",
    "splitk4/source/src/lib.rs": "84fe07f4b9220acbeb964a46a84b4ccf89939932b3138c01692b6061235c781a",
    "splitk4/source/src/projection.rs": "3a1d5fc1ee10a2162e45fa80f8278e17cfeca1099f045e21a53eeb064f70231e",
    "splitk4/source/tests/contract.rs": "d33a98b84c970b213410ddf5b2da42d1cc4643735d51bb1d3de9b0f1bd4b9608",
    "splitk4/source/tests/geometry.rs": "31bdc8ed351a94993905443e2706302f626397af4820fd6ed677b77f7f5178e3",
    "splitk4/test-default/inner.json": "6f2c9dee7a0d99e9a154572fdeb3046b69179d76afccf3730a6dc0abe10773bd",
    "splitk4/test-default/outer.json": "9ef8b93a4ab97fd61e0d3bef5c3c53966f0e45fe8346a1fe64994f91b272f81c",
    "splitk4/test-enabled/inner.json": "a34ceef4babb5ad1174e4ab31b803240a0a8570176aee1f9d970c74f7ee1212b",
    "splitk4/test-enabled/outer.json": "ae3f53e83dde9f42cfda6c379de6927fabf12a21b30fa30d420de26ab2460a4b",
    "splitk4/vendor/inner.json": "72b9797076d8da253560bb4689dfee48d97908705be0ba1e95d889668a90895f",
    "splitk4/vendor/outer.json": "caad76953273dfd8070a4365fcd53f78bdd0e37d1473f388ee7db1677a0a6492",
    "wave/observation.json": "c559d0533907323aff7dda03215adab6326423c3f151b296e22394c9baf37d00"
}
IMAGE_PINS = {
    'wave': '98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502',
    'splitk4': 'd28610d291eeec0589afbf269e26d21b7111c96f08106e1f661d6a66f024bf03',
}
SYMBOLS = {
    'wave': ('ferric_qwen3_tp_batch32_wave_gemv_bf16_v5',),
    'splitk4': ('ferric_qwen3_c1_gate_up_splitk4_mfma_partial_f32_r1',
               'ferric_qwen3_c1_gate_up_splitk4_merge_bf16_r1'),
}
SIZES = {'wave': 116056, 'splitk4': 13136}
HOST_ROLES = ('prepare', 'test-default', 'test-enabled', 'clippy-default', 'clippy-enabled')
PAYLOADS = {'compiler-handoff-v2': ('compiler_handoff', 54782),
            'target-kir.txt': ('kir', 592584), 'pre-worker.ll': ('llvm', 54033)}


def validate_documents(raw, plan, *, decode, require):
    observations = {}
    for arm in IMAGE_PINS:
        require(plan['images'][arm]['sha256'] == IMAGE_PINS[arm]
                and plan['files']['images/' + arm + '.hsaco']['bytes'] == SIZES[arm],
                'exact actual image identity')
        obs = decode(raw[arm + '/observation.json'])
        require(obs['schema'] == 'EngineeringHsacoObservationV1' and obs['authority'] == 'none'
                and obs['target'] == 'gfx950:xnack-' and obs['code_object_version'] == 6
                and obs['grants'] == {'publication': False, 'load': False, 'launch': False}
                and obs['hsaco']['identity'] == {'sha256': IMAGE_PINS[arm], 'byte_len': SIZES[arm]}
                and all(root in obs['hsaco']['kernel_names'] for root in SYMBOLS[arm])
                and obs['execution']['exact_output_replay'] is True,
                'engineering-only observation and selected roots')
        observations[arm] = obs
    obs = observations['splitk4']
    require(observations['wave']['tools'] != obs['tools'],
            'historical compiler lineages are not identical')
    require(obs['hsaco']['kernel_names'] == sorted(SYMBOLS['splitk4']), 'exact two-root candidate')
    capture = decode(raw['splitk4/capture.json'])
    require(capture['state'] == 'payload-complete'
            and capture['requires_cli_completion_acknowledgement'] is True
            and capture['observation'] == {'sha256': PINS['splitk4/observation.json'], 'byte_len': 2836}
            and capture['compiler_handoff'] == obs['compiler_handoff']
            and capture['grants'] == obs['grants']
            and capture['source_authentication'] is False and capture['proof_authority'] is False,
            'complete diagnostic payload without fabricated authority')
    payloads = {}
    for name, (field, size) in PAYLOADS.items():
        key = 'splitk4/payload/' + name
        expected = {'sha256': PINS[key], 'byte_len': size}
        require(capture[field] == expected and len(raw[key]) == size
                and hashlib.sha256(raw[key]).hexdigest() == PINS[key], 'complete bound diagnostic bytes')
        payloads[name] = expected
    inspection = decode(raw['splitk4/isa/receipt.json'])
    require(inspection['schema'] == 'FerricSplitK4GateUpIsaInspectionInputsV1'
            and inspection['accepted'] is True and inspection['abi_admitted'] is False
            and inspection['native_correctness'] is False and inspection['native_performance'] is False
            and inspection['capture_state'] == 'payload-complete' and inspection['payloads'] == payloads
            and inspection['image'] == obs['hsaco']['identity']
            and inspection['emission_inner_sha256'] == PINS['splitk4/emit/inner.json']
            and inspection['emission_outer_sha256'] == PINS['splitk4/emit/outer.json'],
            'inspection does not grant native authority')
    source = {name.removeprefix('splitk4/source/'): value for name, value in PINS.items()
              if name.startswith('splitk4/source/')}
    require(len(source) == 9 and inspection['source'] == source, 'exact nine-file emitted source')
    emission = decode(raw['splitk4/emit/inner.json'])
    require(emission['schema'] == 'FerricSplitK4GateUpStandardEmissionPhaseV1'
            and emission['accepted'] is True and emission['native_executed'] is False
            and emission['sdk_revision'] == '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
            and emission['source_before'] == emission['source_after'] == source
            and emission['compiler_provenance'] == '79a plus published private-access and witness-rank fixes'
            and emission['vendor_receipt_sha256'] == PINS['splitk4/vendor/inner.json']
            and emission['vendor_outer_sha256'] == PINS['splitk4/vendor/outer.json'],
            'actual source, SDK and historical compiler identity')
    for role in HOST_ROLES:
        host = decode(raw['splitk4/' + role + '/inner.json'])
        require(host['schema'] == 'FerricSplitK4GateUpHostQualificationV1'
                and host['accepted'] is True and host['role'] == role and host['native_executed'] is False
                and host['source_after'] == source
                and emission['cpu_receipts'][role] == [PINS['splitk4/' + role + '/inner.json'],
                                                      PINS['splitk4/' + role + '/outer.json']],
                'actual successful host qualification')
        if role == 'prepare':
            require(set(host['source_before']) == set(source) - {'Cargo.lock'},
                    'authored eight files before lock and formatting')
        else:
            require(host['source_before'] == source, 'unchanged qualified source')
    clean = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
             'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
             'log_limit_exceeded': False}
    for role in (*HOST_ROLES, 'vendor', 'emit', 'inspect'):
        outer = decode(raw['splitk4/' + role + '/outer.json'])
        require(all(type(outer.get(key)) is type(value) and outer[key] == value
                    for key, value in clean.items())
                and outer['profile'] == 'FerricCpuFourCore40GiBEmitterV1'
                and outer['cpus'] == [0, 1, 2, 3] and outer['nice'] == 19
                and outer['build_jobs'] == 4 and outer['rust_test_threads'] == 1,
                'clean guarded historical G40 closure')


def validate_review(evidence, plan, *, read, staged_binding, decode, require):
    require(type(evidence) is dict and set(evidence) == set(PINS), 'closed exact image provenance')
    raw = {}
    for name, expected in PINS.items():
        item = staged_binding(evidence[name], plan)
        require(item['sha256'] == expected and item['path'] == plan['stage'] + '/provenance/' + name,
                'fixed retained provenance location')
        raw[name] = read(item['path'], expected, 8 * 1024**2)[0]
        require(hashlib.sha256(raw[name]).hexdigest() == expected, 'actual retained provenance bytes')
    validate_documents(raw, plan, decode=decode, require=require)
