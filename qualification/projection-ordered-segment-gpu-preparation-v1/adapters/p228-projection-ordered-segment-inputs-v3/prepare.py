"""Root-pinned shared/ordered input assembly; no launch or inferred approval."""
import contextlib
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-projection-ordered-segment-gpu-v3'
PACKAGE_SCHEMA = 'ferric-p228-projection-ordered-segment-gpu-package-v3'
OLD_NATIVE = dict(path=str(E / 'prefix-projection-ar4-host-observation-gpu-v228-v1/complete.json'),
    bytes=1156839, sha256='8341d1012f8d57512e7895a1b809ac9298bf2142943b89545c729c5d09af6393')
DEVICE_IDS = [16366993098680759275, 10838076764495710945]
START = time.monotonic()


def require(value, message):
    if not value:
        raise RuntimeError(message)
    if time.monotonic() - START > 600:
        raise RuntimeError('bounded data-only assembly time')


def bytes_at(path, maximum=1 << 20):
    require(path.resolve(strict=True) == path, 'canonical assembly dependency')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= maximum,
        'ordinary bounded assembly dependency')
    with path.open('rb') as stream:
        opened = os.fstat(stream.fileno()); raw = stream.read(maximum + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_nlink)
    require(stamp(before) == stamp(opened) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
        'stable assembly dependency')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest()), raw


@contextlib.contextmanager
def intake(package_sha):
    require(type(package_sha) is str and re.fullmatch('[0-9a-f]{64}', package_sha),
            'explicit root-reviewed frozen package digest')
    manifest_pin, raw = bytes_at(PACKAGE / 'manifest.json')
    require(manifest_pin['sha256'] == package_sha, 'exact root-pinned ordered package')
    manifest = json.loads(raw)
    require(manifest['schema'] == PACKAGE_SCHEMA and type(manifest['pure_tests']) is int
        and manifest['pure_tests'] > 0, 'actual frozen package census')
    bodies, pins = {}, {'manifest.json': manifest_pin}
    for row in manifest['files']:
        require(set(row) == {'path', 'bytes', 'sha256'} and re.fullmatch(r'[A-Za-z0-9_.-]+', row['path'])
            and row['path'] not in bodies and row['path'] != 'manifest.json', 'flat unique package member')
        pin, body = bytes_at(PACKAGE / row['path'])
        require(pin == dict(row, path=str(PACKAGE / row['path'])), 'actual frozen package body')
        bodies[row['path']], pins[row['path']] = body, pin
    missing = object(); saved = {}
    try:
        for name in ('layer_validation', 'prefix_contracts', 'observer_cpu', 'shared_cpu', 'ordered_cpu', 'intake'):
            saved[name] = sys.modules.get(name, missing)
            module = types.ModuleType(name); module.__file__ = str(PACKAGE / (name + '.py'))
            sys.modules[name] = module
            exec(compile(bodies[name + '.py'], module.__file__, 'exec'), module.__dict__)
        I = sys.modules['intake']
        require(I.PACKAGE_SCHEMA == PACKAGE_SCHEMA and I.PACKAGE_FILES == set(bodies)
            and type(I.PURE_TESTS) is int and I.PURE_TESTS == manifest['pure_tests'],
            'loaded closed source and authored test roster matches frozen manifest')
        yield I, pins
    finally:
        for name, previous in reversed(tuple(saved.items())):
            if previous is missing:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = previous


def record_shape(I, record, maximum=4 << 20):
    I.V.keys(record, 'path bytes sha256')
    require(type(record['path']) is str and Path(record['path']).is_absolute()
        and Path(record['path']).is_relative_to(E) and '..' not in Path(record['path']).parts
        and str(Path(record['path'])) == record['path']
        and type(record['bytes']) is int and 0 < record['bytes'] <= maximum
        and type(record['sha256']) is str and re.fullmatch('[0-9a-f]{64}', record['sha256']),
        'root-supplied actual bounded original-path FilePin')


