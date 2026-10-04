"""Bounded retained-data export and syntactic LLVM inspection, never a compiler or GPU launcher."""
import collections
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROW = E / 'row-silu-materialized-checked-probe-v228-v1'
OWNER = E / 'silu-materialized-checked-probe-owner-v228-v1'
BASE = E / 'row-down2-checked-probe-v228-v1'
BASE_SHA = '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e'
PACKAGE = E / 'p228-silu-materialized-lowering-v1'
PACKAGE_SHA = '24ff0ec4b86f2e60b7945b06ede8bcdec4fbd99131d33c3eabdb7c22dfb1e8b3'
SYMBOL = 'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2'
STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join',
          'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
ARTIFACTS = {'mlp-tiles-semantic.bin', 'mlp-tiles-neutral-kir.bin', 'mlp-tiles-target-kir.bin',
             'mlp-tiles.handoff-v3', 'emitted/source.handoff-v3', 'emitted/compiler.handoff-v2',
             'emitted/artifact.hsaco', 'emitted/receipt.txt', 'extracted/formal.archive', 'extracted/module.ll'}
FIXTURE = {'Cargo.toml', 'Cargo.lock', 'src/lib.rs', 'src/wave_numerics_v1.rs',
           'src/mlp_numerics_v1.rs', 'src/mlp_tile_numerics_v2.rs', 'src/mlp_silu_materialized_numerics_v1.rs'}
NARROW = '__fe2o3_f32_to_bf16_rne_v1'
WIDEN = '__fe2o3_bf16_to_f32_v1'
SSA = r'%[A-Za-z0-9_.]+'
ONE = '0x3FF0000000000000'
ZERO = '0x0000000000000000'
PINS = {}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def parse(raw):
    def closed(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=closed,
                      parse_constant=lambda value: (_ for _ in ()).throw(RuntimeError(value)))


