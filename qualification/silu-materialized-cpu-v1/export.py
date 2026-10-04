"""Export actual completed SiLU CPU evidence; no compiler or GPU invocation."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
TARGETS = {'mlp_tiles_numerics_v2': 4, 'mlp_down_two_row_v1': 10,
           'mlp_claimed_numerics_v1': 8, 'mlp_silu_materialized_v1': 16}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.is_file() and path.resolve(strict=True) == path, 'canonical regular retained file')
    h = hashlib.sha256()
    with path.open('rb') as stream:
        while raw := stream.read(1 << 20):
            h.update(raw)
    return dict(path=str(path), bytes=path.stat().st_size, sha256=h.hexdigest())


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and len(sys.argv) == 3, 'export.py ACTUAL_COMPLETE_PATH ACTUAL_COMPLETE_SHA')
    receipt, digest = Path(sys.argv[1]), sys.argv[2]
    require(receipt.parent.parent == E and receipt.name == 'complete.json'
            and re.fullmatch(r'silu-materialized-cpu-v228-v[1-9][0-9]*', receipt.parent.name)
            and pin(receipt)['sha256'] == digest, 'actual CPU completion identity')
    root = receipt.parent
    value = json.loads(receipt.read_bytes())
    require(value['schema'] == 'ferric-p228-silu-materialized-cpu-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['source_unchanged'] is True and value['tests_passed'] == 38 and value['tests_ignored'] == 0
            and len(value['phases']) == 16 and len(value['raw']) == 86
            and set(value['binaries']) == set(value['tests']) == set(TARGETS), 'successful actual CPU38 cohort')
    require(all(row['exit_code'] == 0 and row['reason'] is None and row['group_absent'] is True
                for row in value['phases'].values()), 'all phases naturally completed')
    require(all(value[key] is False for key in ('gpu_execution', 'compiler_hsaco_reproduced',
                'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority')),
            'CPU-only nonclaims')
    selected = {receipt: pin(receipt)}

    def add(record):
        path = Path(record['path'])
        require(path.is_relative_to(root) and pin(path) == record, 'retained body pin')
        require(path not in selected or selected[path] == record, 'conflicting file identity')
        selected[path] = record

    for name, record in value['raw'].items():
        require(Path(record['path']) == root / name and Path(name).name == name, 'flat CPU raw file')
        add(record)
    before = json.loads((root / 'sources-before.json').read_bytes())
    require(json.loads((root / 'sources-after.json').read_bytes()) == before, 'source postchecks')
    require(len(before['fixture']) == 34, 'complete tested numerical fixture')
    for name, record in before['fixture'].items():
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'relative fixture member')
        add(dict(path=str(root / 'fixture' / name), **record))
    for name, row in value['binaries'].items():
        require(Path(row['binary']['path']).is_relative_to(root / 'target')
                and row['cargo']['executable'] == row['binary']['path']
                and value['tests'][name]['passed'] == TARGETS[name]
                and value['tests'][name]['ignored'] == 0, 'actual selected test executable')
        add(row['binary'])
    require(len(value['formatted_sources']) == 3 and len(value['lowering_sources']) == 5,
            'formatted and checked-lowering source rosters')
    source_paths = set()
    for record in [*value['formatted_sources'].values(), *value['lowering_sources'].values()]:
        path = Path(record['path'])
        relative = str(path.relative_to(root / 'fixture'))
        require({key: record[key] for key in ('bytes', 'sha256')} == before['fixture'][relative],
                'actual tested formatted source')
        add(record)
        source_paths.add(path)
    require(len(source_paths) == 6 and len(selected) == 125, 'deduplicated CPU export census')
    archive = E / (root.name + '-retained.tar.gz')
    require(not os.path.lexists(archive), 'fresh export archive')
    with tarfile.open(archive, 'x:gz') as tar:
        for path in sorted(selected):
            require(pin(path) == selected[path], 'input changed before archive write')
            tar.add(path, arcname=str(path.relative_to(E)), recursive=False)
    require(all(pin(path) == record for path, record in selected.items()), 'export input postchecks')
    print(json.dumps(dict(archive=pin(archive), members=len(selected),
                          bytes=sum(row['bytes'] for row in selected.values()),
                          gpu_execution=False, compilation=False)), flush=True)


if __name__ == '__main__':
    main()
