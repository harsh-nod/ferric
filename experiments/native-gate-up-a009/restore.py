"""Restore the retained gate/up source into a new, separate directory."""
import argparse
import hashlib
import json
from pathlib import Path
import stat


def restore(repository, output):
    here = Path(__file__).resolve().parent
    repository = repository.resolve(strict=True)
    output = output.absolute()
    if output.exists() or output.is_symlink() or output.is_relative_to(repository):
        raise ValueError('output must be a new directory outside the repository')
    if output.parent.resolve(strict=True) != output.parent:
        raise ValueError('output parent must be canonical')
    manifest = json.loads((here / 'manifest.json').read_bytes())
    if manifest['schema'] != 'FerricRetainedSourceSnapshotV1':
        raise ValueError('unsupported snapshot schema')
    sources = []
    for name, row in manifest['files'].items():
        relative = Path(name)
        if relative.is_absolute() or '..' in relative.parts or str(relative) != name:
            raise ValueError('noncanonical snapshot path')
        if row['source'] not in ('repository', 'source-overlay'):
            raise ValueError('unknown source location')
        root = repository if row['source'] == 'repository' else here / 'source-overlay'
        path = root / relative
        if path.resolve(strict=True) != path or not stat.S_ISREG(path.lstat().st_mode):
            raise ValueError('snapshot input is not a canonical regular file: ' + name)
        data = path.read_bytes()
        if len(data) != row['bytes'] or hashlib.sha256(data).hexdigest() != row['sha256']:
            raise ValueError('snapshot input changed: ' + name)
        if row['mode'] not in (0o600, 0o644, 0o700, 0o755):
            raise ValueError('unexpected snapshot mode')
        sources.append((relative, row['mode'], data))
    output.mkdir(mode=0o700)
    for relative, mode, data in sources:
        target = output / relative
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with target.open('xb') as stream:
            stream.write(data)
        target.chmod(mode)
    return len(sources)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps({'restored_files': restore(args.repository, args.output),
                      'built': False, 'performance_qualified': False}))
