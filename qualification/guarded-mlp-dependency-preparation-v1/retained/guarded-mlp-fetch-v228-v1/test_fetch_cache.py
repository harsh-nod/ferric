"""Pure fixture tests for explicit fetch cache mutation boundaries."""
import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('guarded_fetch_cache_tests', Path(__file__).with_name('run_fetch.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


def pin(section, name, digest='a'):
    return dict(path='/home/harmenon/.cargo/registry/' + section + '/' + name,
                bytes=3, sha256=digest * 64)


def before():
    return {section: {'old': pin(section, 'old')} for section in ('src', 'cache', 'index')}


class FetchCacheTests(unittest.TestCase):
    def test_unchanged_cache_has_empty_delta(self):
        value = before()
        self.assertEqual(R.registry_changes(value, copy.deepcopy(value)),
            {section: dict(added=[], removed=[], changed=[]) for section in value})

    def test_new_source_and_archive_bodies_are_explicit_additions(self):
        old = before()
        new = copy.deepcopy(old)
        for section in ('src', 'cache'):
            new[section]['new'] = pin(section, 'new', 'b')
        delta = R.registry_changes(old, new)
        for section in ('src', 'cache'):
            self.assertEqual(delta[section], dict(added=['new'], removed=[], changed=[]))

    def test_existing_source_and_archive_mutation_or_removal_is_refused(self):
        for section in ('src', 'cache'):
            for mutation in ('removed', 'changed'):
                with self.subTest(section=section, mutation=mutation):
                    old = before()
                    new = copy.deepcopy(old)
                    if mutation == 'removed':
                        del new[section]['old']
                    else:
                        new[section]['old']['sha256'] = 'b' * 64
                    with self.assertRaisesRegex(RuntimeError, 'existing registry ' + section):
                        R.registry_changes(old, new)

    def test_index_mutations_are_recorded_without_relaxing_source_custody(self):
        old = before()
        old['index']['removed'] = pin('index', 'removed')
        new = copy.deepcopy(old)
        new['index']['old']['sha256'] = 'b' * 64
        del new['index']['removed']
        new['index']['added'] = pin('index', 'added')
        delta = R.registry_changes(old, new)
        self.assertEqual(delta['index'], dict(added=['added'], removed=['removed'], changed=['old']))
        self.assertEqual(delta['src'], dict(added=[], removed=[], changed=[]))
        self.assertEqual(delta['cache'], dict(added=[], removed=[], changed=[]))


if __name__ == '__main__':
    unittest.main()
