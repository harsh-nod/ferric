//! CPU protocol fixtures only. The separate actual-image fixture covers packing.
#[path = "../token_program_abi_fixture.rs"]
mod abi_fixture;
#[path = "tp_worker_down_actual_tests.rs"]
mod down_actual;
#[path = "tp_worker_down_tests.rs"]
mod down_tests;
#[path = "tp_worker_prefill32_actual_tests.rs"]
mod prefill32_actual;
#[path = "tp_worker_prefill32_lifecycle_tests.rs"]
mod prefill32_lifecycle;
use super::super::{Command, Digest, Duration, EngineeringTpBufferAccessV1, Read, Sha256, Stdio};
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn fixture() -> Plan {
    Plan {
        definition: TokenProgramDefinitionV1 {
            dispatches: vec![
                OrderedBatchDispatchV1 {
                    kernel: 1,
                    payload_bytes: 4,
                    workgroup: [64, 1, 1],
                    grid: [64, 1, 1],
                    pointers: Vec::new()
                };
                PACKETS
            ],
            slots: (0..SLOTS)
                .map(|dispatch| Slot::ScalarU32 {
                    dispatch: u16::try_from(dispatch).unwrap(),
                    offset: 0,
                    minimum: 0,
                    maximum: 8192,
                })
                .collect(),
        },
        kernargs: vec![0; PACKETS * 4],
        updates: vec![Update::ScalarU32 { value: 1 }; SLOTS],
    }
}

const CHILD: &str = r"
import json, struct, sys, time
mode, path = sys.argv[1:]
trace = open(path, 'x', buffering=1)
def send(value):
    raw = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I',len(raw)) + raw)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-','device_unique_id':1,'authority':'none'})
program = 0
epoch = 0
counter_queries = 0
program_executions = 0
program_dispatches = 0
while True:
    prefix = sys.stdin.buffer.read(4)
    if not prefix: sys.exit(0)
    command = json.loads(sys.stdin.buffer.read(struct.unpack('<I',prefix)[0]))
    op = command['op']
    trace.write(op + '\n')
    if op == 'describe_token_program_backend_v1':
        if mode == 'backend_stall': time.sleep(60)
        backend = {'backend_native':'native-whole-program-v1', 'backend_disabled':'disabled',
                   'backend_slots512':'native-whole-program-slots512-v1',
                   'backend_unknown':'future-backend'}.get(mode, 'ordered64-groups-v1')
        value = {'op':'token_program_backend_v1', 'backend':backend}
        if mode == 'backend_kind': value = {'op':'performance_configured'}
        send(value)
    elif op == 'token_program_snapshot_v1':
        native = mode.startswith('counter_native') or mode.startswith('counter_prefill')
        backend = 'native-whole-program-v1' if native else 'ordered64-groups-v1'
        if mode.startswith('counter_prefill_width'): backend = 'native-whole-program-slots512-v1'
        executions = 0 if counter_queries == 0 else 1
        if counter_queries and mode == 'counter_stale': executions = 0
        if counter_queries and mode == 'counter_wrong_total': executions = 2
        if mode == 'counter_initial_nonzero': executions = 1
        groups = 1 if native else 11
        counters = {'executions':executions, 'dispatches':executions*652,
                    'publications':executions*groups, 'final_waits':executions*groups,
                    'retirement_signals':executions*652, 'staging_ns':executions,
                    'kernarg_initialized_bytes':executions*652*(256 if native else 65536+256)}
        if mode.startswith('counter_prefill'):
            counters = {'executions':program_executions,'dispatches':program_dispatches,
                        'publications':program_executions,'final_waits':program_executions,
                        'retirement_signals':program_dispatches,'staging_ns':program_executions,
                        'kernarg_initialized_bytes':program_dispatches*256}
            if counter_queries and mode in ('counter_prefill_bad', 'counter_prefill_width_bad'): counters['dispatches'] += 1
        if mode == 'counter_wrong_backend': backend = 'native-whole-program-v1'
        if mode == 'counter_bad_publications': counters['publications'] += 1
        if mode == 'counter_bad_waits': counters['final_waits'] += 1
        if mode == 'counter_bad_signals': counters['retirement_signals'] += 1
        if mode == 'counter_bad_dispatches': counters['dispatches'] += 1
        value = {'op':'token_program_snapshot_v1', 'backend':backend, 'counters':counters}
        if mode == 'counter_kind': value = {'op':'performance_configured'}
        send(value)
        counter_queries += 1
    elif op in ('register_token_program', 'register_token_program_slots512_v1'):
        size = command['definition_bytes'] + command['kernarg_bytes']
        raw = sys.stdin.buffer.read(size)
        if len(raw) != size: sys.exit(4)
        definition = json.loads(raw[:command['definition_bytes']])
        program += 1
        value = {'op':'token_program_registered','program':program,'device_unique_id':1,'queue_epoch':epoch,
                 'dispatches':len(definition['dispatches']), 'slots':len(definition['slots'])}
        if mode.startswith('registration_'):
            field = mode.removeprefix('registration_')
            value[field] = 0 if field == 'program' else value[field]+1
        send(value)
    elif op in ('execute_token_program', 'execute_token_program_slots512_v1'):
        program_executions += 1
        program_dispatches += len(definition['dispatches'])
        if mode == 'stall': time.sleep(60)
        if mode == 'partial':
            send({'op':'error','message':'second native group failed after first retirement','fatal':True})
            continue
        value = {'op':'token_program_completed','program':command['program'],'device_unique_id':1,
                 'queue_epoch':command['expected_epoch'],'completed_dispatches':len(definition['dispatches']),
                 'completed_packets':command['expected_completed_packets']+len(definition['dispatches']),'elapsed_ns':1}
        fields = {'program':'program','device':'device_unique_id','epoch':'queue_epoch','count':'completed_dispatches','frontier':'completed_packets'}
        if mode in fields: value[fields[mode]] += 1
        if mode == 'kind': value = {'op':'dispatch_ordered_batch64_completed','completed_dispatches':652,'elapsed_ns':1}
        send(value)
    elif op == 'release_token_program':
        send({'op':'token_program_released','program':command['program'], 'queue_epoch':command['expected_epoch'] + (1 if mode == 'release' else 0)})
    elif op == 'rollover_queue':
        epoch = command['expected_epoch']+1
        send({'op':'queue_rolled_over','retired_packets':command['expected_completed_packets'],'queue_epoch':epoch})
    elif op == 'dispatch_ordered_batch64':
        size = sum(item['payload_bytes'] for item in command['dispatches'])
        if len(sys.stdin.buffer.read(size)) != size: sys.exit(4)
        send({'op':'dispatch_ordered_batch64_completed','completed_dispatches':len(command['dispatches']),'elapsed_ns':1})
    elif op == 'close':
        send({'op':'closed'})
        sys.exit(0)
    else: sys.exit(5)
";

fn worker(mode: &str) -> (Worker, std::path::PathBuf) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "ferric-token-client-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let child = Command::new("python3")
        .args(["-I", "-B", "-u", "-c", CHILD, mode])
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut worker = Worker::connect(child, 1, Duration::from_secs(5)).unwrap();
    worker.options.ordered64 = true;
    worker.options.ordered_batches = true;
    worker.options.rollover = true;
    worker.token_program = Some(Box::new(State {
        registered: None,
        backend: TokenProgramBackend::Ordered64GroupsV1,
        counter_snapshots: 0,
        completed_executions: 0,
        prefill_enabled: false,
        prefill_rows: 16,
        completed_prefills: 0,
        registrations: 0,
        releases: 0,
        down_selected: None,
        decode_shape: Shape::Decode,
    }));
    (worker, path)
}

