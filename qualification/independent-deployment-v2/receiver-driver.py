"""Recheck actual transported compiler/native evidence on MI350 without execution."""
import hashlib
import json
import os
from pathlib import Path
import resource
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-independent-deployment-v2'
DEPLOYMENT = E / 'prefix-independent-deployment-v228-v1'
OUT = E / 'independent-deployment-verification-v228-v1'
MANIFEST_SHA = '63ff8c1820c12f9863773d77d5a5652a4e02c21340b32128455be292ab95a0fa'
DRIVER = E / 'p228-independent-runtime-audit-v1/run_row_facts_v2.py'
DRIVER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'


def main():
    if sys.flags.optimize or not sys.dont_write_bytecode:
        raise RuntimeError('unoptimized Python without bytecode required')
    assert os.uname().nodename == 'smci350-rck-g03-b19-03' and os.getuid() == 9661
    assert os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
    assert resource.getrlimit(resource.RLIMIT_AS) == (2 << 30, 2 << 30)
    raw = DRIVER.read_bytes()
    assert DRIVER.resolve(strict=True) == DRIVER and hashlib.sha256(raw).hexdigest() == DRIVER_SHA
    D = types.ModuleType('transport_verify_driver')
    D.__file__ = str(DRIVER)
    exec(compile(raw, str(DRIVER), 'exec'), D.__dict__)
    pins = D.Pins()
    pins.pin(DRIVER, DRIVER_SHA)
    manifest, package_pin = pins.json(P / 'manifest.json', MANIFEST_SHA)
    assert len(manifest['files']) == 14 and manifest['pure_tests'] == 30
    for row in manifest['files']:
        record = pins.pin(P / row['path'], row['sha256'])
        assert record['bytes'] == row['bytes']
    reader = D.load_module(pins, P / 'portable.py',
        '0901e10dbcf40e940a98f917cc6bf7b74c4c5c5f8b76eb1af492e672a11fab00', 'transport_verify')
    value, deployment_pin = pins.json(DEPLOYMENT / 'deployment.json',
        'fb73435a16d3be7d1069a776270e2901b71d31fa352ae3cec352832bbea058aa')
    start = time.monotonic()
    _, verified = reader.verify(D, pins, value, DEPLOYMENT)
    pins.recheck()
    OUT.mkdir(mode=0o700)
    body = dict(schema='ferric-p228-independent-deployment-verification-v1', passed=True,
        host=os.uname().nodename, package_manifest=package_pin, deployment=deployment_pin,
        controller=pins.pin(Path(__file__).resolve()), verified=verified,
        source_pins=dict(pins.records), elapsed_host_seconds=time.monotonic() - start,
        authority='none', gpu_execution=False, native_execution=False,
        numerical_acceptance=False, full_model_acceptance=False, production_authority=False)
    with (OUT / 'complete.json').open('x', encoding='ascii') as stream:
        json.dump(body, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(dict(passed=True, complete=pins.pin(OUT / 'complete.json'))), flush=True)


if __name__ == '__main__':
    main()
