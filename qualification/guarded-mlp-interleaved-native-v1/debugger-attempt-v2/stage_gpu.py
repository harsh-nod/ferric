"""Stage eleven frozen inputs into one fresh debugger namespace."""
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-interleaved-debugger-gpu-v228-v2'
ARCHIVE = E / 'guarded-mlp-interleaved-debugger-input-v228-v2.tar.gz'
NAMES = {'run_gpu.py', 'supervisor.py', 'library_audit.py', 'verify_native.py',
         'owned_children.py', 'capture.gdb', 'request.json', 'input-manifest.json',
         'r1.hsaco', 'mlp.hsaco', 'guarded.hsaco'}


def main():
    signal.alarm(30)
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert not os.path.lexists(ROOT) and ROOT.parent.resolve(strict=True) == ROOT.parent
    assert ARCHIVE.resolve(strict=True) == ARCHIVE and ARCHIVE.is_file() and not ARCHIVE.is_symlink()
    body = ARCHIVE.read_bytes()
    assert len(body) == 65864 and hashlib.sha256(body).hexdigest() == '3f345210a0e3b0ce0e8839a023851c7734a4e10772c25789fced74d78654efae'
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == 11 and {m.name for m in members} == NAMES
        assert all(m.isfile() and m.size <= 128 << 10 for m in members)
        files = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(files['input-manifest.json'])
    assert hashlib.sha256(files['input-manifest.json']).hexdigest() == '0d293ee7fb911e9aa8d7eb3ed8adc1f4a2ab8db324063eddb1c2313057905c51'
    assert set(manifest) == {'schema', 'files'} and manifest['schema'] == 'ferric-interleaved-mlp-debugger-input-v1'
    assert set(manifest['files']) == NAMES - {'input-manifest.json'}
    for name, pin in manifest['files'].items():
        assert pin == dict(bytes=len(files[name]), sha256=hashlib.sha256(files[name]).hexdigest())
    os.umask(0o077)
    ROOT.mkdir(mode=0o700)
    for name, content in files.items():
        with (ROOT / name).open('xb') as stream:
            stream.write(content)
        assert (ROOT / name).read_bytes() == content
    print(json.dumps(dict(root=str(ROOT), staged=True, members=11, executed=False)))


if __name__ == '__main__':
    main()
