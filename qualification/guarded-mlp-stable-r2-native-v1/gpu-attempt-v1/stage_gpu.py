"""Stage six fixed data/source inputs into a fresh GPU diagnostic namespace."""
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-stable-r2-native-gpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-stable-r2-native-gpu-input-v228-v2.tar.gz'
NAMES = {'run_gpu.py', 'supervisor.py', 'library_audit.py', 'verify_native.py',
         'request.json', 'input-manifest.json'}


def main():
    signal.alarm(30)
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert not os.path.lexists(ROOT) and ROOT.parent.resolve(strict=True) == ROOT.parent
    assert ARCHIVE.resolve(strict=True) == ARCHIVE and ARCHIVE.is_file() and not ARCHIVE.is_symlink()
    body = ARCHIVE.read_bytes()
    assert len(body) == 26597 and hashlib.sha256(body).hexdigest() == '5a09e67fd63c8aca2a43b4aa1042f338ffbabc5d414af0b2e9f526986d5bc2ee'
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == 6 and {m.name for m in members} == NAMES
        assert all(m.isfile() and m.size <= 128 << 10 for m in members)
        files = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(files['input-manifest.json'])
    assert hashlib.sha256(files['input-manifest.json']).hexdigest() == '1dcbb5d0b8a1474e5b8647cd4385d7a26b3df152f06f7dd3cd3ebe5c4ade4064'
    assert set(manifest['files']) == NAMES - {'input-manifest.json'}
    for name, pin in manifest['files'].items():
        assert pin == dict(bytes=len(files[name]), sha256=hashlib.sha256(files[name]).hexdigest())
    os.umask(0o077)
    ROOT.mkdir(mode=0o700)
    for name, content in files.items():
        with (ROOT / name).open('xb') as stream:
            stream.write(content)
        assert (ROOT / name).read_bytes() == content
    print(json.dumps(dict(root=str(ROOT), staged=True, members=6, executed=False)))


if __name__ == '__main__':
    main()
