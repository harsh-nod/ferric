import pathlib, hashlib, json, io, os, sys, time, signal, resource, types, unittest
root = pathlib.Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/warm-paired-terminal-host-report-source-v1')
out = pathlib.Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/warm-paired-terminal-host-report-tests-v1')
assert os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
assert not os.path.lexists(out) and root.resolve(strict=True) == root
os.umask(0o077)
os.sched_setaffinity(0, {8, 9})
os.nice(10)
for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
    os.environ[key] = ''
for kind, cap in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_CPU, 50), (resource.RLIMIT_FSIZE, 1 << 20), (resource.RLIMIT_CORE, 0)):
    resource.setrlimit(kind, (cap, cap))
def stop(*_):
    raise TimeoutError('sixty-second synthetic-test deadline')
signal.signal(signal.SIGALRM, stop)
signal.alarm(60)
pins = {'render.py': 'eb75baf7174237b89752ca7525de10e88da8b4161e8084a06ec817760645daf5', 'test_render.py': '3f6882ae9621a59046eedbe577ca04f717c2d24fbf98397421eab1570a06ae5a'}
bodies = {name: (root / name).read_bytes() for name in pins}
assert all(hashlib.sha256(body).hexdigest() == pins[name] for name, body in bodies.items())
modules = {}
for name in pins:
    module = types.ModuleType(name[:-3])
    module.__file__ = str(root / name)
    sys.modules[name[:-3]] = module
    exec(compile(bodies[name], module.__file__, 'exec'), module.__dict__)
    modules[name] = module
suite = unittest.defaultTestLoader.loadTestsFromTestCase(modules['test_render.py'].RenderTests)
names = [test.id() for test in suite]
assert len(names) == len(set(names)) == 7
stream = io.StringIO()
result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
posterrors = [name + ' changed' for name in pins if (root / name).read_bytes() != bodies[name]]
passed = result.wasSuccessful() and result.testsRun == 7 and not result.skipped and not posterrors
def pin(body):
    return {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
invocation = sys.orig_argv[-1].encode()
stderr = stream.getvalue().encode()
receipt = {'schema': 'ferric-warm-terminal-host-report-tests-v1', 'passed': passed, 'host': os.uname().nodename, 'uid': os.getuid(), 'python': sys.version, 'tests_run': result.testsRun, 'test_names': names, 'failures': len(result.failures), 'errors': len(result.errors), 'skipped': len(result.skipped), 'sources': {name: pin(body) for name, body in bodies.items()}, 'invocation': pin(invocation), 'stderr': pin(stderr), 'postcheck_errors': posterrors, 'synthetic_tests_only': True, 'subprocesses_started': 0, 'gpu_execution': False, 'performance_claim': False}
out.mkdir(mode=0o700)
for name, body in [('invocation.py', invocation), ('tests.stderr', stderr), ('complete.json' if passed else 'failed.json', (json.dumps(receipt, indent=2, sort_keys=True) + '\n').encode())]:
    with (out / name).open('xb') as f:
        f.write(body)
    assert (out / name).read_bytes() == body
signal.alarm(0)
print(json.dumps({'passed': passed, 'tests_run': result.testsRun, 'output': str(out)}))
sys.exit(0 if passed else 1)
