"""Format a fresh parent-only layer-zero capture proposal with bounded leaves."""
import hashlib
import json
import os
from pathlib import Path
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
SOURCE = E / 'p228-layer0-native-capture-v1'
DEST = E / 'p228-layer0-native-capture-source-v1'
OUT = E / 'layer0-native-capture-format-v228-v1'
MANIFEST_SHA = '5832be8568087725f2d606decc8f8fa72430f4414a99b9e31204b40d6a776858'
PREFIX = 'adapters/m1-engineering-execution-v1/'
FILES = {PREFIX + name for name in (
    'Cargo.toml', 'src/tp_finite_client/prefix_layer.rs',
    'src/tp_finite_client/prefix_layer/evidence.rs',
    'src/tp_finite_client/prefix_layer/capture.rs',
    'src/tp_finite_client/prefix_layer/capture_tests.rs',
    'src/tp_finite_client/prefix_layer/capture_evidence_tests.rs',
    'src/bin/ferric-qwen3-finite-prefix-layer-capture-engineering.rs')}


def require(ok, reason):
    if not ok:
        raise RuntimeError(reason)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical regular file')
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and len(sys.argv) == 1, 'plain bounded formatter')
    require(not any(os.path.lexists(p) for p in (OUT, DEST)), 'fresh outputs')
    manifest = pin(SOURCE / 'source-manifest.json')
    require(manifest['sha256'] == MANIFEST_SHA, 'frozen reviewed source proposal')
    value = json.loads((SOURCE / 'source-manifest.json').read_bytes())
    require(value['schema'] == 'ferric-p228-layer0-native-capture-source-proposal-v1'
            and value['base_commit'] == '1a9a2551ee7af44d5482cc37f46cf008158b8d10'
            and value['formatted'] is value['compiled'] is value['tests_executed'] is False
            and value['authored_tests'] == 14 and len(value['files']) == 7
            and {row['path'] for row in value['files']} == FILES, 'exact unexecuted parent delta')
    before = {}
    for row in value['files']:
        require(row['source'] == 'draft/' + row['path'], 'closed source path')
        actual = pin(SOURCE / row['source'])
        require({key: actual[key] for key in ('bytes', 'sha256')} == row['after'], 'source body')
        before[row['path']] = actual
    require({str(p.relative_to(SOURCE / 'draft')) for p in (SOURCE / 'draft').rglob('*')
             if p.is_file()} == FILES, 'complete seven-file source census')
    require(all(not p.is_symlink() and (p.is_dir() or p.is_file())
                for p in SOURCE.rglob('*')), 'ordinary source entries')
    helper = R / 'evidence/wave-output-lowering-v216/bounded.py'
    helper_pin = pin(helper)
    require(helper_pin['sha256'] == 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1',
            'unchanged bounded helper')
    n = types.ModuleType('layer0_capture_formatter')
    n.__file__ = str(helper)
    exec(compile(helper.read_bytes(), str(helper), 'exec'), n.__dict__)
    OUT.mkdir(mode=0o700)
    n.F, n.T = OUT, OUT / 'scratch'
    n.setup()
    for name, original in before.items():
        require(pin(Path(original['path'])) == original, 'source unchanged before copy')
        target = DEST / 'draft' / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(Path(original['path']).read_bytes())
    tool, controller = pin(n.N / 'bin/rustfmt'), pin(Path(__file__).resolve())
    n.save(OUT / 'sources-before.json', before)
    argv = [tool['path'], '--edition', '2024', '--config', 'skip_children=true',
            *(str(DEST / 'draft' / name) for name in sorted(FILES) if name.endswith('.rs'))]
    phases = {'rustfmt': n.run(OUT, 'rustfmt', argv, deadline=60),
              'rustfmt-check': n.run(OUT, 'rustfmt-check', [argv[0], '--check', *argv[1:]], deadline=60)}
    after = {name: pin(DEST / 'draft' / name) for name in sorted(FILES)}
    n.save(OUT / 'sources-after.json', after)
    require(all(pin(Path(p['path'])) == p for p in before.values()), 'original sources unchanged')
    require(all(pin(Path(p['path'])) == p for p in (manifest, helper_pin, tool, controller)),
            'source manifest/helper/tool/controller unchanged')
    n.save(OUT / 'complete.json', dict(schema='ferric-p228-layer0-native-capture-format-v1', passed=True,
        controller=controller, tool=tool, helper=helper_pin, manifest=manifest, phases=phases,
        formatted=after, tests_executed=False, gpu_execution=False,
        raw={p.name: pin(p) for p in OUT.iterdir() if p.is_file()}))
    for row in value['files']:
        row['after'] = {key: after[row['path']][key] for key in ('bytes', 'sha256')}
    value['formatted'] = True
    value['format_receipt'] = pin(OUT / 'complete.json')
    n.save(DEST / 'source-manifest.json', value)
    print(json.dumps(dict(receipt=pin(OUT / 'complete.json'), manifest=pin(DEST / 'source-manifest.json'))), flush=True)


if __name__ == '__main__':
    main()
