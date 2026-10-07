"""Bind native signal observations to the independently replayed controller spans."""
import copy
import hashlib


def derive(original, stage, worker_sha):
    row = copy.deepcopy(original)
    if row['cell_id'] != 'counter-A' or row['spec']['arm'] != 'A':
        raise ValueError('baseline diagnostic only')
    worker = {'path': str(stage / 'worker-candidate'), 'sha256': worker_sha}
    row['spec']['worker'] = worker
    for argv in (row['spec']['argv'], row['common_args']):
        for flag, value in (('--worker', worker['path']), ('--worker-sha256', worker_sha)):
            if argv.count(flag) != 1 or argv.index(flag) + 1 >= len(argv):
                raise ValueError('one complete worker selector')
            argv[argv.index(flag) + 1] = value
    row['output'] = str(stage / 'cells/counter-A')
    return row


class Replay:
    def __init__(self, legacy, observation):
        self.legacy = legacy
        self.observation = observation
        self.SETUP = legacy.SETUP
        self.MAX_STDERR = observation.MAX_STREAM
        self.exact = legacy.exact
        self.clean_exit = legacy.clean_exit

    def replay(self, raw, spec, setup, counter_replay):
        w = self.observation
        w.require(spec['arm'] == 'A', 'wait capture is baseline only')
        records, legacy_raw = w.split_stream(raw)
        previous = self.legacy.replay(legacy_raw, spec, setup, counter_replay)
        w.require(previous['accepted'] is True, 'independent legacy replay required')
        breakdown = previous['breakdown']
        report = w.summarize(records, breakdown['epoch'])
        w.require(len(breakdown['executions']) == 127, 'complete controller execution list')
        for record, outer in zip(records[4:], breakdown['executions']):
            w.require(record['next_write'] == outer['frontier'], 'same controller/native execution')
            w.require(record['execution_return_ns'] <= outer['host_ns'],
                      'native observation exceeds enclosing controller execution')
        for rows, counters in ((records[:4], breakdown['before']['token']),
                               (records[4:], breakdown['token_counter_delta'])):
            w.require(sum(row['dispatches'] for row in rows) == counters['dispatches']
                      and len(rows) == counters['executions'], 'native/controller aggregate geometry')
        report.update(stderr_sha256=hashlib.sha256(raw).hexdigest(),
                      legacy_stderr_sha256=hashlib.sha256(legacy_raw).hexdigest(),
                      controller_spans_bound=True)
        return {'schema': 'FerricNativeWaitCaptureReplayV1', 'accepted': True,
                'legacy': previous, 'wait': report, 'performance_qualified': False,
                'latency_sample_admitted': False}
