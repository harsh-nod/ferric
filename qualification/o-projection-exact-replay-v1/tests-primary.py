import os,pathlib,hashlib,json,sys,unittest,time,io
p=pathlib.Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/o-projection-source-input-v228-v1"); out=p.parent/"o-projection-exact-replay-tests-v228-v1"
assert os.getuid()==9661 and os.uname().nodename=="smci350-rck-g03-b19-03"
assert os.sched_getaffinity(0)=={8,9} and os.getpriority(os.PRIO_PROCESS,0)==10
raw=(p/"source-manifest.json").read_bytes(); assert hashlib.sha256(raw).hexdigest()=="6655e7b7bd7a7f9b6f2e10aae1b4a5e854a0cf2b39481b237f420366f69a6827"
m=json.loads(raw)
def check():
 for name,row in m["files"].items():
  b=(p/name).read_bytes(); assert len(b)==row["bytes"] and hashlib.sha256(b).hexdigest()==row["sha256"]
check(); out.mkdir(mode=0o700); sys.path.insert(0,str(p)); suite=unittest.defaultTestLoader.loadTestsFromNames(["test_fp32_replay","test_replay"])
def ids(s):
 for x in s:
  if isinstance(x,unittest.TestSuite): yield from ids(x)
  else: yield x.id()
names=sorted(ids(suite)); assert names==m["test_names"] and len(names)==18
log=io.StringIO(); start=time.monotonic(); r=unittest.TextTestRunner(stream=log,verbosity=2).run(suite)
check()
report=dict(passed=r.wasSuccessful() and r.testsRun==18 and not r.skipped,tests=r.testsRun,failures=len(r.failures),errors=len(r.errors),skipped=len(r.skipped),names=names,log=log.getvalue(),source_manifest_sha256=hashlib.sha256(raw).hexdigest(),source_and_input_postchecks_passed=True,cpu_seconds=time.monotonic()-start,gpu_execution=False)
b=(json.dumps(report,indent=2,sort_keys=True)+"\n").encode(); path=out/("complete.json" if report["passed"] else "failed.json")
with path.open("xb") as f: f.write(b)
print(json.dumps(dict(passed=report["passed"],tests=r.testsRun,failures=len(r.failures),errors=len(r.errors),report=dict(path=str(path),bytes=len(b),sha256=hashlib.sha256(b).hexdigest()))))
print(log.getvalue())
sys.exit(0 if report["passed"] else 1)
