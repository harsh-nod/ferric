import copy
import json
from pathlib import Path
import tempfile
import unittest

import child_evidence as C
import prepare as P
from test_validation import fixture


def child_fixture(directory, request, binary, observed):
    """Synthetic process metadata only; never a process execution or GPU observation."""
    parent = dict(pid=12345, ppid=12344, pgid=12345, sid=12345, uid=9661, starttime=100, state='S')
    started = P.save(directory / 'native-started.json', dict(parent=parent, supervisor_pid=12344))
    lineage = [dict(event='owned', reason='spawned-parent', identity=parent)]
    for index, (label, profile) in enumerate(C.PROFILES):
        pid = 12346 + index
        identity = dict(parent, pid=pid, ppid=12345, starttime=101 + index)
        lineage.append(dict(event='owned', reason='ancestry', identity=identity))
        stem = directory / ('prefix-' + label + '-child-')
        def path(kind): return Path(str(stem) + kind)
        P.save(path('command.json'), dict(schema='fe2o3-prefix-profile-command-v1', profile=profile,
            argv=['/proc/12345/fd/9', C.CHILD_FLAG, label, request['path'], request['sha256'], binary['sha256']],
            executable=binary, request=request, parent_pid=12345, session='inherited-outer-owner', stdin='null', retry=False))
        P.save(path('started.json'), dict(schema='fe2o3-prefix-profile-started-v1', profile=profile,
            pid=pid, parent_pid=12345, executable=binary, request=request))
        stdout = P.save(path('stdout.bin'), dict(schema='fe2o3-prefix-profile-child-v1', profile=profile,
            pid=pid, request=request, executable=binary,
            progress=dict(profile=profile, group_open_attempted=True, group_open_returned=True,
                close_attempted=True, closed=True), states=observed['profiles'][index]['states'],
            host_dispatch_elapsed_ns=observed['profiles'][index]['host_dispatch_elapsed_ns'],
            input_sha256=observed['input_sha256'], initial_output_sha256=observed['initial_output_sha256'],
            captures=observed['captures'][index], paired_comparison=False, production_authority=False))
        path('stderr.bin').write_bytes(b''); stderr = P.D.read_file(path('stderr.bin'))[0]
        P.save(path('result.json'), dict(schema='fe2o3-prefix-profile-result-v1', profile=profile,
            pid=pid, parent_pid=12345, wait_returned=True, exit_code=0, signal=None, passed=True,
            error=None, executable=binary, request=request, stdout=stdout, stderr=stderr))
    return dict(started=started, lineage=lineage)


