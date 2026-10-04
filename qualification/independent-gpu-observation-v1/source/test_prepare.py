import ast
import hashlib
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import prepare as P
from test_validation import fixture, pin
from test_run import platform


def inputs():
    value = dict(schema='ferric-p228-independent-gpu-inputs-v1',
        **{key: pin('/e/' + key + '.json') for key in ('deployment', 'deployment_tests', 'deployment_test_source', 'artifact_review',
            'platform_review', 'runtime_review', 'observer_manifest', 'observer_tests', 'observer_test_sources')},
        matrix_label='prefix-independent-profile-gpu-v228-v1', cases=[],
        topology_helper=pin(str(P.R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py')))
    value['topology_helper']['sha256'] = P.TOPOLOGY_SHA
    for case, digest in zip(P.V.CASES, P.BASELINE_SHA):
        record = pin(str(P.R / 'evidence/resident-output-tp2-v217/prepare-mi350-remaining-v1' / case / 'request.json'))
        record['sha256'] = digest
        value['cases'].append(dict(case=case, baseline_request=record,
            reviews=[pin('/e/' + case + '-' + kind + '.json') for kind in P.V.KINDS]))
    return value


def deployment():
    value = dict(schema='ferric-p228-independent-deployment-v1', authority='none')
    verified = dict(metadata=dict(descriptor_sha256='a' * 64,
        canonical_code_object_digest='b' * 64, descriptor_symbol_hex=(P.V.SYMBOL + '.kd').encode().hex()),
        image=pin('/deployed/new-image'), original_image=pin('/compiler-v7/emitted/artifact.hsaco'),
        binary=pin('/deployed/new-binary'), original_binary=pin('/native98/example'),
        compiler_receipt=pin('/compiler-v7/complete.json'), compiler_owner=pin('/compiler-v7-owner/complete.json'),
        native_receipt=pin('/native98/complete.json'), native_owner=pin('/native98-owner/complete.json'),
        compiler_generation=dict(synthetic='ordinary-induction'),
        candidate_cpu_receipt=pin('/reciprocal-v3/complete.json'), candidate_cpu_sources=dict(synthetic=True),
        native_compiler_generation=dict(synthetic='ordinary-induction'), native_overlay=pin('/native-overlay/manifest.json'),
        native_sources=dict(files=5783, before=pin('/native98/before.json'), after=pin('/native98/after.json')),
        qualifications=dict(compiler=dict(synthetic=True), native=dict(synthetic=True)),
        compiler_artifacts=dict(synthetic=True), compiler_phase_records=dict(synthetic=True),
        candidate_sources=dict(synthetic=True), arithmetic_evidence=dict(synthetic=True))
    return value, verified


def artifact_review(given, deployed, verified):
    return dict(schema='ferric-p228-independent-gpu-artifact-review-v1',
        decision='reviewed-engineering-six-case-independent-profiles', deployment=given['deployment'],
        compiler_complete=verified['compiler_receipt'], compiler_owner=verified['compiler_owner'],
        original_object=verified['original_image'], candidate_cpu_receipt=verified['candidate_cpu_receipt'],
        native_complete=verified['native_receipt'], native_owner=verified['native_owner'],
        original_binary=verified['original_binary'], native_overlay=verified['native_overlay'],
        binary=verified['binary'], object=verified['image'],
        descriptor_sha256='a' * 64, canonical_code_object_digest='b' * 64, entry_symbol=P.V.SYMBOL,
        workgroup=[64, 1, 1], grid=[4096, 1, 1], lds_bytes=512, state_words=284, cases=given['cases'],
        production_authority=False, runtime_premises_discharged=False, independent_numerical_acceptance=False,
        performance_claim=False, notes='Synthetic review fixture, not actual artifact approval.')


def runtime_fixture(directory):
    library = directory / 'libc.so.6'; library.write_bytes(b'not an executable, synthetic test only')
    loader = directory / 'ld-linux.so'; loader.write_bytes(b'synthetic loader')
    binary = pin('/e/binary'); reviewed = platform(); documents = {}; bodies = {}
    value = dict(schema='ferric-p227-prefix-parity-runtime-review-v1', authority='none', reviewed=True,
        host=reviewed['host'], boot_id=reviewed['boot_id'], binary=binary,
        libraries=[dict(path=str(path), resolved=P.D.read_file(path)[0]) for path in (library, loader)],
        notes='Synthetic test-only runtime records.', production_authority=False, gpu_execution=False)
    outputs = dict(readelf=' 0x00000001 (NEEDED) Shared library: [libc.so.6]\n',
        ldd=' linux-vdso.so.1 (0x123)\n libc.so.6 => ' + str(library) + ' (0x456)\n '
            + str(loader) + ' (0x789)\n')
    for name, argv in (('readelf', ['/usr/bin/readelf', '-d', binary['path']]),
                      ('ldd', ['/usr/bin/ldd', binary['path']])):
        value[name] = pin('/audit/' + name + '.json')
        stdout, stderr = pin('/audit/' + name + '-stdout', outputs[name].encode()), pin('/audit/' + name + '-stderr', b'')
        documents[value[name]['path']] = dict(argv=argv, exit_code=0, deadline_seconds=30, stdout=stdout, stderr=stderr)
        bodies[stdout['path']] = outputs[name].encode(); bodies[stderr['path']] = b''
    for row in value['libraries']: bodies[row['resolved']['path']] = Path(row['resolved']['path']).read_bytes()
    return value, binary, reviewed, documents, bodies


class Tests(unittest.TestCase):
    def test_actual_runtime_stdout_direct_loader_covers_only_exact_needed_loader(self):
        directory = Path(__file__).resolve().parent / 'fixtures'
        elf = (directory / 'actual-readelf.stdout').read_bytes()
        ldd = (directory / 'actual-ldd.stdout').read_bytes()
        self.assertEqual((len(elf), hashlib.sha256(elf).hexdigest()),
            (1615, 'c4c154492d1bc30d10731adb430915f5a251e3369e379b0c525fa25d52deb48b'))
        self.assertEqual((len(ldd), hashlib.sha256(ldd).hexdigest()),
            (230, '8614b3c271fa6997a2d0096563f3d82c040021118e4ac64a9466870cbee23940'))
        aliases = ['/lib/x86_64-linux-gnu/libgcc_s.so.1', '/lib/x86_64-linux-gnu/libc.so.6',
                   '/lib64/ld-linux-x86-64.so.2']
        for mutation in (None, 'wrong_direct', 'missing', 'duplicate_loader', 'duplicate_named', 'unknown', 'needed'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                value, binary, reviewed, documents, bodies = runtime_fixture(Path(temporary))
                current_aliases, elf_raw, ldd_raw = list(aliases), elf, ldd
                if mutation == 'wrong_direct':
                    current_aliases[-1] = '/lib64/not-loader.so'
                    ldd_raw = ldd_raw.replace(aliases[-1].encode(), current_aliases[-1].encode())
                if mutation == 'missing':
                    current_aliases.pop()
                    ldd_raw = b'\n'.join(line for line in ldd.split(b'\n') if b'/lib64/' not in line)
                if mutation in ('duplicate_loader', 'duplicate_named'):
                    name = 'ld-linux-x86-64.so.2' if mutation == 'duplicate_loader' else 'libgcc_s.so.1'
                    current_aliases.append('/synthetic-second/' + name)
                    ldd_raw += (name + ' => ' + current_aliases[-1] + ' (0x1234)\n').encode()
                if mutation == 'unknown': ldd_raw += b'warning: unknown\n'
                if mutation == 'needed': elf_raw = elf.replace(b'[ld-linux-x86-64.so.2]', b'[other.so]')
                value['libraries'] = []; resolved = {}
                for index, alias in enumerate(current_aliases):
                    path = Path(temporary) / ('synthetic-library-' + str(index))
                    path.write_bytes(b'file-backed non-executable library stand-in')
                    record = P.D.read_file(path)[0]
                    value['libraries'].append(dict(path=alias, resolved=record)); resolved[alias] = path
                    bodies[str(path)] = path.read_bytes()
                for name, raw in (('readelf', elf_raw), ('ldd', ldd_raw)):
                    audit = documents[value[name]['path']]
                    audit['stdout'] = pin(audit['stdout']['path'], raw); bodies[audit['stdout']['path']] = raw
                with mock.patch.object(P, 'document', side_effect=lambda p, r, *a: documents[r['path']]), \
                        mock.patch.object(P, 'read', side_effect=lambda p, r, *a, **k: bodies[r['path']]), \
                        mock.patch.object(Path, 'resolve', autospec=True, side_effect=lambda path, strict=False: resolved[str(path)]):
                    if mutation is None:
                        P.runtime_review(value, binary, reviewed, None)
                    else:
                        with self.assertRaises(RuntimeError): P.runtime_review(value, binary, reviewed, None)

    def test_closed_inputs_refuse_old_live_source_or_missing_runtime_authority(self):
        value = inputs(); P.input_shape(value)
        for mutation in ('old', 'extra', 'runtime', 'case', 'baseline', 'topology'):
            changed = copy.deepcopy(value)
            if mutation == 'old': changed['schema'] = 'ferric-p227-prefix-parity-inputs-v1'
            if mutation == 'extra': changed['source_complete'] = pin('/old/source')
            if mutation == 'runtime': del changed['runtime_review']
            if mutation == 'case': changed['cases'].reverse()
            if mutation == 'baseline': changed['cases'][0]['baseline_request']['sha256'] = '0' * 64
            if mutation == 'topology': changed['topology_helper']['sha256'] = '0' * 64
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError): P.input_shape(changed)

    def test_all_six_original_request_pins_match_retained_closure(self):
        raw = Path(P.__file__).with_name('baseline-inputs.json').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), P.BASELINE_INPUTS_SHA)
        value = json.loads(raw); rows = {row['path']: row for row in value['files']}
        self.assertEqual((len(rows), sum(row['bytes'] for row in rows.values())), (334, 88974677))
        for case, digest in zip(P.V.CASES, P.BASELINE_SHA):
            path = P.R / 'evidence/resident-output-tp2-v217/prepare-mi350-remaining-v1' / case / 'request.json'
            self.assertEqual(rows[str(path)]['sha256'], digest)
        self.assertFalse(value['gpu_execution']); self.assertFalse(value['tests_run'])

    def test_baseline_loader_checks_every_actual_record_without_inventing_hashes(self):
        value = json.loads(Path(P.__file__).with_name('baseline-inputs.json').read_bytes())
        record = dict(path='/e/baseline-inputs.json', bytes=1, sha256=P.BASELINE_INPUTS_SHA)
        pins = mock.Mock(); pins.json.return_value = (value, record)
        with mock.patch.object(P, 'read') as read:
            actual, rows = P.baseline_inputs(pins)
        self.assertEqual(actual, record); self.assertEqual(len(rows), 334)
        self.assertEqual(read.call_count, 334)
        for call, expected in zip(read.call_args_list, value['files']):
            self.assertEqual(call.args, (pins, expected))
        for mutation in ('duplicate', 'missing', 'count', 'authority'):
            changed = json.loads(json.dumps(value))
            if mutation == 'duplicate': changed['files'][-1] = changed['files'][0]
            if mutation == 'missing': changed['files'].pop()
            if mutation == 'count': changed['bytes'] -= 1
            if mutation == 'authority': changed['gpu_execution'] = True
            pins.json.return_value = (changed, record)
            with mock.patch.object(P, 'read') as read, self.assertRaises(RuntimeError):
                P.baseline_inputs(pins)
            read.assert_not_called()

    def test_request_construction_uses_actual_artifact_and_supplied_reviews(self):
        requested, _, _, *_ = fixture()
        metadata = dict(descriptor_sha256='a' * 64, canonical_code_object_digest='b' * 64,
                        descriptor_symbol_hex=(P.V.SYMBOL + '.kd').encode().hex())
        row = dict(baseline_request=requested['baseline_request'], reviews=requested['reviews'])
        result = P.make_request(row, metadata, requested['tiles']['object'], Path('/e/case/captures'))
        self.assertEqual(result, requested)
        self.assertEqual(result['reviews'], row['reviews'])

    def test_pure_receipt_joins_packing_snapshot_not_current_root(self):
        given = inputs(); directory = P.E / 'observer-pure-v227-v1'
        given['observer_tests'] = pin(str(directory / 'complete.json'))
        given['observer_test_sources'] = pin(str(directory / 'sources-before.json'), b'packing snapshot')
        receipt = dict(passed=True, tests=P.PURE_TESTS, manifest_sha256=given['observer_manifest']['sha256'],
            source_sha256=given['observer_test_sources']['sha256'], controller_sha256=P.TEST_RUNNER_SHA,
            gpu_execution=False, numerical_acceptance=False)
        with mock.patch.object(P, 'read') as read:
            P.pure_tests(receipt, given, None)
        read.assert_called_once_with(None, given['observer_test_sources'], maximum=16 << 20)
        for key, replacement in (('tests', True), ('tests', 28), ('gpu_execution', True),
                ('numerical_acceptance', True), ('source_sha256', 'f' * 64),
                ('manifest_sha256', '0' * 64), ('controller_sha256', '0' * 64)):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                P.pure_tests(dict(receipt, **{key: replacement}), given, None)
        changed = copy.deepcopy(given); changed['observer_test_sources']['path'] = '/current/sources-after.json'
        with self.assertRaises(RuntimeError): P.pure_tests(receipt, changed, None)

    def test_package_checks_closed_roster_and_frozen_reader_validator_helpers(self):
        record = pin(str(Path(P.__file__).resolve().with_name('manifest.json')))
        value = dict(schema='ferric-p228-independent-gpu-observation-contracts-v1', pure_tests=P.PURE_TESTS,
            files=[dict(path=name, bytes=1, sha256='a' * 64) for name in sorted(P.MEMBERS)])
        pins = mock.Mock(); pins.pin.return_value = dict(bytes=1)
        with mock.patch.object(P, 'document', return_value=value): P.package(pins, record)
        self.assertIn(mock.call(Path(P.__file__).resolve().with_name('observe.py'), P.OBSERVE_SHA), pins.pin.call_args_list)
        for mutation in ('old', 'count', 'extra', 'missing'):
            changed = copy.deepcopy(value)
            if mutation == 'old': changed['schema'] = 'old'
            if mutation == 'count': changed['pure_tests'] -= 1
            if mutation == 'extra': changed['files'].append(changed['files'][0])
            if mutation == 'missing': changed['files'].pop()
            with mock.patch.object(P, 'document', return_value=changed), self.assertRaises(RuntimeError):
                P.package(pins, record)

    def test_portable_intake_calls_only_checked_reader_at_actual_transport_path(self):
        given = inputs(); directory = P.E / 'prefix-independent-deployment-v228-v1'
        given['deployment'] = pin(str(directory / 'deployment.json'))
        given['deployment_tests'] = pin(str(P.E / 'deployment-pure-v228-v1/complete.json'))
        given['deployment_test_source'] = pin('/deployment/test_portable.py')
        value, verified = deployment(); pins = mock.Mock()
        reader = mock.Mock(); reader.verify.return_value = (value, verified)
        receipt = dict(schema='ferric-p228-independent-deployment-pure-v1', passed=True, tests=27,
            package_manifest=pin('/deployment/manifest.json'), test_source=given['deployment_test_source'],
            controller=pin('/e/pure_controller.py'), transcript=pin(str(P.E / 'deployment-pure-v228-v1/tests.log')),
            gpu_execution=False, numerical_acceptance=False)
        receipt['package_manifest']['sha256'] = 'a' * 64
        receipt['controller']['sha256'] = 'c' * 64
        pins.pin.side_effect = lambda path, *args: (receipt['package_manifest'] if path.name == 'manifest.json'
                                                  else receipt['test_source'])
        def document(p, record, *args):
            return receipt if record == given['deployment_tests'] else value
        with mock.patch.object(P, 'document', side_effect=document), mock.patch.object(P, 'read'), \
                mock.patch.object(P, 'DEPLOYMENT_PACKAGE_SHA', 'a' * 64), \
                mock.patch.object(P, 'DEPLOYMENT_READER_SHA', 'b' * 64), \
                mock.patch.object(P, 'DEPLOYMENT_PURE_TESTS', 27), \
                mock.patch.object(P, 'DEPLOYMENT_TEST_RUNNER_SHA', 'c' * 64), \
                mock.patch.object(P.D, 'package', return_value=Path('/deployment')) as source_package, \
                mock.patch.object(P.D, 'load_module', return_value=reader) as loader:
            self.assertEqual(P.deployment_context(given, pins), (value, verified))
        reader.verify.assert_called_once_with(P.D, pins, value, directory)
        source_package.assert_called_once_with(pins, P.DEPLOYMENT_PACKAGE, 'a' * 64)
        loader.assert_called_once_with(pins, Path('/deployment/portable.py'), 'b' * 64,
                                      'independent_profile_deployment')
        with mock.patch.object(P, 'DEPLOYMENT_PACKAGE_SHA', None), self.assertRaises(RuntimeError):
            P.deployment_context(given, pins)

    def test_deployment_pure_gate_refuses_unfrozen_controller_before_loading_module(self):
        with mock.patch.object(P, 'DEPLOYMENT_PACKAGE_SHA', 'a' * 64), \
                mock.patch.object(P, 'DEPLOYMENT_READER_SHA', 'b' * 64), \
                mock.patch.object(P, 'DEPLOYMENT_PURE_TESTS', 27), \
                mock.patch.object(P, 'DEPLOYMENT_TEST_RUNNER_SHA', None), \
                mock.patch.object(P.D, 'load_module') as load, self.assertRaises(RuntimeError):
            P.deployment_context(inputs(), mock.Mock())
        load.assert_not_called()

    def test_deployment_pure_receipt_rejects_wrong_source_controller_or_authority(self):
        given = inputs(); directory = P.E / 'deployment-pure-v228-v1'
        given['deployment_tests'] = pin(str(directory / 'complete.json'))
        given['deployment_test_source'] = pin('/deployment/test_portable.py')
        manifest = dict(pin('/deployment/manifest.json'), sha256='a' * 64)
        original = dict(schema='ferric-p228-independent-deployment-pure-v1', passed=True, tests=27,
            package_manifest=manifest, test_source=given['deployment_test_source'],
            controller=dict(pin('/e/controller.py'), sha256='c' * 64),
            transcript=pin(str(directory / 'tests.log')), gpu_execution=False, numerical_acceptance=False)
        for mutation in ('schema', 'count', 'source', 'controller', 'manifest', 'transcript', 'gpu', 'numerical'):
            value = copy.deepcopy(original)
            if mutation == 'schema': value['schema'] = 'old'
            if mutation == 'count': value['tests'] = True
            if mutation == 'source': value['test_source'] = pin('/old/test_portable.py')
            if mutation == 'controller': value['controller']['sha256'] = '0' * 64
            if mutation == 'manifest': value['package_manifest']['sha256'] = '0' * 64
            if mutation == 'transcript': value['transcript']['path'] = '/old/tests.log'
            if mutation == 'gpu': value['gpu_execution'] = True
            if mutation == 'numerical': value['numerical_acceptance'] = True
            pins = mock.Mock()
            pins.pin.side_effect = lambda path, *args: manifest if path.name == 'manifest.json' else given['deployment_test_source']
            with self.subTest(mutation=mutation), mock.patch.object(P, 'document', return_value=value), \
                    mock.patch.object(P, 'DEPLOYMENT_PACKAGE_SHA', 'a' * 64), \
                    mock.patch.object(P, 'DEPLOYMENT_READER_SHA', 'b' * 64), \
                    mock.patch.object(P, 'DEPLOYMENT_PURE_TESTS', 27), \
                    mock.patch.object(P, 'DEPLOYMENT_TEST_RUNNER_SHA', 'c' * 64), \
                    mock.patch.object(P.D, 'package', return_value=Path('/deployment')), \
                    mock.patch.object(P.D, 'load_module') as load, self.assertRaises(RuntimeError):
                P.deployment_context(given, pins)
            load.assert_not_called()

    def test_review_and_provenance_never_relabel_old_image_as_current_generation(self):
        given = inputs(); deployed, verified = deployment(); value = artifact_review(given, deployed, verified)
        P.reviewed_artifact(value, given, deployed, verified)
        proof = P.provenance(given, deployed, verified)
        self.assertEqual(proof['compiler_complete'], verified['compiler_receipt'])
        self.assertEqual(proof['native_complete'], verified['native_receipt'])
        self.assertEqual(proof['native_sources']['files'], 5783)
        self.assertNotIn('historical_root_source', proof)
        for key, replacement in (('compiler_complete', verified['native_receipt']),
                ('original_object', verified['image']), ('binary', pin('/old/binary')),
                ('native_overlay', pin('/old/manifest.json')), ('production_authority', True),
                ('notes', ''),
                ('descriptor_sha256', '0' * 64), ('grid', [128, 1, 1]), ('state_words', 22)):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                P.reviewed_artifact(dict(value, **{key: replacement}), given, deployed, verified)

    def test_runtime_review_checks_exact_binary_audits_and_all_canonical_libraries(self):
        with tempfile.TemporaryDirectory() as temporary:
            value, binary, reviewed, documents, bodies = runtime_fixture(Path(temporary))
            with mock.patch.object(P, 'document', side_effect=lambda p, r, maximum: documents[r['path']]), \
                    mock.patch.object(P, 'read', side_effect=lambda p, r, *a, **kw: bodies[r['path']]) as read:
                P.runtime_review(value, binary, reviewed, None)
            self.assertEqual(read.call_count, 6)

    def test_runtime_unknown_missing_libraries_bad_command_and_review_refuse(self):
        with tempfile.TemporaryDirectory() as temporary:
            original = runtime_fixture(Path(temporary))
            for mutation in ('unknown', 'missing', 'needed', 'exit', 'argv', 'deadline', 'stderr', 'library', 'boot', 'binary'):
                value, binary, reviewed, documents, bodies = copy.deepcopy(original)
                ldd = documents[value['ldd']['path']]; elf = documents[value['readelf']['path']]
                if mutation == 'unknown': bodies[ldd['stdout']['path']] += b'warning: ignored\n'
                if mutation == 'missing': bodies[ldd['stdout']['path']] = b'libc.so.6 => not found\n'
                if mutation == 'needed': bodies[elf['stdout']['path']] += b' (NEEDED) Shared library: [unresolved.so]\n'
                if mutation == 'exit': ldd['exit_code'] = 1
                if mutation == 'argv': ldd['argv'][-1] = '/old/binary'
                if mutation == 'deadline': ldd['deadline_seconds'] = 31
                if mutation == 'stderr': bodies[ldd['stderr']['path']] = b'failure'
                if mutation == 'library': value['libraries'].pop()
                if mutation == 'boot': value['boot_id'] = 'other boot'
                if mutation == 'binary': value['binary'] = pin('/old/binary')
                with mock.patch.object(P, 'document', side_effect=lambda p, r, maximum: documents[r['path']]), \
                        mock.patch.object(P, 'read', side_effect=lambda p, r, *a, **kw: bodies[r['path']]), \
                        self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                    P.runtime_review(value, binary, reviewed, None)

    def test_guard_rehashes_deployed_bytes_and_rejects_library_alias_change(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary); old = directory / 'old'; new = directory / 'new'; alias = directory / 'alias'
            old.write_bytes(b'old'); new.write_bytes(b'new'); alias.symlink_to(old)
            pins = P.D.Pins(); record = pins.pin(old)
            c = dict(pins=pins, runtime=dict(libraries=[dict(path=str(alias), resolved=record)]))
            P.guard(c)
            alias.unlink(); alias.symlink_to(new)
            with self.assertRaises(RuntimeError): P.guard(c)
            c['runtime']['libraries'] = []; old.write_bytes(b'bad')
            with self.assertRaises(RuntimeError): P.guard(c)

    def test_preparation_and_load_recompute_six_requests_and_all_provenance(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary); output = directory / 'prefix-independent-prepared-v228-v1'
            given = inputs(); deployed, verified = deployment(); input_pin = pin('/input/actual.json')
            def context(record, destination):
                self.assertEqual(record, input_pin)
                return dict(pins=P.D.Pins(), inputs=given, input_pin=input_pin, output=destination,
                    requests=[(dict(case=case), {}) for case in P.V.CASES], runtime=dict(libraries=[]),
                    provenance=P.provenance(given, deployed, verified), baseline_manifest=pin('/baseline/closure.json'))
            with mock.patch.object(P, 'E', directory), mock.patch.object(P, 'context', side_effect=context), \
                    mock.patch.object(P.shutil, 'disk_usage', return_value=mock.Mock(free=40 << 30)):
                record = P.prepare(input_pin, output); c = P.load(record)
                self.assertEqual(len(c['prepared']['cases']), 6)
                self.assertFalse(c['prepared']['gpu_execution'])
                self.assertNotIn('historical_image_relabelled_as_current', c['prepared'])
                with self.assertRaises(RuntimeError): P.prepare(input_pin, output)
                changed = json.loads((output / 'complete.json').read_bytes()); changed['compiler_complete'] = changed['native_complete']
                (output / 'complete.json').write_text(json.dumps(changed))
                with self.assertRaises(RuntimeError): P.load(P.D.read_file(output / 'complete.json')[0])

    def test_missing_authority_refuses_before_preparation_output_exists(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary); output = directory / 'prefix-independent-prepared-v228-v1'
            with mock.patch.object(P, 'E', directory), \
                    mock.patch.object(P.shutil, 'disk_usage', return_value=mock.Mock(free=40 << 30)), \
                    mock.patch.object(P, 'context', side_effect=RuntimeError('actual source/archive/review missing')), \
                    self.assertRaises(RuntimeError):
                P.prepare(pin('/input/actual.json'), output)
            self.assertFalse(output.exists())

    def test_prepared_request_mutation_refuses_without_executing_any_native_code(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary); output = directory / 'prefix-independent-prepared-v228-v1'
            given = inputs(); deployed, verified = deployment(); input_pin = pin('/input/actual.json')
            def context(record, destination):
                return dict(pins=P.D.Pins(), inputs=given, input_pin=record, output=destination,
                    requests=[(dict(case=case), {}) for case in P.V.CASES], runtime=dict(libraries=[]),
                    provenance=P.provenance(given, deployed, verified), baseline_manifest=pin('/baseline/closure.json'))
            with mock.patch.object(P, 'E', directory), mock.patch.object(P, 'context', side_effect=context), \
                    mock.patch.object(P.shutil, 'disk_usage', return_value=mock.Mock(free=40 << 30)):
                record = P.prepare(input_pin, output)
                (output / (P.V.CASES[0] + '.json')).write_text('{"case":"wrong"}')
                with self.assertRaises(RuntimeError): P.load(record)

    def test_runtime_baseline_and_guard_helpers_remain_exact_preimage_ast(self):
        root = Path(__file__).resolve().parent
        def functions(path):
            return {node.name: ast.dump(node, include_attributes=False)
                    for node in ast.parse(path.read_text()).body if isinstance(node, ast.FunctionDef)}
        before, after = functions(root / 'preimage/prepare.py'), functions(root / 'prepare.py')
        for name in ('bootstrap', 'read', 'document', 'save', 'make_request', 'runtime_review',
                     'platform_review', 'baseline_inputs', 'guard'):
            self.assertEqual(before[name], after[name], name)


if __name__ == '__main__':
    unittest.main()
