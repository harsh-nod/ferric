"""Local root input assembly only; no imports of tested modules or workload launch."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
COMPARISON_SHA = '758c2479a3f5a5f78e6ff088adb70a31ca5fb19c86e5a25cf718c3e1e46f4dab'
PURE_SHA = '20a16b03d188e2fb4e68d1d44a901ba6e0a73863b5a2402f152c7521f2e02516'
SOURCES = {
    E / 'p228-output-residual-boundary-v1/boundary.py': 'a5fac7b587739999623167d81c163fc18ebe9a2baf986da1b192b597e7fb0b9d',
    E / 'p228-output-residual-boundary-v1/test_boundary.py': '2088ec73ed3763647bebc3cf295364b2a029d2c59ca3937d95df1430932d39fb',
    E / 'p228-independent-layer-reference-v1/helpers/residual_oracle.py':
        '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3',
}
SEEN = {}


def require(ok, message):
    if not ok: raise ValueError(message)


def read(path, sha=None):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical local retained input')
    raw = path.read_bytes(); require(len(raw) <= 16 << 20, 'bounded local data')
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    require(sha is None or pin['sha256'] == sha, 'actual root-supplied input hash')
    require(SEEN.setdefault(path, pin) == pin, 'unchanged local input')
    return raw, pin


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON member'); value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def local(remote):
    relative = remote.relative_to(E)
    return L / 'proposals' / relative if relative.parts[0].startswith('p228-') else L / relative


def main(args):
    require(len(args) == 3 and not sys.flags.optimize and sys.dont_write_bytecode
        and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary -B PURE_PATH PURE_SHA VERSION invocation')
    pure_path, pure_sha, version = Path(args[0]), args[1], args[2]
    require(pure_path == L / 'output-residual-boundary-pure-v228-v1/complete.json' and pure_sha == PURE_SHA
        and re.fullmatch('[1-9][0-9]{0,8}', version), 'explicit actual receipt and fresh output version')
    raw, comparison = read(L / 'layer0-current-comparison-v228-v1/complete.json', COMPARISON_SHA)
    prior = parse(raw)
    require(comparison['bytes'] == 180610 and prior['completed'] is True and prior['source_postchecks_passed'] is True
        and prior['numerical_acceptance'] is False and len(prior['consumed']) == 260, 'actual comparison checkpoint')
    raw, pure_pin = read(pure_path, pure_sha); pure = parse(raw)
    require(pure_pin['bytes'] == 1500 and pure['schema'] == 'ferric-p228-output-residual-boundary-pure-v1' and pure['passed'] is True
        and pure['tests'] == 12 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True,
        'actual passing boundary12 receipt')
    before = after = None
    for key in ('sources_before', 'sources_after', 'transcript', 'controller'):
        pin = pure[key]; source = local(Path(pin['path'])); data, actual = read(source, pin['sha256'])
        require(actual['bytes'] == pin['bytes'], 'actual pure record extent')
        if key == 'sources_before': before = parse(data)
        if key == 'sources_after': after = parse(data)
    require(before == after and set(before) == {'boundary.py', 'test_boundary.py', 'residual_oracle.py'},
        'unchanged exact tested source roster')
    for pin in before.values():
        remote = Path(pin['path']); require(SOURCES.get(remote) == pin['sha256'], 'selected boundary/oracle generation')
        _, actual = read(local(remote), pin['sha256']); require(actual['bytes'] == pin['bytes'], 'tested source extent')
    out = L / ('output-residual-boundary-inputs-v228-v' + version)
    require(not os.path.lexists(out), 'fresh local input assembly')
    plan = dict(schema='ferric-p228-output-residual-boundary-inputs-v1',
        comparison=dict(comparison, path=str(E / 'layer0-current-comparison-v228-v1/complete.json')),
        boundary_tests=dict(pure_pin, path=str(E / pure_path.relative_to(L))),
        output_label='output-residual-boundary-v228-v' + version)
    for path, pin in list(SEEN.items()): require(read(path)[1] == pin, 'input postcheck')
    out.mkdir(mode=0o700)
    for name, value in (('plan.json', plan), ('assembly.json', dict(
        schema='ferric-p228-output-residual-boundary-assembly-v1', local_input_pins=list(SEEN.values()),
        prior_transport_map_reused_without_new_aliases=True, retained_input_map_entries=260,
        new_model_transfer_required=False, native_execution=False, gpu_execution=False,
        numerical_acceptance=False, production_authority=False))):
        with (out / name).open('x', encoding='ascii') as stream:
            json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False); stream.write('\n')
    _, pin = read(out / 'plan.json')
    print(json.dumps(dict(pin, path=str(E / pin['path'][len(str(L)) + 1:]))), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])
