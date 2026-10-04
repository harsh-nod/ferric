"""Publish retained AR4 framework records; never import the tested GPU controller."""
import argparse
import hashlib
import io
import json
import math
from pathlib import Path
import struct
import tarfile
import types

HERE = Path(__file__).resolve().parent
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/projection-ar4-framework-v1'
EXPORT_SHA = '7a22191497eb0d302cb0e196fb3d3a19f0f05d199b2b29e10eb068dccb258c39'
LEAF_HELPER = F / 'qualification/layer0-framework-capture-v1/tools/publish.py'
LEAF_SHA = '7467252263c6d739f7851f711f3d61d2c40264afd5e84fcf9d7142eacbec161c'
COMMON = F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py'
POLICY = F / 'qualification/projection-ar4-supervisor-v1'
POLICY_SHA = '7de93a2d6b49da3dd8162a9efd3742404bba3fc723657678a6fa9cad9ec6b342'
NATIVE = F / 'qualification/projection-ar4-native-v1'
NATIVE_PUBLIC_SHA = '010c82b5b47023a2f0a75a01eb857f11d35962282a6410e2ca8b9e126eb0bdc8'


def load(path, digest, name):
    if path.resolve(strict=True) != path:
        raise ValueError('canonical publication data helper')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError('publication data-helper body')
    value = types.ModuleType(name)
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')


