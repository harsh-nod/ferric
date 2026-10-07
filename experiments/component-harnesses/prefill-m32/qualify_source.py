"""Fresh full-source CPU qualification for the M32 native component harness."""
import ast
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
I = D / 'inputs/prefill-m32-harness-a001'
O = D / 'prefill-m32-harness-a001'
Q = D / 'inputs/prefill-m32-m2-a001/qualify_host_followup.py'
Q_SHA = '999a97fdf6b551f7428b5daf5d08497f86a1d331c28cf2f24f17ac4cc849fd0d'


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    require(len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'reviewed archive SHA256')
    require(Q.resolve(strict=True) == Q and Q.stat().st_uid == 1046 and Q.stat().st_size < 32768,
            'bounded owned qualification helper')
    raw = Q.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == Q_SHA, 'frozen G40 helper')
    q = importlib.util.module_from_spec(importlib.util.spec_from_file_location('host_qualifier', Q))
    exec(compile(raw, str(Q), 'exec'), q.__dict__)
    q.environment()
    before = q.allocation(64 * 1024**2)
    archive = I / 'source.tar.gz'
    require(q.digest(archive) == sys.argv[1] and archive.stat().st_size < 8 * 1024**2,
            'bounded exact archive')
    files, total = {}, 0
    with tarfile.open(archive, 'r:gz') as packed:
        for index, entry in enumerate(packed, 1):
            path = Path(entry.name)
            require(index <= 128 and entry.isfile() and entry.name == str(path)
                    and not path.is_absolute() and '..' not in path.parts
                    and path.suffix == '.py' and entry.name not in files
                    and 0 < entry.size < 1024**2, 'distinct bounded Python source')
            total += entry.size
            require(total < 8 * 1024**2, 'bounded source expansion')
            files[entry.name] = packed.extractfile(entry).read(entry.size + 1)
            require(len(files[entry.name]) == entry.size, 'complete source bytes')
    require(files.get('qualify_source.py') == Path(__file__).read_bytes(), 'qualifier in source closure')
    tests = []
    for name, content in sorted(files.items()):
        if '/' in name or not name.startswith('test_'):
            continue
        for node in ast.parse(content, filename=name).body:
            if isinstance(node, ast.ClassDef):
                tests.extend((Path(name).stem, node.name, method.name) for method in node.body
                             if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'))
    require(15 < len(tests) <= 128 and len(tests) == len(set(tests)), 'full distinct harness test roster')
    roster = {name: hashlib.sha256(content).hexdigest() for name, content in files.items()}
    os.umask(0o077)
    O.mkdir(mode=0o700)
    source = O / 'source'
    source.mkdir(mode=0o700)
    for name, content in files.items():
        path = source / name
        path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(content)
    command = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
               '-s', str(source), '-p', 'test_*.py', '-v']
    record = {'schema': 'FerricM32HarnessQualificationV1', 'accepted': False,
        'source_root': str(source), 'source_archive_sha256': sys.argv[1],
        'source_before': roster, 'helper_sha256': q.digest(Path(__file__).resolve()), 'argv': command,
        'planning_increment_bytes': 64 * 1024**2, 'stage_before_bytes': before,
        'native_executed': False, 'native_qualified': False, 'tests_expected': len(tests)}
    try:
        with (O / 'stdout').open('xb') as stdout, (O / 'stderr').open('xb') as stderr:
            result = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                                    timeout=300, check=False)
        record['returncode'] = result.returncode
        require(result.returncode == 0 and (O / 'stdout').stat().st_size == 0, 'successful silent tests')
        lines = [method + ' (' + '.'.join((module, cls, method)) + ') ... ok'
                 for module, cls, method in sorted(tests)]
        prefix = '\n'.join(lines) + '\n\n' + '-' * 70 + '\n'
        log = (O / 'stderr').read_text()
        require(log.startswith(prefix) and re.fullmatch(r'Ran ' + str(len(tests)) +
                r' tests in [0-9]+\.[0-9]+s\n\nOK\n', log[len(prefix):]), 'complete exact test output')
        record['source_after'] = {str(p.relative_to(source)): q.digest(p)
                                  for p in source.rglob('*') if p.is_file()}
        require(record['source_after'] == roster, 'unchanged complete source closure')
        q.environment()
        record['stage_after_bytes'] = q.allocation()
        require(record['stage_after_bytes'] - before <= 64 * 1024**2, 'charged stage growth')
        record['accepted'] = True
    except BaseException as error:
        record['error'] = type(error).__name__ + ': ' + str(error)
    q.save(O / 'receipt.json', record)
    print(json.dumps(record, sort_keys=True), flush=True)
    return 0 if record['accepted'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
