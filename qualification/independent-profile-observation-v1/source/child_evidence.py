"""Replay the two closed disposable profile children; no process or GPU actions."""
from pathlib import Path
import re
import validation as V

PROFILES = (('baseline-v5', 'baseline_v5'), ('tiles-v6', 'tiles_v6'))
KINDS = ('command.json', 'started.json', 'stdout.bin', 'stderr.bin', 'result.json')
NAMES = tuple('prefix-' + label + '-child-' + kind for label, _ in PROFILES for kind in KINDS)
CHILD_FLAG = '--execute-reviewed-engineering-prefix-profile-child-v1'


def records(P, pins, directory, complete):
    """Failure intake retains existing partial sidecars without inventing a child result."""
    present = {path.name for path in directory.iterdir() if path.name.startswith('prefix-')}
    V.require(present <= set(NAMES) and (not complete or present == set(NAMES)),
              'closed ten profile-child sidecars')
    rows = {}
    for name in NAMES:
        if name in present:
            limit = 8 << 20 if name.endswith(('-stdout.bin', '-stderr.bin')) else 64 << 10
            pin, _ = pins.read(directory / name, maximum=limit)
            rows[name] = pin
    return rows


def validate(P, pins, directory, request, binary, observed, native):
    rows = records(P, pins, directory, True)
    started = P.document(pins, native['started'])
    parent = started['parent']; pid = V.uint(parent['pid'], 0xffffffff)
    V.require(pid > 0 and parent['ppid'] == started['supervisor_pid']
        and parent['pgid'] == parent['sid'] == pid and parent['uid'] == 9661,
        'actual outer native parent identity')
    lineage = native['lineage']
    V.require(type(lineage) is list and len(lineage) <= 128, 'bounded actual native lineage')
    owned = [row['identity'] for row in lineage if row.get('event') == 'owned']
    V.require(parent in owned and 1 <= len(owned) <= 3
        and len({(row['pid'], row['starttime']) for row in owned}) == len(owned),
        'native parent and at most two actually observed profile descendants')
    children, pids = [], []
    for index, (label, profile) in enumerate(PROFILES):
        stem = 'prefix-' + label + '-child-'
        file = {kind: rows[stem + kind] for kind in KINDS}
        command = P.document(pins, file['command.json'], 64 << 10)
        start = P.document(pins, file['started.json'], 64 << 10)
        result = P.document(pins, file['result.json'], 64 << 10)
        V.keys(command, 'schema profile argv executable request parent_pid session stdin retry')
        V.require(command['schema'] == 'fe2o3-prefix-profile-command-v1' and command['profile'] == profile
            and command['executable'] == binary and command['request'] == request
            and V.uint(command['parent_pid']) == pid and command['session'] == 'inherited-outer-owner'
            and command['stdin'] == 'null' and command['retry'] is False,
            'exact private profile command scope')
        argv = command['argv']
        V.require(type(argv) is list and len(argv) == 6 and type(argv[0]) is str
            and re.fullmatch('/proc/' + str(pid) + r'/fd/[0-9]+', argv[0])
            and int(argv[0].rsplit('/', 1)[1]) >= 3
            and argv[1:] == [CHILD_FLAG, label, request['path'], request['sha256'], binary['sha256']],
            'retained parent executable FD and one exact child selector')
        V.keys(start, 'schema profile pid parent_pid executable request')
        child = V.uint(start['pid'], 0xffffffff)
        V.require(child > 0 and child != pid and child not in pids
            and start == dict(schema='fe2o3-prefix-profile-started-v1', profile=profile,
                pid=child, parent_pid=pid, executable=binary, request=request), 'fresh distinct profile PID')
        pids.append(child)
        identities = [row for row in owned if row['pid'] == child]
        V.require(len(identities) <= 1, 'no child PID incarnation ambiguity')
        if identities:
            V.require(identities[0]['ppid'] == pid and identities[0]['pgid'] == identities[0]['sid'] == pid
                and identities[0]['uid'] == parent['uid'] and identities[0]['starttime'] >= parent['starttime'],
                'actually observed inherited child lineage')
        V.keys(result, 'schema profile pid parent_pid wait_returned exit_code signal passed error executable request stdout stderr')
        V.require(result == dict(schema='fe2o3-prefix-profile-result-v1', profile=profile, pid=child,
            parent_pid=pid, wait_returned=True, exit_code=0, signal=None, passed=True, error=None,
            executable=binary, request=request, stdout=file['stdout.bin'], stderr=file['stderr.bin'])
            and type(result['exit_code']) is int and result['wait_returned'] is True
            and result['passed'] is True, 'actual successful wait and retained stream identities')
        V.require(P.read(pins, file['stderr.bin'], True, maximum=8 << 20) == b'', 'clean profile stderr')
        receipt = P.document(pins, file['stdout.bin'], 64 << 10)
        V.keys(receipt, 'schema profile pid request executable progress states host_dispatch_elapsed_ns '
            'input_sha256 initial_output_sha256 captures paired_comparison production_authority')
        V.require(receipt['schema'] == 'fe2o3-prefix-profile-child-v1' and receipt['profile'] == profile
            and V.uint(receipt['pid']) == child and receipt['request'] == request
            and receipt['executable'] == binary and receipt['paired_comparison'] is False
            and receipt['production_authority'] is False, 'private child receipt, never paired success')
        V.require(receipt['progress'] == dict(profile=profile, group_open_attempted=True,
            group_open_returned=True, close_attempted=True, closed=True)
            and all(receipt['progress'][key] is True for key in
                ('group_open_attempted', 'group_open_returned', 'close_attempted', 'closed')),
            'explicit per-child native Close')
        native_profile = observed['profiles'][index]
        V.require(native_profile['profile'] == profile and native_profile['closed'] is True
            and receipt['states'] == native_profile['states']
            and receipt['host_dispatch_elapsed_ns'] == native_profile['host_dispatch_elapsed_ns']
            and receipt['captures'] == observed['captures'][index]
            and receipt['input_sha256'] == observed['input_sha256']
            and receipt['initial_output_sha256'] == observed['initial_output_sha256'],
            'child controls/captures/immutable roots join the paired observation')
        children.append(dict(profile=profile, pid=child, identity=identities[0] if identities else None,
            outer_pidfd_observed=bool(identities), files=file))
    V.require({row['pid'] for row in owned} <= {pid, *pids}, 'no unrelated observed descendants')
    return dict(schema='ferric-p227-prefix-profile-process-evidence-v1', parent=parent,
        profiles=children, profile_attempts=2, retries=0, shared_outer_process_group=True,
        production_authority=False)
