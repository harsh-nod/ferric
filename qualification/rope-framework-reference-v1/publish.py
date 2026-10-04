"""Local retained-byte publication only; no Torch import or numeric/GPU run."""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import re
import stat

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = L / 'proposals/p228-rope-framework-reference-v1'
CASE = L / 'rope-framework-reference-v228-v1'
Q = Path('/home/harsh/ferric-p227-integration/qualification/rope-framework-reference-v1')
COMPLETE = (330074, 'f5c47aabf7a307a19f9fe54fa37187f8548ceab4ac92eeeac2638fc9cc4589b4')
SOURCE = {
    'run.py': (19008, 'b6bf8cfee5b336fd2f844dabf67ce2ad903aa3fdb592e84e77833c796fbeb092'),
    'test_run.py': (5275, '98ce1ff24abd4507289ef79933bc848394e7c5402f129ccace0e6710c276816d'),
    'README.md': (4623, '7bff13108db4309b71ddfc77efc933dcb22551d748ada9430b4c8a8237f8bf8f'),
}
POSITIONS = [0, 1, 2, 3, 2047, 2048, 2303]
CORPORA = ('genuine-pos0-conditional', 'synthetic-boundaries')
MODES = ('framework', 'independent-materialized', 'framework-table-fp32-products',
         'f64-model-fp32-products', 'f64-model-materialized')
CHECKED = {}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2) + '\n').encode()


