"""Data-only export; retain CPU633 parent in place and relocate only CPU475 worker."""
import os
from pathlib import Path
import re
import shutil
import sys

import intake as I
import group_fence_portable as G
import policy_portable as P


def records(cpu_pin, cpu, review):
    rows = [cpu_pin, review, *cpu['inputs'], *cpu['raw'].values()]
    rows += [row['binary'] for row in cpu['binaries'].values()]
    result = {}
    for row in rows:
        P.pin(row)
        I.V.require(result.setdefault(row['path'], row) == row, 'no conflicting original identity')
    I.V.require(len(result) == 90, 'closed actual worker courier census')
    return result


def main(args):
    I.V.require(len(args) == 7 and not sys.flags.optimize,
                'CPU_PATH CPU_SHA REVIEW_PATH REVIEW_SHA FRESH_LABEL BASE_PATH BASE_SHA')
    I.V.require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b',
                'build-host data export only')
    I.V.require(shutil.disk_usage(I.R).free >= 40 << 30, 'shared-host space floor')
    I.V.require(re.fullmatch(r'group-fence-deployment-v228-v[1-9][0-9]{0,8}', args[4]), 'fresh delta namespace')
    os.umask(0o077)
    pins = I.D.Pins()
    cpu_pin, raw = pins.read(Path(args[0]), args[1], True, 16 << 20)
    review, _ = pins.read(Path(args[2]), args[3], True, 65536)
    base_pin, base_raw = pins.read(Path(args[5]), args[6], True, 16 << 20)
    I.V.require(cpu_pin['sha256'] == G.CPU_SHA and base_pin['sha256'] == G.BASE_SHA, 'frozen actual cohorts')
    cpu, base = I.D.parse(raw), I.D.parse(base_raw)
    old_runtime = P.verify(I.D, pins, base, Path(base_pin['path']).parent, I.historical_deployment)
    required = records(cpu_pin, cpu, review)
    bodies = {}
    for row in required.values():
        I.V.require(Path(row['path']).is_relative_to(I.R), 'task-owned source evidence only')
        bodies[row['sha256']] = I.read(pins, row, P.MAX_FILE)
    I.V.require(sum(map(len, bodies.values())) <= P.MAX_TOTAL, 'bounded courier bytes')
    pins.recheck()
    out = I.E / args[4]
    out.mkdir(mode=0o700)
    (out / 'objects').mkdir(mode=0o700)
    (out / 'bin').mkdir(mode=0o700)
    for sha, body in sorted(bodies.items()):
        with (out / 'objects' / sha).open('xb') as stream:
            stream.write(body)
            stream.flush()
            os.fsync(stream.fileno())
    aliases = {row['path']: dict(original=row, relocated=pins.pin(out / 'objects' / row['sha256'], row['sha256']))
               for row in required.values()}
    original_worker = cpu['binaries'][P.WORKER]['binary']
    worker = out / 'bin' / P.WORKER
    with worker.open('xb') as stream:
        stream.write(bodies[original_worker['sha256']])
        stream.flush()
        os.fsync(stream.fileno())
    worker.chmod(0o700)
    runtime = dict(old_runtime, worker=pins.pin(worker, G.WORKER_SHA))
    value = dict(schema=G.SCHEMA, base_deployment=base_pin, worker_cpu=cpu_pin,
                 worker_cpu_review=review, aliases=aliases, runtime=runtime)
    G.verify(I.D, pins, value, out, I.historical_deployment)
    I.V.require(shutil.disk_usage(I.R).free >= 38 << 30, 'retained free-space floor')
    I.D.progress(dict(deployment=I.save(out / 'complete.json', value), gpu_execution=False, production_authority=False))


if __name__ == '__main__':
    main(sys.argv[1:])
