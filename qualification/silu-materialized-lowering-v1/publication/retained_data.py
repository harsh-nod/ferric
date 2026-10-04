"""Publish retained Down2 text evidence only; never imports or executes a controller."""
import ast
import hashlib
import json
from pathlib import Path
import re
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OUT = Path('/home/harsh/ferric-p227-integration/qualification/paired-row-mlp-lowering-v1')
ROW = 'row-down2-checked-probe-v228-v1'
OWNER = 'down2-checked-probe-owner-v228-v1'
CPU = 'down2-cpu-v228-v1'
PACKAGE = 'p228-down2-lowering-v2'
SOURCE = 'p228-mlp-down-two-row-source-v2'
PURE = 'down2-lowering-pure-v228-v2'
FORMAT = 'clock-and-down2-format-v228-v3'
DEVICE = 'device/qwen3-tp-wave-rmsnorm-kernels-v15/'
STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join',
          'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
FALSE = ('gpu_execution', 'production_authority', 'launch_authority', 'numerical_acceptance',
         'performance_claim', 'runtime_requirements_discharged', 'full_model_acceptance')
KNOWN = {
    ROW + '/complete.json': '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e',
    OWNER + '/complete.json': 'eeb7d115928426b9a03ca584210a0d79f8da2bc0f525768aefdba93d890df266',
    ROW + '/recipe.json': '4d6168f8d0420a2a48bff3200767192478392b76fdaaf40ac903d471965d610f',
    CPU + '/complete.json': '96ef991246b90f9f02b509b3302df0215309bb172f9cae4340c21eba27bda563',
    PURE + '/complete.json': '0a7742ebc15734e71d0ce953a82e865d7ba655890bb36f637889279398b50837',
    FORMAT + '/complete.json': '824e4437d430b9a9165aa462e101e7d61d54905a8229528be8359de6ebef0e8f',
}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda value: require(False, 'nonfinite JSON constant'))