def settings(I, config, notes):
    I.V.keys(config, 'schema route input_label output_label session cpu supervisor_tests parent_runtime_review worker_runtime_review')
    require(config['schema'] == 'ferric-p228-projection-ordered-segment-assembly-inputs-v1'
        and config['route'] in ('shared', 'ordered'), 'explicit route; no implicit default')
    route = config['route']
    require(re.fullmatch('prefix-projection-ordered-segment-' + route + r'-inputs-v228-v[1-9][0-9]{0,8}', config['input_label'])
        and re.fullmatch('prefix-projection-ordered-segment-' + route + r'-gpu-v228-v[1-9][0-9]{0,8}', config['output_label'])
        and type(config['session']) is str and re.fullmatch('[0-9a-f]{64}', config['session'])
        and config['session'] != '0' * 64, 'fresh route-specific namespaces and root-selected session')
    for key in ('cpu', 'supervisor_tests', 'parent_runtime_review', 'worker_runtime_review'):
        record_shape(I, config[key])
    cpu = Path(config['cpu']['path'])
    require(cpu.parent.parent == E and cpu.name == 'complete.json'
        and re.fullmatch(r'projection-ordered-segment-cpu-v228-v(?:[2-9]|[1-9][0-9]{1,8})', cpu.parent.name),
        'actual successful v2-or-later paired CPU, never failedv1 or old883')
    pure = Path(config['supervisor_tests']['path'])
    require(pure.parent.parent == E and pure.name == 'complete.json'
        and re.fullmatch(r'projection-ordered-segment-gpu-pure-v228-v[1-9][0-9]{0,8}', pure.parent.name),
        'actual new package pure-test receipt')
    require(config['parent_runtime_review'] != config['worker_runtime_review'], 'distinct actual runtime reviews')
    I.V.keys(notes, 'schema configuration reviewed authority gpu_attempts review_topics notes')
    require(notes['schema'] == 'ferric-p228-projection-ordered-segment-root-notes-v1'
        and notes['configuration'] == config and notes['reviewed'] is True and notes['authority'] == 'none'
        and type(notes['gpu_attempts']) is int and notes['gpu_attempts'] == 1,
        'explicit root-reviewed single arm; no automatic approval')
    I.V.keys(notes['review_topics'], ' '.join(I.TOPICS))
    for text in [notes['notes'], *notes['review_topics'].values()]:
        require(type(text) is str and len(text.strip()) >= 32 and len(text.encode()) <= 16384,
            'substantive root-authored review, not generated approval')


def request_copy(original, worker, session, native_out, route):
    require(route in ('shared', 'ordered') and original['schema'] ==
        'FerricFiniteProjectionResidualDecodeRequestV1', 'closed predecessor and explicit route')
    require(original['decode']['mode'] == 'autoregressive'
        and original['decode']['device_ids'] == DEVICE_IDS
        and all(type(uid) is int for uid in original['decode']['device_ids']), 'lossless original AR4 device IDs')
    require(type(session) is str and re.fullmatch('[0-9a-f]{64}', session) and session != '0' * 64,
            'root-selected nonzero session')
    request = copy.deepcopy(original)
    if route == 'ordered':
        request['schema'] = 'FerricFiniteProjectionResidualMlpOrderedRequestV1'
    request['decode'].update(worker=dict(worker, sha256=list(bytes.fromhex(worker['sha256']))),
        session=list(bytes.fromhex(session)), evidence_directory=str(native_out / 'native'))
    changed = {key for key in original['decode'] if request['decode'][key] != original['decode'][key]}
    require(changed == {'worker', 'session', 'evidence_directory'}
        and {k: v for k, v in request.items() if k not in ('decode', 'schema')} ==
            {k: v for k, v in original.items() if k not in ('decode', 'schema')},
        'only worker, session, output and explicit ordered schema change; images/model/prompt preserved')
    return request


