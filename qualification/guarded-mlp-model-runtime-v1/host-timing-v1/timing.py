"""Decode host wall counters, never GPU durations or overlapping intervals."""
import struct

LAYERS = 36
STATE_BYTES = (2 * 284 + 2 * 548 + 2 * 4) * 4
LAYER_BYTES = STATE_BYTES + 3 * 8 + 4 * 8
CONTROL_BYTES = 2 * 8 + LAYERS * LAYER_BYTES + 3 * 8


def counters(raw):
    if not isinstance(raw, bytes) or len(raw) != CONTROL_BYTES:
        raise ValueError('exact guarded control extent required')
    layers = []
    for layer in range(LAYERS):
        offset = 16 + layer * LAYER_BYTES + STATE_BYTES
        left, right, segment = struct.unpack_from('<3Q', raw, offset)
        layers.append(dict(layer=layer, prefix_rank_host_ns=[left, right],
                           guarded_segment_host_ns=segment))
    # Rank dispatch intervals may overlap. Keep their totals separate; summing
    # them would double-count unknown overlap and cannot yield token latency.
    return dict(embedding_host_ns=list(struct.unpack_from('<2Q', raw)),
                tail_host_ns=list(struct.unpack_from('<3Q', raw, len(raw) - 24)),
                layers=layers,
                prefix_rank_host_ns=[sum(row['prefix_rank_host_ns'][rank]
                                         for row in layers) for rank in range(2)],
                guarded_segments_host_ns=sum(row['guarded_segment_host_ns'] for row in layers),
                gpu_duration_measured=False, overlap_measured=False,
                throughput_measured=False)
