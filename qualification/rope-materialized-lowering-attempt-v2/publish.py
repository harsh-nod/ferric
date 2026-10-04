"""Publish retained compiler refusal evidence without rerunning compilation."""
import hashlib
import json
from pathlib import Path
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
Q = Path('/home/harsh/ferric-p227-integration/qualification/rope-materialized-lowering-attempt-v2')
ROW = 'row-rope-materialized-checked-probe-v228-v2'
OWNER = 'rope-materialized-checked-probe-owner-v228-v2'
PACKAGE = 'p228-rope-materialized-lowering-v2'
ARCHIVE_SHA = '9054729f4bc013b287cbe264fc8ea084fa41cde262b8abaabe907ae876b83976'
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

    require(isinstance(ARCHIVE_SHA, str) and len(ARCHIVE_SHA) == 64, 'root-bound actual archive SHA')
    archive = checked(L / 'rope-materialized-checked-lowering-v228-v2-retained.tar.gz',
                      ARCHIVE_SHA)
    failed = doc(L / ROW / 'failed.json', 'e057e77261c6264aaf290f17616616811361b7df34cf7ef57be0ed87437c3784')
    owner = doc(L / OWNER / 'failed.json', '77d6e1d012a3b8ff85f8270e67da01c5ce826259cf8f8d39e3f273e381a84199')
    require(failed['schema'] == 'ferric-p228-rope-materialized-lowering-result-v2'
            and owner['schema'] == 'ferric-p228-rope-materialized-lowering-owned-result-v2'
            and failed['passed'] is False and failed['postcheck_errors'] == []
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
    cpu = doc(L / 'rope-materialized-cpu-v228-v3/complete.json', failed['candidate_cpu'])
    require(failed['candidate_cpu']['sha256'] == '8dcb4f2b326ab909c52039273515f44eb4f88cd91a967c8816f866d5514c864a'
            and cpu['passed'] is True and cpu['tests_passed'] == 33 and cpu['postcheck_errors'] == [],
            'actual preceding V2 CPU result')
    package = doc(L / 'proposals' / PACKAGE / 'manifest.json', failed['package_manifest'])
    checked(L / 'proposals/p228-rope-materialized-source-v2/source-manifest.json', failed['source_manifest'])
    require(failed['package_manifest']['sha256'] == 'fe58dd94a259347e65f8ca6006f6df7ed4b582b57aef5d45bcc0d45d78d0cd47'
            and failed['source_manifest']['sha256'] == '7f1f447852f01bad9b6dedb9b40a7969961d45861592465548e6401615318d23',
            'actual V2 lowering and source proposals')
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
    observation_path = L / 'rope-materialized-lowering-pure-v228-v2.json'
    observation = doc(observation_path,
        '6492c6eb5ee61feb4e5d08eb11de7d5ae617fdde47c682d9f7778fb4b5a5a373')
    require(observation['schema'] == 'ferric-p228-rope-lowering-pure-tool-observation-v2'
            and observation['exit_code'] == 0 and observation['test_count'] == 14
            and observation['manifest_sha256'] == failed['package_manifest']['sha256']
            and observation['controller_sha256'] == checked(L / 'proposals' / PACKAGE / 'run.py')['sha256']
            and observation['test_source_sha256'] == checked(L / 'proposals' / PACKAGE / 'test_run.py')['sha256']
            and observation['gpu_execution'] is False and observation['compiler_executed'] is False
            and observation['numerical_acceptance'] is False, 'actual root-observed V2 pure tests')
    copy(observation_path, 'observations/' + observation_path.name)
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
    result = dict(schema='ferric-p228-rope-lowering-refusal-publication-v2', publication_passed=True,
        compilation_passed=False, compiler_error=ERROR, courier_archive=archive, files=ledger,
        pure_test_observation=pin(observation_path), pure_test_observation_is_remote_supervisor_receipt=False,
        locally_rehashed_inputs=list(consumed.values()), transitive_input_bodies_rehashed=False,
        all_test_executable_bodies_rehashed=False, gpu_execution=False, numerical_acceptance=False,
        full_model_acceptance=False, performance_claim=False, production_authority=False)
    with (Q / 'result.json').open('x') as stream:
        json.dump(result, stream, sort_keys=True, indent=2)
        stream.write('\n')
    print(json.dumps(dict(result=pin(Q / 'result.json'), files=len(ledger), compilation_passed=False)))


if __name__ == '__main__':
    main()
