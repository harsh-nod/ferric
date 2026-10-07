"""One exact two-root prefill image and its engineering-only qualification."""
import hashlib
import re

PINS = {
    "observation.json": "ce03702de1223650f225b1f08e0f134b5228b493f4554319535c800c11d1f71d",
    "capture.json": "8dac8f5b2bb92809e59b243d5ff45d4263ef16c14c56f3fa149250c08dd837e3",
    "isa/receipt.json": "9fc4badc701db66e429fe7a8e1956a79615aa43837d16f1429ce137c0637dbae",
    "isa/disassembly.txt": "db6cda662baad7447ebc2ce17359298c5d95c5591c6feca183485d6306238fb1",
    "isa/metadata.txt": "df2965193fc114b91b4b8012911dcb1996b87c9e960a62398d136ee6799dfbee",
    "isa/rodata.txt": "31d6574580a91d50bec8b2bdd1b9adc783b2f912adefdb8221c2c187f5703146",
    "emit/inner.json": "3bfc31a8fbb5db418ab56be90b86d709a0c1f2f3a4417baec7734142dfa10118",
    "emit/outer.json": "79c0b7e11563019cc3e012a099cc92f6dfa51e0f0bf2bc7e5b6cbc8cb18df8bb",
    "vendor/inner.json": "113110d93344f0898fcd657abac5f2aae0913c8b5048e31189d1497d66691335",
    "vendor/outer.json": "1e0df12c7191c4214cdcdf82e10fda16695bff2c64c2e93cd22ef5f0b9166588",
    "inspect/outer.json": "9cf2ec05c0d1a7a1a8ca0a6ae993b6b8704546dbba22c15bce4b11d7910854ea",
    "source-frozen.json": "e015e034bafdcc7f268c3b1e81f5936dd5f33ab2534299f3f366f1bf115953a5",
    "prepare/inner.json": "9a44de6794866a8ed9474da2d138900e029daf3e45e471d07c2e9364e75acae7",
    "prepare/outer.json": "08c5557a5456cd34aeb40db650a79b1f18b777fe6b104903a5c88f8a6e4a3634",
    "test-default/inner.json": "b64040f5921aeca865ea0da613f37d14bdf4a5d65e18e7b9cd6f5436b28fb820",
    "test-default/outer.json": "9bc86aae98357702621d349c307768b9b6950e067e2205d51469deef8982afdb",
    "test-default/stdout": "b1c95504c4f83b294cf7e78988a678675d9290a0269f3456baaa88c1ae6961ae",
    "test-enabled/inner.json": "0703186fb7cdd4b36f5244f64edab27741ee1ff6f3191443d2335b8c850140c1",
    "test-enabled/outer.json": "b7b1a74e2b65d517299c030f14cffd8f7968aa0d05f1229199de7a63259e9f2b",
    "test-enabled/stdout": "7d393db58ae83cfa11784900d93400e1863bf52a36c766cceb18b84fa7bc1153",
    "clippy-default/inner.json": "d980aadca4bfc934bf055cd2b36934935f4d3a88166a22343cc2cf74221a250a",
    "clippy-default/outer.json": "446df25f9b06f1dc7bfdfe914416fdd432634131871e49be474b950951a2f0f1",
    "clippy-enabled/inner.json": "2fbccbbef23b68df00f5a0ffa454549415fb53fab4619052241ae4eb9cc276b3",
    "clippy-enabled/outer.json": "1307ae2b2ecbaa8ba4fad56cb54dbe1c79e2de43644d65fe424fc7565db98458",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/Cargo.lock": "242427e9c45a14221556e249776f70653c8c475b0ec1d9c98ab04e0510a7b3e8",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/Cargo.toml": "dc1809d3b2de9454ffe206a1716381b0e256b96322eb2b032728b56dceb68d1f",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/README.md": "0bc64174f1cebae9f453712c6d8f68e12319526f67e05682eba42494d6fe393a",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/build.rs": "877738a864edbf849c7e4ba6c092ca9fcc4451a93e0569b8cff8eb1075cbf11e",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/build/target_contract.rs": "d48a07c5e96b4908aedcd332fb470e00812fbeb51ca299fd655d6629a0e0b3ad",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/src/lib.rs": "739dc903843557a220643802b8219bd0ace4b33bc78f561452e8c0fd0a92ac58",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/src/projection.rs": "621b536a073a4860eb6ce8f33499c5669cce0527afad5a744494ef2c747f0928",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/tests/fixtures/control.rs.txt": "7ff47dfa14a2dc1916b71081382d65b2e4b91d119071f4db304a4a751cecd5a6",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/tests/fixtures/paired.rs.txt": "06bbdb5ba89e6b2e01ffbc04ad4b9726a96ba2df72ea3f439efef331c4ce54ff",
    "source/device/qwen3-tp-prefill-k2-kernels-v1/tests/source_contract.rs": "e12bb94a91bbe2ea6457b1bfaba0b1eb23891b1fbaf0a1723240be81d770d0fc"
}
IMAGE_SHA = 'fdbf5f1999e709b695b2ef4399bfc44920df1a0b85a61c5fe1bbc41e719e5357'
IMAGE_PINS = {'control': IMAGE_SHA, 'paired': IMAGE_SHA}
SYMBOLS = {'control': 'ferric_qwen3_prefill_k16_control_bf16_r1',
           'paired': 'ferric_qwen3_prefill_k16_paired_bf16_r1'}
