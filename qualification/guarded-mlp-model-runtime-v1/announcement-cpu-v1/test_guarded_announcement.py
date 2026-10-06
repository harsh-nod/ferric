"""Synthetic parser and retained-lineage regressions; no process or GPU access."""
import copy
import unittest

import guarded_announcement as G

LINE = b'finite guarded owned child pid=202 pgid=202; no native setup acknowledged\n'


def fixture():
    parent = dict(pid=101, pgid=101, ppid=99, sid=101, starttime=1000, state='R', uid=9661)
    worker = dict(pid=202, pgid=202, ppid=101, sid=101, starttime=1001, state='S', uid=9661)
    native = dict(exit_code=0, reason=None, cleanup_signalled=False, owned_groups_absent=True,
        owned_processes_reaped=True, owned_groups=[101, 202], command=dict(sha256='a' * 64),
        lineage=[dict(event='owned', identity=parent, reason='spawned-parent'),
                 dict(event='owned', identity=worker, reason='ancestry')])
    started = dict(parent=copy.deepcopy(parent), supervisor_pid=99, command_sha256='a' * 64)
    return native, started


class GuardedAnnouncementTests(unittest.TestCase):
    def test_exact_guarded_message(self):
        self.assertEqual(G.announcements(LINE + b'finite guarded completed position=0\n'), [202])
        actual = LINE.replace(b'202', b'2809580')
        self.assertEqual(G.announcements(actual), [2809580])

    def test_missing_duplicate_legacy_and_malformed_refuse(self):
        for raw in (b'', b'finite guarded completed position=0\n', LINE * 2,
                    LINE.replace(b'guarded', b'engineering'), LINE + LINE.replace(b'guarded', b'engineering'),
                    b'prefix ' + LINE, LINE.rstrip() + b' suffix\n', LINE.replace(b'pgid=202', b'pgid=203'),
                    LINE.replace(b'202', b'0'), LINE.replace(b'202', b'0202'),
                    LINE.replace(b'202', b'2147483648'), b'x' * ((8 << 20) + 1)):
            with self.subTest(raw=raw[:100]), self.assertRaises(RuntimeError): G.announcements(raw)

    def test_exact_two_owned_process_join(self):
        native, started = fixture()
        joined = G.validate_lineage(LINE, native, started, 202)
        self.assertTrue(joined['exact_two_owned_identities'])
        self.assertEqual(joined['worker']['ppid'], joined['parent']['pid'])

    def test_missing_duplicate_or_extra_owned_process_refuses(self):
        for selection in ([], [0], [1], [0, 0], [0, 1, 1], [1, 0]):
            native, started = fixture(); original = native['lineage']
            native['lineage'] = [copy.deepcopy(original[i]) for i in selection]
            with self.subTest(selection=selection), self.assertRaises(RuntimeError):
                G.validate_lineage(LINE, native, started, 202)
        for groups in ([101], [101, 101], [101, 202, 303], [True, 202]):
            native, started = fixture(); native['owned_groups'] = groups
            with self.assertRaises(RuntimeError): G.validate_lineage(LINE, native, started, 202)

    def test_wrong_worker_ancestry_identity_and_reason_refuse(self):
        for key, value in (('pid', 303), ('pgid', 101), ('ppid', 99), ('sid', 202),
                           ('uid', 0), ('starttime', 999), ('pid', True), ('state', '?')):
            native, started = fixture(); native['lineage'][1]['identity'][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                G.validate_lineage(LINE, native, started, 202)
        for key, value in (('event', 'announced'), ('reason', 'unproven')):
            native, started = fixture(); native['lineage'][1][key] = value
            with self.assertRaises(RuntimeError): G.validate_lineage(LINE, native, started, 202)
        native, started = fixture()
        with self.assertRaises(RuntimeError): G.validate_lineage(LINE, native, started, True)

    def test_nonnatural_or_started_drift_refuses(self):
        for key, value in (('exit_code', 1), ('exit_code', False), ('reason', 'deadline'),
                           ('cleanup_signalled', True), ('owned_groups_absent', False),
                           ('owned_processes_reaped', False)):
            native, started = fixture(); native[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                G.validate_lineage(LINE, native, started, 202)
        for key, value in (('supervisor_pid', 98), ('command_sha256', 'b' * 64)):
            native, started = fixture(); started[key] = value
            with self.assertRaises(RuntimeError): G.validate_lineage(LINE, native, started, 202)
        native, started = fixture(); started['parent']['starttime'] += 1
        with self.assertRaises(RuntimeError): G.validate_lineage(LINE, native, started, 202)


if __name__ == '__main__':
    unittest.main(verbosity=2)
