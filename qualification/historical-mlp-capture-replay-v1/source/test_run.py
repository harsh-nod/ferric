"""Synthetic adapter tests; the unchanged P218 suite tests actual arithmetic."""
import copy
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

import run as r


class Historical:
    @staticmethod
    def rust_pin(value):
        if type(value['bytes']) is not int:
            raise ValueError('extent')
        words = value['sha256']
        if len(words) != 32 or any(type(x) is not int or not 0 <= x <= 255 for x in words):
            raise ValueError('digest')
        return dict(value, sha256=bytes(words).hex())

    @staticmethod
    def digest(value):
        return hashlib.sha256(value).hexdigest()


def fixture():
    def pin(name, size, marker):
        return dict(path='/original/' + name, bytes=size, sha256=('%02x' % marker) * 32)
    return dict(post_norm=pin('norm', 8192, 1), ranks=[dict(rank=rank,
        **{role: pin(f'{role}{rank}', 50331648, 2 + rank * 3 + index)
           for index, role in enumerate(('gate', 'up', 'down'))}) for rank in range(2)])


def source_records(manifest):
    registration, uploads = dict(layers=[]), dict(version=1, uploads=[])
    for rank in range(2):
        weights = []
        for index, role in enumerate(r.ROLES):
            record = manifest['post_norm'] if role == 'post_norm' else manifest['ranks'][rank][role]
            source = 100 + 10 * rank + index
            weights.append(dict(kind=r.KINDS[role], buffer=dict(rank=rank, id=source,
                elements=record['bytes'] // 2, element_bytes=2)))
            uploads['uploads'].append(dict(key=dict(kind='source', rank=rank, id=source),
                bytes=record['bytes'], sha256=list(bytes.fromhex(record['sha256']))))
        registration['layers'].append(dict(rank=rank, layer=0, weights=weights))
    return registration, uploads


def capture():
    return {field: [0] * count for _, field, count, _ in r.STAGES}


class Reference:
    def __init__(self, fail=None):
        self.calls, self.fail = [], fail

    def words(self, values, count, bits=16):
        mask = 0x7f80 if bits == 16 else 0x7f800000
        if type(values) is not list or len(values) != count or any(
                type(x) is not int or not 0 <= x < 1 << bits or x & mask == mask for x in values):
            raise ValueError('finite exact words')
        return values

    def note(self, name, *inputs):
        self.calls.append((name, inputs))
        if name == self.fail:
            raise ValueError('independent ' + name + ' failure')
        return dict(mismatches=0)

    def check_norm(self, inputs, weight, observed):
        return self.note('norm', inputs, weight, observed)

    def check_bf16_gemv(self, weight, inputs, observed):
        return self.note(weight, weight, inputs, observed)

    def check_swiglu(self, gate, up, observed):
        return self.note('swiglu', gate, up, observed)

    def check_f32_gemv(self, weight, inputs, observed):
        return self.note('down', weight, inputs, observed)


class AdapterTests(unittest.TestCase):
    def setUp(self):
        self.manifest = fixture()
        self.registration, self.uploads = source_records(self.manifest)
        self.weights = dict(gate='gate', up='up', down='down')

    def test_actual_upload_roles_and_both_rank_joins(self):
        for rank in range(2):
            result = r.uploaded_weights(self.registration, self.uploads, self.manifest, rank, Historical)
            self.assertEqual(set(result), set(r.ROLES))
            self.assertTrue(all(row['key']['rank'] == rank for row in result.values()))

    def test_wrong_layer_rank_and_boolean_rank_refused(self):
        for field, value in (('layer', 1), ('rank', 1), ('rank', False), ('layer', False)):
            registration = copy.deepcopy(self.registration)
            registration['layers'][0][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                r.uploaded_weights(registration, self.uploads, self.manifest, 0, Historical)
        with self.assertRaises(ValueError):
            r.uploaded_weights(self.registration, self.uploads, self.manifest, True, Historical)

    def test_duplicate_missing_registered_role_and_alias_refused(self):
        for action in ('duplicate', 'missing', 'alias'):
            registration = copy.deepcopy(self.registration)
            weights = registration['layers'][0]['weights']
            if action == 'duplicate':
                weights.append(copy.deepcopy(weights[0]))
            elif action == 'missing':
                weights.pop()
            else:
                weights[1]['buffer']['id'] = weights[0]['buffer']['id']
            with self.subTest(action=action), self.assertRaises(ValueError):
                r.uploaded_weights(registration, self.uploads, self.manifest, 0, Historical)

    def test_duplicate_missing_and_wrong_kind_upload_refused(self):
        for action in ('duplicate', 'missing', 'kind'):
            uploads = copy.deepcopy(self.uploads)
            if action == 'duplicate':
                uploads['uploads'].append(copy.deepcopy(uploads['uploads'][0]))
            elif action == 'missing':
                uploads['uploads'].pop(0)
            else:
                uploads['uploads'][0]['key']['kind'] = 'scratch'
            with self.subTest(action=action), self.assertRaises(ValueError):
                r.uploaded_weights(self.registration, uploads, self.manifest, 0, Historical)

    def test_wrong_weight_bytes_digest_and_boolean_source_refused(self):
        for action in ('bytes', 'digest', 'rank', 'source'):
            uploads = copy.deepcopy(self.uploads)
            item = uploads['uploads'][0]
            if action == 'bytes': item['bytes'] += 2
            elif action == 'digest': item['sha256'][0] ^= 1
            elif action == 'rank': item['key']['rank'] = False
            else: item['key']['id'] = True
            with self.subTest(action=action), self.assertRaises(ValueError):
                r.uploaded_weights(self.registration, uploads, self.manifest, 0, Historical)

    def test_swapped_gate_up_or_rank_weights_refused(self):
        for action in ('gate-up', 'ranks'):
            manifest = copy.deepcopy(self.manifest)
            if action == 'gate-up':
                row = manifest['ranks'][0]
                row['gate'], row['up'] = row['up'], row['gate']
            else:
                manifest['ranks'].reverse()
            with self.subTest(action=action), self.assertRaises(ValueError):
                r.uploaded_weights(self.registration, self.uploads, manifest, 0, Historical)

    def test_registration_element_width_extent_and_boolean_refused(self):
        for field, value in (('element_bytes', 4), ('elements', 4095), ('rank', True), ('id', True)):
            registration = copy.deepcopy(self.registration)
            registration['layers'][0]['weights'][0]['buffer'][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                r.uploaded_weights(registration, self.uploads, self.manifest, 0, Historical)

    def test_selected_mlp548_not_legacy_image(self):
        image = dict(path='/original/tiles.hsaco', bytes=r.MLP_IMAGE[0],
                     sha256=list(bytes.fromhex(r.MLP_IMAGE[1])))
        self.assertEqual(r.selected_image(dict(mlp_tiles_image=image), Historical)['sha256'], r.MLP_IMAGE[1])
        for field, value in (('bytes', 29656), ('sha256', [0] * 32)):
            bad = dict(image, **{field: value})
            with self.subTest(field=field), self.assertRaises(ValueError):
                r.selected_image(dict(mlp_tiles_image=bad), Historical)

    def test_all_five_checks_use_actual_immediate_predecessors(self):
        raw, reference, norm_weight = capture(), Reference(), object()
        stages = r.independent_stages(reference, self.weights, norm_weight, raw)
        self.assertEqual(list(stages), ['norm', 'gate', 'up', 'swiglu', 'down'])
        calls = reference.calls
        self.assertIs(calls[0][1][0], raw['input_words'])
        self.assertIs(calls[0][1][1], norm_weight)
        self.assertIs(calls[0][1][2], raw['post_norm_words'])
        self.assertIs(calls[1][1][1], raw['post_norm_words'])
        self.assertIs(calls[1][1][2], raw['gate_words'])
        self.assertIs(calls[2][1][1], raw['post_norm_words'])
        self.assertIs(calls[2][1][2], raw['up_words'])
        self.assertIs(calls[3][1][0], raw['gate_words'])
        self.assertIs(calls[3][1][1], raw['up_words'])
        self.assertIs(calls[3][1][2], raw['activation_words'])
        self.assertIs(calls[4][1][1], raw['activation_words'])
        self.assertIs(calls[4][1][2], raw['down_partial_bits'])

    def test_each_input_mutation_reaches_the_frozen_checker(self):
        locations = dict(input_words=(0, 0), post_norm_words=(0, 2), gate_words=(1, 2),
                         up_words=(2, 2), activation_words=(3, 2), down_partial_bits=(4, 2))
        for field, (call, slot) in locations.items():
            raw, reference = capture(), Reference()
            raw[field][17] = 0x3f800000 if field == 'down_partial_bits' else 0x3f80
            r.independent_stages(reference, self.weights, object(), raw)
            self.assertEqual(reference.calls[call][1][slot][17], raw[field][17])

    def test_each_stage_failure_stops_instead_of_falling_back(self):
        for index, stage in enumerate(('norm', 'gate', 'up', 'swiglu', 'down')):
            reference = Reference(stage)
            with self.subTest(stage=stage), self.assertRaisesRegex(ValueError, 'independent'):
                r.independent_stages(reference, self.weights, object(), capture())
            self.assertEqual(len(reference.calls), index + 1)

    def test_nonfinite_boolean_truncated_and_extra_words_refused(self):
        for _, field, _, width in r.STAGES:
            for action in ('nonfinite', 'boolean', 'short', 'extra'):
                raw = capture()
                if action == 'nonfinite': raw[field][0] = 0x7f800000 if width == 4 else 0x7f80
                elif action == 'boolean': raw[field][0] = False
                elif action == 'short': raw[field].pop()
                else: raw[field].append(0)
                with self.subTest(field=field, action=action), self.assertRaises(ValueError):
                    r.independent_stages(Reference(), self.weights, object(), raw)

    def test_four_rank_profile_rows_and_twenty_checks(self):
        captures = {label: [capture(), capture()] for label in r.LABELS}
        reference = Reference()
        result = r.compare_profiles(captures, {x: self.registration for x in r.LABELS},
            {x: self.uploads for x in r.LABELS}, self.manifest, reference, object(),
            [self.weights, self.weights], Historical)
        self.assertEqual([row['profile'] for row in result], list(r.LABELS))
        self.assertEqual([[x['rank'] for x in row['ranks']] for row in result], [[0, 1], [0, 1]])
        self.assertEqual(len(reference.calls), 20)

    def test_missing_profile_rank_and_late_bad_upload_refused(self):
        for action in ('profile', 'rank', 'late-upload'):
            captures = {label: [capture(), capture()] for label in r.LABELS}
            uploads = {label: copy.deepcopy(self.uploads) for label in r.LABELS}
            if action == 'profile': captures.pop('candidate')
            elif action == 'rank': captures['candidate'].pop()
            else: uploads['candidate']['uploads'][-1]['sha256'][0] ^= 1
            with self.subTest(action=action), self.assertRaises(ValueError):
                r.compare_profiles(captures, {x: self.registration for x in r.LABELS}, uploads,
                    self.manifest, Reference(), object(), [self.weights, self.weights], Historical)

    def test_capture_layout_uses_supplied_authenticated_spans(self):
        spans, body = [], bytearray()
        for rank in range(2):
            for index in range(14):
                if index < len(r.STAGES):
                    name, _, count, width = r.STAGES[index]
                    word = 0x3f800000 + rank if width == 4 else 0x3f80 + rank
                    raw = struct.pack('<' + ('I' if width == 4 else 'H'), word) * count
                else:
                    name, width, raw = f'unused{index}', 2, b'\0\0'
                spans.append(dict(rank=rank, stage=name, bytes=len(raw), element_bytes=width,
                                  offset=len(body), sha256=Historical.digest(raw)))
                body.extend(raw)
        history = types.SimpleNamespace(digest=Historical.digest,
            residual_rows=lambda raw, validator, record: ({}, spans))
        rows, selected = r.capture_inputs(bytes(body), None, history, None)
        self.assertEqual(len(selected), 12)
        self.assertEqual(rows[1]['post_norm_words'], [0x3f81] * 4096)
        self.assertEqual(rows[1]['down_partial_bits'], [0x3f800001] * 4096)
        with self.assertRaises(ValueError):
            r.capture_inputs(bytes(body[:-20]), None, history, None)
        spans[0]['element_bytes'] = 4
        with self.assertRaises(ValueError):
            r.capture_inputs(bytes(body), None, history, None)

    def test_bootstrap_hash_and_symlink_refusals(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'helper.py'
            raw = b'VALUE = 7\n'
            path.write_bytes(raw)
            pin = (len(raw), Historical.digest(raw))
            self.assertEqual(r.bootstrap(path, pin).VALUE, 7)
            with self.assertRaises(ValueError):
                r.bootstrap(path, (len(raw), '0' * 64))
            link = Path(directory) / 'link.py'
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                r.bootstrap(link, pin)

    def test_reference_import_restores_existing_extract_on_failure(self):
        class Reader:
            def read(self, *args): return b''
        directory = Path('/pinned')
        original, dependency = object(), object()
        def module(reader, path, pin, name):
            if path.name == 'extract.py': return dependency
            self.assertIs(sys.modules['extract'], dependency)
            raise ValueError('reference import failed')
        history = types.SimpleNamespace(canonical=lambda value: directory, module=module)
        with patch.dict(sys.modules, {'extract': original}):
            with self.assertRaisesRegex(ValueError, 'reference import failed'):
                r.load_reference(Reader(), history, directory)
            self.assertIs(sys.modules['extract'], original)

    def test_reference_import_rejects_other_numpy_and_restores_absence(self):
        class Reader:
            def read(self, *args): return b''
        def module(reader, path, pin, name):
            return types.SimpleNamespace(np=types.SimpleNamespace(__version__='2.3.0'))
        history = types.SimpleNamespace(canonical=lambda value: Path('/pinned'), module=module)
        with patch.dict(sys.modules):
            sys.modules.pop('extract', None)
            with self.assertRaisesRegex(ValueError, 'NumPy'):
                r.load_reference(Reader(), history, '/pinned')
            self.assertNotIn('extract', sys.modules)

    def test_fixture_loader_reuses_frozen_reader_and_records_all_payloads(self):
        manifest = copy.deepcopy(self.manifest)
        manifest['numerical_policy'] = dict(path=str(Path(r.FIXTURE['path']).parent / 'policy.json'),
            bytes=r.HELPERS['policy.json'][0], sha256=r.HELPERS['policy.json'][1])
        norm, ranks, calls = object(), object(), []
        class Reader:
            def read(self, path, expected, original=None):
                calls.append((str(path), expected, original))
                return json.dumps(manifest).encode() if Path(path).name == 'manifest.json' else b'policy'
        def load(path, sha):
            self.assertEqual(str(path), r.FIXTURE['path'])
            self.assertEqual(sha, r.FIXTURE['sha256'])
            return manifest, norm, ranks
        reference = types.SimpleNamespace(json_bytes=json.loads, fixture=load)
        historical = types.SimpleNamespace(canonical=Path)
        result = r.load_fixture(Reader(), historical, reference, Path(r.FIXTURE['path']).parent,
                                Path('/helper/policy.json'))
        self.assertIs(result[1], norm)
        self.assertIs(result[2], ranks)
        self.assertEqual(len(calls), 10)  # Manifest, both policies, seven payloads.
        self.assertEqual({call[0] for call in calls[3:]},
            {manifest['post_norm']['path']} | {row[role]['path'] for row in manifest['ranks']
                                             for role in ('gate', 'up', 'down')})
        with self.assertRaisesRegex(ValueError, 'original retained fixture'):
            r.load_fixture(Reader(), historical, reference, '/different', '/helper/policy.json')

    def test_fixture_policy_difference_and_frozen_loader_failure_propagate(self):
        manifest = copy.deepcopy(self.manifest)
        manifest['numerical_policy'] = dict(path='/fixture/policy.json', bytes=r.HELPERS['policy.json'][0],
                                          sha256=r.HELPERS['policy.json'][1])
        class Reader:
            def read(self, path, expected, original=None):
                if Path(path).name == 'manifest.json': return json.dumps(manifest).encode()
                return b'original' if str(path) == '/fixture/policy.json' else b'different'
        reference = types.SimpleNamespace(json_bytes=json.loads, fixture=lambda *args: None)
        historical = types.SimpleNamespace(canonical=Path)
        with self.assertRaisesRegex(ValueError, 'policy byte equality'):
            r.load_fixture(Reader(), historical, reference, Path(r.FIXTURE['path']).parent,
                           Path('/helper/policy.json'))
        def fail(*args): raise ValueError('frozen shape axis or digest failure')
        reference.fixture = fail
        with self.assertRaisesRegex(ValueError, 'frozen shape'):
            r.load_fixture(Reader(), historical, reference, Path(r.FIXTURE['path']).parent,
                           Path('/fixture/policy.json'))

    def test_replay_nonclaims_and_final_recheck_are_not_skipped(self):
        image = dict(path='/original/tiles.hsaco', bytes=r.MLP_IMAGE[0],
                     sha256=list(bytes.fromhex(r.MLP_IMAGE[1])))
        request = dict(mlp_tiles_image=image, expected_model_id='model', expected_bundle_id='bundle',
                       source='/original/model')
        registration = dict(self.registration, model_id='model', bundle_id='bundle')
        checked = dict(layer=0, position=0, token=9112)
        blobs = {'request': json.dumps(request).encode(), 'summary.json': b'summary'}
        for label in r.LABELS:
            blobs[label + '-capture.bin'] = b'capture'
            blobs[label + '-bootstrap.json'] = b'{"input":{}}'
            blobs[label + '-registration.json'] = json.dumps(registration).encode()
            blobs[label + '-uploads.json'] = json.dumps(self.uploads).encode()
        body = set(blobs) - {'request', 'summary.json'}
        receipt = dict(schema='ferric-p227-prefix-layer-replay-v1', passed=True,
            original_status='FAILED_UNCHANGED', new_native_attempts=0, checked=checked,
            request=dict(path='request'), retained_native={name: dict(path=name)
                for name in body | {'summary.json'}})
        blobs['receipt'] = json.dumps(receipt).encode()
        class Reader:
            rechecked = False
            def __init__(self, root): self.records = {}
            def read(self, *args): return b''
            def original(self, pin): return blobs[pin['path']]
            def recheck(self): Reader.rechecked = True
        validator = types.SimpleNamespace(parse=json.loads, BODY=body,
                                         validate=lambda *args: checked)
        historical = types.SimpleNamespace(Reader=Reader, canonical=Path, VALIDATOR=(1, 'x'),
            module=lambda *args: validator, REPLAY=dict(path='receipt'), rust_pin=Historical.rust_pin,
            CAPTURE_BYTES=7, CAPTURE_SHA=Historical.digest(b'capture'), digest=Historical.digest)
        manifest = dict(self.manifest, **{key: {} for key in
            ('source', 'index', 'source_inventory', 'extractor', 'numerical_policy')})
        with patch.object(r, 'bootstrap', return_value=historical), \
             patch.object(r, 'load_reference', return_value=Reference()), \
             patch.object(r, 'load_fixture', return_value=(manifest, object(), [self.weights] * 2)), \
             patch.object(r, 'capture_inputs', return_value=([capture(), capture()], [])):
            value = r.replay('/e', '/helper', '/validator', '/reference', '/fixture', '/image')
            self.assertTrue(Reader.rechecked)
            self.assertTrue(value['conditional_stage_checks_passed'])
            self.assertTrue(value['conditional_on_actual_stage_inputs'])
            self.assertTrue(value['historical_v227_image'])
            self.assertEqual(value['authority'], 'none')
            self.assertEqual(value['original_status'], 'FAILED_UNCHANGED')
            for name in ('new_v7_image_checked', 'gpu_execution_requested', 'original_model_shard_rehashed',
                         'adaptive_tolerance', 'residual_arithmetic_rechecked', 'full_layer_acceptance',
                         'full_model_acceptance', 'runtime_premises_discharged',
                         'arithmetic_prerequisites_verified', 'production_authority', 'performance_claim'):
                self.assertIs(value[name], False, name)
            with patch.object(Reader, 'recheck', side_effect=ValueError('changed retained input')):
                with self.assertRaisesRegex(ValueError, 'changed retained input'):
                    r.replay('/e', '/helper', '/validator', '/reference', '/fixture', '/image')

    def test_python_optimization_environment_and_flag_refused(self):
        with patch.dict(os.environ, {'PYTHONOPTIMIZE': '0'}):
            with self.assertRaises(ValueError): r.ordinary_python()
        with patch.dict(os.environ):
            os.environ.pop('PYTHONOPTIMIZE', None)
            with patch.object(r.sys, 'flags', types.SimpleNamespace(optimize=1)):
                with self.assertRaises(ValueError): r.ordinary_python()


if __name__ == '__main__':
    unittest.main()
