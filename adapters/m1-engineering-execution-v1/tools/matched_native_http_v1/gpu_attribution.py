"""Acceptance contract for qualified descriptor scans and paired GPU endpoints."""
import json

SCANNER_SHA = '4cc1bb26e5eea7bd235ca4c014fd205d4d9fffdf35d96c1b9a2cad1233cd03e2'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def same(left, right):
    encode = lambda value: json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False)
    return encode(left) == encode(right)


def accept(sample, before, after, *, phase, identities, container=None):
    """Does not run commands or reinterpret a refused scan as an accepted one.

    Endpoint acquisition and process/container ownership must be independently
    bound by the outer caller. Empty host fuser is not GPU inactivity evidence.
    """
    require(phase in ('preflight', 'startup', 'active', 'postflight'), 'closed lifecycle phase required')
    require(type(identities) is list and all(type(row) is dict and type(row.get('pid')) is int
        and row['pid'] > 1 for row in identities), 'exact observed owner lifetimes required')
    allowed = {row['pid']: row for row in identities}
    require(len(allowed) == len(identities), 'duplicate owned process identity')
    expected = {'schema': 'FerricDeviceDescriptorSampleV2', 'method': 'proc-fd-rdev-finite-roster-v1',
        'sampling_policy': 'initial-plus-one-birth-frontier-v1', 'root_visibility': True,
        'accepted': True, 'complete': True, 'phase': phase,
        'errors': [], 'foreign_users': []}
    require(all(type(sample.get(key)) is type(value) and same(sample[key], value)
                for key, value in expected.items()), 'qualified complete privileged descriptor scan required')
    require(type(sample.get('started_monotonic_ns')) is int
            and type(sample.get('completed_monotonic_ns')) is int
            and sample['completed_monotonic_ns'] > sample['started_monotonic_ns'], 'positive sample interval')
    for endpoint in (before, after):
        require(type(endpoint) is dict and endpoint.get('root_visibility') is True,
                'root-visible external endpoint required')
        observed = []
        for key in ('fuser_pids', 'sysfs_pids'):
            values = endpoint.get(key)
            require(type(values) is list and all(type(pid) is int and pid > 1 for pid in values)
                    and values == sorted(set(values)), 'exact bounded GPU endpoint PID set')
            observed += values
        require(set(observed) <= set(allowed), 'foreign GPU endpoint PID')
        require(same(endpoint.get('owned_identities'), identities), 'owned lifetime changed across scan')
        require(type(endpoint.get('started_ns')) is int and type(endpoint.get('finished_ns')) is int
                and endpoint['finished_ns'] >= endpoint['started_ns'], 'bounded endpoint interval')
    require(before['finished_ns'] <= sample['started_monotonic_ns']
            and sample['completed_monotonic_ns'] <= after['started_ns'], 'external endpoints must bracket scan')
    users = sample.get('device_users')
    require(type(users) is list and same(users, sample.get('owned_users')), 'all positive descriptors must be owned')
    for user in users:
        identity = user.get('identity', {})
        require(allowed.get(identity.get('pid')) == identity and user.get('descriptors'),
                'positive descriptor must bind the exact owner lifetime')
    if phase == 'active':
        require(users and identities, 'active GPU attribution requires positive owned descriptors')
    if sample.get('native_owned_fd_rescan') is not None:
        require(sample['native_owned_fd_rescan'] is True and phase == 'active' and container is None
                and len(allowed) == 1 and sample.get('process_retry_policy') ==
                    'native-owned-complete-rescan-fd-enoent-v1', 'closed native FD rescan policy required')
        completed = sample.get('completed_owned_users')
        require(type(completed) is list and len(completed) == 1 and completed[0] in users
                and completed[0]['identity'] == identities[0] and completed[0]['descriptors'],
                'fresh complete positive native scan required')
        require(all(endpoint['fuser_pids'] == endpoint['sysfs_pids'] == sorted(allowed)
                    for endpoint in (before, after)), 'same positive native endpoints required')
    if phase not in ('active', 'startup'):
        require(not users and not identities and not before['fuser_pids'] and not before['sysfs_pids']
                and not after['fuser_pids'] and not after['sysfs_pids'], 'empty lifecycle endpoints required')
    if container is not None:
        require(type(container) is dict and set(container) == {'binding', 'state', 'host_pids'}
                and container['host_pids'] == sorted(allowed)
                and same(sample.get('container'), container)
                and same(before.get('container'), container) and same(after.get('container'), container),
                'exact unchanged container ID/image/label/membership/start required')
    else:
        require('container' not in sample and 'container' not in before and 'container' not in after,
                'unexpected container composition')
    return {'schema': 'FerricNativeHttpGpuAttributionV1', 'accepted': True,
        'phase': phase, 'descriptor_positive': bool(users), 'host_fuser_empty': not before['fuser_pids'],
        'timing_admitted': False,
        'scanner_sha256': SCANNER_SHA,
        'scope': 'finite interval samples with paired external endpoints; not continuous isolation'}
