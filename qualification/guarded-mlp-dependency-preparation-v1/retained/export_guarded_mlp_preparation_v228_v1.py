"""Export bounded preparation evidence; never traverse dependency source bodies."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import tarfile
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CPU_SHA = 'd1565161acfe89c4ac9136767961c7b273cd1eb6562a4e9e666ea4a3e306de87'
VENDOR_FAILED_SHA = '254428ec279074de41a85da4b2585b40177b435ec04a17a06a88a5de8544179d'
FETCH_FAILED_SHA = '350a1c8b8b347b5aec5008b1b567ce76585bef1cc15e7aba2b362ef973420419'
CONTROLLERS = {
    'guarded-mlp-vendor-v228-v1': 'ef6e27eb2597f1dbb1e7f5ee494f995bee92412dd1a91729fc49ab8f7bbe88f3',
    'guarded-mlp-fetch-v228-v1': '4a52b1e73055978dbc992227c69115e257f4c638bd5cc01dcfeafa1e505a9336',
    'guarded-mlp-vendor-v228-v2': '2e99310a963700289e7690db36d57f0b07dd33cd0460a5255c104391db81453f',
}
TEST_SHA = '8c53196aefdcd9345b360200f64311eefb0eacbdc7c4dd4b0f883734375c317b'
PREFLIGHT_SHA = '4e5b764dc89fa8241f8c8615f548ae190f32cc5486ae0f399ef74b11115ad855'
FILE_CAP, TOTAL_CAP, ARCHIVE_CAP = 96 << 20, 256 << 20, 128 << 20
DEADLINE = None


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def bounded():
    require(DEADLINE is None or time.monotonic() < DEADLINE, 'export wall bound')


def sha(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'invalid literal SHA256')
    return value


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def read(path, expected=None):
    bounded()
    path = Path(path)
    require(path.is_absolute() and '..' not in path.parts and path.resolve(strict=True) == path,
            'noncanonical evidence path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= FILE_CAP,
            'evidence file type/extent')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed before read')
        body = stream.read(FILE_CAP + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed during read')
    require(len(body) == before.st_size and stamp(path.lstat()) == stamp(before), 'file changed after read')
    value = dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
    require(expected is None or value == expected, 'evidence pin mismatch: ' + str(path))
    bounded()
    return value, body


def file_pin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}
            and type(value['path']) is str and type(value['bytes']) is int
            and 0 <= value['bytes'] <= 1 << 40, 'invalid FilePin')
    sha(value['sha256'])
    return value


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True) + '\n').encode('ascii')


def registry_delta(before, after):
    require(type(before) is dict and type(after) is dict
            and set(before) == set(after) == {'src', 'cache', 'index'}, 'registry section roster')
    result = {}
    for section in ('src', 'cache', 'index'):
        old, new = before[section], after[section]
        require(type(old) is dict and type(new) is dict and max(len(old), len(new)) <= 200000,
                'registry map extent')
        for mapping in (old, new):
            for name, value in mapping.items():
                path = Path(name)
                require(type(name) is str and not path.is_absolute() and '..' not in path.parts,
                        'registry relative path')
                file_pin(value)
                require(value['path'] == str(Path('/home/harmenon/.cargo/registry') / section / path),
                        'registry original path join')
        changes = {
            'added': sorted(set(new) - set(old)),
            'removed': sorted(set(old) - set(new)),
            'changed': sorted(name for name in set(old) & set(new) if old[name] != new[name]),
        }
        groups = {}
        for kind, names in changes.items():
            packages = {}
            for name in names:
                key = '/'.join(Path(name).parts[:2])
                row = packages.setdefault(key, dict(files=0, before_bytes=0, after_bytes=0))
                row['files'] += 1
                row['before_bytes'] += old.get(name, {}).get('bytes', 0)
                row['after_bytes'] += new.get(name, {}).get('bytes', 0)
            require(len(packages) <= 10000, 'registry package-summary bound')
            groups[kind] = dict(files=len(names), names_sha256=hashlib.sha256(encoded(names)).hexdigest(),
                                by_package=packages)
        result[section] = dict(before_files=len(old), after_files=len(new),
                               before_bytes=sum(row['bytes'] for row in old.values()),
                               after_bytes=sum(row['bytes'] for row in new.values()), **groups)
    return dict(schema='ferric-guarded-mlp-registry-delta-summary-v1', sections=result,
                scope='Data-only map comparison; no cause attribution or restoration claim.',
                original_fetch_controller_passed=False, dependency_source_bodies_copied=False)


class Export:
    def __init__(self):
        self.files = {}
        self.total = 0

    def add(self, path, expected=None, digest=None):
        path = Path(path)
        require(path.is_relative_to(E), 'export path outside exact evidence root')
        member = str(path.relative_to(E))
        value, body = read(path, expected)
        require(digest is None or value['sha256'] == sha(digest), 'literal input digest mismatch')
        if member not in self.files:
            self.total += value['bytes']
            self.files[member] = value
        require(self.files[member] == value and len(self.files) <= 64 and self.total <= TOTAL_CAP,
                'closed export member/byte bound')
        return json.loads(body) if path.suffix == '.json' else body

    def terminal(self, namespace, name, digest):
        fetch = 'fetch' in namespace
        label = 'fetch' if fetch else 'vendor'
        root = E / namespace
        require(name in ('complete.json', 'failed.json'), 'terminal filename')
        value = self.add(root / 'evidence' / name, digest=digest)
        require(value['schema'] == ('ferric-guarded-mlp-nightly-fetch-v1' if fetch
                                    else 'ferric-guarded-mlp-vendor-preparation-v1')
                and type(value['passed']) is bool and value['passed'] == (name == 'complete.json'),
                'terminal schema/outcome differs')
        require(value['qualified_cpu_complete']['sha256'] == CPU_SHA, 'qualified CPU join')
        for field in ('host_fixture_crate_binding_used', 'compiler_hsaco_reproduced',
                      'gpu_execution', 'full_model_acceptance', 'performance_claim', 'config_installed'):
            require(value[field] is False, 'unsupported terminal authority: ' + field)
        controller = root / ('run_fetch.py' if fetch else 'run_vendor.py')
        self.add(controller, file_pin(value['controller']), CONTROLLERS[namespace])
        allowed = {label + '.' + suffix for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
        allowed |= {kind + '-' + side + '.json' for kind in ('cpu-sources', 'rust-src') for side in ('before', 'after')}
        allowed |= {'registry-before.json', 'registry-after.json', 'registry-changes.json'} if fetch else {'vendor-files.json'}
        require(type(value['raw']) is dict and set(value['raw']) <= allowed, 'raw evidence roster')
        for key, pin in value['raw'].items():
            require(file_pin(pin)['path'] == str(root / 'evidence' / key), 'raw original path')
            self.add(Path(pin['path']), pin)
        require(type(value['phases']) is list and len(value['phases']) == 1, 'actual single preparation phase')
        phase = value['phases'][0]
        leaf = self.add(root / 'evidence' / (label + '.result.json'), value['raw'][label + '.result.json'])
        require(phase == leaf and phase['label'] == label and phase['reaped'] is True
                and phase['process_group_absent'] is True and phase['natural_exit'] is True
                and phase['forced_cleanup'] is False and phase['timed_out'] is False,
                'actual leaf lifecycle join')
        for stream in ('stdout', 'stderr'):
            require(phase[stream] == value['raw'][label + '.' + stream], 'leaf stream join')
        require(phase['command'] == value['raw'][label + '.command.json'], 'leaf command join')
        if value['passed']:
            require(phase['exit_code'] == 0 and value['failure'] is None
                    and value['postcheck_errors'] == [] and value['input_sources_unchanged'] is True
                    and set(value['raw']) == allowed, 'successful preparation gates')
        if namespace == 'guarded-mlp-vendor-v228-v1':
            require(phase['exit_code'] == 101 and value['postcheck_errors'] == [], 'actual offline refusal changed')
        if fetch:
            require(phase['exit_code'] == 0 and value['failure'] is None
                    and len(value['postcheck_errors']) == 1
                    and 'existing registry src bodies changed' in value['postcheck_errors'][0],
                    'actual fetch/postcheck outcome changed')
        return value


def main():
    global DEADLINE
    DEADLINE = time.monotonic() + 180
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture-sha', required=True, type=sha)
    parser.add_argument('--fixture-stdout-sha', required=True, type=sha)
    parser.add_argument('--vendor-terminal', choices=('complete.json', 'failed.json'))
    parser.add_argument('--vendor-sha', type=sha)
    parser.add_argument('--archive', required=True, type=Path)
    args = parser.parse_args()
    require(bool(args.vendor_terminal) == bool(args.vendor_sha), 'paired optional actual vendor arguments')
    output = args.archive
    require(output.is_absolute() and output.is_relative_to(E) and '..' not in output.parts
            and output.parent.resolve(strict=True) == output.parent and not os.path.lexists(output),
            'archive must be a fresh canonical path under E')
    export = Export()
    vendor = export.terminal('guarded-mlp-vendor-v228-v1', 'failed.json', VENDOR_FAILED_SHA)
    fetch = export.terminal('guarded-mlp-fetch-v228-v1', 'failed.json', FETCH_FAILED_SHA)
    fixture_root = E / 'guarded-mlp-fetch-v228-v1'
    export.add(fixture_root / 'fixture.json', digest=args.fixture_sha)
    export.add(fixture_root / 'fixture.stdout', digest=args.fixture_stdout_sha)
    export.add(fixture_root / 'test_fetch_cache.py', digest=TEST_SHA)
    export.add(E / 'guarded-mlp-cache-restoration-preflight-v228-v1.json', digest=PREFLIGHT_SHA)
    later = export.terminal('guarded-mlp-vendor-v228-v2', args.vendor_terminal, args.vendor_sha) if args.vendor_terminal else None
    old = export.add(fixture_root / 'evidence/registry-before.json', fetch['raw']['registry-before.json'])
    new = export.add(fixture_root / 'evidence/registry-after.json', fetch['raw']['registry-after.json'])
    summary = encoded(registry_delta(old, new))
    del old, new
    exporter_path = Path(__file__).resolve()
    export.add(exporter_path)
    exporter_pin = export.files[str(exporter_path.relative_to(E))]
    manifest = encoded(dict(schema='ferric-guarded-mlp-preparation-export-v1', exporter=exporter_pin,
        files=export.files, files_count=len(export.files), retained_bytes=export.total,
        generated={'registry-delta-summary.json': dict(bytes=len(summary), sha256=hashlib.sha256(summary).hexdigest())},
        terminal_outcomes=dict(vendor_v1=vendor['passed'], fetch_v1=fetch['passed'],
                               vendor_v2=later['passed'] if later else None),
        dependency_source_bodies_copied=False, tested_code_imported=False,
        inputs_or_products_rehashed=False, fixture_claim='Caller-pinned primary observation, not replayed.',
        archive_scope='Original direct receipts, raw maps/logs, controllers and primary fixture evidence only.'))
    require(export.total + len(summary) + len(manifest) <= TOTAL_CAP, 'generated export byte bound')
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz') as archive:
            for member, pin in sorted(export.files.items()):
                _, body = read(pin['path'], pin)
                info = tarfile.TarInfo(member)
                info.size, info.mode = len(body), 0o600
                archive.addfile(info, io.BytesIO(body))
                require(stream.tell() <= ARCHIVE_CAP, 'compressed archive byte bound')
            for member, body in (('registry-delta-summary.json', summary), ('export-manifest.json', manifest)):
                info = tarfile.TarInfo(member)
                info.size, info.mode = len(body), 0o600
                archive.addfile(info, io.BytesIO(body))
        require(stream.tell() <= ARCHIVE_CAP, 'final compressed archive byte bound')
    for pin in export.files.values():
        read(pin['path'], pin)
    require(read(Path(__file__).resolve())[0] == exporter_pin, 'exporter changed')
    # Archives can exceed the metadata-file cap; hash this bounded output separately.
    digest = hashlib.sha256()
    with output.open('rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            bounded()
            digest.update(block)
    print(json.dumps(dict(archive=dict(path=str(output), bytes=output.stat().st_size,
                                      sha256=digest.hexdigest()), files=len(export.files),
                          generated_files=2, retained_bytes=export.total), sort_keys=True))


if __name__ == '__main__':
    main()
