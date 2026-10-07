"""Exact owned-container fallback for an outer HTTP supervisor."""
import json
import re


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validate_intent(intent):
    require(type(intent) is dict and set(intent) == {'name', 'image', 'label_key', 'label_value'},
            'closed predeclared container intent')
    require(type(intent['name']) is str and re.fullmatch(r'ferric-matched128-(vllm|sglang)-[0-9a-f]{16}', intent['name'])
            and intent['label_key'] == 'ferric.matched128' and intent['label_value'] == intent['name']
            and re.fullmatch(r'sha256:[0-9a-f]{64}', intent['image']), 'exact owned container name/image/label')


def observe(value, intent, expected=None):
    validate_intent(intent)
    require(type(value) is list and len(value) == 1, 'one exact owned container inspect record')
    row = value[0]
    require(type(row.get('Id')) is str and re.fullmatch(r'[0-9a-f]{64}', row['Id'])
            and row.get('Name') == '/' + intent['name'] and row.get('Image') == intent['image']
            and row.get('Config', {}).get('Labels', {}).get(intent['label_key']) == intent['label_value']
            and type(row.get('Created')) is str and row['Created'], 'foreign or ambiguous container identity')
    identity = {**intent, 'id': row['Id'], 'created': row['Created']}
    require(expected is None or identity == expected, 'container replaced before cleanup')
    return identity


def list_owned(command, intent, label):
    validate_intent(intent)
    argv = ['docker', 'container', 'ls', '-a', '--no-trunc', '--filter', 'name=^/' + intent['name'] + '$',
            '--format', '{{json .}}']
    status, stdout, stderr = command(argv, label, timeout=5, limit=65536)
    require(status == 0 and not stderr and len(stdout) <= 65536, 'bounded container absence query failed')
    rows = [json.loads(line) for line in stdout.splitlines()]
    require(len(rows) <= 1 and all(row.get('Names') == intent['name']
        and re.fullmatch(r'[0-9a-f]{64}', row.get('ID', '')) for row in rows), 'container list is not exact')
    return rows


def retire(command, intent, retained=None):
    """Never signals discovered host PIDs or mutates a container by name.

    intent is already bound by the immutable launch plan before Docker create.
    retained is the exact post-create identity, when construction reached it.
    The normal and outer-hard-KILL fallback both use this same closed path.
    """
    validate_intent(intent)
    rows = list_owned(command, intent, 'container-retirement-before')
    if not rows:
        return {'schema': 'FerricNativeHttpContainerRetirementV1', 'absent': True,
                'retained_identity': retained, 'stop_sent': False, 'remove_sent': False}
    status, stdout, stderr = command(['docker', 'inspect', rows[0]['ID']], 'container-retirement-inspect',
                                    timeout=5, limit=1024**2)
    require(status == 0 and not stderr, 'container cleanup identity unavailable')
    identity = observe(json.loads(stdout), intent, retained)
    require(identity['id'] == rows[0]['ID'], 'container list/inspect identity changed')
    # Recheck immutable identity immediately before each mutation. The full ID
    # is not reused when a name is replaced; foreign replacement stays untouched.
    status, _, stderr = command(['docker', 'stop', '--time', '30', identity['id']],
                                'container-retirement-stop', timeout=35, limit=65536)
    require(status == 0 and not stderr, 'owned container stop failed')
    status, stdout, stderr = command(['docker', 'inspect', identity['id']],
                                    'container-retirement-stopped', timeout=5, limit=1024**2)
    require(status == 0 and not stderr, 'stopped container identity unavailable')
    value = json.loads(stdout)
    observe(value, intent, identity)
    require(value[0].get('State', {}).get('Running') is False, 'owned container remains running')
    status, _, stderr = command(['docker', 'rm', identity['id']], 'container-retirement-remove',
                                timeout=10, limit=65536)
    require(status == 0 and not stderr, 'owned container removal failed')
    require(not list_owned(command, intent, 'container-retirement-after'), 'container name remains after removal')
    return {'schema': 'FerricNativeHttpContainerRetirementV1', 'absent': True,
            'retained_identity': identity, 'stop_sent': True, 'remove_sent': True}
