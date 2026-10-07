"""Synthetic data-only renderer contracts; no measured values or native execution."""
import copy
import csv
import io
import unittest
import xml.etree.ElementTree as ET

import render as R


def pin(name, digest='a', size=1):
    return dict(path='/fixture/' + name, bytes=size, sha256=digest * 64)


def fixture():
    analysis = dict(schema='ferric-hidden-pair-read-abba-analysis-v1', run_order=list(R.ORDER),
        independent_runs_per_mode=2, per_forward_observations_are_not_independent_runs=True,
        supervisor_manifest_sha256='a' * 64, supervisor_controller=pin('serial.py'),
        runs={}, chronology=[], **{key: False for key in R.FALSE_FLAGS})
    terminals, hosts = {}, {}
    for n, name in enumerate(R.ORDER):
        paired = name.startswith('paired-')
        case = dict(worker=[1, 'a' * 64], parent=[1, 'a' * 64], immutable_request={'model': 'same'},
            platform={'boot_id': 'same'}, input_tokens=[11, 12, 13, 14], payloads=['b' * 64] * 4,
            session=[n + 1] * 32, child_pid=100 + n, forwards=[])
        host = dict(schema='ferric-guarded-mlp-peer-read-pair-host-checked-v2', paired_read=paired,
            counter_names=R.COUNTERS[:], shared_counter_names=R.SHARED[:], snapshots=587, intervals=586,
            same_run_completions_joined=True, native_close_confirmed=True, shared_full_currentness=True,
            inclusive_nested_host_scopes=True, forward_rows=[], gpu_time=False, gpu_overlap=False,
            throughput=False, numerical_acceptance=False, full_model_acceptance=False,
            performance_claim=False, production_authority=False)
        for position in range(4):
            # Closed-form expectation: sum(layer+1, 1..36) = 666.
            factor = (n + 1) * (position + 1)
            layers = []
            for layer in range(36):
                rank = [0] * 19; rank[2] = 0 if paired else 2; rank[13:15] = [1, 8192]
                layers.append(dict(layer=layer, hidden_read=dict(host_elapsed_ns=factor * (layer + 1),
                    ranks=[rank[:], rank[:]], shared=[2 if paired else 4, 0, 0, 0])))
            case['forwards'].append(dict(position=position, hidden_read_ns=factor * 666,
                forward_host_ns=100000, bracket_host_ns=100000))
            host['forward_rows'].append(dict(position=position, forward_host_ns=100000,
                bracket_host_ns=100000, layers=layers))
        case['hidden_read_total_ns'] = (n + 1) * 6660
        phase = dict(exit_code=0, reason=None, cleanup_signalled=False,
            owned_groups_absent=True, owned_processes_reaped=True)
        terminal = dict(schema='ferric-guarded-mlp-peer-read-pair-gpu-v2', passed=True, errors=[], case=name,
            paired_read_requested=paired, native_attempts=1, retries=0, gpu_execution_confirmed=True,
            host_observation_verified=True, shared_full_currentness_verified=True, host_observation=host,
            performance_claim=False, production_authority=False, full_model_acceptance=False,
            numerical_acceptance=False, full_long_workload=False,
            instrumentation_comparison=dict(all_payloads_equal=True, all_histories_equal=True),
            phases=[phase.copy() for _ in range(11)], admission=dict(worker=pin('worker'), parent=pin('parent')),
            platform=case['platform'], observation=dict(input_tokens=case['input_tokens'], child_pid=case['child_pid']),
            raw={'native/observation-%d.bin' % i: pin('payload%d' % i, 'b', 606976) for i in range(4)})
        analysis['chronology'].append(dict(name=name, terminal=pin(name + '/complete.json'),
            started_monotonic_ns=n * 10, finished_monotonic_ns=n * 10 + 5))
        analysis['runs'][name] = case; terminals[name] = terminal; hosts[name] = host
    analysis.update(control_mean_hidden_read_ns=16650, paired_mean_hidden_read_ns=16650,
        observed_hidden_read_reduction_fraction=0.0, observed_hidden_read_ratio=1.0)
    return analysis, terminals, hosts


