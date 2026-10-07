"""Exact ordinary V19 split-K selection and raw graph counts, not GPU counters."""
import hashlib

ARMS = {'A': 'baseline', 'B': 'splitk8-down-mfma-r1'}
PROFILE = 'prefill16-decode-ordered64-v19-splitk-down-r1-live-v1'
ROOTS = ('ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1',
         'ferric_qwen3_c1_down_splitk8_merge_f32_r1')
IMAGE = {'hsaco': '1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae',
         'manifest': '2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54',
         'handoff': 'cd23060449750a973f05742c0d5a043cb637491743e2ac56b56e847cc2c4dbdf'}


def require(value, message):
    if not value:
        raise ValueError(message)


def same(actual, expected, message):
    require(type(actual) is type(expected), message)
    if type(expected) is dict:
        require(set(actual) == set(expected), message)
        for key, value in expected.items():
            same(actual[key], value, message)
    elif type(expected) is list:
        require(len(actual) == len(expected), message)
        for first, second in zip(actual, expected):
            same(first, second, message)
    else:
        require(actual == expected, message)


def expected_mechanism(arm):
    require(arm in ARMS, 'closed split-K arm')
    selected = arm == 'B'
    return {'schema': 'FerricSplitKRawGraphMechanismV1',
        'scope': 'Raw Batch/Closed graph counts; not runtime counters or GPU timing',
        'prefill_chunks': 8, 'prefill_dispatches_per_chunk': 613, 'head_dispatches': 3,
        'decode_batches': 127, 'decode_dispatches_per_batch': 688 if selected else 652,
        'model_batches': 135, 'model_dispatches': 92283 if selected else 87711,
        'runtime_counters': False, 'latency_sample_admitted': False}


def expected_metadata(spec):
    arm = spec['arm']
    require(arm in ARMS, 'closed split-K arm')
    selected = arm == 'B'
    binding = spec['splitk']
    image, roster = binding['image'], binding['roster']
    same({key: image[key] for key in IMAGE}, IMAGE, 'exact measured unpaired image')
    require(type(roster['value']) is list and len(roster['value']) == 2, 'exact two-root roster')
    for root, export in zip(roster['value'], ROOTS):
        require(type(root) is dict and set(root) == {'logical_name', 'export_name'}
                and root['export_name'] == export and type(root['logical_name']) is str
                and 0 < len(root['logical_name']) <= 512 and root['logical_name'].isascii()
                and all(32 <= ord(char) < 127 for char in root['logical_name']),
                'ordered bounded compiler root identity')
    require(roster['value'][0]['logical_name'] != roster['value'][1]['logical_name'],
            'distinct compiler logical names')
    return {'schema': 'FerricSplitKDownLiveSelectionR1', 'authority': 'none',
        'requested_mode': ARMS[arm], 'actual_mode': ARMS[arm],
        'additional_weight_bytes': 0, 'resident_transposed_down_bytes': 3623878656,
        'activation_scratch_bytes': 131072, 'loaded_image_count': 10,
        'composition': 'V5/V8/V11/V14/V15/V19/V20/V21/V27 plus unpaired split-K down; V20 unselected',
        'weight_source': 'existing authenticated resident KxN down tensors, no new transpose or upload',
        'worker_backend': 'ordinary-ordered64', 'selected_phase': 'decode', 'selected_rows': 1,
        'selected_published_rows': 1, 'roles': [2], 'c1_only': True,
        'max_simultaneous_selected_requests': 1, 'prefill_unchanged': True,
        'same_image_set_in_both_arms': True, 'same_scratch_in_both_arms': True,
        'extra_packets_per_selected_forward': 36 if selected else 0,
        'decode_packets': 688 if selected else 652,
        'model_batches_128_128': 135, 'model_dispatches_128_128': 92283 if selected else 87711,
        'source_schedule_only': True, 'native_qualified': False, 'performance_qualified': False,
        'serving_qualified': False,
        'numerical_caveat': 'changed split reduction grouping requires full-model token parity',
        'artifact_path': image['path'],
        'artifact': {'artifact_manifest_id': IMAGE['manifest'], 'artifact_hsaco_id': IMAGE['hsaco'],
                     'artifact_handoff_id': IMAGE['handoff']},
        'compiler_roster': {'path': roster['path'], 'sha256': roster['sha256'], 'value': roster['value']}}


