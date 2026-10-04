"""Publish retained compiler refusal evidence without rerunning compilation."""
import hashlib
import json
from pathlib import Path
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
Q = Path('/home/harsh/ferric-p227-integration/qualification/rope-materialized-lowering-attempt-v1')
ROW = 'row-rope-materialized-checked-probe-v228-v1'
OWNER = 'rope-materialized-checked-probe-owner-v228-v1'
PACKAGE = 'p228-rope-materialized-lowering-v1'
ERROR = ('production semantic SSA partial-move validation for function 0 requires '
         '2097153 storage words, limit is 2097152')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.is_absolute() and path.is_file() and path.resolve(strict=True) == path,
            'canonical retained input')
    return dict(path=str(path), bytes=path.stat().st_size,
                sha256=hashlib.sha256(path.read_bytes()).hexdigest())


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode, 'ordinary -B publication')
    consumed, copies = {}, {}

    def checked(path, expected=None):
        value = pin(path)
        if isinstance(expected, str):
            require(value['sha256'] == expected, 'expected digest: ' + str(path))
        elif expected is not None:
            require(all(value[key] == expected[key] for key in ('bytes', 'sha256')),
                    'expected retained body: ' + str(path))
        consumed[str(path)] = value
        return value

    def doc(path, expected=None):
        checked(path, expected)
        return json.loads(path.read_bytes())

    def copy(path, name):
        require(name not in copies and not Path(name).is_absolute() and '..' not in Path(name).parts,
                'unique publication member')
        checked(path)
        copies[name] = path

    archive = checked(L / 'rope-materialized-checked-lowering-v228-v1-retained.tar.gz',
                      'b00191b860b87418ff761c62f4190e3e465351e21250a537d3c6f9a84e1f9bee')
    failed = doc(L / ROW / 'failed.json', '32aeb4127b38d0f50aabba67afd907f369185b46b64ebc012e2425acf31294f0')
    owner = doc(L / OWNER / 'failed.json', '46586eedb8fc35243c5e4bc920ef35e17323a1c0e371d0a33a8f94df6e9cf99e')
    require(failed['passed'] is False and failed['postcheck_errors'] == []
            and failed['error'] == 'AssertionError: checked-lowering'
            and failed['artifacts'] == {} and [v['name'] for v in failed['commands']] == ['fixture-metadata']
            and owner['passed'] is False and owner['postcheck_errors'] == []
            and owner['completion'] is None, 'actual failed lowering and owner')
    require(all(failed[key] is False for key in ('fresh_checked_lowering', 'fresh_checked_replay',
            'fresh_hsaco_emitted', 'gpu_execution', 'launch_authority', 'numerical_acceptance',
            'full_model_acceptance', 'performance_claim', 'production_authority')),
            'no downstream success or authority')
    owned = owner['owned']
    require(owned['exit_code'] == 1 and owned['reason'] is None
            and owned['owned_groups_absent'] is True and owned['owned_processes_reaped'] is True
            and owned['cleanup_signalled'] is False, 'natural failed owned process tree')
    for name, record in owner['raw'].items():
        require(record['path'] == str(E / OWNER / name), 'owned raw namespace')
        checked(L / OWNER / name, record)
        if name not in ('before.json', 'after.json'):
            copy(L / OWNER / name, 'owner/' + name)
    require(doc(L / OWNER / 'owned-result.json') == owned, 'actual owned result join')
    for directory in (ROW, OWNER):
        before, after = doc(L / directory / 'before.json'), doc(L / directory / 'after.json')
        require(all(after[key] == value for key, value in before.items()), 'unchanged recorded snapshots')
    require(doc(L / ROW / 'package-sources-before.json') == doc(L / ROW / 'package-sources-after.json'),
            'unchanged dependency snapshots')
    cpu = doc(L / 'rope-materialized-cpu-v228-v2/complete.json', failed['candidate_cpu'])
    require(cpu['passed'] is True and cpu['tests_passed'] == 33, 'actual preceding CPU result')
    package = doc(L / 'proposals' / PACKAGE / 'manifest.json', failed['package_manifest'])
    checked(L / 'proposals/p228-rope-materialized-source-v1/source-manifest.json', failed['source_manifest'])
    require(owner['package_manifest'] == failed['package_manifest']
            and owner['candidate_cpu'] == failed['candidate_cpu'], 'same owner/candidate chain')
    for item in package['files']:
        path = L / 'proposals' / PACKAGE / item['path']
        checked(path, item)
        copy(path, 'controller/' + item['path'])
    copy(L / 'proposals' / PACKAGE / 'manifest.json', 'controller/manifest.json')
    recipe = doc(L / ROW / 'recipe.json')
    require(len(recipe['fixture']) == 10 and len(recipe['commands']) == 9, 'original finite compiler recipe')
    for item in recipe['fixture']:
        name = item['destination']
        checked(L / ROW / 'fixture' / name, item['source'])
        expected = cpu['lowering_sources'].get(name, cpu['lowering_fixture_pins'].get(name))
        require(expected == item['source'], 'exact CPU-tested lowering input')
    for stage, code in (('fixture-metadata', 0), ('checked-lowering', 101)):
        result = doc(L / ROW / (stage + '-result.json'))
        require(result['exit_code'] == code and result['reason'] is None and result['group_absent'] is True,
                'actual completed compiler phase')
        for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr'):
            path = L / ROW / (stage + '-' + suffix)
            if suffix in ('stdout', 'stderr'):
                checked(path, result[suffix + '_sha256'])
            copy(path, 'raw/' + stage + '-' + suffix)
    require(ERROR in (L / ROW / 'checked-lowering-stderr').read_text(), 'exact compiler storage refusal')
    copy(L / ROW / 'failed.json', 'failed.json')
    copy(L / OWNER / 'failed.json', 'owner/failed.json')
    copy(L / ROW / 'recipe.json', 'recipe.json')
    for name in ('rope-materialized-lowering-pure-v228-v1.json',
                 'rope-materialized-lowering-first-launch-v228-v1.json',
                 'rope-materialized-lowering-manifest-rejected-v228-v1.json'):
        copy(L / name, 'observations/' + name)
    copy(Path(__file__).resolve(), 'publish.py')
    require(not Q.exists(), 'fresh publication directory')
    require(all(pin(Path(path)) == value for path, value in consumed.items()), 'unchanged publication inputs')
    Q.mkdir()
    ledger = {}
    for name, source in sorted(copies.items()):
        target = Q / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(source.read_bytes())
        value = pin(target)
        require(all(value[key] == consumed[str(source)][key] for key in ('bytes', 'sha256')), 'copied identity')
        ledger[name] = dict(source=consumed[str(source)], retained=value)
    require(all(pin(Path(path)) == value for path, value in consumed.items()), 'publication input postchecks')
    result = dict(schema='ferric-p228-rope-lowering-refusal-publication-v1', publication_passed=True,
        compilation_passed=False, compiler_error=ERROR, courier_archive=archive, files=ledger,
        locally_rehashed_inputs=list(consumed.values()), transitive_input_bodies_rehashed=False,
        all_test_executable_bodies_rehashed=False, gpu_execution=False, numerical_acceptance=False,
        full_model_acceptance=False, performance_claim=False, production_authority=False)
    with (Q / 'result.json').open('x') as stream:
        json.dump(result, stream, sort_keys=True, indent=2)
        stream.write('\n')
    print(json.dumps(dict(result=pin(Q / 'result.json'), files=len(ledger), compilation_passed=False)))


if __name__ == '__main__':
    main()
