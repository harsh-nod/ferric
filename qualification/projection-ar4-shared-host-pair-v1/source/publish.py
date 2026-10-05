"""Publish the actual retained shared-host pair and analysis, without executing evidence."""
import hashlib
import json
import os
from pathlib import Path
import stat

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
EXPORT, ANALYSIS = W / 'shared-host-pair-export-v228-v1', W / 'shared-host-pair-analysis-v228-v1'
OUT = F / 'qualification/projection-ar4-shared-host-pair-v1'
PAIR_SHA = '095070cb312dd286ec76b0b72c11c82c96c073f2dc6fc84996efc30d8ba48994'
HASHES = dict(zip(('analysis.json', 'intervals.csv', 'report.md', 'host-comparison.svg'), (
    'f5a0b6fa5048b1802344610bd7951170956a455f3eab43c32600a08072777e63',
    '9bff155bd88dee6efe2df8906e4c39b7dbb718404a1426baab71cc5f04b4477b',
    '8be6479955c2aa5a464841ed3768ed10982251725e35d533f6103f4e91ce1913',
    'c9677a90380d90993eab43fa4b38bec144c2265c85663b2284bcd40efaa57ec3')))
CHECKED, COPIES, LEDGER = {}, {}, {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(path, raw):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path, expected=None):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local body')
    before = path.stat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 8 << 20, 'bounded ordinary body')
    raw = path.read_bytes(); after = path.stat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) and len(raw) == before.st_size, 'stable body')
    actual = pin(path, raw)
    require(expected is None or all(actual[k] == expected[k] for k in ('bytes', 'sha256')), 'actual FilePin')
    require(CHECKED.setdefault(path, actual) == actual, 'unchanged repeated read')
    return raw


def copy(path, destination, original=None, expected=None, sha=None):
    raw = read(path, expected)
    require(sha is None or hashlib.sha256(raw).hexdigest() == sha, 'actual source/result hash')
    require(destination not in COPIES, 'unique destination')
    COPIES[destination] = raw
    LEDGER[destination] = dict(original=original or pin(path, raw), retained=pin(path, raw), published=pin(OUT / destination, raw))
    return raw


