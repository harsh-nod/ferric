"""Remove only the retained failed generation's unused build and temporary cache."""
import hashlib
import json
import os
import pwd
from pathlib import Path
import shutil
import signal
import stat

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-ranked-graph-work-diagnostic-cpu-v228-v1')
ARCHIVE = ROOT.parent / 'guarded-mlp-ranked-graph-work-diagnostic-cpu-evidence-v228-v1.tar.gz'
OUTPUT = ROOT.parent / 'guarded-mlp-ranked-graph-work-diagnostic-failed-cache-retirement-v228-v2.json'
assert __debug__ and os.getuid() == os.geteuid() == 0
assert os.environ.get('SUDO_UID') == '9661'
os.umask(0o077)
assert os.uname().nodename == 'smci350-rck-g03-b19-03' and ROOT.resolve() == ROOT
assert not os.path.lexists(OUTPUT)
signal.alarm(180)

def pin(path):
    assert path.resolve() == path and stat.S_ISREG(path.lstat().st_mode)
    body = path.read_bytes()
    return {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}

assert pin(ARCHIVE) == {'bytes': 5513950, 'sha256': '6f3782741c51645ce2aba2e1fa79749bb6af324a58e51cbb6a72e399f9b30740'}
receipt = ROOT / 'evidence/failed.json'
assert pin(receipt) == {'bytes': 888900, 'sha256': 'cc75239a5b500eaf515ebe904e0710c2852864efabdaf466d7de4b10a82870eb'}
r = json.loads(receipt.read_bytes())
assert r['passed'] is False and len(r['phases']) == 15
assert r['postcheck_errors'] == [] and r['source_unchanged'] is True
assert all(row['reaped'] and row['process_group_absent'] for row in r['phases'])
assert all(not Path('/proc/' + str(row['pid'])).exists() for row in r['phases'])
preserved = {receipt: pin(receipt), ARCHIVE: pin(ARCHIVE)}
for row in list(r['raw'].values()) + [r['controller'], r['input_manifest']]:
    path = Path(row['path'])
    expected = {key: row[key] for key in ('bytes', 'sha256')}
    assert pin(path) == expected
    preserved[path] = expected
targets = [ROOT / name for name in ('target', 'tmp')]
rows = []
for path in targets:
    assert path.resolve() == path and stat.S_ISDIR(path.lstat().st_mode)
    assert path.stat().st_uid == 9661
    count = size = 0
    for base, dirs, files in os.walk(path, followlinks=False):
        for name in dirs + files:
            entry = Path(base) / name
            value = entry.lstat()
            assert value.st_uid == 9661 and not stat.S_ISLNK(value.st_mode)
            assert stat.S_ISDIR(value.st_mode) or stat.S_ISREG(value.st_mode)
            if stat.S_ISREG(value.st_mode):
                count += 1
                size += value.st_size
    rows.append({'path': str(path), 'files': count, 'logical_bytes': size})
assert rows[0]['files'] == 2028 and rows[0]['logical_bytes'] == 2445502744
assert rows[1]['files'] == rows[1]['logical_bytes'] == 0

# Refuse if an owned process still references either cache through code or handles.
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
before = shutil.disk_usage(ROOT).free
for path in targets:
    assert path.resolve() == path and path.parent == ROOT
    shutil.rmtree(path)
assert all(not os.path.lexists(path) for path in targets)
assert all(pin(path) == expected for path, expected in preserved.items())
result = {'schema': 'ferric-failed-diagnostic-cache-retirement-v2',
          'elevated_process_inspection': True, 'deletion_uid': os.getuid(), 'removed': rows,
          'preserved_pin_count': len(preserved), 'receipt': preserved[receipt],
          'archive': preserved[ARCHIVE], 'free_before': before,
          'free_after': shutil.disk_usage(ROOT).free, 'source_tree_retained': True,
          'successful_generation_touched': False}
with OUTPUT.open('x') as stream:
    json.dump(result, stream, sort_keys=True, indent=2)
    stream.write('\n')
print(json.dumps(result, sort_keys=True))