def read(path, size=None, digest=None):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local path')
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and 0 < info.st_size <= 4 << 20, 'small regular file')
    body = path.read_bytes()
    actual = {'path': str(path), 'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
    require(len(body) == info.st_size and (size is None or len(body) == size), 'extent: ' + str(path))
    require(digest is None or digest == actual['sha256'], 'digest: ' + str(path))
    CHECKED[str(path)] = actual
    return body


def checked_pin(pin, path):
    require(set(pin) == {'path', 'bytes', 'sha256'}, 'FilePin fields')
    require(type(pin['bytes']) is int and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'FilePin values')
    return read(path, pin['bytes'], pin['sha256'])


def local_input(pin):
    path = Path(pin['path'])
    if path.is_relative_to(E):
        relative = path.relative_to(E)
        candidate = L / relative
        if candidate.exists() or candidate.is_symlink():
            return candidate
        candidate = L / 'proposals' / relative
        if candidate.exists() or candidate.is_symlink():
            return candidate
    # Explicit retained source alias, not a claim that site-packages is local.
    if str(path).endswith('/transformers/models/qwen3/modeling_qwen3.py'):
        return L / 'layer0-installed-sources-v228-v1/modeling_qwen3.py'
    return None


def main(args):
    complete_body = read(CASE / 'complete.json', *COMPLETE)
    data = json.loads(complete_body)
    require(data['schema'] == 'ferric-p228-rope-framework-reference-v1' and
            data['completed'] is True and data['source_unchanged'] is True, 'actual complete')
    require(data['positions'] == POSITIONS and (data['head_dim'], data['theta']) == (128, 1000000), 'geometry')
    for key in ('gpu_execution', 'numerical_acceptance', 'full_model_correctness', 'performance_claim',
                'production_authority', 'model_loaded', 'cpu_trig_is_gpu_trig_evidence',
                'actual_rust_tables_replayed', 'new_genuine_later_position_capture',
                'transitive_framework_libraries_rehashed', 'historical_owner_audits_replayed'):
        require(data[key] is False, 'explicit limitation: ' + key)
    require(data['conditional_on_position_zero_qk'] is True and
            data['f64_table_is_python_libm_model'] is True, 'conditional arithmetic only')
    source_bodies = {name: read(P / name, *pin) for name, pin in SOURCE.items()}
    output_bodies, output_ledger = {}, []
    require(len(data['outputs']) == 145, 'actual buffer census')
    for pin in data['outputs']:
        original = Path(pin['path'])
        require(original.parent == E / CASE.name and original.name not in output_bodies, 'closed output path')
        output_bodies[original.name] = checked_pin(pin, CASE / original.name)
        output_ledger.append({'original': pin, 'retained': CHECKED[str(CASE / original.name)]})
    require({x.name for x in CASE.iterdir()} >= set(output_bodies) | {'complete.json'}, 'retained output presence')
    used = {'inv-frequency.f32le'} | {name + '-' + kind + '.bf16le' for name in CORPORA for kind in ('q', 'k')}
    rows = []
    expected_order = [(name, pos, kind) for name in CORPORA for pos in POSITIONS for kind in ('q', 'k')]
    require(len(data['rows']) == 28, 'all comparison rows')
    for row, identity in zip(data['rows'], expected_order):
        require((row['corpus'], row['position'], row['kind']) == identity and set(row['outputs']) == set(MODES), 'row identity')
        name, position, kind = identity
        count = (4096 if kind == 'q' else 1024) if name == CORPORA[0] else (256 if kind == 'q' else 128)
        require(row['output_shape'] == [1, count // 128, 1, 128], 'actual output shape')
        require(set(row['vs_framework']) == set(MODES) - {'framework'}, 'comparison mode roster')
        for mode, pin in row['outputs'].items():
            filename = '%s-pos%d-%s-%s.bf16le' % (name, position, kind, mode)
            require(pin in data['outputs'] and Path(pin['path']).name == filename and pin['bytes'] == count * 2, 'row/body join')
            used.add(filename)
        reference = output_bodies[Path(row['outputs']['framework']['path']).name]
        counts = {}
        for mode, metric in row['vs_framework'].items():
            body = output_bodies[Path(row['outputs'][mode]['path']).name]
            different = sum(body[i:i + 2] != reference[i:i + 2] for i in range(0, len(body), 2))
            first = [{'index': i // 2, 'actual_bits': int.from_bytes(body[i:i + 2], 'little'),
                      'expected_bits': int.from_bytes(reference[i:i + 2], 'little')}
                     for i in range(0, len(body), 2) if body[i:i + 2] != reference[i:i + 2]][:16]
            require(metric == {'words': count, 'exact_words': count - different,
                               'different_words': different, 'first_differences': first}, 'recorded bit counts')
            counts[mode] = {'words': count, 'exact_words': count - different, 'different_words': different}
        rows.append({'corpus': name, 'position': position, 'kind': kind, 'vs_framework': counts})
    require(used == set(output_bodies), 'closed 145-output roster')
    require(len(data['tables']) == 7 and [x['position'] for x in data['tables']] == POSITIONS, 'table positions')
    tables = [{'position': row['position'], 'cos': row['cos_comparison'], 'sin': row['sin_comparison']}
              for row in data['tables']]
    exact = all(row['vs_framework']['independent-materialized']['different_words'] == 0 for row in rows)
    require(data['all_independent_materialized_rows_exact'] is exact, 'materialization summary')
    inputs, unavailable = [], []
    for pin in data['inputs']:
        retained = local_input(pin)
        if retained is None:
            unavailable.append(pin)
        else:
            checked_pin(pin, retained)
            inputs.append({'original': pin, 'retained': CHECKED[str(retained)]})
    require(any(row['original']['sha256'] == SOURCE['run.py'][1] for row in inputs), 'executed source retained')
    pure_body = read(L / 'rope-framework-reference-pure-v228-v1.json', digest=args.pure_sha256)
    pure = json.loads(pure_body)
    require(pure['schema'] == 'ferric-p228-rope-reference-root-test-observation-v1' and
            (pure['exit_code'], pure['tests'], pure['failures'], pure['errors']) == (0, 14, 0, 0), 'actual pure observation')
    require(pure['observation_kind'] == 'Primary-agent observation of actual SSH tool output; not a remote supervisor receipt', 'test evidence scope')
    require(pure['source_sha256'] == {'run': SOURCE['run.py'][1], 'test': SOURCE['test_run.py'][1]}, 'pure sources')
    names = sorted(node.name for node in ast.walk(ast.parse(source_bodies['test_run.py']))
                   if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    observed = re.findall(r'^(test_\w+) \(__main__\.ArithmeticTests\) \.\.\. ok$', pure['test_output_excerpt'], re.M)
    require(len(names) == 14 and sorted(observed) == names and '\nOK\n' in pure['test_output_excerpt'], 'named pure transcript')
    self_path = Path(__file__).resolve()
    self_body = read(self_path)
    copies = {'complete.json': complete_body, 'pure-root-observation.json': pure_body, 'publish.py': self_body,
              **{'source/' + name: body for name, body in source_bodies.items()}}
    copies['table.json'] = encoded({'schema': 'ferric-p228-rope-reference-table-v1', 'rows': rows, 'tables': tables})
    copied_pins = [{'path': name, 'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()} for name, body in copies.items()]
    result = {'schema': 'ferric-p228-rope-reference-publication-v1', 'complete': CHECKED[str(CASE / 'complete.json')],
              'publication_helper': CHECKED[str(self_path)], 'copied': copied_pins,
              'all_output_buffers_rehashed': True, 'output_buffers': output_ledger,
              'locally_rehashed_input_bodies': inputs, 'input_bodies_not_locally_rehashed': unavailable,
              'pure_tests': {'tests': 14, 'observation_kind': pure['observation_kind'], 'pin': CHECKED[str(L / 'rope-framework-reference-pure-v228-v1.json')]},
              'all_28_materialized_rows_exact': exact,
              'inv_frequency_matches_historical_capture': data['inv_frequency_matches_historical_capture'],
              'runtime': data['runtime'], 'conditional_on_position_zero_qk': True,
              'f64_table_is_python_libm_model': True, 'gpu_execution': False, 'model_loaded': False,
              'cpu_trig_is_gpu_trig_evidence': False, 'actual_rust_tables_replayed': False,
              'transitive_framework_libraries_rehashed': False, 'historical_owner_audits_replayed': False,
              'numerical_acceptance': False, 'full_model_correctness': False, 'performance_claim': False,
              'production_authority': False}
    for pin in list(CHECKED.values()):
        read(pin['path'], pin['bytes'], pin['sha256'])
    require(Q.parent.resolve(strict=True) == Q.parent and not Q.is_symlink(), 'publication parent')
    if Q.exists():
        require(Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'} and
                not (Q / 'README.md').is_symlink(), 'fresh publication except root README')
    else:
        Q.mkdir()
    for name, body in {**copies, 'result.json': encoded(result)}.items():
        target = Q / name
        target.parent.mkdir(exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        require(target.read_bytes() == body, 'published bytes')
    print(json.dumps({'publication': str(Q), 'copied_files': len(copies) + 1,
                      'result_sha256': hashlib.sha256(encoded(result)).hexdigest()}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('pure_sha256', help='actual local root-observation JSON digest')
    main(parser.parse_args())
