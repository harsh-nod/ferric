"""CPU-only synthetic replay fixtures; no controller, model, GPU or server launch."""
import collections
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('packet_ticks', Path(__file__).parent / 'harness/measurement/packet_ticks.py')
ticks = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ticks)


def encoded(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False).encode()


def fixture():
    prefix = 'ferric_qwen3_tp_batch32_'
    names = [prefix + suffix for suffix in (
        'embedding_bf16_v5', 'wave_rmsnorm_bf16_v15', 'mfma_gemm_bf16_v5', 'wave_gemv_bf16_v5',
        'rope_v5', 'wave_paged_gqa_query_hoist_bf16_v14', 'mfma_gemm_partial_f32_v5',
        'wave_gemv_partial_f32_v5', 'residual_bf16_v5', 'swiglu_bf16_f32_v5',
        'mfma_head_f32_v8', 'wave_argmax_f32_v11')]
    names += ['qwen3_rmsnorm_v1', 'ferric_qwen3_tp_prefill16_kv_copy_bf16_v27',
              'ferric_qwen3_tp_c1_kv_copy_bf16_v19',
              'ferric_qwen3_tp_c1_split8_attention_partial_f32_v21',
              'ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21']
    names += [f'unused_kernel_{index}' for index in range(10)]
    catalog = {name: f'{index:064x}' for index, name in enumerate(names, 1)}
    ids = {name: index for index, name in enumerate(names, 1)}
    value = {'schema': 'FerricOrdered64RawPacketIntervalsV1',
        'evidence_scope': 'transport_intervals_only_not_numerical_or_performance_qualification',
        'core_revision': ticks.CORE, 'tick_unit': ticks.TICK_UNIT, 'interval_scope': ticks.INTERVAL_SCOPE,
        'original_max_group_packets': 64, 'device_unique_id': 77, 'operations': ticks.OPERATIONS,
        'kernels': [{'kernel': index, 'symbol': name,
                     'object_sha256': list(bytes.fromhex(f'{index:064x}'))} for index, name in enumerate(names, 1)],
        'group_columns': ticks.GROUP_COLUMNS, 'record_columns': ticks.RECORD_COLUMNS,
        'batches': [], 'groups': [], 'records': []}
    partitions = collections.defaultdict(list)
    group = packet = 0
    for ordinal in range(1, 136):
        prefill = ordinal <= 8
        roles = [(0, 0)]
        for layer in range(1, 37):
            roles.extend((layer, op) for op in range(1, 10))
            if not prefill:
                roles.append((layer, 9))
            roles.extend((layer, op) for op in range(10, 18))
        if ordinal >= 8:
            roles.extend([(0, 18), (0, 19), (0, 20)])
        widths = ([1] + [11, 6] * 36 + ([1, 1, 1] if ordinal == 8 else [])) if prefill else [64] * 10 + [12]
        value['batches'].append({'ordinal': ordinal, 'scheduler_batch_id': ordinal,
            'request': [0, 1], 'prefill_rows': 16 if prefill else 0,
            'decode_rows': 0 if prefill else 1, 'expected_packets': len(roles)})
        role_index = 0
        for width in widths:
            value['groups'].append([group, 0 if width == 1 else 2, width])
            for _ in range(width):
                layer, operation = roles[role_index]
                suffix = {0: 'embedding_bf16_v5', 1: 'wave_rmsnorm_bf16_v15',
                    7: 'rope_v5', 11: 'residual_bf16_v5', 12: 'wave_rmsnorm_bf16_v15',
                    15: 'swiglu_bf16_f32_v5', 17: 'residual_bf16_v5',
                    18: 'wave_rmsnorm_bf16_v15', 19: 'mfma_head_f32_v8', 20: 'wave_argmax_f32_v11'}.get(operation)
                if operation in (2, 3, 4, 13, 14):
                    suffix = 'mfma_gemm_bf16_v5' if prefill else 'wave_gemv_bf16_v5'
                if operation in (10, 16):
                    suffix = 'mfma_gemm_partial_f32_v5' if prefill else 'wave_gemv_partial_f32_v5'
                symbol = prefix + suffix if suffix is not None else 'qwen3_rmsnorm_v1'
                if operation == 8:
                    symbol = 'ferric_qwen3_tp_prefill16_kv_copy_bf16_v27' if prefill else 'ferric_qwen3_tp_c1_kv_copy_bf16_v19'
                if operation == 9:
                    symbol = prefix + 'wave_paged_gqa_query_hoist_bf16_v14' if prefill else (
                        'ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21'
                        if roles[role_index - 1] == (layer, 9) else 'ferric_qwen3_tp_c1_split8_attention_partial_f32_v21')
                kernel = ids[symbol]
                interval = 10 + packet % 7
                value['records'].append([group, ordinal, layer, operation, kernel, 0, packet,
                                         packet * 100 + 1, packet * 100 + 1 + interval])
                partitions[(0, prefill, operation, kernel)].append(interval)
                role_index += 1
                packet += 1
            group += 1
    copy_identity = {'requested_kv_copy_mode': 'parallel-c1-v19', 'kv_copy_mode': 'parallel-c1-v19',
        'kv_append_mode': 'parallel-c1-v19', 'kv_copy_artifact_path': '/images/copy',
        'kv_copy_artifact': {'artifact_manifest_id': 'a' * 64, 'artifact_hsaco_id': 'b' * 64,
                             'artifact_handoff_id': 'c' * 64}}
    raw = encoded(value)
    terminal = {'schema': 'FerricOrdered64PacketTicksClosedV1', 'authority': 'none',
        'live_profile': ticks.PROFILE, 'performance_qualified': False, 'serving_qualified': False,
        'raw_capture_path': '/stage/cells/counter-A/cell-results/packet-ticks.json',
        'raw_capture_sha256': hashlib.sha256(raw).hexdigest(), 'raw_capture_bytes': len(raw),
        'core_wire_revision': ticks.CORE, 'scope': ticks.TERMINAL_SCOPE,
        'numerical_status': 'independently compare every emitted model token ID',
        'rank_dispatch_counts': [87711], 'all_workers_exited': True,
        'summary': ticks.summaries(partitions), **copy_identity}
    kwargs = {'sidecar_path': terminal['raw_capture_path'], 'device': 77,
              'allowed_kernels': catalog, 'copy_identity': copy_identity}
    return value, terminal, kwargs


class PacketTicksTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.capture, cls.terminal, cls.kwargs = fixture()
        cls.raw = encoded(cls.capture)

    def reject_capture(self, mutate):
        value = copy.deepcopy(self.capture)
        mutate(value)
        raw = encoded(value)
        terminal = {**self.terminal, 'raw_capture_sha256': hashlib.sha256(raw).hexdigest(),
                    'raw_capture_bytes': len(raw)}
        with self.assertRaises(ValueError):
            ticks.validate(raw, terminal, **self.kwargs)

    def test_complete_exact_shape_and_raw_summary(self):
        result = ticks.validate(self.raw, self.terminal, **self.kwargs)
        self.assertEqual((result['batches'], result['records'], result['groups']), (135, 87711, 1984))
        self.assertEqual(result['group_counts'], {'prefill_direct': 11, 'prefill_ordered': 576, 'decode_ordered': 1397})
        self.assertEqual(len(result['summary']), 43)
        self.assertFalse(result['sums_are_wall_time_shares'])
        self.assertFalse(result['latency_admitted'])

    def test_missing_or_extra_batch(self):
        self.reject_capture(lambda value: value['batches'].pop())
        self.reject_capture(lambda value: value['batches'].append(value['batches'][-1]))

    def test_missing_or_extra_packet(self):
        self.reject_capture(lambda value: value['records'].pop())
        self.reject_capture(lambda value: value['records'].append(value['records'][-1]))

    def test_wrong_group_boundary_or_publication(self):
        self.reject_capture(lambda value: value['groups'][1].__setitem__(2, 10))
        self.reject_capture(lambda value: value['groups'][1].__setitem__(1, 1))

    def test_wrong_batch_row_geometry(self):
        self.reject_capture(lambda value: value['batches'][7].__setitem__('prefill_rows', 32))
        self.reject_capture(lambda value: value['batches'][8].__setitem__('decode_rows', 2))

    def test_wrong_request_or_scheduler_identity(self):
        self.reject_capture(lambda value: value['batches'][1].__setitem__('request', [0, 2]))
        self.reject_capture(lambda value: value['batches'][1].__setitem__('scheduler_batch_id', 1))

    def test_wrong_layer_operation_or_frontier(self):
        for column, replacement in ((2, 2), (3, 17), (5, 1), (6, 3)):
            self.reject_capture(lambda value, col=column, new=replacement: value['records'][1].__setitem__(col, new))

    def test_unknown_kernel_or_wrong_object(self):
        self.reject_capture(lambda value: value['records'][0].__setitem__(4, 1000))
        self.reject_capture(lambda value: value['kernels'][0]['object_sha256'].__setitem__(0, 255))

    def test_consistent_admitted_kernel_permutation_cannot_relabel_operations(self):
        value = copy.deepcopy(self.capture)
        partitions = collections.defaultdict(list)
        for row in value['records']:
            if row[4] in (1, 2):
                row[4] = 3 - row[4]
            partitions[(row[5], row[1] <= 8, row[3], row[4])].append(row[8] - row[7])
        raw = encoded(value)
        terminal = {**self.terminal, 'raw_capture_sha256': hashlib.sha256(raw).hexdigest(),
                    'raw_capture_bytes': len(raw), 'summary': ticks.summaries(partitions)}
        self.assertEqual(len(terminal['summary']), 43)
        with self.assertRaisesRegex(ValueError, 'exact phase/operation/substep'):
            ticks.validate(raw, terminal, **self.kwargs)

    def test_empty_equal_reversed_or_boolean_ticks(self):
        for start, end in ((0, 1), (10, 10), (11, 10), (True, 20)):
            def mutate(value):
                value['records'][0][7:] = [start, end]
            self.reject_capture(mutate)

    def test_clock_unit_and_scope_cannot_be_relabelled(self):
        for key, replacement in (('tick_unit', 'nanoseconds'), ('interval_scope', 'shader_only'),
                                 ('core_revision', '5e2f668d'), ('original_max_group_packets', 16)):
            self.reject_capture(lambda value, field=key, new=replacement: value.__setitem__(field, new))

    def test_terminal_hash_path_size_and_close_are_mandatory(self):
        for key, replacement in (('raw_capture_sha256', 'f' * 64), ('raw_capture_bytes', len(self.raw) - 1),
                                 ('raw_capture_path', '/another.json'), ('all_workers_exited', False),
                                 ('rank_dispatch_counts', [87710]), ('performance_qualified', True),
                                 ('scope', 'calibrated nanoseconds')):
            with self.subTest(key=key), self.assertRaises(ValueError):
                ticks.validate(self.raw, {**self.terminal, key: replacement}, **self.kwargs)

    def test_summary_is_recomputed_from_raw_intervals(self):
        terminal = copy.deepcopy(self.terminal)
        terminal['summary'][0]['p50_raw_dispatch_interval_ticks'] += 1
        with self.assertRaises(ValueError):
            ticks.validate(self.raw, terminal, **self.kwargs)

    def test_copy_binding_cannot_override_terminal_clock_claim(self):
        kwargs = {**self.kwargs, 'copy_identity': {**self.kwargs['copy_identity'], 'scope': 'nanoseconds'}}
        with self.assertRaises(ValueError):
            ticks.validate(self.raw, self.terminal, **kwargs)

    def test_duplicate_unknown_keys_and_nonfinite_json_reject(self):
        for raw in (b'{"schema":1,"schema":2}', b'{"value":NaN}', b'', b'x' * (ticks.MAX_BYTES + 1)):
            with self.assertRaises((ValueError, json.JSONDecodeError)):
                ticks.validate(raw, self.terminal, **self.kwargs)

    def test_private_real_sidecar_read_rejects_mode_links_and_empty_files(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'ticks.json'
            path.write_bytes(self.raw)
            path.chmod(0o600)
            self.assertEqual(ticks.read_sidecar(path), self.raw)
            link = path.with_name('alias.json')
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                ticks.read_sidecar(link)
            link.unlink()
            os.link(path, link)
            with self.assertRaises(ValueError):
                ticks.read_sidecar(path)
            link.unlink()
            path.chmod(0o644)
            with self.assertRaises(ValueError):
                ticks.read_sidecar(path)
            path.chmod(0o600)
            path.write_bytes(b'')
            with self.assertRaises(ValueError):
                ticks.read_sidecar(path)


if __name__ == '__main__':
    unittest.main()
