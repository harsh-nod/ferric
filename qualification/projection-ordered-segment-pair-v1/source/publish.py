"""Copy authenticated ordered-pair evidence and reports; never execute tested code."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
OUT = F / 'qualification/projection-ordered-segment-pair-v1'
PACKAGE_SHA = '054b11e3eae9389dad6dc42f216b2ccfc81115436340cab572fda7f93d8d4d69'
CPU_SHA = '6ac67053d9b7d5d15772b5e4071a09933f013b6efa27394c31eb2b8279af884f'
SOURCE_HASHES = {
    'analyze.py': 'e72d058646ed8eb34d43e3ffa310451629d9051ef3adbdfb60c2dad31e2a9034',
    'plot.py': '46d7469462ebdb8f423cde090fac41e5d3749c79a5c1faa164f9ee9531dea838',
    'export.py': 'bdff4d14a7f0b5441c28979120e06e348cd7bee4137e4248cc47ce85af01a8bd',
    'common.py': '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720',
}
CHECKED, COPIES, LEDGER = {}, {}, {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(path, raw):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def doc(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key'); value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=lambda _: require(False, 'nonfinite JSON'))


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
    require(len(sys.argv) == 11, 'EXPORT_DIR EXPORT_SHA PAIR_SHA ANALYSIS_DIR ANALYSIS_SHA CSV_SHA REPORT_SHA SVG_SHA PRIMARY_PATH PRIMARY_SHA')
    export, analysis_dir, primary = Path(sys.argv[1]), Path(sys.argv[4]), Path(sys.argv[9])
    digests = [sys.argv[i] for i in (2, 3, 5, 6, 7, 8, 10)]
    require(all(re.fullmatch('[0-9a-f]{64}', value) for value in digests), 'caller supplies actual completed hashes')
    require(export.is_relative_to(W) and analysis_dir.is_relative_to(W)
        and export.resolve(strict=True) == export and analysis_dir.resolve(strict=True) == analysis_dir,
        'retained local export and analysis directories')
    manifest = doc(copy(export / 'export-manifest.json', 'export-manifest.json', sha=sys.argv[2]))
    require(manifest['schema'] == 'ferric-p228-projection-ordered-segment-pair-export-v1'
        and manifest['recorded_pair_passed'] is True and manifest['original_root'] == str(E)
        and len(manifest['files']) == 128 and manifest['terminal']['sha256'] == sys.argv[3],
        'actual successful terminal export only; failed exports stay outside this success publication')
    require({str(p.relative_to(export)) for p in export.rglob('*') if p.is_file()}
        == set(manifest['files']) | {'export-manifest.json'}, 'closed 129-file export')
    terminal_name = str(Path(manifest['terminal']['path']).relative_to(E))
    require(Path(terminal_name).name == 'complete.json', 'actual successful pair filename')
    pair = doc(read(export / terminal_name, manifest['terminal']))
    require(pair['schema'] == 'ferric-p228-projection-ordered-segment-pair-v1'
        and pair['passed'] is True and pair['failures'] == [] and pair['fixed_order'] == ['shared', 'ordered']
        and pair['retries'] == 0 and pair['each_arm_max_attempts'] == 1
        and pair['paired_comparison_performed'] is True, 'successful one-attempt matched pair')
    plan_name = str(Path(pair['plan']['path']).relative_to(E))
    plan = doc(read(export / plan_name, pair['plan']))
    require(set(plan) == {'schema', 'output_label', 'shared', 'ordered'}
        and plan['schema'] == 'ferric-p228-projection-ordered-segment-pair-inputs-v1'
        and plan['output_label'] == Path(manifest['terminal']['path']).parent.name, 'closed pair/plan join')
    arm_roots = {role: Path(pair['arms'][role]['path']).parent.name for role in ('shared', 'ordered')}
    input_roots = {role: Path(plan[role]['path']).parent.name for role in ('shared', 'ordered')}
    for role in arm_roots:
        require(re.fullmatch('prefix-projection-ordered-segment-' + role + r'-gpu-v228-v[1-9][0-9]{0,8}', arm_roots[role])
            and re.fullmatch('prefix-projection-ordered-segment-' + role + r'-inputs-v228-v[1-9][0-9]{0,8}', input_roots[role]),
            'closed arm/input namespaces')
    binary, counts, input_counts = {}, dict(shared=0, ordered=0), dict(shared=0, ordered=0)
    for name, record in sorted(manifest['files'].items()):
        relative = Path(name)
        require(not relative.is_absolute() and '..' not in relative.parts and record['path'] == str(E / relative),
            'original E identity')
        local = export / relative; raw = read(local, record); destination = None
        for role in counts:
            if relative.parts[0] == arm_roots[role]:
                if relative.suffix == '.bin':
                    binary[name] = dict(original=record, retained=pin(local, raw))
                else:
                    destination = 'capture/' + role + '/' + str(Path(*relative.parts[1:])); counts[role] += 1
            elif relative.parts[0] == input_roots[role]:
                require(len(relative.parts) == 2 and relative.name in {'plan.json', 'request.json', 'decode-review.json', 'assembly.json'},
                    'four actual assembled inputs per route')
                destination = 'inputs/' + role + '/' + relative.name; input_counts[role] += 1
        if name == terminal_name:
            destination = 'pair-complete.json'
        elif name == plan_name:
            destination = 'pair-plan.json'
        require(destination is not None or name in binary, 'closed publication roster')
        if destination is not None:
            copy(local, destination, record, record)
    require(counts == {'shared': 50, 'ordered': 50} and input_counts == {'shared': 4, 'ordered': 4}
        and len(binary) == 18, '100 native text records,8 input records,18 binary bodies omitted')
    arms = {}
    for role in counts:
        arm = doc(COPIES[f'capture/{role}/complete.json']); arms[role] = arm
        require(LEDGER[f'capture/{role}/complete.json']['original'] == pair['arms'][role] == manifest['arms'][role]['terminal']
            and arm['passed'] is True and arm['failures'] == [] and arm['route'] == role and arm['plan'] == plan[role]
            and arm['supervisor_manifest']['sha256'] == PACKAGE_SHA
            and arm['parent_cpu_complete'] == arm['worker_cpu_complete']
            and arm['parent_cpu_complete']['sha256'] == CPU_SHA, 'actual current-generation arm terminals')
        require(LEDGER[f'inputs/{role}/plan.json']['original'] == plan[role], 'published arm plan identity')
    for key in ('parent_cpu_complete', 'worker_cpu_complete', 'worker', 'prefix_image', 'mlp_image', 'projection_image'):
        require(arms['shared'][key] == arms['ordered'][key], 'same qualified generation and images')
    for name, digest in zip(('analysis.json', 'intervals.csv', 'report.md', 'host-comparison.svg'), sys.argv[5:9]):
        copy(analysis_dir / name, 'analysis/' + name, sha=digest)
    analysis = doc(COPIES['analysis/analysis.json'])
    require(analysis['schema'] == 'ferric-p228-projection-ordered-segment-pair-analysis-v1'
        and analysis['pair'] == pin(export / terminal_name, COPIES['pair-complete.json'])
        and analysis['single_pair'] is True and analysis['fixed_order'] == ['shared', 'ordered']
        and analysis['warmed_cache_order_confound'] is True
        and analysis['payloads_byte_equal'] == pair['comparison']['payloads_byte_equal']
        and analysis['histories'] == pair['comparison']['actual_input_histories']
        and analysis['ordered_wait_spans_overlap'] is True
        and analysis['first_ordered_segment_includes_arena_initialization'] is True
        and all(analysis[k] is False for k in ('performance_claim', 'gpu_time', 'independent_numerical_acceptance',
            'production_authority', 'per_kernel_publish_wait_poll_comparable', 'separate_residual_mlp_timings_available')),
        'actual matched analysis and honest timing scope')
    for record in analysis['consumed']:
        read(Path(record['path']), record)
    source_root = Path(__file__).resolve().parent.parent / 'p228-projection-ordered-segment-pair-publication-v2'
    sources = {
        'analyze.py': (source_root / 'analyze.py', analysis['controller']),
        'plot.py': (source_root / 'plot.py', None),
        'export.py': (source_root.parent / 'p228-projection-ordered-segment-pair-publication-v1' / 'export.py', manifest['exporter']),
        'common.py': (F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py', manifest['data_helper'])}
    for name, (local, original) in sources.items():
        copy(local, 'source/' + name, original, original, SOURCE_HASHES[name])
    copy(primary, 'run-observation.json', sha=sys.argv[10])
    copy(source_root / 'test_trajectory.py', 'source/test_trajectory.py',
        sha='d818ef2e05ab27d8009cddfec4f94c6e4ed0849763adb6f50d6c0914482ff790')
    copy(Path(__file__).resolve(), 'source/publish.py')
    require(len(COPIES) == 122, 'exact copied census')
    result = dict(schema='ferric-p228-projection-ordered-segment-pair-publication-v1', pair=manifest['terminal'],
        arms=pair['arms'], analysis=LEDGER['analysis/analysis.json']['original'], copied=LEDGER, copied_count=len(COPIES),
        verified_export_body_count=128, omitted_binary_bodies=binary,
        observed_pair_passed=True, observed_payload_repeatability=analysis['payloads_byte_equal'],
        recorded_matched_input_histories=analysis['histories'], single_fixed_order_pair=True,
        warm_cache_order_confound_unresolved=True, first_ordered_segment_includes_arena_initialization=True,
        per_kernel_publish_wait_poll_comparable=False, ordered_wait_spans_overlap=True,
        separate_residual_mlp_timings_available=False, timing_categories_are_not_additive=True,
        evidence_or_analysis_executed=False, validators_replayed=False, tensor_math_replayed=False,
        primary_observation=LEDGER['run-observation.json']['original'], primary_observation_is_remote_owner_receipt=False,
        independent_numerical_acceptance=False, gpu_timing=False, performance_claim=False,
        sustained_700_tokens_per_second_claim=False, production_authority=False)
    for path, expected in list(CHECKED.items()):
        read(path, expected)
    require(not OUT.is_symlink() and (not OUT.exists() or {p.name for p in OUT.iterdir()} <= {'README.md'}),
        'fresh destination except root README')
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