def metadata(value, spec):
    same(value, expected_metadata(spec), 'closed actual split-K image/storage/phase metadata')


def stable_metadata(value):
    changed = {'requested_mode', 'actual_mode', 'extra_packets_per_selected_forward',
               'decode_packets', 'model_dispatches_128_128'}
    return {key: item for key, item in value.items() if key not in changed}


def cli(spec):
    require(spec['arm'] in ARMS, 'closed split-K CLI arm')
    image, roster = spec['splitk']['image'], spec['splitk']['roster']
    required = {'--splitk-down-mode': ARMS[spec['arm']], '--splitk-down-artifact': image['path'],
        '--splitk-down-roster': roster['path'], '--splitk-down-roster-sha256': roster['sha256'],
        '--splitk-down-hsaco-sha256': IMAGE['hsaco'],
        '--splitk-down-manifest-sha256': IMAGE['manifest'],
        '--splitk-down-handoff-sha256': IMAGE['handoff']}
    argv = spec['argv']
    require(type(argv) is list and all(type(value) is str for value in argv), 'literal argv')
    forbidden = ('--native-prefill-rows', '--token-program-backend', '--token-program-fence-mode',
                 '--model-timestamps', '--ordered64-packet-ticks', '--ordered64-runtime-counters')
    require(not any(flag in argv for flag in forbidden), 'unrelated instrumented/program/width composition')
    for name, value in required.items():
        require(argv.count(name) == 1 and argv.index(name) + 1 < len(argv)
                and argv[argv.index(name) + 1] == value, 'exact split-K selector: ' + name)
    return required


def common_argv(spec):
    cli(spec)
    value = list(spec['argv'])
    value[value.index('--splitk-down-mode') + 1] = '<declared-down-selection>'
    return value


class Events:
    """Reuse packed-down's raw652/688 adapter while preserving frozen chronology."""

    def __init__(self, runner, legacy, arm):
        require(arm in ARMS, 'closed raw-event arm')
        self.base = legacy.Events(runner, 'split8-v21')
        self.selected = arm == 'B'
        self.extra_dispatches = 0

    @property
    def emission(self):
        return self.base.emission

    @property
    def batches(self):
        return self.base.batches

    @property
    def dispatches(self):
        return self.base.dispatches + self.extra_dispatches

    def validate(self, event):
        if event.get('event') != 'batch':
            self.base.validate(event)
            return
        ordinal = self.batches % 135
        baseline = 613 if ordinal < 7 else 616 if ordinal == 7 else 652
        extra = 36 if self.selected and ordinal >= 8 else 0
        observed = event.get('rank_dispatch_counts')
        require(type(observed) is list and len(observed) == 1 and type(observed[0]) is int
                and observed[0] == baseline + extra, 'exact raw per-phase down dispatch count')
        self.base.validate(dict(event, rank_dispatch_counts=[baseline]))
        self.extra_dispatches += extra


def setup_provenance(raw_setup, experimental_binding):
    """A separate evidence record, never a modification of the controller's Setup."""
    require(type(raw_setup) is bytes and raw_setup, 'original raw Setup bytes')
    require(type(experimental_binding) is dict and set(experimental_binding) == {'path', 'sha256'},
            'exact externally validated experimental retention binding')
    return {'schema': 'FerricSplitKSetupProvenanceV1',
        'raw_setup_sha256': hashlib.sha256(raw_setup).hexdigest(),
        'experimental_retention': dict(experimental_binding),
        'strict_clippy': 'failed', 'binary_lint_coverage': 'incomplete',
        'deferred_roles': ['regression-token', 'default', 'fallback', 'union', 'union-cli',
                           'source-policy', 'clippy-union'],
        'production_qualified': False, 'default_promotion': False,
        'scope': 'Experimental same-image ordinary ordered64/V19 down-only A/B'}
