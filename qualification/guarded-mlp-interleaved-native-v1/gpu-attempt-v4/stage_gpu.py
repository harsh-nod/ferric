"""Stage nine fixed data/source inputs into a fresh paired GPU namespace."""
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-interleaved-native-gpu-v228-v4'
ARCHIVE = E / 'guarded-mlp-interleaved-native-gpu-input-v228-v4.tar.gz'
NAMES = {'run_gpu.py', 'supervisor.py', 'library_audit.py', 'verify_native.py',
         'request.json', 'input-manifest.json', 'r1.hsaco', 'mlp.hsaco', 'guarded.hsaco'}


def main():
    signal.alarm(30)
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert not os.path.lexists(ROOT) and ROOT.parent.resolve(strict=True) == ROOT.parent
    assert ARCHIVE.resolve(strict=True) == ARCHIVE and ARCHIVE.is_file() and not ARCHIVE.is_symlink()
    body = ARCHIVE.read_bytes()
    assert len(body) == 58587 and hashlib.sha256(body).hexdigest() == 'aec4bc99cc7d286b40382dbe08c04f90051d4e181c1608cf7f198b74fc092fa2'
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == 9 and {m.name for m in members} == NAMES
        assert all(m.isfile() and m.size <= 128 << 10 for m in members)
        files = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(files['input-manifest.json'])
    assert hashlib.sha256(files['input-manifest.json']).hexdigest() == '15ba3475170832512f2a742aab4624d7d139009d8c6b3aa11c95f7891fce0c9c'
    assert set(manifest['files']) == NAMES - {'input-manifest.json'}
    for name, pin in manifest['files'].items():
        assert pin == dict(bytes=len(files[name]), sha256=hashlib.sha256(files[name]).hexdigest())
    os.umask(0o077)
    ROOT.mkdir(mode=0o700)
    for name, content in files.items():
        with (ROOT / name).open('xb') as stream:
            stream.write(content)
        assert (ROOT / name).read_bytes() == content
    print(json.dumps(dict(root=str(ROOT), staged=True, members=9, executed=False)))


if __name__ == '__main__':
    main()
