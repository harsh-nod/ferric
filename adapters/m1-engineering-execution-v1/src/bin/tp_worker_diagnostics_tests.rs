use super::*;

const FAKE: &str = r"
import json, struct, sys, time
mode = sys.argv[1]
counters = json.loads(sys.argv[2])
def send(value):
    data = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(data)) + data)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-','device_unique_id':1,'authority':'none'})
snapshots = 0
configured = False
while True:
    prefix = sys.stdin.buffer.read(4)
    if not prefix: break
    command = json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
    if command['op'] == 'configure_performance':
        assert command['profile'] is True
        assert not configured
        configured = True
        send({'op':'performance_configured'})
    elif command['op'] == 'performance_snapshot':
        assert configured
        if mode == 'stall': time.sleep(60)
        if mode == 'wrong': send({'op':'written'}); continue
        if mode == 'fatal': send({'op':'error','message':'injected','fatal':True}); continue
        if mode == 'truncated': sys.stdout.buffer.write(b'\x20'); sys.stdout.buffer.flush(); break
        next_counters = dict(counters)
        next_counters['command_ns'] = 4 if mode == 'regress' and snapshots else 5+snapshots
        next_counters['commands'] = snapshots+1
        if mode == 'missing': del next_counters['dispatch_wait_ns']
        send({'op':'performance_snapshot','counters':next_counters})
        snapshots += 1
    elif command['op'] == 'close': send({'op':'closed'}); break
    else: raise RuntimeError('unexpected command')
";

fn fixture(mode: &str, enabled: bool) -> Worker {
    let child = Command::new("python3")
        .args([
            "-u",
            "-c",
            FAKE,
            mode,
            &serde_json::to_string(&wire::PerformanceCountersV1::default()).unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut worker = Worker::connect(child, 1, Duration::from_millis(250)).unwrap();
    worker
        .configure_options(RuntimeOptions {
            profile: enabled,
            ..RuntimeOptions::default()
        })
        .unwrap();
    worker
}

#[test]
fn two_explicit_snapshots_preserve_identity_and_never_qualify_performance() {
    let mut worker = fixture("normal", true);
    for ordinal in 0..2 {
        let snapshot = worker.runtime_diagnostic_snapshot().unwrap();
        assert_eq!(snapshot["schema"], "FerricRuntimeDiagnosticSnapshotV1");
        assert_eq!(snapshot["performance_qualified"], false);
        assert_eq!(snapshot["authority"], "none");
        assert_eq!(snapshot["process_id"], worker.child.id());
        assert_eq!(snapshot["device_unique_id"], 1);
        assert_eq!(snapshot["ordinal"], ordinal);
        assert_eq!(snapshot["counters"]["dispatches"], 0);
    }
    worker.close().unwrap();
    assert!(worker.runtime_diagnostic_snapshot().is_err());
}

#[test]
fn missing_malformed_failed_timed_out_or_regressing_snapshot_is_terminal() {
    for mode in ["wrong", "fatal", "truncated", "stall", "missing", "regress"] {
        let mut worker = fixture(mode, true);
        if mode == "regress" {
            worker.runtime_diagnostic_snapshot().unwrap();
        }
        assert!(worker.runtime_diagnostic_snapshot().is_err(), "{mode}");
        assert!(worker.failed && worker.exited, "{mode}");
        assert!(worker.runtime_diagnostic_snapshot().is_err());
    }
}

#[test]
fn disabled_pending_and_exhausted_snapshots_never_send_a_snapshot_command() {
    let mut disabled = fixture("normal", false);
    disabled.close().unwrap();
    let mut disabled = fixture("normal", false);
    assert!(disabled.runtime_diagnostic_snapshot().is_err());
    assert!(disabled.failed && disabled.exited);
    let mut pending = fixture("normal", true);
    pending.pending = Some(PendingRequest::Dispatch);
    assert!(pending.runtime_diagnostic_snapshot().is_err());
    assert!(pending.failed && pending.exited);
    let mut exhausted = fixture("normal", true);
    exhausted.runtime_diagnostic_snapshot().unwrap();
    exhausted.runtime_diagnostic_snapshot().unwrap();
    assert!(exhausted.runtime_diagnostic_snapshot().is_err());
    assert!(exhausted.failed && exhausted.exited);
}
