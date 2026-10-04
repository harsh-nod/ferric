"""New paired-worker selection; frozen runtime auditor and ownership unchanged."""
import os
from pathlib import Path
import re
import resource
import shutil
import sys

import intake as I
import bank_batch_portable as B

AUDITOR = ('p227-prefix-runtime-audit-v2', '9b80913aa0dac2da2a52bfe6e053a4db09a5e4c5647731fd269f39719e8bb062')
AUDIT_SHA = 'def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514'


def selected(args, pins):
    I.V.require(len(args) == 4, 'LABEL ROLE DEPLOYMENT_PATH DEPLOYMENT_SHA')
    label, role, path, digest = args
    I.V.require(role in ('parent', 'worker') and re.fullmatch(
        'state-bank-batch-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', label), 'closed role and fresh runtime label')
    record, _ = pins.read(Path(path), digest, maximum=16 << 20)
    _value, runtime = B.deployment(I.D, pins, record, I.historical_deployment)
    return I.E / label, dict(runtime[role])


def main(args):
    I.V.require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
        and os.getuid() == os.geteuid() == 9661 and os.sched_getaffinity(0) == {8, 9}
        and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'same bounded runtime-audit identity')
    I.V.require(shutil.disk_usage(I.R).free >= 40 << 30, 'existing disk floor')
    os.umask(0o077)
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 64 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (limit, limit))
    pins = I.D.Pins()
    B.checked_package(I, pins)
    out, binary = selected(args, pins)
    I.V.require(not os.path.lexists(out), 'fresh output')
    base = I.D.package(pins, *AUDITOR)
    audit = I.D.load_module(pins, base / 'audit.py', AUDIT_SHA, 'layer_runtime_auditor')
    audit.host_identity()
    pins.pin(Path(__file__).resolve()); pins.pin(Path(I.__file__).resolve())
    audit.read_pin(pins, binary, 1 << 30)
    path = Path(binary['path'])
    with path.open('rb') as stream:
        I.V.require(stream.read(6) == b'\x7fELF\x02\x01', 'ELF64 little-endian preserved binary')
    I.V.require(os.access(path, os.X_OK), 'actual preserved executable mode')
    topology = I.D.load_module(pins, audit.TOPOLOGY, audit.TOPOLOGY_SHA, 'layer_runtime_topology')
    owned = I.D.load_module(pins, Path(__file__).with_name('frozen_owned.py'), I.OWNED_SHA, 'layer_runtime_owned')
    record = audit.execute(out, binary, pins, owned, topology)
    I.D.progress(dict(complete=record, gpu_execution=False, reviewed=False))


if __name__ == '__main__':
    main(sys.argv[1:])