def content(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def relative(value):
    path = Path(value)
    require(not path.is_absolute() and '..' not in path.parts
            and path.as_posix() == value and value not in ('', '.'), 'safe relative path')
    return path


def local(path):
    tail = Path(path).relative_to(E)
    candidate = L / tail
    if not candidate.exists() and tail.parts[0].startswith('p228-'):
        candidate = L / 'proposals' / tail
    require(candidate.resolve(strict=True) == candidate and candidate.is_file(), 'canonical retained file')
    return candidate


class Ledger:
    def __init__(self):
        self.inputs = {}
        self.copies = {}

    def take(self, pin, destination=None):
        require(set(pin) == {'path', 'bytes', 'sha256'} and type(pin['bytes']) is int
                and 0 <= pin['bytes'] <= 64 << 20
                and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'closed bounded FilePin')
        path = local(pin['path'])
        require(path.stat().st_size == pin['bytes'], 'retained byte count')
        raw = path.read_bytes()
        require(content(raw) == {key: pin[key] for key in ('bytes', 'sha256')}, 'retained digest')
        previous = self.inputs.setdefault(str(path), dict(source_pin=pin, local_path=str(path)))
        require(previous['source_pin'] == pin, 'conflicting retained pin')
        if destination is not None:
            relative(destination)
            require(len(raw) <= 2 << 20 and b'\0' not in raw, 'bounded text publication only')
            raw.decode('utf-8')
            existing = self.copies.setdefault(destination, (pin, raw))
            require(existing == (pin, raw), 'conflicting publication destination')
        return raw

    def document(self, pin, destination=None):
        return parse(self.take(pin, destination))

    def known(self, name, destination):
        raw = local(E / name).read_bytes()
        require(len(raw) <= 64 << 20 and hashlib.sha256(raw).hexdigest() == KNOWN[name],
                'actual retained prerequisite digest')
        pin = dict(path=str(E / name), **content(raw))
        return self.document(pin, destination), pin

    def recheck(self):
        for record in list(self.inputs.values()):
            self.take(record['source_pin'])


def passed(value, schema):
    require(value['schema'] == schema and value['passed'] is True and value['error'] is None
            and value['postcheck_errors'] == [], 'successful checked completion')
    require(all(value[key] is False for key in FALSE), 'no downstream authority')


def leaf(ledger, name, records, prefix):
    require(set(records) == {'command', 'started', 'result', 'stdout', 'stderr'}, 'closed leaf roster')
    suffix = {'command': '-command.json', 'started': '-started.json', 'result': '-result.json',
              'stdout': '-stdout', 'stderr': '-stderr'}
    for key, pin in records.items():
        require(pin['path'] == str(E / prefix / (name + suffix[key])), 'exact leaf path')
    data = {key: ledger.take(pin, prefix + '/' + Path(pin['path']).name) for key, pin in records.items()}
    command, start, result = (parse(data[key]) for key in ('command', 'started', 'result'))
    require(set(start) == {'pid', 'pgid'} and type(start['pid']) is int
            and start['pid'] > 0 and start['pid'] == start['pgid'], 'owned leaf process group')
    require(result['exit_code'] == 0 and result['reason'] is None and result['group_absent'] is True,
            'natural leaf exit and reap')
    require(all(result[key + '_sha256'] == records[key]['sha256'] for key in ('stdout', 'stderr')),
            'leaf stream hashes')
    require(command['gpu_execution'] is False and command['expected_exit'] == 0
            and command['affinity'] == [8, 9] and command['nice'] == 10
            and command['cache_cap_bytes'] == 6 << 30
            and result['cache_bytes'] <= command['cache_cap_bytes'], 'unchanged CPU leaf bounds')
    require(all(command['env'][key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES',
                                                  'CUDA_VISIBLE_DEVICES')), 'CPU-only leaf visibility')
    return command, result, data


def from_raw(raw, name):
    return {key: raw[name + suffix] for key, suffix in
            (('command', '-command.json'), ('started', '-started.json'), ('result', '-result.json'),
             ('stdout', '-stdout'), ('stderr', '-stderr'))}


def substitute(value, mapping):
    if isinstance(value, str):
        if value.startswith('@candidate.'):
            require(value in mapping, 'known candidate substitution')
            return str(mapping[value])
        return value
    if isinstance(value, list):
        return [substitute(item, mapping) for item in value]
    if isinstance(value, dict):
        return {key: substitute(item, mapping) for key, item in value.items()}
    return value


def rust_results(raw, expected):
    text = raw.decode('utf-8')
    actual = re.findall(r'^test (\S+) \.\.\. ok$', text, re.M)
    require(len(actual) == len(set(actual)) and set(actual) == set(expected), 'exact passing Rust names')
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text, re.M)
    require(summaries == [(str(len(expected)), '0', '0')], 'actual Rust outcomes')


