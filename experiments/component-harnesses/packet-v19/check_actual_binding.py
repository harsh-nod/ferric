"""Disabled bounded final binding of actual V19 artifacts and CPU receipts."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat

ENABLED = False
D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
QUALIFIER = D / 'inputs/packet-v19-harness-a003/prepare_qualification.py'
QUALIFIER_SHA = 'e94fe386e94f4bcbd6e68b5536ccdb1647e3ad780ea3ebcf5b359204ff7774e3'
ARCHIVE_SHA = 'c78b27ec8d7f1f53d3e1d22631bfdb8037327be734ce59cb328dac3f40ccd4f6'
RUNTIME_INPUTS = D / 'inputs/component-ordered-bundle-a001/inputs-latency.json'
RUNTIME_INPUTS_SHA = '842e4b340b30e95b4696bd3a10d942993edd49d4469c196faa957856176743f6'
OUTPUT = D / 'packet-v19-binding-review-a003'
ENVELOPE = 32 * 1024**2


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    require(ENABLED, 'disabled pending review')
    require(QUALIFIER.resolve(strict=True) == QUALIFIER, 'canonical fixed qualifier')
    info = QUALIFIER.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_uid == 1046 and info.st_nlink == 1
            and 0 < info.st_size <= 64 * 1024, 'bounded owned qualifier')
    raw = QUALIFIER.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == QUALIFIER_SHA, 'exact reviewed qualifier')
    q = importlib.util.module_from_spec(importlib.util.spec_from_file_location('packet_source_custody', QUALIFIER))
    exec(compile(raw, str(QUALIFIER), 'exec'), q.__dict__)
    q.ARCHIVE_SHA = ARCHIVE_SHA
    q.environment()
    files, sources, _ = q.payload()
    before_sources = q.check_sources(files)
    before = q.allocation(ENVELOPE)
    c_path = q.ROOT / 'harness/launch_contract.py'
    c = importlib.util.module_from_spec(importlib.util.spec_from_file_location('actual_v19_contract', c_path))
    exec(compile(files['harness/launch_contract.py'], str(c_path), 'exec'), c.__dict__)
    require(c.PACKET_TICKS_QUALIFIED is True, 'explicit enabled successor with complete actual CPU evidence')
    binder = c.module(q.ROOT / 'bind_build.py', sources['bind_build.py'])
    inventory = {}

    def binding(path, expected=None, empty=False):
        path = Path(path)
        require(path.is_relative_to(D), 'retained private-stage input')
        raw, sha = c.read(path, expected, 64 * 1024**2, empty=empty)
        info = path.lstat()
        require(info.st_uid == 1046 and info.st_nlink == 1, 'owned single-link actual input')
        previous = inventory.get(str(path))
        record = {'sha256': sha, 'size_bytes': len(raw), 'device': info.st_dev,
                  'inode': info.st_ino, 'mtime_ns': info.st_mtime_ns, 'ctime_ns': info.st_ctime_ns}
        require(previous is None or previous == record, 'stable duplicate input binding')
        inventory[str(path)] = record
        return {'path': str(path), 'sha256': sha}

    def bound(path, expected=None):
        return c.bound(binding(path, expected))

    runtime = bound(RUNTIME_INPUTS, RUNTIME_INPUTS_SHA)
    base = D / 'client-packet-baseline-55c-a004'
    v19 = D / 'client-packet-v19-55c-a001'
    base_artifacts = bound(base / 'qualification/retention/receipt.json')['artifacts']
    v19_artifacts = bound(v19 / 'qualification/retention/receipt.json')['artifacts']
    require(v19_artifacts['source_archive'] == base_artifacts['source_archive'], 'same frozen A004 archive')
    source = base_artifacts['source_archive']
    binary = v19_artifacts['controller']
    evidence = {'base_source': binding(D / 'client-native32-a003/repair-policy-a001/ferric-native32-55c1a9b6-policy-a001.tar.gz',
                                       c.BASE_CONTROLLER_SOURCE_SHA),
        'source_roster': binding(base / 'source-files.json'), 'source_modes': binding(base / 'source-modes.json'),
        'qualifier': binding(c.PACKET_QUALIFIER, c.PACKET_QUALIFIER_SHA),
        'a005': binding(D / 'inputs/native32-qualification-g36-a001/qualify_client_g36_a005_enabled.py',
                        '5f29d7dd42f913334e9467465a674ac94ec05aaa6f4e6c187d3927421de0c354'),
        'g36': binding(D / 'inputs/component-ordered-cpu-a001/qualify_source.py',
                       'b84d2f83eaffd7adf4f9aa760f88dd44d7fe4b2a58c23ea8a57d7beb3cfd0297')}
    phases = {}
    for name in sorted(c.PHASES):
        if name in c.CLIENT_COMMANDS:
            role = c.CLIENT_COMMANDS[name]
            directory = D / ('results/pages-packet-baseline-55c-' + role + '-g36-a004')
            inner = base / 'qualification' / role / 'receipt.json'
            helper = c.PACKET_QUALIFIER
        elif name in c.V19_COMMANDS:
            role = c.V19_COMMANDS[name]
            directory = D / ('results/pages-packet-v19-55c-' + role + '-g36-a001')
            inner = v19 / 'qualification' / role / 'receipt.json'
            helper = c.V19_QUALIFIER
        else:
            directory = D / ('results/pages-packet-v19-harness-' + name + '-g36-a003')
            inner = q.STAGE_ROOT / (name + '-custody.json')
            helper = QUALIFIER
        paths = {'status': directory / 'exit.status', 'result': directory / 'result.json',
                 'stdout': directory / 'stdout', 'stderr': directory / 'stderr', 'inner': inner, 'helper': helper}
        phases[name] = {key: binding(paths[key], empty=key in ('stdout', 'stderr'))
                        for key in c.phase_receipt_fields(name)}
        outer = c.bound(phases[name]['result'])
        phases[name].update(profile=c.PACKET_PROFILE,
                            argv_sha256=hashlib.sha256(c.encoded(outer['argv'])).hexdigest())
    config = {'schema': 'FerricV19Packet55cBindingInputsV1', 'runtime_main': c.RUNTIME_MAIN,
        'runtime_source': binding(runtime['runtime_evidence']['source-archive']['path'], c.RUNTIME_SOURCE_SHA),
        'controller_source': binding(source['path'], source['sha256']),
        'worker': binding(runtime['worker']['path'], c.WORKER_SHA),
        'controllers': {'diagnostic': binding(binary['path'], binary['sha256'])}, 'phases': phases,
        'harness_sources': {name: sources['harness/' + name] for name in
            (*c.OWN_SOURCES, *('measurement/' + name for name in c.MEASUREMENT_SOURCES))},
        'external_sources': {name: sources[name] for name in c.EXTERNAL_SOURCES},
        'guard_sources': {c.PACKET_PROFILE: {
            'guard': binding(q.GUARD, q.PINS[q.GUARD]),
            'environment': binding(q.ENVIRONMENT, q.PINS[q.ENVIRONMENT])}},
        'controller_evidence': evidence,
        'runtime_evidence': {name: binding(item['path'], item['sha256'])
                             for name, item in runtime['runtime_evidence'].items()},
        'harness_evidence': {'archive': binding(q.ARCHIVE, ARCHIVE_SHA), 'helper': binding(QUALIFIER, QUALIFIER_SHA)},
        'v19_evidence': {'qualifier': binding(c.V19_QUALIFIER, c.V19_QUALIFIER_SHA),
                         'baseline_controller': binding(base_artifacts['controller']['path'], base_artifacts['controller']['sha256'])}}
    require(not os.path.lexists(OUTPUT) and len(inventory) <= 256, 'fresh bounded actual-binding output')
    build, cpu = binder.qualify(config, q.ROOT / 'harness')
    counts = c.validate_cpu(cpu, build)
    for path, record in tuple(inventory.items()):
        binding(path, record['sha256'], empty=record['size_bytes'] == 0)
    after_sources = q.check_sources(files)
    require(before_sources == after_sources == sources, 'unchanged qualified source before/after actual replay')
    q.environment()
    OUTPUT.mkdir(mode=0o700)
    for name, value in (('inputs.json', config), ('input-roster.json', inventory),
                        ('review.json', {'schema': 'FerricV19ActualBindingReviewV1', 'inputs_validated': True,
                            'executed_tests': counts, 'native_executed': False, 'native_admitted': False,
                            'source_before': before_sources, 'source_after': after_sources})):
        with (OUTPUT / name).open('xb') as stream:
            stream.write(c.encoded(value))
    emitted = binder.emit(config, q.ROOT / 'harness', OUTPUT / 'bound')
    for path, record in tuple(inventory.items()):
        binding(path, record['sha256'], empty=record['size_bytes'] == 0)
    require(q.check_sources(files) == sources, 'emission preserves qualified source')
    q.environment()
    after = q.allocation()
    require(after['stage_allocated_bytes'] - before['stage_allocated_bytes'] <= ENVELOPE, 'fixed check-only envelope')
    print(json.dumps({'schema': 'FerricV19ActualBindingCustodyV1', 'output': str(OUTPUT),
        'archive_sha256': ARCHIVE_SHA, 'helper_sha256': QUALIFIER_SHA,
        'allocation_before': before, 'allocation_after': after, 'input_files': len(inventory),
        'executed_tests': counts, 'binding': emitted, 'native_executed': False, 'native_admitted': False}, sort_keys=True))


if __name__ == '__main__':
    main()
