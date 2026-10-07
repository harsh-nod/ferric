"""Pure admission and comparison for four conditional framework MLP calls."""
import capture
import diagnostics as D
from common import compact, encoded, parse, require

STAGES = {'input': 4096, 'gate-input': 4096, 'up-input': 4096,
          'gate': 12288, 'up': 12288, 'silu-input': 12288, 'silu': 12288,
          'product': 12288, 'down-projection': 4096, 'mlp-output': 4096}
CALLS = ('control-1', 'native-1', 'control-2', 'native-2')


class Fields:
    PAYLOAD_BYTES = 606976
    parse = staticmethod(parse)

    @staticmethod
    def keys(value, names):
        require(type(value) is dict and set(value) == set(names.split()), 'closed capture fields')

    @staticmethod
    def uint(value, maximum=(1 << 64) - 1):
        require(type(value) is int and 0 <= value <= maximum, 'strict unsigned integer')
        return value

    @staticmethod
    def octets(value, count=32):
        require(type(value) is list and len(value) == count
                and all(type(v) is int and 0 <= v <= 255 for v in value), 'octet array')
        return bytes(value)

    @staticmethod
    def rust_pin(value):
        require(type(value['path']) is str, 'original native path')
        return dict(path=value['path'], bytes=Fields.uint(value['bytes']),
                    sha256=Fields.octets(value['sha256']).hex())


def native_inputs(bodies, contract):
    terminal, numerical = parse(bodies['terminal']), parse(bodies['numerical'])
    require(terminal['schema'] == 'ferric-guarded-mlp-model-stage-capture-gpu-v1'
            and terminal['passed'] is True and terminal['errors'] == []
            and terminal['capture_requested'] is terminal['capture_verified'] is True
            and terminal['native_attempts'] == 1 and terminal['retries'] == 0,
            'actual successful native capture')
    require(numerical['schema'] == 'ferric-guarded-mlp-model-stage-numerical-report-v1'
            and numerical['passed'] is True and numerical['error'] is None
            and numerical['postcheck_errors'] == [] and numerical['input_posthashes_complete'] is True,
            'prior complete native data admission')
    for value in (terminal, numerical):
        require(all(value[key] is False for key in ('numerical_acceptance', 'full_model_acceptance',
                    'performance_claim', 'production_authority')), 'original nonclaims')
    for relative, role in (('native/complete.json', 'native'), ('native/child-stderr.bin', 'envelope'),
                           ('native/request-0.json', 'request0'), ('native/observation-0.bin', 'observation0')):
        require(terminal['raw'][relative] == contract['files'][role], 'terminal/original raw-body join')
    by_path = {row['path']: bodies[name] for name, row in contract['files'].items()}
    def read_body(row):
        require(row['path'] in by_path and compact(by_path[row['path']]) ==
                {key: row[key] for key in ('bytes', 'sha256')}, 'original same-run capture body')
        return by_path[row['path']]
    checked = capture.capture_admission(bodies['native'], read_body, Fields)
    require(checked['source'] == contract['files']['envelope'], 'capture source is original stderr')
    prior = numerical['diagnostic']['candidate']
    require(encoded(checked) == encoded(terminal['capture_observation']) ==
            encoded(parse(bodies['capture_checked'])) == encoded(prior['capture_observation'])
            and encoded(prior['observation']) == encoded(terminal['observation']), 'saved native capture joins')
    envelope = parse(bodies['envelope'])['capture']
    payload = Fields.octets(envelope['payload'], 256136)
    selected, parts = {}, []
    for row in envelope['parts']:
        if row['boundary'] == 'after_mlp' and row['role'] in ('post_normalized', 'gate', 'up', 'activation'):
            key = (row['rank'], row['role'])
            require(key not in selected, 'unique native MLP part')
            body = payload[row['offset']:row['offset'] + row['bytes']]
            D.words(body, 4096 if row['role'] == 'post_normalized' else 6144)
            selected[key] = body
            parts.append(dict(rank=row['rank'], role=row['role'], offset=row['offset'], **compact(body)))
    require(set(selected) == {(rank, role) for rank in (0, 1)
            for role in ('post_normalized', 'gate', 'up', 'activation')}, 'eight native MLP parts')
    require(selected[(0, 'post_normalized')] == selected[(1, 'post_normalized')],
            'native TP ranks must share the exact matched input')
    return selected, dict(capture_observation=checked, selected_parts=parts,
                         native_activation_is_product=True, native_silu_not_captured=True)


