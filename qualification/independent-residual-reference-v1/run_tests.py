import hashlib, json, os, platform, sys, time, unittest
from pathlib import Path
P=Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/p228-independent-layer-reference-v1")
expected={
"residual_reference.py":"72f22c7134d609034a033e5ac70fb8c67032a95b9ad96eeafb399092b3655ad8",
"test_residual_reference.py":"4ff70d01d01aef66711b4b129a7f394dc7eb10cece9c3df1a812be1a42b03d0b",
"helpers/residual_oracle.py":"551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3",
"README.md":"7ece2a87b1c2b3f9fdb60b28985c6d4c07e3dc3d64cf989d9ab79fe3c89732ea"}
def snapshot():
    assert P.resolve(strict=True)==P
    files={}
    for name,digest in expected.items():
        path=P/name
        assert path.resolve(strict=True)==path
        raw=path.read_bytes()
        actual=hashlib.sha256(raw).hexdigest()
        assert actual==digest
        files[name]={"bytes":len(raw),"sha256":actual}
    assert {str(p.relative_to(P)) for p in P.rglob("*") if p.is_file()}==set(expected)
    return files
assert not sys.flags.optimize and sys.dont_write_bytecode
assert os.getuid()==os.geteuid()==9661 and platform.node()=="smci350-rck-g03-b19-03"
assert os.sched_getaffinity(0)=={8,9} and os.getpriority(os.PRIO_PROCESS,0)==10
assert all(os.environ.get(k)=="" for k in ("HIP_VISIBLE_DEVICES","ROCR_VISIBLE_DEVICES","CUDA_VISIBLE_DEVICES"))
before=snapshot()
sys.path.insert(0,str(P))
import test_residual_reference as T
assert Path(T.__file__).resolve()==P/"test_residual_reference.py"
suite=unittest.defaultTestLoader.loadTestsFromModule(T)
assert suite.countTestCases()==18
start=time.monotonic()
result=unittest.TextTestRunner(verbosity=2).run(suite)
elapsed=time.monotonic()-start
after=snapshot()
assert before==after
passed=result.wasSuccessful() and result.testsRun==18 and not result.skipped
report={"schema":"FerricIndependentResidualCaptureTestsV1","host":platform.node(),"python":platform.python_version(),"passed":passed,"tests":result.testsRun,"errors":len(result.errors),"failures":len(result.failures),"skipped":len(result.skipped),"elapsed_seconds":elapsed,"source_files":after,"source_postchecks_passed":True,"synthetic_capture_tests_only":True,"real_gpu_capture_validation":False,"gpu_execution":False,"full_layer_acceptance":False,"full_model_acceptance":False,"production_authority":False}
print("RESULT_JSON="+json.dumps(report,sort_keys=True),flush=True)
sys.exit(0 if passed else 1)