fn submit(worker: &mut Worker, plan: Plan) -> TpResult<()> {
    let immutable = plan.immutable()?;
    worker.submit_plan(plan, immutable)
}

fn prefill_fixture() -> Plan {
    let mut plan = fixture();
    plan.definition.dispatches.truncate(613);
    plan.kernargs.truncate(613 * 4);
    for dispatch in SLOTS..216 {
        plan.definition.slots.push(Slot::ScalarU32 {
            dispatch: dispatch as u16,
            offset: 0,
            minimum: 0,
            maximum: 8192,
        });
        plan.updates.push(Update::ScalarU32 { value: 1 });
    }
    plan
}

fn select_native_prefill(worker: &mut Worker) {
    let state = worker.token_program.as_mut().unwrap();
    state.backend = TokenProgramBackend::NativeWholeProgramV1;
    state.prefill_enabled = true;
}

fn select_native_width(worker: &mut Worker, rows: u32) {
    assert!(matches!(rows, 16 | 32));
    let state = worker.token_program.as_mut().unwrap();
    state.backend = TokenProgramBackend::NativeWholeProgramSlots512V1;
    state.prefill_enabled = true;
    state.prefill_rows = rows;
}

fn prefill32_fixture() -> Plan {
    let mut plan = fixture();
    plan.definition.dispatches.truncate(649);
    plan.kernargs.truncate(649 * 4);
    for dispatch in SLOTS..396 {
        plan.definition.slots.push(Slot::ScalarU32 {
            dispatch: u16::try_from(dispatch).unwrap(),
            offset: 0,
            minimum: 0,
            maximum: 8192,
        });
        plan.updates.push(Update::ScalarU32 { value: 1 });
    }
    plan
}

