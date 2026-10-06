"""Synthetic source-contract regressions; no compiler or process launch."""
import unittest
import run_cpu as C


class SourceContractTests(unittest.TestCase):
    def insertions(self):
        before = {'one.rs': b'head\nanchor\ntail\n', 'two.rs': b'old\nend\n'}
        patch = (b'*** Begin Patch\n*** Update File: one.rs\n@@\n anchor\n+new\n tail\n'
                 b'*** Update File: two.rs\n@@\n old\n+added\n end\n*** End Patch\n')
        return before, patch

    def relocations(self):
        return {str(C.RPO_SOURCE): str(C.SOURCE),
                str(C.RPO_ROOT / 'target'): str(C.TARGET),
                '/home/harmenon/ferric-asrock-42/toolchain/cargo/registry':
                    '/home/harmenon/.cargo/registry'}

    def test_two_insertion_hunks_preserve_all_other_bytes(self):
        before, patch = self.insertions()
        result = C.inserted_source(before, patch)
        self.assertEqual(result, {'one.rs': b'head\nanchor\nnew\ntail\n',
                                  'two.rs': b'old\nadded\nend\n'})
        self.assertEqual(before['one.rs'], b'head\nanchor\ntail\n')

    def test_insertion_refuses_deletion(self):
        before, patch = self.insertions()
        with self.assertRaises(RuntimeError):
            C.inserted_source(before, patch.replace(b'+new\n', b'-new\n'))

    def test_insertion_refuses_missing_or_duplicate_context(self):
        before, patch = self.insertions()
        for changed in (b'head\nother\ntail\n', b'anchor\ntail\nanchor\ntail\n'):
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                C.inserted_source(dict(before, **{'one.rs': changed}), patch)

    def test_insertion_refuses_unknown_or_repeated_destination(self):
        before, patch = self.insertions()
        for changed in (patch.replace(b'two.rs', b'three.rs'),
                        patch.replace(b'two.rs', b'one.rs')):
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                C.inserted_source(before, changed)

    def test_insertion_refuses_empty_and_missing_destination(self):
        before, patch = self.insertions()
        with self.assertRaises(RuntimeError):
            C.inserted_source(before, b'')
        with self.assertRaises(RuntimeError):
            C.inserted_source(dict(before, **{'three.rs': b'other'}), patch)

    def test_metadata_relocates_paths_and_structured_path_ids(self):
        old = str(C.RPO_SOURCE)
        value = {'workspace_root': old, 'target_directory': str(C.RPO_ROOT / 'target'),
                 'packages': [{'id': 'path+file://' + old + '/crates/example#0.1.0',
                               'manifest_path': old + '/crates/example/Cargo.toml',
                               'source': 'registry+https://example.invalid/index#name@1.0'}]}
        result = C.relocated_metadata(value, self.relocations())
        self.assertEqual(result['workspace_root'], str(C.SOURCE))
        self.assertEqual(result['target_directory'], str(C.TARGET))
        self.assertEqual(result['packages'][0]['id'],
                         'path+file://' + str(C.SOURCE) + '/crates/example#0.1.0')
        self.assertEqual(result['packages'][0]['source'], value['packages'][0]['source'])
        self.assertEqual(value['workspace_root'], old)

    def test_metadata_does_not_replace_embedded_text_or_prefix_sibling(self):
        old = str(C.RPO_SOURCE)
        value = [old + '-unrelated', 'documentation about ' + old,
                 '/home/harmenon/ferric-asrock-42/toolchain/cargo/registry/src/pkg/lib.rs']
        result = C.relocated_metadata(value, self.relocations())
        self.assertEqual(result[:2], value[:2])
        self.assertEqual(result[2], '/home/harmenon/.cargo/registry/src/pkg/lib.rs')

    def test_metadata_refuses_wrong_source_or_external_destination(self):
        wrong = self.relocations()
        wrong[str(C.RPO_SOURCE)] = str(C.TARGET)
        with self.assertRaises(RuntimeError):
            C.relocated_metadata({}, wrong)
        wrong = self.relocations()
        wrong['/home/harmenon/ferric-asrock-42/toolchain/cargo/registry'] = '/tmp/registry'
        with self.assertRaises(RuntimeError):
            C.relocated_metadata({}, wrong)

    def test_metadata_refuses_overlapping_or_duplicate_mapping(self):
        wrong = self.relocations()
        wrong['/home/harmenon/ferric-asrock-42/toolchain/cargo/registry/src'] = '/home/harmenon/.cargo/src'
        with self.assertRaises(RuntimeError):
            C.relocated_metadata({}, wrong)
        wrong = self.relocations()
        wrong['/home/harmenon/ferric-asrock-42/toolchain/cargo/git'] = '/home/harmenon/.cargo/registry'
        with self.assertRaises(RuntimeError):
            C.relocated_metadata({}, wrong)

    def test_five_closed_cohorts_have_43_distinct_names(self):
        cohorts = C.cohort_contracts()
        self.assertEqual([len(row['names']) for row in cohorts.values()], [15, 6, 6, 10, 6])
        names = [name for row in cohorts.values() for name in row['names']]
        self.assertEqual(len(names), len(set(names)))
        for row in cohorts.values():
            self.assertTrue(all(name.startswith(row['prefix']) for name in row['names']))

    def test_fixture_context_derivation_preserves_actual_session_literal(self):
        self.assertEqual(C.option_fixture_metadata_observation(),
                         'd7ad89dbcf1cd370f8237f25a21c5f2fd72d0d0ea63fa1cdc5179b14efe9d324')


if __name__ == '__main__':
    unittest.main()