class RenderTests(unittest.TestCase):
    def test_exact_full_census_and_integer_forward_totals(self):
        data = R.collect(*fixture())
        self.assertEqual(len(data['layers']), 576)
        self.assertEqual(len(data['forwards']), 16)
        self.assertEqual([r['host_hidden_read_total_ns'] for r in data['runs']], [6660, 13320, 19980, 26640])
        self.assertEqual([r['host_hidden_read_ns'] for r in data['forwards'][:4]], [666, 1332, 1998, 2664])
        self.assertEqual(data['layers'][-1]['host_hidden_read_ns'], 4 * 4 * 36)

    def test_incomplete_order_and_independence_refuse(self):
        for choice in range(8):
            a, t, h = fixture()
            if choice == 0: a['run_order'].reverse()
            elif choice == 1: a['runs'].pop('paired-1')
            elif choice == 2: a['chronology'][1]['started_monotonic_ns'] = 0
            elif choice == 3: a['runs']['paired-0']['session'] = a['runs']['control-0']['session']
            elif choice == 4: a['independent_runs_per_mode'] = True
            elif choice == 5: a['per_forward_observations_are_not_independent_runs'] = False
            elif choice == 6: del a['observed_hidden_read_ratio']
            else: a['observed_hidden_read_ratio'] = 2.0
            with self.assertRaises(ValueError): R.collect(a, t, h)

    def test_wrong_mode_counts_or_hidden_effects_refuse(self):
        for choice in range(8):
            a, t, h = fixture(); hidden = h['paired-0']['forward_rows'][0]['layers'][0]['hidden_read']
            if choice == 0: h['paired-0']['paired_read'] = False
            elif choice == 1: hidden['shared'][0] = 4
            elif choice == 2: hidden['ranks'][0][2] = 2
            elif choice == 3: hidden['ranks'][0][13] = 2
            elif choice == 4: hidden['ranks'][1][14] = 8191
            elif choice == 5: hidden['ranks'][0][8] = 1
            elif choice == 6: hidden['ranks'][0][4] = 1
            else: hidden['shared'][2] = 1
            with self.assertRaises(ValueError): R.collect(a, t, h)

    def test_bad_wall_order_overflow_and_drift_refuse(self):
        for choice in range(7):
            a, t, h = fixture(); layer = h['control-0']['forward_rows'][0]['layers'][0]
            if choice == 0: layer['hidden_read']['host_elapsed_ns'] = True
            elif choice == 1: layer['hidden_read']['host_elapsed_ns'] = -1
            elif choice == 2: layer['hidden_read']['host_elapsed_ns'] = 1 << 64
            elif choice == 3: layer['hidden_read']['host_elapsed_ns'] = (1 << 64) - 1
            elif choice == 4: layer['layer'] = 1
            elif choice == 5: a['runs']['control-0']['forwards'][0]['hidden_read_ns'] += 1
            else: h['control-0']['forward_rows'][0]['layers'].pop()
            with self.assertRaises(ValueError): R.collect(a, t, h)

    def test_failure_payload_identity_and_claims_refuse(self):
        for choice in range(8):
            a, t, h = fixture(); terminal = t['paired-1']
            if choice == 0: terminal['passed'] = False
            elif choice == 1: terminal['phases'][0]['cleanup_signalled'] = True
            elif choice == 2: terminal['instrumentation_comparison']['all_payloads_equal'] = False
            elif choice == 3: terminal['raw']['native/observation-0.bin']['sha256'] = 'c' * 64
            elif choice == 4: terminal['admission']['worker']['bytes'] = 2
            elif choice == 5: a['end_to_end_speedup'] = True
            elif choice == 6: h['paired-1']['gpu_time'] = True
            else: terminal['native_attempts'] = True
            with self.assertRaises(ValueError): R.collect(a, t, h)

    def test_csv_plot_and_caption_keep_run_and_forward_units(self):
        data = R.collect(*fixture())
        csv_rows = list(csv.DictReader(io.StringIO(R.csv_bytes(data['forwards']).decode())))
        self.assertEqual(len(csv_rows), 16)
        self.assertEqual(csv_rows[-1], dict(run='control-1', position='3', host_hidden_read_ns='10656'))
        xml = ET.fromstring(R.plot(data, 'a' * 64))
        self.assertEqual((xml.get('width'), xml.get('height')), ('1180', '720'))
        bars = [e for e in xml.findall('{%s}rect' % R.SVG) if e.get('width') == '104']
        self.assertEqual(len(bars), 16)
        self.assertTrue(all(float(e.get('height')) > 0 for e in bars))
        labels = ''.join(xml.itertext())
        self.assertIn('two independent runs per mode', labels.lower())
        self.assertIn('four colored forward observations within each run are correlated', labels.lower())
        self.assertIn('Analysis SHA-256: ' + 'a' * 64, labels)
        note = R.markdown(data, 'a' * 64).decode()
        self.assertIn('| control | 288 | 4 | 2 | 1 | 8192 |', note)
        self.assertIn('| paired | 288 | 2 | 0 | 1 | 8192 |', note)
        self.assertIn('not four independent samples', note)

    def test_duplicate_fields_strict_integers_and_decimal_rounding(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}'):
            with self.assertRaises(ValueError): R.parse(raw)
        for value in (True, -1, 1.0, 1 << 64):
            with self.assertRaises(ValueError): R.uint(value)
        self.assertEqual(R.decimal_ns(499), '0.000')
        self.assertEqual(R.decimal_ns(500), '0.001')
        self.assertEqual(R.decimal_ns(1_234_567), '1.235')
        with self.assertRaises(ValueError): R.total([(1 << 64) - 1, 1])


if __name__ == '__main__':
    unittest.main(verbosity=2)
