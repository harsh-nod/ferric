"""Retire only three retained failed diagnostic generations' obsolete caches."""
import hashlib
import json
import os
import pwd
from pathlib import Path
import shutil
import signal
import stat

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OUTPUT = E / 'guarded-mlp-obsolete-diagnostic-cache-retirement-v228-v1.json'
GENERATIONS = (
    dict(root='guarded-mlp-core-result-diagnostic-cpu-v228-v1',
         schema='ferric-guarded-mlp-core-result-diagnostic-cpu-v1',
         receipt=dict(bytes=243002, sha256='bb922803dee0a5d97617839cb4ed287c1397b09d809ea8b434b75213ccf0d061'),
         archive='guarded-mlp-core-result-diagnostic-evidence-v228-v1.tar.gz',
         archive_pin=dict(bytes=2893598, sha256='5de7c8c1bf4302af4a4a51ed69fe872d7c2bfdc5c185888e5a3d148a8ed50295'),
         source_map=dict(bytes=2053359, sha256='b0de7c984e15aae1e7920e1b046e079b553cfcb9ce226d20065267753eb5ed94'),
         target_files=1939, target_bytes=2120281521),
    dict(root='guarded-mlp-core-checked-attention-diagnostic-cpu-v228-v1',
         schema='ferric-guarded-mlp-core-checked-attention-diagnostic-cpu-v1',
         receipt=dict(bytes=250224, sha256='03fb3430315c24fc2eedfb5aebfbbe9dd1b049fdc70ebfd240886fcf7d004b47'),
         archive='guarded-mlp-core-checked-attention-diagnostic-evidence-v228-v1.tar.gz',
         archive_pin=dict(bytes=2902014, sha256='3e32d1c027787649a96acae11ba7b8f8f3805530332cbd22a07df41bf4cacceb'),
         source_map=dict(bytes=2113321, sha256='f83112c5a13838c5006d94b2866498a8971637a96f785f36487dcbe3b97044e0'),
         target_files=1939, target_bytes=2122561681),
    dict(root='guarded-mlp-core-checked-attention-diagnostic-cpu-v228-v2',
         schema='ferric-guarded-mlp-core-checked-attention-diagnostic-cpu-v1',
         receipt=dict(bytes=250224, sha256='08f77b3f31cd698651ff4f19d561c81693295e71072aa34e6d9f6949e330d4df'),
         archive='guarded-mlp-core-checked-attention-diagnostic-evidence-v228-v2.tar.gz',
         archive_pin=dict(bytes=2902248, sha256='0bf1ff51aae637320ab31268d98ca083f85d735ddf8f35e8ee8a327937f02f1c'),
         source_map=dict(bytes=2113321, sha256='61559489584c87dd1ca5770d0ea6750ec4bc9667c87cb5d012eeffe9d7507656'),
         target_files=1939, target_bytes=2122498291),
)
assert __debug__ and os.getuid() == os.geteuid() == 0
assert os.environ.get('SUDO_UID') == '9661'
os.umask(0o077)
assert os.uname().nodename == 'smci350-rck-g03-b19-03' and E.resolve() == E
assert not os.path.lexists(OUTPUT) and shutil.rmtree.avoids_symlink_attacks
signal.alarm(180)


def pin(path):
    assert path.resolve() == path and stat.S_ISREG(path.lstat().st_mode)
    body = path.read_bytes()
    return {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}


