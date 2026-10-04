"""Conditional residual capture comparison using the unchanged p222 oracle."""

import hashlib
from pathlib import Path
import struct
import types


ELEMENTS = 4096
STAGES = ('post_attention', 'post_mlp')
SCHEMA = 'ferric-p228-independent-tp2-residual-reference-v1'
ORACLE_SHA256 = '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'
FALSE_FIELDS = (
    'input_provenance_verified', 'gpu_execution_verified',
    'upstream_partial_numerics_checked', 'full_layer_numerics_accepted',
    'full_model_acceptance', 'arithmetic_prerequisites_verified',
    'runtime_premises_discharged', 'production_authority', 'performance_claim',
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_oracle(path=None):
    """Authenticate the byte-identical retained helper before executing it."""
    path = Path(__file__).resolve().parent / 'helpers/residual_oracle.py' if path is None else Path(path)
    require(path.is_absolute() and not path.is_symlink()
            and path.resolve(strict=True) == path and path.is_file(), 'canonical oracle file')
    with path.open('rb') as stream:
        raw = stream.read((64 << 10) + 1)
    require(len(raw) <= 64 << 10 and hashlib.sha256(raw).hexdigest() == ORACLE_SHA256,
            'unchanged pinned p222 residual oracle')
    module = types.ModuleType('p228_retained_residual_oracle')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def _pair(value, width):
    require(type(value) in (tuple, list) and len(value) == 2, 'exact ordered rank pair')
    for raw in value:
        require(type(raw) is bytes and len(raw) == ELEMENTS * width,
                'exact 4096-element immutable byte extent')


def compare(stage, partials, residuals, outputs):
    """Check a supplied boundary, not origin/upstream correctness.

    Both TP ranks consume the same partial rows and replicated BF16 hidden
    state. Callers authenticate rank identity and capture-before-scratch-reuse.
    """
    require(type(stage) is str and stage in STAGES, 'closed residual stage')
    _pair(partials, 4)
    _pair(residuals, 2)
    _pair(outputs, 2)
    require(residuals[0] == residuals[1], 'replicated residual inputs differ')
    expected = load_oracle().residual_vector(partials[0], partials[1], residuals[0])
    wanted_words = struct.unpack('<4096H', expected)
    for rank, output in enumerate(outputs):
        for index, (actual, wanted) in enumerate(zip(struct.unpack('<4096H', output), wanted_words)):
            require(actual & 0x7f80 != 0x7f80, 'nonfinite captured residual output')
            require(actual == wanted, f'{stage} rank {rank} element {index}: '
                    f'actual 0x{actual:04x}, expected 0x{wanted:04x}')
    digest = lambda raw: hashlib.sha256(raw).hexdigest()
    return dict(
        schema=SCHEMA, authority='none', stage=stage, elements=ELEMENTS, ranks=2,
        conditional_tp_residual_checks_passed=True, exact_bf16_words=ELEMENTS * 2,
        tolerance='exact BF16 words including signed zero', oracle_sha256=ORACLE_SHA256,
        source_symbol='ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18',
        conditioning=dict(partial_sha256=[digest(raw) for raw in partials],
                          residual_sha256=[digest(raw) for raw in residuals],
                          output_sha256=[digest(raw) for raw in outputs]),
        expected_sha256=digest(expected),
        required_prerequisites=['binary32-rne-gradual-underflow',
                                'separate-ordered-additions-no-reassociation',
                                'bf16-rne-finite-boundaries'],
        **{key: False for key in FALSE_FIELDS},
    )


def compare_stages(input_hidden, prefix_partials, first_residual, mlp_partials, final_hidden):
    """Connect both residual boundaries without checking the intervening MLP."""
    first = compare('post_attention', prefix_partials, input_hidden, first_residual)
    second = compare('post_mlp', mlp_partials, first_residual, final_hidden)
    return dict(
        schema='ferric-p228-independent-two-residual-stages-v1', authority='none',
        conditional_residual_sequence_checks_passed=True,
        oracle_sha256=ORACLE_SHA256, stages=[first, second],
        **{key: False for key in FALSE_FIELDS},
    )
