"""Publish compact actual results; do not rerun or claim to revalidate tensors."""
import hashlib
import json
from pathlib import Path
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PURE_SHA = 'd6739b8a8b3e3ea7e0e48c26f0fdf1ebdb9d1a258c7cbb2caf19f8aa87b9e4d6'
RESULT_SHA = 'c7b0406e5ba6ac4974b7769cd6916afba0b6b597986d070df1f6c3d731bd43ee'


def pin(path, original=None):
    assert path.resolve(strict=True) == path and path.is_file()
    raw = path.read_bytes()
    return dict(path=str(original or path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def write(path, raw):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as stream:
        stream.write(raw)


def main():
    assert not sys.flags.optimize and len(sys.argv) == 2
    retained = Path(sys.argv[1]).resolve(strict=True)
    out = Path(__file__).resolve().parent
    result_path = retained / 'silu-materialization-diagnostic-v228-v1/complete.json'
    pure_path = retained / 'silu-materialization-pure-v228-v1/complete.json'
    assert pin(result_path)['sha256'] == RESULT_SHA and pin(pure_path)['sha256'] == PURE_SHA
    actual = json.loads(result_path.read_bytes())
    pure = json.loads(pure_path.read_bytes())
    assert actual['completed'] and actual['source_postchecks_passed']
    assert pure['passed'] and pure['tests'] == 18 and pure['failures'] == pure['errors'] == pure['skipped'] == 0
    package = retained / 'proposals/p228-silu-materialization-diagnostic-v1'
    manifest = json.loads((package / 'manifest.json').read_bytes())
    assert pin(package / 'manifest.json', E / package.name / 'manifest.json') == actual['package_manifest']
    for name, record in manifest['files'].items():
        path = package / name
        assert {k: pin(path)[k] for k in ('bytes', 'sha256')} == record
        assert pin(path, E / package.name / name) == actual['sources'][name]
    pure_files = ['complete.json', 'sources-before.json', 'sources-after.json', 'tests.log']
    for key in ['sources_before', 'sources_after', 'transcript']:
        row = pure[key]
        assert pin(retained / Path(row['path']).relative_to(E), Path(row['path'])) == row
    assert json.loads((pure_path.parent / 'sources-before.json').read_bytes()) == json.loads((pure_path.parent / 'sources-after.json').read_bytes())
    wrapper = retained / 'run_silu_materialization_pure_p228_v1.py'
    assert pin(wrapper, E / wrapper.name) == pure['controller']
    diagnostic = actual['diagnostic']
    assert diagnostic['framework_product_control']['exact_words'] == 12288
    assert all(actual[k] is False for k in ['gpu_execution', 'numerical_acceptance', 'full_model_correctness', 'performance_claim', 'production_authority'])
    ranks = []
    for row in diagnostic['ranks']:
        assert sum(row['partition_counts'].values()) == row['elements'] == 6144
        assert row['native_equal_materialized'] + row['native_different_materialized'] == row['same_gate_elements']
        assert row['native_different_materialized'] == len(row['mismatch_indices'])
        ranks.append({k: v for k, v in row.items() if k not in ('partitions', 'prediction_indices', 'mismatch_indices', 'same_gate_same_up_native_different_framework_indices')})
        ranks[-1]['same_gate_same_up_native_different_framework_count'] = len(row['same_gate_same_up_native_different_framework_indices'])
    ledger = dict(schema='ferric-silu-materialization-publication-v1',
        actual_result=pin(result_path, E / result_path.parent.name / result_path.name),
        pure_result=pin(pure_path, E / pure_path.parent.name / pure_path.name),
        compiled_lineage=actual['compiled_lineage'], framework_control=diagnostic['framework_product_control'],
        ranks=ranks, original_consumed_files=len(actual['consumed']),
        comparison_receipt=actual['comparison_receipt'], sources=actual['sources'],
        numerical_acceptance=False, full_model_correctness=False, performance_claim=False,
        gpu_execution=False, materialization_only_cause_proven=False,
        publication_rechecks_tensor_bodies=False, publication_reruns_arithmetic=False,
        native_exp_error_measured=False, native_silu_intermediate_observed=False)
    for name in manifest['files']:
        write(out / 'source' / name, (package / name).read_bytes())
    write(out / 'source/manifest.json', (package / 'manifest.json').read_bytes())
    for name in pure_files:
        write(out / 'pure' / name, (pure_path.parent / name).read_bytes())
    write(out / 'pure/controller.py', wrapper.read_bytes())
    write(out / 'result.json', (json.dumps(ledger, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii'))
    print(json.dumps(dict(result=pin(out / 'result.json'), tests=18, framework_exact=12288)))


if __name__ == '__main__':
    main()
