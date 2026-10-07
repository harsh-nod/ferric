"""Closed generated-history reference records, not native numerical acceptance."""
import hashlib
import struct

from common import compact, encoded, require
import reference as R

D = R.D
INPUTS = 2048
OUTPUTS = 256
CALLS = 256
POSITIONS = 2303
SELECTED = (0, 2047, 2048, 2302)
PAYLOAD_BYTES = D.PAYLOAD_BYTES
VOCABULARY = D.VOCABULARY
DECODED_CAP = 128 << 10
PASS_CAP = 2 << 20
OUTPUT_CAP = 32 << 20
prompt_inputs = R.prompt_inputs


def token(value):
    return R.uint(value, VOCABULARY - 1)


def input_ids(tokens, step, generated):
    require(type(tokens) is list and len(tokens) == INPUTS
            and all(type(t) is int and 0 <= t < VOCABULARY for t in tokens), 'full authentic prompt')
    R.uint(step, OUTPUTS - 1)
    require(type(generated) is list and len(generated) == step, 'exact own generated prefix length')
    for value in generated:
        token(value)
    return list(tokens) if step == 0 else [generated[-1]]


def units(bits):
    require(type(bits) is int and 0 <= bits <= 65535 and bits & 0x7f80 != 0x7f80, 'finite BF16 bits')
    exponent, fraction = (bits >> 7) & 255, bits & 127
    magnitude = fraction if exponent == 0 else (128 + fraction) << (exponent - 1)
    return -magnitude if bits & 0x8000 else magnitude


def summarize(raw):
    row = D.words(raw, VOCABULARY)
    first = second = None
    ties = 0
    for index, bits in enumerate(row):
        item = (units(bits), index, bits)
        if first is None or item[0] > first[0]:
            second, first, ties = first, item, 1
        elif item[0] == first[0]:
            ties += 1
            if second is None or item[0] > second[0]:
                second = item
        elif second is None or item[0] > second[0]:
            second = item
    require(second is not None, 'two finite vocabulary entries')
    return dict(token_id=first[1], top_bits=first[2], runner_up_token_id=second[1],
        runner_up_bits=second[2], maximum_tie_count=ties,
        margin_units_2_neg133=str(first[0] - second[0]), finite=True, logits=compact(raw))


def validate_summary(value):
    require(type(value) is dict and set(value) == {'token_id', 'top_bits', 'runner_up_token_id',
        'runner_up_bits', 'maximum_tie_count', 'margin_units_2_neg133', 'finite', 'logits'}, 'closed greedy row')
    token(value['token_id']); token(value['runner_up_token_id'])
    require(value['token_id'] != value['runner_up_token_id'] and value['finite'] is True, 'finite distinct top two')
    margin = units(value['top_bits']) - units(value['runner_up_bits'])
    require(margin >= 0 and type(value['margin_units_2_neg133']) is str
            and value['margin_units_2_neg133'] == str(margin), 'exact BF16 top-two margin')
    count = R.uint(value['maximum_tie_count'], VOCABULARY)
    require(count >= 1 and ((count == 1 and margin > 0) or
            (count >= 2 and margin == 0 and value['token_id'] < value['runner_up_token_id'])),
            'lowest-ID tie and declared top-two consistency')
    R.pin(value['logits'], VOCABULARY * 2)


class Capture(R.Capture):
    def __init__(self, raw, positions):
        require(type(positions) is tuple and positions in ((0, 2047), (2048,), (2302,)), 'closed capture call')
        self.raw, self.positions, self.hidden, self.norm, self.handles = raw, positions, {}, {}, []

    def layer(self, index):
        R.uint(index, 35)
        def observe(_module, _arguments, output):
            require(index not in self.hidden, 'duplicate actual layer hook')
            tensor = output[0] if isinstance(output, (tuple, list)) else output
            self.hidden[index] = {p: self.raw(tensor, p) for p in self.positions}
        return observe

    def norm_input(self, _module, arguments):
        require(len(arguments) >= 1 and not self.norm, 'one actual final norm input')
        self.norm = {p: self.raw(arguments[0], p) for p in self.positions}

    def payload(self, position, final, logits):
        require(position in self.positions and set(self.hidden) == set(range(36))
                and self.norm == self.hidden[35], '36 actual hidden slices and final norm join')
        raw = b''.join(self.hidden[i][position] for i in range(36)) + final + logits
        D.split_payload(raw)
        return raw


def output_bodies(ordinal, generated, decode):
    require(type(ordinal) is int and ordinal in (1, 2)
            and type(generated) is list and len(generated) == OUTPUTS, 'two full output passes')
    for value in generated:
        token(value)
    decoded, preserved = decode(generated, True), decode(generated, False)
    require(type(decoded) is bytes and type(preserved) is bytes
            and len(decoded) <= DECODED_CAP and len(preserved) <= DECODED_CAP, 'bounded raw decoded bytes')
    return {'tokens.u32le': struct.pack('<256I', *generated),
            'decoded.bin': decoded, 'decoded-preserve-special.bin': preserved}


