"""Exact independent historical V5/M32 evidence, without proof or native authority."""
import hashlib

PINS = {
    "v5/observation.json": "c559d0533907323aff7dda03215adab6326423c3f151b296e22394c9baf37d00",
    "m2/observation.json": "1ccccf9b64c94df6d069e1e1034bc766ce27e394fff7e2a61b58fae70153ed7e",
    "m2/capture.json": "735a9b83ede81b3fee7a02f892ea76495e9b6aeb54e5f611d83b4a5c75f91351",
    "m2/isa/receipt.json": "f043828fa47271324337c180a07ce22ee60bedaefadf0d4e256e178331958e43",
    "m2/isa/disassembly.txt": "96b4e9ebb1e4e98889e23b2b57c9b01e98f23ba05823b6e109130e5cf235ced0",
    "m2/isa/metadata.txt": "db2dab54e7bc8ce035fb7eb22fefe3f3f243040302c1d2538e60f78bf9df4308",
    "m2/isa/rodata.txt": "598f0de138f3ed6e7d46c37c682e07fa5ae32147c6af9b86ca0da4f11fd300df",
    "m2/emit/inner.json": "75fcff89c3a49e8cfb49e7c855f7a403079c0a073a6cbe0056c32e2c35a2372c",
    "m2/emit/outer.json": "1a279c642e366459ebbae01df0c77435832d8b1eeb2b791c369cb8b7012af8bf",
    "m2/inspect/outer.json": "60b2afe0f920b46df2d91cf8b14e54a47d2d06b5ff879a8363656313c86f3186",
    "m2/test-default/inner.json": "017bcb536bea42dc098f307535b14cc625f077f34780e6761516987882964e79",
    "m2/test-default/outer.json": "fc132ad09239e65e20c0de86efb11d5061abfdc5a5f5c87a6251c035335a90fc",
    "m2/test-enabled/inner.json": "ad5f37704396984d1f14c3c662eac53934a1408693c82588969a288427c70cfc",
    "m2/test-enabled/outer.json": "825381be6b9cb733cb7a36a83c5a5f4fb654f0ff75e357e580908a64c04ce480",
    "m2/clippy-default/inner.json": "4e0cc00be4338e797975fd574234edc74dbe1eb219a666e8ebec0145307ce3f3",
    "m2/clippy-default/outer.json": "cb6b61e10e5254a73b400a3cc07322ec871ad8a1e534db254d3aea35434cd6f7",
    "m2/clippy-enabled/inner.json": "f860f18b6d2e8da11b6cfcff68b599f44eed235c9be825699d55888e16570ad2",
    "m2/clippy-enabled/outer.json": "33317852af6b2df35fe4efae628edabf3a761e705852d12b22d5885fea26134b",
    "m2/source/Cargo.lock": "8702ac46691f8a94bd2d42c8431b5450f34e2714d858d03c09beca503e568463",
    "m2/source/Cargo.toml": "db5cdcc54ee308756ca04ddc47d5af4d5ac7dd1f86d986b147492cf93ca26af7",
    "m2/source/README.md": "aba42eeea546216b0dfa0f2afe838d89249346f14326b21de6cb8fe71600283e",
    "m2/source/build.rs": "744c09fda45f3c4429b1235b61e050298d832bb5d703f9b76609beba8144e708",
    "m2/source/build/target_contract.rs": "d48a07c5e96b4908aedcd332fb470e00812fbeb51ca299fd655d6629a0e0b3ad",
    "m2/source/src/contract.rs": "16358da0d532f46f6af81c73e2f60ce98b8062802ab7b505ee2941ea16f3d0da",
    "m2/source/src/lib.rs": "4e7749163f45b0d948de760bd8c956ecf080b3e2d5f755853da5cccc582fe970",
    "m2/source/src/projection.rs": "ed30b3a724317b1014c497001e4f717a3efbb05292d0b6e8cad52c6f7240d14f",
    "m2/source/tests/host_contract.rs": "c4a7690cae4f9e051e86a65437a3da953684536358488c459ba2bf0a0fedce72",
    "m2/source/tests/source_contract.rs": "9d472a22ac15bb76820e065752d37cd1b36e5222df029b8c5b56850bffbf8a39"
}
IMAGE_PINS = {
    'v5': '98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502',
    'm2': 'bb164d23d55dec43e55b8d8cce936929a3cc749a29486bdcfff8bfe34ac7d5dc',
}
SYMBOLS = {'v5': 'ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5',
           'm2': 'ferric_qwen3_prefill32_m2_gate_up_bf16_r1'}
SIZES = {'v5': 116056, 'm2': 10968}
HOST_ROLES = ('test-default', 'test-enabled', 'clippy-default', 'clippy-enabled')


