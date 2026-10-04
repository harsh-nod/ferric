"""No-launch preparation from explicit new compiler/image/native evidence."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import time
import types

import validation as V

HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
PACKAGE = 'p228-independent-gpu-observation-v1'
DEPLOYMENT_PACKAGE = 'p228-independent-deployment-v2'
DEPLOYMENT_PACKAGE_SHA = '63ff8c1820c12f9863773d77d5a5652a4e02c21340b32128455be292ab95a0fa'
DEPLOYMENT_READER_SHA = '0901e10dbcf40e940a98f917cc6bf7b74c4c5c5f8b76eb1af492e672a11fab00'
DEPLOYMENT_PURE_TESTS = 30
DEPLOYMENT_TEST_RUNNER_SHA = '42294bc6d26dc40c6e07d52d50246345c89c7cb4051043007ca16f06fadd4558'
VALIDATION_SHA = 'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1'
OBSERVE_SHA = '3ed69ac9af12d0442b0d7c6acef8f6081c923dd410de934df6fbf64784441ec6'
CHILD_SHA = '2cbb74ada0950d1767c8b526008e9d9c7efb48d84649e1aea8f3d6661309edd3'
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
TEST_RUNNER_SHA = 'ea64c61c58cd0e86f7a23f36a17b2ec32673aefbad971c3c8f65960a00a3948a'
PURE_TESTS = 81
MEMBERS = {'README.md', 'baseline-inputs.json', 'validation.py', 'prepare.py', 'run_case.py',
           'test_validation.py', 'test_run.py', 'test_prepare.py', 'run_row_facts_v2.py',
           'frozen_owned.py', 'observe.py', 'test_observe.py', 'preimage/prepare.py', 'preimage/run_case.py',
           'fixtures/actual-readelf.stdout', 'fixtures/actual-ldd.stdout', 'child_evidence.py', 'test_children.py'}
BASELINE_INPUTS_SHA = 'fee39b8bf6343d27044ef2564366b842c1c5f23a251627b89794e1df42fb425f'
TOPOLOGY_SHA = '6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a'
HOST = 'smci350-rck-g03-b19-03'
BINARY_NAME = 'gfx950-qwen-prefix-tiles-comparison-v6'
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
           OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
           HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
BASELINE_SHA = (
    'a8cc62ed04874019ef18f469d5362d8402ae68e039c0d6364df57decb0bf8b52',
    'dff913ec0926bccd977248d87456e4917094f0e93dfbf593da82e92f1eee9976',
    '1c6a41d39ecaf0eae064d0008ddc4e69f3eae7fe60922d6e0a7199248999772a',
    'f91571185c0f914a37106ad6661b27d59e9e3e345a478766f7a4758f847c1f8e',
    'b16cc3593367a51a1235597652ced0ada53135c1603cde96d4a407ecc1e8fe7e',
    'b3e69ae3bf24b0c9307d6d865cdf98e3e83c6d76b0001cb021b14ad137ae4bbd',
)


def bootstrap():
    path = Path(__file__).resolve().with_name('run_row_facts_v2.py')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    V.require(stat.S_ISREG(before.st_mode) and len(raw) <= 1 << 20
        and path.resolve(strict=True) == path and stamp(before) == stamp(after) == stamp(path.lstat())
        and len(raw) == before.st_size and hashlib.sha256(raw).hexdigest() == HELPER_SHA,
        'exact retained custody helper')
    module = types.ModuleType('prefix_parity_custody'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


D = bootstrap()
E, R = D.E, D.E.parents[1]


def read(pins, record, retain=False, maximum=64 << 20):
    V.pin(record, maximum, nonempty=False)
    actual, raw = pins.read(Path(record['path']), record['sha256'], retain, maximum)
    V.require(actual == record, 'exact retained input bytes')
    return raw


def document(pins, record, maximum=1 << 20):
    return D.parse(read(pins, record, True, maximum))


def save(path, value):
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    V.require(len(raw) <= 8 << 20, 'bounded JSON output')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return D.read_file(path, maximum=8 << 20)[0]


def input_shape(value):
    V.keys(value, 'schema deployment deployment_tests deployment_test_source artifact_review platform_review runtime_review '
                 'observer_manifest observer_tests observer_test_sources matrix_label cases topology_helper')
    V.require(value['schema'] == 'ferric-p228-independent-gpu-inputs-v1', 'preparation input schema')
    for key in ('deployment', 'deployment_tests', 'deployment_test_source', 'artifact_review', 'platform_review', 'runtime_review',
                'observer_manifest', 'observer_tests', 'observer_test_sources', 'topology_helper'):
        V.pin(value[key], 16 << 20)
    V.require(type(value['matrix_label']) is str
        and re.fullmatch(r'prefix-independent-profile-gpu-v228-v[1-9][0-9]{0,8}', value['matrix_label']), 'fresh matrix label')
    V.require(type(value['cases']) is list and len(value['cases']) == 6, 'six exact cases')
    for index, (row, case) in enumerate(zip(value['cases'], V.CASES)):
        V.keys(row, 'case baseline_request reviews')
        V.require(row['case'] == case and row['baseline_request']['sha256'] == BASELINE_SHA[index],
                  'unchanged actual six paired requests')
        V.pin(row['baseline_request'], 64 << 10)
        V.require(row['baseline_request']['path'] == str(R / 'evidence/resident-output-tp2-v217'
            / 'prepare-mi350-remaining-v1' / case / 'request.json'), 'original paired request location')
        V.require(type(row['reviews']) is list and len(row['reviews']) == 6, 'six actual scoped reviews')
        for record in row['reviews']: V.pin(record, 64 << 10)
    V.require(value['topology_helper']['path'] == str(R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py')
        and value['topology_helper']['sha256'] == TOPOLOGY_SHA, 'reviewed paired topology helper')


def package(pins, record):
    V.require(type(PURE_TESTS) is int and PURE_TESTS > 0, 'observer test roster not frozen')
    V.require(record['path'] == str(Path(__file__).resolve().with_name('manifest.json')),
              'this immutable observer package')
    value = document(pins, record)
    rows = value['files']
    V.require(type(rows) is list and len(rows) == len(MEMBERS)
        and {row['path'] for row in rows} == MEMBERS and value['pure_tests'] == PURE_TESTS
        and value['schema'] == 'ferric-p228-independent-gpu-observation-contracts-v1',
        'closed observer member/test roster')
    base = Path(__file__).resolve().parent
    for row in rows:
        actual = pins.pin(base / row['path'], row['sha256'])
        V.require(actual['bytes'] == row['bytes'], 'observer member extent')
    for name, digest in (('observe.py', OBSERVE_SHA), ('child_evidence.py', CHILD_SHA),
            ('validation.py', VALIDATION_SHA), ('frozen_owned.py', OWNED_SHA),
            ('run_row_facts_v2.py', HELPER_SHA)):
        pins.pin(base / name, digest)


def pure_tests(value, inputs, pins):
    V.keys(value, 'passed tests manifest_sha256 source_sha256 controller_sha256 gpu_execution numerical_acceptance')
    V.require(type(PURE_TESTS) is int and PURE_TESTS > 0
        and value['passed'] is True and type(value['tests']) is int and value['tests'] == PURE_TESTS
        and value['manifest_sha256'] == inputs['observer_manifest']['sha256']
        and value['controller_sha256'] == TEST_RUNNER_SHA
        and value['gpu_execution'] is False and value['numerical_acceptance'] is False,
        'actual observer pure-test prerequisite, not source/GPU qualification')
    complete, sources = inputs['observer_tests'], inputs['observer_test_sources']
    directory = Path(complete['path']).parent
    V.require(directory.parent == E and Path(complete['path']).name == 'complete.json'
        and sources['path'] == str(directory / 'sources-before.json')
        and value['source_sha256'] == sources['sha256'], 'actual packing snapshot identity')
    read(pins, sources, maximum=16 << 20)


def deployment_context(inputs, pins):
    V.require(DEPLOYMENT_PACKAGE_SHA is not None and DEPLOYMENT_READER_SHA is not None
        and DEPLOYMENT_TEST_RUNNER_SHA is not None
        and type(DEPLOYMENT_PURE_TESTS) is int and DEPLOYMENT_PURE_TESTS > 0,
        'new compiler/native deployment reader is not frozen; no preparation or launch')
    V.digest(DEPLOYMENT_PACKAGE_SHA); V.digest(DEPLOYMENT_READER_SHA); V.digest(DEPLOYMENT_TEST_RUNNER_SHA)
    base = D.package(pins, DEPLOYMENT_PACKAGE, DEPLOYMENT_PACKAGE_SHA)
    pure = document(pins, inputs['deployment_tests'])
    V.keys(pure, 'schema passed tests package_manifest controller test_source transcript gpu_execution numerical_acceptance')
    V.require(pure['schema'] == 'ferric-p228-independent-deployment-pure-v1'
        and pure['passed'] is True and type(pure['tests']) is int
        and pure['tests'] == DEPLOYMENT_PURE_TESTS
        and pure['package_manifest'] == pins.pin(base / 'manifest.json', DEPLOYMENT_PACKAGE_SHA)
        and pure['test_source'] == inputs['deployment_test_source'] == pins.pin(base / 'test_portable.py')
        and pure['controller']['sha256'] == DEPLOYMENT_TEST_RUNNER_SHA and pure['gpu_execution'] is False
        and pure['numerical_acceptance'] is False, 'actual deployment-reader pure qualification')
    pure_directory = Path(inputs['deployment_tests']['path']).parent
    V.require(pure_directory.parent == E and Path(inputs['deployment_tests']['path']).name == 'complete.json'
        and pure['transcript']['path'] == str(pure_directory / 'tests.log'),
        'deployment pure original transcript identity')
    for name in ('package_manifest', 'controller', 'test_source', 'transcript'):
        read(pins, pure[name], maximum=8 << 20)
    value = document(pins, inputs['deployment'], 16 << 20)
    path = Path(inputs['deployment']['path'])
    V.require(path.name == 'deployment.json' and path.parent.parent == E
        and re.fullmatch(r'prefix-independent-deployment-v228-v[1-9][0-9]{0,8}', path.parent.name),
        'exact transported deployment location')
    reader = D.load_module(pins, base / 'portable.py', DEPLOYMENT_READER_SHA,
                           'independent_profile_deployment')
    return reader.verify(D, pins, value, path.parent)


def make_request(row, metadata, image, capture_directory):
    return dict(schema='fe2o3-qwen-prefix-tiles-comparison-request-v6',
        baseline_request=row['baseline_request'],
        tiles=dict(object=image, descriptor_sha256=metadata['descriptor_sha256'],
            canonical_code_object_digest=metadata['canonical_code_object_digest'], entry_symbol=V.SYMBOL,
            descriptor_symbol=bytes.fromhex(metadata['descriptor_symbol_hex']).decode('utf-8')),
        reviews=row['reviews'], timeout_ms=10000, capture_directory=str(capture_directory))


def reviewed_artifact(value, inputs, deployment, verified):
    V.keys(value, 'schema decision deployment compiler_complete compiler_owner original_object '
                 'candidate_cpu_receipt native_complete native_owner original_binary native_overlay '
                 'binary object descriptor_sha256 canonical_code_object_digest '
                 'entry_symbol workgroup grid lds_bytes state_words cases production_authority '
                 'runtime_premises_discharged independent_numerical_acceptance performance_claim '
                 'notes')
    V.require(value['schema'] == 'ferric-p228-independent-gpu-artifact-review-v1'
        and value['decision'] == 'reviewed-engineering-six-case-independent-profiles'
        and value['deployment'] == inputs['deployment']
        and value['compiler_complete'] == verified['compiler_receipt']
        and value['compiler_owner'] == verified['compiler_owner']
        and value['original_object'] == verified['original_image']
        and value['candidate_cpu_receipt'] == verified['candidate_cpu_receipt']
        and value['native_complete'] == verified['native_receipt']
        and value['native_owner'] == verified['native_owner']
        and value['original_binary'] == verified['original_binary']
        and value['native_overlay'] == verified['native_overlay']
        and value['binary'] == verified['binary'] and value['object'] == verified['image']
        and value['cases'] == inputs['cases'], 'actual source/image/scoped operator review')
    for key in ('descriptor_sha256', 'canonical_code_object_digest'):
        V.require(value[key] == verified['metadata'][key], 'actual reviewed metadata')
    V.require(value['entry_symbol'] == V.SYMBOL and value['workgroup'] == [64, 1, 1]
        and value['grid'] == [4096, 1, 1] and type(value['lds_bytes']) is int and value['lds_bytes'] == 512
        and type(value['state_words']) is int and value['state_words'] == 284, 'exact V6 geometry')
    for key in ('production_authority', 'runtime_premises_discharged', 'independent_numerical_acceptance',
                'performance_claim'):
        V.require(value[key] is False, 'review is not a production/numerical/performance grant')
    V.require(type(value['notes']) is str and value['notes'].strip()
        and len(value['notes'].encode()) <= 16384, 'explicit bounded artifact review notes')


def runtime_review(value, binary, platform, pins):
    V.keys(value, 'schema authority reviewed host boot_id binary readelf ldd libraries notes '
                 'production_authority gpu_execution')
    V.require(value['schema'] == 'ferric-p227-prefix-parity-runtime-review-v1'
        and value['authority'] == 'none' and value['reviewed'] is True
        and value['host'] == platform['host'] and value['boot_id'] == platform['boot_id']
        and value['binary'] == binary and value['production_authority'] is False
        and value['gpu_execution'] is False, 'actual binary/runtime compatibility review')
    V.require(type(value['notes']) is str and value['notes'].strip()
              and len(value['notes'].encode()) <= 16384, 'substantive runtime compatibility notes')
    outputs = {}
    for name, argv in (('readelf', ['/usr/bin/readelf', '-d', binary['path']]),
            ('ldd', ['/usr/bin/ldd', binary['path']])):
        audit = document(pins, value[name], 64 << 10)
        V.keys(audit, 'argv exit_code deadline_seconds stdout stderr')
        V.require(audit['argv'] == argv and type(audit['exit_code']) is int and audit['exit_code'] == 0
            and type(audit['deadline_seconds']) is int and 0 < audit['deadline_seconds'] <= 30,
            'actual bounded runtime audit command')
        V.require(read(pins, audit['stderr'], True, 1 << 20) == b'', 'runtime audit empty stderr')
        outputs[name] = read(pins, audit['stdout'], True, 1 << 20).decode('utf-8')
    needed = re.findall(r'\(NEEDED\).*Shared library: \[([^\]\n]+)\]', outputs['readelf'])
    V.require(needed and len(needed) == len(set(needed)), 'actual dynamic ELF dependency list')
    libraries, seen, named, vdso = value['libraries'], set(), set(), 0
    V.require(type(libraries) is list and 1 <= len(libraries) <= 128, 'bounded resolved runtime libraries')
    for row in libraries:
        V.keys(row, 'path resolved')
        V.pin(row['resolved'], 128 << 20)
        path = Path(row['path'])
        V.require(path.is_absolute() and str(path) == row['path'] and '..' not in path.parts
            and row['path'] not in seen and path.resolve(strict=True) == Path(row['resolved']['path']),
            'canonical current resolved shared library')
        seen.add(row['path']); read(pins, row['resolved'], maximum=128 << 20)
    observed = set()
    for line in outputs['ldd'].splitlines():
        line = line.strip()
        if re.fullmatch(r'linux-vdso\.so\.1 \(0x[0-9a-fA-F]+\)', line):
            vdso += 1; continue
        linked = re.fullmatch(r'(\S+) => (/\S+) \(0x[0-9a-fA-F]+\)', line)
        direct = re.fullmatch(r'(/\S+) \(0x[0-9a-fA-F]+\)', line)
        V.require(linked is not None or direct is not None, 'closed ldd output; no unknown/missing dependency')
        path = linked.group(2) if linked else direct.group(1)
        V.require(path not in observed, 'one actual ldd row per resolved path')
        observed.add(path)
        if linked:
            name = linked.group(1)
            V.require(name not in named, 'unique named dependency')
            named.add(name)
        elif Path(path).name == 'ld-linux-x86-64.so.2':
            V.require('ld-linux-x86-64.so.2' not in named, 'unique direct loader dependency')
            named.add('ld-linux-x86-64.so.2')
    V.require(vdso == 1 and observed == seen and set(needed) <= named,
              'complete resolved ELF dependency closure')


def provenance(inputs, deployment, verified):
    return dict(deployment=inputs['deployment'], compiler_complete=verified['compiler_receipt'],
        compiler_owner=verified['compiler_owner'], compiler_generation=verified['compiler_generation'],
        candidate_cpu_receipt=verified['candidate_cpu_receipt'], candidate_cpu_sources=verified['candidate_cpu_sources'],
        original_object=verified['original_image'], native_complete=verified['native_receipt'],
        native_owner=verified['native_owner'], native_compiler_generation=verified['native_compiler_generation'],
        native_overlay=verified['native_overlay'], native_sources=verified['native_sources'],
        original_binary=verified['original_binary'], binary=verified['binary'], object=verified['image'],
        qualifications=verified['qualifications'], compiler_artifacts=verified['compiler_artifacts'],
        compiler_phase_records=verified['compiler_phase_records'], candidate_sources=verified['candidate_sources'],
        arithmetic_evidence=verified['arithmetic_evidence'])


def platform_review(value):
    V.keys(value, 'schema authority reviewed host boot_id devices topology_identity software_audit '
                 'notes production_authority runtime_premises_discharged')
    V.require(value['schema'] == 'ferric-p227-prefix-parity-platform-review-v1'
        and value['authority'] == 'none' and value['reviewed'] is True and value['host'] == HOST
        and value['devices'] == V.DEVICES and value['production_authority'] is False
        and value['runtime_premises_discharged'] is False, 'actual paired platform review')
    V.require(type(value['boot_id']) is str
        and re.fullmatch(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}', value['boot_id']), 'actual boot identity')
    V.require(type(value['topology_identity']) is list and len(value['topology_identity']) == 2,
              'two reviewed topology identities')
    V.pin(value['software_audit'], 8 << 20)
    V.require(type(value['notes']) is str and value['notes'].strip()
              and len(value['notes'].encode()) <= 16384, 'actual platform review notes')


def baseline_inputs(pins):
    value, record = pins.json(Path(__file__).with_name('baseline-inputs.json'), BASELINE_INPUTS_SHA)
    V.keys(value, 'schema files bytes scope tests_run gpu_execution')
    V.require(value['schema'] == 'ferric-p227-prefix-parity-baseline-inputs-v1'
        and value['tests_run'] is False and value['gpu_execution'] is False
        and type(value['files']) is list and len(value['files']) == 334
        and type(value['bytes']) is int and value['bytes'] == 88974677,
        'closed actual baseline metadata/payload census, no new execution claim')
    rows = value['files']
    V.require([row['path'] for row in rows] == sorted({row['path'] for row in rows})
        and sum(row['bytes'] for row in rows) == value['bytes'], 'sorted unique exact baseline records')
    for row in rows:
        V.pin(row, 64 << 20)
        V.require(Path(row['path']).is_relative_to(R / 'evidence'), 'retained task-owned baseline inputs')
        read(pins, row)
    return record, {row['path']: row for row in rows}


def context(input_pin, output):
    V.require(not sys.flags.optimize, 'optimization disabled')
    start = time.monotonic()
    pins = D.Pins(); inputs = document(pins, input_pin, 8 << 20)
    input_shape(inputs); package(pins, inputs['observer_manifest'])
    pure_tests(document(pins, inputs['observer_tests']), inputs, pins)
    deployment, verified = deployment_context(inputs, pins)
    artifact_review = document(pins, inputs['artifact_review'])
    reviewed_artifact(artifact_review, inputs, deployment, verified)
    platform = document(pins, inputs['platform_review']); platform_review(platform)
    read(pins, platform['software_audit'], maximum=8 << 20)
    runtime = document(pins, inputs['runtime_review'])
    runtime_review(runtime, verified['binary'], platform, pins)
    baseline_manifest, baseline_records = baseline_inputs(pins)
    metadata = verified['metadata']; requests = []
    for row, case in zip(inputs['cases'], V.CASES):
        record = row['baseline_request']
        V.require(baseline_records.get(record['path']) == record, 'actual closed baseline request')
        baseline = document(pins, record, 64 << 10)
        requested = make_request(row, metadata, verified['image'], E / inputs['matrix_label'] / case / 'captures')
        V.request(requested, baseline, case)
        for dependency in [baseline[key] for key in ('producer', 'consumer', 'pair_reference')] + baseline['prefix_requests']:
            V.require(baseline_records.get(dependency['path']) == dependency, 'closed original baseline dependency')
        for kind, review in zip(V.KINDS, row['reviews']):
            V.review(document(pins, review, 64 << 10), kind, requested, baseline, case)
        requests.append((requested, baseline))
    topology = D.load_module(pins, Path(inputs['topology_helper']['path']), TOPOLOGY_SHA, 'prefix_pair_topology')
    owned = D.load_module(pins, Path(__file__).with_name('frozen_owned.py'), OWNED_SHA, 'prefix_pair_owned')
    c = dict(pins=pins, inputs=inputs, input_pin=input_pin, requests=requests, platform=platform,
        runtime=runtime, artifact_review=artifact_review, deployment=deployment, verified=verified,
        topology=topology, owned=owned, environment=dict(ENV), output=output,
        provenance=provenance(inputs, deployment, verified), baseline_manifest=baseline_manifest)
    guard(c)
    V.require(time.monotonic() - start <= 600, 'bounded read-only preparation/replay')
    return c


def guard(c):
    # Only numerical-host deployed inputs are live here, never the original build graph.
    c['pins'].recheck()
    for row in c['runtime']['libraries']:
        V.require(Path(row['path']).resolve(strict=True) == Path(row['resolved']['path']),
                  'resolved runtime library alias unchanged')


def prepared_value(c, cases):
    return dict(schema='ferric-p228-independent-gpu-prepared-v1', inputs=c['input_pin'],
        observer_manifest=c['inputs']['observer_manifest'], baseline_manifest=c['baseline_manifest'],
        controller=c['pins'].pin(Path(__file__).resolve()), cases=cases, **c['provenance'],
        gpu_execution=False, production_authority=False, performance_claim=False,
        independent_numerical_acceptance=False, full_model_correctness=False)


def prepare(input_pin, output):
    V.require(output.parent == E and re.fullmatch(r'prefix-independent-prepared-v228-v[1-9][0-9]*', output.name)
        and not os.path.lexists(output), 'fresh flat preparation output')
    V.require(shutil.disk_usage(E).free >= 40 << 30, 'unchanged initial disk floor')
    c = context(input_pin, output)
    V.require(not os.path.lexists(E / c['inputs']['matrix_label']), 'fresh case matrix, no retry')
    V.require(shutil.disk_usage(E).free >= 38 << 30, 'unchanged ongoing disk floor')
    output.mkdir(mode=0o700)
    cases = []
    for case, (requested, _) in zip(V.CASES, c['requests']):
        record = save(output / (case + '.json'), requested)
        read(c['pins'], record, maximum=64 << 10)
        cases.append(dict(case=case, request=record))
    value = prepared_value(c, cases); guard(c)
    V.require(shutil.disk_usage(E).free >= 38 << 30, 'unchanged publication disk floor')
    return save(output / 'complete.json', value)


def load(prepared_pin):
    pins = D.Pins(); value = document(pins, prepared_pin, 8 << 20)
    output = Path(prepared_pin['path']).parent
    V.require(output.parent == E and Path(prepared_pin['path']).name == 'complete.json'
        and re.fullmatch(r'prefix-independent-prepared-v228-v[1-9][0-9]*', output.name),
        'exact retained preparation location')
    c = context(value['inputs'], output)
    cases = value['cases']
    V.require(type(cases) is list and len(cases) == 6, 'six prepared native requests')
    for row, case, (requested, _) in zip(cases, V.CASES, c['requests']):
        V.keys(row, 'case request')
        V.require(row['case'] == case and row['request']['path'] == str(output / (case + '.json'))
            and document(c['pins'], row['request'], 64 << 10) == requested,
            'prepared request recomputed from actual inputs')
    V.require(value == prepared_value(c, cases), 'prepared provenance recomputed without build-host opens')
    read(c['pins'], prepared_pin, maximum=8 << 20)
    V.require(output.resolve(strict=True) == output and output.is_dir()
        and {p.name for p in output.iterdir()} == {'complete.json', *(case + '.json' for case in V.CASES)},
        'closed preparation directory')
    c.update(prepared=value, prepared_pin=prepared_pin)
    guard(c)
    return c


def main():
    V.require(not sys.flags.optimize and len(sys.argv) == 4, 'INPUT_PATH SHA OUTPUT_LABEL')
    path, digest, label = sys.argv[1:]
    V.require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == HOST
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 0,
        'fixed numerical host/UID/affinity/controller nice')
    input_pin, _ = D.read_file(Path(path), maximum=8 << 20)
    V.require(input_pin['sha256'] == digest, 'actual preparation input SHA')
    record = prepare(input_pin, E / label)
    D.progress(dict(prepared=record, gpu_execution=False))


if __name__ == '__main__':
    main()
