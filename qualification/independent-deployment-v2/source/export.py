"""Export exact retained data to a fresh directory; never build or run a binary."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import tarfile
import types

import portable as P

DRIVER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'


def driver(path):
    P.require(path.resolve(strict=True) == path and path.is_file(), 'canonical retained driver')
    raw = path.read_bytes()
    P.require(len(raw) <= 1 << 20 and hashlib.sha256(raw).hexdigest() == DRIVER_SHA,
              'exact retained safe FilePin helper')
    value = types.ModuleType('independent_deployment_pins')
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def copy_file(source, destination, expected):
    digest, size = hashlib.sha256(), 0
    with source.open('rb') as src, destination.open('xb') as dst:
        while block := src.read(1 << 20):
            size += len(block)
            P.require(size <= expected['bytes'], 'copy exceeds qualified extent')
            digest.update(block)
            dst.write(block)
    P.require(dict(bytes=size, sha256=digest.hexdigest()) == P.body_pin(expected), 'exact copied object')


def copy_member(archive, original, destination):
    archive.check(original)
    name = str(Path(original['path']).relative_to(P.E))
    found = False
    with tarfile.open(archive.record['path'], mode='r|gz') as packed:
        for member in packed:
            if member.name != name:
                continue
            P.require(member.isfile() and not member.linkname and member.size == original['bytes'],
                      'exact selected archived object')
            digest, size = hashlib.sha256(), 0
            with packed.extractfile(member) as src, destination.open('xb') as dst:
                while block := src.read(1 << 20):
                    size += len(block)
                    P.require(size <= original['bytes'], 'bounded selected object copy')
                    digest.update(block)
                    dst.write(block)
            P.require(dict(bytes=size, sha256=digest.hexdigest()) == P.body_pin(original), 'selected object identity')
            found = True
            break
    P.require(found, 'selected object retained in archive')


def export(D, evidence_root, qualification_root, output, directory):
    P.require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'unoptimized exporter required')
    for root in (evidence_root, qualification_root):
        P.require(root.is_absolute() and root.resolve(strict=True) == root and root.is_dir(), 'canonical input root')
    P.directory_name(directory)
    P.require(output.is_absolute() and output.name == directory.name
        and output.parent.resolve(strict=True) == output.parent and not os.path.lexists(output), 'fresh export output')
    pins, archives, summaries, local = D.Pins(), {}, {}, {}
    for role, count in (('compiler', 92), ('native', 5856)):
        original = P.ARCHIVES[role]
        path = evidence_root / Path(original['path']).name
        record = P.fp(path, original['bytes'], original['sha256'])
        archives[role] = P.Archive(D, pins, record, count)
        local[role + '_archive'] = record
        original = P.QUALIFICATIONS[role]
        path = qualification_root / Path(original['path']).relative_to(P.PUBLIC)
        record, raw = pins.read(path, original['sha256'], True, 1 << 20)
        P.require(P.body_pin(record) == P.body_pin(original), 'exact root-verified qualification')
        local[role + '_qualification'] = record
        summaries[role] = D.parse(raw)
    P.evidence(D, archives['compiler'], archives['native'], summaries['compiler'], summaries['native'])
    os.umask(0o077)
    output.mkdir(mode=0o700)
    for name in ('evidence', 'qualification', 'image'):
        (output / name).mkdir(mode=0o700)
    value = dict(schema=P.SCHEMA, directory=str(directory), archives={}, qualifications={},
        authority='none', **{name: False for name in P.NONCLAIMS})
    for role in ('compiler', 'native'):
        for group, suffix, originals in (('archives', 'archive', P.ARCHIVES),
                                          ('qualifications', 'qualification', P.QUALIFICATIONS)):
            filename = P.FILENAMES[role + '_' + suffix]
            copy_file(Path(local[role + '_' + suffix]['path']), output / filename, originals[role])
            value[group][role] = dict(original=originals[role], transported=P.fp(directory / filename,
                originals[role]['bytes'], originals[role]['sha256']))
    for name, original, role in (('image', P.IMAGE, 'compiler'), ('binary', P.BINARY, 'native')):
        filename = P.FILENAMES[name]
        copy_member(archives[role], original, output / filename)
        if name == 'binary':
            (output / filename).chmod(0o700)
        value[name] = dict(original=original, transported=P.fp(directory / filename,
            original['bytes'], original['sha256']))
    for name, record in local.items():
        P.require(P.body_pin(pins.pin(output / P.FILENAMES[name], record['sha256'])) == P.body_pin(record),
                  'copied provenance rehash')
    for name, original in (('image', P.IMAGE), ('binary', P.BINARY)):
        P.require(P.body_pin(pins.pin(output / P.FILENAMES[name], original['sha256'])) == P.body_pin(original),
                  'copied runnable object rehash')
    pins.recheck()
    with (output / 'deployment.json').open('xb') as stream:
        stream.write((json.dumps(value, indent=2, sort_keys=True) + '\n').encode('ascii'))
    return value, pins.pin(output / 'deployment.json')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--driver', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--qualification-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--directory', type=Path, required=True,
                        help='Final absolute deployment directory on MI350, not a build-host path.')
    args = parser.parse_args()
    P.require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'unoptimized exporter required')
    value, record = export(driver(args.driver), args.evidence, args.qualification_root, args.output, args.directory)
    print(json.dumps(dict(manifest=record, directory=value['directory'], authority='none',
        **{name: False for name in P.NONCLAIMS}), sort_keys=True))


if __name__ == '__main__':
    main()
