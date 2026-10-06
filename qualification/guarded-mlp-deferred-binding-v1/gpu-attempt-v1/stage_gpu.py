"""Stage nine fixed data/source inputs into a fresh paired GPU namespace."""
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-deferred-binding-gpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-deferred-binding-gpu-input-v228-v1.tar.gz'
ARCHIVE_PIN = dict(bytes=59995, sha256='b65483bbe6370095740b6bd1c4e0027f2f6826910c726e48ee2f485c42e24738')
INPUT_SHA = 'af0a2763c87466f51f206711019c471b393fe5336ac33636ce78da5ef94e845d'
NAMES = {'run_gpu.py', 'supervisor.py', 'library_audit.py', 'verify_native.py',
         'request.json', 'input-manifest.json', 'r1.hsaco', 'mlp.hsaco', 'guarded.hsaco'}


def main():
    signal.alarm(30)
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert not os.path.lexists(ROOT) and ROOT.parent.resolve(strict=True) == ROOT.parent
    assert ARCHIVE.resolve(strict=True) == ARCHIVE and ARCHIVE.is_file() and not ARCHIVE.is_symlink()
    body = ARCHIVE.read_bytes()
    assert dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest()) == ARCHIVE_PIN
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == 9 and {m.name for m in members} == NAMES
        assert all(m.isfile() and m.size <= 128 << 10 for m in members)
        files = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(files['input-manifest.json'])
    assert hashlib.sha256(files['input-manifest.json']).hexdigest() == INPUT_SHA
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