def main():
    manifest = json.loads(copy(EXPORT / 'export-manifest.json', 'export-manifest.json',
        sha='f4fc8045e5f50023864cb0740a89a347dd29a3503432513d3b93794912a85c5d'))
    require(manifest['recorded_pair_passed'] is True and manifest['original_root'] == str(E)
        and len(manifest['files']) == 128 and manifest['terminal']['sha256'] == PAIR_SHA, 'actual successful export')
    require({str(p.relative_to(EXPORT)) for p in EXPORT.rglob('*') if p.is_file()} == set(manifest['files']) | {'export-manifest.json'}, 'closed 129-file export')
    binary, prior_inputs, counts = {}, {}, {'default': 0, 'shared': 0}
    for name, record in sorted(manifest['files'].items()):
        relative = Path(name)
        require(not relative.is_absolute() and '..' not in relative.parts and record['path'] == str(E / relative), 'original E identity')
        local = EXPORT / relative; raw = read(local, record); destination = None
        for role in counts:
            if relative.parts[0] == f'prefix-projection-ar4-shared-host-{role}-gpu-v228-v1':
                if relative.suffix == '.bin':
                    binary[name] = dict(original=record, retained=pin(local, raw))
                else:
                    destination = 'capture/' + role + '/' + str(Path(*relative.parts[1:])); counts[role] += 1
            elif relative.parts[0] == f'prefix-projection-ar4-shared-host-{role}-inputs-v228-v1':
                previous = F / 'qualification/projection-ar4-shared-host-runtime-v1/inputs' / role / relative.name
                require(read(previous, record) == raw, 'eight inputs already published verbatim')
                prior_inputs[name] = dict(original=record, published=pin(previous, raw))
        if record == manifest['terminal']:
            destination = 'pair-complete.json'
        elif relative.name == 'shared-host-pair-plan-v228-v1.json':
            destination = 'pair-plan.json'
        require(destination is not None or name in binary or name in prior_inputs, 'closed selected publication roster')
        if destination is not None:
            copy(local, destination, record, record)
    require(counts == {'default': 50, 'shared': 50} and len(binary) == 18 and len(prior_inputs) == 8, '100 text records;18 binary and8 prior inputs omitted')
    pair = json.loads(COPIES['pair-complete.json']); plan = json.loads(COPIES['pair-plan.json'])
    require(pin(E / Path(pair['plan']['path']).relative_to(E), COPIES['pair-plan.json']) == pair['plan']
        and pair['passed'] is True and pair['failures'] == [] and plan['output_label'] == Path(manifest['terminal']['path']).parent.name, 'pair and plan joins')
    for role in counts:
        arm = json.loads(COPIES[f'capture/{role}/complete.json'])
        require(LEDGER[f'capture/{role}/complete.json']['original'] == pair['arms'][role] == manifest['arms'][role]['terminal']
            and arm['passed'] is True and arm['failures'] == [] and arm['route'] == role and arm['plan'] == plan[role], 'actual two arm terminals')
    for name, digest in HASHES.items():
        copy(ANALYSIS / name, 'analysis/' + name, sha=digest)
    analysis = json.loads(COPIES['analysis/analysis.json'])
    require(analysis['pair'] == pin(EXPORT / Path(manifest['terminal']['path']).relative_to(E), COPIES['pair-complete.json'])
        and analysis['single_pair'] is True and analysis['fixed_order'] == ['default', 'shared']
        and analysis['warmed_cache_order_confound'] is True and analysis['payloads_byte_equal'] == pair['comparison']['payloads_byte_equal']
        and all(analysis[k] is False for k in ('performance_claim', 'gpu_time', 'independent_numerical_acceptance', 'production_authority')), 'actual bounded analysis scope')
    for record in analysis['consumed']:
        read(Path(record['path']), record)
    sources = {
        'analyze.py': (L / 'proposals/p228-projection-ar4-shared-host-pair-analysis-v1/analyze.py', analysis['controller'], '2e51255a44ef4198ea550f83dd1c68e1e3925a3ac17530e5733874a30c335b09'),
        'plot.py': (L / 'proposals/p228-projection-ar4-shared-host-pair-analysis-v1/plot.py', None, 'ed129add853d301b3fe428e48c5cdaff6fe8d75749fb367d90014f0941613114'),
        'export.py': (L / 'proposals/p228-projection-ar4-shared-host-pair-publication-v1/export.py', manifest['exporter'], 'ac98689d07f55fcba11b3e0c0adf3cb70a26106abc2d3345d087965f88e916c3'),
        'common.py': (F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py', manifest['data_helper'], '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720')}
    for name, (local, original, digest) in sources.items():
        copy(local, 'source/' + name, original, original, digest)
    copy(L / 'shared-host-pair-primary-v228-v1.json', 'run-observation.json',
        sha='2e8ba96b82a93c67098edf080b3571519ba4cd90274370b2b187e66e3503a041')
    copy(Path(__file__).resolve(), 'source/publish.py')
    require(len(COPIES) == 113, 'exact copied census')
    result = dict(schema='ferric-p228-projection-ar4-shared-host-pair-publication-v1', pair=manifest['terminal'],
        arms=pair['arms'], analysis=LEDGER['analysis/analysis.json']['original'], copied=LEDGER, copied_count=len(COPIES),
        verified_export_body_count=128, omitted_binary_bodies=binary, already_published_inputs=prior_inputs,
        observed_pair_passed=True, observed_payload_repeatability=analysis['payloads_byte_equal'], single_fixed_order_pair=True,
        warm_cache_order_confound_unresolved=True, evidence_or_analysis_executed=False, validators_replayed=False,
        primary_observation=LEDGER['run-observation.json']['original'], primary_observation_is_remote_owner_receipt=False,
        independent_numerical_acceptance=False, gpu_timing=False, performance_claim=False, production_authority=False)
    for path, expected in list(CHECKED.items()):
        read(path, expected)
    require(not OUT.is_symlink() and (not OUT.exists() or {p.name for p in OUT.iterdir()} <= {'README.md'}), 'fresh destination except root README')
    OUT.mkdir(parents=True, exist_ok=True)
    for name, raw in sorted(COPIES.items()):
        target = OUT / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(raw)
        os.chmod(target, 0o644); require(read(target) == raw, 'published byte identity')
    for path, expected in list(CHECKED.items()):
        read(path, expected)
    with (OUT / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True, allow_nan=False); stream.write('\n')
    print(json.dumps(dict(result=pin(OUT / 'result.json', read(OUT / 'result.json')), copied=len(COPIES)), sort_keys=True))


if __name__ == '__main__':
    main()
