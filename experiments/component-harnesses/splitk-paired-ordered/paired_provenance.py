"""Exact retained engineering-only paired emission, not a new compiler qualification."""
import hashlib
import re


EMITTER = '0974e4cd573db2b0699f4477bbdaa7b616a34c341031b91252179ed1795bbcfa'
PINS = {
    'image.hsaco': '950618ad101779973b4051190266cfdd601329405691cb215296fe61b84278ec',
    'source.tar.gz': '74cd6e02a363475669a06da5b610dda1403e4b7b03077bb5aa4ca24f8c9b7516',
    'source.sha256': 'cd7356417e6668c6549c50e777629df52979858358ba010020cb864d4fdf13b1',
    'observation.json': '40a9a10bd6412a60f3d1f0b003156e8eebda625221b1f19039bcda146c5a15c1',
    'compiler-handoff-v2': '4818112cdbe7feeea55b318af4964bf817cbbc9509bd46c32658abc225ce12b0',
    'capture.json': 'f4db15fab097611bb9c32306b0072467b12313bc4e3be01877758704cd555b8d',
    'disassembly.txt': 'f1b3b5832e33a862c46ab8bb95766c6900796130e2fe6e67fd3d898212b519cb',
    'metadata.txt': '38a5a8aad0fc38577ebb7f86fa3bfc70c2440742f99a418ce51df71800fcc7a1',
}
PHASES = {
    'default-tests': ('c687130e1e470aebd64b3ae17e7c15b4da7dea49a8e6409e061736582061fdd6',
        '1ffc976474c0ff02b29d45425e52f9e3b9e04c657b85a1fabfa1be841a97c289',
        'a40b5f8d0a79163f8cf18d01b9357371dffb1cd5e03e11e121bb5a6105dcd97c'),
    'enabled-tests': ('d8a230cdc952dae61e5b5d9dad8d8f16bdc711fdac74f40d31a658bb9b30bc93',
        'a102c2d69a087e6ce6a5458ec2cdc06a859ebf0295237e85e3052dcdc330da6d',
        '2d57935f3795f49abe48543e39a86793e6371e294091883d084f9d060b7a8dac'),
    'default-clippy': ('fcaef68a15df545ebebf05264081a916f530f3793aa43096c15540a07549cdef',
        'b7074595556a218f9066ce38525df974a7399e9bc7ebdb47517d6bc6fe81b281',
        'c0eb972695ea3a81e8c2369d9097b225b275f9d4b6cdbc1a564e4691c2b9d459'),
    'enabled-clippy': ('a117a0e8ffda3f20edaee13231da20a00207ac0e68fe8562216ba5ccbb41bd03',
        '4b7907734b30c56968c6e19b236497c736e06e21fe47fdab9bf313e18946f625',
        '2ea8a47ecd58be1ac63dfa25574770a10ef6c42092b9022f691b1105488bb4b6'),
    'emission': ('968b9ae48a642a394edc51512972ee6836262e749248c38c62718b470d085743',
        '33f39764c06e0beaa5bffc07b0dc5f638f0126c9a83dcba970cf965e5ead93b3',
        '4bfc3038ad480dee34461b78a43e2d49772d2cc38a0fa68217a366eed20783dc'),
    'isa': ('0bec5d7e76a4a2a0051e1c99de034631eef6ea2a0fcb315468ee3ebf9f77ecbc',
        'a46d63cdb3ff9abf45faabc4a1f73af47db616c560e532648a4adcffe30a6848',
        'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'),
    'retention': ('a83da29d409c2fc0ed71c904deb8804405deb3ef7e36f92c22de8b9c5170ca98',
        '33ca865d5ddaac092709f926539f60c993173ceeb185f195d85513dffdf76b1a',
        'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'),
}
for _role, _values in PHASES.items():
    PINS.update({_role + '/' + name: value for name, value in zip(
        ('result', 'stdout', 'stderr'), _values, strict=True)})
    PINS[_role + '/status'] = '9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa'
EMPTY = {'isa/stderr', 'retention/stderr'}
SYMBOLS = ['ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1',
           'ferric_qwen3_c1_down_splitk8_merge_f32_r1']


def validate_review(evidence, plan, *, read, staged_binding, decode, require):
    require(type(evidence) is dict and set(evidence) == set(PINS), 'closed historical paired evidence')
    raw = {}
    for name, expected in PINS.items():
        item = staged_binding(evidence[name], plan)
        require(item['sha256'] == expected and item['path'] == plan['stage'] + '/paired/' + name,
                'exact retained paired evidence and location')
        raw[name] = read(item['path'], expected, 8 * 1024**2, empty=name in EMPTY)[0]
        require(hashlib.sha256(raw[name]).hexdigest() == expected, 'actual paired evidence bytes')
    require(plan['images']['paired']['sha256'] == PINS['image.hsaco'], 'paired loaded image identity')
    for role in PHASES:
        require(raw[role + '/status'] == b'0\n', 'historical paired raw status must pass')
        result = decode(raw[role + '/result'])
        expected = {'status': 0, 'reason': 'completed', 'returncode': 0,
            'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False,
            'kill_sent': False, 'log_limit_exceeded': False}
        require(all(type(result.get(key)) is type(value) and result[key] == value
                    for key, value in expected.items()), 'clean historical paired phase')
        require(result['profile'] == 'FerricCpuFourCore32GiBEmitterV1'
                and result['cpus'] == [0, 1, 2, 3] and result['nice'] == 19
                and result['build_jobs'] == 4 and result['rust_test_threads'] == 1,
                'actual historical G32 qualification, not relabeled G36')
    for role, expected in (('default-tests', [0, 5, 6, 6, 0]), ('enabled-tests', [0, 6, 6, 6, 0])):
        rows = re.findall(rb'test result: ok\. ([0-9]+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;',
                          raw[role + '/stdout'])
        require([int(value) for value in rows] == expected, 'exact historical paired test footer roster')
    observation = decode(raw['observation.json'])
    require(observation['schema'] == 'EngineeringHsacoObservationV1'
            and observation['authority'] == 'none'
            and observation['grants'] == {'publication': False, 'load': False, 'launch': False}
            and observation['target'] == 'gfx950:xnack-'
            and observation['hsaco']['identity'] == {'sha256': PINS['image.hsaco'], 'byte_len': 13344}
            and sorted(observation['hsaco']['kernel_names']) == sorted(SYMBOLS)
            and observation['compiler_handoff'] == {'sha256': PINS['compiler-handoff-v2'], 'byte_len': 68207}
            and observation['execution']['exact_output_replay'] is True,
            'exact paired observation, symbols, handoff and engineering-only grants')
    unpaired_name = 'images/candidate.observation.json'
    item = plan['files'][unpaired_name]
    require(item['sha256'] == '2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54',
            'exact original unpaired observation')
    unpaired = decode(read(plan['stage'] + '/' + unpaired_name, item['sha256'])[0])
    require(all(observation[key] == unpaired[key] for key in ('tools', 'options', 'providers', 'target')),
            'paired and unpaired use identical historical compiler worker/providers/options')
