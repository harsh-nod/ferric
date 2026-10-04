"""Record root's bounded engineering review and assemble a fresh Down2 request."""
import copy
import hashlib
import json
import os
from pathlib import Path
import secrets
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OUT = E / 'prefix-down2-clock-tf4-inputs-v228-v1'
CASE = 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1'
CLOCK = E / 'prefix-device-clock-tf4-shared-full-currentness-gpu-v228-v1/complete.json'
ROW = E / 'row-down2-checked-probe-v228-v1/complete.json'
PACKAGE = E / 'p228-down2-clock-gpu-v1'
PURE = E / 'down2-clock-gpu-pure-v228-v1/complete.json'
FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
         'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
         'timestamp_calibration', 'clock_domain_validated', 'cross_device_clock_alignment', 'overlap_claim')
PINS = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path, sha=None, maximum=16 << 20):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical retained input')
    require(path.stat().st_size <= maximum, 'bounded input')
    raw = path.read_bytes()
    record = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(sha is None or record['sha256'] == sha, 'retained input digest')
    require(PINS.setdefault(str(path), record) == record, 'input changed')
    return record, raw


def document(path, sha=None):
    record, raw = pin(path, sha)
    return json.loads(raw), record


def recorded(record):
    value, actual = document(Path(record['path']), record['sha256'])
    require(actual == record, 'retained extent')
    return value


