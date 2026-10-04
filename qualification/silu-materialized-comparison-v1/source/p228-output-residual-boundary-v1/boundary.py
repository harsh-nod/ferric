"""Conditional O-projection/residual rounding diagnostics; no process execution."""
import hashlib
import os
from pathlib import Path
import stat
import struct
import types


ELEMENTS = 4096
ORACLE_SHA256 = '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'
SCHEMA = 'ferric-p228-output-residual-boundary-diagnostic-v1'


def require(value, message):
    if not value:
        raise ValueError(message)


def load_oracle(path=None):
    """Reuse the unchanged finite binary32/BF16 integer-RNE implementation."""
    if path is None:
        path = (Path(__file__).resolve().parent.parent
                / 'p228-independent-layer-reference-v1/helpers/residual_oracle.py')
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical pinned oracle')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= 64 << 10, 'bounded regular oracle')
        raw = stream.read((64 << 10) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size
            and hashlib.sha256(raw).hexdigest() == ORACLE_SHA256, 'unchanged retained oracle bytes')
    module = types.ModuleType('retained_output_residual_integer_oracle')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def words(raw, width, name):
    require(type(raw) is bytes and len(raw) == ELEMENTS * width, name + ': exact immutable extent')
    values = struct.unpack('<4096' + ('I' if width == 4 else 'H'), raw)
    mask = 0x7f800000 if width == 4 else 0x7f80
    require(all(value & mask != mask for value in values), name + ': finite values')
    return values


def pair(value, width, name):
    require(type(value) in (tuple, list) and len(value) == 2, name + ': ordered TP2 pair')
    return tuple(words(raw, width, name + '-rank' + str(rank)) for rank, raw in enumerate(value))


def compare_words(name, expected, actual):
    require(len(expected) == len(actual) == ELEMENTS, 'complete comparison extent')
    differences = [index for index, (left, right) in enumerate(zip(expected, actual, strict=True))
                   if left != right]
    encode = lambda values: struct.pack('<4096H', *values)
    return dict(name=name, elements=ELEMENTS, byte_equal=not differences,
        exact_words=ELEMENTS - len(differences), differing_words=len(differences),
        expected_sha256=hashlib.sha256(encode(expected)).hexdigest(),
        actual_sha256=hashlib.sha256(encode(actual)).hexdigest(),
        first_differences=[dict(index=index, expected_bits=expected[index], actual_bits=actual[index])
                           for index in differences[:16]],
        first_differences_limit=16)


def compare(partials, embedding, native_residuals, framework_projection, framework_residual):
    """Compare fixed formulas on supplied captures, not a new genuine model chain.

    The caller authenticates both rank partials and the genuine framework rows.
    No native embedding equality, GEMM correctness, or source/ISA admission is
    inferred from these arguments. Differences remain observations, not errors
    accepted by a tolerance. Malformed or nonfinite inputs/intermediates refuse.
    """
    p0, p1 = pair(partials, 4, 'output-partial')
    skip = words(embedding, 2, 'framework-embedding')
    native = pair(native_residuals, 2, 'native-first-residual')
    projected = words(framework_projection, 2, 'framework-o-projection')
    reference = words(framework_residual, 2, 'framework-first-residual')
    oracle = load_oracle()
    fused, materialized, framework_add, rank_sum_bf16 = [], [], [], []
    for index, (left, right, residual, framework_o) in enumerate(zip(p0, p1, skip, projected, strict=True)):
        try:
            _, rank_sum, _, final = oracle.staged_residual_bits(left, right, residual)
            narrowed = oracle.narrow_bf16_rne(rank_sum)
            rounded_add = oracle.add_f32_rne(narrowed << 16, residual << 16, 'materialized-residual')
            framework_sum = oracle.add_f32_rne(framework_o << 16, residual << 16, 'framework-residual')
            fused.append(final)
            rank_sum_bf16.append(narrowed)
            materialized.append(oracle.narrow_bf16_rne(rounded_add))
            framework_add.append(oracle.narrow_bf16_rne(framework_sum))
        except oracle.FinitePolicyError as error:
            raise ValueError('element ' + str(index) + ': ' + str(error)) from error
    rows = [compare_words('native-formula-vs-native-rank' + str(rank), fused, output)
            for rank, output in enumerate(native)]
    rows.append(compare_words('native-formula-vs-framework-residual', reference, fused))
    rows.extend(compare_words('materialized-formula-vs-native-rank' + str(rank), materialized, output)
                for rank, output in enumerate(native))
    rows.extend((compare_words('materialized-formula-vs-framework-residual', reference, materialized),
                 compare_words('framework-add-vs-framework-residual', reference, framework_add),
                 compare_words('rounded-rank-sum-vs-framework-projection', projected, rank_sum_bf16)))
    pin = lambda raw: dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    return dict(schema=SCHEMA, authority='none', elements=ELEMENTS, ranks=2,
        oracle_sha256=ORACLE_SHA256, conditional_replay_performed=True,
        conditioning=dict(output_partials=[pin(raw) for raw in partials], framework_embedding=pin(embedding),
            native_first_residuals=[pin(raw) for raw in native_residuals],
            framework_o_projection=pin(framework_projection), framework_first_residual=pin(framework_residual)),
        comparisons=rows, acceptance_threshold=None,
        arithmetic_premises=['binary32-rne-gradual-underflow', 'separate-ordered-additions-no-reassociation',
                             'bf16-rne-finite-boundaries'],
        input_provenance_verified=False, native_embedding_equality_verified=False,
        upstream_partial_numerics_checked=False, projection_gemm_equivalence_proven=False,
        causal_explanation_proven=False, genuine_framework_chain_rerun=False,
        arithmetic_prerequisites_verified=False, runtime_premises_discharged=False,
        numerical_acceptance=False, full_layer_numerics_accepted=False, full_model_acceptance=False,
        gpu_execution=False, performance_claim=False, production_authority=False)