def verify_owner(X, D, P, outer, inputs, read, local, pin):
    doc = lambda path: X.parse(read(path))
    owner = local(D.OWNER)
    # Reuse only the published data-only leaf/audit functions, with retained-path I/O.
    P.body = lambda path: read(D.E / Path(path).relative_to(local(D.E)))
    P.pin = lambda path: pin(D.E / Path(path).relative_to(local(D.E)))
    def verify(record):
        read(Path(X.pin(record)['path']), record)
        return local(Path(record['path']))
    P.verify = verify
    P.document = lambda path, expected=None: X.parse(P.body(path))
    plan = doc(D.OWNER / 'capture-plan.json')
    reference = doc(Path(plan['reference_plan']['path']))
    supervisor = reference['supervisor_pid']
    X.require(type(supervisor) is int and supervisor > 1 and outer['plan'] == pin(D.OWNER / 'capture-plan.json')
              and plan == doc(D.REFERENCE / 'input-plan.json')
              and plan['output_root'] == str(D.REFERENCE)
              and plan['harness_sha256'] == D.SOURCE_SHAS['run.py'], 'actual framework plan/source')
    ready = doc(D.OWNER / 'ready.json')
    projection = doc(D.OWNER / 'capture-projection.json')
    X.require(ready['supervisor_pid'] == supervisor and ready['reference_plan'] == plan['reference_plan']
              and ready['capture_projection'] == pin(D.OWNER / 'capture-projection.json')
              and projection == {k: v for k, v in plan.items() if k != 'execution_review'}
              and ready['projection_sha256'] == X.extent(encoded(projection))['sha256'], 'fresh PID/plan projection')
    review = doc(Path(plan['execution_review']['path']))
    approval = doc(D.OWNER / 'approval.json')
    X.require(plan['execution_review'] == pin(Path(plan['execution_review']['path']))
              and review['schema'] == 'ferric-p228-projection-ar4-framework-execution-review-v1'
              and review['reviewed'] is True and review['gpu_execution_authorized'] is True
              and review['plan_projection_sha256'] == ready['projection_sha256']
              and approval['execution_review'] == plan['execution_review']
              and approval['projection_sha256'] == ready['projection_sha256']
              and all(review[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority')),
              'separately authored root execution approval')
    root_review = doc(Path(inputs['execution_review']['path']))
    X.require(root_review['inputs_projection_sha256'] == X.extent(encoded({k: v for k, v in inputs.items()
                  if k != 'execution_review'}))['sha256'] and root_review['reviewed'] is True
              and root_review['gpu_execution_authorized'] is True
              and root_review['resources'] == review['resources'] == reference['resources']
              and inputs['topology'] == outer['topology'], 'root launch review and topology')
    monitor = doc(Path(inputs['platform_monitor']['path']))
    X.require(monitor['read_only_attestation_reviewed'] is True and monitor['host'] == outer['topology']['host']
              and monitor['boot_id'] == outer['topology']['boot_id'], 'same admitted monitor boot/host')
    leaves = {name: P.leaf(owner, name, supervisor) for name in sorted(D.LEAVES)}
    X.require(len(leaves) == 25, 'four ordinary and twenty-one utility leaves')
    for side in ('before', 'after'):
        X.require(len(outer[side + '_audits']) == 3, 'three audit rounds per side')
        for index, value in enumerate(outer[side + '_audits']):
            P.audit(value, side + '-' + str(index), owner, leaves, monitor)
    P.audit(doc(D.OWNER / 'immediate-idle.json'), 'immediate', owner, leaves, monitor)
    for field, name in (('cpu_result', 'cpu-tests'), ('inspection_result', 'inspect'), ('execution_result', 'execute')):
        X.require(outer[field] == pin(D.OWNER / name / 'result.json'), 'actual selected leaf')
    names = P.test_names(local(D.PACKAGE / 'test_run.py'))
    X.require(len(names) == 20, 'executed framework test census')
    P.tests_log(read(D.OWNER / 'cpu-tests/stderr'), names)
    X.require(leaves['cpu-tests'][1]['argv'] == ['/usr/bin/python3', '-B', '-m', 'unittest', 'discover', '-v',
              '-s', str(D.PACKAGE), '-p', 'test_run.py'], 'actual in-owner policy command')
    for name, mode, seconds in (('inspect', '--inspect', 60), ('execute', '--run-reviewed-ar4-reference', 900)):
        command = leaves[name][1]
        X.require(command['argv'] == [reference['python_executable'], '-I', '-B', str(D.PACKAGE / 'run.py'),
                  mode, outer['plan']['path'], outer['plan']['sha256']] and command['seconds'] == seconds,
                  'exact framework inspect/execute command')
    inspected = doc(D.OWNER / 'inspect/stdout')
    X.require(inspected['schema'] == 'ferric-p228-projection-ar4-framework-inspection-v1'
              and inspected['gpu_opened'] is False and inspected['installed_callables_loaded'] is False
              and doc(D.OWNER / 'execute/stdout') == outer['reference'], 'inspection and child report join')
    return leaves


def tensors(X, D, report, native, read):
    X.require(report['genuine_framework_chain'] is True and report['repeat_passes_byte_equal'] is True
              and report['captured_tensors_per_forward'] == 38 and report['seed'] == 9112
              and report['gpu_execution'] is True and report['acceptance_threshold'] is None
              and all(report[k] is False for k in ('candidate_gpu_execution', 'candidate_intermediate_inputs',
                  'conditional_replay_performed', 'numerical_acceptance', 'full_model_correctness',
                  'production_authority', 'performance_measured', 'sustained_2048_256',
                  'external_native_lifecycle_replayed', 'native_source_image_bodies_rehashed')),
              'genuine reference with no acceptance or transitive native authority')
    X.require(report['native_checked'] == native['checked'] and report['native_complete']['sha256'] == D.NATIVE_SHA,
              'actual native structural record unchanged')
    checked = native['checked']
    aliases = {row['original']['path']: row['retained'] for row in report['native_consumed']}
    order = [f'layer{n}-hidden' for n in range(36)] + ['final-norm', 'logits']
    def split(raw):
        X.require(len(raw) == 606976, 'whole forward payload')
        return {name: raw[index * 8192:(index + 1) * 8192] if index < 37 else raw[37 * 8192:]
                for index, name in enumerate(order)}
    def argmax(raw):
        words = [word for (word,) in struct.iter_unpack('<H', raw)]
        X.require(all(word & 0x7f80 != 0x7f80 for word in words), 'all finite BF16 values')
        values = [struct.unpack('<f', struct.pack('<I', word << 16))[0] for word in words[-151936:]]
        return max(range(len(values)), key=values.__getitem__)
    native_rows, ledger, framework = [], [], []
    for position in range(4):
        record = native['retained_native'][f'observation-{position}.bin']
        raw = read(Path(aliases[record['path']]['path']), record)
        X.require(argmax(raw) == checked['output_tokens'][position], 'native actual lowest argmax')
        native_rows.append(split(raw))
    for ordinal, run in enumerate(report['genuine_passes'], 1):
        X.require(run['kind'] == 'genuine_ar' and run['fresh_cache'] is True, 'independent fresh-cache run')
        values, previous = [], 9112
        for position, case in enumerate(run['cases']):
            expected = dict(generation=position + 1, position=position, input_token=previous,
                            output_token=checked['output_tokens'][position])
            X.require(case['record'] == expected and previous == checked['input_tokens'][position],
                      'own-output recurrence and identical native/framework input history')
            raw = read(Path(case['payload']['path']), case['payload'])
            X.require(argmax(raw) == expected['output_token'], 'framework actual lowest argmax')
            rows = split(raw)
            X.require(set(case['tensors']) == set(order) and len(case['cache_sha256']) == 36,
                      '38 tensors and36 cache metadata rows')
            for index, name in enumerate(order):
                X.require(X.extent(rows[name]) == case['tensors'][name], 'recorded tensor slice body')
                ledger.append(dict(pass_ordinal=ordinal, position=position, tensor=name, dtype='bfloat16',
                    payload=case['payload'], offset=index * 8192, **X.extent(rows[name])))
            values.append(rows)
            previous = expected['output_token']
        framework.append(values)
    X.require(framework[0] == framework[1] and all(a['record'] == b['record']
              and a['cache_sha256'] == b['cache_sha256'] for a, b in zip(
                  report['genuine_passes'][0]['cases'], report['genuine_passes'][1]['cases'])),
              'all eight payloads repeat exactly with matching cache metadata')
    X.require(set(report['comparisons']) == {'genuine_ar'}, 'no unexecuted conditional comparison')
    comparison = report['comparisons']['genuine_ar']
    X.require(comparison['conditional'] is False and comparison['numerical_acceptance'] is False
              and comparison['acceptance_threshold'] is None and comparison['tensor_rows'] == 152
              and len(comparison['comparisons']) == 4, 'actual four comparable forwards')
    exact_slices = 0
    for position, row in enumerate(comparison['comparisons']):
        X.require(row['position'] == position and row['comparable'] is True and row['output_token_equal'] is True
                  and row['candidate_input_token'] == row['reference_input_token'] == checked['input_tokens'][position]
                  and row['candidate_output_token'] == row['reference_output_token'] == checked['output_tokens'][position]
                  and set(row['tensors']) == set(order), 'recorded comparison trajectories')
        for name in order:
            a, b, stats = framework[0][position][name], native_rows[position][name], row['tensors'][name]
            equal = sum(a[i:i + 2] == b[i:i + 2] for i in range(0, len(a), 2))
            X.require(stats['elements'] == len(a) // 2 and stats['exact_words'] == equal
                      and all(type(stats[key]) in (int, float) and math.isfinite(stats[key]) and stats[key] >= 0
                              for key in ('relative_l2', 'rmse', 'max_abs_error', 'max_bf16_steps')),
                      'exact word counts and finite recorded diagnostics')
            exact_slices += int(a == b)
    return ledger, comparison, exact_slices


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path)
    parser.add_argument('archive', type=Path)
    parser.add_argument('archive_sha256')
    args = parser.parse_args()
    D = load(HERE / 'export.py', EXPORT_SHA, 'ar4_framework_export_data')
    X = D.common(COMMON)
    P = load(LEAF_HELPER, LEAF_SHA, 'prior_framework_publication_data')
    root = args.retained_root.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical explicit courier root')
    seen = {}
    local = lambda path: root / Path(path).relative_to(D.E)
    def read(path, expected=None):
        path = Path(path)
        raw = X.read(local(path), expected)
        record = dict(path=str(path), **X.extent(raw))
        X.require(str(path) not in seen or seen[str(path)] == record, 'stable original-path input')
        seen[str(path)] = record
        return raw
    def pin(path):
        return dict(path=str(path), **X.extent(read(path)))
    outer, report, inputs, records = D.roster(X, read)
    manifest_raw = X.read(root / 'export-manifest.json')
    manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-projection-ar4-framework-export-v1'
              and manifest['files'] == records and manifest['original_root'] == str(D.E)
              and manifest['owner_sha256'] == D.OWNER_SHA and manifest['reference_sha256'] == D.REFERENCE_SHA
              and manifest['exporter']['sha256'] == EXPORT_SHA and manifest['data_helper']['sha256'] == D.COMMON_SHA
              and manifest['private_cache_exported'] is False and manifest['model_or_executable_bodies_exported'] is False,
              'exact courier manifest closure')
    archive_raw = X.read(args.archive.absolute())
    X.require(X.extent(archive_raw)['sha256'] == X.digest(args.archive_sha256), 'actual retained archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
        members = archive.getmembers()
        X.require(len(members) == len(records) + 1 and {m.name for m in members} == set(records) | {'export-manifest.json'}
                  and all(m.isfile() and str(X.relative(m.name)) == m.name for m in members), 'closed regular archive')
        for member in members:
            expected = X.extent(manifest_raw) if member.name == 'export-manifest.json' else records[member.name]
            X.require(member.size == expected['bytes'] and X.extent(archive.extractfile(member).read())
                      == {k: expected[k] for k in ('bytes', 'sha256')}, 'archive/extracted byte equality')
    policy_raw, native_public_raw = X.read(POLICY / 'result.json'), X.read(NATIVE / 'result.json')
    X.require(X.extent(policy_raw)['sha256'] == POLICY_SHA
              and X.extent(native_public_raw)['sha256'] == NATIVE_PUBLIC_SHA, 'immutable published prerequisites')
    policy, native_public = X.parse(policy_raw), X.parse(native_public_raw)
    X.require(policy['suites']['independent_framework_policy']['passed'] == 38
              and policy['suites']['independent_framework_policy']['remote_supervisor_receipt'] is False
              and native_public['gpu_observation'] == report['native_complete'], 'actual policy/native lineage')
    for name in D.SOURCE_SHAS:
        record = policy['files']['framework/' + name]
        X.require(record['original'] == pin(D.PACKAGE / name), 'published tested source equals executed source')
        X.read(POLICY / 'framework' / name, record)
    leaves = verify_owner(X, D, P, outer, inputs, read, local, pin)
    for key, record in report['retained_implementation_sources'].items():
        X.require(all(record[k] == inputs['implementation_sources'][key][k] for k in ('bytes', 'sha256'))
                  and report['runtime']['implementation_sources'][key] == inputs['implementation_sources'][key],
                  'installed/captured implementation identity, body excluded from Git')
    aliases = inputs['native']['transport']
    native_pin = aliases[report['native_complete']['path']]
    native = X.parse(read(Path(native_pin['path']), native_pin))
    X.require(native['passed'] is True and native['failures'] == []
              and native_public['structural'] == native['checked'], 'published native structure identity')
    ledger, comparison, exact_slices = tensors(X, D, report, native, read)
    copies = {}
    def copy(name, raw, original):
        X.relative(name)
        X.require(name not in copies and not raw.startswith(bytes([127, 69, 76, 70])), 'unique non-ELF publication')
        raw.decode('utf-8')
        copies[name] = (raw, original)
    for name in sorted(D.OWNER_FILES):
        path = D.OWNER / name
        copy('owner/' + name, read(path), pin(path))
    for name in ('reference.json', 'input-plan.json'):
        path = D.REFERENCE / name
        copy('reference/' + name, read(path), pin(path))
    for name in ('inputs.json', 'monitor.json', 'root-review.json'):
        path = D.INPUTS / name
        copy('inputs/' + name, read(path), pin(path))
    for name in D.SOURCE_SHAS:
        copy('source/' + name, read(D.PACKAGE / name), pin(D.PACKAGE / name))
    for name in ('export.py', 'publish.py'):
        raw = X.read(HERE / name)
        copy('publication/' + name, raw, dict(path=str(HERE / name), **X.extent(raw)))
    copy('comparison.json', encoded(comparison), outer['reference'])
    copy('tensor-ledger.json', encoded(dict(schema='ferric-p228-ar4-framework-tensor-ledger-v1',
        tensors=ledger, raw_tensor_bodies_published=False)), outer['reference'])
    summary = dict(schema='ferric-p228-projection-ar4-framework-publication-v1', status='PASS',
        owner_complete=pin(D.OWNER / 'complete.json'), reference=outer['reference'],
        native_complete=report['native_complete'], native_publication=dict(path=str(NATIVE / 'result.json'),
            **X.extent(native_public_raw)), policy_publication=dict(path=str(POLICY / 'result.json'), **X.extent(policy_raw)),
        policy_tests_previously_observed=38, in_owner_policy_tests=20, owned_leaves=25,
        pre_audits=3, immediate_audits=1, post_audits=3, native_attempts=1, retries=0,
        framework_forward_count=8, fresh_cache_passes=2, repeated_framework_payloads_byte_equal=True,
        input_tokens=report['native_checked']['input_tokens'], output_tokens=report['native_checked']['output_tokens'],
        all_four_output_tokens_equal=True, same_input_histories=True, captured_tensor_rows=152,
        whole_slices_bitwise_equal=exact_slices,
        logit_relative_l2=[row['tensors']['logits']['relative_l2'] for row in comparison['comparisons']],
        framework_typed_slice_hashes_replayed=True, exact_word_counts_recomputed=True,
        floating_error_metrics_recomputed=False, floating_error_metrics_source=outer['reference'],
        conditional_replay_performed=False, comparison='comparison.json', tensor_ledger='tensor-ledger.json',
        elapsed_seconds_includes_setup=leaves['execute'][0]['elapsed_seconds'],
        peak_owned_child_rss_bytes=leaves['execute'][0]['peak_group_rss_bytes'],
        host_elapsed_is_performance=False, natural_owned_child_and_utility_exits_verified=True,
        top_level_ssh_reaping_independently_verified=False, model=report['model'], revision=report['revision'],
        model_id=report['model_id'], bundle_id=report['bundle_id'], runtime=report['runtime'], topology=outer['topology'],
        archive=dict(path=str(args.archive.absolute()), **X.extent(archive_raw)), retained_files=records,
        data_helpers=[dict(path=str(path), **X.extent(X.read(path))) for path in (COMMON, LEAF_HELPER)],
        genuine_framework_gpu_execution_recorded=True, model_or_gpu_rerun=False, tested_controller_imported=False,
        private_cache_published=False, raw_tensor_bodies_published=False, third_party_implementation_bodies_published=False,
        all_transitive_inputs_rehashed=False, model_or_library_bodies_rehashed=False, native_lifecycle_replayed=False,
        numerical_acceptance=False, acceptance_threshold=None, full_model_correctness=False,
        production_authority=False, performance_claim=False, sustained_2048_256=False,
        limitations=['The genuine framework report is authenticated; this publisher never runs a model or imports its controller.',
            'Raw payload hashes, tensor slices, recurrence, lowest argmax and exact-word counts are independently checked.',
            'Relative L2/RMSE and maximum-error values are retained recorded diagnostics, not recomputed here or acceptance gates.',
            'The eight forwards use two fresh KV caches and one seed; this is not the 2048-prompt/256-decode workload.',
            'Native ownership is linked to the existing published checkpoint, not rerun in this framework-only replay.',
            'The inner pre-exit report flag is preserved; natural child exit is established by the outer owned result.'])
    for record in records.values():
        read(Path(record['path']), record)
    X.require(X.read(POLICY / 'result.json') == policy_raw and X.read(NATIVE / 'result.json') == native_public_raw
              and X.read(args.archive.absolute()) == archive_raw, 'immutable prerequisite/archive postchecks')
    for record in summary['data_helpers']:
        X.read(Path(record['path']), record)
    for name, (raw, original) in copies.items():
        if name.startswith('publication/'):
            X.require(X.read(Path(original['path'])) == raw, 'publication source postcheck')
    if Q.exists():
        X.require(Q.resolve(strict=True) == Q and Q.is_dir() and all(p.name == 'README.md'
                  and p.is_file() and not p.is_symlink() for p in Q.iterdir()), 'only root README may preexist')
    else:
        X.require(not Q.is_symlink(), 'no dangling destination alias')
        Q.mkdir()
    summary['files'] = []
    for name, (raw, original) in sorted(copies.items()):
        path = Q / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
        X.require(X.read(path) == raw, 'exact published bytes')
        summary['files'].append(dict(path=name, original=original, **X.extent(raw)))
    with (Q / 'result.json').open('xb') as stream:
        stream.write(encoded(summary))
    print(json.dumps(dict(output=str(Q), files=len(copies), result=X.extent(X.read(Q / 'result.json')))))


if __name__ == '__main__':
    main()
