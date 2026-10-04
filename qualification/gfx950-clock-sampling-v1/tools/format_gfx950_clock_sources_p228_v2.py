"""Format a frozen gfx950 clock source delta on ASROCK with bounded leaves."""
import hashlib
import json
import os
from pathlib import Path
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
SOURCE = E / 'p228-gfx950-clock-sampling-v1'
DEST = E / 'p228-gfx950-clock-source-v1'
OUT = E / 'gfx950-clock-source-format-v228-v2'

def require(ok, reason):
    if not ok:
        raise RuntimeError(reason)

def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical regular source')
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())

def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and len(sys.argv) == 2, 'SOURCE_MANIFEST_SHA')
    manifest = pin(SOURCE / 'source-manifest.json')
    readme = pin(SOURCE / 'README.md')
    require(manifest['sha256'] == sys.argv[1], 'frozen input manifest')
    value = json.loads((SOURCE / 'source-manifest.json').read_bytes())
    require(value['schema'] == 'ferric-p228-gfx950-clock-sampling-source-manifest-v1' and len(value['files']) == 10, 'source scope')
    before = {}
    for row in value['files']:
        p = Path(row['path'])
        require(not p.is_absolute() and '..' not in p.parts and str(p).startswith('crates/fe2o3-kfd/src/')
                and p.suffix == '.rs', 'runtime Rust path')
        actual = pin(SOURCE / 'draft' / p)
        require({k: actual[k] for k in ('bytes', 'sha256')} == row['after'], 'draft body')
        require(row['path'] not in before, 'unique path')
        before[row['path']] = actual
    require({str(p.relative_to(SOURCE / 'draft')) for p in (SOURCE / 'draft').rglob('*') if p.is_file()} == set(before), 'source census')
    helper = R / 'evidence/wave-output-lowering-v216/bounded.py'
    require(pin(helper)['sha256'] == 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1', 'bounded helper')
    n = types.ModuleType('gfx950_clock_formatter')
    n.__file__ = str(helper)
    exec(compile(helper.read_bytes(), str(helper), 'exec'), n.__dict__)
    require(not any(os.path.lexists(p) for p in (OUT, DEST)), 'fresh output paths')
    OUT.mkdir(mode=0o700)
    n.F, n.T = DEST, OUT / 'scratch'
    n.setup()
    require({p.name for p in SOURCE.iterdir()} == {"source-manifest.json", "draft", "README.md"}, "exact source root")
    require(all(not p.is_symlink() and (p.is_dir() or p.is_file()) for p in SOURCE.rglob("*")), "ordinary source entries only")
    DEST.mkdir(mode=0o700)
    for name, row in before.items():
        source = SOURCE / "draft" / name
        require(pin(source) == row, "source unchanged before copy")
        target = DEST / "draft" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open("xb") as output:
            output.write(source.read_bytes())
    tool, controller = pin(n.N / 'bin/rustfmt'), pin(Path(__file__).resolve())
    n.save(OUT / 'sources-before.json', before)
    argv = [tool['path'], '--edition', '2024', '--config', 'skip_children=true',
            *(str(DEST / 'draft' / name) for name in before if name.endswith(".rs"))]
    phases = { 'rustfmt': n.run(OUT, 'rustfmt', argv, deadline=60),
        'rustfmt-check': n.run(OUT, 'rustfmt-check', [argv[0], '--check', *argv[1:]], deadline=60) }
    after = {name: pin(DEST / 'draft' / name) for name in before}
    n.save(OUT / 'sources-after.json', after)
    require(all(pin(Path(v['path'])) == v for v in before.values()) and pin(SOURCE / 'source-manifest.json') == manifest, 'original sources unchanged')
    require(pin(SOURCE / 'README.md') == readme, 'proposal scope unchanged')
    require(pin(Path(tool['path'])) == tool and pin(Path(__file__).resolve()) == controller, 'tool/controller unchanged')
    n.save(OUT / 'complete.json', dict(schema='ferric-gfx950-clock-format-v1', passed=True,
        controller=controller, tool=tool, manifest=manifest, phases=phases,
        formatted=after, tests_executed=False, gpu_execution=False,
        raw={p.name: pin(p) for p in OUT.iterdir() if p.is_file()}))
    print(json.dumps(pin(OUT / 'complete.json')), flush=True)

if __name__ == '__main__':
    main()
