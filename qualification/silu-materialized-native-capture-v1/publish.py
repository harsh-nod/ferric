"""Retain one actual SiLU capture and replay data checks, never native execution."""
import argparse
import ast
import copy
import hashlib
import os
from pathlib import Path
import re
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/silu-materialized-native-capture-v1'
PRIOR_PUBLISHER = F / 'qualification/projection-residual-native-capture-v1/publish.py'
PRIOR_SHA = 'bc2b867b6936d80e50717a6a5e4fbb3b20db7cf367be860f05002153121646a3'
PACKAGE = 'p228-silu-materialized-capture-gpu-v1'
PACKAGE_SHA = '71cae69a25ac82c53bdf2976ab53b9d9af615c5af4b8337c2e386beb44a8a18b'
PURE_SHA = '2e0b66258165f49c720df093c17ca12cb1d638e75c694027ae75ea1265b8005e'
BASE_SHA = '4f25030567c470062fd50862876bc35e93d080f1778c0be4ca36f2b39af4199a'
PLAN_SHA = '381a1a4347b55a02b9d4160eeac38cbb1e5768e757e7b3cdee6371215a04c774'
LOWERING_PUBLIC_SHA = '96bb450efc5e988c5d09d2df641879f9acf87dde02b6e3c1a005b9b60244f054'
CPU_SHA = 'bf1a12f78981d9ff9b8157e1dec6dca300752b238680e16380b98e9d1260bafb'
COPIES = {}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def helpers():
    require(PRIOR_PUBLISHER.resolve(strict=True) == PRIOR_PUBLISHER, 'canonical existing publisher')
    raw = PRIOR_PUBLISHER.read_bytes()
    require(len(raw) <= 128 << 10 and hashlib.sha256(raw).hexdigest() == PRIOR_SHA, 'exact prior data-only helper')
    module = types.ModuleType('prior_capture_publication'); module.__file__ = str(PRIOR_PUBLISHER)
    exec(compile(raw, str(PRIOR_PUBLISHER), 'exec'), module.__dict__)
    H = module.helper()
    require(H.body(PRIOR_PUBLISHER) == raw, 'stable prior helper bytes')
    return module, H


def local_pin(H, path):
    raw = H.body(path)
    return dict(path=str(path), bytes=len(raw), sha256=H.digest(raw))


def original(H, path):
    rel = path.relative_to(L)
    if rel.parts[0] == 'proposals':
        rel = Path(*rel.parts[1:])
    return dict(local_pin(H, path), path=str(E / rel))


def read(H, record):
    record = H.normalize(record); rel = Path(record['path']).relative_to(E)
    path = L / ('proposals' if rel.parts[0].startswith(('p227-', 'p228-')) else '') / rel
    if rel == Path('run_silu_materialized_capture_gpu_pure_p228_v1.py'):
        path = L / 'proposals/p228-silu-materialized-capture-harness-v1' / rel
    raw = H.body(path)
    require(len(raw) == record['bytes'] and H.digest(raw) == record['sha256'], 'original/retained body identity')
    return raw


def doc(H, record):
    return H.parse(read(H, record))


def keep(H, record, destination, local=False):
    raw = H.body(Path(record['path'])) if local else read(H, record)
    require(len(raw) == record['bytes'] and H.digest(raw) == record['sha256'], 'copied source identity')
    require(destination not in COPIES and not Path(destination).is_absolute() and '..' not in Path(destination).parts
        and len(raw) <= 2 << 20 and b'\0' not in raw, 'unique bounded text output')
    raw.decode('utf-8')
    COPIES[destination] = (raw, record)


