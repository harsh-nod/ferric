"""Exact qualified capture-admission function; called only on authenticated data."""
import hashlib
import struct

IDS = [16366993098680759275, 10838076764495710945]

def require(ok, message):
    if not ok:
        raise ValueError(message)


def capture_admission(summary_raw, read_body, validator):
    """Additional capture checks after the ordinary model validator accepts this summary."""
    summary = validator.parse(summary_raw)
    pin = validator.rust_pin(summary['files']['child_stderr'])
    raw = read_body(pin)
    require(0 < len(raw) <= 1100000 and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'nonempty exact bounded capture stderr')
    envelope = validator.parse(raw)
    validator.keys(envelope, 'schema profile_sha256 registration_sha256 session device_ids '
                   'completed_forwards native_closed sampling capture numerical_acceptance '
                   'performance_claim production_authority')
    require(envelope['schema'] == 'FerricFiniteGuardedMlpLayerZeroCaptureV1'
            and validator.octets(envelope['profile_sha256']) == validator.octets(summary['profile_sha256'])
            and validator.octets(envelope['registration_sha256']) == validator.octets(summary['registration_sha256'])
            and validator.octets(envelope['session']) == validator.octets(summary['request']['decode']['session'])
            and envelope['device_ids'] == IDS and all(type(v) is int for v in envelope['device_ids'])
            and validator.uint(envelope['completed_forwards']) == 4 and envelope['native_closed'] is True
            and envelope['sampling'] == 'prefix-boundaries-and-post-paired-retained'
            and all(envelope[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority')),
            'closed capture identity, completed workload and nonclaims')
    capture = envelope['capture']
    validator.keys(capture, 'schema generation position layer parts payload_bytes payload_sha256 payload '
                   'full_cache_capture native_close_confirmed numerical_acceptance performance_claim production_authority')
    require(capture['schema'] == 'FerricFiniteLayerZeroCaptureV1'
            and [validator.uint(capture[k]) for k in ('generation', 'position', 'layer')] == [1, 0, 0]
            and type(capture['parts']) is list and len(capture['parts']) == 34
            and validator.uint(capture['payload_bytes']) == 256136
            and capture['native_close_confirmed'] is True
            and all(capture[k] is False for k in ('full_cache_capture', 'numerical_acceptance',
                                                'performance_claim', 'production_authority')),
            'exact capture layout and lifetime scope')
    payload = validator.octets(capture['payload'], 256136)
    require(hashlib.sha256(payload).digest() == validator.octets(capture['payload_sha256']), 'whole capture digest')
    first = summary['files']['frames'][0]
    request = validator.parse(read_body(validator.rust_pin(first['request'])))
    observation = read_body(validator.rust_pin(first['observation']))
    command = request['command']
    metadata = struct.pack('<145I', *command['cache_metadata'])
    rotary = struct.pack('<128I', *command['rotary_bits'])
    require(request['id'] == command['generation'] == 1 and command['cache_metadata'][0] == 0
            and len(observation) == validator.PAYLOAD_BYTES, 'already validated first-forward capture join')
    stages = (
        ('before_prefix', (('input', 'bf16', 8192), ('cache_metadata', 'u32', 580), ('rotary', 'f32', 512))),
        ('after_prefix', (('input_normalized', 'bf16', 8192), ('raw_qkv', 'bf16', 6144),
                          ('query', 'bf16', 4096), ('current_key', 'bf16', 1024), ('current_value', 'bf16', 1024),
                          ('attention', 'bf16', 4096), ('output_partial', 'f32', 16384))),
        ('after_first_residual', (('first_residual', 'bf16', 8192),)),
        ('after_mlp', (('post_normalized', 'bf16', 8192), ('gate', 'bf16', 12288), ('up', 'bf16', 12288),
                      ('activation', 'bf16', 12288), ('down_partial', 'f32', 16384))),
        ('after_final_residual', (('final_hidden', 'bf16', 8192),)),
    )
    offset = ordinal = 0
    for boundary, specs in stages:
        for rank in (0, 1):
            for role, scalar, size in specs:
                part = capture['parts'][ordinal]
                validator.keys(part, 'boundary rank role scalar elements offset bytes source_byte_offset sha256')
                width = 2 if scalar == 'bf16' else 4
                source_offset = command['cache_metadata'][1] * 16384 if role in ('current_key', 'current_value') else 0
                require((part['boundary'], validator.uint(part['rank']), part['role'], part['scalar'],
                         validator.uint(part['elements']), validator.uint(part['offset']), validator.uint(part['bytes']),
                         validator.uint(part['source_byte_offset']))
                        == (boundary, rank, role, scalar, size // width, offset, size, source_offset),
                        'closed ordered capture part')
                data = payload[offset:offset + size]
                require(hashlib.sha256(data).digest() == validator.octets(part['sha256']), 'capture part digest')
                if scalar in ('bf16', 'f32'):
                    mask = 0x7f80 if width == 2 else 0x7f800000
                    require(all(int.from_bytes(data[i:i + width], 'little') & mask != mask
                                for i in range(0, size, width)), 'finite captured scalar')
                if role == 'cache_metadata':
                    require(data == metadata, 'capture actual first metadata')
                elif role == 'rotary':
                    require(data == rotary, 'capture actual first rotary')
                elif role == 'final_hidden':
                    require(data == observation[:8192], 'capture actual first layer output')
                offset += size
                ordinal += 1
    require(ordinal == 34 and offset == len(payload), 'complete capture coverage')
    return dict(schema='ferric-guarded-mlp-model-stage-capture-checked-v1', source=pin,
                generation=1, position=0, layer=0, parts=34, payload_bytes=len(payload),
                payload_sha256=hashlib.sha256(payload).hexdigest(), native_close_confirmed=True,
                same_run_request_and_layer_output_joined=True,
                sampling='prefix-boundaries-and-post-paired-retained',
                independent_framework_comparison=False, numerical_acceptance=False,
                performance_claim=False, production_authority=False)
