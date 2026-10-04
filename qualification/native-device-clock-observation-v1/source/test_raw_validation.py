"""Synthetic raw records only; no native execution or numerical qualification."""
import copy
import hashlib
import json
import struct
from types import SimpleNamespace
import unittest

import raw_validation as D


def encoded(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False).encode()


def fixture():
    digest = lambda n: [n] * 32
    part = lambda n: dict(bytes=32, sha256=digest(n))
    bootstrap = dict(prefix_image=part(1), tiles_image=part(2),
        begin=dict(residual_image=part(3), tail_image=part(4)))
    request = dict(worker=dict(path='/worker', bytes=32, sha256=digest(5)),
                   device_ids=[11, 12], evidence_directory='/case/native')
    observed = dict(child_pid=10, bootstrap=bootstrap, request=request,
        profile_sha256=digest(6), transcript_sha256=digest(7),
        files=dict(frames=[], bytes_before_summary=100))
    rows, packets, files = [], [0, 0], {}
    for position in range(4):
        raw = bytearray(241960); offset = 0
        times = [position * 1000 + i for i in range(D.PER_FORWARD)]
        struct.pack_into('<2Q', raw, 0, *times[:2]); offset = 16
        for layer in range(36):
            offset += 2 * (284 + 548) * 4
            struct.pack_into('<8Q', raw, offset, *times[2 + layer * 8:10 + layer * 8]); offset += 64
        struct.pack_into('<3Q', raw, offset, *times[-3:])
        files[f'control-{position}.bin'] = bytes(raw)
        completed = dict(generation=position + 1, position=position)
        observed['files']['frames'].append(dict(response=dict(event=dict(status='completed', **completed))))
        for slot, elapsed in enumerate(times):
            generation, pos, stage, layer, rank = D.expected(position * D.PER_FORWARD + slot)
            image = 3 if stage.endswith('-residual') or stage == 'copy' else 1 if stage == 'prefix' else 2 if stage == 'mlp' else 4
            rows.append(dict(generation=generation, position=pos, stage=stage, layer=layer, rank=rank,
                entry=D.ENTRIES[stage], image_sha256=digest(image), group_incarnation=8,
                unique_id=[11, 12][rank], queue_epoch=rank + 20,
                packet_id=packets[rank], signal_generation=packets[rank] + 1,
                start_tick=100 + slot, end_tick=101 + slot, host_elapsed_ns=elapsed))
            packets[rank] += 1
    value = dict(schema='FerricPrefixDecodeDeviceObservationV1', bootstrap=bootstrap,
        worker_sha256=digest(5), child_pid=10, profile_sha256=digest(6), group_incarnation=8,
        ranks=[dict(rank=r, unique_id=11 + r, queue_epoch=20 + r) for r in range(2)],
        images=dict(prefix=digest(1), mlp=digest(2), residual=digest(3), tail=digest(4), copy=digest(3)),
        rows=rows, final_dispatches=[592, 580],
        completions=[dict(f['response']['event']) for f in observed['files']['frames']],
        transcript_sha256=digest(7), raw_timestamp_queue=True, shared_full_currentness=True,
        cache_kernel_admission=False, operational_currentness=False, native_closed=True,
        raw_completion_ticks=True, **{k: False for k in D.FALSE})
    for completion in value['completions']: completion.pop('status')
    # This mock isolates adapter routing. Production uses the frozen validator,
    # which separately checks every typed terminal word before timing extraction.
    validator = SimpleNamespace(control=lambda raw: 144 if len(raw) == 241960 else 0)
    return value, observed, files, validator