#[test]
fn prefill_shape_transitions_release_before_registration_and_reuse_within_phase() {
    let (mut worker, path) = worker("normal");
    select_native_prefill(&mut worker);
    for shape in [
        Shape::Prefill16,
        Shape::Prefill16,
        Shape::Decode,
        Shape::Prefill16,
    ] {
        let plan = if shape == Shape::Decode {
            fixture()
        } else {
            prefill_fixture()
        };
        let immutable = plan.immutable().unwrap();
        worker.submit_shape(plan, immutable, shape).unwrap();
        worker.wait_fixed_token(shape.packets()).unwrap();
    }
    let state = worker.token_program.as_ref().unwrap();
    assert_eq!(
        (state.completed_prefills, state.completed_executions),
        (3, 1)
    );
    assert_eq!((state.registrations, state.releases), (3, 2));
    assert_eq!(worker.queue_packets, 3 * 613 + 652);
    worker.close().unwrap();
    assert_eq!(
        std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        [
            "register_token_program",
            "execute_token_program",
            "execute_token_program",
            "release_token_program",
            "register_token_program",
            "execute_token_program",
            "release_token_program",
            "register_token_program",
            "execute_token_program",
            "release_token_program",
            "close"
        ]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn prefill_shape_rejects_missing_selection_pending_wrong_slot_and_same_shape_drift() {
    for mutation in 0..5 {
        let (mut worker, path) = worker("normal");
        if mutation != 0 {
            select_native_prefill(&mut worker);
        }
        if mutation >= 2 {
            let plan = prefill_fixture();
            let immutable = plan.immutable().unwrap();
            worker
                .submit_shape(plan, immutable, Shape::Prefill16)
                .unwrap();
            worker.wait_fixed_token(613).unwrap();
            if mutation == 2 {
                worker.pending = Some(PendingRequest::TokenExecute {
                    program: 1,
                    epoch: 0,
                    next: 1226,
                });
            }
        }
        let before = std::fs::read_to_string(&path).unwrap();
        let mut plan = prefill_fixture();
        match mutation {
            1 => {
                plan.definition.slots.pop();
            }
            3 => plan.definition.dispatches[612].grid[0] += 64,
            4 => {
                worker.token_program.as_mut().unwrap().backend =
                    TokenProgramBackend::Ordered64GroupsV1
            }
            _ => {}
        }
        let immutable = plan.immutable().unwrap();
        assert!(
            worker
                .submit_shape(plan, immutable, Shape::Prefill16)
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn prefill_failed_release_does_not_register_next_shape() {
    let (mut worker, path) = worker("release");
    select_native_prefill(&mut worker);
    let plan = prefill_fixture();
    let immutable = plan.immutable().unwrap();
    worker
        .submit_shape(plan, immutable, Shape::Prefill16)
        .unwrap();
    worker.wait_fixed_token(613).unwrap();
    assert!(submit(&mut worker, fixture()).is_err());
    assert_eq!(worker.queue_packets, 613);
    assert_eq!(
        std::fs::read_to_string(&path)
            .unwrap()
            .matches("register_token_program")
            .count(),
        1
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn prefill_invalid_graph_sends_nothing_even_after_successful_decode() {
    let (mut worker, path) = worker("normal");
    select_native_prefill(&mut worker);
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(652).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    assert!(worker.submit_fixed_prefill(&[]).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn prefill_counters_distinguish_phase_totals_and_reject_dispatch_drift() {
    for bad in [false, true] {
        let (mut worker, path) = worker(if bad {
            "counter_prefill_bad"
        } else {
            "counter_prefill"
        });
        select_native_prefill(&mut worker);
        worker.options.profile = true;
        worker.options.ordered64_runtime_counters = true;
        let initial = worker.token_program_counter_snapshot().unwrap();
        assert_eq!(initial["schema"], "FerricPrefillProgramCountersV1");
        assert_eq!(initial["program_phases"]["prefill16"]["executions"], 0);
        for shape in [Shape::Prefill16, Shape::Prefill16, Shape::Decode] {
            let plan = if shape == Shape::Decode {
                fixture()
            } else {
                prefill_fixture()
            };
            let immutable = plan.immutable().unwrap();
            worker.submit_shape(plan, immutable, shape).unwrap();
            worker.wait_fixed_token(shape.packets()).unwrap();
        }
        worker.release_registered_token().unwrap();
        let final_snapshot = worker.token_program_counter_snapshot();
        if bad {
            assert!(final_snapshot.is_err());
        } else {
            let snapshot = final_snapshot.unwrap();
            assert_eq!(snapshot["counters"]["executions"], 3);
            assert_eq!(snapshot["counters"]["dispatches"], 1878);
            assert_eq!(snapshot["counters"]["publications"], 3);
            assert_eq!(snapshot["program_phases"]["prefill16"]["executions"], 2);
            assert_eq!(snapshot["program_phases"]["decode_c1"]["executions"], 1);
            assert_eq!(snapshot["program_phases"]["registrations"], 2);
            assert_eq!(snapshot["program_phases"]["releases"], 2);
            // The test consumed both diagnostic phases explicitly.
            worker.options.ordered64_runtime_counters = false;
            worker.options.profile = false;
            worker.close().unwrap();
        }
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn token_backend_identity_is_positive_distinct_and_retained_before_any_model_command() {
    for (mode, backend) in [
        ("normal", TokenProgramBackend::Ordered64GroupsV1),
        ("backend_native", TokenProgramBackend::NativeWholeProgramV1),
    ] {
        let (mut worker, path) = worker(mode);
        worker.token_program = None;
        assert_eq!(worker.token_program_backend(), None);
        worker.verify_token_program_backend(backend).unwrap();
        assert_eq!(worker.token_program_backend(), Some(backend));
        assert!(worker.supports_token_program());
        assert!(worker.kernels.is_empty() && worker.buffers.is_empty());
        worker.close().unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "describe_token_program_backend_v1\nclose\n"
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn token_backend_identity_rejects_wrong_disabled_unknown_or_missing_reply_without_fallback() {
    for (mode, expected) in [
        ("normal", TokenProgramBackend::NativeWholeProgramV1),
        ("backend_native", TokenProgramBackend::Ordered64GroupsV1),
        (
            "backend_disabled",
            TokenProgramBackend::NativeWholeProgramV1,
        ),
        ("backend_unknown", TokenProgramBackend::NativeWholeProgramV1),
        ("backend_kind", TokenProgramBackend::NativeWholeProgramV1),
        ("backend_stall", TokenProgramBackend::NativeWholeProgramV1),
    ] {
        let (mut worker, path) = worker(mode);
        worker.token_program = None;
        if mode == "backend_stall" {
            worker.timeout = Duration::from_millis(100);
        }
        assert!(worker.verify_token_program_backend(expected).is_err());
        assert!(worker.failed && worker.exited);
        assert_eq!(worker.token_program_backend(), None);
        assert_eq!(worker.queue_packets, 0);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "describe_token_program_backend_v1\n"
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn token_backend_identity_cannot_be_reselected_or_queried_after_resource_use() {
    for allocated in [false, true] {
        let (mut worker, path) = worker("normal");
        if allocated {
            worker.token_program = None;
            worker.buffers.insert(1, 8);
        }
        assert!(
            worker
                .verify_token_program_backend(TokenProgramBackend::Ordered64GroupsV1)
                .is_err()
        );
        assert!(worker.failed && worker.exited);
        assert!(std::fs::read_to_string(&path).unwrap().is_empty());
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn token_counters_bind_both_backends_and_remain_separate_from_latency_runs() {
    for (mode, backend, publications) in [
        (
            "counter_baseline",
            TokenProgramBackend::Ordered64GroupsV1,
            11,
        ),
        (
            "counter_native",
            TokenProgramBackend::NativeWholeProgramV1,
            1,
        ),
    ] {
        let (mut worker, path) = worker(mode);
        worker.token_program.as_mut().unwrap().backend = backend;
        worker.options = worker.options.with_ordered64_runtime_counters().unwrap();
        let first = worker.token_program_counter_snapshot().unwrap();
        submit(&mut worker, fixture()).unwrap();
        worker.wait_fixed_token(PACKETS).unwrap();
        let second = worker.token_program_counter_snapshot().unwrap();
        assert_eq!(first["phase"], "worker_start");
        assert_eq!(first["ordinal"], 0);
        assert_eq!(first["counters"]["executions"], 0);
        assert_eq!(second["phase"], "before_close");
        assert_eq!(second["ordinal"], 1);
        assert_eq!(second["backend"], backend.identity());
        assert_eq!(second["counters"]["dispatches"], 652);
        assert_eq!(second["counters"]["publications"], publications);
        assert_eq!(second["counters"]["retirement_signals"], 652);
        assert_eq!(second["latency_sample_admitted"], false);
        assert_eq!(second["runtime_profiling"], true);
        assert_eq!(second["process_id"], worker.pid());
        assert!(worker.token_program_counter_snapshot().is_err());
        assert!(worker.failed && worker.exited);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "token_program_snapshot_v1\nregister_token_program\nexecute_token_program\ntoken_program_snapshot_v1\n"
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn token_counter_mechanism_drift_or_wrong_response_is_fatal() {
    for mode in [
        "counter_initial_nonzero",
        "counter_wrong_backend",
        "counter_bad_publications",
        "counter_bad_waits",
        "counter_bad_signals",
        "counter_bad_dispatches",
        "counter_kind",
    ] {
        let (mut worker, path) = worker(mode);
        worker.options = worker.options.with_ordered64_runtime_counters().unwrap();
        assert!(worker.token_program_counter_snapshot().is_err());
        assert!(worker.failed && worker.exited);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "token_program_snapshot_v1\n"
        );
        std::fs::remove_file(path).unwrap();
    }
    let (mut worker, path) = worker("normal");
    assert!(worker.token_program_counter_snapshot().is_err());
    assert!(worker.failed && worker.exited);
    assert!(std::fs::read_to_string(&path).unwrap().is_empty());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn token_counter_execution_total_must_match_locally_validated_completions() {
    for mode in ["counter_stale", "counter_wrong_total"] {
        let (mut worker, path) = worker(mode);
        worker.options = worker.options.with_ordered64_runtime_counters().unwrap();
        worker.token_program_counter_snapshot().unwrap();
        submit(&mut worker, fixture()).unwrap();
        worker.wait_fixed_token(PACKETS).unwrap();
        assert_eq!(
            worker.token_program.as_ref().unwrap().completed_executions,
            1
        );
        assert!(worker.token_program_counter_snapshot().is_err());
        assert!(worker.failed && worker.exited);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn token_state_storage_is_pointer_sized_and_worker_is_bounded() {
    let _: fn(&Worker) -> &Option<Box<State>> = |worker| &worker.token_program;
    assert_eq!(
        std::mem::size_of::<Option<Box<State>>>(),
        std::mem::size_of::<usize>()
    );
    #[cfg(target_pointer_width = "64")]
    assert!(std::mem::size_of::<Worker>() <= 272);
}

#[test]
fn positional_updates_preserve_geometry_extents_and_every_undeclared_byte() {
    let plan = fixture();
    let expected = plan.immutable().unwrap();
    let mut changed = fixture();
    changed.kernargs[..4].copy_from_slice(&7u32.to_le_bytes());
    changed.updates[0] = Update::ScalarU32 { value: 7 };
    assert_eq!(changed.immutable().unwrap(), expected);
    changed.kernargs[PACKETS * 4 - 1] = 1;
    assert_ne!(changed.immutable().unwrap(), expected);
    let mut changed = fixture();
    changed.definition.dispatches[PACKETS - 1].grid[0] += 64;
    assert_ne!(changed.immutable().unwrap(), expected);
}

#[test]
fn counted_encoding_matches_owned_for_full_graph_and_late_rejections() {
    for case in 0..6 {
        let mut candidate = fixture();
        match case {
            0 => {}
            1 => candidate.definition.dispatches[PACKETS - 1].payload_bytes += 1,
            2 => candidate.kernargs.push(0),
            3 => candidate
                .definition
                .slots
                .push(candidate.definition.slots[0].clone()),
            4 => {
                candidate.definition.slots[0] = Slot::ScalarU32 {
                    dispatch: PACKETS as u16,
                    offset: 0,
                    minimum: 0,
                    maximum: 1,
                };
            }
            5 => {
                candidate.definition.dispatches[PACKETS - 1].payload_bytes =
                    wire::MAX_KERNARG_BYTES_V1 + 1;
            }
            _ => unreachable!(),
        }
        let owned = wire::encode_token_program_v1(&candidate.definition, &candidate.kernargs)
            .map(|_| ())
            .map_err(|error| (error.kind(), error.to_string()));
        let counted =
            wire::validate_token_program_encoding_v1(&candidate.definition, &candidate.kernargs)
                .map_err(|error| (error.kind(), error.to_string()));
        assert_eq!(counted, owned, "case {case}");
        assert_eq!(counted.is_ok(), case == 0, "case {case}");
    }
}

#[test]
fn registration_is_reused_and_rollover_releases_before_reregistering() {
    let (mut worker, path) = worker("normal");
    for _ in 0..2 {
        submit(&mut worker, fixture()).unwrap();
        worker.wait_fixed_token(PACKETS).unwrap();
    }
    assert_eq!(worker.queue_packets, 1304);
    worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1;
    worker.prepare_packets(PACKETS as u64).unwrap();
    assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(PACKETS).unwrap();
    worker.close().unwrap();
    let events = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        events.lines().collect::<Vec<_>>(),
        [
            "register_token_program",
            "execute_token_program",
            "execute_token_program",
            "release_token_program",
            "rollover_queue",
            "register_token_program",
            "execute_token_program",
            "release_token_program",
            "close"
        ]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn ordinary_fallback_releases_before_publication_and_allows_reregistration() {
    let (mut worker, path) = worker("normal");
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(PACKETS).unwrap();
    let ordinary = fixture().definition.dispatches.remove(0);
    worker
        .send(
            CommandV1::DispatchOrderedBatch64 {
                dispatches: vec![ordinary],
                timeout_ms: DISPATCH_TIMEOUT_MS,
            },
            vec![0; 4],
        )
        .unwrap();
    worker.wait_ordered_batch(1).unwrap();
    assert_eq!(worker.queue_packets, 653);
    assert!(worker.token_program.as_ref().unwrap().registered.is_none());
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(PACKETS).unwrap();
    assert_eq!(worker.queue_packets, 1305);
    worker.close().unwrap();
    assert_eq!(
        std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        [
            "register_token_program",
            "execute_token_program",
            "release_token_program",
            "dispatch_ordered_batch64",
            "register_token_program",
            "execute_token_program",
            "release_token_program",
            "close"
        ]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn late_immutable_change_sends_no_command_and_never_advances_frontier() {
    let (mut worker, path) = worker("normal");
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(PACKETS).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    let mut late = fixture();
    late.kernargs[PACKETS * 4 - 1] = 1;
    assert!(submit(&mut worker, late).is_err());
    assert!(worker.failed && worker.exited);
    assert_eq!(worker.queue_packets, PACKETS as u64);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn aggregate_wrong_identity_kind_partial_failure_and_timeout_are_terminal() {
    for mode in [
        "program", "device", "epoch", "count", "frontier", "kind", "partial", "stall",
    ] {
        let (mut worker, path) = worker(mode);
        if mode == "stall" {
            worker.timeout = Duration::from_millis(100);
        }
        submit(&mut worker, fixture()).unwrap();
        assert!(worker.wait_fixed_token(PACKETS).is_err(), "{mode}");
        assert_eq!(worker.queue_packets, 0);
        assert!(worker.failed && worker.exited);
        assert!(worker.submit_fixed_token(&[]).is_err());
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn registration_identity_epoch_counts_and_handle_must_match_before_execution() {
    for field in [
        "program",
        "device_unique_id",
        "queue_epoch",
        "dispatches",
        "slots",
    ] {
        let (mut worker, path) = worker(&format!("registration_{field}"));
        assert!(submit(&mut worker, fixture()).is_err());
        assert!(worker.failed && worker.exited);
        assert_eq!(worker.queue_packets, 0);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "register_token_program\n"
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn mismatched_pending_count_and_failed_release_do_not_roll_epoch() {
    let (mut worker, path) = worker("normal");
    submit(&mut worker, fixture()).unwrap();
    assert!(worker.wait_fixed_token(PACKETS - 1).is_err());
    assert_eq!(worker.queue_packets, 0);
    std::fs::remove_file(path).unwrap();
    let (mut worker, path) = self::worker("release");
    submit(&mut worker, fixture()).unwrap();
    worker.wait_fixed_token(PACKETS).unwrap();
    worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1;
    assert!(worker.prepare_packets(PACKETS as u64).is_err());
    assert_eq!(worker.queue_epoch, 0);
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("rollover_queue")
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "requires FERRIC_V8_TEST_ARTIFACT; actual ABI packing with CPU fake worker, no GPU"]
fn actual_metadata_late_invalid_652nd_dispatch_sends_no_registration() {
    let path = std::env::var_os("FERRIC_V8_TEST_ARTIFACT").expect("explicit retained V8 image");
    let artifact = EngineeringTpArtifactV1::open_fp32_head32(
        Path::new(&path),
        &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
    )
    .unwrap();
    let symbol = "ferric_qwen3_tp_batch32_argmax_f32_v8";
    let metadata = artifact
        .inspection()
        .hsaco()
        .kernels()
        .iter()
        .find(|kernel| kernel.name() == symbol)
        .unwrap()
        .clone();
    let (mut worker, trace) = worker("normal");
    worker.kernels.insert(
        symbol.into(),
        LoadedKernel {
            id: 1,
            image: *artifact.hsaco_id().as_bytes(),
            metadata,
        },
    );
    worker
        .buffers
        .extend([(11, 32 * 151_936 * 4), (12, 32 * 4)]);
    let mut commands = vec![
        EngineeringTpDispatchV1 {
            kernel: symbol,
            grid_workgroups: 1,
            workgroup_size: 64,
            arguments: vec![
                EngineeringTpArgumentV1::Buffer {
                    id: 11,
                    offset: 0,
                    elements: 32 * 151_936,
                    element_bytes: 4,
                    access: EngineeringTpBufferAccessV1::Read
                },
                EngineeringTpArgumentV1::Buffer {
                    id: 12,
                    offset: 0,
                    elements: 32,
                    element_bytes: 4,
                    access: EngineeringTpBufferAccessV1::Write
                },
                EngineeringTpArgumentV1::U32(1)
            ]
        };
        PACKETS
    ];
    commands[PACKETS - 1].kernel = "unloaded_final_kernel";
    assert!(
        worker
            .submit_fixed_token(&commands)
            .unwrap_err()
            .contains("unloaded token program kernel")
    );
    assert_eq!(worker.queue_packets, 0);
    assert!(worker.failed && worker.exited);
    assert!(std::fs::read_to_string(&trace).unwrap().is_empty());
    std::fs::remove_file(trace).unwrap();
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AbiImages {
    target_v5: std::path::PathBuf,
    head_v8: std::path::PathBuf,
    argmax_v11: std::path::PathBuf,
    attention_v14: std::path::PathBuf,
    rmsnorm_v15: std::path::PathBuf,
    copy_v19: std::path::PathBuf,
    gemv_v20: std::path::PathBuf,
    split_v21: std::path::PathBuf,
    prefill_v27: std::path::PathBuf,
}

fn abi_symbol(name: &str) -> &'static str {
    use ferric_m1_engineering_execution_v1::tp_artifact::{
        ENGINEERING_TP_BATCH32_EXPORTS_V5, ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19,
        ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11, ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8,
        ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20, ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27,
        ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14, ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21,
        ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15,
    };
    let mut symbols = ENGINEERING_TP_BATCH32_EXPORTS_V5.to_vec();
    symbols.extend(ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8);
    symbols.extend(ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11);
    symbols.extend(ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14);
    symbols.extend(ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15);
    symbols.extend(ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19);
    symbols.extend(ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20);
    symbols.extend(ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21);
    symbols.extend(ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27);
    symbols
        .into_iter()
        .find(|symbol| *symbol == name)
        .expect("exact runner image symbol")
}

fn abi_commands(graph: &abi_fixture::Graph) -> Vec<EngineeringTpDispatchV1> {
    abi_commands_count(graph, PACKETS)
}

fn abi_commands_count(graph: &abi_fixture::Graph, count: usize) -> Vec<EngineeringTpDispatchV1> {
    abi_commands_with_symbol(graph, count, abi_symbol)
}

fn abi_commands_with_symbol(
    graph: &abi_fixture::Graph,
    count: usize,
    symbol: fn(&str) -> &'static str,
) -> Vec<EngineeringTpDispatchV1> {
    use abi_fixture::{Access, Argument};
    assert_eq!(graph.commands.len(), count);
    graph
        .commands
        .iter()
        .map(|command| {
            assert!(command.arguments.len() <= 32);
            EngineeringTpDispatchV1 {
                kernel: symbol(&command.kernel),
                grid_workgroups: command.grid_workgroups,
                workgroup_size: command.workgroup_size,
                arguments: command
                    .arguments
                    .iter()
                    .map(|argument| match *argument {
                        Argument::Buffer {
                            id,
                            offset,
                            elements,
                            element_bytes,
                            ref access,
                        } => EngineeringTpArgumentV1::Buffer {
                            id,
                            offset,
                            elements,
                            element_bytes,
                            access: match access {
                                Access::Read => EngineeringTpBufferAccessV1::Read,
                                Access::Write => EngineeringTpBufferAccessV1::Write,
                                Access::ReadWrite => EngineeringTpBufferAccessV1::ReadWrite,
                            },
                        },
                        Argument::U32 { value } => EngineeringTpArgumentV1::U32(value),
                        Argument::F32 { bits } => {
                            EngineeringTpArgumentV1::F32(f32::from_bits(bits))
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}

fn abi_sha256_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    digest
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

#[test]
fn abi_sha256_text_is_lowercase_fixed_width() {
    assert_eq!(
        abi_sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        abi_sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

fn abi_read(path: &Path, maximum: usize) -> Vec<u8> {
    let metadata = std::fs::symlink_metadata(path).unwrap();
    assert!(metadata.is_file() && metadata.len() <= u64::try_from(maximum).unwrap());
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .unwrap()
        .take(u64::try_from(maximum + 1).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= maximum);
    bytes
}

fn abi_expected_slots(
    commands: &[EngineeringTpDispatchV1],
    worker: &Worker,
) -> (Vec<Slot>, Vec<Update>) {
    let mut slots = Vec::new();
    let mut updates = Vec::new();
    for (index, command) in commands.iter().enumerate() {
        let dispatch = u16::try_from(index).unwrap();
        let fields = worker.kernels[command.kernel].metadata.explicit_arguments();
        if command.kernel == ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0] {
            for pointer in [2_u16, 3] {
                let EngineeringTpArgumentV1::Buffer {
                    id,
                    offset,
                    elements,
                    element_bytes,
                    ..
                } = command.arguments[usize::from(pointer)]
                else {
                    panic!("V19 output pointer");
                };
                assert_eq!(elements * usize::try_from(element_bytes).unwrap(), 2048);
                slots.push(Slot::Pointer {
                    dispatch,
                    pointer,
                    buffers: vec![id],
                    maximum_offset: u64::try_from(worker.buffers[&id] - 2048).unwrap(),
                });
                updates.push(Update::Pointer {
                    buffer: id,
                    offset: u64::try_from(offset).unwrap(),
                });
            }
            // Four slice arguments occupy eight physical fields before these scalars.
            for (argument, field, minimum, maximum) in [
                (4, 8, 127, 255),
                (5, 9, 0, scalar(&command.arguments, 6).unwrap() - 1),
            ] {
                slots.push(Slot::ScalarU32 {
                    dispatch,
                    offset: u32::try_from(fields[field].offset()).unwrap(),
                    minimum,
                    maximum,
                });
                updates.push(Update::ScalarU32 {
                    value: scalar(&command.arguments, argument).unwrap(),
                });
            }
        } else if command.kernel == ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0] {
            // Seven slice arguments plus four scalar arguments precede context.
            slots.push(Slot::ScalarU32 {
                dispatch,
                offset: u32::try_from(fields[18].offset()).unwrap(),
                minimum: 128,
                maximum: 256,
            });
            updates.push(Update::ScalarU32 {
                value: scalar(&command.arguments, 11).unwrap(),
            });
        }
    }
    (slots, updates)
}

fn abi_pointer_roster() -> BTreeMap<&'static str, (usize, usize, usize)> {
    // Per kernel: dispatches, pointer arguments, and mandatory empty slice arguments.
    BTreeMap::from([
        ("ferric_qwen3_tp_batch32_embedding_bf16_v5", (1, 3, 0)),
        ("ferric_qwen3_tp_batch32_mfma_head_f32_v8", (1, 3, 0)),
        ("ferric_qwen3_tp_batch32_residual_bf16_v5", (72, 3, 0)),
        ("ferric_qwen3_tp_batch32_rope_v5", (36, 7, 0)),
        ("ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5", (36, 3, 0)),
        ("ferric_qwen3_tp_batch32_wave_argmax_f32_v11", (1, 2, 0)),
        ("ferric_qwen3_tp_batch32_wave_gemv_bf16_v5", (180, 3, 0)),
        (
            "ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5",
            (72, 3, 0),
        ),
        ("ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15", (73, 5, 2)),
        ("ferric_qwen3_tp_c1_kv_copy_bf16_v19", (36, 4, 0)),
        (
            "ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21",
            (36, 3, 0),
        ),
        (
            "ferric_qwen3_tp_c1_split8_attention_partial_f32_v21",
            (36, 7, 0),
        ),
        ("qwen3_rmsnorm_v1", (72, 5, 2)),
    ])
}

fn abi_pointer_counts(
    commands: &[EngineeringTpDispatchV1],
    worker: &Worker,
    definition: &TokenProgramDefinitionV1,
) -> (usize, usize, usize) {
    abi_pointer_counts_for_roster(commands, worker, definition, PACKETS, abi_pointer_roster())
}

fn abi_pointer_counts_for_roster(
    commands: &[EngineeringTpDispatchV1],
    worker: &Worker,
    definition: &TokenProgramDefinitionV1,
    count: usize,
    roster: BTreeMap<&'static str, (usize, usize, usize)>,
) -> (usize, usize, usize) {
    use super::super::ExplicitValueKind;

    assert_eq!(commands.len(), count);
    assert_eq!(definition.dispatches.len(), commands.len());
    let mut seen = BTreeMap::new();
    let mut total = 0usize;
    let mut empty = 0usize;
    for (command, packed) in commands.iter().zip(&definition.dispatches) {
        let (_, pointer_arity, empty_arity) = roster[command.kernel];
        *seen.entry(command.kernel).or_insert(0usize) += 1;
        let fields = worker.kernels[command.kernel]
            .metadata
            .explicit_arguments()
            .iter()
            .filter(|field| field.value_kind() == ExplicitValueKind::GlobalBuffer)
            .collect::<Vec<_>>();
        let arguments = command
            .arguments
            .iter()
            .enumerate()
            .filter(|(_, argument)| matches!(argument, EngineeringTpArgumentV1::Buffer { .. }))
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), pointer_arity);
        assert_eq!(arguments.len(), pointer_arity);
        assert_eq!(packed.pointers.len(), pointer_arity);
        let mut dispatch_empty = 0usize;
        for (((argument_index, argument), field), pointer) in
            arguments.into_iter().zip(fields).zip(&packed.pointers)
        {
            let EngineeringTpArgumentV1::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                access,
            } = *argument
            else {
                unreachable!("filtered buffer argument");
            };
            let extent = elements
                .checked_mul(usize::try_from(element_bytes).unwrap())
                .unwrap();
            assert_eq!(
                pointer.kernarg_offset,
                u32::try_from(field.offset()).unwrap()
            );
            assert_eq!(pointer.buffer, id);
            assert_eq!(pointer.buffer_offset, u64::try_from(offset).unwrap());
            assert_eq!(pointer.extent_bytes, u64::try_from(extent).unwrap());
            assert_eq!(
                pointer.access,
                match access {
                    EngineeringTpBufferAccessV1::Read => BufferAccessV1::Read,
                    EngineeringTpBufferAccessV1::Write => BufferAccessV1::Write,
                    EngineeringTpBufferAccessV1::ReadWrite => BufferAccessV1::ReadWrite,
                }
            );
            let expected_empty = empty_arity != 0 && matches!(argument_index, 1 | 3);
            assert_eq!(extent == 0, expected_empty);
            if expected_empty {
                assert_eq!(offset, 0);
                assert_eq!(
                    pointer.access,
                    if argument_index == 1 {
                        BufferAccessV1::Read
                    } else {
                        BufferAccessV1::Write
                    }
                );
                dispatch_empty += 1;
            }
        }
        assert_eq!(dispatch_empty, empty_arity);
        total += pointer_arity;
        empty += dispatch_empty;
    }
    assert_eq!(
        seen,
        roster
            .iter()
            .map(|(&kernel, &(count, _, _))| (kernel, count))
            .collect::<BTreeMap<_, _>>()
    );
    (total, total - empty, empty)
}

#[test]
#[ignore = "requires FERRIC_TOKEN_ABI_GRAPH and FERRIC_TOKEN_ABI_IMAGES; retained real images, CPU fake IPC only"]
fn actual_images_pack_full652_and_reuse180_updates_across_page_boundary() {
    run_actual_image_fixture(false);
}

#[test]
#[ignore = "remote-only release CPU replay; requires retained FERRIC_TOKEN_ABI_GRAPH and FERRIC_TOKEN_ABI_IMAGES"]
fn actual_images_counted_encoding_cpu_abba() {
    assert!(
        !cfg!(debug_assertions),
        "CPU replay requires a release test build"
    );
    run_actual_image_fixture(true);
}

fn run_actual_image_fixture(counted_encoding_cpu_replay: bool) {
    run_actual_image_fixture_mode(counted_encoding_cpu_replay, false);
}

#[test]
#[ignore = "requires retained FERRIC_TOKEN_ABI_GRAPH prefill capture and FERRIC_TOKEN_ABI_IMAGES; CPU only"]
fn actual_images_pack_prefill613_and_reuse216_slots_across_rotation() {
    run_actual_image_fixture_mode(false, true);
}

#[test]
#[ignore = "requires retained FERRIC_TOKEN_ABI_GRAPH 649 capture and FERRIC_TOKEN_ABI_IMAGES; CPU only"]
fn actual_images_pack_prefill649_and_reuse396_slots_across_rotation() {
    run_actual_image_fixture_width(false, 32);
}

fn run_actual_image_fixture_mode(counted_encoding_cpu_replay: bool, prefill_program: bool) {
    run_actual_image_fixture_width(
        counted_encoding_cpu_replay,
        if prefill_program { 16 } else { 0 },
    );
}

fn abi_open_images(images: &AbiImages) -> [(&'static str, EngineeringTpArtifactV1); 9] {
    for path in [
        &images.target_v5,
        &images.head_v8,
        &images.argmax_v11,
        &images.attention_v14,
        &images.rmsnorm_v15,
        &images.copy_v19,
        &images.gemv_v20,
        &images.split_v21,
        &images.prefill_v27,
    ] {
        assert!(path.is_absolute() && path.canonicalize().unwrap() == *path);
    }
    let artifacts = [
        (
            "target_v5",
            EngineeringTpArtifactV1::open_batch32(
                &images.target_v5,
                &ferric_qwen3_tp_batch32_roster_bridge_v5::COMPILER_NAMES,
                true,
            )
            .unwrap(),
        ),
        (
            "head_v8",
            EngineeringTpArtifactV1::open_fp32_head32(
                &images.head_v8,
                &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
            )
            .unwrap(),
        ),
        (
            "argmax_v11",
            EngineeringTpArtifactV1::open_fp32_argmax32_v11(&images.argmax_v11).unwrap(),
        ),
        (
            "attention_v14",
            EngineeringTpArtifactV1::open_query_hoist_v14(&images.attention_v14).unwrap(),
        ),
        (
            "rmsnorm_v15",
            EngineeringTpArtifactV1::open_wave_rmsnorm_v15(&images.rmsnorm_v15).unwrap(),
        ),
        (
            "copy_v19",
            EngineeringTpArtifactV1::open_c1_kv_copy_v19(&images.copy_v19).unwrap(),
        ),
        (
            "gemv_v20",
            EngineeringTpArtifactV1::open_gemv_prefetch_v20(&images.gemv_v20).unwrap(),
        ),
        (
            "split_v21",
            EngineeringTpArtifactV1::open_split_attention_v21(&images.split_v21).unwrap(),
        ),
        (
            "prefill_v27",
            EngineeringTpArtifactV1::open_prefill_kv_copy_v27(&images.prefill_v27).unwrap(),
        ),
    ];
    artifacts
}

fn run_actual_image_fixture_width(counted_encoding_cpu_replay: bool, width: u32) {
    let prefill_program = width != 0;
    let fixture_path =
        std::env::var_os("FERRIC_TOKEN_ABI_GRAPH").expect("explicit recording capture");
    let image_path =
        std::env::var_os("FERRIC_TOKEN_ABI_IMAGES").expect("explicit nine-image directory map");
    let raw = abi_read(Path::new(&fixture_path), abi_fixture::MAX_BYTES);
    let snapshot: abi_fixture::Snapshot = serde_json::from_slice(&raw).unwrap();
    assert_eq!(snapshot.schema, abi_fixture::SCHEMA);
    assert_eq!(
        snapshot
            .graphs
            .iter()
            .map(|graph| graph.position)
            .collect::<Vec<_>>(),
        if width == 32 {
            vec![0, 32, 96]
        } else if prefill_program {
            vec![0, 16, 112]
        } else {
            vec![143, 144]
        }
    );
    assert!(!snapshot.buffers.is_empty() && snapshot.buffers.len() <= 2048);
    assert!(
        snapshot
            .buffers
            .iter()
            .all(|&(id, bytes)| id != 0 && bytes != 0)
    );
    let images_raw = abi_read(Path::new(&image_path), 16 * 1024);
    let images: AbiImages = serde_json::from_slice(&images_raw).unwrap();
    let artifacts = abi_open_images(&images);
    let (mut worker, trace) = worker("normal");
    for (_, artifact) in &artifacts {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        assert_eq!(hash, *artifact.hsaco_id().as_bytes());
        for metadata in artifact.inspection().hsaco().kernels() {
            let id = u64::try_from(worker.kernels.len() + 1).unwrap();
            assert!(
                worker
                    .kernels
                    .insert(
                        metadata.name().into(),
                        LoadedKernel {
                            id,
                            image: hash,
                            metadata: metadata.clone()
                        }
                    )
                    .is_none()
            );
        }
    }
    worker.buffers = snapshot.buffers.iter().copied().collect();
    assert_eq!(worker.buffers.len(), snapshot.buffers.len());
    if width == 32 {
        prefill32_actual::run(worker, trace, &snapshot, &raw, &images_raw, &artifacts);
        return;
    }
    if prefill_program {
        select_native_prefill(&mut worker);
        let commands = snapshot
            .graphs
            .iter()
            .map(|graph| abi_commands_count(graph, 613))
            .collect::<Vec<_>>();
        let mut reference = None;
        let mut encoding_bytes = Vec::new();
        for (index, graph) in commands.iter().enumerate() {
            let plan = prefill::plan_prefill(graph, &worker.kernels, &worker.buffers).unwrap();
            assert_eq!(plan.definition.slots.len(), 216);
            assert_eq!(
                plan.definition
                    .slots
                    .iter()
                    .filter(|slot| matches!(slot, Slot::Pointer { .. }))
                    .count(),
                72
            );
            assert_eq!(
                plan.definition
                    .slots
                    .iter()
                    .filter(|slot| matches!(slot, Slot::ScalarU32 { .. }))
                    .count(),
                144
            );
            let immutable = plan.immutable().unwrap();
            if let Some(expected) = &reference {
                assert_eq!(&immutable, expected);
            } else {
                reference = Some(immutable);
            }
            wire::validate_token_program_encoding_v1(&plan.definition, &plan.kernargs).unwrap();
            let (header, payload) =
                wire::encode_token_program_v1(&plan.definition, &plan.kernargs).unwrap();
            let CommandV1::RegisterTokenProgram {
                definition_bytes,
                kernarg_bytes,
            } = header
            else {
                panic!("legacy family");
            };
            let execute = CommandV1::ExecuteTokenProgram {
                program: 1,
                expected_epoch: 0,
                expected_completed_packets: u64::try_from(index).unwrap() * 613,
                timeout_ms: DISPATCH_TIMEOUT_MS,
                updates: plan.updates.clone(),
            };
            encoding_bytes.push(serde_json::json!({"definition_bytes":definition_bytes,"kernarg_bytes":kernarg_bytes,"registration_header_bytes":serde_json::to_vec(&header).unwrap().len(),"execute_header_bytes":serde_json::to_vec(&execute).unwrap().len(),"payload_bytes":payload.len()}));
        }
        for mutation in 0..10 {
            let mut invalid = commands[0].clone();
            match mutation {
                0 => {
                    invalid.pop();
                }
                1 => invalid[612].kernel = "unloaded_final_kernel",
                2 => invalid[8].arguments[4] = EngineeringTpArgumentV1::U32(1),
                3 => invalid[8].arguments[5] = EngineeringTpArgumentV1::U32(512),
                4 => invalid[8].arguments[6] = EngineeringTpArgumentV1::U32(0),
                5 => invalid[8].arguments[7] = EngineeringTpArgumentV1::U32(2),
                6 => invalid[9].arguments[6] = EngineeringTpArgumentV1::U32(1),
                7 => invalid[9].arguments[10] = EngineeringTpArgumentV1::U32(32),
                8 => invalid.swap(8, 9),
                _ => {
                    let EngineeringTpArgumentV1::Buffer { ref mut offset, .. } =
                        invalid[8].arguments[2]
                    else {
                        panic!("copy buffer")
                    };
                    *offset += 2;
                }
            }
            assert!(
                prefill::plan_prefill(&invalid, &worker.kernels, &worker.buffers).is_err(),
                "{mutation}"
            );
        }
        assert!(std::fs::read_to_string(&trace).unwrap().is_empty());
        for graph in &commands {
            worker.submit_fixed_prefill(graph).unwrap();
            worker.wait_fixed_token(613).unwrap();
        }
        assert_eq!(worker.queue_packets, 1839);
        worker.close().unwrap();
        assert_eq!(
            std::fs::read_to_string(&trace)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            [
                "register_token_program",
                "execute_token_program",
                "execute_token_program",
                "execute_token_program",
                "release_token_program",
                "close"
            ]
        );
        std::fs::remove_file(trace).unwrap();
        println!(
            "{}",
            serde_json::json!({
                "schema":"FerricPrefillProgramActualAbiCpuGateV1",
                "graph_sha256":abi_sha256_hex(&raw),"image_map_sha256":abi_sha256_hex(&images_raw),
                "images":artifacts.iter().map(|(role,artifact)| (role,abi_sha256_hex(artifact.bytes()))).collect::<BTreeMap<_,_>>(),
                "graphs":3,"positions":[0,16,112],"dispatches_per_graph":613,"dynamic_slots":216,
                "pointer_slots":72,"scalar_slots":144,"negative_cases":10,
                "encoding_bytes":encoding_bytes,
                "native_executed":false,"model_parity":false
            })
        );
        return;
    }
    let commands = snapshot.graphs.iter().map(abi_commands).collect::<Vec<_>>();
    let plans = commands
        .iter()
        .map(|commands| plan(commands, &worker.kernels, &worker.buffers).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(plans[0].immutable().unwrap(), plans[1].immutable().unwrap());
    assert!(
        plans[0]
            .updates
            .iter()
            .zip(&plans[1].updates)
            .all(|(before, after)| before != after)
    );
    let mut measurements = Vec::new();
    for (index, candidate) in plans.iter().enumerate() {
        let (slots, updates) = abi_expected_slots(&commands[index], &worker);
        assert_eq!(candidate.definition.slots, slots);
        assert_eq!(candidate.updates, updates);
        assert_eq!(
            slots
                .iter()
                .filter(|slot| matches!(slot, Slot::Pointer { .. }))
                .count(),
            72
        );
        assert_eq!(
            slots
                .iter()
                .filter(|slot| matches!(slot, Slot::ScalarU32 { .. }))
                .count(),
            108
        );
        let (pointer_fixups, nonempty_pointer_fixups, empty_pointer_fixups) =
            abi_pointer_counts(&commands[index], &worker, &candidate.definition);
        assert_eq!(
            (
                pointer_fixups,
                nonempty_pointer_fixups,
                empty_pointer_fixups
            ),
            (2569, 2279, 290)
        );
        let (header, payload) =
            wire::encode_token_program_v1(&candidate.definition, &candidate.kernargs).unwrap();
        wire::validate_token_program_encoding_v1(&candidate.definition, &candidate.kernargs)
            .unwrap();
        let CommandV1::RegisterTokenProgram {
            definition_bytes,
            kernarg_bytes,
        } = header
        else {
            panic!("registration header");
        };
        assert_eq!(
            usize::try_from(kernarg_bytes).unwrap(),
            candidate.kernargs.len()
        );
        assert_eq!(
            payload.len(),
            usize::try_from(definition_bytes + kernarg_bytes).unwrap()
        );
        assert!(definition_bytes <= wire::MAX_TOKEN_PROGRAM_DEFINITION_BYTES_V1);
        assert!(payload.len() <= usize::try_from(wire::MAX_TRANSFER_BYTES_V1).unwrap());
        let execute = CommandV1::ExecuteTokenProgram {
            program: 1,
            expected_epoch: 0,
            expected_completed_packets: u64::try_from(index * PACKETS).unwrap(),
            timeout_ms: DISPATCH_TIMEOUT_MS,
            updates,
        };
        let header_bytes = serde_json::to_vec(&execute).unwrap().len();
        assert!(header_bytes <= wire::MAX_HEADER_BYTES_V1);
        measurements.push(serde_json::json!({"position": snapshot.graphs[index].position,
            "definition_bytes": definition_bytes, "kernarg_bytes": kernarg_bytes, "payload_bytes": payload.len(),
            "execute_header_bytes": header_bytes, "pointer_fixups": pointer_fixups,
            "nonempty_pointer_fixups": nonempty_pointer_fixups, "empty_pointer_fixups": empty_pointer_fixups,
            "pointer_slots": 72, "scalar_slots": 108}));
        worker.submit_fixed_token(&commands[index]).unwrap();
        worker.wait_fixed_token(PACKETS).unwrap();
    }
    assert_eq!(worker.queue_packets, 1304);
    worker.close().unwrap();
    assert_eq!(
        std::fs::read_to_string(&trace)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        [
            "register_token_program",
            "execute_token_program",
            "execute_token_program",
            "release_token_program",
            "close"
        ]
    );
    std::fs::remove_file(trace).unwrap();
    println!(
        "{}",
        serde_json::json!({"schema": "FerricTokenProgramActualAbiCpuGateV1",
        "graph_sha256": abi_sha256_hex(&raw), "image_map_sha256": abi_sha256_hex(&images_raw),
        "images": artifacts.iter().map(|(role, artifact)| (role, abi_sha256_hex(artifact.bytes()))).collect::<BTreeMap<_, _>>(),
        "measurements": measurements, "native_executed": false, "model_parity": false})
    );
    if counted_encoding_cpu_replay {
        let replay = counted_encoding_cpu_abba(&plans, &snapshot.graphs);
        println!(
            "{}",
            serde_json::json!({
                "schema": "FerricCountedTokenEncodingCpuReplayV1",
                "graph_sha256": abi_sha256_hex(&raw),
                "image_map_sha256": abi_sha256_hex(&images_raw),
                "images": artifacts.iter().map(|(role, artifact)| (role, abi_sha256_hex(artifact.bytes()))).collect::<BTreeMap<_, _>>(),
                "replay": replay,
                "native_executed": false,
                "model_parity": false,
                "allocation_counts_measured": false
            })
        );
    }
}

#[derive(Clone, Copy)]
enum EncodingCpuArm {
    OwnedEncodeDiscard,
    CountedValidation,
}

impl EncodingCpuArm {
    fn name(self) -> &'static str {
        match self {
            Self::OwnedEncodeDiscard => "owned_encode_discard",
            Self::CountedValidation => "counted_validation",
        }
    }
}

fn encoding_cpu_sample(plan: &Plan, arm: EncodingCpuArm, iterations: usize) -> (u64, usize) {
    use std::hint::black_box;

    let mut returned_payload_bytes = 0usize;
    let started = std::time::Instant::now();
    match arm {
        EncodingCpuArm::OwnedEncodeDiscard => {
            for _ in 0..iterations {
                let (header, payload) = black_box(wire::encode_token_program_v1(
                    black_box(&plan.definition),
                    black_box(&plan.kernargs),
                ))
                .unwrap();
                returned_payload_bytes += payload.len();
                drop(black_box((header, payload)));
            }
        }
        EncodingCpuArm::CountedValidation => {
            for _ in 0..iterations {
                black_box(wire::validate_token_program_encoding_v1(
                    black_box(&plan.definition),
                    black_box(&plan.kernargs),
                ))
                .unwrap();
            }
        }
    }
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    (elapsed_ns, returned_payload_bytes)
}

fn counted_encoding_cpu_abba(plans: &[Plan], graphs: &[abi_fixture::Graph]) -> serde_json::Value {
    const WARMUPS_PER_ARM: usize = 8;
    const BLOCKS: usize = 3;
    const ITERATIONS: usize = 64;
    use EncodingCpuArm::{CountedValidation, OwnedEncodeDiscard};

    assert_eq!(plans.len(), 2);
    assert_eq!(
        graphs
            .iter()
            .map(|graph| graph.position)
            .collect::<Vec<_>>(),
        [143, 144]
    );
    let mut references = Vec::with_capacity(plans.len());
    let mut warmups = Vec::with_capacity(plans.len() * WARMUPS_PER_ARM * 2);
    let mut samples = Vec::with_capacity(plans.len() * BLOCKS * 4);
    for (plan_index, (plan, graph)) in plans.iter().zip(graphs).enumerate() {
        assert_eq!(plan.definition.dispatches.len(), PACKETS);
        assert_eq!(plan.definition.slots.len(), SLOTS);
        assert_eq!(plan.updates.len(), SLOTS);
        let reference = wire::encode_token_program_v1(&plan.definition, &plan.kernargs).unwrap();
        wire::validate_token_program_encoding_v1(&plan.definition, &plan.kernargs).unwrap();
        let CommandV1::RegisterTokenProgram {
            definition_bytes,
            kernarg_bytes,
        } = &reference.0
        else {
            panic!("registration header");
        };
        let payload_bytes = reference.1.len();
        assert_eq!(payload_bytes, (*definition_bytes + *kernarg_bytes) as usize);
        references.push(serde_json::json!({
            "position": graph.position,
            "dispatches": PACKETS,
            "slots": SLOTS,
            "updates": plan.updates.len(),
            "definition_bytes": definition_bytes,
            "kernarg_bytes": kernarg_bytes,
            "registration_payload_bytes": payload_bytes,
            "registration_payload_sha256": abi_sha256_hex(&reference.1)
        }));
        for round in 0..WARMUPS_PER_ARM {
            for arm in [OwnedEncodeDiscard, CountedValidation] {
                let (elapsed_ns, returned_payload_bytes) = encoding_cpu_sample(plan, arm, 1);
                let expected = match arm {
                    OwnedEncodeDiscard => payload_bytes,
                    CountedValidation => 0,
                };
                assert_eq!(returned_payload_bytes, expected);
                warmups.push(serde_json::json!({
                    "ordinal": warmups.len(), "plan_index": plan_index,
                    "position": graph.position, "round": round, "arm": arm.name(),
                    "iterations": 1, "elapsed_ns": elapsed_ns,
                    "returned_payload_bytes": returned_payload_bytes
                }));
            }
        }
        for block in 0..BLOCKS {
            for (leg, arm) in [
                OwnedEncodeDiscard,
                CountedValidation,
                CountedValidation,
                OwnedEncodeDiscard,
            ]
            .into_iter()
            .enumerate()
            {
                let (elapsed_ns, returned_payload_bytes) =
                    encoding_cpu_sample(plan, arm, ITERATIONS);
                let expected = match arm {
                    OwnedEncodeDiscard => payload_bytes.checked_mul(ITERATIONS).unwrap(),
                    CountedValidation => 0,
                };
                assert_eq!(returned_payload_bytes, expected);
                // Equality checks and sample recording are outside the measured interval.
                let observed =
                    wire::encode_token_program_v1(&plan.definition, &plan.kernargs).unwrap();
                assert_eq!(observed, reference);
                wire::validate_token_program_encoding_v1(&plan.definition, &plan.kernargs).unwrap();
                samples.push(serde_json::json!({
                    "ordinal": samples.len(), "plan_index": plan_index,
                    "position": graph.position, "block": block, "leg": leg,
                    "arm": arm.name(), "iterations": ITERATIONS, "elapsed_ns": elapsed_ns,
                    "returned_payload_bytes": returned_payload_bytes,
                    "post_sample_encoding_equal": true, "post_sample_validation_equal": true
                }));
            }
        }
    }
    assert_eq!(warmups.len(), 32);
    assert_eq!(samples.len(), 24);
    serde_json::json!({
        "warmups_per_arm_per_plan": WARMUPS_PER_ARM,
        "blocks_per_plan": BLOCKS,
        "order": ["owned_encode_discard", "counted_validation", "counted_validation", "owned_encode_discard"],
        "iterations_per_sample": ITERATIONS,
        "references": references,
        "warmups": warmups,
        "samples": samples,
        "actual_abi_preflight_passed": true,
        "fake_worker_closed_before_timing": true,
        "release_build": !cfg!(debug_assertions),
        "allocation_counts_measured": false
    })
}
