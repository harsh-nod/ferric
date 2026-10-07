"""One bounded, GPU-hidden run of all ninety-eight inherited and sixteen separate Tail V4 synthetic data tests."""
import ast
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-tail-checker-cpu-v228-v1')
OUT = ROOT / 'evidence'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
SOURCES = {
  "validate_readiness.py": [
    19800,
    "0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a"
  ],
  "readiness_announcement.py": [
    3246,
    "b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1"
  ],
  "test_readiness.py": [
    17802,
    "4b644462a71120ee45b3351757ecb0d5867c9453daadb6a10c07bdbcd38e8bb5"
  ],
  "validate_shared.py": [
    8015,
    "6557fe5c082b2c92bae15274dd0d19bba3da8c3c36b9fc74757b4c1b74a87eca"
  ],
  "test_shared.py": [
    10737,
    "a122f42f8a47a0213010f1812ef1f95fa7ff582db6a9ffa190356a35874f6b96"
  ],
  "validate_timing.py": [
    10104,
    "27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f"
  ],
  "test_timing.py": [
    10433,
    "630febf4d840081887692144cc570d70c1d67f9ad3bb5bab1fd6d7d1f61cedb1"
  ],
  "validate_matched.py": [
    9362,
    "d31c94a4dd1f254d166be185fe1cca10bc4eed26e500a0cb1fc3c5a74a574437"
  ],
  "test_matched.py": [
    15400,
    "d5a6b522a1effe29e1ee08430754163cc217cf34c3713ce234926b7c860f9647"
  ],
  "validate_scoped.py": [
    9184,
    "c2541e11e534cf1e5a4faade31fe4a734726552171fed9617d6ea938f680b6fd"
  ],
  "test_scoped.py": [
    15936,
    "4c015cdfa072943960e0ffa1f25e0e604790e8dbe222875b4bed207be6563fdc"
  ],
  "validate_scoped_pair.py": [
    3233,
    "495fc476454e4052467810b55e177f9d2b91d86d3496a6b90876963f7f60ac62"
  ],
  "test_scoped_pair.py": [
    3629,
    "aa95ff79fd3c72ee3658d66e4117738a06f7984a2a77dcc6712b28a272e57f8f"
  ],
  "validate_bank_scoped.py": [
    9534,
    "2432fc14c0ba40fc934abcd7886459c12fc31ca6167ba4cf908a8c7974d26772"
  ],
  "validate_bank_pair.py": [
    4428,
    "9d3f1470f6e9d7a17f3c91d1deb29a54c6fa7e8c766dc358e9d7d73e15c4e004"
  ],
  "test_bank_scoped.py": [
    14853,
    "3ecc622e660cb6e0790a18ff8bb022a84535c91466d02d6fdbb8625ae080a99b"
  ],
  "validate_census.py": [
    10392,
    "090431e59a481d5b681311d5a015ddfd35705cc828c920118d4f0043b862ec9f"
  ],
  "validate_census_pair.py": [
    4730,
    "d4f50afe412b68c4066e934a80b96b5a5c0f83ef8ea33c59c53380aa1ab819c4"
  ],
  "test_census.py": [
    19688,
    "aca29934c68523dbc91b30439a2c7a5ae552b64a9faf077ddb99b97c861cb5b0"
  ],
  "validate_tail.py": [
    10447,
    "696e55d4bebea97ab82e5b613a76978c5e191087789d299551c304527e598d49"
  ],
  "validate_tail_pair.py": [
    4883,
    "ab69bcafcc245b8eca05d7e625c42eec7aaec2de4084e719090de1894f4d6995"
  ],
  "test_tail.py": [
    19524,
    "2235e1c4b97b396094df484ccaedb25619f70c757ed1656e5eaf6b92f8140c4a"
  ]
}
ROSTERS = {
  "test_readiness": [
    "ReadinessTests",
    [
      "test_all144_pages_and_page_boundary_are_closed",
      "test_authentic_prompt_only_not_argmax_feedback",
      "test_bank_retirement_local_generation_and_bool_are_checked",
      "test_capture_path_digest_order_and_truncation",
      "test_close_generated_tokens_and_native_retirement",
      "test_complete40_original_chain_and_four_selected_payloads",
      "test_extra_fields_authority_and_observer_stderr_refused",
      "test_frame_order_missing_and_extra_are_refused",
      "test_full2303_and_legacy_profiles_are_refused",
      "test_marker_alone_never_substitutes_owned_lineage",
      "test_original_readiness_profile_and_schema_are_refused",
      "test_payload_finite_lowest_tie_and_signed_zero",
      "test_readiness_marker_is_distinct_and_unique",
      "test_selected_guard_and_queue_words_are_independently_checked",
      "test_stream_and_retention_accounting_are_exact",
      "test_strict_json_scalars_and_boolean_identity",
      "test_unselected_hashes_and_chains_remain_authenticated"
    ]
  ],
  "test_shared": [
    "SharedTests",
    [
      "test_canonical_single_record_no_suffix_duplicates_or_reordering",
      "test_complete_policy_keeps_original_stderr_and_ordinary_contract",
      "test_default_empty_stderr_and_shared_opt_in_are_not_interchangeable",
      "test_every_policy_flag_is_closed_and_bool_is_not_integer",
      "test_exact_extent_captures_schema_and_no_extra_fields",
      "test_identity_scope_hash_and_device_order_are_joined",
      "test_parity_compares_all40_records_and_all_four_payloads",
      "test_parity_ignores_only_independently_admitted_control_timer_bytes",
      "test_parity_refuses_same_session_model_drift_or_numerical_authority",
      "test_pinned_original_bytes_and_all40_chain_still_required",
      "test_real_close_and_position5_only_precede_policy_admission",
      "test_wrapper_cannot_relabel_ordinary_timing_or_different_policy"
    ]
  ],
  "test_timing": [
    "TimingTests",
    [
      "test_close_and_false_authority_are_required",
      "test_complete_timeline_zero_spans_and_large_integer_precision",
      "test_exact_retention_accounting_and_unchanged_bound",
      "test_gaps_overlap_regression_and_duration_are_refused",
      "test_order_extent_and_forward_sum_are_closed",
      "test_parity_accepts_distinct_sessions_and_rejects_any_record_change",
      "test_parity_compares_payload_not_control_timer_bytes",
      "test_parity_rejects_same_session_model_drift_and_authority",
      "test_report_hashes_scope_and_capture_set_are_joined",
      "test_sidecar_actual_body_path_hash_and_size_are_joined",
      "test_strict_u64_refuses_bool_float_negative_and_overflow",
      "test_wrapper_and_ordinary_admission_are_not_interchangeable"
    ]
  ],
  "test_matched": [
    "MatchedTests",
    [
      "test_both_original_modes_join_policy_timeline_and_false_authority",
      "test_combined_policy_joins_original_worker_transcript_and_flags",
      "test_default_empty_stderr_and_shared_original_record_remain_separate",
      "test_explicit_mode_and_exact_wrapper_cannot_be_inferred_or_relabelled",
      "test_full_ordinary_frames_captures_and_close_precede_timing",
      "test_matched_pair_requires_same_all_four_cpu_and_product_pins",
      "test_original_policy_missing_suffix_changed_bytes_and_budgets_refuse",
      "test_pair_checks_every_semantic_record_and_complete_selected_payload",
      "test_pair_refuses_same_worker_identity_or_same_mode",
      "test_shared_sidecar_actual_path_hash_extent_and_ordinary_pin",
      "test_shared_timeline_strict_scalars_order_and_exact_disjoint_sum",
      "test_shared_timing_retention_close_and_false_authority_are_not_relaxed"
    ]
  ],
  "test_scoped": [
    "ScopedTests",
    [
      "test_counter_census_and_checked_relations_refuse_missing_warm_calls",
      "test_counter_scalars_are_exact_unsigned_integers_not_bool_float_or_overflow",
      "test_default_shared_and_scoped_policy_routes_are_not_interchangeable",
      "test_every_policy_flag_and_execution_identity_is_closed",
      "test_explicit_wrapper_and_mode_cannot_relabel_ordinary_or_shared",
      "test_large_valid_poll_dependent_counters_preserve_all_integer_bits",
      "test_ordinary_and_timed_scoped_keep_original_policy_bytes_without_shared_label",
      "test_original_pinned_stderr_all40_frames_four_captures_and_retention_stay_required",
      "test_parity_checks_every_semantic_record_and_all_four_complete_payloads",
      "test_parity_rejects_relabelled_policy_or_another_admitted_summary",
      "test_policy_requires_single_canonical_ordered_bounded_record",
      "test_real_close_and_position5_are_required_not_full_or_old_readiness",
      "test_record_joins_bootstrap_session_devices_worker_profile_and_transcript",
      "test_timed_route_preserves_all124_spans_exact_pins_caps_and_false_authority"
    ]
  ],
  "test_scoped_pair": [
    "ScopedPairTests",
    [
      "test_pair_rejects_wrong_mode_summary_timeline_and_original_policy_body",
      "test_pair_requires_each_same_actual_cpu_and_product_pin",
      "test_same_binary_pair_preserves_original_policy_and_host_timing_without_authority"
    ]
  ],
  "test_bank_scoped": [
    "BankScopedTests",
    [
      "test_all_policy_flags_identity_close_and_profile_remain_closed",
      "test_bank_policy_is_one_canonical_bounded_record_not_v1_or_shared",
      "test_bank_scalar_types_generations_and_every_counter_relation_are_strict",
      "test_large_integer_counters_remain_exact_and_overflow_is_refused",
      "test_layer_counts_cannot_be_replaced_by_bank_counts_or_weakened",
      "test_original40_chain_payloads_policy_accounting_and_timing_are_required",
      "test_original_bank_record_retains_separate_layers_banks_and124_spans",
      "test_pair_checks_every_semantic_record_and_complete_selected_payload",
      "test_pair_rechecks_both_original_policy_files_and_no_relabelled_authority",
      "test_pair_requires_same_all_cpu_product_pins_and_distinct_cases",
      "test_same_elf_scoped_control_and_bank_candidate_preserve_both_original_policies",
      "test_timing_byte_cap_and_native_close_cannot_be_upgraded_by_wrapper"
    ]
  ],
  "test_census": [
    "CensusTests",
    [
      "test_census_all_policy_flags_identity_close_and_profile_remain_closed",
      "test_census_bank_policy_is_one_canonical_bounded_record_not_v1_or_shared",
      "test_census_bank_scalar_types_generations_and_every_counter_relation_are_strict",
      "test_census_every_subset_scalar_and_nested_field_is_strict",
      "test_census_exact_owner_assertion_is_ranked_non_authoritative_and_not_equalized",
      "test_census_large_integer_counters_remain_exact_and_overflow_is_refused",
      "test_census_layer_counts_cannot_be_replaced_by_bank_counts_or_weakened",
      "test_census_original40_chain_payloads_policy_accounting_and_timing_are_required",
      "test_census_original_bank_record_retains_separate_layers_banks_and124_spans",
      "test_census_pair_checks_every_semantic_record_and_complete_selected_payload",
      "test_census_pair_rechecks_both_original_policy_files_and_no_relabelled_authority",
      "test_census_pair_refuses_subset_relabel_and_each_forward_timeline_gap",
      "test_census_pair_requires_same_all_cpu_product_pins_and_distinct_cases",
      "test_census_same_elf_bank_control_and_census_candidate_preserve_both_original_policies",
      "test_census_subset_is_not_added_twice_and_remaining_layer_work_is_required",
      "test_census_timing_byte_cap_and_native_close_cannot_be_upgraded_by_wrapper"
    ]
  ],
  "test_tail": [
    "TailTests",
    [
      "test_tail_all_counter_fields_and_fixed_extent_relations_are_closed",
      "test_tail_and_old_policy_routes_cannot_admit_each_other",
      "test_tail_case_and_pair_nonclaims_cannot_be_promoted_to_native_acceptance",
      "test_tail_counts_never_replace_or_relax_inherited_layer_bank_census",
      "test_tail_one_original_canonical_bounded_record_is_required",
      "test_tail_original40_chain_capture_accounting_and_timing_are_required",
      "test_tail_original_record_keeps_separate_counters_bytes_and124_spans",
      "test_tail_pair_checks_all40_semantic_records_and_four_complete_payloads",
      "test_tail_pair_preserves_original_v3_v4_policies_and_exact_payload_parity",
      "test_tail_pair_rechecks_both_original_files_and_policy_authority",
      "test_tail_pair_requires_all_four_same_cpu_product_identities",
      "test_tail_same_side_requires_independent_scope_flags_and_rehashes_policy",
      "test_tail_scalar_types_widths_and_checked_overflow_are_strict",
      "test_tail_scope_flags_identities_close_and_profile_cannot_be_relabelled",
      "test_tail_variable_periodic_checks_and_large_integers_are_exact",
      "test_tail_wrapper_cannot_replace_policy_or_timing_and_bounds"
    ]
  ]
}
NAMES = tuple(sorted(module + "." + cls + "." + name
    for module, (cls, names) in ROSTERS.items() for name in names))
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
WHOLE_SECONDS, LEAF_SECONDS, CLEANUP_SECONDS = 180, 120, 50


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def load_supervisor():
    path = ROOT / 'supervisor.py'; before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 1 << 20, 'ordinary bounded supervisor')
    with path.open('rb') as stream:
        raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size
            and hashlib.sha256(raw).hexdigest() == SUPERVISOR_SHA, 'frozen supervisor bytes')
    module = types.ModuleType('readiness40_checker_owned'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def interrupted(number, _frame):
    raise RuntimeError('controller signal ' + str(number))


def main():
    start = time.monotonic(); deadline = start + WHOLE_SECONDS
    def arm_deadline():
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'whole CPU deadline exhausted; retain failed prefix only')
        signal.setitimer(signal.ITIMER_REAL, remaining)
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B run_cpu.py only')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT and not os.path.lexists(OUT),
            'fresh exact output namespace')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged CPU host')
    require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', *SOURCES}, 'closed twenty-four-file input')
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice level')
    if priority == 0: os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = load_supervisor()
    h.OUT = OUT; h.AS_LIMIT = 512 << 20; h.FILE_LIMIT = 16 << 20
    h.STREAM_LIMIT = 4 << 20; h.CPU_LIMIT = 120; h.CACHE_LIMIT = 4 << 20
    h.CLEANUP_RESERVE = CLEANUP_SECONDS
    require(h.shutil.disk_usage(ROOT).free >= h.START_FREE, 'initial40GiB free floor')
    paths = [ROOT / name for name in sorted({'run_cpu.py', 'supervisor.py', *SOURCES})]
    def sources():
        require(all(p.resolve(strict=True) == p for p in paths), 'canonical CPU inputs')
        return {p.name: h.pin(p) for p in paths}
    h.sources = sources
    before = sources()
    for name, (size, sha) in SOURCES.items():
        require((before[name]['bytes'], before[name]['sha256']) == (size, sha), 'exact readiness40 validator/marker/test source')
    for module, (class_name, names) in ROSTERS.items():
        tree = ast.parse((ROOT / (module + '.py')).read_bytes())
        classes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
        require(len(classes) == 1 and classes[0].name == class_name
                and tuple(sorted(node.name for node in classes[0].body if isinstance(node, ast.FunctionDef)
                    and node.name.startswith('test_'))) == tuple(names), 'closed source-level test rosters')
    python = Path('/usr/bin/python3').resolve(strict=True)
    prlimit = Path('/usr/bin/prlimit').resolve(strict=True)
    tools = {'python': h.pin(python), 'prlimit': h.pin(prlimit)}
    environment = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        PYTHONPATH='', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    script = ('import sys,unittest;sys.path.insert(0,' + repr(str(ROOT)) + ');'
              'suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(name) '
              'for name in ("test_readiness","test_shared","test_timing","test_matched","test_scoped","test_scoped_pair","test_bank_scoped","test_census","test_tail"));'
              'result=unittest.TextTestRunner(verbosity=2).run(suite);sys.exit(not result.wasSuccessful())')
    argv = [str(python), '-I', '-B', '-c', script]
    OUT.mkdir(mode=0o700); phases = []; errors = []; failure = None; census = None; after = None
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for number in handlers: signal.signal(number, interrupted)
    try:
        h.save('sources-before.json', before)
        h.run('readiness-tests', argv, environment, phases, deadline, before,
              seconds=LEAF_SECONDS, cwd=ROOT)
        stdout = (OUT / 'readiness-tests.stdout').read_bytes()
        stderr = (OUT / 'readiness-tests.stderr').read_text()
        require(stdout == b'', 'unexpected readiness test stdout')
        expression = r'^(test_[A-Za-z0-9_]+) \((test_readiness\.ReadinessTests|test_shared\.SharedTests|test_timing\.TimingTests|test_matched\.MatchedTests|test_scoped\.ScopedTests|test_scoped_pair\.ScopedPairTests|test_bank_scoped\.BankScopedTests|test_census\.CensusTests|test_tail\.TailTests)\.\1\) \.\.\. ok$'
        found = [prefix + '.' + name for name, prefix in re.findall(expression, stderr, re.M)]
        require(tuple(sorted(found)) == NAMES and len(found) == len(set(found)) == 114,
                'exact one hundred fourteen named passing tests')
        tail = re.sub(expression, '', stderr, flags=re.M)
        tail = '\n'.join(line for line in tail.splitlines() if line)
        require(re.fullmatch(r'-{70}\nRan 114 tests in [0-9]+\.[0-9]+s\nOK', tail),
                'exact unittest summary, no skips/errors')
        census = dict(names=list(NAMES), passed=114, failed=0, errors=0, skipped=0)
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    finally:
        for number in handlers:
            signal.signal(number, interrupted if number == signal.SIGALRM else signal.SIG_IGN)
        arm_deadline()
        try:
            after = sources(); require(after == before, 'CPU source drift')
            h.save('sources-after.json', after)
            require(Path('/usr/bin/python3').resolve(strict=True) == python
                    and Path('/usr/bin/prlimit').resolve(strict=True) == prlimit
                    and h.pin(python) == tools['python'] and h.pin(prlimit) == tools['prlimit'], 'CPU tool drift')
            require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', 'evidence', *SOURCES},
                    'unexpected bytecode/cache/input-root write')
            require(len(phases) == 1 and all(row['reaped'] and row['process_group_absent'] for row in phases),
                    'sole child completely reaped')
            require(time.monotonic() < deadline, 'whole CPU bound')
        except BaseException as error:
            if time.monotonic() >= deadline:
                raise
            errors.append(type(error).__name__ + ': ' + str(error))
    arm_deadline()
    raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    try:
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'final raw phase pin join')
            require(json.loads((OUT / 'readiness-tests.result.json').read_bytes()) == row, 'original phase result join')
            started = json.loads((OUT / 'readiness-tests.started.json').read_bytes())
            require(started == dict(pid=row['pid'], pgid=row['pgid'], argv=row['argv']), 'original child registration join')
        require(set(raw) <= {'sources-before.json', 'sources-after.json', 'readiness-tests.command.json',
            'readiness-tests.started.json', 'readiness-tests.result.json', 'readiness-tests.stdout', 'readiness-tests.stderr'},
            'closed original raw prefix')
    except BaseException as error:
        if time.monotonic() >= deadline:
            raise
        errors.append('raw reconciliation: ' + repr(error))
    failure = failure or ('postcheck failed' if errors else None)
    value = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-census-tail-checker-cpu-v1', passed=failure is None and census is not None,
        failure=failure, postcheck_errors=errors, controller=before['run_cpu.py'], supervisor=before['supervisor.py'],
        sources_before=before, sources_after=after, source_unchanged=after == before,
        tool_pins=tools, environment=environment, phases=phases, tests=census, raw=raw,
        limits=dict(whole_seconds=WHOLE_SECONDS, test_seconds=LEAF_SECONDS, cleanup_reserve_seconds=CLEANUP_SECONDS,
            address_space_bytes=512 << 20, stream_bytes=4 << 20, affinity=[8, 9], nice=10),
        elapsed_seconds=time.monotonic() - start, synthetic_data_tests_only=True,
        gpu_execution=False, native_parent_execution=False, model_execution=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False, production_authority=False)
    arm_deadline()
    h.save('complete.json' if value['passed'] else 'failed.json', value)
    print(json.dumps({key: value[key] for key in ('passed', 'failure', 'postcheck_errors', 'tests')}, sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items(): signal.signal(number, handler)
    return 0 if value['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
