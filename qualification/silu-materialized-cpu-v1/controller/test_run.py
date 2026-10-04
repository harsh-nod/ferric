"""Synthetic controller policies only; these tests do not compile Rust or run a GPU."""
import copy
import json
from pathlib import Path
import types
import unittest
from unittest.mock import patch

import run as C


def proposal():
    return dict(schema='ferric-p228-silu-materialized-kernel-proposal-v1',
                status='authored-not-executed', files=[
                    dict(path=C.DEVICE + '/' + name, source='draft/' + C.DEVICE + '/' + name,
                         before=None, after=dict(bytes=1, sha256='a' * 64))
                    for name in sorted(C.NEW_PATHS)],
                test_census={C.TARGETS[-1]: ['test_' + str(i) for i in range(16)]})


def prior():
    return dict(schema='ferric-p228-down2-cpu-result-v1', passed=True, error=None,
                postcheck_error=None, source_unchanged=True,
                actual_provider_source_sha256=C.PROVIDER_SHA, tests_passed=14, tests_ignored=0,
                phases={str(i): {} for i in range(8)}, tests={
                    name: dict(passed=count, ignored=0, names=['test_' + str(i) for i in range(count)])
                    for name, count in zip(C.TARGETS[:2], (4, 10))})


def messages():
    return [dict(reason='compiler-artifact', manifest_path='/fixture/Cargo.toml',
                 target=dict(name=name, kind=['test'], src_path='/fixture/tests/' + name + '.rs'),
                 profile=dict(test=True, opt_level='2', debug_assertions=True, overflow_checks=True),
                 executable='/target/debug/deps/' + name)
            for name in C.TARGETS] + [dict(reason='build-finished', success=True)]


def select(rows):
    with patch.object(C, 'pin', side_effect=lambda path: dict(path=str(path), bytes=1, sha256='a' * 64)), \
            patch.object(C.os, 'access', return_value=True):
        return C.select_tests('\n'.join(json.dumps(row) for row in rows), Path('/fixture'), Path('/target'))


class PolicyTests(unittest.TestCase):
    def test_exact_additive_candidate_and_sixteen_names(self):
        changes, names = C.source_proposal(proposal())
        self.assertEqual((len(changes), len(names)), (3, 16))

    def test_duplicate_candidate_path_or_test_name_is_refused(self):
        for mutate in ('path', 'name'):
            value = proposal()
            if mutate == 'path':
                value['files'][1] = copy.deepcopy(value['files'][0])
            else:
                value['test_census'][C.TARGETS[-1]][1] = 'test_0'
            with self.assertRaises(RuntimeError):
                C.source_proposal(value)

    def test_existing_file_replacement_is_refused(self):
        value = proposal()
        value['files'][0]['before'] = dict(bytes=1, sha256='b' * 64)
        with self.assertRaises(RuntimeError):
            C.source_proposal(value)

    def test_original_four_and_ten_cohorts_are_required(self):
        C.prior_contract(prior())
        value = prior()
        value['tests'][C.TARGETS[1]]['names'].pop()
        with self.assertRaises(RuntimeError):
            C.prior_contract(value)

    def test_failed_or_changed_provider_prerequisite_is_refused(self):
        for field, bad in (('passed', False), ('source_unchanged', False),
                           ('actual_provider_source_sha256', 'b' * 64), ('tests_ignored', 1)):
            value = prior()
            value[field] = bad
            with self.assertRaises(RuntimeError):
                C.prior_contract(value)

    def test_formatting_only_new_copies_is_allowed(self):
        before = {name: 'old' for name in C.NEW_PATHS | {'Cargo.lock', 'src/mlp_numerics_v1.rs'}}
        after = {**before, **{name: 'formatted' for name in C.NEW_PATHS}}
        C.format_transition(before, after)

    def test_formatter_cannot_change_original_macro_or_lock(self):
        before = {name: 'old' for name in C.NEW_PATHS | {'Cargo.lock', 'src/mlp_numerics_v1.rs'}}
        for name in ('Cargo.lock', 'src/mlp_numerics_v1.rs'):
            with self.assertRaises(RuntimeError):
                C.format_transition(before, {**before, name: 'changed'})

    def test_formatter_cannot_change_roster(self):
        before = {name: 'old' for name in C.NEW_PATHS}
        with self.assertRaises(RuntimeError):
            C.format_transition(before, {**before, 'src/unrelated.rs': 'new'})

    def test_actual_cargo_shape_selects_all_four_fresh_binaries(self):
        self.assertEqual(set(select(messages())), set(C.TARGETS))

    def test_unchecked_or_unoptimized_artifacts_are_refused(self):
        for field, bad in (('opt_level', '0'), ('debug_assertions', False),
                           ('overflow_checks', False), ('test', False)):
            rows = messages()
            rows[0]['profile'][field] = bad
            with self.assertRaises(RuntimeError):
                select(rows)

    def test_wrong_manifest_source_target_duplicate_or_failed_build_is_refused(self):
        for key, bad in (('manifest_path', '/old/Cargo.toml'), ('executable', '/old/target/test')):
            rows = messages()
            rows[0][key] = bad
            with self.assertRaises(RuntimeError):
                select(rows)
        rows = messages()
        rows[0]['target']['src_path'] = '/fixture/tests/unrelated.rs'
        with self.assertRaises(RuntimeError):
            select(rows)
        rows = messages()
        rows.insert(0, copy.deepcopy(rows[0]))
        with self.assertRaises(RuntimeError):
            select(rows)
        rows = messages()
        rows[-1]['success'] = False
        with self.assertRaises(RuntimeError):
            select(rows)

    def test_optimized_python_or_bytecode_writes_are_refused(self):
        with patch.dict(C.os.environ, {}, clear=True), \
                patch.object(C.sys, 'flags', types.SimpleNamespace(optimize=0)), \
                patch.object(C.sys, 'dont_write_bytecode', True):
            C.plain_python()
            with patch.dict(C.os.environ, {'PYTHONOPTIMIZE': '0'}):
                with self.assertRaises(RuntimeError):
                    C.plain_python()
            with patch.object(C.sys, 'flags', types.SimpleNamespace(optimize=1)):
                with self.assertRaises(RuntimeError):
                    C.plain_python()
            with patch.object(C.sys, 'dont_write_bytecode', False):
                with self.assertRaises(RuntimeError):
                    C.plain_python()


if __name__ == '__main__':
    unittest.main()
