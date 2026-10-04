"""Format frozen Ferric worker, parent and Down2 proposals on the owned CPU host."""
import hashlib
import json
import os
from pathlib import Path
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
OUT = E / 'clock-and-down2-format-v228-v3'
SPECS = (
    ('p228-gfx950-clock-recorder-v1', 'p228-gfx950-clock-recorder-source-v2', 12,
     '0f9a0c190a02e6c39c8e7fe11b545e66758ed758d68ae647ae18beeb468fa01a',
     'adapters/tp-peer-finite-engineering-worker-v1/src/'),
    ('p228-gfx950-clock-parent-v1', 'p228-gfx950-clock-parent-source-v2', 8,
     '76dac508791b63fdf361fd1f088cc0ed56ac4176492adfa1d12dcab198f6c74f',
     'adapters/m1-engineering-execution-v1/'),
    ('p228-mlp-down-two-row-v1', 'p228-mlp-down-two-row-source-v2', 3,
     '1a077037611a1c931e4f060cb6142340964fcfe5452d8fd12399e247df2492da',
     'device/qwen3-tp-wave-rmsnorm-kernels-v15/'),
)


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical regular file')
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and len(sys.argv) == 1, 'plain bounded formatter')
    require(not OUT.exists(), 'fresh formatting output')
    before, manifests, copies = {}, {}, []
    for source_name, dest_name, count, digest, prefix in SPECS:
        source, dest = E / source_name, E / dest_name
        require(not os.path.lexists(dest), 'fresh formatted source directory')
        manifest = pin(source / 'source-manifest.json')
        require(manifest['sha256'] == digest, 'frozen proposal manifest')
        value = json.loads((source / 'source-manifest.json').read_bytes())
        require(value['base_commit'] == '924703865d9cfcb1dc791af211b7558ff7c58894'
                and len(value['files']) == count, 'exact base and file count')
        manifests[source_name] = manifest
        before[source_name] = {}
        for row in value['files']:
            relative = Path(row['path'])
            require(not relative.is_absolute() and '..' not in relative.parts
                    and relative.as_posix() == row['path'] and row['path'].startswith(prefix)
                    and (relative.suffix == '.rs' or row['path'] == 'adapters/m1-engineering-execution-v1/Cargo.toml')
                    and row.get('source', 'draft/' + row['path']) == 'draft/' + row['path'],
                    'bounded Rust source path')
            require(row['path'] not in before[source_name], 'unique source path')
            actual = pin(source / 'draft' / row['path'])
            require({key: actual[key] for key in ('bytes', 'sha256')} == row['after'], 'frozen source body')
            before[source_name][row['path']] = actual
            copies.append((actual, dest / 'draft' / row['path']))
        require({str(p.relative_to(source / 'draft')) for p in (source / 'draft').rglob('*') if p.is_file()}
                == set(before[source_name]), 'complete source body census')
        require(all(not p.is_symlink() for p in (source / 'draft').rglob('*')), 'no source symlinks')
    helper = R / 'evidence/wave-output-lowering-v216/bounded.py'
    require(pin(helper)['sha256'] == 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1', 'bounded helper')
    n = types.ModuleType('clock_down2_formatter')
    n.__file__ = str(helper)
    exec(compile(helper.read_bytes(), str(helper), 'exec'), n.__dict__)
    OUT.mkdir(mode=0o700)
    n.F, n.T = OUT, OUT / 'scratch'
    n.setup()
    for original, target in copies:
        require(pin(Path(original['path'])) == original, 'source unchanged before copying')
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(Path(original['path']).read_bytes())
    tool, controller = pin(n.N / 'bin/rustfmt'), pin(Path(__file__).resolve())
    n.save(OUT / 'sources-before.json', before)
    argv = [tool['path'], '--edition', '2024', '--config', 'skip_children=true',
            *(str(target) for _, target in copies if target.suffix == '.rs')]
    phases = {'rustfmt': n.run(OUT, 'rustfmt', argv, deadline=60),
              'rustfmt-check': n.run(OUT, 'rustfmt-check', [argv[0], '--check', *argv[1:]], deadline=60)}
    after = {source_name: {name: pin(E / dest_name / 'draft' / name)
                          for name in before[source_name]}
             for source_name, dest_name, _, _, _ in SPECS}
    n.save(OUT / 'sources-after.json', after)
    require(all(pin(Path(row['path'])) == row for rows in before.values() for row in rows.values()), 'original sources unchanged')
    require(all(pin(Path(row['path'])) == row for row in manifests.values()), 'original manifests unchanged')
    require(pin(Path(tool['path'])) == tool and pin(Path(__file__).resolve()) == controller, 'tool/controller unchanged')
    n.save(OUT / 'complete.json', dict(schema='ferric-clock-and-down2-format-v1', passed=True,
        controller=controller, tool=tool, manifests=manifests, phases=phases, formatted=after,
        tests_executed=False, gpu_execution=False,
        raw={p.name: pin(p) for p in OUT.iterdir() if p.is_file()}))
    print(json.dumps(pin(OUT / 'complete.json')), flush=True)


if __name__ == '__main__':
    main()


