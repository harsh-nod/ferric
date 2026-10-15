"""Pin a closed retained-capture replay input set; no project code execution."""
import hashlib
import json
from pathlib import Path
import stat

ROOT = Path(__file__).resolve().parent
E = ROOT.parent
CPU = E / 'guarded-mlp-finite-predicate-inline-cpu-v228-v3'
PAIR = E / 'guarded-mlp-early-stop-compile-v228-v18'
ARCHIVE = E / 'guarded-mlp-finite-predicate-inline-publication-v228-v1/originals.tar.gz'
CPU_SHA = '41d0425beff3ebef1c98910bc904402452c83cddbb8c72229cb9c05e42787a1a'
FIXED_SHA = '65f856d211c78cdbc4322060fbfea200f78299b8504032ff4f55db8c4e897017'


def pin(path):
    before = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
    assert before.st_size <= 512 << 20
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            h.update(chunk)
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert stamp(before) == stamp(path.lstat())
    return dict(path=str(path), bytes=before.st_size, sha256=h.hexdigest())


assert ROOT.name == 'guarded-mlp-fixed-formal-replay-cpu-v228-v1'
assert not (ROOT / 'input-manifest.json').exists()
assert pin(CPU / 'evidence/complete.json')['sha256'] == CPU_SHA
cpu = json.loads((CPU / 'evidence/complete.json').read_bytes())
assert cpu['passed'] and cpu['source_unchanged'] and not cpu['postcheck_errors']
names = [name for name in cpu['inventory'] if name.endswith(
    '::actual_mlp_v39_reaches_formal_archive_and_outer_lineage_replay')]
assert len(names) == 1 and names[0] in cpu['ignored'] and len(cpu['ignored']) == 24
assert pin(PAIR / 'fixed-run/evidence/result.json')['sha256'] == FIXED_SHA
paths = {Path(row['path']) for row in cpu['final_sources'].values()}
paths |= {Path(row['path']) for row in cpu['tool_pins'].values()}
paths |= {Path(row['pin']['path']) for row in cpu['artifacts'].values()}
paths |= {ROOT / 'run.py', ROOT / 'prepare_inputs.py', CPU / 'evidence/complete.json',
          CPU / 'evidence/sources-after.json', CPU / 'input-manifest.json',
          CPU / 'evidence/compiler-tests.command.json', ARCHIVE}
paths |= {PAIR / ('fixed-run/evidence/' + name) for name in (
    'result.json', 'compile.stderr', 'sources-before.json', 'sources-after.json')}
paths |= {PAIR / ('fixed-run/fe2o3-engineering-diagnostics-v1/' + name)
          for name in ('semantic-mir-v1.bin', 'semantic-source-map-v1.json')}
fixture = sorted(path for path in (PAIR / 'fixed').rglob('*') if path.is_file())
assert len(fixture) == 7
paths |= set(fixture)
files = {str(path): pin(path) for path in sorted(paths)}
for row in cpu['final_sources'].values():
    assert files[row['path']] == row
for row in cpu['tool_pins'].values():
    assert files[row['path']] == row
for row in cpu['artifacts'].values():
    assert files[row['pin']['path']] == row['pin']
assert files[str(ARCHIVE)]['sha256'] == '65008a7eb7aeb5435beac87a56a3138568ca6fdba18552a282b91894f6e4c68d'
capture = PAIR / 'fixed-run/fe2o3-engineering-diagnostics-v1/semantic-mir-v1.bin'
assert files[str(capture)]['bytes'] == 957157
assert files[str(capture)]['sha256'] == '7a149f67fd9fa233c6e72f038c6462016475e585bc14172a9b70cc94ed958bed'
value = dict(schema='ferric-fixed-formal-replay-input-v1', cpu_receipt_sha256=CPU_SHA,
             fixed_receipt_sha256=FIXED_SHA, selected_test=names[0], files=files)
raw = (json.dumps(value, indent=2, sort_keys=True) + '\n').encode()
with (ROOT / 'input-manifest.json').open('xb') as stream:
    stream.write(raw)
print(json.dumps(dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(),
                      input_count=len(files), selected_test=names[0]), sort_keys=True))
