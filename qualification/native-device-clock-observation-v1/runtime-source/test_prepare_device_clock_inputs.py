"""Data assembly policy only; no audit, controller import or subprocess."""
import copy
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import prepare_device_clock_inputs as P


def pin(name):
    return dict(path=str(P.E / name), bytes=1, sha256='a' * 64)


def config(mode='draft'):
    value = dict(schema='ferric-p228-device-clock-preparation-v1',
        input_label='prefix-device-clock-tf4-inputs-v228-v1',
        output_label='prefix-device-clock-tf4-shared-full-currentness-gpu-v228-v1',
        package_manifest=pin('manifest.json'), supervisor_tests=pin('pure/complete.json'),
        parent_audit=pin('parent/complete.json'), worker_audit=pin('worker/complete.json'))
    for key in ('parent_runtime_review', 'worker_runtime_review', 'decode_review'):
        value[key] = None if mode == 'draft' else pin(key + '.json')
    return value


class PreparationTests(unittest.TestCase):
    def test_closed_configuration_requires_explicit_review_mode(self):
        P.config_shape(config(), 'draft'); P.config_shape(config('finalize'), 'finalize')
        for mode, value in (('finalize', config()), ('draft', config('finalize')),
                ('draft', dict(config(), extra=True)),
                ('draft', dict(config(), output_label='../escape'))):
            with self.assertRaises(RuntimeError): P.config_shape(value, mode)

    def test_duplicate_nonfinite_and_conflicting_json_are_refused(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}'):
            with self.assertRaises(RuntimeError): P.parse(raw)
        self.assertEqual(P.parse(P.encoded({'a': 1})), {'a': 1})

    def test_runtime_template_never_auto_approves(self):
        audit = dict(host='host', boot_id='boot', binary=pin('parent'), readelf=pin('readelf'),
                     ldd=pin('ldd'), libraries=[])
        template = P.runtime_template(audit)
        self.assertIs(template['reviewed'], False)
        self.assertIs(template['production_authority'], False)
        with self.assertRaises(RuntimeError): P.reviewed(template, template)
        good = dict(template, reviewed=True, notes='Root reviewed these actual retained audit records.')
        self.assertEqual(P.reviewed(good, template), good)
        for key, invalid in (('binary', pin('other')), ('production_authority', True),
                             ('production_authority', 0), ('notes', 'TODO')):
            with self.assertRaises(RuntimeError): P.reviewed(dict(good, **{key: invalid}), template)

    def test_engineering_review_requires_actual_topics_and_exact_binding_types(self):
        template = dict(schema='review', reviewed=False, authority='none', notes='', gpu_attempts=1,
            review_topics={key: '' for key in P.TOPICS}, **{key: False for key in P.FALSE})
        good = dict(template, reviewed=True, notes='Root reviewed this exact one-attempt engineering case.',
            review_topics={key: 'Root inspected the actual selected evidence for this topic.' for key in P.TOPICS})
        P.reviewed(good, template, engineering=True)
        for key, invalid in (('gpu_attempts', True), ('reviewed', 1), ('clock_domain_validated', True),
                             ('clock_domain_validated', 0), ('review_topics', {})):
            with self.assertRaises(RuntimeError): P.reviewed(dict(good, **{key: invalid}), template, engineering=True)

    def test_derived_request_changes_only_worker_session_and_evidence(self):
        c = config(); baseline_pin = pin('baseline'); plan_pin = pin('plan')
        old_request_pin, old_review_pin = pin('request'), pin('review')
        prerequisites = dict(image_deployment=pin('image'), standalone_prepared=pin('prepared'),
            standalone_cases=[pin('case' + str(i)) for i in range(6)],
            numericals=[pin('numerical' + str(i)) for i in range(6)])
        old_plan = dict(prerequisites, request=old_request_pin, decode_review=old_review_pin)
        old_request = dict(decode=dict(worker={'path': '/old-worker'}, session=[1] * 32,
            evidence_directory='/old/native', mode='teacher_forced', model={'weights': 'unchanged'}))
        baseline = dict(prerequisites, passed=True, failures=[], plan=plan_pin,
            selected_runtime={'image': pin('v7'), 'parent': pin('old-parent'), 'worker': pin('old-worker')})
        documents = {plan_pin['path']: old_plan, old_request_pin['path']: old_request,
                     old_review_pin['path']: {'image_provenance': {'retained': 'same'}}}
        inputs = SimpleNamespace(known=lambda *_: (baseline, baseline_pin), doc=lambda record: documents[record['path']])
        def actual_audit(_inputs, record, role, _original):
            return dict(host='host', boot_id='boot', binary=pin(role), readelf=pin(role + '/readelf'),
                        ldd=pin(role + '/ldd'), libraries=[])
        before = copy.deepcopy(old_request)
        with patch.object(P, 'pure', return_value={'sources_before': pin('pure/sources-before')}), \
             patch.object(P, 'cpu', side_effect=lambda _, role: (pin(role + '/cpu'), pin(role))), \
             patch.object(P, 'audit', side_effect=actual_audit):
            plan, raw, templates, review = P.derive(inputs, c, 'draft')
        request = P.parse(raw)
        self.assertEqual(request['schema'], 'FerricFinitePrefixDecodeDeviceClockRequestV2')
        self.assertEqual(old_request, before)
        self.assertEqual({key for key in before['decode'] if request['decode'][key] != before['decode'][key]},
                         {'worker', 'session', 'evidence_directory'})
        self.assertEqual(plan['request'], P.filepin(P.E / c['input_label'] / 'request.json', raw))
        self.assertTrue(all(template['reviewed'] is False for template in templates.values()))
        self.assertIs(review['reviewed'], False)
        self.assertTrue(all(review[key] is False for key in P.FALSE))
        self.assertEqual(review['image_provenance'], {'retained': 'same'})


if __name__ == '__main__': unittest.main()
