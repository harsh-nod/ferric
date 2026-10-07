"""Hash-bound remote-only capture harness CPU test; no GPU or build."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
HERE = Path(__file__).resolve().parent
SOURCES = ('decode_capture.py','run_decode.py','test_decode_capture.py','prepare_capture.py','qualify_capture.py')


def main():
    path = D / 'owner/cpu_profile_42g.py'
    if hashlib.sha256(path.read_bytes()).hexdigest() != '517671a75602c92beb7f3d43a5584be10a4284d15f68033e616a9462ff7fa224':
        raise ValueError('G42 profile pin')
    q = importlib.util.module_from_spec(importlib.util.spec_from_file_location('capture_cpu',path))
    q.__spec__.loader.exec_module(q)
    env = q.environment()
    before = q.allocation(8*1024**2)
    roster = lambda:{n:hashlib.sha256((HERE / n).read_bytes()).hexdigest() for n in SOURCES}
    row = {'schema':'FerricDecodeCaptureCpuTestR1','accepted':False,'source_before':roster(),
        'stage_before_bytes':before,'native_executed':False,'performance_qualified':False}
    output = D / ('gate-up-decode-capture-' + HERE.name.rsplit('-',1)[-1])
    os.umask(0o077)
    output.mkdir(mode=0o700)
    try:
        command = ['/usr/bin/python3','-I','-B',str(HERE / 'test_decode_capture.py')]
        result = subprocess.run(command,cwd=HERE,env=env,stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=120,check=False)
        if len(result.stdout)+len(result.stderr) > 1024**2:
            raise ValueError('test log bound')
        sys.stdout.buffer.write(result.stdout)
        sys.stderr.buffer.write(result.stderr)
        row.update(argv=command,returncode=result.returncode,source_after=roster(),
            log_sha256={'stdout':hashlib.sha256(result.stdout).hexdigest(),
                'stderr':hashlib.sha256(result.stderr).hexdigest()})
        if result.returncode != 0 or row['source_before'] != row['source_after']:
            raise ValueError('CPU test failed or source changed')
        if not re.search(rb'\nRan 15 tests in [0-9.]+s\n\nOK\n$',result.stderr):
            raise ValueError('exact fifteen tests, no skips')
        row.update(tests=15,skipped=0)
        row['stage_after_bytes'] = q.allocation()
        if row['stage_after_bytes']-before > 8*1024**2:
            raise ValueError('test phase envelope')
        row['accepted'] = True
    except BaseException as error:
        row['error'] = type(error).__name__+': '+str(error)
        raise
    finally:
        q.save(output / 'receipt.json',row)


if __name__ == '__main__':
    main()