def validate_pass(value, tokens, payloads, outputs, decode=None):
    fields = {'ordinal', 'fresh_cache', 'framework_calls', 'positions_processed', 'generated_tokens',
              'prompt_tokens', 'cases', 'captures', 'logit_stream', 'logit_pin_chain_sha256', 'output_pins'}
    require(type(value) is dict and set(value) == fields and value['fresh_cache'] is True
            and type(value['ordinal']) is int and value['ordinal'] in (1, 2)
            and type(value['framework_calls']) is int and value['framework_calls'] == CALLS
            and type(value['positions_processed']) is int and value['positions_processed'] == POSITIONS,
            'closed two-pass generated reference shape')
    input_ids(tokens, 0, [])
    require(value['prompt_tokens'] == compact(struct.pack('<2048I', *tokens)), 'all2048 prompt token binding')
    generated, cases = value['generated_tokens'], value['cases']
    require(type(generated) is list and len(generated) == OUTPUTS and type(cases) is list
            and len(cases) == CALLS, '256 own outputs and model calls')
    for value_token in generated:
        token(value_token)
    for step, case in enumerate(cases):
        ids = input_ids(tokens, step, generated[:step])
        require(type(case) is dict and set(case) == {'step', 'position', 'input_token', 'input_length',
                'cache_before', 'cache_after', 'greedy'}, 'closed generated call record')
        expected = dict(step=step, position=INPUTS - 1 + step, input_token=ids[-1],
                        input_length=len(ids), cache_before=0 if step == 0 else INPUTS - 1 + step,
                        cache_after=INPUTS + step)
        require(all(type(case[k]) is int and case[k] == v for k, v in expected.items()),
                'exact own-history recurrence and actual cache extent')
        validate_summary(case['greedy'])
        require(case['greedy']['token_id'] == generated[step], 'committed choice from own logits')
    R.pin(value['logit_stream'], OUTPUTS * VOCABULARY * 2)
    chain = hashlib.sha256(b''.join(bytes.fromhex(c['greedy']['logits']['sha256']) for c in cases)).hexdigest()
    require(value['logit_pin_chain_sha256'] == chain, 'ordered 256-row hash chain')
    require(type(outputs) is dict and set(outputs) == {'tokens.u32le', 'decoded.bin', 'decoded-preserve-special.bin'}
            and all(type(b) is bytes for b in outputs.values())
            and outputs['tokens.u32le'] == struct.pack('<256I', *generated)
            and all(len(outputs[n]) <= DECODED_CAP for n in ('decoded.bin', 'decoded-preserve-special.bin'))
            and value['output_pins'] == {n: compact(raw) for n, raw in outputs.items()}, 'exact bounded output bytes')
    if decode is not None:
        require(outputs == output_bodies(value['ordinal'], generated, decode), 'authentic raw-byte decoding')
    for name, raw in outputs.items():
        R.pin(value['output_pins'][name], len(raw))
    captures = value['captures']
    require(type(payloads) is dict and set(payloads) == set(SELECTED)
            and all(type(k) is int for k in payloads) and type(captures) is list
            and len(captures) == 4, 'four exact Full2303 selected captures')
    for position, capture in zip(SELECTED, captures):
        step = max(0, position - INPUTS + 1)
        require(type(capture) is dict and set(capture) == {'position', 'forward_call', 'generated_index',
                'input_token', 'cache_length_at_call', 'greedy', 'payload', 'tensors', 'cache_hashes'}, 'closed full capture')
        require(type(capture['position']) is int and capture['position'] == position
                and type(capture['forward_call']) is int and capture['forward_call'] == step
                and type(capture['input_token']) is int
                and capture['input_token'] == (tokens[position] if position < INPUTS else generated[step - 1])
                and type(capture['cache_length_at_call']) is int
                and capture['cache_length_at_call'] == INPUTS + step, 'matrix-prefill versus decode capture chronology')
        require(capture['generated_index'] is None if position == 0 else
                type(capture['generated_index']) is int and capture['generated_index'] == step,
                'position0 is diagnostic, not generated output')
        raw = payloads[position]
        rows = D.split_payload(raw)
        validate_summary(capture['greedy'])
        require(capture['payload'] == compact(raw)
                and capture['tensors'] == {n: compact(b) for n, b in rows.items()}
                and capture['greedy'] == summarize(rows['logits']), 'full finite capture/argmax/role byte join')
        if position != 0:
            require(capture['greedy'] == cases[step]['greedy'], 'selected generated row identity')
        kv = capture['cache_hashes']
        require(type(kv) is list and len(kv) == 36, '36 actual own cache pairs')
        for pair in kv:
            require(type(pair) is dict and set(pair) == {'key', 'value'}, 'closed KV roles')
            for row in pair.values():
                R.pin(row, 2 * 8 * (INPUTS + step) * 128)
    require(captures[0]['cache_hashes'] == captures[1]['cache_hashes'],
            'both prefill slices observe the same actual post-prefill cache')
    require(len(encoded(value)) <= PASS_CAP, 'compact pass record cap')


def repeat_gate(passes, tokens, payloads, outputs, decode=None):
    require(type(passes) is list and len(passes) == len(payloads) == len(outputs) == 2, 'two fresh full passes')
    for ordinal, (value, raw, out) in enumerate(zip(passes, payloads, outputs), 1):
        validate_pass(value, tokens, raw, out, decode)
        require(value['ordinal'] == ordinal, 'ordered independent passes')
    left, right = [dict(value, ordinal=0) for value in passes]
    return left == right and payloads[0] == payloads[1] and outputs[0] == outputs[1]


def evidence_bound(passes, payloads, outputs):
    total = sum(len(encoded(p)) for p in passes) + sum(len(raw) for group in payloads for raw in group.values())
    total += sum(len(raw) for group in outputs for raw in group.values())
    require(total <= 12 << 20, 'closed pass/capture/output aggregate budget')
    # A terminal embeds both pass documents; four implementation files retain an additional fixed allowance.
    require(total + sum(len(encoded(p)) for p in passes) + (4 << 20) <= OUTPUT_CAP,
            'terminal plus implementation allowance within existing owner output budget')
    return total