def package(H, value, plan):
    manifest_pin = value['supervisor_manifest']; manifest = doc(H, manifest_pin)
    require(manifest_pin['path'] == str(E / PACKAGE / 'manifest.json') and manifest_pin['sha256'] == PACKAGE_SHA
        and manifest['schema'] == 'ferric-p228-silu-materialized-capture-gpu-package-v1'
        and manifest['pure_tests'] == 36, 'frozen capture package')
    sources = {row['path']: dict(row, path=str(E / PACKAGE / row['path'])) for row in manifest['files']}
    require(len(sources) == len(manifest['files']) == 11 and value['controller'] == sources['run.py'], 'closed source census')
    keep(H, manifest_pin, 'controller/manifest.json')
    for name, record in sources.items():
        keep(H, record, 'controller/' + name)
    constants = {}
    for node in ast.parse(read(H, sources['intake.py'])).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            try:
                constants[node.targets[0].id] = ast.literal_eval(node.value)
            except (ValueError, TypeError):
                pass
    require(set(sources) == constants['PACKAGE_FILES'], 'exact frozen roster')
    tested = doc(H, plan['supervisor_tests'])
    require(plan['supervisor_tests']['sha256'] == PURE_SHA and tested['passed'] is True and tested['tests'] == 36
        and tested['schema'] == constants['PURE_SCHEMA'] and tested['manifest_sha256'] == PACKAGE_SHA
        and tested['controller_sha256'] == constants['TEST_RUNNER_SHA']
        and tested['errors'] == tested['failures'] == tested['skipped'] == 0
        and tested['source_postchecks_passed'] is True and tested['sources_before'] == plan['supervisor_test_sources']
        and doc(H, tested['sources_before']) == doc(H, tested['sources_after']) == sources
        and tested['synthetic_policy_tests_only'] is True and all(tested[k] is False for k in
            ('native_execution','gpu_execution','numerical_acceptance','full_model_acceptance','performance_claim','production_authority')),
        'actual bounded pure36 only')
    names = {}
    for name, record in sources.items():
        if name.startswith('test_') and name.endswith('.py'):
            names[name] = sorted(name[:-3] + '.' + cls.name + '.' + fn.name
                for cls in ast.parse(read(H, record)).body if isinstance(cls, ast.ClassDef)
                for fn in cls.body if isinstance(fn, ast.FunctionDef) and fn.name.startswith('test_'))
    require(tested['test_inventory_before'] == tested['test_inventory_after'] == names
        and sum(map(len, names.values())) == 36, 'actual named pure inventory')
    log = read(H, tested['transcript']).decode(); rows = []
    for method, scope in re.findall(r'^(test_\w+) \(([^()\n]+)\) \.\.\. ok$', log, re.M):
        rows.append(scope if scope.endswith('.' + method) else scope + '.' + method)
    require(sorted(rows) == sorted(n for group in names.values() for n in group)
        and log.rstrip().endswith('OK'), 'actual pure successes')
    keep(H, plan['supervisor_tests'], 'pure/complete.json')
    for key, name in (('sources_before','sources-before.json'),('sources_after','sources-after.json'),('transcript','tests.log')):
        keep(H, tested[key], 'pure/' + name)
    keep(H, dict(path=str(E / 'run_silu_materialized_capture_gpu_pure_p228_v1.py'),
        bytes=7493, sha256=constants['TEST_RUNNER_SHA']), 'pure/controller.py')
    # Only the two frozen data validators are evaluated, never intake or a supervisor.
    missing = object(); previous = sys.modules.get('layer_validation', missing)
    try:
        for name in ('layer_validation', 'capture_validation'):
            module = types.ModuleType('silu_published_' + name); module.__file__ = sources[name + '.py']['path']
            exec(compile(read(H, sources[name + '.py']), module.__file__, 'exec'), module.__dict__)
            if name == 'layer_validation':
                sys.modules[name] = module
    finally:
        if previous is missing:
            sys.modules.pop('layer_validation', None)
        else:
            sys.modules['layer_validation'] = previous
    return module, constants, sources


