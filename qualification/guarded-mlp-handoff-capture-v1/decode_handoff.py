"""Decode the pinned V2 outer wire format as data, without compiler admission."""
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import sys

HANDOFF = {'bytes': 288742, 'sha256': '5f52c141f577162cbc3eda8704b173df7035e8c336b8b436c23558f101fe844c'}
RECEIPT = {'bytes': 26129, 'sha256': '6795b6383e4434ad190af753b757a6f74edfeb3e0b3cf8ae0a8f8404887a03d6'}
IMAGE = {'bytes': 28440, 'sha256': 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'}
DESCRIPTOR = '8cce86c5641366ab51cd4eabb466f91683d791d2e36a9c258fdd2b549a08ff07'


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def read(path):
    before = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
    assert 0 <= before.st_size <= 1 << 20
    body = path.read_bytes()
    after = path.lstat()
    identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert len(body) == before.st_size and identity(before) == identity(after)
    return body


def decode(data):
    assert pin(data) == HANDOFF
    pos = 0

    def take(size):
        nonlocal pos
        assert 0 <= size <= len(data) - pos, 'truncated V2 field'
        result = data[pos:pos + size]
        pos += size
        return result

    def uint(size):
        return int.from_bytes(take(size), 'little')

    magic = b'FE2O3/COMPILER-MODULE-HANDOFF/V2\0'
    assert take(len(magic)) == magic
    target_length = uint(4)
    assert 0 < target_length <= 128
    assert take(target_length) == b'gfx950:xnack-'
    assert uint(1) == 6 and uint(1) == 1  # Code-object 6, LLVM text IR.
    module_sha, module_length = take(32), uint(8)
    envelope_length = uint(4)
    manifest_sha, manifest_length = take(32), uint(8)
    assert 0 < module_length <= 64 << 20
    assert 0 < envelope_length <= 512 << 10
    assert 0 < manifest_length <= 16 << 20
    offsets = {'envelope.bin': pos}
    envelope = take(envelope_length)
    offsets['symbol-manifest.bin'] = pos
    manifest = take(manifest_length)
    offsets['module.ll'] = pos
    module = take(module_length)
    assert pos == len(data), 'trailing V2 bytes'
    assert hashlib.sha256(module).digest() == module_sha
    assert hashlib.sha256(manifest).digest() == manifest_sha
    module.decode('utf-8', errors='strict')
    return {'envelope.bin': envelope, 'symbol-manifest.bin': manifest, 'module.ll': module}, offsets


def main():
    assert __debug__ and sys.dont_write_bytecode and len(sys.argv) == 3
    signal.alarm(30)
    root, out = map(Path, sys.argv[1:])
    assert root.is_absolute() and root.resolve(strict=True) == root
    assert out.is_absolute() and out.parent.resolve(strict=True) == out.parent
    assert not os.path.lexists(out)
    receipt_body = read(root / 'complete.json')
    assert pin(receipt_body) == RECEIPT
    receipt = json.loads(receipt_body)
    assert receipt['passed'] and receipt['compile_passed'] and receipt['capture_verified']
    assert receipt['failure'] is None and receipt['postcheck_errors'] == []
    assert receipt['replay_joins'] == dict(handoff_matches_prior=True,
        handoff_matches_current=True, image_matches_prior=True, descriptor_matches_prior=True)
    inputs = {root / 'complete.json': receipt_body, root / 'compiler-handoff-v2': read(root / 'compiler-handoff-v2')}
    for key in ('observation', 'image'):
        row = receipt['artifact'][key]
        path = Path(row['path'])
        inputs[path] = read(path)
        assert pin(inputs[path]) == {k: row[k] for k in ('bytes', 'sha256')}
    value = json.loads(inputs[Path(receipt['artifact']['observation']['path'])])
    assert value == receipt['artifact']['value']
    assert value['compiler_handoff'] == dict(byte_len=HANDOFF['bytes'], sha256=HANDOFF['sha256'])
    assert value['hsaco']['identity'] == dict(byte_len=IMAGE['bytes'], sha256=IMAGE['sha256'])
    assert pin(inputs[Path(receipt['artifact']['image']['path'])]) == IMAGE
    assert value['hsaco']['canonical_descriptor_sha256'] == DESCRIPTOR
    assert value['authority'] == 'none' and value['grants'] == dict(publication=False, load=False, launch=False)
    bodies, offsets = decode(inputs[root / 'compiler-handoff-v2'])
    script = Path(__file__).resolve(strict=True)
    inputs[script] = read(script)
    bodies['decoder.py'] = inputs[script]
    result = dict(schema='ferric-pinned-compiler-handoff-decode-v1', passed=True,
        host=os.uname().nodename, source_receipt=RECEIPT, handoff=HANDOFF,
        image=IMAGE, canonical_descriptor_sha256=DESCRIPTOR,
        inputs={str(p): pin(b) for p, b in inputs.items()},
        files={n: pin(b) for n, b in bodies.items()}, offsets=offsets,
        outer_wire_validated=True, nested_envelope_semantics_revalidated=False,
        nested_symbol_manifest_semantics_revalidated=False, compiler_executed=False,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False, load_authority=False, launch_authority=False)
    assert all(read(p) == b for p, b in inputs.items())
    out.mkdir(mode=0o700)
    for name, body in bodies.items():
        with (out / name).open('xb') as stream:
            stream.write(body)
        assert read(out / name) == body
    assert all(read(p) == b for p, b in inputs.items())
    result_body = (json.dumps(result, sort_keys=True, indent=2) + '\n').encode()
    with (out / 'complete.json').open('xb') as stream:
        stream.write(result_body)
    assert read(out / 'complete.json') == result_body
    print(json.dumps(dict(out=str(out), receipt=pin(result_body), files=result['files'], offsets=offsets)))


if __name__ == '__main__':
    main()