preserved, targets, reports = {}, [], []
for generation in GENERATIONS:
    root = E / generation['root']
    archive = E / generation['archive']
    assert root.resolve() == root and root.stat().st_uid == 9661
    assert not os.path.lexists(root / 'evidence/complete.json')
    assert pin(archive) == generation['archive_pin']
    receipt = root / 'evidence/failed.json'
    assert pin(receipt) == generation['receipt']
    r = json.loads(receipt.read_bytes())
    assert r['schema'] == generation['schema'] and r['passed'] is False
    assert r['failure'] is not None and r['postcheck_errors'] == [] and r['source_unchanged'] is True
    assert r['qualification_passed'] is False and r['diagnostic_only'] is True
    assert len(r['phases']) == 19 and len(r['raw']) == 99
    assert [row['exit_code'] for row in r['phases']] == [0] * 18 + [101]
    assert all(row['natural_exit'] and not row['timed_out'] and not row['forced_cleanup']
               and row['reaped'] and row['process_group_absent'] for row in r['phases'])
    assert all(not Path('/proc/' + str(row['pid'])).exists() for row in r['phases'])
    assert r['controller']['path'] == str(root / 'run_cpu.py')
    assert r['input_manifest']['path'] == str(root / 'input-manifest.json')
    assert all(Path(name).name == name and row['path'] == str(root / 'evidence' / name)
               for name, row in r['raw'].items())
    assert r['input_sources'] == r['raw']['sources-before.json']
    assert r['final_sources'] == r['raw']['sources-after.json']
    assert all({key: r[field][key] for key in ('bytes', 'sha256')} == generation['source_map']
               for field in ('input_sources', 'final_sources'))
    generation_pins = {receipt: generation['receipt'], archive: generation['archive_pin']}
    for row in list(r['raw'].values()) + [r['controller'], r['input_manifest']]:
        path = Path(row['path'])
        expected = {key: row[key] for key in ('bytes', 'sha256')}
        assert pin(path) == expected
        generation_pins[path] = expected
    assert len(generation_pins) == 103 and not (set(generation_pins) & set(preserved))
    preserved.update(generation_pins)
    rows = []
    for name in ('target', 'tmp'):
        path = root / name
        assert path.resolve() == path and stat.S_ISDIR(path.lstat().st_mode)
        assert path.stat().st_uid == 9661
        if name == 'tmp':
            assert not any(path.iterdir())
        count = size = 0
        for base, dirs, files in os.walk(path, followlinks=False):
            for entry_name in dirs + files:
                entry = Path(base) / entry_name
                value = entry.lstat()
                assert value.st_uid == 9661 and not stat.S_ISLNK(value.st_mode)
                assert stat.S_ISDIR(value.st_mode) or stat.S_ISREG(value.st_mode)
                if stat.S_ISREG(value.st_mode):
                    count += 1
                    size += value.st_size
                    assert count <= generation['target_files'] and size <= generation['target_bytes']
        rows.append(dict(path=str(path), files=count, logical_bytes=size))
        targets.append(path)
    assert rows[0]['files'] == generation['target_files']
    assert rows[0]['logical_bytes'] == generation['target_bytes']
    assert rows[1]['files'] == rows[1]['logical_bytes'] == 0
    reports.append(dict(root=str(root), removed=rows, preserved_pin_count=103,
                        receipt=generation['receipt'], archive=generation['archive_pin'],
                        source_map=generation['source_map']))
assert len(targets) == len(set(targets)) == 6 and len(preserved) == 309

# Prevalidate every generation before checking all six cache paths together.
prefixes = [str(path) for path in targets]
for proc in Path('/proc').iterdir():
    if not proc.name.isdigit() or int(proc.name) == os.getpid():
        continue
    try:
        if proc.stat().st_uid != 9661:
            continue
        bodies = [(proc / name).read_bytes() for name in ('cmdline', 'maps')]
        for name in ('cwd', 'exe'):
            bodies.append(os.readlink(proc / name).encode())
        for entry in (proc / 'fd').iterdir():
            try:
                bodies.append(os.readlink(entry).encode())
            except FileNotFoundError:
                pass
        assert not any(prefix.encode() in body for prefix in prefixes for body in bodies), proc.name
    except (FileNotFoundError, ProcessLookupError):
        continue

# Privilege is used only for protected process metadata; deletion stays unprivileged.
account = pwd.getpwuid(9661)
assert account.pw_name == 'harmenon'
os.initgroups(account.pw_name, account.pw_gid)
os.setgid(account.pw_gid)
os.setuid(9661)
assert os.getuid() == os.geteuid() == 9661 and os.getgid() == account.pw_gid
assert all(pin(path) == expected for path, expected in preserved.items())
before = shutil.disk_usage(E).free
roots = {E / generation['root'] for generation in GENERATIONS}
for path in targets:
    assert path.resolve() == path and path.parent in roots and path.name in ('target', 'tmp')
    shutil.rmtree(path)
assert all(not os.path.lexists(path) for path in targets)
assert all(pin(path) == expected for path, expected in preserved.items())
result = dict(schema='ferric-obsolete-diagnostic-cache-retirement-v1',
              elevated_process_inspection=True, deletion_uid=os.getuid(), generations=reports,
              preserved_pin_count=len(preserved), free_before=before,
              free_after=shutil.disk_usage(E).free, source_tree_retained=True,
              successful_generation_touched=False)
with OUTPUT.open('x') as stream:
    json.dump(result, stream, sort_keys=True, indent=2)
    stream.write('\n')
print(json.dumps(result, sort_keys=True))