def read(path, expected=None, maximum=64 << 20):
    path = Path(path)
    require(path.is_relative_to(E) and path.resolve(strict=True) == path, 'original canonical evidence path')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= maximum, 'bounded regular body')
        raw = stream.read(maximum + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(len(raw) == before.st_size and stamp(before) == stamp(after) == stamp(path.lstat()), 'stable body')
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(expected is None or (pin['sha256'] == expected if type(expected) is str else pin == expected), 'exact body pin')
    require(str(path) not in PINS or PINS[str(path)] == pin, 'conflicting original identity')
    PINS[str(path)] = pin
    require(len(PINS) <= 200 and sum(row['bytes'] for row in PINS.values()) <= 256 << 20, 'bounded complete export')
    return pin, raw


def doc(record):
    path = record['path'] if type(record) is dict else record
    return parse(read(path, record if type(record) is dict else None)[1])


def helper_body(text, name):
    matches = list(re.finditer(r'^define internal [^\n]*@' + re.escape(name) + r'\([^\n]*\{\n.*?^}', text, re.M | re.S))
    require(len(matches) == 1, 'one exact internal BF16 helper')
    return matches[0].group()


def llvm_chain(text, materialized):
    """Recognize only this emitter's literal SSA shapes; not a general LLVM proof."""
    match = re.search(r'^define amdgpu_kernel void @' + SYMBOL + r'\([^\n]*\{\n(.*?)^}', text, re.M | re.S)
    require(match is not None, 'actual selected LLVM kernel')
    definitions = {}
    for line_number, line in enumerate(text[:match.end()].splitlines(), 1):
        row = re.fullmatch(r'  (' + SSA + r') = (.+)', line)
        if row and line_number > text[:match.start()].count('\n'):
            require(row[1] not in definitions, 'unique kernel SSA definitions')
            definitions[row[1]] = (row[2], line_number)

    def unique(pattern):
        rows = [(name, re.fullmatch(pattern, body), number) for name, (body, number) in definitions.items()]
        rows = [(name, found, number) for name, found, number in rows if found]
        require(len(rows) == 1, 'one literal SSA match: ' + pattern)
        return rows[0]

    def rhs(name, pattern):
        found = re.fullmatch(pattern, definitions[name][0])
        require(found is not None, 'literal SSA dependency: ' + name)
        return found

    def alias_i16(name):
        seen = set()
        while True:
            require(name not in seen, 'no integer alias cycle')
            seen.add(name)
            found = re.fullmatch(r'add i16 (' + SSA + r'), 0', definitions[name][0])
            if not found:
                return name
            name = found[1]

    exp, call, _ = unique(r'call float @__ocml_exp_f32\(float (' + SSA + r')\)')
    negative = call[1]
    absolute = rhs(negative, r'fneg float (' + SSA + r')')[1]
    gate = rhs(absolute, r'call float @llvm.fabs.f32\(float (' + SSA + r')\)')[1]
    rhs(gate, r'call float @' + WIDEN + r'\(i16 (' + SSA + r')\)')
    denominator, _, _ = unique(r'fadd float ' + ONE + ', ' + re.escape(exp))
    sigmoid, divide, _ = unique(r'fdiv float (' + SSA + r'), ' + re.escape(denominator))
    numerator = divide[1]
    rhs(numerator, r'phi float \[ ' + ONE + r', %[^ ]+ \], \[ ' + re.escape(exp) + r', %[^ ]+ \]')
    sign, _, _ = unique(r'fcmp oge float ' + re.escape(gate) + ', ' + ZERO)
    silu, _, _ = unique(r'fmul float ' + re.escape(gate) + ', ' + re.escape(sigmoid))
    nodes = dict(gate=gate, absolute=absolute, negative=negative, exp=exp, sign_test=sign,
                 numerator=numerator, denominator=denominator, sigmoid=sigmoid, silu_fp32=silu)
    product_input = silu
    if materialized:
        narrow, _, _ = unique(r'call i16 @' + NARROW + r'\(float ' + re.escape(silu) + r'\)')
        widenings = [(name, found) for name, (body, _) in definitions.items()
                     if (found := re.fullmatch(r'call float @' + WIDEN + r'\(i16 (' + SSA + r')\)', body))
                     and alias_i16(found[1]) == narrow]
        require(len(widenings) == 1, 'one actual SiLU narrow/widen dependency')
        product_input = widenings[0][0]
        nodes.update(silu_bf16=narrow, silu_widened=product_input)
    product, multiply, _ = unique(r'fmul float ' + re.escape(product_input) + ', (' + SSA + r')')
    up = multiply[1]
    rhs(up, r'call float @' + WIDEN + r'\(i16 (' + SSA + r')\)')
    final, _, _ = unique(r'call i16 @' + NARROW + r'\(float ' + re.escape(product) + r'\)')
    nodes.update(up=up, product_fp32=product, final_bf16=final)
    rows = {key: dict(ssa=name, line=definitions[name][1], instruction=definitions[name][0])
            for key, name in nodes.items()}
    first = max(1, min(row['line'] for row in rows.values()) - 6)
    last = min(len(text.splitlines()), max(row['line'] for row in rows.values()) + 45)
    require(last - first <= 320, 'bounded contiguous arithmetic excerpt')
    return dict(literal_chain_matched=True, materialization_expected=materialized, nodes=rows,
                excerpt=[dict(line=i, text=text.splitlines()[i - 1]) for i in range(first, last + 1)],
                phi_predecessor_semantics_verified=False, finite_guard_control_flow_verified=False,
                input_root_equivalence_verified=False, complete_arithmetic_equivalence_proved=False)


def resources(notes, isa):
    values = {}
    for key in ('group_segment_fixed_size', 'private_segment_fixed_size', 'kernarg_segment_size',
                'kernarg_segment_align', 'max_flat_workgroup_size', 'wavefront_size',
                'sgpr_count', 'sgpr_spill_count', 'vgpr_count', 'vgpr_spill_count'):
        rows = re.findall(r'^\s*\.' + key + r':\s*([0-9]+)\s*$', notes, re.M)
        require(len(rows) == 1, 'one actual kernel resource field: ' + key)
        values[key] = int(rows[0])
    require(re.findall(r'^\s*\.name:\s*' + SYMBOL + r'\s*$', notes, re.M), 'selected metadata symbol')
    instructions = re.findall(r'^\s+([a-z][a-z0-9_]+)\b[^\n]*// [0-9A-Fa-f]+:', isa, re.M)
    require(instructions and 's_endpgm' in instructions, 'actual disassembled instructions')
    return dict(metadata=values, decoded_instruction_lines=len(instructions),
                mnemonic_counts=dict(sorted(collections.Counter(instructions).items())),
                instruction_census_scope='whole retained disassembly, including library bodies',
                instruction_count_is_not_dynamic_work=True, gpu_duration_measured=False)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python')
    require(len(sys.argv) == 4, 'OWNER_COMPLETE_PATH ACTUAL_OWNER_SHA NEW_EXPORT_LABEL')
    owner_path, digest, label = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    require(owner_path == OWNER / 'complete.json' and re.fullmatch('[0-9a-f]{64}', digest)
            and re.fullmatch(r'silu-materialized-inspection-v228-v[1-9][0-9]*', label), 'closed actual inputs')
    owner_pin, owner_raw = read(owner_path, digest)
    owner = parse(owner_raw)
    require(owner['schema'] == 'ferric-p228-silu-materialized-lowering-owned-result-v1'
            and owner['passed'] is True and owner['error'] is None and owner['postcheck_errors'] == [], 'successful owner')
    owned = owner['owned']
    require(type(owned['exit_code']) is int and owned['exit_code'] == 0 and owned['reason'] is None
            and owned['cleanup_signalled'] is False and owned['owned_groups_absent'] is True
            and owned['owned_processes_reaped'] is True, 'natural reaped compiler owner')
    require(owner['completion']['path'] == str(ROW / 'complete.json'), 'inner completion namespace')
    lower = doc(owner['completion'])
    require(lower['schema'] == 'ferric-p228-silu-materialized-lowering-result-v1'
            and lower['passed'] is True and lower['error'] is None and lower['postcheck_errors'] == []
            and all(lower[key] == owner[key] for key in
                    ('candidate_cpu', 'source_manifest', 'prior_lowering', 'package_manifest', 'compiler_generation'))
            and lower['unresolved_runtime_requirements'] == 8
            and all(lower[key] is True for key in ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted')),
            'actual full checked completion')
    require(lower['package_manifest']['path'] == str(PACKAGE / 'manifest.json')
            and lower['package_manifest']['sha256'] == PACKAGE_SHA, 'exact reviewed lowering package')
    for value in (lower, owner):
        require(all(value[key] is False for key in ('gpu_execution', 'launch_authority', 'numerical_acceptance',
                    'full_model_acceptance', 'runtime_requirements_discharged', 'production_authority', 'performance_claim')),
                'compiler receipt is not runtime acceptance')
    require(set(owner['raw']) == {'before.json', 'after.json', 'command.json', 'started.json',
                                  'owned-result.json', 'stdout', 'stderr'}, 'all owner raw records')
    for name, record in owner['raw'].items():
        require(record['path'] == str(OWNER / name), 'owner record namespace')
        read(record['path'], record)
    require(doc(owner['raw']['owned-result.json']) == owned, 'owned result byte join')
    require(tuple(row['name'] for row in lower['commands']) == STAGES and set(lower['artifacts']) == ARTIFACTS,
            'nine checked stages and ten artifacts')
    for row in lower['commands']:
        for key in ('command', 'started', 'result', 'stdout', 'stderr'):
            suffix = '-' + key + ('.json' if key in ('command', 'started', 'result') else '')
            record = row[key]
            require(record['path'] == str(ROW / (row['name'] + suffix)), 'actual leaf path')
            read(record['path'], record)
        result = doc(row['result'])
        require(type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and all(result[key + '_sha256'] == row[key]['sha256']
                                                         for key in ('stdout', 'stderr')), 'natural leaf streams')
    for name, record in lower['artifacts'].items():
        require(record['path'] == str(ROW / name), 'actual artifact path')
        read(record['path'], record, 32 << 20)
    for name in ('recipe.json', 'before.json', 'after.json', 'package-sources-before.json', 'package-sources-after.json'):
        read(ROW / name)
    recipe = doc(ROW / 'recipe.json')
    require(len(recipe['fixture']) == 7 and {row['destination'] for row in recipe['fixture']} == FIXTURE,
            'exact seven-file fixture')
    for row in recipe['fixture']:
        expected = dict(row['source'], path=str(ROW / 'fixture' / row['destination']))
        read(expected['path'], expected, 4 << 20)
    manifest = doc(lower['package_manifest'])
    require({row['path'] for row in manifest['files']} == {'run.py', 'contracts.py', 'test_run.py', 'README.md'}, 'package members')
    for row in manifest['files']:
        read(PACKAGE / row['path'], dict(row, path=str(PACKAGE / row['path'])))
    doc(lower['source_manifest'])
    cpu = doc(lower['candidate_cpu'])
    read(cpu['runner']['path'], cpu['runner'])
    for key in ('sources-before.json', 'sources-after.json'):
        read(cpu['raw'][key]['path'], cpu['raw'][key])
    for record in [*cpu['formatted_sources'].values(), *cpu['lowering_sources'].values()]:
        read(record['path'], record, 4 << 20)
    require(lower['prior_lowering']['path'] == str(BASE / 'complete.json')
            and lower['prior_lowering']['sha256'] == BASE_SHA, 'actual checked Down2 predecessor')
    baseline = doc(lower['prior_lowering'])
    modules, machine = {}, {}
    for role, value in (('baseline', baseline), ('candidate', lower)):
        llvm_pin = value['artifacts']['extracted/module.ll']
        modules[role] = read(llvm_pin['path'], llvm_pin, 16 << 20)[1].decode('utf-8')
        streams = {row['name']: row['stdout'] for row in value['commands']}
        machine[role] = resources(read(streams['elf-notes']['path'], streams['elf-notes'])[1].decode(),
                                  read(streams['disassembly']['path'], streams['disassembly'])[1].decode())
    inspection = {}
    try:
        require(all(helper_body(modules['baseline'], name) == helper_body(modules['candidate'], name)
                    for name in (NARROW, WIDEN)), 'exact unchanged RNE and widening helper bodies')
        inspection = {role: llvm_chain(text, role == 'candidate') for role, text in modules.items()}
        inspection.update(literal_fp32_exp_div_expression_shape_matches=True,
                          exact_bf16_helper_bodies_equal=True, error=None)
    except (RuntimeError, KeyError, IndexError) as failure:
        inspection = dict(error=str(failure), literal_fp32_exp_div_expression_shape_matches=False,
                          exact_bf16_helper_bodies_equal=None)
    out = E / label
    require(not os.path.lexists(out), 'fresh inspection output')
    self_pin = read(Path(__file__).resolve(), maximum=128 << 10)[0]
    result = dict(schema='ferric-p228-silu-materialized-inspection-v1', owner=owner_pin,
        completion=owner['completion'], controller=self_pin, source_manifest=lower['source_manifest'],
        candidate_cpu=lower['candidate_cpu'], llvm=inspection, resources=machine,
        static_inspection_complete=inspection['error'] is None, root_manual_review_required=True,
        control_flow_equivalence_proved=False, isa_arithmetic_equivalence_proved=False,
        ocml_accuracy_proved=False, native_execution=False, gpu_execution=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False,
        transitive_compiler_sources_replayed=False, cargo_cache_exported=False,
        exported_inputs=sorted(PINS.values(), key=lambda row: row['path']))
    out.mkdir(mode=0o700)
    archive_path = out / 'retained.tar.gz'
    with archive_path.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for record in result['exported_inputs']:
            raw = read(record['path'], record)[1]
            info = tarfile.TarInfo(str(Path(record['path']).relative_to(E)))
            info.size, info.mode, info.mtime = len(raw), 0o600, 0
            archive.addfile(info, io.BytesIO(raw))
    for record in result['exported_inputs']:
        read(record['path'], record)
    # The compressed tar is an output, not part of its own original-body ledger.
    result['archive'] = read(archive_path, maximum=256 << 20)[0]
    with (out / 'complete.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    print(json.dumps(dict(output=str(out), static_inspection_complete=result['static_inspection_complete'],
                          manual_review_required=True, files=len(result['exported_inputs']), archive=result['archive'])), flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