def framework_inputs(bodies, contract):
    aliases = parse(bodies['framework_aliases'])['files']
    owner, prior = parse(bodies['framework_owner']), parse(bodies['framework_capture'])
    require(owner['schema'] == 'ferric-p228-layer0-framework-launch-complete-v1'
            and owner['passed'] is True and owner['failures'] == [] and owner['native_attempts'] == 1
            and owner['retries'] == 0,
            'original framework owned success')
    require(owner['reference'] == contract['framework']['framework_capture']['original'],
            'original owner authenticates original capture report')
    require(prior['schema'] == 'ferric-p228-layer0-framework-capture-v1'
            and prior['status'] == 'PASS' and prior['repeat_passes_byte_equal'] is True
            and prior['genuine_framework_chain'] is True
            and prior['candidate_intermediate_inputs'] is False
            and len(prior['passes']) == 2, 'original genuine two-pass repeat')
    for name, row in contract['framework'].items():
        require(aliases[row['original']['path']] == row, 'original-to-physical framework alias')
        require(compact(bodies[name]) == {key: row['original'][key] for key in ('bytes', 'sha256')},
                'framework original content identity')
    expected = {}
    for stage, count in STAGES.items():
        original = 'post-norm' if stage == 'input' else stage
        first, second = (bodies['framework-pass%d-%s' % (i, original)] for i in (1, 2))
        D.words(first, count)
        require(first == second, 'two original framework stage bodies differ')
        for i, body in ((1, first), (2, second)):
            stage_pin = prior['passes'][i - 1]['stages'][original]['pin']
            require(stage_pin == contract['framework']['framework-pass%d-%s' % (i, original)]['original']
                    and {key: stage_pin[key] for key in ('bytes', 'sha256')} == compact(body),
                    'original framework report/stage join')
        expected[stage] = first
    validate_stages(expected, expected['input'])
    return expected


class Hooks:
    def __init__(self, raw, retain):
        self.raw, self.retain, self.values, self.handles = raw, retain, {}, []

    def add(self, name, tensor):
        require(name in STAGES and name not in self.values, 'duplicate/unknown actual hook')
        body = self.raw(tensor, (1, 1, STAGES[name]))
        D.words(body, STAGES[name])
        self.retain(name, body)
        self.values[name] = body

    def attach(self, module):
        for component, before, after in ((module.gate_proj, 'gate-input', 'gate'),
                (module.up_proj, 'up-input', 'up'), (module.act_fn, 'silu-input', 'silu'),
                (module.down_proj, 'product', 'down-projection')):
            self.handles.append(component.register_forward_pre_hook(
                lambda _module, arguments, name=before: self.add(name, arguments[0])))
            self.handles.append(component.register_forward_hook(
                lambda _module, _arguments, output, name=after: self.add(name, output)))
        self.handles.append(module.register_forward_hook(
            lambda _module, _arguments, output: self.add('mlp-output', output)))

    def close(self):
        errors = []
        for handle in reversed(self.handles):
            try:
                handle.remove()
            except BaseException as error:
                errors.append(str(error))
        self.handles.clear()
        require(not errors, 'hook cleanup: ' + repr(errors))


def validate_stages(values, input_raw):
    require(type(values) is dict and set(values) == set(STAGES), 'closed ten-stage hook roster')
    for stage, count in STAGES.items():
        D.words(values[stage], count)
    require(values['input'] == values['gate-input'] == values['up-input'] == input_raw,
            'actual module arguments equal selected BF16 input')
    require(values['gate'] == values['silu-input'], 'actual SiLU input is gate output')
    require(values['down-projection'] == values['mlp-output'], 'actual Down output is module output')


def rank_half(raw, rank):
    require(type(rank) is int and rank in (0, 1), 'strict TP rank')
    D.words(raw, 12288)
    return raw[rank * 12288:(rank + 1) * 12288]


def compare_calls(calls, native, original):
    require(type(calls) is dict and tuple(calls) == CALLS, 'exact four-call order')
    for name, values in calls.items():
        validate_stages(values, original['input'] if name.startswith('control')
                        else native[(0, 'post_normalized')])
    control = {stage: calls['control-1'][stage] == calls['control-2'][stage] == original[stage]
               for stage in STAGES}
    repeat = {stage: calls['native-1'][stage] == calls['native-2'][stage] for stage in STAGES}
    gate = all(control.values()) and all(repeat.values())
    rows = None
    if gate:
        rows = []
        for rank in (0, 1):
            for stage, role in (('gate', 'gate'), ('up', 'up'), ('product', 'activation')):
                actual = native[(rank, role)]
                rows.append(dict(rank=rank, stage=stage, native_role=role,
                    matched_input=D.compare_tensor(rank_half(calls['native-1'][stage], rank), actual),
                    original_chain=D.compare_tensor(rank_half(original[stage], rank), actual),
                    framework_input_effect=D.compare_tensor(rank_half(original[stage], rank),
                                                           rank_half(calls['native-1'][stage], rank))))
    return dict(schema='ferric-guarded-mlp-matched-input-diagnostic-v1',
        control_reproduces_original=control, native_input_repeat_equal=repeat,
        control_and_repeat_gate_passed=gate, comparisons=rows,
        input_difference=D.compare_tensor(original['input'], native[(0, 'post_normalized')]),
        framework_down_input_effect=D.compare_tensor(original['down-projection'],
            calls['native-1']['down-projection']) if gate else None,
        native_down_partials_compared_to_full_bf16=False, native_standalone_silu_compared=False,
        module_calls=4, full_model_forward_calls=0, model_layer=0, original_position=0,
        input_substitution_explicit=True, matched_input_is_genuine_full_model_history=False,
        numerical_acceptance=False, acceptance_threshold=None, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