def assemble(I, package_pins, config_path, config_sha, notes_path, notes_sha):
    pins = I.D.Pins()
    config_pin, raw = pins.read(config_path, config_sha, retain=True, maximum=64 << 10)
    config = I.D.parse(raw)
    notes_pin, raw = pins.read(notes_path, notes_sha, retain=True, maximum=192 << 10)
    notes = I.D.parse(raw)
    settings(I, config, notes)
    out, native_out = E / config['input_label'], E / config['output_label']
    require(not os.path.lexists(out) and not os.path.lexists(native_out), 'fresh input and native outputs')
    manifest, manifest_pin = I.package_record(pins)
    require(manifest_pin == package_pins['manifest.json'], 'loaded and registered package agree')
    old = I.doc(pins, OLD_NATIVE)
    require(old['schema'] == 'ferric-p228-projection-ar4-host-observation-gpu-v1' and old['passed'] is True
        and old['failures'] == [] and old['native_attempts'] == 1 and old['retries'] == 0
        and old['own_output_trajectory_checked'] is True and old['numerical_acceptance'] is False,
        'actual prior AR4 observation, not numerical authority')
    plan = copy.deepcopy(I.doc(pins, old['plan']))
    original = I.doc(pins, plan['request'], 64 << 10)
    require(old['request'] == plan['request'] and old['parent'] == plan['parent'] and old['worker'] == plan['worker']
        and original['schema'] == 'FerricFiniteProjectionResidualDecodeRequestV1'
        and original['decode']['mode'] == 'autoregressive', 'actual previous request and runtime identity')
    require(original['decode']['device_ids'] == DEVICE_IDS
        and all(type(uid) is int for uid in original['decode']['device_ids']), 'lossless selected GPU u64 identities')
    plan.update(schema=I.INPUT_SCHEMA, route=config['route'], output_label=config['output_label'],
        supervisor_tests=config['supervisor_tests'], parent_cpu=config['cpu'], worker_cpu=config['cpu'],
        parent_runtime_review=config['parent_runtime_review'], worker_runtime_review=config['worker_runtime_review'])
    require(all(plan[role + '_runtime_review'] != old[role + '_runtime_review']
                for role in ('parent', 'worker')), 'new ELF reviews cannot reuse old records')
    pure = I.doc(pins, config['supervisor_tests'])
    plan['supervisor_test_sources'] = pure['sources_before']
    I.supervisor_tests(pins, plan, manifest, manifest_pin)
    capture = I.baseline(pins, plan)
    prior = capture['prior']
    _, selected = I.cpu_evidence(pins, plan, capture)
    for role in ('parent', 'worker'):
        plan[role] = selected[role]['binary']
    runtime = dict(parent=I.deployed_binary(pins, plan['parent'], selected['parent']['binary']),
        worker=I.deployed_binary(pins, plan['worker'], selected['worker']['binary']), image=plan['prefix_image'])
    image = I.image_evidence(pins, plan)
    require(image == capture['image_provenance'] and plan['projection_image'] == capture['plan']['projection_image'],
        'unchanged corrected residual image and lineage')
    prefix = I.prefix_evidence(pins, plan, prior)
    for role in ('parent', 'worker'):
        prior['P'].runtime_review(I.doc(pins, plan[role + '_runtime_review']), runtime[role],
            prior['standalone']['platform'], pins)
    request = request_copy(original, runtime['worker'], config['session'], native_out, config['route'])
    I.request_check(pins, request, prior, runtime, native_out, plan['projection_image'], plan['mlp_image'], config['route'])
    pending = {}

    def pack(name, value):
        raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
        require(name not in pending and len(raw) <= 4 << 20, 'bounded distinct output body')
        pending[name] = raw
        return dict(path=str(out / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())

    plan['request'] = pack('request.json', request)
    review = {key: plan[key] for key in I.REVIEW_BINDINGS}
    review.update(schema=I.REVIEW_SCHEMA, reviewed=notes['reviewed'], authority=notes['authority'],
        gpu_attempts=notes['gpu_attempts'], output_label=plan['output_label'],
        review_topics=notes['review_topics'], notes=notes['notes'], image=runtime['image'],
        down2_image=prior['plan']['down2_image'], image_provenance=prior['standalone']['provenance'],
        down2_provenance=prior['down2_provenance'], projection_provenance=image,
        mlp_provenance=capture['mlp_provenance'], prefix_provenance=prefix,
        **{key: False for key in I.REVIEW_FALSE})
    I.engineering_review(review, plan, runtime, prior, image, capture['mlp_provenance'], prefix)
    plan['decode_review'] = pack('decode-review.json', review)
    I.input_shape(plan)
    plan_pin = pack('plan.json', plan)
    own = pins.pin(Path(__file__).resolve())
    pins.recheck(); prior['P'].guard(prior['standalone'])
    for pin in package_pins.values():
        require(bytes_at(Path(pin['path']))[0] == pin, 'all package bodies unchanged before writing')
    pack('assembly.json', dict(schema='ferric-p228-projection-ordered-segment-root-assembly-v1', route=config['route'],
        plan=plan_pin, configuration=config_pin, root_notes=notes_pin, assembler=own,
        supervisor_manifest=manifest_pin, supervisor_tests=config['supervisor_tests'], prior_native=OLD_NATIVE,
        selected_cpu=config['cpu'], selected_runtime=runtime, changed_request_fields=['decode.worker', 'decode.session', 'decode.evidence_directory']
            + (['schema'] if config['route'] == 'ordered' else []),
        inputs=dict(pins.records), outputs={name: dict(path=str(out / name), bytes=len(raw),
            sha256=hashlib.sha256(raw).hexdigest()) for name, raw in pending.items()},
        source_postchecks_passed=True, copied_root_decisions=True, automatic_approval=False,
        new_gpu_execution=False, new_compiler_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False, all_transitive_compiler_inputs_rehashed=False,
        compiler_binary_bodies_replayed=False, fresh_runtime_audits_performed=False))
    require(len(pending) == 4 and not os.path.lexists(out) and not os.path.lexists(native_out), 'exclusive four-file assembly')
    out.mkdir(mode=0o700)
    for name, raw in pending.items():
        with (out / name).open('xb') as stream:
            stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        require(hashlib.sha256(bytes_at(out / name, 4 << 20)[1]).hexdigest() == hashlib.sha256(raw).hexdigest(),
            'retained assembly byte identity')
    pins.recheck(); prior['P'].guard(prior['standalone'])
    print(json.dumps(plan_pin, sort_keys=True), flush=True)


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
        'ordinary Python -B')
    require(len(sys.argv) == 6, 'PACKAGE_SHA CONFIG_PATH CONFIG_SHA ROOT_NOTES_PATH ROOT_NOTES_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
        'numerical host data preparation, never build-host platform substitution')
    os.sched_setaffinity(0, {8, 9}); os.nice(10); os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 300),
                      (resource.RLIMIT_FSIZE, 4 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        limit = min([cap] + [v for v in old if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    with intake(sys.argv[1]) as (module, package_pins):
        assemble(module, package_pins, Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]), sys.argv[5])


if __name__ == '__main__':
    main()