class DeviceValidationTests(unittest.TestCase):
    def invariance_fixture(self):
        old_request = dict(worker={'path': '/old'}, session=[1], evidence_directory='/old/native',
            mode='teacher_forced', device_ids=[11, 12], immutable={'sha256': 'a' * 64})
        new_request = dict(old_request, worker={'path': '/new'}, session=[2], evidence_directory='/new/native')
        records = [dict(generation=i + 1, position=i, input_token=i, output_token=i + 10) for i in range(4)]
        old = dict(request=old_request, records=copy.deepcopy(records))
        new = dict(request=new_request, records=copy.deepcopy(records))
        files = {}
        for i in range(4):
            files[f'request-{i}.json'] = encoded(dict(id=i + 1, protocol='finite', device_ids=[11, 12],
                command=dict(cache_metadata=[i, 3, 2], rotary_bits=[0, 1], token=i)))
            files[f'observation-{i}.bin'] = bytes([i]) * 606976
        rows = [dict(same_input_history=True, tensors=[dict(byte_equal=True) for _ in range(38)]) for _ in range(4)]
        # Mock only the already-frozen numerical diagnostic layer. Payload and
        # request invariance checks below still execute the actual adapter code.
        C = SimpleNamespace(document=json.loads, records=lambda value: value['records'],
            compare_rows=lambda *args: copy.deepcopy(rows))
        return C, dict(observed=old, files=files), new, dict(files), {'diagnostics': object()}, rows

    def test_all1172_ordered_packets_and_control_slots(self):
        value, observed, files, validator = fixture()
        self.assertEqual(D.report(encoded(value), observed, files, validator), value)
        self.assertEqual([sum(row['rank'] == r for row in value['rows']) for r in range(2)], [592, 580])
        self.assertEqual(D.expected(0), (1, 0, 'embedding', None, 0))
        self.assertEqual(D.expected(289), (1, 0, 'post-mlp-residual', 35, 1))
        self.assertEqual(D.expected(1171), (4, 3, 'argmax', None, 0))

    def test_packet_index_types_and_boundaries(self):
        for index in (-1, 1172, True, 0.0):
            with self.assertRaises(RuntimeError): D.expected(index)

    def test_every_control_field_joins_without_using_ticks_as_nanoseconds(self):
        value, observed, files, validator = fixture()
        for index in (0, 1, 2, 3, 8, 9, 288, 289, 290, 291, 292, 293, 1171):
            changed = copy.deepcopy(value); changed['rows'][index]['host_elapsed_ns'] += 1
            with self.subTest(index=index), self.assertRaises(RuntimeError):
                D.report(encoded(changed), observed, files, validator)
        changed = copy.deepcopy(value)
        for row in changed['rows']: row['start_tick'] += 10000; row['end_tick'] += 10000
        D.report(encoded(changed), observed, files, validator)

    def test_control_validation_is_mandatory_and_truncated_input_refused(self):
        value, observed, files, validator = fixture()
        with self.assertRaises(RuntimeError):
            D.report(encoded(value), observed, files, SimpleNamespace(control=lambda raw: 0))
        files['control-2.bin'] = files['control-2.bin'][:-1]
        with self.assertRaises(RuntimeError): D.report(encoded(value), observed, files, validator)

    def test_first_middle_last_packet_identity_failures(self):
        value, observed, files, validator = fixture()
        for index in (0, 585, 1171):
            for key in ('packet_id', 'signal_generation', 'group_incarnation', 'unique_id', 'queue_epoch', 'rank',
                        'generation', 'position', 'start_tick', 'end_tick'):
                changed = copy.deepcopy(value)
                changed['rows'][index][key] = 0 if key in ('start_tick', 'end_tick') else changed['rows'][index][key] + 1
                with self.subTest(index=index, key=key), self.assertRaises(RuntimeError):
                    D.report(encoded(changed), observed, files, validator)

    def test_typed_uints_reject_boolean_or_float_aliases(self):
        value, observed, files, validator = fixture()
        for key, index in (('generation', 0), ('position', 0), ('rank', 0), ('layer', 2),
                           ('packet_id', 0), ('queue_epoch', 0), ('host_elapsed_ns', 0)):
            for number in (False, 0.0):
                changed = copy.deepcopy(value); changed['rows'][index][key] = number
                with self.subTest(key=key, value=number), self.assertRaises(RuntimeError):
                    D.report(encoded(changed), observed, files, validator)

    def test_stage_entry_image_and_layer_corruption(self):
        value, observed, files, validator = fixture()
        for index, key, replacement in ((0, 'entry', 'other'), (0, 'layer', 0), (2, 'layer', None),
                                        (2, 'stage', 'mlp'), (292, 'image_sha256', [1] * 32)):
            changed = copy.deepcopy(value); changed['rows'][index][key] = replacement
            with self.subTest(index=index, key=key), self.assertRaises(RuntimeError):
                D.report(encoded(changed), observed, files, validator)

    def test_selected_tail_copy_and_typed_images_bind_original_parts(self):
        value, observed, files, validator = fixture()
        for key in value['images']:
            changed = copy.deepcopy(value); changed['images'][key] = [9] * 32
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                D.report(encoded(changed), observed, files, validator)

    def test_no_incomplete_census_or_extra_fields(self):
        value, observed, files, validator = fixture()
        for key in ('rows', 'completions', 'ranks'):
            changed = copy.deepcopy(value); changed[key].pop()
            with self.subTest(key=key), self.assertRaises(RuntimeError): D.report(encoded(changed), observed, files, validator)
        for key, replacement in (('extra', True), ('final_dispatches', [591, 580])):
            changed = dict(value, **{key: replacement})
            with self.assertRaises(RuntimeError): D.report(encoded(changed), observed, files, validator)

    def test_false_authority_or_wrong_queue_policy_refused(self):
        value, observed, files, validator = fixture()
        for key in (*D.FALSE, 'cache_kernel_admission', 'operational_currentness'):
            for replacement in (True, 0, None):
                with self.subTest(key=key, value=replacement), self.assertRaises(RuntimeError):
                    D.report(encoded(dict(value, **{key: replacement})), observed, files, validator)
        for key in ('raw_timestamp_queue', 'shared_full_currentness', 'native_closed', 'raw_completion_ticks'):
            with self.assertRaises(RuntimeError): D.report(encoded(dict(value, **{key: False})), observed, files, validator)

    def test_completions_pid_profile_worker_and_bootstrap_are_actual(self):
        value, observed, files, validator = fixture()
        for key, replacement in (('child_pid', 11), ('profile_sha256', [8] * 32),
                                  ('transcript_sha256', [8] * 32), ('worker_sha256', [8] * 32)):
            with self.assertRaises(RuntimeError): D.report(encoded(dict(value, **{key: replacement})), observed, files, validator)
        changed = copy.deepcopy(value); changed['completions'][3]['generation'] = 1
        with self.assertRaises(RuntimeError): D.report(encoded(changed), observed, files, validator)
        changed = copy.deepcopy(value); changed['bootstrap']['extra'] = 1
        with self.assertRaises(RuntimeError): D.report(encoded(changed), observed, files, validator)

    def test_closed_device_request_refuses_old_schema_policy_and_extra_fields(self):
        value = dict(schema=D.REQUEST_SCHEMA, decode={})
        self.assertEqual(D.request(value), {})
        for changed in (dict(value, policy='shared-full-currentness'), dict(value, schema='HostV2'),
                         dict(value, decode=[])):
            with self.assertRaises(RuntimeError): D.request(changed)

    def test_parent_wrapper_projection_and_exact_nested_bytes(self):
        value, observed, files, validator = fixture()
        summary = encoded(observed) + b'\n'
        parent = dict(path='/parent', bytes=10, sha256='a' * 64)
        raw = encoded(value); sidecar = dict(path='/case/native-device-v1.json', bytes=len(raw),
                                           sha256=hashlib.sha256(raw).hexdigest())
        external = dict(schema=D.REQUEST_SCHEMA, decode=observed['request'])
        wrapper = dict(schema='FerricFinitePrefixDecodeDeviceDiagnosticV1', parent_binary=parent,
            request_projection_sha256=list(hashlib.sha256(encoded(external)).digest()),
            observation=observed, device_sidecar=sidecar, raw_completion_ticks=True,
            **{k: False for k in D.FALSE})
        C = SimpleNamespace(pin=lambda value: value)
        D.validate(C, encoded(wrapper) + b'\n', raw, sidecar, summary, observed, parent, external, files, validator)
        for key, replacement in (('request_projection_sha256', [0] * 32), ('parent_binary', dict(parent, bytes=11)),
                                  ('device_sidecar', dict(sidecar, path='/elsewhere')), ('raw_completion_ticks', False),
                                  ('schema', 'FerricFinitePrefixDecodeHostDiagnosticV2')):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                D.validate(C, encoded(dict(wrapper, **{key: replacement})) + b'\n', raw, sidecar,
                           summary, observed, parent, external, files, validator)

    def test_duplicate_json_and_oversized_sidecar_refused(self):
        value, observed, files, validator = fixture()
        for raw in (b'{"schema":0,' + encoded(value)[1:], b' ' * (D.MAX_BYTES + 1), b''):
            with self.assertRaises(RuntimeError): D.report(raw, observed, files, validator)

    def test_all_four_payloads_tokens_and152_rows_are_required(self):
        C, baseline, observed, files, helpers, _ = self.invariance_fixture()
        rows = D.invariance(C, baseline, observed, files, helpers)
        self.assertEqual((len(rows), sum(len(row['tensors']) for row in rows)), (4, 152))
        for position in range(4):
            changed = dict(files); key = f'observation-{position}.bin'
            changed[key] = bytes([255]) + changed[key][1:]
            with self.subTest(position=position), self.assertRaises(RuntimeError):
                D.invariance(C, baseline, observed, changed, helpers)
        changed = copy.deepcopy(observed); changed['records'][3]['output_token'] += 1
        with self.assertRaises(RuntimeError): D.invariance(C, baseline, changed, files, helpers)

    def test_immutable_request_and_all_forward_input_fields_match(self):
        C, baseline, observed, files, helpers, _ = self.invariance_fixture()
        for key, replacement in (('mode', 'autoregressive'), ('immutable', {}),
                                  ('session', baseline['observed']['request']['session'])):
            changed = copy.deepcopy(observed); changed['request'][key] = replacement
            with self.subTest(key=key), self.assertRaises(RuntimeError): D.invariance(C, baseline, changed, files, helpers)
        for field in ('cache_metadata', 'rotary_bits', 'token'):
            changed = dict(files); frame = json.loads(changed['request-2.json'])
            frame['command'][field] = 999
            changed['request-2.json'] = encoded(frame)
            with self.subTest(field=field), self.assertRaises(RuntimeError): D.invariance(C, baseline, observed, changed, helpers)

    def test_incomparable_histories_and_partial_tensor_comparisons_refused(self):
        C, baseline, observed, files, helpers, rows = self.invariance_fixture()
        for change in ('history', 'tensor', 'count'):
            changed = copy.deepcopy(rows)
            if change == 'history': changed[1]['same_input_history'] = False
            elif change == 'tensor': changed[3]['tensors'][37]['byte_equal'] = False
            else: changed[0]['tensors'].pop()
            C.compare_rows = lambda *args: changed
            with self.subTest(change=change), self.assertRaises(RuntimeError): D.invariance(C, baseline, observed, files, helpers)


if __name__ == '__main__': unittest.main()