class Tests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name).resolve()
        _, self.request, _, _, self.observed, _ = fixture()
        self.binary = dict(path='/deployed/fixed-binary', bytes=100, sha256='b' * 64)
        self.native = child_fixture(self.directory, self.request, self.binary, self.observed)

    def check(self):
        return C.validate(P, P.D.Pins(), self.directory, self.request, self.binary, self.observed, self.native)

    def change(self, kind, action, label='baseline-v5', repin=False):
        path = self.directory / ('prefix-' + label + '-child-' + kind)
        value = json.loads(path.read_bytes()); action(value)
        path.write_text(json.dumps(value))
        if repin:
            result = self.directory / ('prefix-' + label + '-child-result.json')
            body = json.loads(result.read_bytes()); body['stdout' if kind == 'stdout.bin' else 'stderr'] = P.D.read_file(path)[0]
            result.write_text(json.dumps(body))

    def test_exact_ten_sidecars_join_two_reaped_profile_descendants(self):
        value = self.check()
        self.assertEqual([row['pid'] for row in value['profiles']], [12346, 12347])
        self.assertEqual(len(C.records(P, P.D.Pins(), self.directory, True)), 10)
        self.assertFalse(value['production_authority']); self.assertEqual(value['retries'], 0)

    def test_closed_command_rejects_foreign_fd_profile_or_extra_arguments(self):
        path = self.directory / 'prefix-baseline-v5-child-command.json'; raw = path.read_bytes()
        mutations = [lambda x: x['argv'].__setitem__(0, '/proc/999/fd/9'),
            lambda x: x['argv'].__setitem__(0, '/proc/12345/fd/2'),
            lambda x: x['argv'].__setitem__(2, 'tiles-v6'), lambda x: x['argv'].append('extra'),
            lambda x: x.update(retry=True), lambda x: x.update(session='new-session')]
        for mutate in mutations:
            path.write_bytes(raw); self.change('command.json', mutate)
            with self.assertRaises(RuntimeError): self.check()

    def test_closed_child_receipt_rejects_claims_scope_and_open_state(self):
        path = self.directory / 'prefix-baseline-v5-child-stdout.bin'; raw = path.read_bytes()
        for mutate in (lambda x: x.update(paired_comparison=True), lambda x: x.update(production_authority=True),
                lambda x: x.update(profile='tiles_v6'), lambda x: x.update(pid=12345),
                lambda x: x['progress'].update(closed=False), lambda x: x.update(unknown=False),
                lambda x: x['request'].update(sha256='a' * 64)):
            path.write_bytes(raw); self.change('stdout.bin', mutate, repin=True)
            with self.assertRaises(RuntimeError): self.check()

    def test_actual_owner_lineage_cannot_be_detached_reused_or_extended(self):
        original = copy.deepcopy(self.native)
        mutations = [lambda n: n['lineage'].pop(0),
            lambda n: n['lineage'][1]['identity'].update(pgid=12346),
            lambda n: n['lineage'][1]['identity'].update(ppid=999),
            lambda n: n['lineage'][1]['identity'].update(uid=999),
            lambda n: n['lineage'][1]['identity'].update(pid=999),
            lambda n: n['lineage'][1]['identity'].update(starttime=99),
            lambda n: n['lineage'].append(copy.deepcopy(n['lineage'][1]))]
        for mutate in mutations:
            self.native = copy.deepcopy(original); mutate(self.native)
            with self.assertRaises(RuntimeError): self.check()
        self.native = original
        self.change('started.json', lambda x: x.update(pid=12346), label='tiles-v6')
        with self.assertRaises(RuntimeError): self.check()

    def test_short_lived_waited_child_never_gets_invented_outer_pidfd_membership(self):
        self.native['lineage'].pop()
        mixed = self.check()
        self.assertEqual([row['outer_pidfd_observed'] for row in mixed['profiles']], [True, False])
        self.native['lineage'] = self.native['lineage'][:1]
        value = self.check()
        self.assertTrue(all(row['identity'] is None and row['outer_pidfd_observed'] is False
                            for row in value['profiles']))
        self.change('result.json', lambda x: x.update(wait_returned=False))
        with self.assertRaises(RuntimeError): self.check()

    def test_wait_error_status_and_stream_swap_cannot_be_success(self):
        path = self.directory / 'prefix-baseline-v5-child-result.json'; raw = path.read_bytes()
        for mutate in (lambda x: x.update(wait_returned=False), lambda x: x.update(exit_code=1),
                lambda x: x.update(exit_code=False), lambda x: x.update(signal=9),
                lambda x: x.update(error='wait failure'), lambda x: x.update(stdout=x['stderr'])):
            path.write_bytes(raw); self.change('result.json', mutate)
            with self.assertRaises(RuntimeError): self.check()

    def test_duplicate_json_extra_stdout_and_nonempty_stderr_are_refused(self):
        path = self.directory / 'prefix-baseline-v5-child-stdout.bin'; raw = path.read_bytes()
        for changed in (raw + b'{}', raw.replace(b'"pid": 12346', b'"pid": 12346, "pid": 12346')):
            path.write_bytes(changed)
            result = self.directory / 'prefix-baseline-v5-child-result.json'
            body = json.loads(result.read_bytes()); body['stdout'] = P.D.read_file(path)[0]
            result.write_text(json.dumps(body))
            with self.assertRaises((RuntimeError, ValueError)): self.check()
        path.write_bytes(raw)
        stderr = self.directory / 'prefix-baseline-v5-child-stderr.bin'
        stderr.write_bytes(b'diagnostic')
        result = self.directory / 'prefix-baseline-v5-child-result.json'
        body = json.loads(result.read_bytes()); body['stdout'] = P.D.read_file(path)[0]
        body['stderr'] = P.D.read_file(stderr)[0]; result.write_text(json.dumps(body))
        with self.assertRaises(RuntimeError): self.check()

    def test_child_capture_state_timing_and_immutable_hashes_match_native_record(self):
        path = self.directory / 'prefix-baseline-v5-child-stdout.bin'; raw = path.read_bytes()
        for mutate in (lambda x: x['captures'][0].update(sha256='a' * 64),
                lambda x: x['states'][0].__setitem__(0, 0),
                lambda x: x['host_dispatch_elapsed_ns'].__setitem__(0, 999),
                lambda x: x['input_sha256'][0].__setitem__(0, 'f' * 64),
                lambda x: x['initial_output_sha256'][1].__setitem__(0, 'f' * 64)):
            path.write_bytes(raw); self.change('stdout.bin', mutate, repin=True)
            with self.assertRaises(RuntimeError): self.check()

    def test_failure_retains_only_existing_profile_files_without_complete_claim(self):
        oversized = self.directory / 'prefix-baseline-v5-child-stdout.bin'
        oversized.write_bytes(b'x' * ((64 << 10) + 1))
        partial = C.records(P, P.D.Pins(), self.directory, False)
        self.assertEqual(partial[oversized.name]['bytes'], (64 << 10) + 1)
        with self.assertRaises(RuntimeError): self.check()
        for name in C.NAMES[5:]: (self.directory / name).unlink()
        rows = C.records(P, P.D.Pins(), self.directory, False)
        self.assertEqual(set(rows), set(C.NAMES[:5]))
        with self.assertRaises(RuntimeError): self.check()
        (self.directory / 'prefix-unexpected.json').write_text('{}')
        with self.assertRaises(RuntimeError): C.records(P, P.D.Pins(), self.directory, False)

    def test_alias_and_retained_byte_drift_are_rejected(self):
        path = self.directory / C.NAMES[0]; saved = self.directory / 'saved'
        saved.write_bytes(path.read_bytes()); path.unlink(); path.symlink_to(saved)
        with self.assertRaises(RuntimeError): C.records(P, P.D.Pins(), self.directory, True)
        path.unlink(); path.write_bytes(saved.read_bytes())
        pins = P.D.Pins(); C.records(P, pins, self.directory, True)
        path.write_bytes(path.read_bytes() + b' ')
        with self.assertRaises(RuntimeError): pins.recheck()


if __name__ == '__main__':
    unittest.main()
