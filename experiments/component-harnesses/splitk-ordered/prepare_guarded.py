"""Bounded CPU-only actual-input preparation under the existing G36 admission."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
INPUTS = D / 'inputs/component-ordered-bundle-a001'
QUALIFIER = D / 'inputs/component-ordered-cpu-a001/qualify_source.py'
QUALIFIER_SHA = 'b84d2f83eaffd7adf4f9aa760f88dd44d7fe4b2a58c23ea8a57d7beb3cfd0297'
PREPARER = INPUTS / 'prepare_ordered_bundle.py'
PREPARER_SHA = 'c9536460bea725338d6de17f55848d63df31e45349a8af4fb78b72fd93955ea9'
ARCHIVE_SHA = '75fafc7cafe346e16bc9a66a4ccc307edd0d5817dfa48ec95069f99d366561aa'
CONFIG_PINS = {
    'latency': '842e4b340b30e95b4696bd3a10d942993edd49d4469c196faa957856176743f6',
    'counters': '23d01566d180c88096af02627fc2904712e56816f14f4746f4ba3ac465679f72',
    'ticks': '641f55b0fa257efcd0e5c77a7eab686c1deac199b3c74723cc32b8e4e3cb18c0',
}
BASE_PLAN_SHA = 'f4f51dbdbdb36be3991e53e02422bde2cf05fd1fcec85314114c7422a22524e6'
ENVELOPE = 128 * 1024**2


def require(value, message):
    if not value:
        raise ValueError(message)


def file_hash(path, expected):
    require(path.is_absolute() and path.is_relative_to(D) and path.resolve(strict=True) == path,
            'canonical private-stage input')
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and before.st_uid == 1046 and before.st_nlink == 1
                and 0 <= before.st_size <= 64 * 1024**2, 'bounded owned single-link input')
        digest, count = hashlib.sha256(), 0
        while value := os.read(descriptor, 1024 * 1024):
            count += len(value)
            require(count <= 64 * 1024**2, 'input grew')
            digest.update(value)
        after, current = os.fstat(descriptor), path.lstat()
        fields = ('st_dev', 'st_ino', 'st_uid', 'st_mode', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
        require(count == before.st_size and all(getattr(before, name) == getattr(after, name)
                == getattr(current, name) for name in fields), 'input changed while read')
        require(digest.hexdigest() == expected, 'exact input digest')
        return {name: getattr(before, name) for name in fields} | {'sha256': expected}
    finally:
        os.close(descriptor)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=tuple(CONFIG_PINS))
    args = parser.parse_args()
    file_hash(QUALIFIER, QUALIFIER_SHA)
    spec = importlib.util.spec_from_file_location('ordered_charged_qualifier', QUALIFIER)
    q = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(q)
    q.ARCHIVE_SHA = ARCHIVE_SHA
    q.environment()
    files, _, _ = q.payload()
    source_before = q.check_sources(files)
    before_allocation = q.allocation(ENVELOPE)
    path = INPUTS / ('inputs-' + args.mode + '.json')
    raw, _ = q.read(path, CONFIG_PINS[args.mode])
    config = json.loads(raw)
    require(config['mode'] == args.mode
            and config['source_archive']['sha256'] == ARCHIVE_SHA,
            'exact mode and qualified source')
    base_path = Path(config['base_payload']) / 'plan.json'
    file_hash(base_path, BASE_PLAN_SHA)
    base = json.loads(base_path.read_bytes())
    inventory = {str(path): CONFIG_PINS[args.mode], str(QUALIFIER): QUALIFIER_SHA,
                 str(PREPARER): PREPARER_SHA, str(base_path): BASE_PLAN_SHA}

    def collect(value):
        if type(value) is dict and set(value) == {'path', 'sha256'}:
            require(value['path'] not in inventory or inventory[value['path']] == value['sha256'],
                    'one digest per input path')
            inventory[value['path']] = value['sha256']
        elif type(value) is dict:
            for item in value.values():
                collect(item)
        elif type(value) is list:
            for item in value:
                collect(item)

    collect(config)
    for name, item in base['files'].items():
        relative = Path(name)
        require(not relative.is_absolute() and '..' not in relative.parts
                and str(relative) == name, 'canonical base input name')
        selected = str(base_path.parent / relative)
        require(selected not in inventory or inventory[selected] == item['sha256'],
                'base input digest agrees')
        inventory[selected] = item['sha256']
    require(len(inventory) <= 512, 'bounded complete input roster')
    before = {name: file_hash(Path(name), digest) for name, digest in sorted(inventory.items())}
    output = D / ('component-ordered-' + args.mode + '-prepared-a001')
    require(not output.exists() and not output.is_symlink(), 'create-only actual preparation')
    argv = ['/usr/bin/python3', '-I', '-B', str(PREPARER), '--inputs', str(path),
            '--inputs-sha256', CONFIG_PINS[args.mode], '--output', str(output)]
    completed = None
    try:
        completed = subprocess.run(argv, check=False, timeout=1100)
    finally:
        after = {name: file_hash(Path(name), digest) for name, digest in sorted(inventory.items())}
        source_after = q.check_sources(files)
        allocation_after = q.allocation()
        q.environment()
        require(before == after and source_before == source_after, 'preparation changed retained inputs/source')
        require(allocation_after['stage_allocated_bytes'] - before_allocation['stage_allocated_bytes'] <= ENVELOPE,
                'observed preparation growth exceeded128MiB')
        receipt = {'schema': 'FerricOrderedComponentPreparationCustodyV1', 'mode': args.mode,
            'argv': argv, 'returncode': None if completed is None else completed.returncode,
            'source_before': source_before, 'source_after': source_after,
            'inputs_before': before, 'inputs_after': after,
            'allocation_before': before_allocation, 'allocation_after': allocation_after,
            'build_executed': False, 'tests_executed': False, 'native_executed': False}
        with (D / ('component-ordered-' + args.mode + '-preparation-custody-a001.json')).open('x') as stream:
            json.dump(receipt, stream, indent=2, sort_keys=True)
            stream.write('\n')
    return completed.returncode


if __name__ == '__main__':
    raise SystemExit(main())