def assembly(H, plan_pin, plan, review):
    directory = L / Path(plan_pin['path']).parent.name
    value = H.document(directory / 'assembly.json')
    require(value['schema'] == 'ferric-p228-silu-materialized-capture-root-assembly-v1'
        and value['plan'] == plan_pin and value['explicit_root_decisions_copied'] is True
        and value['existing_runtime_reviews_copied_verbatim'] is True and value['automatic_approval'] is False
        and all(value[k] is False for k in ('gpu_execution','numerical_acceptance','production_authority','performance_claim')),
        'actual data-only root assembly')
    expected = {'plan.json':plan_pin,'request.json':plan['request'],'capture-review.json':plan['capture_review'],
        **{role+'-runtime-review.json':plan[role+'_runtime_review'] for role in ('parent','worker')}}
    require(value['outputs'] == expected, 'exact five assembled input pins')
    for name, record in expected.items():
        keep(H, record, 'inputs/' + name)
    keep(H, original(H, directory / 'assembly.json'), 'inputs/assembly.json')
    for key, name in (('configuration','configuration.json'),('root_notes','root-notes.json'),('assembler','prepare.py')):
        record = value[key]
        require(Path(record['path']).is_relative_to(L), 'authentic local-only assembly input')
        keep(H, record, 'inputs/' + name, local=True)
    notes = H.document(Path(value['root_notes']['path']))
    require(notes['configuration'] == H.document(Path(value['configuration']['path']))
        and all(notes['capture'][key] == review[key] for key in ('reviewed','authority','gpu_attempts','notes','review_topics')),
        'root decisions copied, not manufactured')
    return original(H, directory / 'assembly.json')


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('case_label'); parser.add_argument('complete_sha256'); args = parser.parse_args()
    require(re.fullmatch(r'prefix-silu-materialized-capture-gpu-v228-v[1-9][0-9]{0,8}', args.case_label)
        and re.fullmatch('[0-9a-f]{64}', args.complete_sha256), 'actual terminal receipt arguments')
    P, H = helpers(); directory = L / args.case_label
    value = H.document(directory / 'complete.json', args.complete_sha256)
    receipt = original(H, directory / 'complete.json'); plan = doc(H, value['plan']); request = doc(H, plan['request'])
    require(value['schema'] == 'ferric-p228-silu-materialized-capture-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and type(value['native_attempts']) is int and value['native_attempts'] == 1
        and type(value['retries']) is int and value['retries'] == 0 and value['gpu_execution_requested'] is True
        and value['current_tf4_hidden_equal'] is None and value['old_hidden_equality_required'] is False
        and value['pre_swiglu_arrays_equal'] is True and value['pre_swiglu_array_count'] == 22
        and value['pre_residual_arrays_equal'] is True and value['conditional_residual_checks_performed'] is False
        and value['captured_arrays'] == 28 and all(value[k] is False for k in H.FALSE)
        and plan['output_label'] == args.case_label and value['plan']['sha256'] == PLAN_SHA, 'actual structural-only native result')
    CV, K, sources = package(H, value, plan)
    require(plan['schema'] == K['INPUT_SCHEMA'] and set(plan) == set(K['PLAN_FIELDS'].split()), 'closed tested input plan')
    bindings = [key for key in K['PLAN_FIELDS'].split()
        if key not in ('schema','output_label','supervisor_tests','supervisor_test_sources')]
    for key in bindings:
        field = key + '_complete' if key in ('parent_cpu','worker_cpu') else key
        require(value[field] == plan[key], 'actual case/plan binding: ' + key)
    old = doc(H, value['baseline_capture']); old_plan = doc(H, old['plan'])
    require(value['baseline_capture']['sha256'] == BASE_SHA and old['passed'] is True and old['failures'] == []
        and value['baseline'] == old['baseline'] and value['baseline']['sha256'] == H.BASE_SHA
        and value['selected_runtime'] == old['selected_runtime'], 'authentic corrected capture and unchanged runtime')
    cpu_public_path = F / 'qualification/projection-residual-runtime-v1/result.json'; cpu_public = H.document(cpu_public_path)
    require(plan['parent_cpu'] == plan['worker_cpu'] == cpu_public['cpu_receipt']['original']
        and plan['parent_cpu']['sha256'] == CPU_SHA, 'actual published CPU988')
    for role, name in (('parent','ferric-qwen3-finite-projection-residual-layer-capture-engineering'),
                       ('worker','ferric-tp-peer-finite-engineering-worker-v1')):
        require(value[role] == old[role] == cpu_public['selected_binaries'][name]['binary']
            and read(H, plan[role+'_runtime_review']) == read(H, old_plan[role+'_runtime_review']), 'same qualified binary/runtime review')
    for key in ('projection_image','lowering_complete','inspection_complete'):
        require(plan[key] == old_plan[key], 'unchanged corrected residual source')
    expected = copy.deepcopy(doc(H, old['request']))
    for key in ('session','evidence_directory','mlp_tiles_image'):
        expected['layer'][key] = request['layer'][key]
    require(request == expected and request['layer']['session'] != doc(H, old['request'])['layer']['session']
        and request['layer']['evidence_directory'] == str(E / args.case_label / 'native')
        and request['layer']['mlp_tiles_image'] == dict(plan['mlp_image'],sha256=list(bytes.fromhex(plan['mlp_image']['sha256']))),
        'only selected MLP/session/output changed')
    lower_public_path = F / 'qualification/silu-materialized-lowering-v1/result.json'
    lower_public = H.document(lower_public_path, LOWERING_PUBLIC_SHA)
    require(plan['mlp_cpu'] == lower_public['cpu'] and plan['mlp_lowering_complete'] == lower_public['lowering']
        and plan['mlp_lowering_owner'] == lower_public['owner']
        and (plan['mlp_image']['bytes'],plan['mlp_image']['sha256']) == tuple(lower_public['artifacts']['emitted/artifact.hsaco'][k]
            for k in ('bytes','sha256')), 'actual published SiLU checked image')
    read(H, plan['mlp_image'])
    native = value['retained_native']
    require(set(native) == CV.BODY | {'summary.json'}, 'twelve native records')
    for name, record in native.items():
        require(record['path'] == str(E / args.case_label / 'native' / name), 'case-contained capture body')
    bodies = {name:read(H, native[name]) for name in CV.BODY}; summary = read(H, native['summary.json'])
    require(value['baseline_capture_payload'] == old['retained_native']['candidate-capture.bin'], 'corrected baseline raw payload')
    checked = CV.validate(summary,bodies,request,read(H,value['baseline_capture_payload']),
        doc(H,old['retained_native']['candidate-bootstrap.json'])['layer']['input'])
    require(checked == value['checked'] == doc(H,value['observation']), '28-stage partition and 22 upstream comparisons replayed')
    P.owned(H,value,plan,summary,checked,CV,old)
    review = doc(H,plan['capture_review'])
    require(review['schema'] == K['REVIEW_SCHEMA'] and review['reviewed'] is True and review['authority'] == 'none'
        and review['gpu_attempts'] == 1 and review['output_label'] == args.case_label
        and all(review[key] == plan[key] for key in bindings if key != 'capture_review')
        and all(review[key] is False for key in K['REVIEW_FALSE']), 'root review keeps all authority limitations')
    assembled = assembly(H,value['plan'],plan,review)
    keep(H,receipt,'complete.json'); keep(H,value['observation'],'observation.json')
    for name, leaf in value['leaves'].items():
        for filename, record in leaf['retained_files'].items():
            keep(H,record,'leaves/'+name+'/'+filename)
    for side in ('before','after'):
        for index,row in enumerate(value[side+'_audits']):
            keep(H,row['topology'],'audits/'+side+'-'+str(index)+'-topology.json')
    for name, record in native.items():
        if name.endswith('.json'):
            keep(H,record,'native/'+name)
    for path,name in ((Path(__file__).resolve(),'publish.py'),(PRIOR_PUBLISHER,'tools/prior-publisher.py'),
                      (P.HELPER,'tools/retained-data.py')):
        keep(H,local_pin(H,path),name,local=True)
    require(sum(len(raw) for raw,_ in COPIES.values()) <= 8 << 20,'bounded text publication')
    result = dict(schema='ferric-p228-silu-materialized-native-publication-v1',authority='none',gpu_observation=receipt,
        plan=value['plan'],assembly=assembled,cpu_publication=local_pin(H,cpu_public_path),
        lowering_publication=local_pin(H,lower_public_path),supervisor_manifest=value['supervisor_manifest'],
        sources=sources,pure=plan['supervisor_tests'],pure_tests=36,selected_runtime=value['selected_runtime'],
        mlp_image=plan['mlp_image'],projection_image=plan['projection_image'],baseline_capture=value['baseline_capture'],
        layer=0,position=0,token=9112,native_attempts=1,retries=0,captured_arrays=28,
        stages=checked['stages'],retained_native=native,leaves=value['leaves'],owned_children=value['owned_children'],
        before_audits=value['before_audits'],after_audits=value['after_audits'],
        gpu_execution_observed=True,structural_validator_replayed=True,full_kv_checked=True,
        pre_swiglu_arrays_equal=True,pre_swiglu_array_count=22,old_hidden_equality_required=False,
        current_tf4_hidden_equal=None,conditional_residual_checks_performed=False,
        published_files={name:dict(source_pin=record,bytes=len(raw),sha256=H.digest(raw)) for name,(raw,record) in sorted(COPIES.items())},
        locally_rehashed=[dict(path=str(path),**record) for path,record in H.CHECKED.items()],
        binary_captures_in_git=False,all_transitive_inputs_replayed=False,selected_executable_bodies_locally_rehashed=False,
        dynamic_library_bodies_locally_rehashed=False,runtime_audits_reexecuted=False,frozen_controller_reexecuted=False,
        numerical_comparison_reexecuted=False,top_level_observer_reaping_independently_verified=False,
        **{key:False for key in H.FALSE})
    require(Q.parent.resolve(strict=True) == Q.parent,'canonical qualification parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'},'fresh checkpoint/root README only')
        if (Q/'README.md').exists(): H.body(Q/'README.md')
    for path in list(H.CHECKED): H.body(path)
    outputs = {name:raw for name,(raw,_) in COPIES.items()}; outputs['result.json'] = H.json_bytes(result)
    Q.mkdir(exist_ok=True)
    for name,raw in outputs.items():
        path = Q/name; path.parent.mkdir(parents=True,exist_ok=True)
        with path.open('xb') as stream: stream.write(raw)
        require(H.body(path) == raw,'exact publication bytes')
    for path in list(H.CHECKED): H.body(path)
    print(H.json_bytes(dict(result=local_pin(H,Q/'result.json'),files=len(outputs))).decode(),end='')


if __name__ == '__main__':
    main()
