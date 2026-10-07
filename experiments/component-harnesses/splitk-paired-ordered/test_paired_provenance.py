import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import paired_provenance as provenance


def encoded(value):
    return (json.dumps(value, sort_keys=True) + '\n').encode()


def require(value, message):
    if not value:
        raise ValueError(message)


class ProvenanceTests(unittest.TestCase):
    def fixture(self):
        clean = {'status': 0, 'reason': 'completed', 'returncode': 0,
            'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False,
            'kill_sent': False, 'log_limit_exceeded': False,
            'profile': 'FerricCpuFourCore32GiBEmitterV1', 'cpus': [0, 1, 2, 3],
            'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1}
        raw = {name: ('fixture:' + name).encode() for name in provenance.PINS}
        for role in provenance.PHASES:
            raw[role + '/status'] = b'0\n'
            raw[role + '/result'] = encoded(clean)
        for role, counts in (('default-tests', [0, 5, 6, 6, 0]), ('enabled-tests', [0, 6, 6, 6, 0])):
            raw[role + '/stdout'] = ''.join('test result: ok. ' + str(count) +
                ' passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
                for count in counts).encode()
        for name in provenance.EMPTY:
            raw[name] = b''
        observation = {'schema': 'EngineeringHsacoObservationV1', 'authority': 'none',
            'grants': {'publication': False, 'load': False, 'launch': False},
            'target': 'gfx950:xnack-', 'tools': {'emitter': 'historical'},
            'options': {'wave': 64}, 'providers': ['fixed'],
            'hsaco': {'identity': {'sha256': hashlib.sha256(raw['image.hsaco']).hexdigest(), 'byte_len': 13344},
                      'kernel_names': provenance.SYMBOLS},
            'compiler_handoff': {'sha256': hashlib.sha256(raw['compiler-handoff-v2']).hexdigest(), 'byte_len': 68207},
            'execution': {'exact_output_replay': True}}
        raw['observation.json'] = encoded(observation)
        raw['unpaired'] = encoded(copy.deepcopy(observation))
        pins = {name: hashlib.sha256(value).hexdigest() for name, value in raw.items() if name != 'unpaired'}
        evidence = {name: {'path': '/stage/paired/' + name, 'sha256': value} for name, value in pins.items()}
        plan = {'stage': '/stage', 'images': {'paired': {'sha256': pins['image.hsaco']}},
                'files': {'images/candidate.observation.json': {
                    'sha256': '2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54'}}}
        return raw, pins, evidence, plan

    def replace(self, raw, pins, evidence, name, value):
        raw[name] = encoded(value)
        pins[name] = evidence[name]['sha256'] = hashlib.sha256(raw[name]).hexdigest()

    def check(self, raw, pins, evidence, plan):
        def read(path, *_args, **_kwargs):
            name = path.removeprefix('/stage/paired/') if '/paired/' in path else 'unpaired'
            return raw[name], ''
        with patch.object(provenance, 'PINS', pins):
            provenance.validate_review(evidence, plan, read=read,
                staged_binding=lambda value, _: value, decode=json.loads, require=require)

    def test_complete_historical_evidence_accepts(self):
        self.check(*self.fixture())

    def test_missing_or_unexpected_member_rejects(self):
        for extra in (False, True):
            raw, pins, evidence, plan = self.fixture()
            if extra:
                evidence['extra'] = evidence['image.hsaco']
            else:
                del evidence['capture.json']
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)

    def test_changed_raw_hash_or_location_rejects(self):
        for change in ('raw', 'hash', 'path'):
            raw, pins, evidence, plan = self.fixture()
            if change == 'raw':
                raw['metadata.txt'] += b'changed'
            elif change == 'hash':
                evidence['metadata.txt']['sha256'] = '0' * 64
            else:
                evidence['metadata.txt']['path'] = '/other/metadata.txt'
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)

    def test_failed_or_signaled_or_relabelled_phase_rejects(self):
        for key, value in (('status', 1), ('term_sent', True), ('child_reaped', False),
                           ('profile', 'FerricCpuFourCore36GiBEmitterV1')):
            raw, pins, evidence, plan = self.fixture()
            name = 'emission/result'
            result = json.loads(raw[name])
            result[key] = value
            self.replace(raw, pins, evidence, name, result)
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)

    def test_nonzero_raw_status_rejects(self):
        raw, pins, evidence, plan = self.fixture()
        raw['isa/status'] = b'1\n'
        pins['isa/status'] = evidence['isa/status']['sha256'] = hashlib.sha256(b'1\n').hexdigest()
        with self.assertRaises(ValueError):
            self.check(raw, pins, evidence, plan)

    def test_incomplete_or_changed_test_counts_rejects(self):
        for name in ('default-tests/stdout', 'enabled-tests/stdout'):
            raw, pins, evidence, plan = self.fixture()
            raw[name] = raw[name].replace(b'6 passed', b'5 passed', 1)
            pins[name] = evidence[name]['sha256'] = hashlib.sha256(raw[name]).hexdigest()
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)

    def test_observation_authority_symbol_image_or_replay_drift_rejects(self):
        for change in ('grants', 'symbol', 'image', 'replay', 'handoff'):
            raw, pins, evidence, plan = self.fixture()
            observation = json.loads(raw['observation.json'])
            if change == 'grants':
                observation['grants']['launch'] = True
            elif change == 'symbol':
                observation['hsaco']['kernel_names'].pop()
            elif change == 'image':
                observation['hsaco']['identity']['sha256'] = '0' * 64
            elif change == 'replay':
                observation['execution']['exact_output_replay'] = False
            else:
                observation['compiler_handoff']['sha256'] = '0' * 64
            self.replace(raw, pins, evidence, 'observation.json', observation)
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)

    def test_different_compiler_providers_or_options_rejects(self):
        for key in ('tools', 'options', 'providers', 'target'):
            raw, pins, evidence, plan = self.fixture()
            unpaired = json.loads(raw['unpaired'])
            unpaired[key] = 'changed'
            raw['unpaired'] = encoded(unpaired)
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)

    def test_loaded_image_must_be_exact_paired_artifact(self):
        raw, pins, evidence, plan = self.fixture()
        plan['images']['paired']['sha256'] = '0' * 64
        with self.assertRaises(ValueError):
            self.check(raw, pins, evidence, plan)