def save(path, value):
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    with path.open('xb') as stream:
        stream.write(raw)
    return pin(path)[0]


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and len(sys.argv) == 2, 'unoptimized MANIFEST_SHA invocation')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9}
            and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'fixed MI350 CPU identity')
    require(all(os.environ.get(key) == '' for key in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    require(Path('/proc/sys/kernel/random/boot_id').read_text().strip()
            == '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a', 'reviewed boot unchanged')
    require(not os.path.lexists(OUT) and not os.path.lexists(E / CASE), 'fresh inputs and native output')
    clock, clock_pin = document(CLOCK, 'e34189597dc7db7a7c325040f5381932e84390c1a2cf32055830858cb9278ddb')
    row, row_pin = document(ROW, '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e')
    require(clock['passed'] is True and clock['failures'] == [] and clock['native_attempts'] == 1
            and clock['retries'] == 0 and row['passed'] is True and row['postcheck_errors'] == [],
            'actual successful input receipts')
    manifest, manifest_pin = document(PACKAGE / 'manifest.json', sys.argv[1])
    require(manifest['schema'] == 'ferric-p228-down2-clock-package-v1'
            and manifest['pure_tests'] == 75 and len(manifest['files']) == 17, 'frozen complete package')
    for member in manifest['files']:
        actual, _ = pin(PACKAGE / member['path'], member['sha256'])
        require(actual['bytes'] == member['bytes'], 'package member extent')
    pure, pure_pin = document(PURE)
    require(pure['passed'] is True and pure['tests'] == 75
            and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['manifest_sha256'] == manifest_pin['sha256']
            and pure['controller_sha256'] == '5995bcb2ee48f95b9f1c11e8eb9942ecae41e49032fc2fc56d9f9c909f33123b'
            and pure['source_postchecks_passed'] is True, 'actual successful successor tests')
    require(recorded(pure['sources_before']) == recorded(pure['sources_after']), 'tested source stability')
    plan = copy.deepcopy(recorded(clock['plan']))
    request = copy.deepcopy(recorded(clock['request']))
    prior_request = copy.deepcopy(request)
    review = copy.deepcopy(recorded(plan['decode_review']))
    for role in ('parent', 'worker'):
        runtime_review = recorded(plan[role + '_runtime_review'])
        require(runtime_review['reviewed'] is True
                and runtime_review['boot_id'] == '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a', 'retained runtime review')
    original_image = row['artifacts']['emitted/artifact.hsaco']
    image_pin, raw = pin(Path(original_image['path']), original_image['sha256'])
    require(image_pin == original_image and (image_pin['bytes'], image_pin['sha256']) ==
            (33112, '65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449'), 'actual Down2 image')
    require(row['unresolved_runtime_requirements'] == 8
            and row['runtime_requirements_discharged'] is False, 'no compiler receipt authority upgrade')
    for artifact in row['artifacts'].values():
        actual, _ = pin(Path(artifact['path']), artifact['sha256'])
        require(actual == artifact, 'all ten artifact extents')
    OUT.mkdir(mode=0o700)
    image_path = OUT / 'tiles.hsaco'
    with image_path.open('xb') as stream:
        stream.write(raw)
    image = pin(image_path)[0]
    request['decode']['tiles_image'] = dict(image, sha256=list(bytes.fromhex(image['sha256'])))
    request['decode']['session'] = list(secrets.token_bytes(32))
    request['decode']['evidence_directory'] = str(E / CASE / 'native')
    require(request['decode']['session'] != prior_request['decode']['session'], 'fresh session')
    delta = {name for name in request['decode']
             if request['decode'][name] != prior_request['decode'][name]}
    require(delta == {'tiles_image', 'session', 'evidence_directory'}, 'exact request change set')
    request_pin = save(OUT / 'request.json', request)
    plan.update(schema='ferric-p228-down2-clock-inputs-v1', output_label=CASE,
                clock_baseline=clock_pin, down2_lowering=row_pin, down2_image=image,
                request=request_pin, supervisor_tests=pure_pin, supervisor_test_sources=pure['sources_before'])
    commands = {command['name']: command for command in row['commands']}
    provenance = dict(lowering=row_pin, original_image=original_image, transported_image=image,
        candidate_cpu=row['candidate_cpu'], compiler_generation=row['compiler_generation'],
        source_handoff=row['artifacts']['emitted/source.handoff-v3'],
        formal_archive=row['artifacts']['extracted/formal.archive'], llvm=row['artifacts']['extracted/module.ll'],
        emission_receipt=row['artifacts']['emitted/receipt.txt'],
        descriptor=commands['descriptor-metadata']['stdout'], isa=commands['disassembly']['stdout'],
        elf_notes=commands['elf-notes']['stdout'], unresolved_runtime_requirements=8,
        runtime_requirements_discharged=False, production_authority=False,
        numerical_acceptance=False, performance_claim=False)
    topics = dict(
        source_lineage='Root inspected the exact paired-row macro and its fourteen CPU tests, plus '
            'the nine actual lowering/replay/emission phases. Each independent row keeps its 96-step '
            'ordered FP32 accumulation and Wave64 sum. A rejected first row invalidates the cached '
            'second partial without skipping its collective. Provider53 and all other stages are unchanged.',
        formal='Actual ranked/formal replay completed with eleven pointer roots and eight unresolved '
            'runtime requirements. The emission is engineering-only and does not discharge those '
            'requirements. The existing worker retains exact image, argument, allocation, geometry, '
            'finite lifecycle and completion checks. This review grants no production authority.',
        isa='Root inspected strict fp-contract-off LLVM, two separate multiply/add accumulator chains, '
            'the exact entry symbol, ELF and ISA. Descriptor and runtime agree on Wave64, 64x1x1 '
            'workgroup, maximum64 workgroups, eleven roots/88 explicit argument bytes, 344 executable '
            'kernarg bytes, LDS512 and private0. Resource counts are106 VGPR/106 SGPR with no spills. '
            'This is scalar FP32 code; there is no MFMA or occupancy/speedup claim.',
        coherence='The source provider, all three LDS exchange phases, fixed512 scheduler rounds and '
            'ordered acquired input/weight views remain unchanged. Root retained the exact original '
            'images and storage layouts. The worker must still complete every acquired-state and '
            'output check. Raw device clocks are not calibrated durations or cross-device ordering.',
        lifecycle='Exactly one native attempt uses the existing owned process groups/pidfds, deadlines '
            'and memory bounds. Require four complete forwards, all1172 dispatch rows/16 clock samples, '
            'all four606976-byte outputs/152 tensor comparisons, consuming Close and natural reaping. '
            'All six process/topology audits remain mandatory, including post-audits after failure.',
        selected_device='The same MI350 host and boot2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a are selected, '
            'with ordered GPU identities16366993098680759275 and10838076764495710945. The unchanged '
            'parent/worker reviews and live library/platform checks remain required. No GPU reset, '
            'unrelated process termination, automatic retry or host substitution is permitted.')
    notes = ('Root reviewed this exact engineering-only Down2 image substitution against the real '
             'clock-enabled TF4 capture. Accept only complete byte-identical state/output evidence '
             'and normal Close/reap; preserve failures without retry. This is not independent '
             'full-model acceptance, the sustained2048/256 workload, calibrated timing or700 tokens/s.')
    image_review = dict(schema='ferric-p228-down2-image-engineering-review-v1', reviewed=True,
        authority='none', gpu_attempts=1, unresolved_runtime_requirements=8,
        parent=plan['parent'], worker=plan['worker'], provenance=provenance,
        review_topics=topics, notes=notes, **{name: plan[name] for name in
            ('down2_lowering', 'down2_image', 'clock_baseline', 'request')}, **{key: False for key in FALSE})
    plan['down2_review'] = save(OUT / 'down2-review.json', image_review)
    review.update(schema='ferric-p228-down2-clock-engineering-review-v1', reviewed=True,
                  notes=notes, review_topics=topics, historical_runtime=clock['selected_runtime'])
    for name in ('request', 'clock_baseline', 'down2_lowering', 'down2_image', 'down2_review'):
        review[name] = plan[name]
    require(all(review[name] is False for name in FALSE), 'no acceptance claims')
    plan['decode_review'] = save(OUT / 'decode-review.json', review)
    plan_pin = save(OUT / 'plan.json', plan)
    for path, record in list(PINS.items()):
        require(pin(Path(path), record['sha256'])[0] == record, 'input/output changed during assembly')
    print(json.dumps(plan_pin), flush=True)


if __name__ == '__main__':
    main()
