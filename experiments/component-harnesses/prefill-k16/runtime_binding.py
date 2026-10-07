"""Exact existing 55c runtime qualification; separate from LLVM image compilation."""
import hashlib
from pathlib import Path

WORKER_SHA = 'af246e5b872b639b90906336493629eb2bbfa90ff57ffc4ffd859f8b963d9b30'
WORKER_BYTES = 2097272
REVISION = '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
STATUS_SHA = '9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa'
PINS = {
    'identity': 'a3e4c446021373c3e9054fb532f2dab1a8937fa8fa4717b41480b34a952fb56c',
    'source-roster': 'de4683bf5fe2097b23c3583c1f27edb0b41fc9545cb00401a379501798474443',
    'source-archive': 'a9772e55f9a140d0749a621c07ea2c9cb19c36b2416656278dd8d2a2095321d3',
    'full-result': 'ac680b1674d5b7bfe1617c8cb8cbdcfca9132e9c03f86d5aa45556f2cb049bd1',
    'full-stdout': '88178615f10980c1bf93789d4b577b8f5e361ea3d1903b05018acff9fc0a17f7',
    'full-stderr': 'cc97b8f0673f912e99134ef0dc806ee1504c682d9b4517a1293eb49cf031d7c1',
    'clippy-result': 'f3d62ffa2a4b2e60d9ff54c5e203c0c95bd4a1dc2372e60c39b76de4f818c162',
    'clippy-stdout': 'd8ce17884a619192d112420fc7d8952283fbaa42e0b2cae78a2bc65c261c0b0e',
    'clippy-stderr': '6b2fcb48a2dfe70afd0d086e3983f66426b60b3cce617f6077c751f9ad3f1060',
    'default-result': 'c75cb4b389e6385f84417f1c9cc806842c7af29799e26d53161aec4525f0dfe2',
    'default-stdout': 'dd7bde88fddc5d0a63687d75f362662eebd85e147b0d58d2524831be76438afe',
    'default-stderr': '8604afd9033be34efd73632b970f10bbaa4814ac37f72a70f4d4ebd59c1bf02c',
    'release-result': 'f585f0937923a8d8c57819091f8fffef36b9dc269fb1bc2c11e446cf87c28afc',
    'release-stdout': '1921d22390fe267162451795e63ecfe25c6155a52f09de690ad79123989e7c6c',
    'release-stderr': '0a90f8883155e07940ae89f0708e4324ac1ccec087caa2be4918315b6075ed82',
    'aql-result': '5a13e400713e8c27bde64430d7269e0156dcd1b590dce4ce9562ac714496bc74',
    'aql-stdout': '81bed99a36b1b6e566621bb8a9e4ad3cdae110c7bf92bb348c6094f5a3f36738',
    'aql-helper': 'b2b416e86e7b1cfe90a6da4cf3b85f4f8c51a63dbaa32a586620285b09d11d4f',
    'aql-inner': '4d9084f49c293950216ac817d431d687e564878848c18eecd4df7ad3fbe2b929',
} | {role + '-status': STATUS_SHA for role in ('full', 'clippy', 'default', 'release', 'aql')}


def validate_review(evidence, plan, *, read, staged_binding, decode, require):
    require(type(evidence) is dict and set(evidence) == set(PINS), 'exact runtime evidence roster')
    require(plan['worker']['sha256'] == WORKER_SHA
            and plan['files']['worker-candidate']['bytes'] == WORKER_BYTES,
            'current qualified runtime binary identity')
    for name, expected in PINS.items():
        item = staged_binding(evidence[name], plan)
        require(item['sha256'] == expected
                and item['path'] == str(Path(plan['stage']) / 'runtime' / name),
                'fixed existing runtime evidence identity')
        raw = read(item['path'], expected, 64 * 1024**2)[0]
        require(hashlib.sha256(raw).hexdigest() == expected, 'runtime evidence bytes')
        if name.endswith('-status'):
            require(raw == b'0\n', 'runtime qualification failed')
        if name.endswith('-result'):
            result = decode(raw)
            clean = {'status': 0, 'reason': 'completed', 'returncode': 0,
                'cleanup_ok': True, 'child_reaped': True, 'errors': [],
                'term_sent': False, 'kill_sent': False, 'log_limit_exceeded': False}
            require(all(type(result.get(key)) is type(value) and result[key] == value
                        for key, value in clean.items()), 'runtime clean terminal qualification')
        if name == 'full-stdout':
            require(b'test result: ok. 734 passed; 0 failed; 3 ignored;' in raw
                    and b'test result: ok. 9 passed; 0 failed; 0 ignored;' in raw,
                    'complete current runtime and worker test results')