SIZES = {'control': 23272, 'paired': 23272}
HOST_ROLES = ('prepare', 'test-default', 'test-enabled', 'clippy-default', 'clippy-enabled')
HOST_HELPER_SHA = '17a421d5fc0a94955d1b122b3c2532ad19ea3f310da0afd76eb9476cf450e9fb'
EMITTER_HELPER_SHA = 'bbb8e1bdbf4667e0ce9df1cf8f53bb005a018339fa13cfa469b6032372d2381f'
INSPECTOR_HELPER_SHA = '594966de66ea1864a26dfea8f504621e9b1edb0aaf8e1487305ba2ffb76066f3'
REVISION = '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'


def validate_documents(raw, plan, *, decode, require):
    for arm in SYMBOLS:
        require(plan['images'][arm]['sha256'] == IMAGE_SHA
                and plan['files']['images/' + arm + '.hsaco']['bytes'] == SIZES[arm],
                'same exact two-root image for both arms')
    obs = decode(raw['observation.json'])
    no_grants = {'publication': False, 'load': False, 'launch': False}
    require(obs['schema'] == 'EngineeringHsacoObservationV1' and obs['authority'] == 'none'
            and obs['namespace'] == 'fe2o3-engineering-v1'
            and obs['crate_name'] == 'ferric_qwen3_tp_prefill_k2_kernels_device_v1'
            and obs['target'] == 'gfx950:xnack-' and obs['code_object_version'] == 6
            and obs['grants'] == no_grants and obs['providers'] == []
            and obs['hsaco']['identity'] == {'sha256': IMAGE_SHA, 'byte_len': 23272}
            and obs['hsaco']['kernel_names'] == sorted(SYMBOLS.values())
            and obs['execution']['exact_output_replay'] is True
            and obs['options'] == {'optimization': 'O2', 'strip_debug': True, 'verify_each': True,
                                  'timeout_seconds': 600, 'maximum_output_bytes': 4194304},
            'one exact compiler-matched engineering observation, not native authority')
    require(obs['compiler_handoff'] == {
                'sha256': 'f52669142a51851087a699f393786da5ca3f0ca574a225cfcb9a91990e31ccb4',
                'byte_len': 128408}
            and obs['tools']['extractor_backend']['sha256'] ==
                '94d663de7150d25e3449778df1618e5b274e5d1a8b981c47ad01c7d7dd9a51bf'
            and obs['tools']['worker']['executable']['sha256'] ==
                'c6c92db6158bdab5a87f46c5a7b08907d51ded378babd88dc2257c0852415fe1',
            'actual shared compiler handoff and toolchain')
    capture = decode(raw['capture.json'])
    require(capture['schema'] == 'EngineeringDiagnosticCaptureV1'
            and capture['state'] == 'omitted-ineligible' and capture['kir'] is None and capture['llvm'] is None
            and capture['observation'] == {'sha256': PINS['observation.json'], 'byte_len': 2815}
            and capture['compiler_handoff'] == obs['compiler_handoff']
            and capture['grants'] == no_grants
            and capture['source_authentication'] is False and capture['proof_authority'] is False,
            'actual omitted capture is not a fabricated proof payload')
    source = {name.removeprefix('source/'): digest for name, digest in PINS.items()
              if name.startswith('source/')}
    frozen = decode(raw['source-frozen.json'])
    require(len(source) == 10 and frozen['formatted_locked'] == source
            and frozen['source_archive_sha256'] ==
                'fdf979240b5a5963e315fd293205c59656b1395a70669620324dfb5040ce4387',
            'full prefixed ten-file frozen source')
    inspection = decode(raw['isa/receipt.json'])
    require(inspection['schema'] == 'FerricMatchedK16PrefillIsaInspectionInputsV1'
            and inspection['accepted'] is True and inspection['abi_admitted'] is False
            and inspection['native_correctness'] is False and inspection['native_performance'] is False
            and inspection['helper_sha256'] == INSPECTOR_HELPER_SHA
            and inspection['capture_state'] == 'omitted-ineligible' and inspection['payloads'] == {}
            and inspection['source'] == source and inspection['image'] == obs['hsaco']['identity']
            and inspection['observation'] == capture['observation']
            and inspection['emission_inner_sha256'] == PINS['emit/inner.json']
            and inspection['emission_outer_sha256'] == PINS['emit/outer.json']
            and all(inspection['outputs'][name] == PINS['isa/' + name]
                    for name in ('disassembly.txt', 'metadata.txt', 'rodata.txt')),
            'retained actual inspection is not native performance')
    emission = decode(raw['emit/inner.json'])
    require(emission['schema'] == 'FerricPrefillK2CurrentStandardEmissionPhaseV1'
            and emission['accepted'] is True and emission['role'] == 'emit'
            and emission['helper_sha256'] == EMITTER_HELPER_SHA and emission['native_executed'] is False
            and emission['sdk_revision'] == REVISION
            and emission['source_before'] == emission['source_after'] == source
            and emission['compiler_provenance'] == '79a plus published private-access and witness-rank fixes'
            and emission['vendor_receipt_sha256'] == PINS['vendor/inner.json']
            and emission['vendor_outer_sha256'] == PINS['vendor/outer.json'],
            'successful standard emission lineage; not a current-main compiler rebuild')
    vendor = decode(raw['vendor/inner.json'])
    require(vendor['accepted'] is True and vendor['role'] == 'vendor'
            and vendor['helper_sha256'] == EMITTER_HELPER_SHA
            and vendor['native_executed'] is False
            and vendor['source_before'] == vendor['source_after'] == source,
            'same source qualified vendor closure')
    for role in HOST_ROLES:
        host = decode(raw[role + '/inner.json'])
        require(host['schema'] == 'FerricMatchedK16PrefillHostQualificationV1'
                and host['accepted'] is True and host['role'] == role
                and host['helper_sha256'] == HOST_HELPER_SHA and host['native_executed'] is False
                and host['source_after'] == source
                and host['source_before'] == (frozen['authored'] if role == 'prepare' else source)
                and emission['cpu_receipts'][role] ==
                    [PINS[role + '/inner.json'], PINS[role + '/outer.json']],
                'actual five-role qualification; only prepare may format and lock')
    for role, count in (('test-default', 7), ('test-enabled', 8)):
        totals = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',
                            raw[role + '/stdout'].decode('utf-8'))
        require(totals and sum(int(row[0]) for row in totals) == count
                and all(row[1:] == ('0', '0') for row in totals), 'actual test counts without skips')
    clean = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
             'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
             'log_limit_exceeded': False}
    for role in (*HOST_ROLES, 'vendor', 'emit', 'inspect'):
        outer = decode(raw[role + '/outer.json'])
        require(all(type(outer.get(key)) is type(value) and outer[key] == value
                    for key, value in clean.items())
                and outer['profile'] == 'FerricCpuFourCore40GiBEmitterV1'
                and outer['cpus'] == [0, 1, 2, 3] and outer['nice'] == 19
                and outer['build_jobs'] == 4 and outer['rust_test_threads'] == 1,
                'all eight CPU phases clean and unsignaled')


def validate_review(evidence, plan, *, read, staged_binding, decode, require):
    require(type(evidence) is dict and set(evidence) == set(PINS), 'closed matched image provenance')
    raw = {}
    for name, expected in PINS.items():
        item = staged_binding(evidence[name], plan)
        require(item['sha256'] == expected and item['path'] == plan['stage'] + '/provenance/' + name,
                'fixed exact retained provenance location')
        raw[name] = read(item['path'], expected, 8 * 1024**2)[0]
        require(hashlib.sha256(raw[name]).hexdigest() == expected, 'actual retained provenance bytes')
    validate_documents(raw, plan, decode=decode, require=require)