def validate_documents(raw, plan, *, decode, require):
    observations = {}
    for arm in ('v5', 'm2'):
        item = plan['images'][arm]
        require(item['sha256'] == IMAGE_PINS[arm]
                and plan['files']['images/' + arm + '.hsaco']['bytes'] == SIZES[arm],
                'exact actual BF16 control and M32 image identity')
        obs = decode(raw[arm + '/observation.json'])
        require(obs['schema'] == 'EngineeringHsacoObservationV1' and obs['authority'] == 'none'
                and obs['target'] == 'gfx950:xnack-' and obs['code_object_version'] == 6
                and obs['grants'] == {'publication': False, 'load': False, 'launch': False}
                and obs['hsaco']['identity'] == {'sha256': IMAGE_PINS[arm], 'byte_len': SIZES[arm]}
                and SYMBOLS[arm] in obs['hsaco']['kernel_names']
                and obs['execution']['exact_output_replay'] is True,
                'exact engineering-only observation and selected root')
        observations[arm] = obs
    require(observations['v5']['tools'] != observations['m2']['tools'],
            'different historical toolchains must not be called compiler-matched')
    require(observations['m2']['hsaco']['kernel_names'] == [SYMBOLS['m2']], 'one-root candidate')
    capture = decode(raw['m2/capture.json'])
    require(capture['state'] == 'omitted-ineligible' and capture['kir'] is None and capture['llvm'] is None
            and capture['observation'] == {'sha256': PINS['m2/observation.json'], 'byte_len': 2781}
            and capture['compiler_handoff'] == observations['m2']['compiler_handoff']
            and capture['grants'] == observations['m2']['grants']
            and capture['source_authentication'] is False and capture['proof_authority'] is False,
            'no fabricated diagnostic payload or authority')
    inspection = decode(raw['m2/isa/receipt.json'])
    require(inspection['schema'] == 'FerricM32IsaInspectionInputsV1' and inspection['accepted'] is True
            and inspection['abi_admitted'] is False and inspection['native_correctness'] is False
            and inspection['native_performance'] is False and inspection['capture_state'] == 'omitted-ineligible'
            and inspection['payloads'] == {}
            and inspection['image'] == observations['m2']['hsaco']['identity']
            and inspection['emission_inner_sha256'] == PINS['m2/emit/inner.json']
            and inspection['emission_outer_sha256'] == PINS['m2/emit/outer.json'],
            'inspection is not native admission or performance')
    source = {name.removeprefix('m2/source/'): digest for name, digest in PINS.items()
              if name.startswith('m2/source/')}
    require(len(source) == 10 and inspection['source'] == source, 'frozen ten-file emitted source')
    emission = decode(raw['m2/emit/inner.json'])
    require(emission['schema'] == 'FerricM32StandardEmissionPhaseV1' and emission['accepted'] is True
            and emission['native_executed'] is False
            and emission['sdk_revision'] == '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
            and emission['source_before'] == emission['source_after'] == source
            and emission['compiler_provenance'] == '79a plus published private-access and witness-rank fixes',
            'actual emission lineage, not a current-main compiler rebuild')
    for role in HOST_ROLES:
        host = decode(raw['m2/' + role + '/inner.json'])
        require(host['schema'] == 'FerricM32HostQualificationV1' and host['accepted'] is True
                and host['role'] == role and host['native_executed'] is False
                and host['cold_attempt_accepted'] is False and host['warm_followup'] is True
                and host['source_before'] == host['source_after'] == source
                and emission['cpu_receipts'][role + '-a002'] ==
                    [PINS['m2/' + role + '/inner.json'], PINS['m2/' + role + '/outer.json']],
                'unchanged accepted warm host qualification; cold failure remains rejected')
    clean = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
             'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
             'log_limit_exceeded': False}
    for role in (*HOST_ROLES, 'emit', 'inspect'):
        outer = decode(raw['m2/' + role + '/outer.json'])
        require(all(type(outer.get(key)) is type(value) and outer[key] == value
                    for key, value in clean.items())
                and outer['profile'] == 'FerricCpuFourCore40GiBEmitterV1'
                and outer['cpus'] == [0, 1, 2, 3] and outer['nice'] == 19
                and outer['build_jobs'] == 4 and outer['rust_test_threads'] == 1,
                'clean actual historical G40 outer receipt')


def validate_review(evidence, plan, *, read, staged_binding, decode, require):
    require(type(evidence) is dict and set(evidence) == set(PINS), 'closed independent image provenance')
    raw = {}
    for name, expected in PINS.items():
        item = staged_binding(evidence[name], plan)
        require(item['sha256'] == expected and item['path'] == plan['stage'] + '/provenance/' + name,
                'fixed exact retained provenance location')
        raw[name] = read(item['path'], expected, 8 * 1024**2)[0]
        require(hashlib.sha256(raw[name]).hexdigest() == expected, 'actual retained provenance bytes')
    validate_documents(raw, plan, decode=decode, require=require)
