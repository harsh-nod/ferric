"""Bounded CPU-only extension of the already-authenticated actual comparison."""
import hashlib
import json
import os
from pathlib import Path
import resource
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-silu-materialization-diagnostic-v1'
J_PATH = E / 'p228-layer0-current-comparison-v1/run.py'
J_SHA = '9c3db491d4f8c20890b045474bd1ab5bd8565d42916a8eb7772c0eb07c44717e'
COMPARISON_SHA = 'affe711d0dcac8e95609396c74e0f546ca09d9d6a4abf798eab5bfc72a7e7551'
LOWERING_SHA = '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e'
ORACLE_SHA = '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'
SOURCE_SHA = '2e0edf9efdc1caf19ea574ba9dfeffa375072584d27e21a31308539a6b8115d1'
LLVM_SHA = 'd6001c014f8160b1355008742f290fdd58f78c0f12a039aa6d577bbb82cad6fb'
IMAGE_SHA = '65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449'


def bootstrap():
    if sys.flags.optimize or os.environ.get('PYTHONOPTIMIZE'):
        raise RuntimeError('ordinary Python only')
    if J_PATH.resolve(strict=True) != J_PATH:
        raise ValueError('canonical pinned reader')
    with os.fdopen(os.open(J_PATH, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        raw = stream.read(65537)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    if not stat.S_ISREG(before.st_mode) or stamp(before) != stamp(after) or stamp(after) != stamp(J_PATH.lstat()) \
            or len(raw) != 23607 or hashlib.sha256(raw).hexdigest() != J_SHA:
        raise ValueError('exact retained Reader source')
    module = types.ModuleType('retained_current_comparison_reader')
    module.__file__ = str(J_PATH)
    exec(compile(raw, str(J_PATH), 'exec'), module.__dict__)
    return module


def main():
    J = bootstrap()
    J.require(len(sys.argv) == 4, 'usage: run.py COMPARISON_COMPLETE SHA256 OUTPUT_DIRECTORY')
    for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        J.require(os.environ.get(name) == '', 'CPU-only hidden GPU environment')
    for limit, requested in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                             (resource.RLIMIT_FSIZE, 16 << 20)):
        soft, hard = resource.getrlimit(limit)
        bound = min(x for x in (requested, soft, hard) if x != resource.RLIM_INFINITY)
        resource.setrlimit(limit, (bound, hard))
    receipt_pin, raw = J.actual(Path(sys.argv[1]), sys.argv[2])
    J.require(receipt_pin['sha256'] == COMPARISON_SHA, 'actual completed projection comparison')
    prior = J.parse(raw)
    J.require(prior['schema'] == 'ferric-p228-projection-residual-comparison-observation-v1'
        and prior['completed'] is True and prior['source_postchecks_passed'] is True
        and prior['native_structural_replayed'] is True
        and prior['candidate_owned_leaves_replayed'] == prior['baseline_owned_leaves_replayed'] == 7
        and prior['candidate_audits_replayed'] == prior['baseline_audits_replayed'] == 6
        and prior['framework_owned_leaf_results_rechecked'] == 21
        and prior['numerical_acceptance'] is False, 'existing actual ownership/reference checkpoint')
    mapping = {}
    for row in prior['consumed']:
        original, retained = J.filepin(row['original']), J.filepin(row['retained'])
        J.require((original['bytes'], original['sha256']) == (retained['bytes'], retained['sha256']), 'retained map identity')
        if original != retained:
            J.require(mapping.setdefault(original['path'], retained) == retained, 'unique transport identity')
    read = J.Reader(mapping)
    read(receipt_pin)
    read(J.actual(J_PATH, J_SHA)[0])
    manifest_pin, manifest_raw = J.actual(PACKAGE / 'manifest.json')
    manifest = J.parse(manifest_raw)
    J.require(manifest['schema'] == 'ferric-p228-silu-materialization-package-v1'
        and set(manifest['files']) == {'run.py', 'diagnostic.py', 'test_diagnostic.py', 'README.md'}, 'closed package')
    sources = {}
    for name, value in manifest['files'].items():
        sources[name] = dict(path=str(PACKAGE / name), bytes=value['bytes'], sha256=value['sha256'])
        read(sources[name])
    J.require(Path(__file__).resolve(strict=True) == PACKAGE / 'run.py', 'actual package controller')
    read(manifest_pin)
    diag = J.module(read, sources['diagnostic.py'], 'silu_materialization_diagnostic')
    old = prior['comparison_sources']
    def helper(suffix, name, aliases=None):
        pins = [pin for key, pin in old.items() if key == suffix]
        J.require(len(pins) == 1, 'qualified helper source')
        return J.module(read, pins[0], name, aliases)
    C = helper('p228-layer0-current-diagnostic-v1/compare.py', 'genuine_compare')
    K = helper('p228-layer0-current-diagnostic-v1/current.py', 'native_current', {'compare': C})
    D = helper('p228-layer0-current-diagnostic-v1/diagnostics.py', 'bf16_diagnostics')
    B = helper('p228-output-residual-boundary-v1/boundary.py', 'residual_boundary')
    A = helper('p228-projection-residual-comparison-v1/comparison.py', 'projection_capture',
               {'current': K, 'compare': C, 'boundary': B})
    oracle_pin = old['p228-independent-layer-reference-v1/helpers/residual_oracle.py']
    J.require(oracle_pin['sha256'] == ORACLE_SHA, 'unchanged BF16 RNE oracle')
    oracle = J.module(read, oracle_pin, 'integer_residual_oracle')
    inputs = prior['comparison']['inputs']
    framework_doc = read.doc(inputs['framework_capture'])
    framework = C.read_framework(framework_doc, read, D)
    for pin in framework_doc['retained_implementation_sources'].values():
        read(pin)
    summary = read.doc(inputs['candidate_capture'])
    A.observation(summary, A.OBSERVATION)
    bodies, native_pins = A.native_files(summary, read)
    J.require(native_pins == prior['comparison']['candidate_files'], 'actual qualified native file roster')
    page = A.bootstrap_page(summary, bodies)
    J.require(page == prior['comparison']['physical_pages']['candidate'], 'actual native physical page')
    parts = A.stage_bytes(summary, bodies['candidate-capture.bin'], page, D)
    row = E / 'row-down2-checked-probe-v228-v1'
    lowering_pin, _ = J.actual(row / 'complete.json', LOWERING_SHA)
    lowering = read.doc(lowering_pin)
    J.require(lowering['passed'] is True and lowering['postcheck_errors'] == []
        and lowering['fresh_checked_lowering'] is True and lowering['fresh_hsaco_emitted'] is True,
        'actual checked Down2 image generation')
    source_pin, _ = J.actual(row / 'fixture/src/mlp_numerics_v1.rs', SOURCE_SHA)
    read(source_pin)
    snapshot_pins = []
    for name, digest in (('before.json', 'ff7487ca2fc98c980add9357a6db45733330aa48d9b9d462db037887f648a948'),
                         ('after.json', '733dfe9d74c3de0b3f089602affa0f21c9b0f5505db42ecea55afc94194d6fde')):
        pin, _ = J.actual(row / name, digest)
        J.require(read.doc(pin)['fixture'][source_pin['path']]['pin'] == source_pin, 'unchanged compiled source snapshot')
        snapshot_pins.append(pin)
    llvm = lowering['artifacts']['extracted/module.ll']
    image = lowering['artifacts']['emitted/artifact.hsaco']
    J.require(llvm['sha256'] == LLVM_SHA and image['sha256'] == IMAGE_SHA
        and K.identity(summary['request']['layer']['mlp_tiles_image']) == (image['bytes'], image['sha256']),
        'captured selected Down2 image and checked LLVM')
    read(llvm)
    read(image)
    result = diag.compare({key: framework[key] for key in ('gate', 'silu-input', 'silu', 'up', 'product')},
        [{key: rank[key] for key in ('gate', 'up', 'activation')} for rank in parts], oracle.narrow_bf16_rne)
    output = Path(sys.argv[3])
    J.require(output.is_absolute() and output.parent == E and output.name.startswith('silu-materialization-diagnostic-v228-v')
        and not os.path.lexists(output) and output.parent.resolve(strict=True) == E, 'fresh CPU-only output')
    read.recheck()
    value = dict(schema='ferric-p228-silu-materialization-observation-v1', completed=True,
        controller=sources['run.py'], package_manifest=manifest_pin, sources=sources,
        comparison_receipt=receipt_pin, native_outer=prior['native_outer'], framework_outer=prior['framework_outer'],
        prior_ownership_replayed_in_this_run=False, trusted_prior_ownership_checkpoint=receipt_pin,
        inputs=inputs, compiled_lineage=dict(lowering=lowering_pin, source=source_pin,
            snapshots=snapshot_pins, llvm=llvm, image=image), diagnostic=result,
        consumed=list(read.consumed.values()), source_postchecks_passed=True,
        gpu_execution=False, numerical_acceptance=False, full_model_correctness=False,
        performance_claim=False, production_authority=False)
    output.mkdir(mode=0o700)
    with (output / 'complete.json').open('x', encoding='ascii') as stream:
        json.dump(value, stream, sort_keys=True, indent=2, allow_nan=False)
        stream.write('\n')
    print(json.dumps(dict(completed=True, output=str(output / 'complete.json'),
        framework_exact=result['framework_product_control']['exact_words'],
        same_gate_elements=[rank['same_gate_elements'] for rank in result['ranks']])))


if __name__ == '__main__':
    main()