def main():
    require(not sys.flags.optimize, 'optimized Python refused')
    require(len(sys.argv) == 1 and not OUT.is_symlink(), 'no arguments or symlink publication root')
    def fresh_output():
        require(not OUT.is_symlink(), 'no symlink publication root')
        if OUT.exists():
            require(OUT.resolve(strict=True) == OUT and OUT.is_dir()
                    and {path.name for path in OUT.iterdir()} <= {'README.md'}, 'only root-authored README may preexist')
            readme = OUT / 'README.md'
            require(not readme.is_symlink(), 'no symlink README')
            if readme.exists():
                require(readme.resolve(strict=True) == readme and readme.is_file(), 'regular root-authored README')
    fresh_output()
    ledger = Ledger()
    publisher_path = L / 'proposals/p228-down2-publication-v1/publish.py'
    require(Path(__file__).resolve(strict=True) == publisher_path and not publisher_path.is_symlink(),
            'exact local publication helper')
    require(publisher_path.stat().st_size <= 128 << 10, 'bounded publisher source')
    publisher_pin = dict(path=str(E / 'p228-down2-publication-v1/publish.py'),
                         **content(publisher_path.read_bytes()))
    ledger.take(publisher_pin, 'publication/publish.py')
    row, row_pin = ledger.known(ROW + '/complete.json', 'lowering/complete.json')
    owner, owner_pin = ledger.known(OWNER + '/complete.json', 'owner/complete.json')
    passed(row, 'ferric-p228-down2-lowering-result-v1')
    passed(owner, 'ferric-p228-down2-lowering-owned-result-v1')
    require(owner['completion'] == row_pin and all(owner[key] == row[key] for key in
            ('package_manifest', 'candidate_cpu', 'compiler_generation')), 'outer/inner exact identity joins')
    require(all(row[key] is True for key in ('fresh_checked_lowering', 'fresh_checked_replay',
            'fresh_hsaco_emitted', 'frontend_recipe_is_diagnostic'))
            and row['unresolved_runtime_requirements'] == 8, 'checked engineering-only scope')
    owned = owner['owned']
    require(owned['exit_code'] == 0 and owned['reason'] is None and owned['cleanup_signalled'] is False
            and owned['owned_groups_absent'] is True and owned['owned_processes_reaped'] is True,
            'natural outer completion and all owned descendants reaped')
    owner_data = {name: ledger.take(pin, None if name in ('before.json', 'after.json') else 'owner/' + name)
                  for name, pin in owner['raw'].items()}
    require(set(owner_data) == {'before.json', 'after.json', 'command.json', 'owned-result.json',
            'started.json', 'stderr', 'stdout'} and parse(owner_data['owned-result.json']) == owned,
            'closed outer raw records')
    start = parse(owner_data['started.json'])
    require(owned['lineage'][0]['identity'] == start['parent']
            and start['parent']['pid'] in owned['owned_groups'], 'outer start/ownership identity')
    before, after = parse(owner_data['before.json']), parse(owner_data['after.json'])
    require(all(before[key] == after[key] for key in ('files', 'old_targets', 'compiler_source_roster')),
            'recorded immutable-source/product/cache postchecks')
    package = ledger.document(row['package_manifest'], 'controller/manifest.json')
    require(row['package_manifest']['path'] == str(E / PACKAGE / 'manifest.json')
            and package['schema'] == 'ferric-p228-down2-lowering-package-v2'
            and package['pure_tests'] == 17 and {entry['path'] for entry in package['files']}
            == {'run.py', 'test_run.py', 'contracts.py', 'README.md'}, 'exact four-file controller package')
    package_pins = {}
    for entry in package['files']:
        pin = dict(entry, path=str(E / PACKAGE / entry['path']))
        package_pins[entry['path']] = pin
        ledger.take(pin, 'controller/' + entry['path'])
    recipe, recipe_pin = ledger.known(ROW + '/recipe.json', 'lowering/recipe.json')
    require(recipe['compiler_generation'] == row['compiler_generation']['prerequisites']
            and tuple(item['name'] for item in recipe['commands']) == STAGES
            and tuple(item['name'] for item in row['commands']) == STAGES, 'exact nine-stage generation')
    outer_command = parse(owner_data['command.json'])
    require(outer_command['argv'] == ['/usr/bin/python3', '-B', str(E / PACKAGE / 'run.py'),
            row['package_manifest']['sha256'], '--child']
            and outer_command['env'] == recipe['commands'][0]['env']
            and outer_command['deadline_seconds'] == owned['deadline_seconds'] == 10800
            and outer_command['affinity'] == [8, 9] and outer_command['nice'] == 10
            and outer_command['gpu_execution'] is False and outer_command['address_space_bytes'] <= 12 << 30
            and outer_command['file_cap_bytes'] <= 1 << 30 and outer_command['core_bytes'] == 0,
            'actual bounded owner launches only this exact child package')
    artifacts = row['artifacts']
    require(set(artifacts) == {'mlp-tiles-semantic.bin', 'mlp-tiles-neutral-kir.bin',
            'mlp-tiles-target-kir.bin', 'mlp-tiles.handoff-v3', 'emitted/source.handoff-v3',
            'emitted/compiler.handoff-v2', 'emitted/artifact.hsaco', 'emitted/receipt.txt',
            'extracted/formal.archive', 'extracted/module.ll'}, 'ten closed artifact records')
    for name, pin in artifacts.items():
        require(pin['path'] == str(E / ROW / relative(name)) and pin['bytes'] > 0, 'artifact path/extent')
        ledger.take(pin, 'lowering/' + name if name in ('extracted/module.ll', 'emitted/receipt.txt') else None)
    require(all(artifacts['mlp-tiles.handoff-v3'][key] == artifacts['emitted/source.handoff-v3'][key]
                for key in ('bytes', 'sha256')), 'captured and emitted source handoff equality')
    replacements = {'@candidate.semantic.sha256': artifacts['mlp-tiles-semantic.bin']['sha256'],
        '@candidate.handoff.sha256': artifacts['mlp-tiles.handoff-v3']['sha256'],
        '@candidate.handoff.bytes': artifacts['mlp-tiles.handoff-v3']['bytes'],
        '@candidate.image.sha256': artifacts['emitted/artifact.hsaco']['sha256'],
        '@candidate.image.bytes': artifacts['emitted/artifact.hsaco']['bytes']}
    phase_summary, streams = {}, {}
    for recorded, template in zip(row['commands'], recipe['commands']):
        name = recorded['name']
        command, result, data = leaf(ledger, name, {key: recorded[key] for key in
            ('command', 'started', 'result', 'stdout', 'stderr')}, ROW)
        expected = substitute({key: value for key, value in template.items() if key not in ('name', 'cwd')}, replacements)
        require(0 < command['deadline_seconds'] <= expected['deadline_seconds'], 'unchanged or shortened deadline')
        expected['deadline_seconds'] = command['deadline_seconds']
        require(command == expected, 'actual command equals authenticated recipe substitution')
        phase_summary[name] = result
        streams[name] = data
    for name in ('actual-replay', 'actual-inert-join'):
        rust_results(streams[name]['stdout'], [parse(streams[name]['command'])['argv'][2]])
    require(all(('stage=' + name + ' status=complete').encode() in streams['checked-lowering']['stderr']
                for name in ('pre-ranked', 'neutral', 'target-before-llvm')), 'actual checked stage completion')
    require(b'stage=ranked status=complete' in streams['actual-replay']['stdout']
            and b'stage=formal status=complete roots=11 unresolved=8' in streams['actual-replay']['stdout'],
            'ranked/formal actual replay completed')
    metadata = {}
    lines = streams['descriptor-metadata']['stdout'].decode().splitlines()
    require(lines[0] == 'fe2o3-finite-join-request-metadata-v1', 'descriptor metadata header')
    for line in lines[1:]:
        key, value = line.split(' ', 1)
        require(key not in metadata, 'duplicate descriptor key')
        metadata[key] = value
    require(metadata['authority'] == 'none' and metadata['target'] == 'gfx950:xnack-'
            and metadata['object_sha256'] == artifacts['emitted/artifact.hsaco']['sha256']
            and metadata['object_bytes'] == str(artifacts['emitted/artifact.hsaco']['bytes'])
            and metadata['explicit_argument_bytes'] == '88' and metadata['kernarg_segment_bytes'] == '344'
            and metadata['workgroup'] == metadata['max_grid_workgroups'] == '64,1,1', 'image/ABI/geometry join')
    require(all(marker in streams['elf-notes']['stdout'] for marker in
            (b'.group_segment_fixed_size: 512', b'.private_segment_fixed_size: 0', b'.wavefront_size: 64')),
            'actual resource notes')
    require(b'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2' in streams['disassembly']['stdout']
            and b's_endpgm' in streams['disassembly']['stdout'], 'actual symbol disassembly')
    emission = ledger.take(artifacts['emitted/receipt.txt']).decode()
    image = artifacts['emitted/artifact.hsaco']
    require(all(line in emission.splitlines() for line in (
            'wave_mlp_tile_engineering_emission_v2', 'status PASS', 'authority none',
            'runtime_requirements required_undischarged', 'unresolved_requirement_count 8',
            'finalized_hsaco_content ' + image['sha256'] + ' ' + str(image['bytes']))),
            'inert emission receipt joins exact image without runtime authority')

    cpu, cpu_pin = ledger.known(CPU + '/complete.json', 'cpu/complete.json')
    require(cpu['schema'] == 'ferric-p228-down2-cpu-result-v1'
            and cpu_pin == row['candidate_cpu'] and cpu['passed'] is True and cpu['error'] is None
            and cpu['postcheck_error'] is None and cpu['source_unchanged'] is True
            and cpu['tests_passed'] == 14 and cpu['tests_ignored'] == 0 and cpu['cpu_arithmetic_only'] is True,
            'actual CPU14 qualification')
    require(all(cpu[key] is False for key in ('gpu_execution', 'compiler_hsaco_reproduced',
            'numerical_acceptance', 'performance_claim', 'production_authority', 'full_model_acceptance')), 'CPU scope')
    require(len(cpu['phases']) == 8 and len(cpu['raw']) == 45, 'CPU phase/raw census')
    cpu_raw = {name: ledger.take(pin, None if name.endswith('-before.json') or name.endswith('-after.json')
                or name == 'sources-base.json' else 'cpu/' + name) for name, pin in cpu['raw'].items()}
    for name, result in cpu['phases'].items():
        _, actual, _ = leaf(ledger, name, from_raw(cpu['raw'], name), CPU)
        require(actual == result, 'CPU actual result join')
    require(cpu_raw['sources-before.json'] == cpu_raw['sources-after.json']
            and cpu_raw['dependencies-before.json'] == cpu_raw['dependencies-after.json'], 'CPU recorded source stability')
    for name, count in (('mlp_down_two_row_v1', 10), ('mlp_tiles_numerics_v2', 4)):
        names = cpu['tests'][name]['names']
        require(len(names) == count and cpu['tests'][name]['passed'] == count
                and cpu['tests'][name]['ignored'] == 0, 'CPU test census')
        listed = re.findall(r'^(\S+): test$', cpu_raw[name + '-list-stdout'].decode(), re.M)
        require(sorted(listed) == sorted(names) and cpu_raw[name + '-ignored-list-stdout'] == b'', 'CPU actual inventories')
        rust_results(cpu_raw[name + '-stdout'], names)
    ledger.take(cpu['runner'], 'cpu/controller.py')
    ledger.take(cpu['source_inputs'], 'cpu/source-inputs.json')
    overlay = ledger.document(cpu['overlay'], 'source/source-manifest.json')
    require(cpu['overlay']['path'] == str(E / SOURCE / 'source-manifest.json')
            and len(overlay['files']) == 3, 'actual formatted three-file overlay')
    formatter, format_pin = ledger.known(FORMAT + '/complete.json', 'format/complete.json')
    require(formatter['passed'] is True and formatter['gpu_execution'] is False
            and formatter['tests_executed'] is False, 'formatter is not a test receipt')
    for name in ('rustfmt', 'rustfmt-check'):
        _, actual, _ = leaf(ledger, name, from_raw(formatter['raw'], name), FORMAT)
        require(actual == formatter['phases'][name], 'formatter actual results')
    for name, pin in formatter['raw'].items():
        ledger.take(pin, 'format/' + name)
    ledger.take(formatter['controller'], 'format/controller.py')
    cpu_sources = parse(cpu_raw['sources-before.json'])['fixture']
    for entry in overlay['files']:
        path = relative(entry['path'])
        require(entry['path'].startswith(DEVICE) and entry['source'] == 'draft/' + entry['path'], 'source scope')
        pin = dict(path=str(E / SOURCE / entry['source']), **entry['after'])
        require(formatter['formatted']['p228-mlp-down-two-row-v1'][entry['path']] == pin, 'formatted source join')
        ledger.take(pin, 'source/' + str(path))
        require(cpu_sources[entry['path'][len(DEVICE):]] == entry['after'], 'CPU fixture exact source body')
    require(set(cpu['tests']['mlp_down_two_row_v1']['names']) == set(overlay['tests_authored_not_run']), 'authored/actual new tests')
    for destination, source_name in (('src/lib.rs', 'src/finite_mlp_tiles_v2.rs'),
                                    ('src/mlp_tile_numerics_v2.rs', 'src/mlp_tile_numerics_v2.rs')):
        fixture = next(item for item in recipe['fixture'] if item['destination'] == destination)
        expected = next(item for item in overlay['files'] if item['path'] == DEVICE + source_name)
        require(fixture['source'] == dict(path=str(E / SOURCE / expected['source']), **expected['after']),
                'checked compiler fixture equals CPU tested arithmetic')

    pure, pure_pin = ledger.known(PURE + '/complete.json', 'pure/complete.json')
    require(pure['schema'] == 'ferric-p228-down2-lowering-pure-v1' and pure['passed'] is True
            and pure['tests'] == 17 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
            and pure['manifest_sha256'] == row['package_manifest']['sha256'], 'actual pure17 qualification')
    require(all(pure[key] is False for key in ('gpu_execution', 'native_execution', 'numerical_acceptance',
            'performance_claim', 'production_authority', 'full_model_acceptance')), 'pure policy tests only')
    pure_before = ledger.document(pure['sources_before'], 'pure/sources-before.json')
    require(pure_before == ledger.document(pure['sources_after'], 'pure/sources-after.json') == package_pins,
            'tested exact controller source census')
    transcript = ledger.take(pure['transcript'], 'pure/tests.log').decode()
    tree = ast.parse(ledger.take(package_pins['test_run.py']).decode())
    expected = {node.name for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith('test_')}
    actual = re.findall(r'^(test_\w+) \(test_run\.PolicyTests\) \.\.\. ok$', transcript, re.M)
    require(len(actual) == len(set(actual)) == len(expected) == 17 and set(actual) == expected
            and re.search(r'^Ran 17 tests in .+s$', transcript, re.M) and transcript.rstrip().endswith('OK'), 'actual named pure passes')
    pure_controller = local(E / 'run_down2_lowering_pure_p228_v2.py').read_bytes()
    ledger.take(dict(path=str(E / 'run_down2_lowering_pure_p228_v2.py'), bytes=len(pure_controller),
                     sha256=pure['controller_sha256']), 'pure/controller.py')
    ledger.recheck()
    require(sum(len(body) for _, body in ledger.copies.values()) <= 8 << 20, 'bounded text checkpoint')
    summary = dict(schema='ferric-p228-paired-row-mlp-lowering-publication-v1', passed=True,
        publication_helper=publisher_pin,
        lower_receipt=row_pin, owner_receipt=owner_pin, cpu_receipt=cpu_pin, pure_receipt=pure_pin,
        formatter_receipt=format_pin, recipe=recipe_pin, metadata=metadata, artifacts=artifacts,
        phases=phase_summary, cpu_tests=cpu['tests'], pure_tests_passed=17,
        source_fixture_join_checked=True, ten_artifact_bodies_rehashed=True,
        natural_nine_phase_exits_checked=True, outer_owned_reaping_checked=True,
        transitive_inputs_replayed=False, compiler_tool_bodies_rehashed=False,
        gpu_images_selected=False, isa_review_accepted=False, **{key: False for key in FALSE},
        limitations=[
            'This local publisher rehashes only the explicit ledger, not every transitive compiler/provider/library input.',
            'Actual retained controller receipts report complete pre/post custody; large source/cache maps are not published.',
            'The fourteen CPU tests exercise source arithmetic and rejection paths; no GPU numerical acceptance follows.',
            'Eight runtime requirements remain undischarged. No runtime review or launch authority is minted.',
            'Binary artifacts, compiler ELFs, archives and pre/post maps remain in retained evidence outside Git.',
            'No live image is selected; no calibration, kernel speedup, whole-model or throughput claim is made.'],
        inputs=list(ledger.inputs.values()), published_files=[dict(path=name, source_pin=pin)
            for name, (pin, _) in sorted(ledger.copies.items())])
    fresh_output()
    OUT.mkdir(exist_ok=True)
    for name, (_, raw) in sorted(ledger.copies.items()):
        path = OUT / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    with (OUT / 'result.json').open('x') as stream:
        json.dump(summary, stream, indent=2, sort_keys=True)
        stream.write('\n')
    ledger.recheck()
    print(json.dumps(dict(output=str(OUT), published_files=len(ledger.copies),
                          result=content((OUT / 'result.json').read_bytes())), sort_keys=True))


if __name__ == '__main__':
    main()
