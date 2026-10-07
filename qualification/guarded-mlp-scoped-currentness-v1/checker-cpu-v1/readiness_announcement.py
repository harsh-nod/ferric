"""Pure guarded-parent marker and already-recorded owned-lineage validation."""
import re

MARKER = re.compile(rb'finite guarded readiness owned child pid=([1-9][0-9]{0,9}) pgid=([1-9][0-9]{0,9}); setup pending')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def announcements(data):
    require(type(data) is bytes and len(data) <= 8 << 20, 'bounded original parent stderr')
    found = []
    for line in data.splitlines():
        if b'owned child' in line:
            match = MARKER.fullmatch(line)
            require(match is not None, 'exact guarded child announcement')
            pid, pgid = map(int, match.groups())
            require(pid == pgid and pid <= 2147483647, 'guarded child process-group leader')
            found.append(pid)
    require(len(found) == 1, 'one guarded child announcement')
    return found


def validate_lineage(stderr, native, started, child_pid):
    require(type(child_pid) is int and 0 < child_pid <= 2147483647
            and announcements(stderr) == [child_pid], 'observed guarded worker identity')
    require(type(native['exit_code']) is int and native['exit_code'] == 0
            and native['reason'] is None and native['cleanup_signalled'] is False
            and native['owned_groups_absent'] is True and native['owned_processes_reaped'] is True,
            'natural completed owned tree')
    records = native['lineage']
    require(type(records) is list and len(records) == 2
            and [r['event'] for r in records] == ['owned', 'owned']
            and [r['reason'] for r in records] == ['spawned-parent', 'ancestry'],
            'exact parent plus ancestry-discovered worker')
    parent, worker = [r['identity'] for r in records]
    fields = {'pid', 'pgid', 'ppid', 'sid', 'starttime', 'state', 'uid'}
    for identity in (parent, worker):
        require(set(identity) == fields and all(type(identity[k]) is int and identity[k] > 0
                for k in fields - {'state'}) and identity['uid'] == 9661
                and identity['state'] in ('R', 'S', 'D', 'I', 'T', 't', 'Z'),
                'closed positive process identity')
    require(set(started) == {'parent', 'supervisor_pid', 'command_sha256'}
            and type(started['supervisor_pid']) is int and started['supervisor_pid'] > 0
            and started['parent'] == parent and parent['ppid'] == started['supervisor_pid']
            and started['command_sha256'] == native['command']['sha256'], 'parent registration join')
    require(parent['pid'] == parent['pgid'] == parent['sid']
            and worker['pid'] == worker['pgid'] == child_pid
            and worker['ppid'] == parent['pid'] and worker['sid'] == parent['sid']
            and worker['pid'] != parent['pid'] and worker['starttime'] >= parent['starttime'],
            'worker same-session separate-group ancestry')
    groups = native['owned_groups']
    require(type(groups) is list and len(groups) == 2 and all(type(x) is int for x in groups)
            and sorted(groups) == sorted([parent['pid'], child_pid]), 'exact two retired groups')
    return dict(parent=parent, worker=worker, announcement_pid=child_pid,
                exact_two_owned_identities=True, separate_worker_process_group=True)
